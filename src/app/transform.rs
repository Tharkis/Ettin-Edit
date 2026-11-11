//! Running the Transform menu: line tools, conversions, hashes, and sending
//! text through a shell command.

use std::cell::RefCell;
use std::ops::Range;
use std::process::Child;

use super::EttinApp;
use crate::tools::convert;
use crate::tools::pipe::{Output, pipe_output, spawn};
use crate::tools::transform::{self, LineMatcher, TransformOp};
use crate::transform_ui::{Dest, DialogAction, DialogKind, MAX_COMMANDS, Source};

/// A shell command running for Filter Through Command: one run per
/// selection, one after another.
pub(super) struct PipeJob {
    command: String,
    doc: u64,
    version: u64,
    ranges: Vec<Range<usize>>,
    inputs: Vec<String>,
    outputs: Vec<String>,
    child: Option<Child>,
    /// Standard output and error of the running command, read on threads.
    output: Option<Output>,
    /// Where the output goes.
    dest: Dest,
}

impl EttinApp {
    /// Each selection grown to whole lines, or the whole text. Line tools
    /// work on these so a selection that starts mid-line still covers it.
    fn line_targets(&self) -> Vec<Range<usize>> {
        let editor = &self.docs[self.active].editor;
        if !editor.selections.has_selection() {
            return editor.tool_targets();
        }
        let buf = &editor.buffer;
        let mut out: Vec<Range<usize>> = Vec::new();
        for r in editor.tool_targets() {
            let first = buf.char_to_line(r.start);
            // A selection ending at the start of a line doesn't take that line.
            let mut last = buf.char_to_line(r.end);
            if last > first && buf.line_start(last) == r.end {
                last -= 1;
            }
            let range = buf.line_start(first)..buf.line_start(last) + buf.line_len(last);
            match out.last_mut() {
                Some(prev) if prev.end >= range.start => prev.end = prev.end.max(range.end),
                _ => out.push(range),
            }
        }
        out
    }

    pub(super) fn open_transform_dialog(&mut self, kind: DialogKind) {
        let width = if self.settings.edge_column > 0 {
            self.settings.edge_column
        } else {
            80
        };
        let dest = self.settings.transform_results.get(kind.key()).copied();
        self.transform_dialog.show_dialog(kind, width, dest);
    }

    pub(super) fn run_transform(&mut self, ctx: &egui::Context, op: TransformOp) {
        self.run_transform_op(ctx, op);
        // Only the run a dialog started sends its result elsewhere.
        self.transform_dest = Dest::ThisTab;
    }

    /// Sends a transform's result to a new tab or the clipboard, leaving
    /// the tab as it is. `syntax` is the new tab's language.
    pub(super) fn send_result(
        &mut self,
        ctx: &egui::Context,
        dest: Dest,
        text: String,
        syntax: &str,
    ) {
        match dest {
            Dest::ThisTab => {}
            Dest::Clipboard => {
                let lines = text.lines().count().max(1);
                ctx.copy_text(text);
                self.notify(ctx, t!("result-copied", lines = lines), None);
            }
            Dest::NewTab => {
                self.new_tab();
                let doc = &mut self.docs[self.active];
                doc.editor = crate::editor::Editor::new(&text);
                doc.editor.buffer.mark_modified();
                if self.languages.iter().any(|l| l == syntax) {
                    doc.syntax = syntax.to_owned();
                }
                self.focus_editor = true;
            }
        }
    }

    fn run_transform_op(&mut self, ctx: &egui::Context, op: TransformOp) {
        if self.docs.is_empty() {
            return;
        }
        let unit = self.format_unit(self.active);
        match op {
            TransformOp::FilterLines {
                pattern,
                regex,
                match_case,
                keep,
            } => {
                let matcher = match LineMatcher::new(&pattern, regex, match_case) {
                    Ok(m) => m,
                    Err(e) => {
                        return self.tool_error(
                            ctx,
                            &t!("menu-transform"),
                            &e.message,
                            None,
                            false,
                        );
                    }
                };
                let what = if keep {
                    t!("keep-matching-lines")
                } else {
                    t!("remove-matching-lines")
                };
                let targets = self.line_targets();
                self.transform_ranges(ctx, &what, targets, |t| {
                    Ok(transform::filter_lines(t, &matcher, keep))
                });
            }
            TransformOp::CountLines => {
                let editor = &self.docs[self.active].editor;
                let text: String = self
                    .line_targets()
                    .into_iter()
                    .map(|r| editor.buffer.slice_string(r))
                    .collect::<Vec<_>>()
                    .join("\n");
                let out = transform::count_lines(&text);
                self.new_tab();
                let doc = &mut self.docs[self.active];
                doc.editor = crate::editor::Editor::new(&out);
                doc.editor.buffer.mark_modified();
            }
            TransformOp::AffixLines {
                prefix,
                suffix,
                skip_empty,
            } => {
                let targets = self.line_targets();
                self.transform_ranges(ctx, &t!("affix-lines"), targets, |t| {
                    Ok(transform::affix_lines(t, &prefix, &suffix, skip_empty))
                });
            }
            TransformOp::NumberLines {
                start,
                step,
                separator,
                pad,
                skip_empty,
            } => {
                let targets = self.line_targets();
                self.transform_ranges(ctx, &t!("number-lines"), targets, |t| {
                    Ok(transform::number_lines(
                        t, start, step, &separator, pad, skip_empty,
                    ))
                });
            }
            TransformOp::Reflow { width } => {
                let targets = self.line_targets();
                self.transform_ranges(ctx, &t!("reflow"), targets, |t| {
                    Ok(transform::reflow(t, width))
                });
            }
            TransformOp::Convert { from, to } => {
                let whole = !self.docs[self.active].editor.selections.has_selection();
                let what = t!("convert-to", from = from.name(), to = to.name());
                let changed = self.transform(ctx, &what, |t| convert::convert(t, from, to, &unit));
                let syntax = to.syntax();
                if changed && whole && self.languages.iter().any(|l| l == syntax) {
                    self.docs[self.active].syntax = syntax.to_owned();
                }
            }
            TransformOp::ConvertTo { to, from } => self.run_convert_to(ctx, to, from),
            TransformOp::Hash(kind) => {
                let doc = &mut self.docs[self.active];
                if doc.editor.selections.has_selection() {
                    self.transform(ctx, kind.name(), |t| {
                        Ok(transform::hash(kind, t.as_bytes()))
                    });
                } else {
                    // The whole document: as saved in UTF-8 with its line
                    // endings, copied rather than replacing the text.
                    let text = doc
                        .editor
                        .text_snapshot()
                        .replace('\n', doc.line_ending.as_str());
                    let hash = transform::hash(kind, text.as_bytes());
                    if self.settings.hash_whole_document == crate::settings::HashResult::NewTab {
                        self.new_tab();
                        let doc = &mut self.docs[self.active];
                        doc.editor = crate::editor::Editor::new(&hash);
                        doc.editor.buffer.mark_modified();
                        self.focus_editor = true;
                    } else {
                        ctx.copy_text(hash.clone());
                        self.notify(
                            ctx,
                            t!("hash-copied", kind = kind.name(), hash = hash),
                            None,
                        );
                    }
                }
            }
            TransformOp::HexEncode => {
                self.transform(ctx, &t!("hex-encode"), |t| Ok(transform::hex_encode(t)));
            }
            TransformOp::HexDecode => {
                self.transform(ctx, &t!("hex-decode"), transform::hex_decode);
            }
            TransformOp::JwtDecode => {
                let note = RefCell::new(None);
                let changed = self.transform(ctx, &t!("jwt-decode"), |t| {
                    let jwt = transform::jwt_decode(t, &unit)?;
                    *note.borrow_mut() = Some((jwt.issued, jwt.expires, jwt.expired));
                    Ok(jwt.json)
                });
                if changed && let Some((issued, expires, expired)) = note.into_inner() {
                    let mut text = t!("jwt-not-verified");
                    if let Some(at) = issued {
                        text = format!("{text} {}", t!("jwt-issued", time = at));
                    }
                    if let Some(at) = expires {
                        let when = if expired {
                            t!("jwt-expired", time = at)
                        } else {
                            t!("jwt-expires", time = at)
                        };
                        text = format!("{text} {when}");
                    }
                    self.notify(ctx, text, None);
                }
                if changed && !self.docs[self.active].editor.selections.has_selection() {
                    self.docs[self.active].syntax = "JSON".into();
                }
            }
            TransformOp::Pipe { command } => self.start_pipe(ctx, command),
        }
    }

    fn start_pipe(&mut self, ctx: &egui::Context, command: String) {
        if self.pipe.is_some() {
            return;
        }
        let dest = self.transform_dest;
        let doc = &self.docs[self.active];
        if doc.editor.buffer.read_only && dest == Dest::ThisTab {
            self.transform_dialog.close();
            return self.tool_error(ctx, &t!("pipe-title"), &t!("tool-read-only"), None, false);
        }
        let history = &mut self.settings.pipe_commands;
        history.retain(|c| *c != command);
        history.insert(0, command.clone());
        history.truncate(MAX_COMMANDS);
        let ranges = doc.editor.tool_targets();
        let inputs = ranges
            .iter()
            .map(|r| doc.editor.buffer.slice_string(r.clone()))
            .collect();
        self.pipe = Some(PipeJob {
            command: command.clone(),
            doc: doc.id,
            version: doc.editor.buffer.version(),
            ranges,
            inputs,
            outputs: Vec::new(),
            child: None,
            output: None,
            dest,
        });
        self.transform_dialog.running = Some(command);
        self.transform_dialog.error = None;
        // Opened from a macro or plugin, the dialog shows the progress too.
        if !self.transform_dialog.is_open() {
            self.open_transform_dialog(DialogKind::Pipe);
            self.transform_dialog.running = self.pipe.as_ref().map(|p| p.command.clone());
        }
        self.poll_pipe(ctx);
    }

    /// Checks on the running command; starts the next selection's run, or
    /// applies every output once all have finished.
    pub(super) fn poll_pipe(&mut self, ctx: &egui::Context) {
        let Some(job) = &mut self.pipe else {
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        let failed = |app: &mut Self, message: String| {
            if let Some(mut child) = app.pipe.take().and_then(|j| j.child) {
                let _ = child.kill();
                let _ = child.wait();
            }
            app.transform_dialog.running = None;
            app.transform_dialog.error = Some(message);
        };
        if job.child.is_none() {
            let Some(index) = self.docs.iter().position(|d| d.id == job.doc) else {
                return failed(self, t!("pipe-tab-closed"));
            };
            let doc = &self.docs[index];
            let input = job.inputs[job.outputs.len()].clone();
            let dir = doc
                .path
                .as_deref()
                .and_then(|p| p.parent())
                .map(|p| p.to_owned())
                .or_else(dirs::home_dir);
            match spawn(&job.command, dir.as_deref(), doc.path.as_deref(), input) {
                Ok((child, rx)) => {
                    job.child = Some(child);
                    job.output = Some(rx);
                }
                Err(e) => return failed(self, t!("pipe-start-failed", error = e)),
            }
        }
        let Some((out, err)) = job.output.as_ref().and_then(|rx| rx.try_recv().ok()) else {
            return;
        };
        let status = job.child.take().and_then(|mut c| c.wait().ok());
        job.output = None;
        let ok = status.is_some_and(|s| s.success());
        if !ok {
            let code = status
                .and_then(|s| s.code())
                .map_or_else(|| "?".to_owned(), |c| c.to_string());
            let stderr = String::from_utf8_lossy(&err).trim().to_owned();
            let message = if stderr.is_empty() {
                t!("pipe-failed", code = code)
            } else {
                format!("{}\n{stderr}", t!("pipe-failed", code = code))
            };
            return failed(self, message);
        }
        let input = &job.inputs[job.outputs.len()];
        job.outputs.push(pipe_output(input, &out));
        if job.outputs.len() < job.inputs.len() {
            return;
        }
        let job = self.pipe.take().expect("running");
        self.transform_dialog.close();
        let Some(index) = self.docs.iter().position(|d| d.id == job.doc) else {
            return;
        };
        if job.dest != Dest::ThisTab {
            // The tab is left as it was, even if it changed meanwhile.
            let syntax = self.docs[index].syntax.clone();
            let text = job.outputs.join("\n");
            return self.send_result(ctx, job.dest, text, &syntax);
        }
        if self.docs[index].editor.buffer.version() != job.version {
            return self.tool_error(
                ctx,
                &t!("pipe-title"),
                &t!("pipe-text-changed"),
                None,
                false,
            );
        }
        let whole = !self.docs[index].editor.selections.has_selection();
        let edits: Vec<(Range<usize>, String)> = job
            .ranges
            .into_iter()
            .zip(job.outputs)
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

    /// Stops the running command; the text stays as it was.
    pub(super) fn stop_pipe(&mut self) {
        if let Some(mut child) = self.pipe.take().and_then(|j| j.child) {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.transform_dialog.running = None;
    }

    /// The Transform dialogs, when one is open, and the green and red line
    /// marks of Keep or Remove Matching Lines.
    pub(super) fn transform_dialog_ui(&mut self, ctx: &egui::Context) {
        if !self.transform_dialog.is_open() || self.docs.is_empty() {
            for doc in &mut self.docs {
                doc.editor.line_preview = None;
            }
            return;
        }
        let palette = self.palette();
        let history = self.settings.pipe_commands.clone();
        let targets = self.line_targets();
        let editor = &self.docs[self.active].editor;
        let doc = &self.docs[self.active];
        let selection = editor.selections.has_selection();
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            doc.id.hash(&mut h);
            editor.buffer.version().hash(&mut h);
            targets.hash(&mut h);
            h.finish()
        };
        let line_ranges: Vec<Range<usize>> = targets
            .iter()
            .map(|r| {
                let first = editor.buffer.char_to_line(r.start);
                let mut last = editor.buffer.char_to_line(r.end);
                // The empty "line" after a final line break isn't one.
                if last > first && editor.buffer.line_start(last) == r.end {
                    last -= 1;
                }
                first..last + 1
            })
            .collect();
        let lines = line_ranges.iter().map(|r| r.len()).sum();
        let text = || {
            targets
                .iter()
                .map(|r| editor.buffer.slice_string(r.clone()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let dir = doc
            .path
            .as_deref()
            .and_then(|p| p.parent())
            .map(|p| p.to_owned())
            .or_else(dirs::home_dir);
        let source = Source {
            key,
            text: &text,
            lines,
            selection,
            dir,
            file: doc.path.clone(),
        };
        let kind = self.transform_dialog.kind;
        let action = self
            .transform_dialog
            .ui(ctx, &palette, &history, &source, self.editor_rect);
        let marks = self.transform_dialog.line_preview();
        let active = self.active;
        for (i, doc) in self.docs.iter_mut().enumerate() {
            doc.editor.line_preview = match (&marks, i == active) {
                (Some((matcher, keep)), true) => Some(crate::editor::LinePreview {
                    matcher: matcher.clone(),
                    keep: *keep,
                    lines: line_ranges.clone(),
                }),
                _ => None,
            };
        }
        match action {
            Some(DialogAction::Run(op, dest)) => {
                if let Some(kind) = kind {
                    self.settings
                        .transform_results
                        .insert(kind.key().to_owned(), dest);
                }
                let pipe = matches!(op, TransformOp::Pipe { .. });
                self.transform_dest = dest;
                self.pending.push(crate::actions::Action::Transform(op));
                if !pipe {
                    self.focus_editor = true;
                }
            }
            Some(DialogAction::Stop) => self.stop_pipe(),
            None => {}
        }
    }
}
