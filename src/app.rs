//! The Ettin Edit window: menu bar, tab strip, editor and status bar.

mod commands;
mod compare;
mod csv_query;
mod languages;
mod menus;
mod panels;
mod plugins;
mod printing;
mod text_tools;
mod views;
mod xml_schema;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use egui::{
    Align, CornerRadius, FontId, Frame, Id, Key, Layout, Margin, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, Vec2, ViewportCommand,
};

use crate::actions::{self, Action};
use crate::document::{DiskChange, DiskStamp, Document, LineEnding};
use crate::editor::lines::comment_tokens;
use crate::editor::{EditOptions, FindOutcome, MAX_HIGHLIGHTS, ViewConfig};
use crate::encoding::TextEncoding;
use crate::file_ui::{self, RenameDialog, Summary};
use crate::highlight::{Highlighter, PLAIN_TEXT};
use crate::instance;
use crate::macros::{MacroStep, Recorder, SavedMacro};
use crate::search::{self, FileSearch, Query};
use crate::search_ui::{
    FifDialog, FifRequest, FileJob, FindBar, GotoDialog, Jump, Results, Source,
};
use crate::session::{Session, SessionStore, SessionTab};
use crate::settings::{self, Settings};
use crate::system;
use crate::text_tools_ui::{JsonTree, Notice, XPathPanel};
use crate::theme::{self, Palette, ThemeDef};
use crate::tools_ui::{ColumnDialog, RunMacroDialog, RunTimes, SaveMacroDialog};

const MIN_FONT: f32 = 8.0;
const MAX_FONT: f32 = 48.0;

/// Tabs waiting to be closed, one unsaved-changes prompt at a time.
struct CloseQueue {
    doc_ids: Vec<u64>,
    quit_after: bool,
}

pub struct EttinApp {
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
    /// The app icon shown in the menu bar and About, loaded on first use.
    app_icon: Option<egui::TextureHandle>,
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
    /// Session and backups; only the first Ettin Edit window keeps them.
    session: Option<SessionStore>,
    instance: Option<instance::Listener>,
    last_backup: f64,
    last_disk_check: f64,
    was_focused: bool,
    /// The tab being dragged to a new place.
    drag_tab: Option<u64>,
    rename: RenameDialog,
    trash_confirm: Option<u64>,
    summary: Option<Summary>,
    json_tree: JsonTree,
    csv: crate::csv_query::CsvQueryPanel,
    /// Select the tree node at the caret once the tree is built.
    json_tree_reveal: bool,
    xpath: XPathPanel,
    xml_outline: Option<text_tools::OutlineCache>,
    /// What the last text tool did, shown briefly.
    notice: Option<Notice>,
    split: views::Split,
    /// The open comparison.
    compare: Option<crate::compare_ui::CompareView>,
    /// The tab used before the active one, for Compare.
    previous_tab: Option<u64>,
    /// When Compare to Clipboard asked for the clipboard text.
    clipboard_compare: Option<f64>,
    plugins: crate::plugins::PluginHost,
    plugin_ui: plugins::PluginUi,
    prefs: crate::prefs_ui::PrefsDialog,
    shortcut_mapper: crate::shortcut_ui::ShortcutMapper,
    command_palette: crate::palette_ui::Picker,
    style_editor: crate::style_ui::StyleEditor,
    tab_picker: crate::palette_ui::Picker,
    folders: crate::folder_ui::FolderPanel,
    updates: crate::update::UpdateCheck,
    function_list: crate::function_list_ui::FunctionList,
    /// Full screen with only the text showing.
    distraction_free: bool,
}

/// Seconds between backups of unsaved changes.
const BACKUP_INTERVAL: f64 = 7.0;
/// Seconds between checks for files changed on disk.
const DISK_CHECK_INTERVAL: f64 = 2.0;

impl EttinApp {
    /// Reads the themes folder again and keeps the current theme if it still exists.
    fn reload_themes(&mut self) {
        let name = self.themes[self.theme_idx].name.clone();
        let (themes, errors) = theme::load_themes(settings::themes_dir().as_deref());
        self.errors.extend(errors);
        self.theme_idx = themes.iter().position(|t| t.name == name).unwrap_or(0);
        self.themes = themes;
        self.applied_theme = None;
    }

    /// A theme's `.tmTheme` text: its file in the themes folder, or the built-in copy.
    fn theme_source(&self, name: &str) -> String {
        settings::themes_dir()
            .and_then(|dir| std::fs::read_to_string(theme::theme_path(&dir, name)).ok())
            .or_else(|| {
                theme::BUILTIN
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, s)| (*s).to_owned())
            })
            .unwrap_or_default()
    }

    /// Acts on what the theme colour editor did.
    fn style_events(&mut self, ctx: &egui::Context, events: Vec<crate::style_ui::StyleEvent>) {
        use crate::style_ui::StyleEvent;
        for event in events {
            match event {
                StyleEvent::Preview { name, source } => {
                    if let Some(i) = self.themes.iter().position(|t| t.name == name) {
                        match theme::from_source(&name, &source, self.themes[i].custom) {
                            Ok(def) => {
                                self.themes[i] = def;
                                self.applied_theme = None;
                            }
                            Err(e) => self.errors.push(format!("The theme has a problem: {e}")),
                        }
                    }
                }
                StyleEvent::Save { name, source } => {
                    let Some(dir) = settings::themes_dir() else {
                        continue;
                    };
                    let path = theme::theme_path(&dir, &name);
                    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, source))
                    {
                        Ok(()) => {
                            self.reload_themes();
                            self.run(ctx, Action::SetTheme(name));
                        }
                        Err(e) => self.errors.push(format!(
                            "Couldn't save the theme to {}: {e}",
                            path.display()
                        )),
                    }
                }
                StyleEvent::Discard => self.reload_themes(),
                StyleEvent::Switch(name) => {
                    self.run(ctx, Action::SetTheme(name.clone()));
                    let source = self.theme_source(&name);
                    self.style_editor.load(&name, &source);
                }
            }
        }
        self.apply_style(ctx);
    }

    /// `owner` is false when another Ettin Edit window already runs and owns the session.
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<PathBuf>, owner: bool) -> Self {
        let settings = Settings::load();
        let (themes, mut theme_errors) = theme::load_themes(settings::themes_dir().as_deref());
        let theme_idx = themes
            .iter()
            .position(|t| t.name == settings.theme)
            .unwrap_or(0);
        let (highlighter, language_errors) =
            Highlighter::new(crate::highlight::languages_dir().as_deref());
        theme_errors.extend(language_errors);
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
            errors: theme_errors,
            show_about: false,
            app_icon: None,
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
            session: None,
            instance: None,
            last_backup: 0.0,
            last_disk_check: 0.0,
            was_focused: true,
            drag_tab: None,
            rename: RenameDialog::default(),
            trash_confirm: None,
            summary: None,
            json_tree: JsonTree::default(),
            csv: crate::csv_query::CsvQueryPanel::default(),
            json_tree_reveal: false,
            xpath: XPathPanel::default(),
            xml_outline: None,
            notice: None,
            split: views::Split::default(),
            compare: None,
            previous_tab: None,
            clipboard_compare: None,
            plugins: crate::plugins::PluginHost::new({
                let ctx = cc.egui_ctx.clone();
                std::sync::Arc::new(move || ctx.request_repaint())
            }),
            plugin_ui: plugins::PluginUi::default(),
            prefs: crate::prefs_ui::PrefsDialog::default(),
            shortcut_mapper: crate::shortcut_ui::ShortcutMapper::default(),
            command_palette: crate::palette_ui::Picker::default(),
            style_editor: crate::style_ui::StyleEditor::default(),
            tab_picker: crate::palette_ui::Picker::default(),
            folders: crate::folder_ui::FolderPanel::default(),
            updates: Default::default(),
            function_list: Default::default(),
            distraction_free: false,
        };
        if let Some(dir) = app.settings.workspace.clone() {
            app.folders.set_root(&dir);
        }
        app.fif.dir = app.settings.fif_dir.clone();
        app.fif.filters = app.settings.fif_filters.clone();
        app.fif.subfolders = app.settings.fif_subfolders;
        app.fif.hidden = app.settings.fif_hidden;
        if let Err(e) = crate::fonts::install(&cc.egui_ctx, &app.settings.editor_font) {
            app.errors.push(e);
        }
        crate::editor::layout::set_tab_width(app.settings.tab_width);
        let shortcut_errors = actions::set_overrides(&app.settings.shortcuts);
        app.errors.extend(shortcut_errors);
        app.apply_style(&cc.egui_ctx);
        let now = chrono::Utc::now().timestamp();
        if app.settings.check_updates
            && now - app.settings.last_update_check
                >= crate::update::CHECK_EVERY_DAYS * 24 * 60 * 60
        {
            app.settings.last_update_check = now;
            app.updates.start(&cc.egui_ctx, false);
        }

        if owner && let Some(dir) = settings::config_dir() {
            let ctx = cc.egui_ctx.clone();
            app.instance = instance::listen(&dir, move || ctx.request_repaint());
            let store = SessionStore::new(&dir);
            let session = store.load().unwrap_or_else(|| Session {
                // Sessions from before 0.5 were a list of files in the settings.
                tabs: app
                    .settings
                    .session_files
                    .iter()
                    .map(|p| SessionTab {
                        path: Some(p.clone()),
                        backup: None,
                        untitled_number: 0,
                        encoding: None,
                        line_ending: None,
                        syntax: None,
                        pinned: false,
                        read_only: false,
                        caret: 0,
                        view: 0,
                    })
                    .collect(),
                active: app.settings.session_active,
            });
            app.session = Some(store);
            app.restore_session(session);
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
        let mut doc = Document::new_untitled(id, self.untitled_counter);
        let s = &self.settings;
        if let Some(le) = s.new_line_ending {
            doc.line_ending = le;
        }
        if let Some(enc) = TextEncoding::from_name(&s.new_encoding) {
            doc.encoding = enc;
        }
        if self.languages.contains(&s.new_language) {
            doc.syntax = s.new_language.clone();
        }
        self.add_doc(doc);
    }

    /// Adds a tab to the focused view and selects it.
    fn add_doc(&mut self, mut doc: Document) {
        let view = if self.is_split() {
            self.split.focused
        } else {
            0
        };
        doc.views = [view == 0, view == 1];
        doc.live_view = view;
        self.docs.push(doc);
        self.focus_view(view, self.docs.len() - 1);
    }

    /// Selects a tab, in the focused view when it is shown there.
    fn select(&mut self, index: usize) {
        let Some(doc) = self.docs.get(index) else {
            return;
        };
        let focused = self.split.focused;
        let view = if doc.views[focused] {
            focused
        } else {
            1 - focused
        };
        self.focus_view(view, index);
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
        match Document::open(id, &path, None) {
            Ok(mut doc) => {
                doc.syntax = self
                    .highlighter
                    .detect(doc.path.as_deref(), &doc.editor.buffer.line_text(0));
                if doc.lossy {
                    self.errors.push(format!(
                        "{} has bytes that aren't valid {}. They are shown as \u{FFFD} and will be replaced if you save. If the text looks wrong, pick its encoding in Encoding > Reopen With.",
                        path.display(),
                        doc.encoding.name()
                    ));
                }
                // Opening a file replaces an untouched "new" tab, like most editors.
                let replace = self
                    .docs
                    .get(self.active)
                    .is_some_and(|d| d.is_pristine_untitled());
                if replace {
                    let old = &self.docs[self.active];
                    doc.views = old.views;
                    doc.live_view = old.live_view;
                    self.docs[self.active] = doc;
                    self.fix_views();
                    self.select(self.active);
                } else {
                    self.add_doc(doc);
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
                self.check_after_save(index);
                self.plugin_ui.saved.push(self.docs[index].id);
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
            // The view showing it moves to the tab next to it.
            let focused = self.split.focused;
            let next_active = if i == self.active {
                self.neighbour_in(focused, i)
            } else {
                Some(self.docs[self.active].id)
            };
            if self.split.other_active == Some(id) {
                self.split.other_active = self.neighbour_in(1 - focused, i);
            }
            let doc = self.docs.remove(i);
            if let Some(backup) = doc.backup {
                let _ = std::fs::remove_file(backup);
            }
            self.active = next_active
                .and_then(|id| self.docs.iter().position(|d| d.id == id))
                .unwrap_or(usize::MAX);
            if self.active == usize::MAX && self.is_split() {
                // The focused view is empty: focus the other one.
                self.split.focused = 1 - focused;
                self.active = self.active_in(1 - focused).unwrap_or(0);
            }
            self.fix_views();
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

    // ---------------------------------------------------------------------
    // Session, backups and files changed on disk

    fn restore_session(&mut self, session: Session) {
        let mut active = None;
        for (i, tab) in session.tabs.into_iter().enumerate() {
            // Without session restore, only unsaved work left by a crash comes back.
            if !self.settings.restore_session && tab.backup.is_none() {
                continue;
            }
            if let Some(index) = self.restore_tab(tab)
                && (i <= session.active || active.is_none())
            {
                active = Some(index);
            }
        }
        if let Some(a) = active {
            self.active = a;
            self.split.focused = usize::from(!self.docs[a].views[0]);
        }
        self.fix_views();
    }

    fn restore_tab(&mut self, tab: SessionTab) -> Option<usize> {
        let store = self.session.as_ref()?;
        let backup_text = tab.backup.as_ref().and_then(|b| store.read_backup(b).ok());
        let backup_path = tab.backup.as_ref().map(|b| store.backup_dir().join(b));
        let encoding = tab.encoding.as_deref().and_then(TextEncoding::from_name);
        let id = self.alloc_id();
        let mut doc = match (&tab.path, backup_text) {
            (_, Some(text)) => {
                let mut doc = Document::new_untitled(id, tab.untitled_number);
                doc.editor = crate::editor::Editor::new(&text);
                doc.editor.buffer.mark_modified();
                if let Some(p) = &tab.path {
                    doc.path = Some(p.clone());
                    doc.disk = DiskStamp::read(p).ok();
                }
                if let Some(e) = encoding {
                    doc.encoding = e;
                }
                if let Some(le) = tab.line_ending {
                    doc.line_ending = le;
                }
                doc.backup = backup_path;
                doc.backup_version = doc.editor.buffer.version();
                doc
            }
            (Some(p), None) if p.is_file() => Document::open(id, p, encoding).ok()?,
            _ => return None,
        };
        doc.syntax = tab.syntax.unwrap_or_else(|| {
            self.highlighter
                .detect(doc.path.as_deref(), &doc.editor.buffer.line_text(0))
        });
        doc.pinned = tab.pinned;
        doc.views = match tab.view {
            1 => [false, true],
            2 => [true, true],
            _ => [true, false],
        };
        doc.live_view = usize::from(tab.view == 1);
        doc.editor.buffer.read_only = tab.read_only;
        doc.editor.selections = crate::editor::selection::Selections::caret(
            tab.caret.min(doc.editor.buffer.len_chars()),
        );
        if doc.path.is_none() {
            self.untitled_counter = self.untitled_counter.max(doc.untitled_number);
        }
        // Pinned tabs stay in front.
        let at = if doc.pinned {
            self.docs.iter().filter(|d| d.pinned).count()
        } else {
            self.docs.len()
        };
        self.docs.insert(at, doc);
        Some(at)
    }

    /// The open tabs as a session. Unsaved changes are included only with
    /// `with_backups`; tabs that exist only as unsaved text need a backup.
    fn session_snapshot(&self, with_backups: bool) -> Session {
        let mut tabs = Vec::new();
        let mut active = 0;
        for (i, doc) in self.docs.iter().enumerate() {
            let backup = (with_backups && doc.is_dirty())
                .then(|| doc.backup.as_ref()?.file_name())
                .flatten()
                .map(|n| n.to_string_lossy().into_owned());
            if doc.path.is_none() && backup.is_none() {
                continue;
            }
            if i <= self.active {
                active = tabs.len();
            }
            tabs.push(SessionTab {
                path: doc.path.clone(),
                backup,
                untitled_number: doc.untitled_number,
                encoding: Some(doc.encoding.name()),
                line_ending: Some(doc.line_ending),
                syntax: Some(doc.syntax.clone()),
                pinned: doc.pinned,
                read_only: doc.editor.buffer.read_only,
                caret: doc.editor.selections.primary().head,
                view: match doc.views {
                    [true, true] => 2,
                    [false, true] => 1,
                    _ => 0,
                },
            });
        }
        Session { tabs, active }
    }

    /// Backs up every tab with unsaved changes that changed since its last
    /// backup, drops backups of tabs that are saved again, and records the session.
    fn write_backups(&mut self) {
        let Some(store) = &self.session else {
            return;
        };
        for doc in &mut self.docs {
            if doc.is_dirty() {
                let version = doc.editor.buffer.version();
                if doc.backup.is_some() && doc.backup_version == version {
                    continue;
                }
                let path = doc
                    .backup
                    .clone()
                    .unwrap_or_else(|| store.new_backup_path(&doc.title()));
                match store.write_backup(&path, doc.editor.buffer.rope().chunks()) {
                    Ok(()) => {
                        doc.backup = Some(path);
                        doc.backup_version = version;
                    }
                    Err(e) => eprintln!("ettin-edit: backup to {} failed: {e}", path.display()),
                }
            } else if let Some(path) = doc.backup.take() {
                let _ = std::fs::remove_file(path);
            }
        }
        let session = self.session_snapshot(true);
        if let Some(store) = &mut self.session
            && let Err(e) = store.save(&session)
        {
            eprintln!("ettin-edit: could not save the session: {e}");
        }
    }

    /// Writes the session for the next start and clears out backups it doesn't
    /// need. `remember` keeps unsaved changes for next time.
    fn finish_session(&mut self, remember: bool) {
        if self.session.is_none() {
            return;
        }
        if remember {
            self.write_backups();
        }
        let mut session = self.session_snapshot(remember);
        if !self.settings.restore_session {
            session.tabs.retain(|t| t.backup.is_some());
            session.active = session.active.min(session.tabs.len().saturating_sub(1));
        }
        if let Some(store) = &mut self.session {
            let keep: Vec<PathBuf> = session
                .tabs
                .iter()
                .filter_map(|t| t.backup.as_ref().map(|b| store.backup_dir().join(b)))
                .collect();
            if let Err(e) = store.save(&session) {
                eprintln!("ettin-edit: could not save the session: {e}");
            }
            store.remove_stale_backups(&keep);
        }
    }

    /// Looks for open files that other programs changed or deleted. Runs
    /// every couple of seconds while the window has focus, and when it gets focus.
    fn check_disk(&mut self, ctx: &egui::Context) {
        let (now, focused) = ctx.input(|i| (i.time, i.viewport().focused.unwrap_or(true)));
        let regained = focused && !self.was_focused;
        self.was_focused = focused;
        if !focused {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(DISK_CHECK_INTERVAL));
        if !regained && now - self.last_disk_check < DISK_CHECK_INTERVAL {
            return;
        }
        self.last_disk_check = now;
        for doc in &mut self.docs {
            if doc.disk_change.is_some() {
                continue;
            }
            match doc.check_disk() {
                Some(DiskChange::Modified) if self.settings.auto_reload && !doc.is_dirty() => {
                    if let Err(e) = doc.reload(None) {
                        self.errors
                            .push(format!("Could not reload {}: {e}", doc.title()));
                    }
                }
                Some(change) => doc.disk_change = Some(change),
                None => {}
            }
        }
    }

    /// Opens files sent by Ettin Edit started again, and brings the window forward.
    fn poll_instance(&mut self, ctx: &egui::Context) {
        let Some(listener) = &self.instance else {
            return;
        };
        let batches: Vec<Vec<PathBuf>> = listener.files.try_iter().collect();
        if batches.is_empty() {
            return;
        }
        for path in batches.into_iter().flatten() {
            self.open_path(&path, true);
        }
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        ctx.send_viewport_cmd(ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));
    }

    /// Moves a tab, keeping the same document active.
    fn move_doc(&mut self, from: usize, to: usize) {
        if from == to || from >= self.docs.len() || to >= self.docs.len() {
            return;
        }
        let active_id = self.docs[self.active].id;
        let doc = self.docs.remove(from);
        self.docs.insert(to, doc);
        self.active = self
            .docs
            .iter()
            .position(|d| d.id == active_id)
            .unwrap_or(0);
    }

    fn toggle_pin(&mut self, index: usize) {
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        doc.pinned = !doc.pinned;
        // Pinned tabs come first: a pinned tab joins the end of that group,
        // an unpinned one the start of the rest.
        let pinned_total = self.docs.iter().filter(|d| d.pinned).count();
        let to = if self.docs[index].pinned {
            pinned_total - 1
        } else {
            pinned_total
        };
        self.move_doc(index, to.min(self.docs.len() - 1));
    }

    fn rename_file(&mut self, id: u64, name: &str) -> Result<(), String> {
        let Some(i) = self.docs.iter().position(|d| d.id == id) else {
            return Ok(());
        };
        let Some(old) = self.docs[i].path.clone() else {
            return Ok(());
        };
        if name.contains(['/', '\\']) {
            return Err("The name can't contain / or \\.".into());
        }
        let new = old.with_file_name(name);
        if new == old {
            return Ok(());
        }
        let only_case =
            old.to_string_lossy().to_lowercase() == new.to_string_lossy().to_lowercase();
        if new.exists() && !only_case {
            return Err(format!("\"{name}\" already exists in this folder."));
        }
        std::fs::rename(&old, &new).map_err(|e| format!("Couldn't rename the file: {e}"))?;
        let doc = &mut self.docs[i];
        doc.path = Some(new.clone());
        doc.disk = DiskStamp::read(&new).ok();
        doc.syntax = self
            .highlighter
            .detect(Some(&new), &doc.editor.buffer.line_text(0));
        for p in &mut self.settings.recent_files {
            if *p == old {
                *p = new.clone();
            }
        }
        Ok(())
    }

    fn summary(&self) -> Option<Summary> {
        let doc = self.docs.get(self.active)?;
        let rope = doc.editor.buffer.rope();
        let newlines: usize = rope.chunks().map(|c| c.matches('\n').count()).sum();
        let text = doc.editor.buffer.text();
        let words = unicode_segmentation::UnicodeSegmentation::unicode_words(text.as_str()).count();
        let eol_len = match doc.line_ending {
            LineEnding::CrLf => 2,
            _ => 1,
        };
        let bytes = if doc.encoding.is_utf8() {
            let bom = if doc.encoding == (TextEncoding::Utf8 { bom: true }) {
                3
            } else {
                0
            };
            Some(rope.len_bytes() + newlines * (eol_len - 1) + bom)
        } else {
            let text = if eol_len == 2 {
                text.replace('\n', "\r\n")
            } else {
                text
            };
            doc.encoding.encode(&text).ok().map(|b| b.len())
        };
        let meta = doc.path.as_ref().and_then(|p| std::fs::metadata(p).ok());
        let time = |t: std::io::Result<std::time::SystemTime>| {
            t.ok().map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
        };
        let primary = doc.editor.selections.primary();
        let sel = primary.start()..primary.end();
        let selected_lines = if sel.is_empty() {
            0
        } else {
            rope.char_to_line(sel.end) - rope.char_to_line(sel.start) + 1
        };
        Some(Summary {
            path: doc
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| format!("{} (not saved yet)", doc.title())),
            created: meta.as_ref().and_then(|m| time(m.created())),
            modified: meta.as_ref().and_then(|m| time(m.modified())),
            encoding: doc.encoding.name(),
            chars: rope.len_chars() - newlines,
            words,
            lines: doc.line_count(),
            bytes,
            selected_chars: doc.editor.selected_chars(),
            selected_lines,
        })
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
                // A tab shown in both views only closes in this one.
                if !self.close_in_view(self.active, self.split.focused) {
                    let id = self.docs[self.active].id;
                    self.request_close(vec![id], false);
                }
            }
            Action::CloseTabInView(i, view) => {
                if !self.close_in_view(i, view)
                    && let Some(d) = self.docs.get(i)
                {
                    self.request_close(vec![d.id], false);
                }
            }
            Action::SelectTabInView(i, view) => self.focus_view(view, i),
            Action::MoveToOtherView(i, view) => self.move_to_other_view(i, view),
            Action::CloneToOtherView(i, view) => self.clone_to_other_view(i, view),
            Action::MoveActiveToOtherView => {
                self.move_to_other_view(self.active, self.split.focused)
            }
            Action::CloneActiveToOtherView => {
                self.clone_to_other_view(self.active, self.split.focused)
            }
            Action::FocusOtherView => self.focus_other_view(),
            Action::CloseTabAt(i) => {
                if let Some(d) = self.docs.get(i) {
                    self.request_close(vec![d.id], false);
                }
            }
            // Pinned tabs stay open when closing many at once.
            Action::CloseAll => self.request_close(
                self.docs
                    .iter()
                    .filter(|d| !d.pinned)
                    .map(|d| d.id)
                    .collect(),
                false,
            ),
            Action::CloseOthers => {
                let keep = self.docs[self.active].id;
                self.request_close(
                    self.docs
                        .iter()
                        .filter(|d| !d.pinned)
                        .map(|d| d.id)
                        .filter(|&id| id != keep)
                        .collect(),
                    false,
                );
            }
            Action::Quit => {
                if self.session.is_some() && self.settings.remember_unsaved {
                    // Unsaved changes are backed up and come back next time.
                    self.finish_session(true);
                    self.quitting = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                } else {
                    self.request_close(
                        self.docs
                            .iter()
                            .filter(|d| d.is_dirty())
                            .map(|d| d.id)
                            .collect(),
                        true,
                    );
                }
            }
            Action::NextTab => self.step_tab(1),
            Action::PrevTab => self.step_tab(-1),
            Action::SelectTab(i) => self.select(i),
            Action::ZoomIn => {
                self.settings.font_size = (self.settings.font_size + 1.0).min(MAX_FONT)
            }
            Action::ZoomOut => {
                self.settings.font_size = (self.settings.font_size - 1.0).max(MIN_FONT)
            }
            Action::ZoomReset => self.settings.font_size = self.settings.default_font_size,
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
                    if let Err(e) = system::open_folder(&dir) {
                        self.errors.push(e);
                    }
                }
            }
            Action::ReloadThemes => self.reload_themes(),
            Action::RestoreThemes => {
                if let Some(dir) = settings::themes_dir()
                    && let Err(e) = theme::install_builtins(&dir, true)
                {
                    self.errors
                        .push(format!("Couldn't restore the built-in themes: {e}"));
                }
                self.reload_themes();
            }
            Action::ClearRecent => self.settings.recent_files.clear(),
            Action::ReloadFromDisk => {
                let doc = &mut self.docs[self.active];
                if doc.path.is_some()
                    && let Err(e) = doc.reload(None)
                {
                    self.errors
                        .push(format!("Could not reload {}: {e}", doc.title()));
                }
            }
            Action::IgnoreDiskChange => {
                let doc = &mut self.docs[self.active];
                doc.disk = doc.path.as_deref().and_then(|p| DiskStamp::read(p).ok());
                doc.disk_change = None;
                doc.editor.buffer.mark_modified();
            }
            Action::RenameFile => {
                let doc = &self.docs[self.active];
                if doc.path.is_some() {
                    self.rename.show_dialog(doc.id, &doc.title());
                } else {
                    self.save(self.active, true);
                }
            }
            Action::MoveToTrash => {
                let doc = &self.docs[self.active];
                if doc.path.is_some() {
                    self.trash_confirm = Some(doc.id);
                }
            }
            Action::OpenContainingFolder => {
                if let Some(path) = &self.docs[self.active].path
                    && let Err(e) = system::reveal_file(path)
                {
                    self.errors.push(e);
                }
            }
            Action::OpenTerminalHere => {
                let dir = self.docs[self.active]
                    .path
                    .as_ref()
                    .and_then(|p| p.parent().map(Path::to_path_buf))
                    .or_else(dirs::home_dir)
                    .unwrap_or_else(|| PathBuf::from("."));
                if let Err(e) = system::open_terminal(&dir) {
                    self.errors.push(e);
                }
            }
            Action::Summary => self.summary = self.summary(),
            Action::TogglePin(i) => self.toggle_pin(i),
            Action::ConvertEncoding(name) => {
                if let Some(e) = TextEncoding::from_name(&name) {
                    self.docs[self.active].set_encoding(e);
                }
            }
            Action::ReopenWithEncoding(name) => {
                if let Some(e) = TextEncoding::from_name(&name) {
                    let doc = &mut self.docs[self.active];
                    if doc.path.is_none() {
                        doc.set_encoding(e);
                    } else if let Err(e) = doc.reload(Some(e)) {
                        self.errors
                            .push(format!("Could not reload {}: {e}", doc.title()));
                    } else if doc.lossy {
                        self.errors.push(format!(
                            "Some bytes in {} aren't valid {}. They are shown as \u{FFFD}.",
                            doc.title(),
                            doc.encoding.name()
                        ));
                    }
                }
            }
            Action::About => self.show_about = true,
            Action::CheckForUpdates => self.updates.start(ctx, true),
            Action::Print => self.print(false),
            Action::NewLanguage => self.new_language(),
            Action::OpenLanguagesFolder => self.open_languages_folder(),
            Action::ReloadLanguages => self.reload_languages(),
            Action::LanguageGuide => self.language_guide(),
            Action::ExportHtml => self.print(true),
            Action::ToggleUpdateCheck => self.settings.check_updates = !self.settings.check_updates,
            Action::Preferences => {
                self.prefs.show();
                drop_focus(ctx);
            }
            Action::ShortcutMapper => {
                self.shortcut_mapper.show();
                drop_focus(ctx);
            }
            Action::StyleConfigurator => {
                let name = self.themes[self.theme_idx].name.clone();
                let source = self.theme_source(&name);
                self.style_editor.show(&name, &source);
                drop_focus(ctx);
            }
            Action::CommandPalette => {
                self.tab_picker.open = false;
                self.command_palette
                    .show("Command palette", "Type part of a command's name");
            }
            Action::TabList => {
                self.command_palette.open = false;
                self.tab_picker
                    .show("Open tabs", "Type part of a tab's name");
            }
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
            Action::Mime(_)
            | Action::Json(_)
            | Action::Xml(_)
            | Action::ToggleJsonTree
            | Action::ToggleXPathPanel
            | Action::CopyXPath => self.run_text_tool(ctx, action),
            Action::CompareTabs
            | Action::CompareWith(_)
            | Action::CompareToSaved
            | Action::CompareToClipboard
            | Action::CompareToGitHead
            | Action::CompareSelections
            | Action::NextDifference
            | Action::PrevDifference
            | Action::ShowComparison
            | Action::CloseComparison
            | Action::ExportComparePatch
            | Action::ToggleCompareOption(_) => self.run_compare(ctx, action),
            Action::ToggleCsvQuery
            | Action::RunCsvQuery
            | Action::CopyCsvResults
            | Action::CsvResultsToNewTab => self.run_csv_action(ctx, action),
            Action::PluginCommand { .. }
            | Action::ManagePlugins
            | Action::ShowPluginLog
            | Action::OpenPluginsFolder
            | Action::ReloadPlugins
            | Action::InstallSamplePlugins
            | Action::PluginGuide => self.run_plugin_action(ctx, action),
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
            Action::ToggleDocMap => self.settings.doc_map = !self.settings.doc_map,
            Action::ToggleFunctionList => {
                self.settings.function_list = !self.settings.function_list
            }
            Action::ToggleFolderPanel => {
                if self.settings.workspace.is_none() && !self.settings.folder_panel {
                    self.pending.push(Action::OpenFolderAsWorkspace);
                } else {
                    self.settings.folder_panel = !self.settings.folder_panel;
                }
            }
            Action::OpenFolderAsWorkspace => {
                let start = self.settings.workspace.clone().or_else(|| {
                    self.docs
                        .get(self.active)
                        .and_then(|d| d.path.as_ref()?.parent().map(Path::to_path_buf))
                });
                let mut dialog = rfd::FileDialog::new().set_title("Open Folder as Workspace");
                if let Some(dir) = start {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(dir) = dialog.pick_folder() {
                    self.folders.set_root(&dir);
                    self.settings.workspace = Some(dir);
                    self.settings.folder_panel = true;
                }
            }
            Action::CloseWorkspace => {
                self.settings.workspace = None;
                self.settings.folder_panel = false;
                self.folders = crate::folder_ui::FolderPanel::default();
            }
            Action::ToggleDistractionFree => {
                self.distraction_free = !self.distraction_free;
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.distraction_free));
                if self.distraction_free {
                    self.notice = Some(Notice {
                        text: "Distraction-free mode. Press F11 to leave it.".to_owned(),
                        error: false,
                        since: None,
                        jump: None,
                    });
                }
            }
            Action::ToggleAutoClose => self.settings.auto_close = !self.settings.auto_close,
            Action::ToggleSmartIndent => self.settings.smart_indent = !self.settings.smart_indent,
            Action::ToggleUseSpaces => self.settings.use_spaces = !self.settings.use_spaces,
            Action::ToggleCompletion => {
                self.settings.word_completion = !self.settings.word_completion
            }
            Action::ToggleSingleInstance => {
                self.settings.single_instance = !self.settings.single_instance
            }
            Action::ToggleRememberUnsaved => {
                self.settings.remember_unsaved = !self.settings.remember_unsaved
            }
            Action::ToggleAutoReload => self.settings.auto_reload = !self.settings.auto_reload,
            Action::ToggleCloseTags => self.settings.close_tags = !self.settings.close_tags,
            Action::ToggleXmlCheckOnSave => {
                self.settings.xml_check_on_save = !self.settings.xml_check_on_save
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
                // Shortcuts follow their macro: drop the deleted one's, move the later ones up.
                let shortcuts = std::mem::take(&mut self.settings.shortcuts);
                for (key, texts) in shortcuts {
                    let key = match serde_json::from_str::<Action>(&key) {
                        Ok(Action::PlaySavedMacro(j)) if j == i => continue,
                        Ok(Action::PlaySavedMacro(j)) if j > i => {
                            actions::command_key(&Action::PlaySavedMacro(j - 1))
                        }
                        _ => key,
                    };
                    self.settings.shortcuts.insert(key, texts);
                }
                actions::set_overrides(&self.settings.shortcuts);
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
            || self.updates.is_open()
            || self.goto.open
            || self.fif.open
            || self.column.open
            || self.run_macro.open
            || self.save_macro.open
            || self.rename.open
            || self.trash_confirm.is_some()
            || self.summary.is_some()
            || self.plugin_modal_open()
            || self.prefs.open
            || self.shortcut_mapper.open
            || self.command_palette.open
            || self.tab_picker.open
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
        self.shortcut_mapper.grab_keys(ctx);
        if !self.any_dialog_open() {
            let mut fired = Vec::new();
            let mut shortcuts = actions::default_shortcuts();
            // Shift and Alt may be held for a shortcut without them, so
            // the ones with more modifiers must be tried first; this sorts too.
            shortcuts.extend(self.plugins.shortcuts());
            actions::apply_overrides(&mut shortcuts);
            shortcuts.sort_by_key(|(s, _)| {
                std::cmp::Reverse(crate::plugins::shortcut::modifier_count(s))
            });
            ctx.input_mut(|i| {
                for (shortcut, action) in shortcuts {
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

    /// The app icon as a texture, loaded the first time it is drawn.
    fn app_icon(&mut self, ctx: &egui::Context) -> egui::TextureHandle {
        self.app_icon
            .get_or_insert_with(|| {
                let png = include_bytes!("../assets/icons/ettin-edit-128.png");
                let image = eframe::icon_data::from_png_bytes(png).unwrap_or_default();
                let size = [image.width as usize, image.height as usize];
                ctx.load_texture(
                    "app-icon",
                    egui::ColorImage::from_rgba_unmultiplied(size, &image.rgba),
                    egui::TextureOptions::LINEAR,
                )
            })
            .clone()
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        let icon = self.app_icon(ui.ctx());
        egui::MenuBar::new().ui(ui, |ui| {
            let size = Vec2::splat(ui.spacing().interact_size.y);
            let response = ui
                .add(
                    egui::Image::new(&icon)
                        .fit_to_exact_size(size)
                        .sense(Sense::click()),
                )
                .on_hover_text("About Ettin Edit");
            if response.clicked() {
                self.pending.push(Action::About);
            }
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
                let has_file = self.docs.get(self.active).is_some_and(|d| d.path.is_some());
                ui.add_enabled_ui(has_file, |ui| {
                    self.menu_item(ui, "Reload from Disk", Action::ReloadFromDisk);
                    self.menu_item(ui, "Rename…", Action::RenameFile);
                    self.menu_item(ui, "Move to Trash…", Action::MoveToTrash);
                    ui.separator();
                    self.menu_item(ui, "Open Containing Folder", Action::OpenContainingFolder);
                });
                self.menu_item(ui, "Open Terminal Here", Action::OpenTerminalHere);
                ui.separator();
                self.menu_item(ui, "Print…", Action::Print);
                self.menu_item(ui, "Export as HTML…", Action::ExportHtml);
                ui.separator();
                self.menu_item(
                    ui,
                    "Open Folder as Workspace…",
                    Action::OpenFolderAsWorkspace,
                );
                if self.settings.workspace.is_some() {
                    self.menu_item(ui, "Close Workspace", Action::CloseWorkspace);
                }
                ui.separator();
                self.menu_item(ui, "Close", Action::CloseTab);
                self.menu_item(ui, "Close All", Action::CloseAll);
                self.menu_item(ui, "Close Others", Action::CloseOthers);
                ui.separator();
                self.menu_item(ui, "Exit", Action::Quit);
            });
            ui.menu_button("Edit", |ui| self.shared_menu(ui, false));
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
                ui.separator();
                self.toggle_item(
                    ui,
                    "Function List",
                    s.function_list,
                    Action::ToggleFunctionList,
                );
                self.toggle_item(ui, "Document Map", s.doc_map, Action::ToggleDocMap);
                self.toggle_item(
                    ui,
                    "Folder as Workspace",
                    s.folder_panel,
                    Action::ToggleFolderPanel,
                );
                self.toggle_item(
                    ui,
                    "Distraction-Free Mode",
                    self.distraction_free,
                    Action::ToggleDistractionFree,
                );
                self.menu_item(ui, "Tab List…", Action::TabList);
                self.menu_item(ui, "Command Palette…", Action::CommandPalette);
                ui.separator();
                self.menu_item(ui, "Move to Other View", Action::MoveActiveToOtherView);
                self.menu_item(ui, "Clone to Other View", Action::CloneActiveToOtherView);
                if self.is_split() {
                    self.menu_item(ui, "Focus Other View", Action::FocusOtherView);
                }
                ui.separator();
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
                ui.separator();
                self.menu_item(ui, "Summary…", Action::Summary);
            });
            ui.menu_button("Encoding", |ui| self.encoding_menu(ui));
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
                self.menu_item(ui, "Edit Theme Colours…", Action::StyleConfigurator);
                self.menu_item(ui, "Open Themes Folder", Action::OpenThemesFolder);
                self.menu_item(ui, "Reload Themes", Action::ReloadThemes);
                self.menu_item(ui, "Restore Built-in Themes", Action::RestoreThemes);
            });
            ui.menu_button("Language", |ui| self.language_menu(ui));
            ui.menu_button("Settings", |ui| {
                self.menu_item(ui, "Preferences…", Action::Preferences);
                self.menu_item(ui, "Shortcut Mapper…", Action::ShortcutMapper);
                ui.separator();
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
                self.toggle_item(
                    ui,
                    "Single Window (Open Files in the Running Ettin Edit)",
                    s.single_instance,
                    Action::ToggleSingleInstance,
                );
                self.toggle_item(
                    ui,
                    "Remember Unsaved Changes on Exit",
                    s.remember_unsaved,
                    Action::ToggleRememberUnsaved,
                );
                self.toggle_item(
                    ui,
                    "Reload Files Changed on Disk Automatically",
                    s.auto_reload,
                    Action::ToggleAutoReload,
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Long-line column");
                    ui.add(egui::DragValue::new(&mut self.settings.edge_column).range(10..=400));
                });
            });
            ui.menu_button("Tools", |ui| self.shared_menu(ui, true));
            ui.menu_button("Plugins", |ui| self.plugins_menu(ui));
            ui.menu_button("Macro", |ui| self.macro_menu(ui));
            ui.menu_button("Help", |ui| {
                self.menu_item(ui, "Check for Updates…", Action::CheckForUpdates);
                self.toggle_item(
                    ui,
                    "Check for Updates Automatically",
                    self.settings.check_updates,
                    Action::ToggleUpdateCheck,
                );
                ui.separator();
                self.menu_item(ui, "About Ettin Edit", Action::About);
            });
        });
    }

    /// Convert the encoding used when saving, or read the file again in another one.
    fn encoding_menu(&mut self, ui: &mut Ui) {
        let Some(doc) = self.docs.get(self.active) else {
            return;
        };
        let current = doc.encoding;
        let has_file = doc.path.is_some();
        let mut picked = None;
        ui.label(RichText::new("Save as").small().weak());
        file_ui::encoding_menu(ui, current, Action::ConvertEncoding, &mut picked);
        ui.separator();
        ui.add_enabled_ui(has_file, |ui| {
            ui.menu_button("Reopen With", |ui| {
                file_ui::encoding_menu(ui, current, Action::ReopenWithEncoding, &mut picked);
            })
            .response
            .on_disabled_hover_text("Only for files that are saved on disk");
        });
        self.pending.extend(picked);
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
        ui.separator();
        self.menu_item(ui, "New Language…", Action::NewLanguage);
        self.menu_item(ui, "Open Languages Folder", Action::OpenLanguagesFolder);
        self.menu_item(ui, "Reload Languages", Action::ReloadLanguages);
        self.menu_item(ui, "Language Guide", Action::LanguageGuide);
    }

    /// The tabs of one view.
    fn tab_strip(&mut self, ui: &mut Ui, view: usize) {
        let palette = self.palette();
        let tab_height = 32.0;
        let tabs = self.tabs_in(view);
        // The tab list button stays at the right; the tabs scroll beside it.
        let full = ui.available_rect_before_wrap();
        let list_w = 26.0;
        let strip = Rect::from_min_max(
            full.min,
            Pos2::new(full.right() - list_w, full.top() + tab_height),
        );
        let list_rect = Rect::from_min_max(
            Pos2::new(full.right() - list_w, full.top()),
            Pos2::new(full.right(), full.top() + tab_height - 2.0),
        );
        let list = ui.put(
            list_rect,
            egui::Button::new(RichText::new("⏷").size(12.0).color(palette.text_dim)).frame(false),
        );
        if list.on_hover_text("All open tabs").clicked() {
            self.pending.push(Action::TabList);
        }
        ui.allocate_rect(
            Rect::from_min_max(full.min, Pos2::new(full.right(), full.top() + tab_height)),
            Sense::hover(),
        );
        let mut strip_ui = ui.new_child(egui::UiBuilder::new().max_rect(strip));
        strip_ui.set_clip_rect(strip.intersect(ui.clip_rect()));
        let ui = &mut strip_ui;
        ScrollArea::horizontal()
            .id_salt(("tab-strip", view))
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    if view == 0 {
                        self.compare_tab(ui, tab_height, &palette);
                    }
                    let rects: Vec<Rect> = tabs
                        .iter()
                        .map(|&i| self.tab(ui, i, view, tab_height, &palette))
                        .collect();
                    self.drag_tabs(ui, &tabs, &rects);
                    ui.add_space(4.0);
                    let plus = ui.add_sized(
                        [28.0, tab_height - 6.0],
                        egui::Button::new(RichText::new("+").size(16.0).color(palette.text_dim))
                            .frame(false),
                    );
                    if plus.on_hover_text("New tab").clicked() {
                        if view != self.split.focused {
                            self.pending.push(Action::FocusOtherView);
                        }
                        self.pending.push(Action::NewTab);
                    }
                });
            });
        if view == self.split.focused || !self.is_split() {
            self.scroll_to_active_tab = false;
        }
    }

    /// Moves the tab being dragged to where the pointer is, among the tabs of
    /// its view (`tabs`, drawn at `rects`). Pinned tabs stay among pinned
    /// ones, and the others after them.
    fn drag_tabs(&mut self, ui: &Ui, tabs: &[usize], rects: &[Rect]) {
        let Some(id) = self.drag_tab else {
            return;
        };
        let (down, pointer) = ui.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
        let Some(from) = tabs.iter().position(|&i| self.docs[i].id == id) else {
            if !down {
                self.drag_tab = None;
            }
            return;
        };
        let Some(pointer) = pointer.filter(|_| down) else {
            self.drag_tab = None;
            return;
        };
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let pinned = self.docs[tabs[from]].pinned;
        let same = |k: usize| self.docs[tabs[k]].pinned == pinned;
        // Move past every neighbour whose middle the pointer has crossed.
        let target = (from + 1..rects.len())
            .rev()
            .find(|&k| same(k) && pointer.x > rects[k].center().x)
            .or_else(|| (0..from).find(|&k| same(k) && pointer.x < rects[k].center().x));
        if let Some(to) = target {
            self.move_doc(tabs[from], tabs[to]);
        }
    }

    /// Draws one tab of `view` and returns where it is.
    fn tab(
        &mut self,
        ui: &mut Ui,
        index: usize,
        view: usize,
        height: f32,
        palette: &Palette,
    ) -> Rect {
        let doc = &self.docs[index];
        let split = self.is_split();
        let focused_view = !split || view == self.split.focused;
        let active =
            self.active_in(view) == Some(index) && !self.compare.as_ref().is_some_and(|c| c.shown);
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

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
        if response.drag_started() {
            self.drag_tab = Some(doc.id);
        }
        let close_rect = Rect::from_center_size(
            Pos2::new(rect.right() - 8.0 - close_size / 2.0, rect.center().y),
            Vec2::splat(close_size),
        );
        let close_response = ui.interact(
            close_rect,
            Id::new(("tab-close", doc.id, view)),
            Sense::click(),
        );

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
                if focused_view {
                    palette.accent
                } else {
                    palette.text_dim.gamma_multiply(0.5)
                },
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
        // Pinned tabs show a pin instead; clicking it unpins the tab.
        let show_close =
            close_response.hovered() || response.hovered() || active || !doc.is_dirty();
        if close_response.hovered() {
            painter.rect_filled(
                close_rect,
                CornerRadius::same(4),
                palette.tab_hover_bg.gamma_multiply(1.6),
            );
        }
        if doc.pinned {
            let color = if doc.is_dirty() {
                palette.modified
            } else if close_response.hovered() {
                palette.text
            } else {
                palette.text_dim
            };
            draw_pin(painter, close_rect.center(), color);
        } else if doc.is_dirty() && !close_response.hovered() {
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
        let pinned = doc.pinned;
        if close_response.clicked() && pinned {
            self.pending.push(Action::TogglePin(index));
        } else if close_response.clicked() || (response.middle_clicked() && !pinned) {
            self.pending.push(Action::CloseTabInView(index, view));
        } else if response.clicked() {
            self.pending.push(Action::SelectTabInView(index, view));
        }
        let path = doc.path.clone();
        let response = match &path {
            Some(p) => response.on_hover_text(p.display().to_string()),
            None => response,
        };
        response.context_menu(|ui| {
            if ui.button("Close").clicked() {
                self.pending.push(Action::CloseTabInView(index, view));
            }
            if ui.button("Close Others").clicked() {
                self.pending.push(Action::SelectTabInView(index, view));
                self.pending.push(Action::CloseOthers);
            }
            if ui
                .button(if pinned { "Unpin Tab" } else { "Pin Tab" })
                .clicked()
            {
                self.pending.push(Action::TogglePin(index));
            }
            ui.separator();
            if ui.button("Move to Other View").clicked() {
                self.pending.push(Action::MoveToOtherView(index, view));
            }
            if ui.button("Clone to Other View").clicked() {
                self.pending.push(Action::CloneToOtherView(index, view));
            }
            if let Some(p) = &path {
                ui.separator();
                for (label, action) in [
                    ("Reload from Disk", Action::ReloadFromDisk),
                    ("Rename…", Action::RenameFile),
                    ("Move to Trash…", Action::MoveToTrash),
                    ("Open Containing Folder", Action::OpenContainingFolder),
                    ("Open Terminal Here", Action::OpenTerminalHere),
                ] {
                    if ui.button(label).clicked() {
                        self.pending.push(Action::SelectTabInView(index, view));
                        self.pending.push(action);
                    }
                }
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
        rect
    }

    /// Draws tab `index` in `view`. Returns true when its editor has keyboard focus.
    fn editor_pane(&mut self, ui: &mut Ui, view: usize, index: usize) -> bool {
        let palette = self.palette();
        let theme_def = &self.themes[self.theme_idx];
        let focused_view = view == self.split.focused;
        let request_focus =
            focused_view && std::mem::take(&mut self.focus_editor) && !self.any_dialog_open();
        let ctx = ui.ctx().clone();
        let mut menu_model = menus::MenuModel::new(&ctx);
        let context_menu = self.context_menu(&mut menu_model, index);
        let Some(doc) = self.docs.get_mut(index) else {
            return false;
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
            close_tags: s.close_tags,
            html: matches!(
                doc.syntax.as_str(),
                "HTML" | "Vue Component" | "PHP" | "ASP"
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
            theme_name: &theme_def.key,
            plain: doc.syntax == PLAIN_TEXT,
            request_focus,
            show_whitespace: s.show_whitespace,
            show_eol: s.show_eol,
            eol_label,
            indent_guides: s.indent_guides,
            edge_column: s.show_edge.then_some(s.edge_column),
            context_menu: &context_menu,
        };
        // A document shown in both views keeps each view's carets apart.
        let swapped = doc.editor.has_other_view() && doc.live_view != view;
        if swapped {
            doc.editor.swap_view();
        }
        let id = Id::new(("editor", doc.id, view));
        let clicked = doc.editor.show(ui, id, &cfg);
        if swapped {
            doc.editor.swap_view();
        }
        if let Some(action) = clicked.and_then(|id| menu_model.action(id)) {
            // The right-click menu acts on the view it was opened in.
            if !focused_view {
                self.focus_view(view, index);
            }
            self.pending.push(action);
        }
        ui.memory(|m| m.has_focus(id))
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let xpath = self.current_xpath(&ui.ctx().clone());
        let Some(doc) = self.docs.get(self.active) else {
            return;
        };
        let (line, col) = doc.editor.caret_line_col();
        let sel = doc.editor.selected_chars();
        let carets = doc.editor.caret_count();
        let syntax = doc.syntax.clone();
        let line_ending = doc.line_ending;
        let encoding = doc.encoding.name();
        let length = doc.editor.buffer.len_chars();
        let lines = doc.line_count();
        let zoom =
            (self.settings.font_size / self.settings.default_font_size.max(1.0) * 100.0).round();
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
            if let Some(path) = xpath {
                let shown = if path.chars().count() > 80 {
                    let tail: String = path.chars().rev().take(79).collect();
                    format!("…{}", tail.chars().rev().collect::<String>())
                } else {
                    path.clone()
                };
                if ui
                    .add(egui::Label::new(dim(shown)).sense(Sense::click()))
                    .on_hover_text(format!("{path}\nClick to copy"))
                    .clicked()
                {
                    self.pending.push(Action::CopyXPath);
                }
            }
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
                ui.menu_button(dim(encoding), |ui| self.encoding_menu(ui));
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
                    self.finish_session(false);
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
        for event in self.updates.ui(ctx, &palette) {
            match event {
                crate::update::UpdateEvent::OpenPage(url) => {
                    if let Err(e) = system::open_url(&url) {
                        self.errors.push(e);
                    }
                }
                crate::update::UpdateEvent::StopChecking => self.settings.check_updates = false,
            }
        }
        let languages = self.languages.clone();
        for event in self.prefs.ui(ctx, &mut self.settings, &languages, &palette) {
            match event {
                crate::prefs_ui::PrefsEvent::Font(font) => {
                    match crate::fonts::install(ctx, &font) {
                        Ok(()) => self.settings.editor_font = font,
                        Err(e) => {
                            self.errors.push(e);
                            let _ = crate::fonts::install(ctx, &self.settings.editor_font);
                        }
                    }
                }
                crate::prefs_ui::PrefsEvent::Shortcuts => self.pending.push(Action::ShortcutMapper),
                crate::prefs_ui::PrefsEvent::Styles => self.pending.push(Action::StyleConfigurator),
            }
        }
        crate::editor::layout::set_tab_width(self.settings.tab_width);
        if self.style_editor.open {
            let names: Vec<(String, bool)> = self
                .themes
                .iter()
                .map(|t| (t.name.clone(), t.custom))
                .collect();
            let events = self.style_editor.ui(ctx, &names, &palette);
            self.style_events(ctx, events);
        }
        if self.command_palette.open {
            let items: Vec<crate::palette_ui::Item<Action>> = self
                .command_catalog(ctx)
                .into_iter()
                .map(|c| crate::palette_ui::Item {
                    detail: actions::effective_shortcut(&c.action, c.plugin_shortcut)
                        .or_else(|| {
                            actions::all_shortcuts()
                                .into_iter()
                                .find(|(_, a)| *a == c.action)
                                .map(|(s, _)| s)
                        })
                        .map(|s| ctx.format_shortcut(&s))
                        .unwrap_or_default(),
                    label: c.path,
                    value: c.action,
                })
                .collect();
            if let Some(action) = self.command_palette.ui(ctx, &items, &palette) {
                self.pending.push(action);
                self.focus_editor = true;
            } else if !self.command_palette.open {
                self.focus_editor = true;
            }
        }
        if self.tab_picker.open {
            let items: Vec<crate::palette_ui::Item<usize>> = self
                .docs
                .iter()
                .enumerate()
                .map(|(i, d)| crate::palette_ui::Item {
                    label: format!("{}{}", if d.is_dirty() { "● " } else { "" }, d.title()),
                    detail: d
                        .path
                        .as_ref()
                        .and_then(|p| p.parent())
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    value: i,
                })
                .collect();
            if let Some(i) = self.tab_picker.ui(ctx, &items, &palette) {
                self.pending.push(Action::SelectTab(i));
                self.focus_editor = true;
            } else if !self.tab_picker.open {
                self.focus_editor = true;
            }
        }
        if self.shortcut_mapper.open {
            let rows = self.shortcut_rows(ctx);
            if self
                .shortcut_mapper
                .ui(ctx, &rows, &mut self.settings.shortcuts, &palette)
            {
                let errors = actions::set_overrides(&self.settings.shortcuts);
                self.errors.extend(errors);
            }
        }
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

        if let Some(name) = self.rename.ui(ctx, &palette) {
            match self.rename_file(self.rename.doc_id, &name) {
                Ok(()) => {
                    self.rename.open = false;
                    self.focus_editor = true;
                }
                Err(e) => self.rename.error = Some(e),
            }
        }
        if let Some(id) = self.trash_confirm
            && let Some(doc) = self.docs.iter().find(|d| d.id == id)
        {
            let name = doc.title();
            let path = doc.path.clone();
            match file_ui::confirm_trash(ctx, &name, &palette) {
                Some(true) => {
                    self.trash_confirm = None;
                    if let Some(path) = path {
                        match trash::delete(&path) {
                            Ok(()) => self.remove_doc(id),
                            Err(e) => self
                                .errors
                                .push(format!("Couldn't move {name} to the trash: {e}")),
                        }
                    }
                }
                Some(false) => {
                    self.trash_confirm = None;
                    self.focus_editor = true;
                }
                None => {}
            }
        }
        if let Some(summary) = &self.summary
            && !file_ui::summary_ui(ctx, summary)
        {
            self.summary = None;
            self.focus_editor = true;
        }

        if self.show_about {
            let response = egui::Modal::new(Id::new("about")).show(ctx, |ui| {
                ui.set_width(380.0);
                let icon = self.app_icon(ctx);
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.add(egui::Image::new(&icon).fit_to_exact_size(Vec2::splat(64.0)));
                    ui.label(
                        RichText::new("Ettin Edit")
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
                format!(
                    "{}{} - Ettin Edit",
                    if doc.is_dirty() { "*" } else { "" },
                    name
                )
            }
            None => "Ettin Edit".to_owned(),
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }
}

impl eframe::App for EttinApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.apply_style(&ctx);
        self.poll_instance(&ctx);
        self.check_disk(&ctx);
        if self.session.is_some() {
            let now = ctx.input(|i| i.time);
            if now - self.last_backup >= BACKUP_INTERVAL {
                self.last_backup = now;
                self.write_backups();
            }
            if self.docs.iter().any(|d| d.is_dirty()) {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(BACKUP_INTERVAL));
            }
        }
        self.fix_views();
        self.handle_input(&ctx);
        self.take_clipboard(&ctx);
        self.start_plugins(&ctx);
        self.plugins_frame(&ctx);
        let palette = self.palette();

        if !self.distraction_free {
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
                .show(ui, |ui| self.tab_bars(ui));
            egui::Panel::bottom("status-bar")
                .frame(
                    Frame::new()
                        .fill(palette.chrome_bg)
                        .inner_margin(Margin::symmetric(10, 3)),
                )
                .show_separator_line(false)
                .show(ui, |ui| self.status_bar(ui));
        }
        let panels = !self.distraction_free;
        if panels {
            self.xpath_panel(ui, &ctx);
        }
        if panels && self.results.open {
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
        if let Some(doc) = self.docs.get(self.active)
            && let Some(change) = doc.disk_change
        {
            let dirty = doc.is_dirty();
            let mut picked = None;
            egui::Panel::top("disk-notice")
                .frame(
                    Frame::new()
                        .fill(palette.chrome_bg)
                        .inner_margin(Margin::symmetric(10, 5)),
                )
                .show_separator_line(false)
                .show(ui, |ui| {
                    picked = file_ui::disk_notice(ui, change, dirty, &palette)
                });
            self.pending.extend(picked);
        }
        if panels {
            self.plugin_panels(ui);
            self.json_tree_panel(ui, &ctx);
            self.csv_query_panel(ui, &ctx);
            self.folder_panel(ui);
            // The map sits next to the text, inside the function list.
            self.function_list_panel(ui);
            self.doc_map_panel(ui);
        }
        self.record_editor_input(&ctx);
        egui::CentralPanel::no_frame()
            .frame(Frame::new().fill(palette.editor_bg))
            .show(ui, |ui| {
                if self.distraction_free {
                    // A centred column about 100 characters wide.
                    let font = FontId::monospace(self.settings.font_size);
                    let char_w = ui.fonts_mut(|f| f.glyph_width(&font, 'M'));
                    let rect = ui.max_rect();
                    let width = (char_w * 100.0 + 80.0).min(rect.width());
                    let column =
                        Rect::from_center_size(rect.center(), Vec2::new(width, rect.height()));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                        self.editor_area(ui)
                    });
                } else {
                    self.editor_area(ui)
                }
            });

        self.close_queue_ui(&ctx);
        self.dialogs(&ctx);
        self.plugin_dialogs(&ctx);
        self.notice_ui(&ctx);

        for action in std::mem::take(&mut self.pending) {
            self.run(&ctx, action);
        }
        self.update_title(&ctx);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        if !self.quitting {
            self.write_backups();
        }
        self.settings.save();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.plugins.shutdown();
    }
}

/// Takes keyboard focus from the editor, so a window that just opened gets the keys.
fn drop_focus(ctx: &egui::Context) {
    ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
}

/// A small push-pin, drawn where a pinned tab's close button would be.
fn draw_pin(painter: &egui::Painter, c: Pos2, color: egui::Color32) {
    let head = c + Vec2::new(1.5, -1.5);
    painter.circle_filled(head, 3.0, color);
    painter.line_segment(
        [head + Vec2::new(-3.2, -3.2), head + Vec2::new(3.2, 3.2)],
        Stroke::new(1.6, color),
    );
    painter.line_segment([head, c + Vec2::new(-4.5, 4.5)], Stroke::new(1.4, color));
}
