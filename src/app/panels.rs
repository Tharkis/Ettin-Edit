//! The side panels from the View menu: the folder panel on the left and the
//! document map on the right.

use egui::{Frame, Margin, Ui};

use super::EttinApp;
use crate::actions::Action;
use crate::folder_ui::FolderAction;
use crate::system;

/// Width of the document map, in points.
const MAP_WIDTH: f32 = 110.0;

impl EttinApp {
    pub(super) fn folder_panel(&mut self, ui: &mut Ui) {
        if !self.settings.folder_panel || self.distraction_free {
            return;
        }
        let palette = self.palette();
        let active = self.docs.get(self.active).and_then(|d| d.path.clone());
        let mut action = None;
        egui::Panel::left("folders")
            .resizable(true)
            .default_size(240.0)
            .min_size(140.0)
            .frame(
                Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(6, 4)),
            )
            .show(ui, |ui| {
                action = self.folders.ui(ui, active.as_deref(), &palette)
            });
        match action {
            Some(FolderAction::Open(path)) => self.pending.push(Action::OpenPath(path)),
            Some(FolderAction::FindInFolder(dir)) => {
                let (seed, _) = self.selection_seed();
                self.fif.show_dialog(seed, Some(dir));
            }
            Some(FolderAction::OpenTerminal(dir)) => {
                if let Err(e) = system::open_terminal(&dir) {
                    self.errors.push(e);
                }
            }
            Some(FolderAction::Close) => self.settings.folder_panel = false,
            None => {}
        }
    }

    pub(super) fn doc_map_panel(&mut self, ui: &mut Ui) {
        if !self.settings.doc_map || self.distraction_free || self.docs.is_empty() {
            return;
        }
        let palette = self.palette();
        let theme = &self.themes[self.theme_idx].syntax;
        let syntaxes = &self.highlighter.syntaxes;
        let doc = &mut self.docs[self.active];
        egui::Panel::right("doc-map")
            .resizable(false)
            .exact_size(MAP_WIDTH)
            .frame(Frame::new().fill(palette.editor_bg))
            .show(ui, |ui| doc.editor.show_map(ui, &palette, syntaxes, theme));
    }
}

impl EttinApp {
    pub(super) fn char_panel(&mut self, ui: &mut Ui) {
        if !self.settings.char_panel || self.distraction_free {
            return;
        }
        let palette = self.palette();
        let mut action = None;
        egui::Panel::right("char-panel")
            .resizable(true)
            .default_size(420.0)
            .min_size(240.0)
            .frame(
                Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| action = self.char_panel.ui(ui, &palette));
        match action {
            Some(crate::char_panel::CharAction::Insert(text)) => {
                if let Some(doc) = self.docs.get_mut(self.active) {
                    if doc.editor.buffer.read_only {
                        self.errors.push(t!("read-only-status"));
                    } else {
                        doc.editor.insert_text(ui.ctx(), &text);
                        self.focus_editor = true;
                    }
                }
            }
            Some(crate::char_panel::CharAction::Close) => self.settings.char_panel = false,
            None => {}
        }
    }

    pub(super) fn function_list_panel(&mut self, ui: &mut Ui) {
        if !self.settings.function_list || self.distraction_free || self.docs.is_empty() {
            return;
        }
        let palette = self.palette();
        let ctx = ui.ctx().clone();
        let doc = &mut self.docs[self.active];
        let key = (doc.id, doc.editor.buffer.version());
        let editor = &mut doc.editor;
        self.function_list
            .update(&ctx, key, &doc.syntax, || editor.text_snapshot());
        let caret_line = doc.editor.caret_line_col().0;
        let mut jump = None;
        egui::Panel::right("function-list")
            .resizable(true)
            .default_size(240.0)
            .min_size(150.0)
            .frame(
                Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| {
                jump = self.function_list.ui(ui, caret_line, &palette)
            });
        if let Some(line) = jump {
            self.docs[self.active].editor.go_to_near_top(line);
            self.focus_editor = true;
            ctx.request_repaint();
        }
    }
}
