//! Plugins in the app: answering their calls about documents, sending them
//! events, their panels, menus and windows.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::time::{Duration, Instant};

use egui::{Frame, Margin, Ui};
use serde_json::{Value, json};

use super::NixPadApp;
use crate::actions::Action;
use crate::plugins::panel::Position;
use crate::plugins::ui::{self as plugin_ui, ManagerAction, Permission, TextPrompt};
use crate::plugins::{Call, CallError, FAILED, INVALID_PARAMS, METHOD_NOT_FOUND, str_param};
use crate::text_tools_ui::Notice;

/// Seconds to wait after the last edit before sending `changed`.
const CHANGED_DELAY: f64 = 0.3;
/// How long one frame may spend answering plugins before drawing.
const FRAME_BUDGET: Duration = Duration::from_millis(8);
/// How long to wait for a plugin's next call before drawing.
const FOLLOW_UP_WAIT: Duration = Duration::from_millis(2);

/// A document as last seen: version, title and path.
type Seen = HashMap<u64, (u64, String, Option<String>)>;

/// Plugin state the app keeps: open windows, prompts, and what the
/// documents looked like last frame, to send events.
#[derive(Default)]
pub(crate) struct PluginUi {
    pub manager_open: bool,
    pub log_open: bool,
    prompts: VecDeque<TextPrompt>,
    /// Documents seen last frame: id → (version, title, path).
    seen: Option<Seen>,
    active: Option<u64>,
    /// Documents edited since the last `changed` event, with when.
    edited: HashMap<u64, f64>,
    /// Documents saved since last frame.
    pub saved: Vec<u64>,
    /// The paths in Manage Plugins, while being edited.
    python: Option<String>,
    node: Option<String>,
}

fn param_usize(params: &Value, key: &str) -> Result<Option<usize>, CallError> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_u64().map(|n| Some(n as usize)).ok_or((
            INVALID_PARAMS,
            format!("\"{key}\" must be a whole number of 0 or more"),
        )),
    }
}

fn param_str<'a>(params: &'a Value, key: &str) -> Result<&'a str, CallError> {
    str_param(params, key).ok_or((INVALID_PARAMS, format!("\"{key}\" must be text")))
}

fn read_only() -> CallError {
    (FAILED, "The document is read-only".to_owned())
}

impl NixPadApp {
    /// Starts the plugins, once.
    pub(super) fn start_plugins(&mut self, ctx: &egui::Context) {
        if self.plugins.scanned {
            return;
        }
        let now = ctx.input(|i| i.time);
        self.plugins.reload(
            &self.settings.plugins,
            &self.settings.python_path,
            &self.settings.node_path,
            now,
        );
    }

    /// Handles what plugins sent, and sends them events. Called each frame.
    pub(super) fn plugins_frame(&mut self, ctx: &egui::Context) {
        let start = Instant::now();
        loop {
            let now = ctx.input(|i| i.time);
            let calls = self.plugins.poll(now);
            for call in calls {
                self.plugin_call(ctx, call);
            }
            // A plugin often calls again right after an answer: keep going
            // for a little while instead of waiting a whole frame.
            if start.elapsed() >= FRAME_BUDGET {
                ctx.request_repaint();
                break;
            }
            if !self.plugins.wait(FOLLOW_UP_WAIT) {
                break;
            }
        }
        for text in std::mem::take(&mut self.plugins.notices) {
            self.notice = Some(Notice {
                text,
                error: true,
                since: None,
                jump: None,
            });
        }
        self.plugin_events(ctx);
    }

    /// Works out which documents opened, closed, changed or got focus, and
    /// tells the plugins that asked.
    fn plugin_events(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let current: Seen = self
            .docs
            .iter()
            .map(|d| {
                (
                    d.id,
                    (
                        d.editor.buffer.version(),
                        d.title(),
                        d.path.as_ref().map(|p| p.display().to_string()),
                    ),
                )
            })
            .collect();
        let active = self.docs.get(self.active).map(|d| d.id);
        let Some(seen) = self.plugin_ui.seen.replace(current.clone()) else {
            // First frame: nothing happened yet.
            self.plugin_ui.active = active;
            return;
        };
        let mut events: Vec<(&str, Value)> = Vec::new();
        for (id, (version, ..)) in &current {
            match seen.get(id) {
                None => events.push(("opened", self.plugin_doc_json(*id))),
                Some((old, ..)) if old != version => {
                    self.plugin_ui.edited.insert(*id, now);
                }
                _ => {}
            }
        }
        for (id, (_, title, path)) in &seen {
            if !current.contains_key(id) {
                self.plugin_ui.edited.remove(id);
                events.push(("closed", json!({ "id": id, "title": title, "path": path })));
            }
        }
        for id in std::mem::take(&mut self.plugin_ui.saved) {
            events.push(("saved", self.plugin_doc_json(id)));
        }
        if active != self.plugin_ui.active {
            self.plugin_ui.active = active;
            if let Some(id) = active {
                events.push(("activated", self.plugin_doc_json(id)));
            }
        }
        let due: Vec<u64> = self
            .plugin_ui
            .edited
            .iter()
            .filter(|(_, t)| now - **t >= CHANGED_DELAY)
            .map(|(id, _)| *id)
            .collect();
        for id in due {
            self.plugin_ui.edited.remove(&id);
            events.push(("changed", self.plugin_doc_json(id)));
        }
        if !self.plugin_ui.edited.is_empty() {
            ctx.request_repaint_after(Duration::from_secs_f64(CHANGED_DELAY));
        }
        for (event, doc) in events {
            if self.plugins.wants(event) && !doc.is_null() {
                self.plugins.event(event, json!({ "doc": doc }));
            }
        }
    }

    fn plugin_doc_json(&self, id: u64) -> Value {
        match self.docs.iter().position(|d| d.id == id) {
            Some(i) => self.plugin_doc_info(i),
            None => Value::Null,
        }
    }

    /// What plugins are told about a document.
    fn plugin_doc_info(&self, i: usize) -> Value {
        let d = &self.docs[i];
        let (line, column) = d.editor.caret_line_col();
        json!({
            "id": d.id,
            "title": d.title(),
            "path": d.path.as_ref().map(|p| p.display().to_string()),
            "language": d.syntax,
            "modified": d.is_dirty(),
            "readOnly": d.editor.buffer.read_only,
            "length": d.editor.buffer.len_chars(),
            "lineCount": d.line_count(),
            "active": i == self.active,
            "caret": {
                "line": line,
                "column": column,
                "offset": d.editor.selections.primary().head,
            },
            "encoding": d.encoding.name(),
            "lineEnding": d.line_ending.short_label(),
        })
    }

    /// The document a call is about: `doc` by id, or the active one.
    fn plugin_doc(&self, params: &Value) -> Result<usize, CallError> {
        match params.get("doc") {
            None | Some(Value::Null) => {
                if self.docs.is_empty() {
                    Err((FAILED, "No document is open".to_owned()))
                } else {
                    Ok(self.active)
                }
            }
            Some(v) => {
                let id = v
                    .as_u64()
                    .ok_or((INVALID_PARAMS, "\"doc\" must be a document id".to_owned()))?;
                self.docs.iter().position(|d| d.id == id).ok_or((
                    FAILED,
                    format!("No open document has id {id}; it may have been closed"),
                ))
            }
        }
    }

    /// Answers one call from a plugin.
    fn plugin_call(&mut self, ctx: &egui::Context, call: Call) {
        if call.method == "ui.prompt" {
            if let Some(id) = call.id {
                let name = self
                    .plugins
                    .plugins
                    .iter()
                    .find(|p| p.id == call.plugin)
                    .map(|p| p.name().to_owned())
                    .unwrap_or_default();
                self.plugin_ui.prompts.push_back(TextPrompt {
                    title: str_param(&call.params, "title").unwrap_or(&name).to_owned(),
                    prompt: str_param(&call.params, "prompt")
                        .unwrap_or_default()
                        .to_owned(),
                    value: str_param(&call.params, "value")
                        .unwrap_or_default()
                        .to_owned(),
                    plugin: call.plugin,
                    run: call.run,
                    id,
                    focused: false,
                });
            }
            return;
        }
        let result = self.plugin_answer(ctx, &call.method, &call.params);
        if let Some(id) = call.id {
            self.plugins.respond(&call.plugin, call.run, id, result);
        } else if let Err((_, message)) = result {
            self.plugins
                .log(&call.plugin, format!("{}: {message}", call.method), true);
        }
    }

    fn plugin_answer(
        &mut self,
        ctx: &egui::Context,
        method: &str,
        params: &Value,
    ) -> Result<Value, CallError> {
        match method {
            "documents.list" => Ok(Value::Array(
                (0..self.docs.len())
                    .map(|i| self.plugin_doc_info(i))
                    .collect(),
            )),
            "documents.active" | "documents.info" => {
                let i = self.plugin_doc(params)?;
                Ok(self.plugin_doc_info(i))
            }
            "documents.open" => {
                let path = Path::new(param_str(params, "path")?);
                let errors = self.errors.len();
                self.open_path(path, true);
                if self.errors.len() > errors {
                    let message = self.errors.pop().unwrap_or_default();
                    return Err((FAILED, message));
                }
                let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                let i = self
                    .docs
                    .iter()
                    .position(|d| d.path.as_deref() == Some(path.as_path()))
                    .unwrap_or(self.active);
                Ok(self.plugin_doc_info(i))
            }
            "documents.new" => {
                let text = str_param(params, "text").unwrap_or_default();
                self.new_tab();
                let language = str_param(params, "language").and_then(|l| {
                    self.languages
                        .iter()
                        .find(|n| n.eq_ignore_ascii_case(l))
                        .cloned()
                });
                let doc = &mut self.docs[self.active];
                if !text.is_empty() {
                    doc.editor = crate::editor::Editor::new(text);
                    doc.editor.buffer.mark_modified();
                }
                if let Some(l) = language {
                    doc.syntax = l;
                }
                Ok(self.plugin_doc_info(self.active))
            }
            "documents.activate" => {
                let i = self.plugin_doc(params)?;
                self.select(i);
                Ok(Value::Null)
            }
            "documents.save" => {
                let i = self.plugin_doc(params)?;
                Ok(json!(self.save(i, false)))
            }
            "documents.close" => {
                let i = self.plugin_doc(params)?;
                let id = self.docs[i].id;
                self.request_close(vec![id], false);
                Ok(Value::Null)
            }
            "text.get" => {
                let i = self.plugin_doc(params)?;
                let buffer = &self.docs[i].editor.buffer;
                let len = buffer.len_chars();
                let start = param_usize(params, "start")?.unwrap_or(0).min(len);
                let end = param_usize(params, "end")?.unwrap_or(len).clamp(start, len);
                Ok(json!(buffer.slice_string(start..end)))
            }
            "text.set" => {
                let i = self.plugin_doc(params)?;
                let text = param_str(params, "text")?;
                if !self.docs[i].editor.set_text_undoable(ctx, text) {
                    return Err(read_only());
                }
                Ok(Value::Null)
            }
            "text.replace" => {
                let i = self.plugin_doc(params)?;
                let len = self.docs[i].editor.buffer.len_chars();
                let start = param_usize(params, "start")?.unwrap_or(0);
                let end = param_usize(params, "end")?.unwrap_or(start);
                if start > end || end > len {
                    return Err((
                        INVALID_PARAMS,
                        format!("{start}..{end} isn't inside the text (0..{len})"),
                    ));
                }
                let text = param_str(params, "text")?.to_owned();
                if !self.docs[i]
                    .editor
                    .edit_ranges(ctx, vec![(start..end, text)])
                {
                    return Err(read_only());
                }
                Ok(Value::Null)
            }
            "text.insert" => {
                let i = self.plugin_doc(params)?;
                let text = param_str(params, "text")?;
                let editor = &mut self.docs[i].editor;
                if editor.buffer.read_only {
                    return Err(read_only());
                }
                editor.insert_text(ctx, text);
                Ok(Value::Null)
            }
            "selection.get" => {
                let i = self.plugin_doc(params)?;
                let editor = &self.docs[i].editor;
                let primary = editor.selections.primary_index();
                Ok(Value::Array(
                    editor
                        .selections
                        .iter()
                        .enumerate()
                        .map(|(n, s)| {
                            json!({
                                "start": s.start(),
                                "end": s.end(),
                                "anchor": s.anchor,
                                "head": s.head,
                                "text": editor.buffer.slice_string(s.range()),
                                "primary": n == primary,
                            })
                        })
                        .collect(),
                ))
            }
            "selection.set" => {
                let i = self.plugin_doc(params)?;
                let ranges: Vec<(usize, usize)> = params
                    .get("ranges")
                    .and_then(Value::as_array)
                    .and_then(|a| {
                        a.iter()
                            .map(|r| {
                                let r = r.as_array()?;
                                let anchor = r.first()?.as_u64()? as usize;
                                let head = r
                                    .get(1)
                                    .and_then(Value::as_u64)
                                    .map_or(anchor, |h| h as usize);
                                Some((anchor, head))
                            })
                            .collect()
                    })
                    .filter(|r: &Vec<_>| !r.is_empty())
                    .ok_or((
                        INVALID_PARAMS,
                        "\"ranges\" must be a list of [start, end] pairs".to_owned(),
                    ))?;
                self.docs[i].editor.select_ranges(&ranges);
                Ok(Value::Null)
            }
            "selection.replace" => {
                let i = self.plugin_doc(params)?;
                let texts: Vec<String> = match (params.get("texts"), str_param(params, "text")) {
                    (Some(Value::Array(a)), _) => a
                        .iter()
                        .map(|v| v.as_str().map(str::to_owned))
                        .collect::<Option<_>>()
                        .ok_or((
                            INVALID_PARAMS,
                            "\"texts\" must be a list of text".to_owned(),
                        ))?,
                    (_, Some(t)) => vec![t.to_owned()],
                    _ => return Err((INVALID_PARAMS, "Give \"text\" or \"texts\"".to_owned())),
                };
                self.docs[i]
                    .editor
                    .replace_selections(ctx, &texts)
                    .map_err(|e| (FAILED, e))?;
                Ok(Value::Null)
            }
            "editor.goto" => {
                let i = self.plugin_doc(params)?;
                let line = param_usize(params, "line")?.unwrap_or(1).max(1) - 1;
                let column = param_usize(params, "column")?.unwrap_or(1).max(1) - 1;
                let editor = &mut self.docs[i].editor;
                let line = line.min(editor.buffer.len_lines().saturating_sub(1));
                let pos = editor.buffer.line_start(line) + column.min(editor.buffer.line_len(line));
                editor.select_range(pos..pos);
                if i == self.active {
                    self.focus_editor = true;
                }
                Ok(Value::Null)
            }
            "editor.runAction" => {
                let action: Action =
                    serde_json::from_value(params.get("action").cloned().unwrap_or_default())
                        .map_err(|e| (INVALID_PARAMS, format!("Unknown action: {e}")))?;
                self.run(ctx, action);
                Ok(Value::Null)
            }
            "ui.message" => {
                let text = param_str(params, "text")?.to_owned();
                let kind = str_param(params, "kind").unwrap_or("info");
                self.notice = Some(Notice {
                    text,
                    error: kind != "info",
                    since: None,
                    jump: None,
                });
                Ok(Value::Null)
            }
            "clipboard.set" => {
                ctx.copy_text(param_str(params, "text")?.to_owned());
                Ok(Value::Null)
            }
            _ => Err((
                METHOD_NOT_FOUND,
                format!("NixPad has no method \"{method}\""),
            )),
        }
    }

    // ---------------------------------------------------------------------
    // Panels, windows and actions

    /// Draws the plugins' panels.
    pub(super) fn plugin_panels(&mut self, ui: &mut Ui) {
        if self.plugins.panels.is_empty() {
            return;
        }
        let palette = self.palette();
        let mut events = Vec::new();
        for (i, panel) in self.plugins.panels.iter_mut().enumerate() {
            let id = egui::Id::new(("plugin-panel", &panel.plugin, &panel.id));
            let frame = Frame::new()
                .fill(palette.chrome_bg)
                .inner_margin(Margin::symmetric(8, 4));
            let position = panel.position;
            let mut show = |ui: &mut Ui| {
                for e in panel.ui(ui, &palette) {
                    events.push((i, e));
                }
            };
            match position {
                Position::Right => {
                    egui::Panel::right(id)
                        .resizable(true)
                        .default_size(300.0)
                        .min_size(180.0)
                        .frame(frame)
                        .show(ui, |ui| show(ui));
                }
                Position::Bottom => {
                    egui::Panel::bottom(id)
                        .resizable(true)
                        .default_size(200.0)
                        .min_size(80.0)
                        .frame(frame)
                        .show(ui, |ui| {
                            ui.set_min_height(ui.available_height());
                            show(ui)
                        });
                }
            }
        }
        // Closing removes a panel, so go from the last.
        for (i, e) in events.into_iter().rev() {
            self.plugins.panel_event(i, e);
        }
    }

    /// The permission prompt, text prompts and plugin windows.
    pub(super) fn plugin_dialogs(&mut self, ctx: &egui::Context) {
        let palette = self.palette();
        if let Some(id) = self.plugins.prompts.front().cloned() {
            let changed = self
                .settings
                .plugins
                .get(&id)
                .is_some_and(|p| p.approved.is_some());
            match plugin_ui::permission(ctx, &self.plugins, &id, changed, &palette) {
                Some(Permission::Allow) => {
                    self.plugins.prompts.pop_front();
                    self.allow_plugin(&id);
                    // Remember the answer even if NixPad doesn't close normally.
                    self.settings.save();
                }
                Some(Permission::Deny) => {
                    self.plugins.prompts.pop_front();
                    self.settings
                        .plugins
                        .entry(id.clone())
                        .or_default()
                        .disabled = true;
                    if let Some(p) = self.plugins.index_of(&id) {
                        self.plugins.plugins[p].status = crate::plugins::Status::Disabled;
                    }
                    self.settings.save();
                }
                Some(Permission::Later) => {
                    self.plugins.prompts.pop_front();
                }
                None => {}
            }
        } else if let Some(prompt) = self.plugin_ui.prompts.front_mut() {
            if !self.plugins.is_current(&prompt.plugin, prompt.run) {
                self.plugin_ui.prompts.pop_front();
            } else if let Some(answer) = plugin_ui::text_prompt(ctx, prompt) {
                let p = self.plugin_ui.prompts.pop_front().expect("prompt is open");
                self.plugins
                    .respond(&p.plugin, p.run, p.id, Ok(json!(answer)));
                self.focus_editor = true;
            }
        }

        if self.plugin_ui.manager_open {
            let python = self
                .plugin_ui
                .python
                .get_or_insert_with(|| self.settings.python_path.clone());
            let node = self
                .plugin_ui
                .node
                .get_or_insert_with(|| self.settings.node_path.clone());
            let actions = plugin_ui::manager(
                ctx,
                &mut self.plugin_ui.manager_open,
                &self.plugins,
                &self.settings.plugins,
                python,
                node,
                &palette,
            );
            // The paths count once they're edited; plugins pick them up when they start.
            self.settings.python_path = python.trim().to_owned();
            self.settings.node_path = node.trim().to_owned();
            for a in actions {
                self.manager_action(ctx, a);
            }
        } else {
            self.plugin_ui.python = None;
            self.plugin_ui.node = None;
        }
        if self.plugin_ui.log_open {
            plugin_ui::log_window(
                ctx,
                &mut self.plugin_ui.log_open,
                &mut self.plugins,
                &palette,
            );
        }
    }

    /// True while a plugin dialog that blocks the editor is open.
    pub(super) fn plugin_modal_open(&self) -> bool {
        !self.plugins.prompts.is_empty() || !self.plugin_ui.prompts.is_empty()
    }

    fn allow_plugin(&mut self, id: &str) {
        let Some(i) = self.plugins.index_of(id) else {
            return;
        };
        let fingerprint = crate::plugins::manifest::fingerprint(&self.plugins.plugins[i].dir);
        let pref = self.settings.plugins.entry(id.to_owned()).or_default();
        pref.approved = Some(fingerprint);
        pref.disabled = false;
        self.start_plugin(i);
    }

    fn start_plugin(&mut self, i: usize) {
        self.plugins.start_if_allowed(
            i,
            &self.settings.plugins,
            &self.settings.python_path,
            &self.settings.node_path,
        );
    }

    fn manager_action(&mut self, ctx: &egui::Context, action: ManagerAction) {
        let now = ctx.input(|i| i.time);
        match action {
            ManagerAction::SetEnabled(id, on) => {
                self.settings
                    .plugins
                    .entry(id.clone())
                    .or_default()
                    .disabled = !on;
                if let Some(i) = self.plugins.index_of(&id) {
                    if on {
                        self.start_plugin(i);
                    } else {
                        self.plugins.stop(i, now);
                        self.plugins.plugins[i].status = crate::plugins::Status::Disabled;
                        self.plugins.prompts.retain(|p| *p != id);
                    }
                }
            }
            ManagerAction::SetTrust(id, on) => {
                self.settings.plugins.entry(id).or_default().trust_changes = on;
            }
            ManagerAction::Start(id) => {
                if let Some(i) = self.plugins.index_of(&id) {
                    self.plugins.stop(i, now);
                    self.start_plugin(i);
                }
            }
            ManagerAction::Stop(id) => {
                if let Some(i) = self.plugins.index_of(&id) {
                    self.plugins.stop(i, now);
                }
            }
            ManagerAction::Allow(id) => {
                if !self.plugins.prompts.contains(&id) {
                    self.plugins.prompts.push_back(id);
                }
            }
            ManagerAction::OpenFolder(id) => {
                if let Some(i) = self.plugins.index_of(&id) {
                    let dir = self.plugins.plugins[i].dir.clone();
                    if let Err(e) = crate::system::open_folder(&dir) {
                        self.errors.push(e);
                    }
                }
            }
            ManagerAction::Reload => self.run(ctx, Action::ReloadPlugins),
            ManagerAction::OpenPluginsFolder => self.run(ctx, Action::OpenPluginsFolder),
            ManagerAction::InstallSamples => self.run(ctx, Action::InstallSamplePlugins),
            ManagerAction::ShowLog => self.plugin_ui.log_open = true,
            ManagerAction::ShowGuide => self.run(ctx, Action::PluginGuide),
        }
    }

    /// Runs the plugin commands from the Plugins menu.
    pub(super) fn run_plugin_action(&mut self, ctx: &egui::Context, action: Action) {
        let now = ctx.input(|i| i.time);
        match action {
            Action::PluginCommand { plugin, command } => {
                self.plugins.run_command(&plugin, &command)
            }
            Action::ManagePlugins => self.plugin_ui.manager_open = true,
            Action::ShowPluginLog => self.plugin_ui.log_open = true,
            Action::OpenPluginsFolder => {
                if let Some(dir) = &self.plugins.dir {
                    let _ = std::fs::create_dir_all(dir);
                    if let Err(e) = crate::system::open_folder(dir) {
                        self.errors.push(e);
                    }
                }
            }
            Action::ReloadPlugins => {
                self.plugins.reload(
                    &self.settings.plugins,
                    &self.settings.python_path,
                    &self.settings.node_path,
                    now,
                );
                self.notice = Some(Notice {
                    text: match self
                        .plugins
                        .plugins
                        .iter()
                        .filter(|p| p.is_running())
                        .count()
                    {
                        1 => "1 plugin running".to_owned(),
                        n => format!("{n} plugins running"),
                    },
                    error: false,
                    since: None,
                    jump: None,
                });
            }
            Action::InstallSamplePlugins => self.install_samples(now),
            Action::PluginGuide => {
                // A file, so the tab has a proper name; read-only, as it's
                // rewritten by each NixPad version.
                let Some(dir) = self.plugins.dir.as_ref().map(|d| d.join(".sdk")) else {
                    return;
                };
                let path = dir.join("Plugin Guide.md");
                let written = std::fs::create_dir_all(&dir).and_then(|_| {
                    if std::fs::read_to_string(&path).ok().as_deref() == Some(crate::plugins::GUIDE)
                    {
                        Ok(())
                    } else {
                        std::fs::write(&path, crate::plugins::GUIDE)
                    }
                });
                if let Err(e) = written {
                    self.errors
                        .push(format!("Couldn't write {}: {e}", path.display()));
                    return;
                }
                self.open_path(&path, false);
                if let Some(doc) = self.docs.get_mut(self.active)
                    && doc
                        .path
                        .as_deref()
                        .is_some_and(|p| p.ends_with("Plugin Guide.md"))
                {
                    doc.editor.buffer.read_only = true;
                }
            }
            _ => {}
        }
    }

    /// Copies the sample plugins into the plugins folder and starts them.
    fn install_samples(&mut self, now: f64) {
        let Some(root) = self.plugins.dir.clone() else {
            self.errors
                .push("There's no config folder to put plugins in.".to_owned());
            return;
        };
        let mut installed = Vec::new();
        for (folder, files) in crate::plugins::SAMPLES {
            let dir = root.join(folder);
            if dir.exists() {
                continue;
            }
            let written = std::fs::create_dir_all(&dir).and_then(|_| {
                files
                    .iter()
                    .try_for_each(|(name, text)| std::fs::write(dir.join(name), text))
            });
            if let Err(e) = written {
                self.errors.push(format!(
                    "Couldn't install the sample plugin {folder} in {}: {e}",
                    root.display()
                ));
                continue;
            }
            // The user asked for these, so they don't need asking again.
            let pref = self
                .settings
                .plugins
                .entry((*folder).to_owned())
                .or_default();
            pref.approved = Some(crate::plugins::manifest::fingerprint(&dir));
            pref.disabled = false;
            installed.push(*folder);
        }
        self.settings.save();
        self.plugins.reload(
            &self.settings.plugins,
            &self.settings.python_path,
            &self.settings.node_path,
            now,
        );
        let names: Vec<String> = installed
            .iter()
            .map(|id| match self.plugins.index_of(id) {
                Some(i) => self.plugins.plugins[i].name().to_owned(),
                None => (*id).to_owned(),
            })
            .collect();
        let text = if names.is_empty() {
            "The sample plugins are already in the plugins folder.".to_owned()
        } else {
            format!(
                "Installed {}. Find them in the Plugins menu.",
                names.join(" and ")
            )
        };
        self.notice = Some(Notice {
            text,
            error: false,
            since: None,
            jump: None,
        });
    }

    /// The Plugins menu.
    pub(super) fn plugins_menu(&mut self, ui: &mut Ui) {
        let mut any = false;
        let mut clicked = None;
        for p in &self.plugins.plugins {
            if !p.is_running() || p.commands.is_empty() {
                continue;
            }
            any = true;
            ui.menu_button(p.name(), |ui| {
                for c in &p.commands {
                    let mut button = egui::Button::new(&c.title);
                    if let Some(s) = &c.shortcut {
                        button = button.shortcut_text(ui.ctx().format_shortcut(s));
                    }
                    if ui.add(button).clicked() {
                        clicked = Some(Action::PluginCommand {
                            plugin: p.id.clone(),
                            command: c.id.clone(),
                        });
                    }
                }
            });
        }
        if !any {
            ui.label(egui::RichText::new("No plugin commands").weak());
        }
        self.pending.extend(clicked);
        ui.separator();
        self.menu_item(ui, "Manage Plugins…", Action::ManagePlugins);
        self.menu_item(ui, "Plugin Log", Action::ShowPluginLog);
        self.menu_item(ui, "Open Plugins Folder", Action::OpenPluginsFolder);
        self.menu_item(ui, "Reload Plugins", Action::ReloadPlugins);
        ui.separator();
        self.menu_item(ui, "Install Sample Plugins", Action::InstallSamplePlugins);
        self.menu_item(ui, "Plugin Guide", Action::PluginGuide);
    }
}
