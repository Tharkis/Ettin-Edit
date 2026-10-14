//! Dialogs and bars for file commands: rename, move to trash, summary, the
//! "changed on disk" bar, and the encoding menu.

use egui::{Align, Id, Key, Layout, Margin, RichText, Ui, Vec2};

use crate::actions::Action;
use crate::document::DiskChange;
use crate::encoding::{TextEncoding, legacy_groups};
use crate::theme::Palette;

fn button_row(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 26.0),
        Layout::right_to_left(Align::Center),
        add,
    );
}

#[derive(Default)]
pub struct RenameDialog {
    pub open: bool,
    /// The tab being renamed.
    pub doc_id: u64,
    name: String,
    focus: bool,
    pub error: Option<String>,
}

impl RenameDialog {
    pub fn show_dialog(&mut self, doc_id: u64, current: &str) {
        *self = Self {
            open: true,
            doc_id,
            name: current.to_owned(),
            focus: true,
            error: None,
        };
    }

    /// Returns the new file name once confirmed.
    pub fn ui(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<String> {
        if !self.open {
            return None;
        }
        let mut result = None;
        let mut close = false;
        let response = egui::Modal::new(Id::new("rename-file")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.heading("Rename File");
            ui.add_space(6.0);
            ui.label("New name");
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.name)
                    .desired_width(f32::INFINITY)
                    .margin(Margin::symmetric(6, 4)),
            );
            if self.focus {
                r.request_focus();
                // Select the name without its extension, as file managers do.
                let stem = match self.name.rfind('.') {
                    Some(dot) if dot > 0 => self.name[..dot].chars().count(),
                    _ => self.name.chars().count(),
                };
                if let Some(mut state) = egui::TextEdit::load_state(ctx, r.id) {
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(stem),
                        )));
                    state.store(ctx, r.id);
                }
                self.focus = false;
            }
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(palette.modified).size(12.0));
            }
            ui.add_space(8.0);
            button_row(ui, |ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                let name = self.name.trim();
                let ok = !name.is_empty();
                if ui.add_enabled(ok, egui::Button::new("Rename")).clicked() || (enter && ok) {
                    result = Some(name.to_owned());
                }
            });
        });
        if close || response.should_close() {
            self.open = false;
        }
        result
    }
}

/// Asks before moving a file to the trash. Returns Some(true) to go ahead,
/// Some(false) to cancel.
pub fn confirm_trash(ctx: &egui::Context, name: &str, palette: &Palette) -> Option<bool> {
    let bin = if cfg!(windows) {
        "the Recycle Bin"
    } else {
        "the trash"
    };
    let mut choice = None;
    let response = egui::Modal::new(Id::new("trash-file")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading("Move to Trash");
        ui.add_space(6.0);
        ui.label(format!(
            "Move \"{name}\" to {bin} and close its tab? You can restore it from {bin}."
        ));
        ui.add_space(12.0);
        button_row(ui, |ui| {
            if ui.button("Cancel").clicked() {
                choice = Some(false);
            }
            if ui
                .add(egui::Button::new("Move to Trash").fill(palette.modified.gamma_multiply(0.5)))
                .clicked()
            {
                choice = Some(true);
            }
        });
    });
    if response.should_close() {
        choice = Some(false);
    }
    choice
}

/// Facts shown in the Summary dialog.
pub struct Summary {
    pub path: String,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub encoding: String,
    pub chars: usize,
    pub words: usize,
    pub lines: usize,
    pub bytes: Option<usize>,
    pub selected_chars: usize,
    pub selected_lines: usize,
}

/// Returns false once closed.
pub fn summary_ui(ctx: &egui::Context, s: &Summary) -> bool {
    let mut keep = true;
    let response = egui::Modal::new(Id::new("summary")).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.heading("Summary");
        ui.add_space(6.0);
        ui.label(RichText::new("File").weak());
        ui.add(egui::Label::new(&s.path).wrap());
        ui.add_space(6.0);
        let num = |n: usize| {
            let digits = n.to_string();
            let mut out = String::new();
            for (i, c) in digits.chars().enumerate() {
                if i > 0 && (digits.len() - i).is_multiple_of(3) {
                    out.push(',');
                }
                out.push(c);
            }
            out
        };
        egui::Grid::new("summary-grid")
            .num_columns(2)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                let mut row = |label: &str, value: String| {
                    ui.label(RichText::new(label).weak());
                    ui.label(value);
                    ui.end_row();
                };
                if let Some(c) = &s.created {
                    row("Created", c.clone());
                }
                if let Some(m) = &s.modified {
                    row("Modified", m.clone());
                }
                row("Encoding", s.encoding.clone());
                row("Characters (without line endings)", num(s.chars));
                row("Words", num(s.words));
                row("Lines", num(s.lines));
                row(
                    "Size when saved",
                    s.bytes
                        .map(|b| format!("{} bytes", num(b)))
                        .unwrap_or_else(|| "can't be saved in this encoding".into()),
                );
                if s.selected_chars > 0 {
                    row(
                        "Selection",
                        format!(
                            "{} characters on {} lines",
                            num(s.selected_chars),
                            num(s.selected_lines)
                        ),
                    );
                }
            });
        ui.add_space(12.0);
        button_row(ui, |ui| {
            if ui.button("Close").clicked() {
                keep = false;
            }
        });
    });
    keep && !response.should_close()
}

/// The bar above the editor when the file changed on disk. Returns the action picked.
pub fn disk_notice(
    ui: &mut Ui,
    change: DiskChange,
    dirty: bool,
    palette: &Palette,
) -> Option<Action> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.label(RichText::new("●").color(palette.modified));
        let text = match (change, dirty) {
            (DiskChange::Modified, false) => "This file was changed by another program.",
            (DiskChange::Modified, true) => {
                "This file was changed by another program, and you have unsaved changes here. Reloading replaces them (Undo brings them back)."
            }
            (DiskChange::Deleted, _) => "This file was deleted or moved by another program.",
        };
        ui.label(text);
        match change {
            DiskChange::Modified => {
                if ui.button("Reload").clicked() {
                    action = Some(Action::ReloadFromDisk);
                }
                if ui.button("Keep My Version").clicked() {
                    action = Some(Action::IgnoreDiskChange);
                }
            }
            DiskChange::Deleted => {
                if ui.button("Keep Open").clicked() {
                    action = Some(Action::IgnoreDiskChange);
                }
                if ui.button("Close").clicked() {
                    action = Some(Action::CloseTab);
                }
            }
        }
    });
    action
}

/// The encoding choices: Unicode ones first, then the others by region.
/// `make` turns an encoding name into the action to run.
pub fn encoding_menu(
    ui: &mut Ui,
    current: TextEncoding,
    make: impl Fn(String) -> Action,
    picked: &mut Option<Action>,
) {
    let mut item = |ui: &mut Ui, label: &str, enc: TextEncoding| {
        if ui
            .add(egui::Button::new(label).selected(enc == current))
            .clicked()
        {
            *picked = Some(make(enc.name()));
            ui.close();
        }
    };
    for enc in TextEncoding::UNICODE {
        item(ui, &enc.name(), enc);
    }
    ui.separator();
    for (group, list) in legacy_groups() {
        ui.menu_button(group, |ui| {
            for (label, enc) in list {
                item(ui, label, TextEncoding::Legacy(enc));
            }
        });
    }
}
