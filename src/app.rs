//! The NixPad window: menu bar, tab strip, editor and status bar.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use egui::{
    Align, CornerRadius, FontId, Frame, Id, Key, Layout, Margin, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, Vec2, ViewportCommand,
};

use crate::actions::{self, Action};
use crate::document::{Document, LineEnding};
use crate::editor::lines::{CaseKind, SortKind, TrimKind, comment_tokens};
use crate::editor::{EditOptions, FindOutcome, MAX_HIGHLIGHTS, ViewConfig};
use crate::highlight::{Highlighter, PLAIN_TEXT};
use crate::macros::{MacroStep, Recorder, SavedMacro};
use crate::search::{self, FileSearch, Query};
use crate::search_ui::{
    FifDialog, FifRequest, FileJob, FindBar, GotoDialog, Jump, Results, Source,
};
use crate::settings::{self, Settings};
use crate::theme::{self, Palette, ThemeDef};
use crate::tools_ui::{ColumnDialog, RunMacroDialog, RunTimes, SaveMacroDialog};

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
    find_bar: FindBar,
    results: Results,
    goto: GotoDialog,
    fif: FifDialog,
    column: ColumnDialog,
    run_macro: RunMacroDialog,
    save_macro: SaveMacroDialog,
    recorder: Recorder,
}

impl NixPadApp {
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<PathBuf>) -> Self {
        let settings = Settings::load();
        let themes = theme::load_themes(settings::themes_dir().as_deref());
        let theme_idx = themes
            .iter()
            .position(|t| t.name == settings.theme)
            .unwrap_or(0);
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
            find_bar: FindBar::default(),
            results: Results::default(),
            goto: GotoDialog::default(),
            fif: FifDialog::default(),
            column: ColumnDialog::default(),
            run_macro: RunMacroDialog::default(),
            save_macro: SaveMacroDialog::default(),
            recorder: Recorder::default(),
        };
        app.fif.dir = app.settings.fif_dir.clone();
        app.fif.filters = app.settings.fif_filters.clone();
        app.fif.subfolders = app.settings.fif_subfolders;
        app.fif.hidden = app.settings.fif_hidden;
        crate::fonts::install_fallbacks(&cc.egui_ctx);
        app.apply_style(&cc.egui_ctx);

        if app.settings.restore_session {
            let session = std::mem::take(&mut app.settings.session_files);
            for path in session.iter().filter(|p| p.is_file()) {
                app.open_path(path, false);
            }
            app.active = app
                .settings
                .session_active
                .min(app.docs.len().saturating_sub(1));
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
        self.docs
            .push(Document::new_untitled(id, self.untitled_counter));
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
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| d.path.as_deref() == Some(path.as_path()))
        {
            self.select(i);
            return;
        }
        let id = self.alloc_id();
        match Document::open(id, &path) {
            Ok(mut doc) => {
                doc.syntax = self
                    .highlighter
                    .detect(doc.path.as_deref(), &doc.editor.buffer.line_text(0));
                if doc.lossy {
                    self.errors.push(format!(
                        "{} is not valid UTF-8. Invalid bytes are shown as \u{FFFD} and will be replaced if you save.",
                        path.display()
                    ));
                }
                // Opening a file replaces an untouched "new" tab, like most editors.
                let replace = self
                    .docs
                    .get(self.active)
                    .is_some_and(|d| d.is_pristine_untitled());
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
            Err(e) => self
                .errors
                .push(format!("Could not open {}: {e}", path.display())),
        }
    }

    fn open_dialog(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open");
        if let Some(dir) = self
            .docs
            .get(self.active)
            .and_then(|d| d.path.as_ref()?.parent().map(Path::to_path_buf))
        {
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
        let Some(doc) = self.docs.get(index) else {
            return false;
        };
        let path = match (&doc.path, force_dialog) {
            (Some(p), false) => p.clone(),
            _ => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Save As")
                    .set_file_name(doc.title());
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
                    doc.syntax = self
                        .highlighter
                        .detect(Some(&path), &doc.editor.buffer.line_text(0));
                }
                self.settings.push_recent(path);
                true
            }
            Err(e) => {
                self.errors
                    .push(format!("Could not save {}: {e}", path.display()));
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
            self.close_queue = Some(CloseQueue {
                doc_ids,
                quit_after,
            });
        }
    }

    fn capture_session(&mut self) {
        self.settings.session_files = self.docs.iter().filter_map(|d| d.path.clone()).collect();
        let before_active = self
            .docs
            .iter()
            .take(self.active)
            .filter(|d| d.path.is_some())
            .count();
        self.settings.session_active = before_active;
    }

    // ---------------------------------------------------------------------
    // Actions

    fn run(&mut self, ctx: &egui::Context, action: Action) {
        let focus_editor = action.focuses_editor();
        if action.recordable() {
            self.recorder.push(MacroStep::Action(action.clone()));
        }
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
                    if self.docs[i].is_dirty() && !self.save(i, false) {
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
                self.request_close(
                    self.docs
                        .iter()
                        .map(|d| d.id)
                        .filter(|&id| id != keep)
                        .collect(),
                    false,
                );
            }
            Action::Quit => {
                self.capture_session();
                self.request_close(
                    self.docs
                        .iter()
                        .filter(|d| d.is_dirty())
                        .map(|d| d.id)
                        .collect(),
                    true,
                );
            }
            Action::NextTab => self.select((self.active + 1) % self.docs.len()),
            Action::PrevTab => self.select((self.active + self.docs.len() - 1) % self.docs.len()),
            Action::SelectTab(i) => self.select(i),
            Action::ZoomIn => {
                self.settings.font_size = (self.settings.font_size + 1.0).min(MAX_FONT)
            }
            Action::ZoomOut => {
                self.settings.font_size = (self.settings.font_size - 1.0).max(MIN_FONT)
            }
            Action::ZoomReset => self.settings.font_size = DEFAULT_FONT,
            Action::ToggleWordWrap => self.settings.word_wrap = !self.settings.word_wrap,
            Action::ToggleLineNumbers => {
                self.settings.show_line_numbers = !self.settings.show_line_numbers
            }
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
                if let Some(doc) = self.docs.get_mut(self.active) {
                    doc.set_line_ending(le);
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
            Action::Find | Action::Replace => {
                let (seed, multi_line) = self.selection_seed();
                self.find_bar
                    .show_bar(action == Action::Replace, seed, multi_line);
                let doc = &self.docs[self.active];
                self.find_bar.origin = Some((doc.id, doc.editor.selections.primary().start()));
            }
            Action::FindNext => self.find_step(true),
            Action::FindPrev => self.find_step(false),
            Action::FindSelectedNext | Action::FindSelectedPrev => {
                if let Some(word) = self.docs[self.active].editor.selection_or_word() {
                    self.find_bar.find = word;
                }
                self.find_step(action == Action::FindSelectedNext);
            }
            Action::ReplaceOne => self.replace_one(ctx),
            Action::ReplaceAll => self.replace_all(ctx),
            Action::FindAllInDoc => self.find_all(false),
            Action::FindAllInTabs => self.find_all(true),
            Action::SelectAllMatches => {
                if let Some(q) = self.query() {
                    let n = self.docs[self.active].editor.select_all_matches(&q);
                    self.find_bar.message = Some(match n {
                        0 => "No results".to_owned(),
                        n => format!("{n} selected"),
                    });
                }
            }
            Action::MarkAll { bookmark } => {
                if let Some(q) = self.query() {
                    let n = self.docs[self.active].editor.mark_all(&q, bookmark);
                    self.find_bar.message = Some(match n {
                        0 => "No results".to_owned(),
                        n => format!("{n} marked"),
                    });
                }
            }
            Action::ClearMarks => self.docs[self.active].editor.clear_marks(),
            Action::CloseFindBar => {
                self.find_bar.close();
                for doc in &mut self.docs {
                    doc.editor.search_hits = Arc::default();
                }
            }
            Action::FindInFiles => {
                let (seed, _) = self.selection_seed();
                let seed = seed.or_else(|| {
                    (!self.find_bar.find.is_empty()).then(|| self.find_bar.find.clone())
                });
                let dir = if self.fif.dir.is_empty() {
                    self.docs[self.active]
                        .path
                        .as_ref()
                        .and_then(|p| p.parent().map(Path::to_path_buf))
                        .or_else(dirs::home_dir)
                } else {
                    None
                };
                self.fif.show_dialog(seed, dir);
            }
            Action::GoToLine => self.goto.show_dialog(),
            Action::ToggleBookmark => self.docs[self.active].editor.toggle_bookmark_at_caret(),
            Action::NextBookmark => {
                self.docs[self.active].editor.goto_bookmark(true);
            }
            Action::PrevBookmark => {
                self.docs[self.active].editor.goto_bookmark(false);
            }
            Action::ClearBookmarks => self.docs[self.active].editor.bookmarks.clear(),
            Action::CopyBookmarkedLines | Action::CutBookmarkedLines => {
                let editor = &mut self.docs[self.active].editor;
                if !editor.bookmarks.is_empty() {
                    ctx.copy_text(editor.bookmarked_text());
                    if action == Action::CutBookmarkedLines {
                        editor.remove_lines(ctx, false);
                    }
                }
            }
            Action::RemoveBookmarkedLines => self.docs[self.active].editor.remove_lines(ctx, false),
            Action::RemoveUnbookmarkedLines => {
                self.docs[self.active].editor.remove_lines(ctx, true)
            }
            Action::InverseBookmarks => self.docs[self.active].editor.inverse_bookmarks(),
            _ => self.run_edit(ctx, action),
        }
        if focus_editor {
            self.focus_editor = true;
        }
    }

    /// Editing, folding, view and macro commands.
    fn run_edit(&mut self, ctx: &egui::Context, action: Action) {
        let edge = self.settings.edge_column;
        let syntax = self.docs[self.active].syntax.clone();
        let editor = &mut self.docs[self.active].editor;
        match action {
            Action::Undo => editor.undo(),
            Action::Redo => editor.redo(),
            Action::Cut => {
                if let Some(t) = editor.cut(ctx) {
                    ctx.copy_text(t);
                }
            }
            Action::Copy => {
                if let Some(t) = editor.selected_text() {
                    ctx.copy_text(t);
                }
            }
            Action::Paste => ctx.send_viewport_cmd(ViewportCommand::RequestPaste),
            Action::SelectAll => editor.select_all(),
            Action::Duplicate => editor.duplicate(ctx),
            Action::DeleteLines => editor.delete_lines(ctx),
            Action::MoveLinesUp => editor.move_lines(ctx, true),
            Action::MoveLinesDown => editor.move_lines(ctx, false),
            Action::JoinLines => editor.join_lines(ctx),
            Action::SplitLines => editor.split_lines(ctx, edge),
            Action::InsertBlankAbove => editor.insert_blank_line(ctx, true),
            Action::InsertBlankBelow => editor.insert_blank_line(ctx, false),
            Action::SortLines { kind, desc } => editor.sort_lines(ctx, kind, desc),
            Action::ReverseLines => editor.reverse_lines(ctx),
            Action::RemoveDuplicateLines => editor.remove_duplicate_lines(ctx, false),
            Action::RemoveConsecutiveDuplicates => editor.remove_duplicate_lines(ctx, true),
            Action::RemoveEmptyLines { blank } => editor.remove_empty_lines(ctx, blank),
            Action::Trim(kind) => editor.trim_lines(ctx, kind),
            Action::EolToSpace => editor.eol_to_space(ctx),
            Action::TabsToSpaces => editor.tabs_to_spaces(ctx),
            Action::SpacesToTabs { all } => editor.spaces_to_tabs(ctx, all),
            Action::ConvertCase(kind) => editor.convert_case(ctx, kind),
            Action::ToggleLineComment => editor.toggle_line_comment(ctx, &comment_tokens(&syntax)),
            Action::ToggleBlockComment => {
                editor.toggle_block_comment(ctx, &comment_tokens(&syntax))
            }
            Action::InsertDateTime { long } => {
                let now = chrono::Local::now();
                let text = if long {
                    now.format("%A, %-d %B %Y %H:%M:%S").to_string()
                } else {
                    now.format("%Y-%m-%d %H:%M").to_string()
                };
                editor.insert_text(ctx, &text);
            }
            Action::SelectNextOccurrence => editor.select_next_occurrence(),
            Action::SelectAllOccurrences => {
                editor.select_all_occurrences();
            }
            Action::ColumnEditor => self.column.show_dialog(),
            Action::ToggleReadOnly => editor.buffer.read_only = !editor.buffer.read_only,
            Action::FoldAll => editor.fold_all(),
            Action::UnfoldAll => editor.unfold_all(),
            Action::FoldCurrent => editor.fold_current(true),
            Action::UnfoldCurrent => editor.fold_current(false),
            Action::FoldLevel(level) => editor.fold_level(level),
            Action::ToggleWhitespace => {
                self.settings.show_whitespace = !self.settings.show_whitespace
            }
            Action::ToggleEol => self.settings.show_eol = !self.settings.show_eol,
            Action::ToggleIndentGuides => {
                self.settings.indent_guides = !self.settings.indent_guides
            }
            Action::ToggleEdge => self.settings.show_edge = !self.settings.show_edge,
            Action::ToggleAutoClose => self.settings.auto_close = !self.settings.auto_close,
            Action::ToggleSmartIndent => self.settings.smart_indent = !self.settings.smart_indent,
            Action::ToggleUseSpaces => self.settings.use_spaces = !self.settings.use_spaces,
            Action::ToggleCompletion => {
                self.settings.word_completion = !self.settings.word_completion
            }
            Action::ToggleMacroRecording => self.recorder.toggle(),
            Action::PlayMacro => {
                if !self.recorder.is_recording() {
                    let steps = self.recorder.current.clone();
                    self.play_macro(ctx, &steps);
                }
            }
            Action::RunMacroMultiple => {
                if !self.recorder.is_recording() {
                    self.run_macro.show_dialog();
                }
            }
            Action::SaveMacro => {
                if !self.recorder.current.is_empty() {
                    self.save_macro.show_dialog();
                }
            }
            Action::PlaySavedMacro(i) => {
                if !self.recorder.is_recording()
                    && let Some(m) = self.settings.macros.get(i)
                {
                    let steps = m.steps.clone();
                    self.play_macro(ctx, &steps);
                }
            }
            Action::DeleteSavedMacro(i) if i < self.settings.macros.len() => {
                self.settings.macros.remove(i);
            }
            _ => {}
        }
    }

    /// Plays a macro once against the active document.
    fn play_macro(&mut self, ctx: &egui::Context, steps: &[MacroStep]) {
        for step in steps {
            let editor = &mut self.docs[self.active].editor;
            match step {
                MacroStep::Text(t) => {
                    // Char by char, so auto-closed brackets behave as they did.
                    for c in t.chars() {
                        editor.type_text(ctx, c.encode_utf8(&mut [0; 4]));
                    }
                }
                MacroStep::Key { .. } => {
                    if let Some((key, mods)) = step.key() {
                        editor.replay_key(ctx, key, mods);
                    }
                }
                MacroStep::Paste(t) => editor.paste(ctx, t),
                MacroStep::Copy => {
                    if let Some(t) = editor.selected_text() {
                        ctx.copy_text(t);
                    }
                }
                MacroStep::Cut => {
                    if let Some(t) = editor.cut(ctx) {
                        ctx.copy_text(t);
                    }
                }
                MacroStep::Action(a) => self.run(ctx, a.clone()),
            }
        }
    }

    /// Plays the current macro several times, or until it reaches the end of
    /// the document or stops changing anything.
    fn play_macro_times(&mut self, ctx: &egui::Context, times: RunTimes) {
        let steps = self.recorder.current.clone();
        if steps.is_empty() {
            return;
        }
        let limit = match times {
            RunTimes::Count(n) => n,
            RunTimes::ToEnd => 100_000,
        };
        for _ in 0..limit {
            let state = |app: &Self| {
                let e = &app.docs[app.active].editor;
                (e.buffer.version(), e.selections.primary().head)
            };
            let before = state(self);
            self.play_macro(ctx, &steps);
            if times == RunTimes::ToEnd {
                let after = state(self);
                let editor = &self.docs[self.active].editor;
                if after == before || after.1 >= editor.buffer.len_chars() {
                    break;
                }
            }
        }
    }

    /// Inserts the column editor's text or numbers.
    fn column_insert(&mut self, ctx: &egui::Context, what: crate::tools_ui::ColumnInsert) {
        let editor = &mut self.docs[self.active].editor;
        let count = if editor.caret_count() > 1 {
            editor.caret_count()
        } else {
            let (line, _) = editor.caret_line_col();
            editor.buffer.len_lines() - (line - 1)
        };
        let values = what.values(count);
        editor.column_insert(ctx, |i| values.get(i).cloned().unwrap_or_default());
    }

    /// Records the keys typed into the editor while a macro is recording.
    fn record_editor_input(&mut self, ctx: &egui::Context) {
        if !self.recorder.is_recording() {
            return;
        }
        let Some(doc) = self.docs.get(self.active) else {
            return;
        };
        let id = Id::new(("editor", doc.id));
        if !ctx.memory(|m| m.has_focus(id)) {
            return;
        }
        let steps: Vec<MacroStep> =
            ctx.input(|i| i.events.iter().filter_map(MacroStep::from_event).collect());
        for step in steps {
            self.recorder.push(step);
        }
    }

    fn any_dialog_open(&self) -> bool {
        self.close_queue.is_some()
            || self.show_about
            || self.goto.open
            || self.fif.open
            || self.column.open
            || self.run_macro.open
            || self.save_macro.open
    }

    // ---------------------------------------------------------------------
    // Search

    /// The selection to seed a search with (if it is on one line), and whether
    /// the selection spans several lines.
    fn selection_seed(&self) -> (Option<String>, bool) {
        let Some(doc) = self.docs.get(self.active) else {
            return (None, false);
        };
        let sel = doc.editor.selections.primary();
        if sel.is_empty() {
            return (None, false);
        }
        let text = doc.editor.buffer.slice_string(sel.range());
        if text.contains('\n') {
            (None, true)
        } else {
            (Some(text), false)
        }
    }

    fn query(&mut self) -> Option<Query> {
        let opts = self.settings.search;
        if self.find_bar.find.is_empty() {
            return None;
        }
        self.find_bar.query(&opts)
    }

    fn find_step(&mut self, forward: bool) {
        if self.find_bar.find.is_empty() {
            self.run_find_bar_open();
            return;
        }
        self.find_bar.message = None;
        let Some(q) = self.query() else {
            self.find_bar.open = true;
            return;
        };
        let wrap = self.settings.search.wrap;
        let doc = &mut self.docs[self.active];
        let outcome = doc.editor.find_next(&q, forward, wrap);
        self.find_bar.message = match outcome {
            FindOutcome::Found => None,
            FindOutcome::Wrapped if forward => Some("Wrapped to the top".to_owned()),
            FindOutcome::Wrapped => Some("Wrapped to the bottom".to_owned()),
            FindOutcome::NotFound => Some("Not found".to_owned()),
        };
        self.find_bar.origin = Some((doc.id, doc.editor.selections.primary().start()));
    }

    fn run_find_bar_open(&mut self) {
        let (seed, multi_line) = self.selection_seed();
        self.find_bar.show_bar(false, seed, multi_line);
    }

    /// Search as you type: select the first match from where the search started.
    fn find_incremental(&mut self) {
        self.find_bar.message = None;
        if self.find_bar.in_selection || self.find_bar.find.is_empty() {
            return;
        }
        let Some(q) = self.query() else { return };
        let wrap = self.settings.search.wrap;
        let doc = &mut self.docs[self.active];
        let origin = match self.find_bar.origin {
            Some((id, pos)) if id == doc.id => pos,
            _ => {
                let pos = doc.editor.selections.primary().start();
                self.find_bar.origin = Some((doc.id, pos));
                pos
            }
        };
        doc.editor.find_incremental(&q, origin, wrap);
    }

    fn replace_one(&mut self, ctx: &egui::Context) {
        self.find_bar.message = None;
        let Some(q) = self.query() else { return };
        let wrap = self.settings.search.wrap;
        let template = self.find_bar.replace.clone();
        let (_, outcome) = self.docs[self.active]
            .editor
            .replace_current(ctx, &q, &template, wrap);
        if outcome == FindOutcome::NotFound {
            self.find_bar.message = Some("No more matches".to_owned());
        }
    }

    fn replace_all(&mut self, ctx: &egui::Context) {
        let Some(q) = self.query() else { return };
        let template = self.find_bar.replace.clone();
        let in_selection = self.find_bar.in_selection;
        let doc = &mut self.docs[self.active];
        let n = doc.editor.replace_all(ctx, &q, &template, in_selection);
        self.find_bar.message_at = Some((doc.id, doc.editor.buffer.version()));
        self.find_bar.message = Some(match n {
            0 => "No results".to_owned(),
            1 => "1 replaced".to_owned(),
            n => format!("{n} replaced"),
        });
    }

    /// Lists every matching line in the results panel.
    fn find_all(&mut self, all_tabs: bool) {
        let Some(q) = self.query() else { return };
        let title = format!("\"{}\"", self.find_bar.find);
        self.results.start(title);
        let mut remaining = search::MAX_HITS;
        let indices: Vec<usize> = if all_tabs {
            (0..self.docs.len()).collect()
        } else {
            vec![self.active]
        };
        for i in indices {
            let doc = &mut self.docs[i];
            let (hits, cut) = doc.editor.line_hits(&q, remaining);
            remaining -= hits.iter().map(|h| h.matches).sum::<usize>();
            if !hits.is_empty() {
                let label = doc
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| doc.title());
                self.results.add_group(Source::Doc(doc.id), label, hits);
            }
            if cut || remaining == 0 {
                self.results.truncated = true;
                break;
            }
        }
    }

    fn start_file_search(&mut self, req: FifRequest) {
        self.settings.fif_dir = req.dir.display().to_string();
        self.settings.fif_filters = req.filters.clone();
        self.settings.fif_subfolders = req.subfolders;
        self.settings.fif_hidden = req.hidden;
        self.settings.search = req.opts;
        let Ok(Some(query)) = Query::new(&req.find, &req.opts) else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let search = FileSearch {
            dir: req.dir.clone(),
            filters: req.filters,
            subfolders: req.subfolders,
            hidden: req.hidden,
        };
        let worker_cancel = cancel.clone();
        let spawned = std::thread::Builder::new()
            .name("find-in-files".into())
            .spawn(move || search::search_files(search, query, worker_cancel, tx));
        if let Err(e) = spawned {
            self.errors.push(format!("Could not start the search: {e}"));
            return;
        }
        self.results
            .start(format!("\"{}\" in {}", req.find, req.dir.display()));
        self.results.job = Some(FileJob { rx, cancel });
    }

    /// Opens a search result.
    fn jump_to(&mut self, jump: Jump) {
        let index = match &jump.source {
            Source::Doc(id) => self.docs.iter().position(|d| d.id == *id),
            Source::File(path) => {
                self.open_path(path, true);
                let path = path.canonicalize().unwrap_or_else(|_| path.clone());
                self.docs
                    .iter()
                    .position(|d| d.path.as_deref() == Some(path.as_path()))
            }
        };
        if let Some(i) = index {
            self.select(i);
            self.docs[i].editor.select_at(jump.line, jump.col, jump.len);
        }
    }

    /// Keeps the find bar's match highlights and count up to date.
    fn update_search_hits(&mut self) {
        if !self.find_bar.open {
            return;
        }
        let opts = self.settings.search;
        let Some(doc) = self.docs.get(self.active) else {
            return;
        };
        let (id, version) = (doc.id, doc.editor.buffer.version());
        // A "N replaced" note goes away once the text changes again (e.g. undo).
        if self
            .find_bar
            .message_at
            .is_some_and(|at| at != (id, version))
        {
            self.find_bar.message = None;
            self.find_bar.message_at = None;
        }
        if self.find_bar.hits_stale(id, version, &opts) {
            let query = if self.find_bar.find.is_empty() {
                None
            } else {
                match Query::new(&self.find_bar.find, &opts) {
                    Ok(q) => q,
                    Err(e) => {
                        self.find_bar.message = Some(format!("Invalid regex: {e}"));
                        None
                    }
                }
            };
            let editor = &mut self.docs[self.active].editor;
            match query {
                Some(q) => {
                    let (ranges, more) = editor.match_ranges(&q, MAX_HIGHLIGHTS);
                    self.find_bar.count = Some((ranges.len(), more));
                    editor.search_hits = Arc::new(ranges);
                }
                None => {
                    self.find_bar.count = None;
                    editor.search_hits = Arc::default();
                }
            }
            self.find_bar.set_hits_key(id, version, opts);
        }
        let editor = &self.docs[self.active].editor;
        let sel = editor.selections.primary();
        self.find_bar.current = editor
            .search_hits
            .binary_search_by_key(&sel.start(), |r| r.start)
            .ok()
            .filter(|&i| editor.search_hits[i].end == sel.end());
    }

    fn apply_style(&mut self, ctx: &egui::Context) {
        if self.applied_theme == Some(self.theme_idx) {
            return;
        }
        let palette = self.palette();
        let theme = if palette.dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
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
        if !self.any_dialog_open() {
            let mut fired = Vec::new();
            ctx.input_mut(|i| {
                for (shortcut, action) in actions::shortcuts() {
                    // A key can repeat more than once in a frame.
                    while i.consume_shortcut(&shortcut) {
                        fired.push(action.clone());
                    }
                }
                // Alt shortcuts also arrive as typed characters; don't insert them.
                if !fired.is_empty() {
                    i.events.retain(|e| !matches!(e, egui::Event::Text(_)));
                }
            });
            self.pending.extend(fired);
        }

        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        self.pending
            .extend(dropped.into_iter().map(Action::OpenPath));

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
            ui.menu_button("Edit", |ui| self.edit_menu(ui));
            ui.menu_button("Search", |ui| {
                self.menu_item(ui, "Find…", Action::Find);
                self.menu_item(ui, "Replace…", Action::Replace);
                self.menu_item(ui, "Find Next", Action::FindNext);
                self.menu_item(ui, "Find Previous", Action::FindPrev);
                self.menu_item(ui, "Find Selected Next", Action::FindSelectedNext);
                self.menu_item(ui, "Find Selected Previous", Action::FindSelectedPrev);
                self.menu_item(ui, "Find in Files…", Action::FindInFiles);
                ui.separator();
                self.menu_item(ui, "Find All in Current Document", Action::FindAllInDoc);
                self.menu_item(ui, "Find All in Open Tabs", Action::FindAllInTabs);
                self.menu_item(ui, "Select All Matches", Action::SelectAllMatches);
                self.menu_item(ui, "Mark All", Action::MarkAll { bookmark: false });
                self.menu_item(ui, "Clear All Marks", Action::ClearMarks);
                ui.separator();
                self.menu_item(ui, "Go to Line…", Action::GoToLine);
                ui.menu_button("Bookmark", |ui| {
                    self.menu_item(ui, "Toggle Bookmark", Action::ToggleBookmark);
                    self.menu_item(ui, "Next Bookmark", Action::NextBookmark);
                    self.menu_item(ui, "Previous Bookmark", Action::PrevBookmark);
                    self.menu_item(ui, "Clear All Bookmarks", Action::ClearBookmarks);
                    ui.separator();
                    self.menu_item(ui, "Cut Bookmarked Lines", Action::CutBookmarkedLines);
                    self.menu_item(ui, "Copy Bookmarked Lines", Action::CopyBookmarkedLines);
                    self.menu_item(ui, "Remove Bookmarked Lines", Action::RemoveBookmarkedLines);
                    self.menu_item(
                        ui,
                        "Remove Unbookmarked Lines",
                        Action::RemoveUnbookmarkedLines,
                    );
                    self.menu_item(ui, "Inverse Bookmarks", Action::InverseBookmarks);
                });
            });
            ui.menu_button("View", |ui| {
                let s = self.settings.clone();
                self.toggle_item(ui, "Word Wrap", s.word_wrap, Action::ToggleWordWrap);
                self.toggle_item(
                    ui,
                    "Line Numbers",
                    s.show_line_numbers,
                    Action::ToggleLineNumbers,
                );
                self.toggle_item(
                    ui,
                    "Highlight Current Line",
                    s.highlight_current_line,
                    Action::ToggleCurrentLine,
                );
                ui.separator();
                ui.menu_button("Show Symbol", |ui| {
                    self.toggle_item(
                        ui,
                        "Show Whitespace",
                        s.show_whitespace,
                        Action::ToggleWhitespace,
                    );
                    self.toggle_item(ui, "Show End of Line", s.show_eol, Action::ToggleEol);
                    self.toggle_item(
                        ui,
                        "Indent Guides",
                        s.indent_guides,
                        Action::ToggleIndentGuides,
                    );
                });
                self.toggle_item(
                    ui,
                    &format!("Long-Line Marker (column {})", s.edge_column),
                    s.show_edge,
                    Action::ToggleEdge,
                );
                ui.menu_button("Folding", |ui| {
                    self.menu_item(ui, "Fold All", Action::FoldAll);
                    self.menu_item(ui, "Unfold All", Action::UnfoldAll);
                    self.menu_item(ui, "Fold Current Level", Action::FoldCurrent);
                    self.menu_item(ui, "Unfold Current Level", Action::UnfoldCurrent);
                    ui.separator();
                    for level in 1..=5 {
                        self.menu_item(
                            ui,
                            &format!("Fold Level {level}"),
                            Action::FoldLevel(level),
                        );
                    }
                });
                ui.separator();
                self.menu_item(ui, "Zoom In", Action::ZoomIn);
                self.menu_item(ui, "Zoom Out", Action::ZoomOut);
                self.menu_item(ui, "Reset Zoom", Action::ZoomReset);
            });
            ui.menu_button("Theme", |ui| {
                let current = self.theme_idx;
                let names: Vec<(String, bool, bool)> = self
                    .themes
                    .iter()
                    .map(|t| (t.name.clone(), t.palette.dark, t.custom))
                    .collect();
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
            ui.menu_button("Settings", |ui| {
                let s = self.settings.clone();
                self.toggle_item(
                    ui,
                    "Auto-Close Brackets and Quotes",
                    s.auto_close,
                    Action::ToggleAutoClose,
                );
                self.toggle_item(
                    ui,
                    "Smart Indentation",
                    s.smart_indent,
                    Action::ToggleSmartIndent,
                );
                self.toggle_item(
                    ui,
                    "Indent With Spaces",
                    s.use_spaces,
                    Action::ToggleUseSpaces,
                );
                self.toggle_item(
                    ui,
                    "Word Completion",
                    s.word_completion,
                    Action::ToggleCompletion,
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Long-line column");
                    ui.add(egui::DragValue::new(&mut self.settings.edge_column).range(10..=400));
                });
            });
            ui.menu_button("Macro", |ui| self.macro_menu(ui));
            ui.menu_button("Help", |ui| {
                self.menu_item(ui, "About NixPad", Action::About);
            });
        });
    }

    fn edit_menu(&mut self, ui: &mut Ui) {
        let read_only = self
            .docs
            .get(self.active)
            .is_some_and(|d| d.editor.buffer.read_only);
        self.menu_item(ui, "Undo", Action::Undo);
        self.menu_item(ui, "Redo", Action::Redo);
        ui.separator();
        self.menu_item(ui, "Cut", Action::Cut);
        self.menu_item(ui, "Copy", Action::Copy);
        self.menu_item(ui, "Paste", Action::Paste);
        self.menu_item(ui, "Select All", Action::SelectAll);
        ui.separator();
        ui.menu_button("Line Operations", |ui| {
            self.menu_item(ui, "Duplicate", Action::Duplicate);
            self.menu_item(ui, "Delete Lines", Action::DeleteLines);
            self.menu_item(ui, "Move Up", Action::MoveLinesUp);
            self.menu_item(ui, "Move Down", Action::MoveLinesDown);
            self.menu_item(ui, "Join Lines", Action::JoinLines);
            self.menu_item(ui, "Split Lines at Long-Line Column", Action::SplitLines);
            self.menu_item(ui, "Insert Blank Line Above", Action::InsertBlankAbove);
            self.menu_item(ui, "Insert Blank Line Below", Action::InsertBlankBelow);
            ui.separator();
            for (label, kind) in [
                ("Lexicographically", SortKind::Lexical),
                ("Ignoring Case", SortKind::IgnoreCase),
                ("As Integers", SortKind::Integer),
                ("As Decimals", SortKind::Decimal),
                ("By Length", SortKind::Length),
            ] {
                self.menu_item(
                    ui,
                    &format!("Sort {label} Ascending"),
                    Action::SortLines { kind, desc: false },
                );
                self.menu_item(
                    ui,
                    &format!("Sort {label} Descending"),
                    Action::SortLines { kind, desc: true },
                );
            }
            self.menu_item(ui, "Reverse Line Order", Action::ReverseLines);
            ui.separator();
            self.menu_item(ui, "Remove Duplicate Lines", Action::RemoveDuplicateLines);
            self.menu_item(
                ui,
                "Remove Consecutive Duplicate Lines",
                Action::RemoveConsecutiveDuplicates,
            );
            self.menu_item(
                ui,
                "Remove Empty Lines",
                Action::RemoveEmptyLines { blank: false },
            );
            self.menu_item(
                ui,
                "Remove Empty Lines (Including Blank Ones)",
                Action::RemoveEmptyLines { blank: true },
            );
        });
        ui.menu_button("Convert Case To", |ui| {
            self.menu_item(ui, "UPPERCASE", Action::ConvertCase(CaseKind::Upper));
            self.menu_item(ui, "lowercase", Action::ConvertCase(CaseKind::Lower));
            self.menu_item(ui, "Title Case", Action::ConvertCase(CaseKind::Title));
            self.menu_item(ui, "Sentence case", Action::ConvertCase(CaseKind::Sentence));
            self.menu_item(ui, "iNVERT cASE", Action::ConvertCase(CaseKind::Invert));
        });
        ui.menu_button("Comment", |ui| {
            self.menu_item(ui, "Toggle Line Comment", Action::ToggleLineComment);
            self.menu_item(ui, "Toggle Block Comment", Action::ToggleBlockComment);
        });
        ui.menu_button("Blank Operations", |ui| {
            self.menu_item(ui, "Trim Trailing Space", Action::Trim(TrimKind::Trailing));
            self.menu_item(ui, "Trim Leading Space", Action::Trim(TrimKind::Leading));
            self.menu_item(
                ui,
                "Trim Leading and Trailing Space",
                Action::Trim(TrimKind::Both),
            );
            self.menu_item(ui, "EOL to Space", Action::EolToSpace);
            ui.separator();
            self.menu_item(ui, "Tabs to Spaces", Action::TabsToSpaces);
            self.menu_item(
                ui,
                "Leading Spaces to Tabs",
                Action::SpacesToTabs { all: false },
            );
            self.menu_item(ui, "All Spaces to Tabs", Action::SpacesToTabs { all: true });
        });
        ui.menu_button("Insert", |ui| {
            self.menu_item(
                ui,
                "Date and Time (Short)",
                Action::InsertDateTime { long: false },
            );
            self.menu_item(
                ui,
                "Date and Time (Long)",
                Action::InsertDateTime { long: true },
            );
        });
        ui.menu_button("Multi-Select", |ui| {
            self.menu_item(ui, "Select Next Occurrence", Action::SelectNextOccurrence);
            self.menu_item(ui, "Select All Occurrences", Action::SelectAllOccurrences);
        });
        self.menu_item(ui, "Column Editor…", Action::ColumnEditor);
        ui.separator();
        self.toggle_item(ui, "Read-Only", read_only, Action::ToggleReadOnly);
    }

    fn macro_menu(&mut self, ui: &mut Ui) {
        let recording = self.recorder.is_recording();
        let has_macro = !self.recorder.current.is_empty();
        let label = if recording {
            "Stop Recording"
        } else {
            "Start Recording"
        };
        self.menu_item(ui, label, Action::ToggleMacroRecording);
        ui.add_enabled_ui(!recording && has_macro, |ui| {
            self.menu_item(ui, "Playback", Action::PlayMacro);
            self.menu_item(ui, "Run a Macro Multiple Times…", Action::RunMacroMultiple);
            self.menu_item(ui, "Save Current Macro…", Action::SaveMacro);
        });
        if !self.settings.macros.is_empty() {
            ui.separator();
            let names: Vec<String> = self
                .settings
                .macros
                .iter()
                .map(|m| m.name.clone())
                .collect();
            ui.add_enabled_ui(!recording, |ui| {
                for (i, name) in names.iter().enumerate() {
                    self.menu_item(ui, name, Action::PlaySavedMacro(i));
                }
            });
            ui.menu_button("Delete Saved Macro", |ui| {
                for (i, name) in names.iter().enumerate() {
                    self.menu_item(ui, name, Action::DeleteSavedMacro(i));
                }
            });
        }
    }

    /// Languages grouped by first letter, like Notepad++'s Language menu.
    fn language_menu(&mut self, ui: &mut Ui) {
        let current = self
            .docs
            .get(self.active)
            .map(|d| d.syntax.clone())
            .unwrap_or_default();
        self.toggle_item(
            ui,
            PLAIN_TEXT,
            current == PLAIN_TEXT,
            Action::SetLanguage(PLAIN_TEXT.to_owned()),
        );
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
                for name in languages.iter().filter(|l| {
                    l.as_str() != PLAIN_TEXT
                        && l.chars().next().map(|c| c.to_ascii_uppercase()) == Some(letter)
                }) {
                    self.toggle_item(
                        ui,
                        name,
                        *name == current,
                        Action::SetLanguage(name.clone()),
                    );
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
                        egui::Button::new(RichText::new("+").size(16.0).color(palette.text_dim))
                            .frame(false),
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
        let text_color = if active {
            palette.text
        } else {
            palette.text_dim
        };
        let galley = ui.painter().layout_no_wrap(title, font, text_color);
        let pad = 14.0;
        let close_size = 18.0;
        let width = (pad + galley.size().x + 8.0 + close_size + 8.0).max(90.0);

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        let close_rect = Rect::from_center_size(
            Pos2::new(rect.right() - 8.0 - close_size / 2.0, rect.center().y),
            Vec2::splat(close_size),
        );
        let close_response =
            ui.interact(close_rect, Id::new(("tab-close", doc.id)), Sense::click());

        let painter = ui.painter();
        let radius = CornerRadius {
            nw: 6,
            ne: 6,
            sw: 0,
            se: 0,
        };
        if active {
            painter.rect_filled(rect, radius, palette.editor_bg);
            painter.rect_filled(
                Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), 2.0)),
                CornerRadius {
                    nw: 6,
                    ne: 6,
                    sw: 0,
                    se: 0,
                },
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
        let show_close =
            close_response.hovered() || response.hovered() || active || !doc.is_dirty();
        if close_response.hovered() {
            painter.rect_filled(
                close_rect,
                CornerRadius::same(4),
                palette.tab_hover_bg.gamma_multiply(1.6),
            );
        }
        if doc.is_dirty() && !close_response.hovered() {
            painter.circle_filled(close_rect.center(), 4.0, palette.modified);
        } else if show_close {
            let c = close_rect.center();
            let d = 4.0;
            let stroke = Stroke::new(
                1.4,
                if close_response.hovered() {
                    palette.text
                } else {
                    palette.text_dim
                },
            );
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
                    ui.ctx().copy_text(
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        });
    }

    fn editor(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let theme_def = &self.themes[self.theme_idx];
        let request_focus = std::mem::take(&mut self.focus_editor) && !self.any_dialog_open();
        let Some(doc) = self.docs.get_mut(self.active) else {
            return;
        };
        let syntax = self.highlighter.syntax(&doc.syntax);
        let s = &self.settings;
        doc.editor.opts = EditOptions {
            auto_close: s.auto_close,
            smart_indent: s.smart_indent,
            use_spaces: s.use_spaces,
            completion: s.word_completion && !self.recorder.is_recording(),
            colon_indents: matches!(doc.syntax.as_str(), "Python" | "YAML" | "Nim"),
            markup: matches!(
                doc.syntax.as_str(),
                "HTML" | "XML" | "SVG" | "Vue Component" | "PHP" | "ASP"
            ),
        };
        let eol_label = doc.line_ending.short_label();
        let cfg = ViewConfig {
            font: FontId::monospace(self.settings.font_size),
            wrap: self.settings.word_wrap,
            show_line_numbers: self.settings.show_line_numbers,
            highlight_current_line: self.settings.highlight_current_line,
            palette: &palette,
            syntax,
            syntaxes: &self.highlighter.syntaxes,
            theme: &theme_def.syntax,
            theme_name: &theme_def.name,
            plain: doc.syntax == PLAIN_TEXT,
            request_focus,
            show_whitespace: s.show_whitespace,
            show_eol: s.show_eol,
            eol_label,
            indent_guides: s.indent_guides,
            edge_column: s.show_edge.then_some(s.edge_column),
        };
        doc.editor.show(ui, Id::new(("editor", doc.id)), &cfg);
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let Some(doc) = self.docs.get(self.active) else {
            return;
        };
        let (line, col) = doc.editor.caret_line_col();
        let sel = doc.editor.selected_chars();
        let carets = doc.editor.caret_count();
        let syntax = doc.syntax.clone();
        let line_ending = doc.line_ending;
        let encoding = if doc.has_bom { "UTF-8 BOM" } else { "UTF-8" };
        let length = doc.editor.buffer.len_chars();
        let lines = doc.line_count();
        let zoom = (self.settings.font_size / DEFAULT_FONT * 100.0).round();
        let read_only = doc.editor.buffer.read_only;
        let recording = self.recorder.is_recording();
        let dim = |s: String| RichText::new(s).size(12.0).color(palette.text_dim);

        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.menu_button(dim(syntax.clone()), |ui| {
                ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    for name in self.languages.clone() {
                        self.toggle_item(
                            ui,
                            &name,
                            name == syntax,
                            Action::SetLanguage(name.clone()),
                        );
                    }
                });
            });
            ui.label(dim(format!("length: {length}   lines: {lines}")));
            if recording {
                ui.label(
                    RichText::new("● Recording macro")
                        .size(12.0)
                        .color(palette.modified),
                );
            }
            if read_only
                && ui
                    .add(egui::Label::new(dim("Read-only".to_owned())).sense(Sense::click()))
                    .on_hover_text("Click to allow editing")
                    .clicked()
            {
                self.pending.push(Action::ToggleReadOnly);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(4.0);
                ui.label(dim(format!("{zoom}%")));
                ui.label(dim(encoding.to_owned()));
                ui.menu_button(dim(line_ending.label().to_owned()), |ui| {
                    for le in [LineEnding::CrLf, LineEnding::Lf, LineEnding::Cr] {
                        self.toggle_item(
                            ui,
                            le.label(),
                            le == line_ending,
                            Action::SetLineEnding(le),
                        );
                    }
                });
                let mut position = format!("Ln {line}, Col {col}");
                if sel > 0 {
                    position += &format!("   Sel {sel}");
                }
                if carets > 1 {
                    position += &format!("   {carets} carets");
                }
                if ui
                    .add(egui::Label::new(dim(position)).sense(Sense::click()))
                    .on_hover_text("Go to line (Ctrl+G)")
                    .clicked()
                {
                    self.pending.push(Action::GoToLine);
                }
            });
        });
    }

    /// Steps through the close queue, asking about one unsaved document at a time.
    fn close_queue_ui(&mut self, ctx: &egui::Context) {
        loop {
            let Some(queue) = &mut self.close_queue else {
                return;
            };
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
                Some(i) if self.docs[i].is_dirty() => {
                    let name = self.docs[i].title();
                    let mut choice = None;
                    egui::Modal::new(Id::new("unsaved-changes")).show(ctx, |ui| {
                        ui.set_width(360.0);
                        ui.heading("Save changes?");
                        ui.add_space(6.0);
                        ui.label(format!(
                            "\"{name}\" has unsaved changes. Save them before closing?"
                        ));
                        ui.add_space(12.0);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Cancel").clicked()
                                || ui.input(|i| i.key_pressed(Key::Escape))
                            {
                                choice = Some(0);
                            }
                            if ui.button("Don't Save").clicked() {
                                choice = Some(1);
                            }
                            if ui
                                .add(
                                    egui::Button::new("Save")
                                        .fill(self.palette().accent.gamma_multiply(0.6)),
                                )
                                .clicked()
                            {
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
        if !quitting {
            self.remove_doc(id);
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let palette = self.palette();
        if let Some(doc) = self.docs.get(self.active) {
            let lines = doc.line_count();
            if let Some((line, col)) = self.goto.ui(ctx, lines, &palette) {
                self.docs[self.active].editor.go_to(line, col);
                self.focus_editor = true;
            }
        }
        if let Some(req) = self.fif.ui(ctx, &mut self.settings.search, &palette) {
            self.start_file_search(req);
        }
        if let Some(what) = self.column.ui(ctx, &palette) {
            self.column_insert(ctx, what);
            self.focus_editor = true;
        }
        let has_macro = !self.recorder.current.is_empty();
        if let Some(times) = self.run_macro.ui(ctx, has_macro) {
            self.play_macro_times(ctx, times);
            self.focus_editor = true;
        }
        if let Some(name) = self.save_macro.ui(ctx) {
            let steps = self.recorder.current.clone();
            match self.settings.macros.iter_mut().find(|m| m.name == name) {
                Some(m) => m.steps = steps,
                None => self.settings.macros.push(SavedMacro { name, steps }),
            }
            self.focus_editor = true;
        }

        if self.show_about {
            let response = egui::Modal::new(Id::new("about")).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("NixPad")
                            .size(26.0)
                            .strong()
                            .color(self.palette().accent),
                    );
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
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.button("OK").clicked()
                })
                .inner
            });
            if response.inner || response.should_close() {
                self.errors.remove(0);
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match self.docs.get(self.active) {
            Some(doc) => {
                let name = doc
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| doc.title());
                format!("{}{} - NixPad", if doc.is_dirty() { "*" } else { "" }, name)
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
            .frame(
                Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(6, 4)),
            )
            .show_separator_line(false)
            .show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("tab-bar")
            .frame(Frame::new().fill(palette.chrome_bg).inner_margin(Margin {
                left: 6,
                right: 6,
                top: 2,
                bottom: 0,
            }))
            .show_separator_line(false)
            .show(ui, |ui| self.tab_bar(ui));
        egui::Panel::bottom("status-bar")
            .frame(
                Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(10, 3)),
            )
            .show_separator_line(false)
            .show(ui, |ui| self.status_bar(ui));
        if self.results.open {
            if self.results.poll() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            let mut jump = None;
            egui::Panel::bottom("results")
                .resizable(true)
                .default_size(220.0)
                .min_size(80.0)
                .frame(
                    Frame::new()
                        .fill(palette.chrome_bg)
                        .inner_margin(Margin::symmetric(8, 4)),
                )
                .show(ui, |ui| jump = self.results.ui(ui, &palette));
            if let Some(j) = jump {
                self.jump_to(j);
            }
        }
        if self.find_bar.open {
            let mut actions = Vec::new();
            let mut changed = false;
            egui::Panel::top("find-bar")
                .frame(
                    Frame::new()
                        .fill(palette.chrome_bg)
                        .inner_margin(Margin::symmetric(8, 5)),
                )
                .show_separator_line(false)
                .show(ui, |ui| {
                    changed =
                        self.find_bar
                            .ui(ui, &mut self.settings.search, &palette, &mut actions)
                });
            if changed {
                self.find_incremental();
            }
            self.pending.extend(actions);
            self.update_search_hits();
        }
        self.record_editor_input(&ctx);
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
