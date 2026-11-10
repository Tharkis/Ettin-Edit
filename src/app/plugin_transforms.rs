//! Running plugins' transforms: on each selection (or the whole tab) as one
//! undo step, with a preview and Result goes to, and recorded into recipes.

use std::ops::Range;

use serde_json::{Map, Value, json};

use super::EttinApp;
use crate::plugin_transform_ui::{PluginDialogAction, Target};
use crate::plugins::headless::{TIMEOUT, reply_error_text, transform_text};
use crate::recipes::Step;
use crate::transform_ui::{Dest, Source};

/// A plugin transform running on a tab: one request per selection.
pub(super) struct PluginJob {
    plugin: String,
    title: String,
    doc: u64,
    version: u64,
    ranges: Vec<Range<usize>>,
    inputs: Vec<String>,
    requests: Vec<u64>,
    outputs: Vec<Option<String>>,
    dest: Dest,
    started: f64,
    /// Opened from the dialog, which shows the progress and errors.
    from_dialog: bool,
}

/// The key Result goes to is remembered under for a plugin transform.
fn dest_key(plugin: &str, transform: &str) -> String {
    format!("plugin:{plugin}/{transform}")
}

/// Where a 1-based line and column of `text` is, as a char offset.
fn offset_of(text: &str, line: usize, column: Option<usize>) -> usize {
    let mut offset = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i + 1 == line {
            let col = column.unwrap_or(1).saturating_sub(1).min(l.chars().count());
            return offset + col;
        }
        offset += l.chars().count() + 1;
    }
    text.chars().count()
}

impl EttinApp {
    /// Transform > Plugin Transforms > a transform: asks for its options,
    /// or runs it at once when it has none.
    pub(super) fn plugin_transform_action(
        &mut self,
        ctx: &egui::Context,
        plugin: &str,
        transform: &str,
    ) {
        if self.docs.is_empty() {
            return;
        }
        let Some((p, t)) = self.plugins.transform(plugin, transform) else {
            return self.tool_error(
                ctx,
                &t!("menu-plugin-transforms"),
                &t!("plugin-transform-gone", transform = transform.to_owned()),
                None,
                false,
            );
        };
        let target = Target {
            plugin: plugin.to_owned(),
            transform: transform.to_owned(),
            title: t.title.clone(),
            plugin_name: p.name().to_owned(),
        };
        let defaults = t.options.clone();
        if defaults.is_empty() {
            self.start_plugin_job(ctx, &target, defaults, Dest::ThisTab, false);
        } else {
            let dest = self
                .settings
                .transform_results
                .get(&dest_key(plugin, transform))
                .copied();
            self.plugin_dialog.show(target, defaults, dest);
        }
    }

    /// Sends each selection (or the whole tab) to the plugin.
    fn start_plugin_job(
        &mut self,
        ctx: &egui::Context,
        target: &Target,
        options: Map<String, Value>,
        dest: Dest,
        from_dialog: bool,
    ) {
        if self.plugin_job.is_some() {
            return;
        }
        let doc = &self.docs[self.active];
        if doc.editor.buffer.read_only && dest == Dest::ThisTab {
            self.plugin_dialog.close();
            return self.tool_error(ctx, &target.title, &t!("tool-read-only"), None, false);
        }
        if let Some(steps) = &mut self.recipe_recording {
            steps.push(Step::Plugin {
                plugin: target.plugin.clone(),
                transform: target.transform.clone(),
                options: options.clone(),
            });
        }
        let ranges = doc.editor.tool_targets();
        let inputs: Vec<String> = ranges
            .iter()
            .map(|r| doc.editor.buffer.slice_string(r.clone()))
            .collect();
        let (id, version) = (doc.id, doc.editor.buffer.version());
        let mut requests = Vec::new();
        for input in &inputs {
            let params = json!({ "id": target.transform, "text": input, "options": options });
            match self
                .plugins
                .request(&target.plugin, "transform.run", params)
            {
                Some(r) => requests.push(r),
                None => {
                    self.plugin_dialog.close();
                    return self.tool_error(
                        ctx,
                        &target.title,
                        &t!("plugin-transform-gone", transform = target.title.clone()),
                        None,
                        false,
                    );
                }
            }
        }
        self.plugin_job = Some(PluginJob {
            plugin: target.plugin.clone(),
            title: target.title.clone(),
            doc: id,
            version,
            outputs: vec![None; inputs.len()],
            ranges,
            inputs,
            requests,
            dest,
            started: ctx.input(|i| i.time),
            from_dialog,
        });
        self.plugin_dialog.running = from_dialog;
        self.poll_plugin_job(ctx);
    }

    /// Stops waiting for the plugin; the text stays as it was.
    fn stop_plugin_job(&mut self) {
        if let Some(job) = self.plugin_job.take() {
            for id in job.requests {
                self.plugins.take_reply(&job.plugin, id);
            }
        }
        self.plugin_dialog.running = false;
    }

    fn plugin_job_failed(&mut self, ctx: &egui::Context, message: String, pos: Option<usize>) {
        let job = self.plugin_job.take();
        self.plugin_dialog.running = false;
        let Some(job) = job else { return };
        if job.from_dialog && self.plugin_dialog.is_open() {
            self.plugin_dialog.error = Some(message);
            return;
        }
        let at = pos.filter(|_| self.docs.get(self.active).is_some_and(|d| d.id == job.doc));
        self.tool_error(ctx, &job.title, &message, at, false);
    }

    /// Takes the plugin's answers; applies them once every selection has one.
    pub(super) fn poll_plugin_job(&mut self, ctx: &egui::Context) {
        self.poll_plugin_preview();
        let Some(job) = &mut self.plugin_job else {
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        let now = ctx.input(|i| i.time);
        let mut failure = None;
        for (i, id) in job.requests.iter().enumerate() {
            if job.outputs[i].is_some() {
                continue;
            }
            let Some(reply) = self.plugins.take_reply(&job.plugin, *id) else {
                continue;
            };
            match reply.result.map_err(|e| (reply_error_text(&e), e)) {
                Ok(v) => match transform_text(&v) {
                    Ok(text) => job.outputs[i] = Some(text),
                    Err(e) => failure = Some((e, None)),
                },
                Err((text, e)) => {
                    let pos = e
                        .line
                        .map(|l| job.ranges[i].start + offset_of(&job.inputs[i], l, e.column));
                    failure = Some((text, pos));
                }
            }
            if failure.is_some() {
                break;
            }
        }
        if failure.is_none() && !self.plugins.is_running_plugin(&job.plugin) {
            failure = Some((
                t!("plugin-ended-during-transform", plugin = job.title.clone()),
                None,
            ));
        }
        if failure.is_none() && now - job.started > TIMEOUT.as_secs_f64() {
            failure = Some((
                t!(
                    "plugin-transform-timeout",
                    seconds = TIMEOUT.as_secs(),
                    plugin = job.title.clone()
                ),
                None,
            ));
        }
        if let Some((message, pos)) = failure {
            if let Some(job) = &self.plugin_job {
                let plugin = job.plugin.clone();
                for id in job.requests.clone() {
                    self.plugins.take_reply(&plugin, id);
                }
            }
            return self.plugin_job_failed(ctx, message, pos);
        }
        if job.outputs.iter().any(Option::is_none) {
            return;
        }
        let job = self.plugin_job.take().expect("running");
        self.plugin_dialog.running = false;
        if job.from_dialog {
            self.plugin_dialog.close();
        }
        let outputs: Vec<String> = job.outputs.into_iter().flatten().collect();
        let Some(index) = self.docs.iter().position(|d| d.id == job.doc) else {
            return;
        };
        if job.dest != Dest::ThisTab {
            let syntax = self.docs[index].syntax.clone();
            return self.send_result(ctx, job.dest, outputs.join("\n"), &syntax);
        }
        if self.docs[index].editor.buffer.version() != job.version {
            return self.tool_error(ctx, &job.title, &t!("pipe-text-changed"), None, false);
        }
        let whole = !self.docs[index].editor.selections.has_selection();
        let edits: Vec<(Range<usize>, String)> = job
            .ranges
            .into_iter()
            .zip(outputs)
            .zip(&job.inputs)
            .filter(|((_, out), input)| out != *input)
            .map(|((r, out), _)| (r, out))
            .collect();
        if index != self.active {
            self.select(index);
        }
        self.docs[index].editor.replace_ranges(ctx, edits, !whole);
        self.focus_editor = true;
    }

    /// Takes the answer to the dialog's preview, when it has come.
    fn poll_plugin_preview(&mut self) {
        let Some((id, ..)) = self.plugin_dialog.waiting else {
            return;
        };
        let Some(target) = &self.plugin_dialog.target else {
            return;
        };
        if !self.plugins.is_running_plugin(&target.plugin) {
            let name = target.plugin_name.clone();
            return self
                .plugin_dialog
                .preview_done(Err(t!("plugin-ended-during-transform", plugin = name)));
        }
        let plugin = target.plugin.clone();
        if let Some(reply) = self.plugins.take_reply(&plugin, id) {
            let result = match reply.result {
                Ok(v) => transform_text(&v),
                Err(e) => Err(reply_error_text(&e)),
            };
            self.plugin_dialog.preview_done(result);
        }
    }

    /// The plugin transform dialog, while it is open.
    pub(super) fn plugin_transform_dialog_ui(&mut self, ctx: &egui::Context) {
        if !self.plugin_dialog.is_open() {
            return;
        }
        if self.docs.is_empty() {
            return self.plugin_dialog.close();
        }
        let palette = self.palette();
        let editor = &self.docs[self.active].editor;
        let doc = &self.docs[self.active];
        let targets = editor.tool_targets();
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            doc.id.hash(&mut h);
            editor.buffer.version().hash(&mut h);
            targets.hash(&mut h);
            h.finish()
        };
        let lines = targets
            .iter()
            .map(|r| editor.buffer.char_to_line(r.end) - editor.buffer.char_to_line(r.start) + 1)
            .sum();
        let text = || {
            targets
                .iter()
                .map(|r| editor.buffer.slice_string(r.clone()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let source = Source {
            key,
            text: &text,
            lines,
            selection: editor.selections.has_selection(),
            dir: None,
            file: None,
        };
        let action = self.plugin_dialog.ui(ctx, &palette, &source);
        let Some(target) = self.plugin_dialog.target.clone() else {
            return;
        };
        match action {
            Some(PluginDialogAction::Preview { input, key }) => {
                let params = json!({
                    "id": target.transform,
                    "text": input,
                    "options": self.plugin_dialog.options,
                });
                match self
                    .plugins
                    .request(&target.plugin, "transform.run", params)
                {
                    Some(id) => self.plugin_dialog.previewing(id, key, input),
                    None => {
                        self.plugin_dialog.previewing(0, key, input);
                        self.plugin_dialog.preview_done(Err(t!(
                            "plugin-ended-during-transform",
                            plugin = target.plugin_name.clone()
                        )));
                    }
                }
            }
            Some(PluginDialogAction::Run(options, dest)) => {
                self.settings
                    .transform_results
                    .insert(dest_key(&target.plugin, &target.transform), dest);
                self.start_plugin_job(ctx, &target, options, dest, true);
            }
            Some(PluginDialogAction::Stop) => self.stop_plugin_job(),
            None => {}
        }
    }

    /// A small window while a transform without a dialog takes a while.
    pub(super) fn plugin_job_ui(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.plugin_job else {
            return;
        };
        if job.from_dialog || ctx.input(|i| i.time) - job.started < 0.4 {
            return;
        }
        let name = job.title.clone();
        let palette = self.palette();
        if crate::recipe_ui::running_ui(ctx, &palette, &name) {
            self.stop_plugin_job();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_positions_are_found_in_the_selection() {
        let text = "ab\ncdé\nf";
        assert_eq!(offset_of(text, 1, Some(2)), 1);
        assert_eq!(offset_of(text, 2, Some(3)), 5);
        assert_eq!(offset_of(text, 2, None), 3);
        assert_eq!(offset_of(text, 3, Some(9)), 8);
        assert_eq!(offset_of(text, 9, None), 8);
        assert_eq!(dest_key("p", "t"), "plugin:p/t");
    }
}
