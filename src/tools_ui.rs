//! Dialogs for the editing tools: the column editor and the macro dialogs.

use egui::{Align, Id, Key, Layout, Margin, RichText, Ui, Vec2};

use crate::theme::Palette;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberFormat {
    Dec,
    Hex,
    Oct,
    Bin,
}

/// What the column editor inserts.
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnInsert {
    Text(String),
    Numbers {
        initial: i64,
        step: i64,
        /// How many times each number repeats before the next one.
        repeat: usize,
        leading_zeros: bool,
        format: NumberFormat,
    },
}

impl ColumnInsert {
    /// The text for each of `count` rows.
    pub fn values(&self, count: usize) -> Vec<String> {
        match self {
            ColumnInsert::Text(t) => vec![t.clone(); count],
            ColumnInsert::Numbers {
                initial,
                step,
                repeat,
                leading_zeros,
                format,
            } => {
                let repeat = (*repeat).max(1) as i64;
                let fmt = |n: i64| {
                    let sign = if n < 0 { "-" } else { "" };
                    let a = n.unsigned_abs();
                    let digits = match format {
                        NumberFormat::Dec => a.to_string(),
                        NumberFormat::Hex => format!("{a:X}"),
                        NumberFormat::Oct => format!("{a:o}"),
                        NumberFormat::Bin => format!("{a:b}"),
                    };
                    (sign, digits)
                };
                let nums: Vec<(&str, String)> = (0..count as i64)
                    .map(|i| fmt(initial + step * (i / repeat)))
                    .collect();
                let width = nums.iter().map(|(_, d)| d.len()).max().unwrap_or(0);
                nums.into_iter()
                    .map(|(sign, d)| {
                        if *leading_zeros {
                            format!("{sign}{d:0>width$}")
                        } else {
                            // Right-align so the column stays straight.
                            format!(
                                "{:>width$}",
                                format!("{sign}{d}"),
                                width = width + sign.len()
                            )
                        }
                    })
                    .collect()
            }
        }
    }
}

pub struct ColumnDialog {
    pub open: bool,
    numbers: bool,
    text: String,
    initial: String,
    step: String,
    repeat: String,
    leading_zeros: bool,
    format: NumberFormat,
    focus: bool,
    error: Option<String>,
}

impl Default for ColumnDialog {
    fn default() -> Self {
        Self {
            open: false,
            numbers: false,
            text: String::new(),
            initial: "1".into(),
            step: "1".into(),
            repeat: "1".into(),
            leading_zeros: false,
            format: NumberFormat::Dec,
            focus: false,
            error: None,
        }
    }
}

fn field(ui: &mut Ui, label: &str, value: &mut String) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 20.0], egui::Label::new(label));
        ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(120.0)
                .margin(Margin::symmetric(6, 3)),
        )
    })
    .inner
}

impl ColumnDialog {
    pub fn show_dialog(&mut self) {
        self.open = true;
        self.focus = true;
        self.error = None;
    }

    /// Returns what to insert once confirmed.
    pub fn ui(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<ColumnInsert> {
        if !self.open {
            return None;
        }
        let mut result = None;
        let mut close = false;
        let response = egui::Modal::new(Id::new("column-editor")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.heading(t!("column-editor-title"));
            ui.add_space(4.0);
            ui.label(
                RichText::new(t!("column-editor-hint"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
            ui.add_space(6.0);
            ui.radio_value(&mut self.numbers, false, t!("text-to-insert"));
            let mut enter = false;
            ui.add_enabled_ui(!self.numbers, |ui| {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.text)
                        .desired_width(f32::INFINITY)
                        .margin(Margin::symmetric(6, 4)),
                );
                if self.focus && !self.numbers {
                    r.request_focus();
                    self.focus = false;
                }
                enter |= r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            });
            ui.add_space(6.0);
            ui.radio_value(&mut self.numbers, true, t!("numbers-to-insert"));
            ui.add_enabled_ui(self.numbers, |ui| {
                ui.indent("numbers", |ui| {
                    field(ui, &t!("initial-number"), &mut self.initial);
                    field(ui, &t!("increase-by"), &mut self.step);
                    field(ui, &t!("repeat"), &mut self.repeat);
                    ui.checkbox(&mut self.leading_zeros, t!("leading-zeros"));
                    ui.horizontal(|ui| {
                        for (f, label) in [
                            (NumberFormat::Dec, "format-dec"),
                            (NumberFormat::Hex, "format-hex"),
                            (NumberFormat::Oct, "format-oct"),
                            (NumberFormat::Bin, "format-bin"),
                        ] {
                            ui.radio_value(&mut self.format, f, crate::i18n::tr(label));
                        }
                    });
                });
            });
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(palette.modified).size(12.0));
            }
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 26.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if ui.button(t!("cancel")).clicked() {
                        close = true;
                    }
                    if ui.button(t!("insert")).clicked() || enter {
                        match self.request() {
                            Ok(r) => {
                                result = Some(r);
                                close = true;
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                },
            );
        });
        if close || response.should_close() {
            self.open = false;
        }
        result
    }

    fn request(&self) -> Result<ColumnInsert, String> {
        if !self.numbers {
            return Ok(ColumnInsert::Text(self.text.clone()));
        }
        let num = |s: &str, what: &str| {
            s.trim()
                .parse::<i64>()
                .map_err(|_| t!("must-be-whole-number", field = what))
        };
        Ok(ColumnInsert::Numbers {
            initial: num(&self.initial, &t!("initial-number"))?,
            step: num(&self.step, &t!("increase-by"))?,
            repeat: num(&self.repeat, &t!("repeat"))?.max(1) as usize,
            leading_zeros: self.leading_zeros,
            format: self.format,
        })
    }
}

/// How many times to run a macro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunTimes {
    Count(usize),
    /// Until the caret reaches the end of the document or stops moving.
    ToEnd,
}

pub struct RunMacroDialog {
    pub open: bool,
    to_end: bool,
    count: String,
    focus: bool,
}

impl Default for RunMacroDialog {
    fn default() -> Self {
        Self {
            open: false,
            to_end: false,
            count: "1".into(),
            focus: false,
        }
    }
}

impl RunMacroDialog {
    pub fn show_dialog(&mut self) {
        self.open = true;
        self.focus = true;
    }

    pub fn ui(&mut self, ctx: &egui::Context, has_macro: bool) -> Option<RunTimes> {
        if !self.open {
            return None;
        }
        let mut result = None;
        let mut close = false;
        let response = egui::Modal::new(Id::new("run-macro")).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.heading(t!("run-macro-title"));
            ui.add_space(6.0);
            let mut enter = false;
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.to_end, false, t!("run"));
                let r = ui.add_enabled(
                    !self.to_end,
                    egui::TextEdit::singleline(&mut self.count)
                        .desired_width(60.0)
                        .margin(Margin::symmetric(6, 3)),
                );
                if self.focus {
                    r.request_focus();
                    self.focus = false;
                }
                enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                ui.label(t!("times"));
            });
            ui.radio_value(&mut self.to_end, true, t!("run-to-end"));
            if !has_macro {
                ui.label(RichText::new(t!("record-macro-first")).weak());
            }
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 26.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if ui.button(t!("cancel")).clicked() {
                        close = true;
                    }
                    let run = ui.add_enabled(has_macro, egui::Button::new(t!("run")));
                    if run.clicked() || (enter && has_macro) {
                        result = Some(if self.to_end {
                            RunTimes::ToEnd
                        } else {
                            RunTimes::Count(self.count.trim().parse().unwrap_or(1).max(1))
                        });
                        close = true;
                    }
                },
            );
        });
        if close || response.should_close() {
            self.open = false;
        }
        result
    }
}

#[derive(Default)]
pub struct SaveMacroDialog {
    pub open: bool,
    name: String,
    focus: bool,
}

impl SaveMacroDialog {
    pub fn show_dialog(&mut self) {
        self.open = true;
        self.focus = true;
        self.name.clear();
    }

    /// Returns the name to save the current macro under.
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<String> {
        if !self.open {
            return None;
        }
        let mut result = None;
        let mut close = false;
        let response = egui::Modal::new(Id::new("save-macro")).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.heading(t!("save-macro-title"));
            ui.add_space(6.0);
            ui.label(t!("rule-name"));
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.name)
                    .desired_width(f32::INFINITY)
                    .margin(Margin::symmetric(6, 4)),
            );
            if self.focus {
                r.request_focus();
                self.focus = false;
            }
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 26.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if ui.button(t!("cancel")).clicked() {
                        close = true;
                    }
                    let ok = !self.name.trim().is_empty();
                    if ui.add_enabled(ok, egui::Button::new(t!("save"))).clicked() || (enter && ok)
                    {
                        result = Some(self.name.trim().to_owned());
                        close = true;
                    }
                },
            );
        });
        if close || response.should_close() {
            self.open = false;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_numbers() {
        let n = ColumnInsert::Numbers {
            initial: 8,
            step: 1,
            repeat: 1,
            leading_zeros: true,
            format: NumberFormat::Dec,
        };
        assert_eq!(n.values(3), vec!["08", "09", "10"]);
        let n = ColumnInsert::Numbers {
            initial: 0,
            step: 10,
            repeat: 2,
            leading_zeros: false,
            format: NumberFormat::Hex,
        };
        assert_eq!(n.values(5), vec![" 0", " 0", " A", " A", "14"]);
    }
}
