//! Running the text tools against the active document, and the JSON tree,
//! XPath panel, notices and status-bar XPath that go with them.

use std::cell::RefCell;

use egui::{Frame, Margin, Ui};

use super::EttinApp;
use crate::actions::Action;
use crate::text_tools_ui::{NoticeResponse, TreeAction, XPathAction, XPathRow};
use crate::tools::json::{self, JsonOp};
use crate::tools::mime::MimeOp;
use crate::tools::xml::{self, Outline, XmlOp};
use crate::tools::xpath::{self, Outcome};
use crate::tools::{ToolResult, floor_char_boundary, js, line_col};

/// Syntaxes that are XML for the status-bar XPath and the check on save.
pub(super) fn is_xml(syntax: &str) -> bool {
    syntax == "XML"
}

/// Documents larger than this (in bytes) get no XPath in the status bar.
const OUTLINE_MAX_BYTES: usize = 8 << 20;
/// While typing, the status-bar XPath is worked out at most this often (seconds).
const OUTLINE_DELAY: f64 = 0.3;

/// The element outline behind the status-bar XPath.
pub(super) struct OutlineCache {
    doc: u64,
    version: u64,
    built_at: f64,
    outline: Option<Outline>,
}

/// Formatters keep a final line break when the text had one.
fn keep_final_newline(input: &str, mut output: String) -> String {
    if input.ends_with('\n') && !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

/// A value on one line, at most `max` chars.
fn one_line(s: &str, max: usize) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    let joined = words.join(" ");
    if joined.chars().count() > max {
        joined.chars().take(max).collect::<String>() + "…"
    } else {
        joined
    }
}

impl EttinApp {
    pub(super) fn notify(&mut self, ctx: &egui::Context, text: String, jump: Option<(u64, usize)>) {
        self.notice = Some(crate::text_tools_ui::Notice {
            text,
            error: false,
            since: Some(ctx.input(|i| i.time)),
            jump,
        });
    }

    /// Reports a tool error, with the line and column when it has a place,
    /// and (with `move_caret`) puts the caret there.
    pub(super) fn tool_error(
        &mut self,
        ctx: &egui::Context,
        what: &str,
        message: &str,
        pos: Option<usize>,
        move_caret: bool,
    ) {
        let doc = &mut self.docs[self.active];
        let text = match pos {
            Some(p) => {
                let line = doc.editor.buffer.char_to_line(p);
                let col = p - doc.editor.buffer.line_start(line);
                if move_caret {
                    doc.editor.select_range(p..p);
                }
                t!(
                    "tool-error-at",
                    tool = what,
                    line = line + 1,
                    column = col + 1,
                    message = message
                )
            }
            None => t!("tool-error", tool = what, message = message),
        };
        let jump = pos.map(|p| (doc.id, p));
        self.notice = Some(crate::text_tools_ui::Notice {
            text,
            error: true,
            since: Some(ctx.input(|i| i.time)),
            jump,
        });
    }

    /// Runs `f` on each selection, or on the whole text when nothing is
    /// selected, and replaces it with the result as one undo step. Nothing
    /// changes if any part fails. Returns true if the text changed.
    fn transform(
        &mut self,
        ctx: &egui::Context,
        what: &str,
        f: impl Fn(&str) -> ToolResult,
    ) -> bool {
        let editor = &self.docs[self.active].editor;
        if editor.buffer.read_only {
            self.tool_error(ctx, what, &t!("tool-read-only"), None, false);
            return false;
        }
        let whole = !editor.selections.has_selection();
        let mut edits = Vec::new();
        for range in editor.tool_targets() {
            let text = editor.buffer.slice_string(range.clone());
            match f(&text) {
                Ok(new) => {
                    if new != text {
                        edits.push((range, new));
                    }
                }
                Err(e) => {
                    let pos = e.offset.map(|o| {
                        range.start
                            + text[..floor_char_boundary(&text, o.min(text.len()))]
                                .chars()
                                .count()
                    });
                    self.tool_error(ctx, what, &e.message, pos, true);
                    return false;
                }
            }
        }
        let changed = !edits.is_empty();
        self.docs[self.active]
            .editor
            .replace_ranges(ctx, edits, !whole);
        changed
    }

    /// Checks each selection (or the whole text) with `check`; reports the
    /// first problem or says all is well.
    fn validate(
        &mut self,
        ctx: &egui::Context,
        what: &str,
        ok: &str,
        check: impl Fn(&str) -> Result<(), crate::tools::ToolError>,
    ) {
        let editor = &self.docs[self.active].editor;
        let selection = editor.selections.has_selection();
        for range in editor.tool_targets() {
            let text = editor.buffer.slice_string(range.clone());
            if let Err(e) = check(&text) {
                let pos = e.offset.map(|o| {
                    range.start
                        + text[..floor_char_boundary(&text, o.min(text.len()))]
                            .chars()
                            .count()
                });
                self.tool_error(ctx, what, &e.message, pos, true);
                return;
            }
        }
        let text = if selection {
            t!("tool-ok-selection", result = ok)
        } else {
            ok.to_owned()
        };
        self.notify(ctx, text, None);
    }

    pub(super) fn run_text_tool(&mut self, ctx: &egui::Context, action: Action) {
        let unit = self.docs[self.active].editor.indent_unit();
        match action {
            Action::Mime(op) => {
                self.transform(ctx, &t!("menu-mime-tools"), |t| {
                    let out = op.run(t)?;
                    // Encoders treat a final line break as data; SAML output is a document.
                    Ok(if op == MimeOp::SamlDecode {
                        keep_final_newline(t, out)
                    } else {
                        out
                    })
                });
            }
            Action::Json(op @ (JsonOp::Format | JsonOp::Minify)) => {
                let format = op == JsonOp::Format;
                let fell_back = RefCell::new(false);
                // JSON when the text is JSON, JavaScript otherwise.
                let what = if format {
                    t!("json-format")
                } else {
                    t!("json-minify")
                };
                self.transform(ctx, &what, |t| {
                    let out = match json::parse(t) {
                        Ok(doc) => doc.write(t, format.then_some(unit.as_str()), false),
                        Err(_) => {
                            *fell_back.borrow_mut() = true;
                            if format {
                                js::format(t, &unit)?
                            } else {
                                js::minify(t)?
                            }
                        }
                    };
                    Ok(keep_final_newline(t, out))
                });
                if fell_back.into_inner()
                    && self.docs[self.active].syntax == "JSON"
                    && self.notice.as_ref().is_none_or(|n| !n.error)
                {
                    self.notify(ctx, t!("json-handled-as-js"), None);
                }
            }
            Action::Json(JsonOp::Validate) => self.validate(ctx, "JSON", &t!("json-valid"), |t| {
                json::parse(t).map(|_| ())
            }),
            Action::Json(JsonOp::SortKeys) => {
                self.transform(ctx, &t!("json-sort-keys"), |t| {
                    Ok(keep_final_newline(t, json::sort_keys(t, &unit)?))
                });
            }
            Action::Xml(op) => match op {
                XmlOp::PrettyPrint | XmlOp::PrettyPrintAttributes => {
                    let attrs = op == XmlOp::PrettyPrintAttributes;
                    self.transform(ctx, &t!("xml-pretty-print"), |t| {
                        Ok(keep_final_newline(t, xml::pretty_print(t, &unit, attrs)?))
                    });
                }
                XmlOp::Linearize => {
                    self.transform(ctx, &t!("xml-linearize"), |t| {
                        Ok(keep_final_newline(t, xml::linearize(t)?))
                    });
                }
                XmlOp::CheckSyntax => self.validate(ctx, "XML", &t!("xml-well-formed"), xml::check),
                XmlOp::ValidateSchema => self.validate_schema(ctx, false),
                XmlOp::ChooseSchema => self.validate_schema(ctx, true),
                XmlOp::XslTransform => self.xsl_transform(ctx),
                XmlOp::Escape => {
                    self.transform(ctx, &t!("xml-escape"), |t| Ok(xml::escape(t)));
                }
                XmlOp::Unescape => {
                    self.transform(ctx, &t!("xml-unescape"), |t| Ok(xml::unescape(t)));
                }
            },
            Action::ToggleJsonTree => {
                self.json_tree.open = !self.json_tree.open;
                self.json_tree_reveal = self.json_tree.open;
            }
            Action::ToggleXPathPanel => {
                if self.xpath.open && !self.xpath.focus {
                    self.xpath.open = false;
                } else {
                    self.xpath.show_panel();
                }
            }
            Action::CopyXPath => match self.current_xpath(ctx) {
                Some(path) => {
                    ctx.copy_text(path.clone());
                    self.notify(ctx, t!("copied-text", text = path), None);
                }
                None => {
                    let syntax = &self.docs[self.active].syntax;
                    let message = if is_xml(syntax) {
                        t!("xpath-no-element")
                    } else {
                        t!("xpath-not-xml")
                    };
                    self.tool_error(ctx, &t!("copy-xpath"), &message, None, false);
                }
            },
            _ => {}
        }
    }

    /// After saving: checks XML when that's turned on.
    pub(super) fn check_after_save(&mut self, index: usize) {
        if !self.settings.xml_check_on_save || !is_xml(&self.docs[index].syntax) {
            return;
        }
        let text = self.docs[index].editor.text_snapshot();
        if let Err(e) = xml::check(&text) {
            let doc = &self.docs[index];
            let (line, col) = line_col(&text, e.offset.unwrap_or(0));
            let pos = e.offset.map(|o| doc.editor.buffer.rope().byte_to_char(o));
            let text = t!(
                "saved-not-well-formed",
                name = doc.title(),
                line = line,
                column = col,
                message = e.message
            );
            let jump = pos.map(|p| (doc.id, p));
            self.notice = Some(crate::text_tools_ui::Notice {
                text,
                error: true,
                since: None,
                jump,
            });
        }
    }

    /// The XPath of the element at the caret, for XML documents.
    pub(super) fn current_xpath(&mut self, ctx: &egui::Context) -> Option<String> {
        let doc = self.docs.get_mut(self.active)?;
        if !is_xml(&doc.syntax) || doc.editor.buffer.rope().len_bytes() > OUTLINE_MAX_BYTES {
            return None;
        }
        let version = doc.editor.buffer.version();
        let now = ctx.input(|i| i.time);
        let fresh = self
            .xml_outline
            .as_ref()
            .is_some_and(|c| c.doc == doc.id && c.version == version);
        if !fresh {
            let recent = self
                .xml_outline
                .as_ref()
                .is_some_and(|c| c.doc == doc.id && now - c.built_at < OUTLINE_DELAY);
            if recent {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(OUTLINE_DELAY));
            } else {
                let text = doc.editor.text_snapshot();
                self.xml_outline = Some(OutlineCache {
                    doc: doc.id,
                    version,
                    built_at: now,
                    outline: Outline::new(&text),
                });
            }
        }
        let cache = self.xml_outline.as_ref().filter(|c| c.doc == doc.id)?;
        let head = doc.editor.selections.primary().head;
        let rope = doc.editor.buffer.rope();
        let byte = rope.char_to_byte(head.min(rope.len_chars()));
        cache.outline.as_ref()?.path_at(byte)
    }

    fn evaluate_xpath(&mut self) {
        let doc = &mut self.docs[self.active];
        let text = doc.editor.text_snapshot();
        let panel = &mut self.xpath;
        panel.rows.clear();
        panel.value = None;
        panel.error = None;
        panel.source = Some((doc.id, doc.editor.buffer.version()));
        if panel.expr.trim().is_empty() {
            panel.error = Some(t!("xpath-type-expression"));
            return;
        }
        panel.remember();
        let parsed = match xml::parse_strict(&text) {
            Ok(d) => d,
            Err(e) => {
                let (line, col) = line_col(&text, e.offset.unwrap_or(0));
                panel.error = Some(t!(
                    "xpath-not-well-formed",
                    line = line,
                    column = col,
                    message = e.message.trim_end_matches('.').to_owned()
                ));
                return;
            }
        };
        let rope = doc.editor.buffer.rope();
        match xpath::evaluate(&parsed, &panel.expr) {
            Ok(Outcome::Nodes(nodes)) => {
                panel.rows = nodes
                    .into_iter()
                    .map(|n| {
                        let start = rope.byte_to_char(n.range.start);
                        XPathRow {
                            range: start..rope.byte_to_char(n.range.end),
                            line: rope.char_to_line(start),
                            label: n.label,
                            value: one_line(&n.value, 200),
                            full: n.value,
                        }
                    })
                    .collect();
            }
            Ok(Outcome::Number(n)) => panel.value = Some(xpath::number_to_string(n)),
            Ok(Outcome::String(s)) => panel.value = Some(s),
            Ok(Outcome::Boolean(b)) => panel.value = Some(b.to_string()),
            Err(e) => {
                panel.error = Some(match e.offset {
                    Some(o) => t!(
                        "xpath-error-at",
                        message = e.message,
                        position = panel.expr
                            [..floor_char_boundary(&panel.expr, o.min(panel.expr.len()))]
                            .chars()
                            .count()
                            + 1
                    ),
                    None => e.message,
                });
            }
        }
    }

    /// The JSON tree on the right, when open.
    pub(super) fn json_tree_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if !self.json_tree.open || self.docs.is_empty() {
            return;
        }
        let palette = self.palette();
        let doc = &mut self.docs[self.active];
        let now = ctx.input(|i| i.time);
        let waiting = self
            .json_tree
            .update(doc.id, doc.editor.buffer.version(), now, || {
                doc.editor.text_snapshot()
            });
        if waiting {
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }
        if std::mem::take(&mut self.json_tree_reveal) {
            let rope = doc.editor.buffer.rope();
            let head = doc.editor.selections.primary().head.min(rope.len_chars());
            self.json_tree.reveal(rope.char_to_byte(head));
        }
        let mut action = None;
        egui::Panel::right("json-tree")
            .resizable(true)
            .default_size(340.0)
            .min_size(200.0)
            .frame(
                Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| action = self.json_tree.ui(ui, &palette));
        match action {
            Some(TreeAction::Select(bytes)) => {
                let editor = &mut self.docs[self.active].editor;
                let rope = editor.buffer.rope();
                let len = rope.len_bytes();
                let range =
                    rope.byte_to_char(bytes.start.min(len))..rope.byte_to_char(bytes.end.min(len));
                editor.select_range(range);
                self.focus_editor = true;
            }
            Some(TreeAction::Copy(text)) => ctx.copy_text(text),
            None => {}
        }
    }

    /// The XPath panel at the bottom, when open.
    pub(super) fn xpath_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if !self.xpath.open || self.docs.is_empty() {
            return;
        }
        let palette = self.palette();
        let doc = &self.docs[self.active];
        let current = Some((doc.id, doc.editor.buffer.version()));
        let mut action = None;
        egui::Panel::bottom("xpath")
            .resizable(true)
            .default_size(200.0)
            .min_size(70.0)
            .frame(
                Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| action = self.xpath.ui(ui, &palette, current));
        match action {
            Some(XPathAction::Evaluate) => self.evaluate_xpath(),
            Some(XPathAction::Select(range)) => {
                if let Some((id, _)) = self.xpath.source
                    && let Some(i) = self.docs.iter().position(|d| d.id == id)
                {
                    self.select(i);
                    self.docs[i].editor.select_range(range);
                    self.focus_editor = true;
                }
            }
            Some(XPathAction::Copy(text)) => ctx.copy_text(text),
            None => {}
        }
    }

    pub(super) fn notice_ui(&mut self, ctx: &egui::Context) {
        let palette = self.palette();
        let Some(notice) = &mut self.notice else {
            return;
        };
        notice.since.get_or_insert(ctx.input(|i| i.time));
        match notice.ui(ctx, &palette) {
            Some(NoticeResponse::Close) => self.notice = None,
            Some(NoticeResponse::Jump(id, pos)) => {
                if let Some(i) = self.docs.iter().position(|d| d.id == id) {
                    self.select(i);
                    self.docs[i].editor.select_range(pos..pos);
                    self.focus_editor = true;
                }
                self.notice = None;
            }
            None => {}
        }
    }
}
