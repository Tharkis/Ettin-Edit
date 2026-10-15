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
            ui.heading(t!("rename-file-title"));
            ui.add_space(6.0);
            ui.label(t!("new-name"));
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
                if ui.button(t!("cancel")).clicked() {
                    close = true;
                }
                let name = self.name.trim();
                let ok = !name.is_empty();
                if ui
                    .add_enabled(ok, egui::Button::new(t!("rename")))
                    .clicked()
                    || (enter && ok)
                {
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
    let windows = if cfg!(windows) { "yes" } else { "no" };
    let mut choice = None;
    let response = egui::Modal::new(Id::new("trash-file")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading(t!("move-to-trash-title"));
        ui.add_space(6.0);
        ui.label(t!("move-to-trash-question", name = name, windows = windows));
        ui.add_space(12.0);
        button_row(ui, |ui| {
            if ui.button(t!("cancel")).clicked() {
                choice = Some(false);
            }
            if ui
                .add(
                    egui::Button::new(t!("move-to-trash-title"))
                        .fill(palette.modified.gamma_multiply(0.5)),
                )
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
        ui.heading(t!("summary-title"));
        ui.add_space(6.0);
        ui.label(RichText::new(t!("summary-file")).weak());
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
                    row(&t!("summary-created"), c.clone());
                }
                if let Some(m) = &s.modified {
                    row(&t!("summary-modified"), m.clone());
                }
                row(&t!("menu-encoding"), s.encoding.clone());
                row(&t!("summary-characters"), num(s.chars));
                row(&t!("summary-words"), num(s.words));
                row(&t!("summary-lines"), num(s.lines));
                row(
                    &t!("summary-size"),
                    s.bytes
                        .map(|b| t!("summary-bytes", count = b, bytes = num(b)))
                        .unwrap_or_else(|| t!("summary-cant-encode")),
                );
                if s.selected_chars > 0 {
                    row(
                        &t!("summary-selection"),
                        t!(
                            "summary-selection-size",
                            characters = num(s.selected_chars),
                            lines = num(s.selected_lines),
                            line_count = s.selected_lines
                        ),
                    );
                }
            });
        ui.add_space(12.0);
        button_row(ui, |ui| {
            if ui.button(t!("close")).clicked() {
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
            (DiskChange::Modified, false) => t!("disk-changed"),
            (DiskChange::Modified, true) => t!("disk-changed-unsaved"),
            (DiskChange::Deleted, _) => t!("disk-deleted"),
        };
        ui.label(text);
        match change {
            DiskChange::Modified => {
                if ui.button(t!("reload")).clicked() {
                    action = Some(Action::ReloadFromDisk);
                }
                if ui.button(t!("keep-my-version")).clicked() {
                    action = Some(Action::IgnoreDiskChange);
                }
            }
            DiskChange::Deleted => {
                if ui.button(t!("keep-open")).clicked() {
                    action = Some(Action::IgnoreDiskChange);
                }
                if ui.button(t!("close")).clicked() {
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
        ui.menu_button(crate::i18n::tr(group), |ui| {
            for (label, enc) in list {
                item(ui, label, TextEncoding::Legacy(enc));
            }
        });
    }
}
