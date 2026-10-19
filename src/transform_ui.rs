//! The Transform menu's dialogs: Filter Through Command, Keep or Remove
//! Matching Lines, Add Prefix and Suffix, Number Lines and Reflow.

use egui::{Align, Id, Key, Layout, Margin, RichText, Ui, Vec2};
use serde::{Deserialize, Serialize};

use crate::theme::Palette;
use crate::tools::transform::{LineMatcher, NumberPad, TransformOp};

/// Which dialog a Transform menu item opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogKind {
    Pipe,
    FilterLines { keep: bool },
    AffixLines,
    NumberLines,
    Reflow,
}

/// What the dialog asks the app to do.
pub enum DialogAction {
    Run(TransformOp),
    /// Stop the running command.
    Stop,
}

/// A filter's pattern, regex and match-case settings, and how many lines
/// matched (or why the pattern is wrong).
type MatchCount = ((String, bool, bool), Result<(usize, usize), String>);

/// How many recent commands Filter Through Command remembers.
pub const MAX_COMMANDS: usize = 15;

pub struct TransformDialog {
    pub kind: Option<DialogKind>,
    focus: bool,
    pub error: Option<String>,
    /// The command running now, while Filter Through Command waits for it.
    pub running: Option<String>,
    command: String,
    pattern: String,
    regex: bool,
    match_case: bool,
    keep: bool,
    /// The settings and result of the last match count.
    count: Option<MatchCount>,
    prefix: String,
    suffix: String,
    skip_empty: bool,
    start: String,
    step: String,
    separator: String,
    pad: NumberPad,
    width: String,
}

impl Default for TransformDialog {
    fn default() -> Self {
        Self {
            kind: None,
            focus: false,
            error: None,
            running: None,
            command: String::new(),
            pattern: String::new(),
            regex: false,
            match_case: false,
            keep: true,
            count: None,
            prefix: String::new(),
            suffix: String::new(),
            skip_empty: true,
            start: "1".into(),
            step: "1".into(),
            separator: ". ".into(),
            pad: NumberPad::None,
            width: "80".into(),
        }
    }
}

fn field(ui: &mut Ui, label: &str, value: &mut String, width: f32) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 20.0], egui::Label::new(label));
        ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(width)
                .margin(Margin::symmetric(6, 3)),
        )
    })
    .inner
}

impl TransformDialog {
    /// Opens a dialog; `width` is the long-line column, for Reflow.
    pub fn show_dialog(&mut self, kind: DialogKind, width: usize) {
        if self.running.is_some() {
            return;
        }
        if let DialogKind::FilterLines { keep } = kind {
            self.keep = keep;
        }
        if kind == DialogKind::Reflow {
            self.width = width.to_string();
        }
        self.kind = Some(kind);
        self.focus = true;
        self.error = None;
        self.count = None;
    }

    pub fn is_open(&self) -> bool {
        self.kind.is_some()
    }

    pub fn close(&mut self) {
        self.kind = None;
        self.running = None;
    }

    /// Draws the open dialog. `text` gives the lines a filter would look at,
    /// for its match count; `history` is the recent commands.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        palette: &Palette,
        history: &[String],
        text: &dyn Fn() -> String,
    ) -> Option<DialogAction> {
        let kind = self.kind?;
        let mut result = None;
        let mut close = false;
        let mut enter = false;
        let response = egui::Modal::new(Id::new("transform-dialog")).show(ctx, |ui| {
            ui.set_width(if kind == DialogKind::Pipe {
                460.0
            } else {
                360.0
            });
            let (title, hint) = match kind {
                DialogKind::Pipe => (t!("pipe-title"), t!("pipe-hint")),
                DialogKind::FilterLines { .. } => {
                    (t!("filter-lines-title"), t!("filter-lines-hint"))
                }
                DialogKind::AffixLines => (t!("affix-lines-title"), t!("affix-lines-hint")),
                DialogKind::NumberLines => (t!("number-lines-title"), t!("number-lines-hint")),
                DialogKind::Reflow => (t!("reflow-title"), t!("reflow-hint")),
            };
            ui.heading(title);
            ui.add_space(4.0);
            ui.label(RichText::new(hint).size(12.0).color(palette.text_dim));
            ui.add_space(6.0);
            let first = match kind {
                DialogKind::Pipe => self.pipe_ui(ui, history),
                DialogKind::FilterLines { .. } => self.filter_ui(ui, palette, text),
                DialogKind::AffixLines => {
                    let r = field(ui, &t!("affix-prefix"), &mut self.prefix, 200.0);
                    {
                        let r = field(ui, &t!("affix-suffix"), &mut self.suffix, 200.0);
                        enter |= submitted(ui, &r);
                    }
                    ui.checkbox(&mut self.skip_empty, t!("skip-empty-lines"));
                    r
                }
                DialogKind::NumberLines => {
                    let r = field(ui, &t!("initial-number"), &mut self.start, 80.0);
                    {
                        let r = field(ui, &t!("increase-by"), &mut self.step, 80.0);
                        enter |= submitted(ui, &r);
                    }
                    {
                        let r = field(ui, &t!("number-separator"), &mut self.separator, 80.0);
                        enter |= submitted(ui, &r);
                    }
                    ui.horizontal(|ui| {
                        for (p, label) in [
                            (NumberPad::None, "number-pad-none"),
                            (NumberPad::Spaces, "number-pad-spaces"),
                            (NumberPad::Zeros, "number-pad-zeros"),
                        ] {
                            ui.radio_value(&mut self.pad, p, crate::i18n::tr(label));
                        }
                    });
                    ui.checkbox(&mut self.skip_empty, t!("skip-empty-lines"));
                    r
                }
                DialogKind::Reflow => field(ui, &t!("reflow-width"), &mut self.width, 80.0),
            };
            if std::mem::take(&mut self.focus) {
                first.request_focus();
            }
            enter |= submitted(ui, &first);
            if let Some(e) = &self.error {
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(e).color(palette.modified).size(12.0));
                    });
            }
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 26.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if let Some(cmd) = &self.running {
                        if ui.button(t!("stop")).clicked() {
                            result = Some(DialogAction::Stop);
                        }
                        ui.spinner();
                        ui.label(
                            RichText::new(t!("pipe-running", command = cmd.clone()))
                                .color(palette.text_dim),
                        );
                        return;
                    }
                    if ui.button(t!("cancel")).clicked() {
                        close = true;
                    }
                    let ok = match kind {
                        DialogKind::Pipe => t!("pipe-run"),
                        _ => t!("ok"),
                    };
                    if ui.button(ok).clicked() || enter {
                        match self.request(kind) {
                            Ok(op) => {
                                result = Some(DialogAction::Run(op));
                                // The command dialog stays open until the command ends.
                                close = kind != DialogKind::Pipe;
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                },
            );
        });
        if response.should_close() && self.running.is_none() {
            close = true;
        }
        if close {
            self.kind = None;
        }
        result
    }

    fn pipe_ui(&mut self, ui: &mut Ui, history: &[String]) -> egui::Response {
        let r = ui.add_enabled(
            self.running.is_none(),
            egui::TextEdit::singleline(&mut self.command)
                .hint_text("sort -u")
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(6, 4)),
        );
        if !history.is_empty() {
            ui.add_space(4.0);
            ui.add_enabled_ui(self.running.is_none(), |ui| {
                egui::ComboBox::from_id_salt("pipe-history")
                    .selected_text(t!("pipe-recent"))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for cmd in history {
                            if ui.selectable_label(false, cmd).clicked() {
                                self.command = cmd.clone();
                                self.focus = true;
                            }
                        }
                    });
            });
        }
        r
    }

    fn filter_ui(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        text: &dyn Fn() -> String,
    ) -> egui::Response {
        let r = ui.add(
            egui::TextEdit::singleline(&mut self.pattern)
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(6, 4)),
        );
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.match_case, t!("match-case"));
            ui.checkbox(&mut self.regex, t!("regular-expression"));
        });
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.keep, true, t!("filter-keep"));
            ui.radio_value(&mut self.keep, false, t!("filter-remove"));
        });
        let key = (self.pattern.clone(), self.regex, self.match_case);
        if self.count.as_ref().is_none_or(|(k, _)| *k != key) {
            let counted = if self.pattern.is_empty() {
                Ok((0, 0))
            } else {
                LineMatcher::new(&self.pattern, self.regex, self.match_case)
                    .map(|m| crate::tools::transform::count_matching(&text(), &m))
                    .map_err(|e| e.message)
            };
            self.count = Some((key, counted));
        }
        if let Some((_, counted)) = &self.count {
            let line = match counted {
                Ok(_) if self.pattern.is_empty() => String::new(),
                Ok((hits, total)) => t!("filter-count", matches = *hits, lines = *total),
                Err(e) => e.lines().last().unwrap_or("").to_owned(),
            };
            ui.label(RichText::new(line).size(12.0).color(palette.text_dim));
        }
        r
    }

    fn request(&self, kind: DialogKind) -> Result<TransformOp, String> {
        let num = |s: &str, what: &str| {
            s.trim()
                .parse::<i64>()
                .map_err(|_| t!("must-be-whole-number", field = what))
        };
        Ok(match kind {
            DialogKind::Pipe => {
                let command = self.command.trim().to_owned();
                if command.is_empty() {
                    return Err(t!("pipe-no-command"));
                }
                TransformOp::Pipe { command }
            }
            DialogKind::FilterLines { .. } => {
                if self.pattern.is_empty() {
                    return Err(t!("filter-no-pattern"));
                }
                LineMatcher::new(&self.pattern, self.regex, self.match_case)
                    .map_err(|e| e.message)?;
                TransformOp::FilterLines {
                    pattern: self.pattern.clone(),
                    regex: self.regex,
                    match_case: self.match_case,
                    keep: self.keep,
                }
            }
            DialogKind::AffixLines => TransformOp::AffixLines {
                prefix: self.prefix.clone(),
                suffix: self.suffix.clone(),
                skip_empty: self.skip_empty,
            },
            DialogKind::NumberLines => TransformOp::NumberLines {
                start: num(&self.start, &t!("initial-number"))?,
                step: num(&self.step, &t!("increase-by"))?,
                separator: self.separator.clone(),
                pad: self.pad,
                skip_empty: self.skip_empty,
            },
            DialogKind::Reflow => {
                let width = num(&self.width, &t!("reflow-width"))?;
                TransformOp::Reflow {
                    width: width.clamp(1, 10_000) as usize,
                }
            }
        })
    }
}

fn submitted(ui: &Ui, r: &egui::Response) -> bool {
    r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
}
