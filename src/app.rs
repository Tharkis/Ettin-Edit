//! The NixPad window: menu bar, tab strip, editor and status bar.

use std::path::{Path, PathBuf};

use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Id, Key, Layout, Margin, Pos2, Rect,
    RichText, ScrollArea, Sense, Shape, Stroke, TextEdit, Ui, Vec2, ViewportCommand,
};

use crate::actions::{self, Action};
use crate::document::{Document, LineEnding};
use crate::highlight::{Highlighter, PLAIN_TEXT};
use crate::settings::{self, Settings};
use crate::theme::{self, Palette, ThemeDef};

const MIN_FONT: f32 = 8.0;
const MAX_FONT: f32 = 48.0;
const DEFAULT_FONT: f32 = 14.0;

/// Tabs waiting to be closed, one unsaved-changes prompt at a time.
struct CloseQueue {
    doc_ids: Vec<u64>,
    quit_after: bool,
}

pub struct NixPadApp {
    docs: Vec<Document>,
    active: usize,
    next_id: u64,
    untitled_counter: usize,
    settings: Settings,
    themes: Vec<ThemeDef>,
    theme_idx: usize,
    applied_theme: Option<usize>,
    highlighter: Highlighter,
    languages: Vec<String>,
    pending: Vec<Action>,
    close_queue: Option<CloseQueue>,
    errors: Vec<String>,
    show_about: bool,
    quitting: bool,
    focus_editor: bool,
    scroll_to_active_tab: bool,
    window_title: String,
}

impl NixPadApp {
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<PathBuf>) -> Self {
        let settings = Settings::load();
        let themes = theme::load_themes(settings::themes_dir().as_deref());
        let theme_idx = themes.iter().position(|t| t.name == settings.theme).unwrap_or(0);
        let highlighter = Highlighter::new();
        let languages = highlighter.language_names();

        let mut app = Self {
            docs: Vec::new(),
            active: 0,
            next_id: 1,
            untitled_counter: 0,
            settings,
            themes,
            theme_idx,
            applied_theme: None,
            highlighter,
            languages,
            pending: Vec::new(),
            close_queue: None,
            errors: Vec::new(),
            show_about: false,
            quitting: false,
            focus_editor: true,
            scroll_to_active_tab: true,
            window_title: String::new(),
        };
        app.apply_style(&cc.egui_ctx);

        if app.settings.restore_session {
            let session = std::mem::take(&mut app.settings.session_files);
            for path in session.iter().filter(|p| p.is_file()) {
                app.open_path(path, false);
            }
            app.active = app.settings.session_active.min(app.docs.len().saturating_sub(1));
        }
        for path in files {
            app.open_path(&path, true);
        }
        if app.docs.is_empty() {
            app.new_tab();
        }
        app
    }

    fn palette(&self) -> Palette {
        self.themes[self.theme_idx].palette
    }

    // ---------------------------------------------------------------------
    // Documents

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn new_tab(&mut self) {
        self.untitled_counter += 1;
        let id = self.alloc_id();
        self.docs.push(Document::new_untitled(id, self.untitled_counter));
        self.select(self.docs.len() - 1);
    }

    fn select(&mut self, index: usize) {
        if index < self.docs.len() {
            self.active = index;
            self.focus_editor = true;
            self.scroll_to_active_tab = true;
        }
    }

    fn open_path(&mut self, path: &Path, record_recent: bool) {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Some(i) = self.docs.iter().position(|d| d.path.as_deref() == Some(path.as_path())) {
            self.select(i);
            return;
        }
        let id = self.alloc_id();
        match Document::open(id, &path) {
            Ok(mut doc) => {
                doc.syntax = self.highlighter.detect(doc.path.as_deref(), &doc.text);
                if doc.lossy {
                    self.errors.push(format!(
                        "{} is not valid UTF-8. Invalid bytes are shown as \u{FFFD} and will be replaced if you save.",
                        path.display()
                    ));
                }
                // Opening a file replaces an untouched "new" tab, like most editors.
                let replace = self.docs.get(self.active).is_some_and(|d| d.is_pristine_untitled());
                if replace {
                    self.docs[self.active] = doc;
                    self.select(self.active);
                } else {
                    self.docs.push(doc);
                    self.select(self.docs.len() - 1);
                }
                if record_recent {
                    self.settings.push_recent(path);
                }
            }
            Err(e) => self.errors.push(format!("Could not open {}: {e}", path.display())),
        }
    }

    fn open_dialog(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open");
        if let Some(dir) = self.docs.get(self.active).and_then(|d| d.path.as_ref()?.parent().map(Path::to_path_buf)) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(paths) = dialog.pick_files() {
            for path in paths {
                self.open_path(&path, true);
            }
        }
    }

    /// Saves the document at `index`, asking for a path if needed. Returns false if not saved.
    fn save(&mut self, index: usize, force_dialog: bool) -> bool {
        let Some(doc) = self.docs.get(index) else { return false };
        let path = match (&doc.path, force_dialog) {
            (Some(p), false) => p.clone(),
            _ => {
                let mut dialog = rfd::FileDialog::new().set_title("Save As").set_file_name(doc.title());
                if let Some(dir) = doc.path.as_ref().and_then(|p| p.parent()) {
                    dialog = dialog.set_directory(dir);
                }
                match dialog.save_file() {
                    Some(p) => p,
                    None => return false,
                }
            }
        };
        let doc = &mut self.docs[index];
        let was_unnamed = doc.path.is_none();
        match doc.save_to(&path) {
            Ok(()) => {
                if was_unnamed || doc.syntax == PLAIN_TEXT {
                    doc.syntax = self.highlighter.detect(Some(&path), &doc.text);
                }
                self.settings.push_recent(path);
                true
            }
            Err(e) => {
                self.errors.push(format!("Could not save {}: {e}", path.display()));
                false
            }
        }
    }

    fn remove_doc(&mut self, id: u64) {
        if let Some(i) = self.docs.iter().position(|d| d.id == id) {
            self.docs.remove(i);
            if self.active > i || self.active >= self.docs.len() {
                self.active = self.active.saturating_sub(1);
            }
            self.focus_editor = true;
        }
        if self.docs.is_empty() && !self.quitting {
            self.new_tab();
        }
    }

    fn request_close(&mut self, doc_ids: Vec<u64>, quit_after: bool) {
        if self.close_queue.is_none() {
            self.close_queue = Some(CloseQueue { doc_ids, quit_after });
        }
    }

    fn capture_session(&mut self) {
        self.settings.session_files = self.docs.iter().filter_map(|d| d.path.clone()).collect();
        let before_active = self.docs.iter().take(self.active).filter(|d| d.path.is_some()).count();
        self.settings.session_active = before_active;
    }

    // ---------------------------------------------------------------------
    // Actions

    fn run(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::NewTab => self.new_tab(),
            Action::Open => self.open_dialog(),
            Action::OpenPath(p) => self.open_path(&p, true),
            Action::Save => {
                self.save(self.active, false);
            }
            Action::SaveAs => {
                self.save(self.active, true);
            }
            Action::SaveAll => {
                for i in 0..self.docs.len() {
                    if self.docs[i].dirty && !self.save(i, false) {
                        break;
                    }
                }
            }
            Action::CloseTab => {
                let id = self.docs[self.active].id;
                self.request_close(vec![id], false);
            }
            Action::CloseTabAt(i) => {
                if let Some(d) = self.docs.get(i) {
                    self.request_close(vec![d.id], false);
                }
            }
            Action::CloseAll => self.request_close(self.docs.iter().map(|d| d.id).collect(), false),
            Action::CloseOthers => {
                let keep = self.docs[self.active].id;
                self.request_close(self.docs.iter().map(|d| d.id).filter(|&id| id != keep).collect(), false);
            }
            Action::Quit => {
                self.capture_session();
                self.request_close(self.docs.iter().filter(|d| d.dirty).map(|d| d.id).collect(), true);
            }
            Action::NextTab => self.select((self.active + 1) % self.docs.len()),
            Action::PrevTab => self.select((self.active + self.docs.len() - 1) % self.docs.len()),
            Action::SelectTab(i) => self.select(i),
            Action::ZoomIn => self.settings.font_size = (self.settings.font_size + 1.0).min(MAX_FONT),
            Action::ZoomOut => self.settings.font_size = (self.settings.font_size - 1.0).max(MIN_FONT),
            Action::ZoomReset => self.settings.font_size = DEFAULT_FONT,
            Action::ToggleWordWrap => self.settings.word_wrap = !self.settings.word_wrap,
            Action::ToggleLineNumbers => self.settings.show_line_numbers = !self.settings.show_line_numbers,
            Action::ToggleCurrentLine => {
                self.settings.highlight_current_line = !self.settings.highlight_current_line
            }
            Action::SetTheme(name) => {
                if let Some(i) = self.themes.iter().position(|t| t.name == name) {
                    self.theme_idx = i;
                    self.settings.theme = name;
                    self.apply_style(ctx);
                }
            }
            Action::SetLanguage(name) => {
                if let Some(doc) = self.docs.get_mut(self.active) {
                    doc.syntax = name;
                }
            }
            Action::SetLineEnding(le) => {
                if let Some(doc) = self.docs.get_mut(self.active)
                    && doc.line_ending != le {
                        doc.line_ending = le;
                        doc.dirty = true;
                    }
            }
            Action::OpenThemesFolder => {
                if let Some(dir) = settings::themes_dir() {
                    let _ = std::fs::create_dir_all(&dir);
                    ctx.open_url(egui::OpenUrl::new_tab(format!("file://{}", dir.display())));
                }
            }
            Action::ClearRecent => self.settings.recent_files.clear(),
            Action::About => self.show_about = true,
        }
        self.focus_editor = true;
    }

    fn apply_style(&mut self, ctx: &egui::Context) {
        if self.applied_theme == Some(self.theme_idx) {
            return;
        }
        let palette = self.palette();
        let theme = if palette.dark { egui::Theme::Dark } else { egui::Theme::Light };
        ctx.set_theme(theme);
        ctx.set_visuals_of(theme, palette.visuals());
        ctx.style_mut_of(theme, |style| {
            style.spacing.item_spacing = Vec2::new(8.0, 4.0);
            style.spacing.button_padding = Vec2::new(8.0, 3.0);
            style.spacing.menu_margin = Margin::same(6);
            style.interaction.selectable_labels = false;
        });
        self.applied_theme = Some(self.theme_idx);
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        if self.close_queue.is_none() && !self.show_about {
            let mut fired = Vec::new();
            ctx.input_mut(|i| {
                for (shortcut, action) in actions::shortcuts() {
                    if i.consume_shortcut(&shortcut) {
                        fired.push(action);
                    }
                }
                // Alt shortcuts also arrive as typed characters; don't insert them.
                if !fired.is_empty() {
                    i.events.retain(|e| !matches!(e, egui::Event::Text(_)));
                }
            });
            self.pending.extend(fired);
        }

        let dropped: Vec<PathBuf> =
            ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        self.pending.extend(dropped.into_iter().map(Action::OpenPath));

        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.pending.push(Action::Quit);
        }
    }

    // ---------------------------------------------------------------------
    // UI

    fn menu_item(&mut self, ui: &mut Ui, label: &str, action: Action) {
        let mut button = egui::Button::new(label);
        if let Some(sc) = actions::shortcut_label(ui.ctx(), &action) {
            button = button.shortcut_text(sc);
        }
        if ui.add(button).clicked() {
            self.pending.push(action);
        }
    }

    fn toggle_item(&mut self, ui: &mut Ui, label: &str, checked: bool, action: Action) {
        let mut button = egui::Button::new(label).selected(checked);
        if let Some(sc) = actions::shortcut_label(ui.ctx(), &action) {
            button = button.shortcut_text(sc);
        }
        if ui.add(button).clicked() {
            self.pending.push(action);
        }
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                self.menu_item(ui, "New", Action::NewTab);
                self.menu_item(ui, "Open…", Action::Open);
                ui.menu_button("Open Recent", |ui| {
                    if self.settings.recent_files.is_empty() {
                        ui.label(RichText::new("No recent files").weak());
                    }
                    for path in self.settings.recent_files.clone() {
                        if ui.button(path.display().to_string()).clicked() {
                            self.pending.push(Action::OpenPath(path));
                        }
                    }
                    ui.separator();
                    self.menu_item(ui, "Clear Recent Files", Action::ClearRecent);
                });
                ui.separator();
                self.menu_item(ui, "Save", Action::Save);
                self.menu_item(ui, "Save As…", Action::SaveAs);
                self.menu_item(ui, "Save All", Action::SaveAll);
                ui.separator();
                self.menu_item(ui, "Close", Action::CloseTab);
                self.menu_item(ui, "Close All", Action::CloseAll);
                self.menu_item(ui, "Close Others", Action::CloseOthers);
                ui.separator();
                self.menu_item(ui, "Exit", Action::Quit);
            });
            ui.menu_button("View", |ui| {
                let s = self.settings.clone();
                self.toggle_item(ui, "Word Wrap", s.word_wrap, Action::ToggleWordWrap);
                self.toggle_item(ui, "Line Numbers", s.show_line_numbers, Action::ToggleLineNumbers);
                self.toggle_item(ui, "Highlight Current Line", s.highlight_current_line, Action::ToggleCurrentLine);
                ui.separator();
                self.menu_item(ui, "Zoom In", Action::ZoomIn);
                self.menu_item(ui, "Zoom Out", Action::ZoomOut);
                self.menu_item(ui, "Reset Zoom", Action::ZoomReset);
            });
            ui.menu_button("Theme", |ui| {
                let current = self.theme_idx;
                let names: Vec<(String, bool, bool)> =
                    self.themes.iter().map(|t| (t.name.clone(), t.palette.dark, t.custom)).collect();
                ui.label(RichText::new("Dark").small().weak());
                for (i, (name, _, _)) in names.iter().enumerate().filter(|(_, t)| t.1 && !t.2) {
                    self.toggle_item(ui, name, i == current, Action::SetTheme(name.clone()));
                }
                ui.separator();
                ui.label(RichText::new("Light").small().weak());
                for (i, (name, _, _)) in names.iter().enumerate().filter(|(_, t)| !t.1 && !t.2) {
                    self.toggle_item(ui, name, i == current, Action::SetTheme(name.clone()));
                }
                if names.iter().any(|t| t.2) {
                    ui.separator();
                    ui.label(RichText::new("Custom").small().weak());
                    for (i, (name, _, _)) in names.iter().enumerate().filter(|(_, t)| t.2) {
                        self.toggle_item(ui, name, i == current, Action::SetTheme(name.clone()));
                    }
                }
                ui.separator();
                self.menu_item(ui, "Open Themes Folder", Action::OpenThemesFolder);
            });
            ui.menu_button("Language", |ui| self.language_menu(ui));
            ui.menu_button("Help", |ui| {
                self.menu_item(ui, "About NixPad", Action::About);
            });
        });
    }

    /// Languages grouped by first letter, like Notepad++'s Language menu.
    fn language_menu(&mut self, ui: &mut Ui) {
        let current = self.docs.get(self.active).map(|d| d.syntax.clone()).unwrap_or_default();
        self.toggle_item(ui, PLAIN_TEXT, current == PLAIN_TEXT, Action::SetLanguage(PLAIN_TEXT.to_owned()));
        ui.separator();
        let languages = self.languages.clone();
        let mut letters: Vec<char> = languages
            .iter()
            .filter(|l| l.as_str() != PLAIN_TEXT)
            .filter_map(|l| l.chars().next().map(|c| c.to_ascii_uppercase()))
            .collect();
        letters.dedup();
        for letter in letters {
            ui.menu_button(letter.to_string(), |ui| {
                for name in languages
                    .iter()
                    .filter(|l| l.as_str() != PLAIN_TEXT && l.chars().next().map(|c| c.to_ascii_uppercase()) == Some(letter))
                {
                    self.toggle_item(ui, name, *name == current, Action::SetLanguage(name.clone()));
                }
            });
        }
    }

    fn tab_bar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let tab_height = 32.0;
        ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    for i in 0..self.docs.len() {
                        self.tab(ui, i, tab_height, &palette);
                    }
                    ui.add_space(4.0);
                    let plus = ui.add_sized(
                        [28.0, tab_height - 6.0],
                        egui::Button::new(RichText::new("+").size(16.0).color(palette.text_dim)).frame(false),
                    );
                    if plus.on_hover_text("New tab").clicked() {
                        self.pending.push(Action::NewTab);
                    }
                });
            });
        self.scroll_to_active_tab = false;
    }

    fn tab(&mut self, ui: &mut Ui, index: usize, height: f32, palette: &Palette) {
        let doc = &self.docs[index];
        let active = index == self.active;
        let title = doc.title();
        let font = FontId::proportional(13.0);
        let text_color = if active { palette.text } else { palette.text_dim };
        let galley = ui.painter().layout_no_wrap(title, font, text_color);
        let pad = 14.0;
        let close_size = 18.0;
        let width = (pad + galley.size().x + 8.0 + close_size + 8.0).max(90.0);

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        let close_rect = Rect::from_center_size(
            Pos2::new(rect.right() - 8.0 - close_size / 2.0, rect.center().y),
            Vec2::splat(close_size),
        );
        let close_response = ui.interact(close_rect, Id::new(("tab-close", doc.id)), Sense::click());

        let painter = ui.painter();
        let radius = CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
        if active {
            painter.rect_filled(rect, radius, palette.editor_bg);
            painter.rect_filled(
                Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), 2.0)),
                CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 },
                palette.accent,
            );
        } else if response.hovered() || close_response.hovered() {
            painter.rect_filled(rect, radius, palette.tab_hover_bg);
        } else {
            painter.rect_filled(rect, radius, palette.tab_bg);
        }
        painter.galley(
            Pos2::new(rect.left() + pad, rect.center().y - galley.size().y / 2.0),
            galley,
            text_color,
        );

        // Unsaved tabs show a dot that turns into a close button on hover.
        let show_close = close_response.hovered() || response.hovered() || active || !doc.dirty;
        if close_response.hovered() {
            painter.rect_filled(close_rect, CornerRadius::same(4), palette.tab_hover_bg.gamma_multiply(1.6));
        }
        if doc.dirty && !close_response.hovered() {
            painter.circle_filled(close_rect.center(), 4.0, palette.modified);
        } else if show_close {
            let c = close_rect.center();
            let d = 4.0;
            let stroke = Stroke::new(1.4, if close_response.hovered() { palette.text } else { palette.text_dim });
            painter.line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], stroke);
            painter.line_segment([c + Vec2::new(-d, d), c + Vec2::new(d, -d)], stroke);
        }

        if active && self.scroll_to_active_tab {
            response.scroll_to_me(None);
        }
        if close_response.clicked() || response.middle_clicked() {
            self.pending.push(Action::CloseTabAt(index));
        } else if response.clicked() {
            self.pending.push(Action::SelectTab(index));
        }
        let path = doc.path.clone();
        let response = match &path {
            Some(p) => response.on_hover_text(p.display().to_string()),
            None => response,
        };
        response.context_menu(|ui| {
            if ui.button("Close").clicked() {
                self.pending.push(Action::CloseTabAt(index));
            }
            if ui.button("Close Others").clicked() {
                self.pending.push(Action::SelectTab(index));
                self.pending.push(Action::CloseOthers);
            }
            if let Some(p) = &path {
                ui.separator();
                if ui.button("Copy Full Path").clicked() {
                    ui.ctx().copy_text(p.display().to_string());
                }
                if ui.button("Copy File Name").clicked() {
                    ui.ctx().copy_text(p.file_name().unwrap_or_default().to_string_lossy().into_owned());
                }
            }
        });
    }

    fn editor(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let font = FontId::monospace(self.settings.font_size);
        let wrap = self.settings.word_wrap;
        let show_numbers = self.settings.show_line_numbers;
        let highlight_line = self.settings.highlight_current_line;
        let theme_def = &self.themes[self.theme_idx];
        let highlighter = &self.highlighter;
        let focus = std::mem::take(&mut self.focus_editor) && self.close_queue.is_none() && !self.show_about;

        let Some(doc) = self.docs.get_mut(self.active) else { return };
        let digits = doc.line_count().max(1000).to_string().len() as f32;
        let char_width = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
        let gutter_width = if show_numbers { digits * char_width + 28.0 } else { 12.0 };
        let editor_id = Id::new(("editor", doc.id));
        if std::mem::take(&mut doc.fresh) {
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
            let start = egui::text::CCursor::new(0);
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(start)));
            state.store(ui.ctx(), editor_id);
        }

        let Document { text, syntax, highlight, .. } = doc;
        let syntax_name = syntax.clone();
        let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = highlighter.layout(
                highlight,
                buf.as_str(),
                &syntax_name,
                &theme_def.name,
                &theme_def.syntax,
                &font,
                palette.editor_fg,
            );
            job.wrap.max_width = if wrap { wrap_width } else { f32::INFINITY };
            ui.fonts_mut(|f| f.layout_job(job))
        };

        let output = ScrollArea::both()
            .id_salt(("editor-scroll", editor_id))
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    let (gutter_rect, _) =
                        ui.allocate_exact_size(Vec2::new(gutter_width, ui.available_height()), Sense::hover());
                    let line_bg = ui.painter().add(Shape::Noop);
                    let available = ui.available_size();
                    let output = TextEdit::multiline(text)
                        .id(editor_id)
                        .font(font.clone())
                        .frame(Frame::NONE)
                        .margin(Margin { left: 0, right: 8, top: 6, bottom: 6 })
                        .lock_focus(true)
                        .desired_width(if wrap { available.x } else { f32::INFINITY })
                        .min_size(available)
                        .layouter(&mut layouter)
                        .show(ui);
                    (gutter_rect, line_bg, output)
                })
                .inner
            })
            .inner;
        let (gutter_rect, line_bg, output) = output;
        if focus {
            output.response.response.request_focus();
        }

        let doc = &mut self.docs[self.active];
        if output.response.response.changed() {
            doc.dirty = true;
        }
        let cursor = output.cursor_range;
        if let Some(range) = cursor {
            let sel = range.primary.index.0.abs_diff(range.secondary.index.0);
            doc.update_cursor(range.primary.index.0, sel);
        }

        // Line numbers and the current-line band, positioned from the laid-out rows
        // so they stay correct with word wrap on.
        let galley = &output.galley;
        let origin = output.galley_pos;
        let clip = ui.clip_rect();
        let painter = ui.painter();
        let current_row = cursor.map(|r| galley.pos_from_cursor(r.primary));

        if highlight_line && output.response.response.has_focus()
            && let Some(row_rect) = current_row {
                let band = Rect::from_x_y_ranges(
                    clip.x_range(),
                    (origin.y + row_rect.min.y)..=(origin.y + row_rect.max.y),
                );
                painter.set(line_bg, Shape::rect_filled(band, 0.0, palette.line_highlight));
            }

        if show_numbers {
            let number_font = FontId::monospace(self.settings.font_size * 0.9);
            let mut line = 1usize;
            let mut starts_line = true;
            for row in galley.rows.iter() {
                let top = origin.y + row.pos.y;
                let bottom = top + row.rect().height();
                if starts_line && bottom >= clip.top() && top <= clip.bottom() {
                    let is_current = current_row.is_some_and(|r| {
                        let y = origin.y + r.center().y;
                        y >= top && y <= bottom
                    });
                    painter.text(
                        Pos2::new(gutter_rect.right() - 14.0, (top + bottom) / 2.0),
                        Align2::RIGHT_CENTER,
                        line.to_string(),
                        number_font.clone(),
                        if is_current { palette.gutter_active_fg } else { palette.gutter_fg },
                    );
                }
                if top > clip.bottom() {
                    break;
                }
                starts_line = row.ends_with_newline;
                if row.ends_with_newline {
                    line += 1;
                }
            }
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let Some(doc) = self.docs.get(self.active) else { return };
        let (line, col) = doc.cursor;
        let sel = doc.selection_chars;
        let syntax = doc.syntax.clone();
        let line_ending = doc.line_ending;
        let encoding = if doc.has_bom { "UTF-8 BOM" } else { "UTF-8" };
        let length = doc.text.len();
        let lines = doc.line_count();
        let zoom = (self.settings.font_size / DEFAULT_FONT * 100.0).round();
        let dim = |s: String| RichText::new(s).size(12.0).color(palette.text_dim);

        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.menu_button(dim(syntax.clone()), |ui| {
                ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    for name in self.languages.clone() {
                        self.toggle_item(ui, &name, name == syntax, Action::SetLanguage(name.clone()));
                    }
                });
            });
            ui.label(dim(format!("length: {length}   lines: {lines}")));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(4.0);
                ui.label(dim(format!("{zoom}%")));
                ui.label(dim(encoding.to_owned()));
                ui.menu_button(dim(line_ending.label().to_owned()), |ui| {
                    for le in [LineEnding::CrLf, LineEnding::Lf, LineEnding::Cr] {
                        self.toggle_item(ui, le.label(), le == line_ending, Action::SetLineEnding(le));
                    }
                });
                let position = if sel > 0 {
                    format!("Ln {line}, Col {col}   Sel {sel}")
                } else {
                    format!("Ln {line}, Col {col}")
                };
                ui.label(dim(position));
            });
        });
    }

    /// Steps through the close queue, asking about one unsaved document at a time.
    fn close_queue_ui(&mut self, ctx: &egui::Context) {
        loop {
            let Some(queue) = &mut self.close_queue else { return };
            let Some(&id) = queue.doc_ids.first() else {
                let quit = queue.quit_after;
                self.close_queue = None;
                if quit {
                    self.quitting = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                return;
            };
            match self.docs.iter().position(|d| d.id == id) {
                Some(i) if self.docs[i].dirty => {
                    let name = self.docs[i].title();
                    let mut choice = None;
                    egui::Modal::new(Id::new("unsaved-changes")).show(ctx, |ui| {
                        ui.set_width(360.0);
                        ui.heading("Save changes?");
                        ui.add_space(6.0);
                        ui.label(format!("\"{name}\" has unsaved changes. Save them before closing?"));
                        ui.add_space(12.0);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                                choice = Some(0);
                            }
                            if ui.button("Don't Save").clicked() {
                                choice = Some(1);
                            }
                            if ui.add(egui::Button::new("Save").fill(self.palette().accent.gamma_multiply(0.6))).clicked() {
                                choice = Some(2);
                            }
                        });
                    });
                    match choice {
                        Some(0) => self.close_queue = None,
                        Some(1) => self.finish_close(id),
                        Some(2) => {
                            if self.save(i, false) {
                                self.finish_close(id);
                            } else {
                                self.close_queue = None;
                            }
                        }
                        _ => {}
                    }
                    return;
                }
                Some(_) => self.finish_close(id),
                None => {
                    queue.doc_ids.remove(0);
                }
            }
        }
    }

    fn finish_close(&mut self, id: u64) {
        let quitting = self.close_queue.as_ref().is_some_and(|q| q.quit_after);
        if let Some(queue) = &mut self.close_queue {
            queue.doc_ids.retain(|&d| d != id);
        }
        // When quitting, documents stay open so the session is saved as it was.
        if quitting {
            if let Some(doc) = self.docs.iter_mut().find(|d| d.id == id) {
                doc.dirty = false;
            }
        } else {
            self.remove_doc(id);
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.show_about {
            let response = egui::Modal::new(Id::new("about")).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("NixPad").size(26.0).strong().color(self.palette().accent));
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(8.0);
                    ui.label("A fast, modern text editor for Linux and Windows.");
                    ui.label(RichText::new("Written in Rust with egui and syntect.").weak());
                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
            });
            if response.should_close() {
                self.show_about = false;
            }
        }

        if let Some(message) = self.errors.first().cloned() {
            let response = egui::Modal::new(Id::new("error")).show(ctx, |ui| {
                ui.set_width(400.0);
                ui.heading("Something went wrong");
                ui.add_space(6.0);
                ui.label(message);
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.button("OK").clicked()).inner
            });
            if response.inner || response.should_close() {
                self.errors.remove(0);
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match self.docs.get(self.active) {
            Some(doc) => {
                let name = doc.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| doc.title());
                format!("{}{} - NixPad", if doc.dirty { "*" } else { "" }, name)
            }
            None => "NixPad".to_owned(),
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }
}

impl eframe::App for NixPadApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.apply_style(&ctx);
        self.handle_input(&ctx);
        let palette = self.palette();

        egui::Panel::top("menu-bar")
            .frame(Frame::new().fill(palette.chrome_bg).inner_margin(Margin::symmetric(6, 4)))
            .show_separator_line(false)
            .show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("tab-bar")
            .frame(Frame::new().fill(palette.chrome_bg).inner_margin(Margin { left: 6, right: 6, top: 2, bottom: 0 }))
            .show_separator_line(false)
            .show(ui, |ui| self.tab_bar(ui));
        egui::Panel::bottom("status-bar")
            .frame(Frame::new().fill(palette.chrome_bg).inner_margin(Margin::symmetric(10, 3)))
            .show_separator_line(false)
            .show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::no_frame()
            .frame(Frame::new().fill(palette.editor_bg))
            .show(ui, |ui| self.editor(ui));

        self.close_queue_ui(&ctx);
        self.dialogs(&ctx);

        for action in std::mem::take(&mut self.pending) {
            self.run(&ctx, action);
        }
        self.update_title(&ctx);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        if !self.quitting {
            self.capture_session();
        }
        self.settings.save();
    }
}
