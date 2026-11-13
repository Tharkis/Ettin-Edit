//! The table tab's link to the app (2.7): opening workbooks and text as
//! tables, the Table commands, and what the table tab asks for.

use std::path::Path;
use std::sync::Arc;

use egui::{Id, Ui};

use super::EttinApp;
use crate::actions::Action;
use crate::document::{DiskStamp, Document};
use crate::table::TableJob;
use crate::table::import::{self, Source};
use crate::table_ui::{Data, TableCmd, TableEvent, TableTab};

/// A table tab's state, with the file stamp its workbook was read at.
pub struct TableState {
    pub tab: TableTab,
    pub stamp: Option<DiskStamp>,
}

impl TableState {
    /// A table read from a text tab's text.
    pub fn for_text(doc: &mut Document, job: TableJob, shown: bool) -> Self {
        let text = doc.editor.text_snapshot();
        let version = doc.editor.buffer.version();
        let mut tab = TableTab::new(job, Data::Text(text), Some(version), doc.path.clone());
        tab.shown = shown;
        Self { tab, stamp: None }
    }
}

impl EttinApp {
    /// Whether tab `index` shows as a table.
    pub(super) fn table_shown(&self, index: usize) -> bool {
        self.docs
            .get(index)
            .is_some_and(|d| self.tables.get(&d.id).is_some_and(|t| t.tab.shown))
    }

    /// A workbook's tab and table, not yet added.
    pub(super) fn workbook_table(
        &self,
        id: u64,
        path: &Path,
        job: Option<TableJob>,
        shown: bool,
    ) -> Result<(Document, TableState), String> {
        let bytes = std::fs::read(path).map_err(|e| {
            t!(
                "open-failed",
                path = path.display().to_string(),
                error = e.to_string()
            )
        })?;
        let doc = Document::open_workbook(id, path);
        let job = job.unwrap_or_else(|| TableJob {
            read: Source::Excel {
                sheet: String::new(),
                header_row: import::first_row(&bytes, "").unwrap_or(1),
                detect_types: true,
            },
            ..TableJob::default()
        });
        let mut tab = TableTab::new(
            job,
            Data::Bytes(Arc::new(bytes)),
            None,
            Some(path.to_path_buf()),
        );
        tab.shown = shown;
        let stamp = doc.disk;
        Ok((doc, TableState { tab, stamp }))
    }

    /// Opens a workbook in a table tab.
    pub(super) fn open_workbook(&mut self, path: &Path) -> bool {
        let id = self.alloc_id();
        let (doc, state) = match self.workbook_table(id, path, None, true) {
            Ok(r) => r,
            Err(e) => {
                self.errors.push(e);
                return false;
            }
        };
        self.tables.insert(id, state);
        let replace = self
            .docs
            .get(self.active)
            .is_some_and(|d| d.is_pristine_untitled());
        if replace {
            let old = &self.docs[self.active];
            let mut doc = doc;
            doc.views = old.views;
            doc.live_view = old.live_view;
            self.docs[self.active] = doc;
            self.fix_views();
            self.select(self.active);
        } else {
            self.add_doc(doc);
        }
        true
    }

    /// Shows the active text tab as a table, reading its text.
    fn show_as_table(&mut self) {
        let Some(doc) = self.docs.get_mut(self.active) else {
            return;
        };
        let id = doc.id;
        if let Some(state) = self.tables.get_mut(&id) {
            state.tab.shown = true;
            return;
        }
        if doc.binary.is_some() || doc.welcome {
            self.errors.push(t!("table-cant-show"));
            return;
        }
        let ext = doc
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .map(|e| e.to_string_lossy().to_lowercase());
        let read = Source::guess(&doc.editor.text_snapshot(), ext.as_deref());
        let job = TableJob {
            read,
            ..TableJob::default()
        };
        let state = TableState::for_text(doc, job, true);
        self.tables.insert(id, state);
    }

    /// File > Open as Table…: a workbook, or a CSV/JSON file shown as a table.
    fn open_as_table_dialog(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title(t!("table-open-as-table"))
            .add_filter(
                t!("table-filter-tables"),
                &[
                    "csv", "tsv", "txt", "json", "xlsx", "xlsm", "xls", "xlsb", "ods",
                ],
            )
            .add_filter(t!("table-filter-all"), &["*"]);
        if let Some(dir) = self
            .docs
            .get(self.active)
            .and_then(|d| d.path.as_ref()?.parent().map(Path::to_path_buf))
        {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };
        self.open_path(&path, true);
        if !import::is_workbook(&path) {
            self.show_as_table();
        }
    }

    /// Runs a Table command.
    pub(super) fn run_table(&mut self, cmd: TableCmd) {
        match cmd {
            TableCmd::OpenAsTable => return self.open_as_table_dialog(),
            TableCmd::ShowAsTable if !self.table_shown(self.active) => {
                return self.show_as_table();
            }
            TableCmd::ShowAsTable | TableCmd::ShowAsText => {
                let Some(doc) = self.docs.get(self.active) else {
                    return;
                };
                if doc.workbook {
                    self.errors.push(t!("table-workbook-no-text"));
                    return;
                }
                if let Some(state) = self.tables.get_mut(&doc.id) {
                    state.tab.shown = false;
                }
                self.focus_editor = true;
                return;
            }
            _ => {}
        }
        if !self.table_shown(self.active) {
            self.show_as_table();
        }
        let Some(id) = self.docs.get(self.active).map(|d| d.id) else {
            return;
        };
        if let Some(state) = self.tables.get_mut(&id) {
            state.tab.command(cmd);
        }
    }

    /// Commands that act differently on a table tab. Returns the action
    /// when it's for the text instead.
    pub(super) fn table_intercept(
        &mut self,
        ctx: &egui::Context,
        action: Action,
    ) -> Option<Action> {
        if !self.table_shown(self.active) {
            return Some(action);
        }
        let id = self.docs[self.active].id;
        let workbook = self.docs[self.active].workbook;
        let state = self.tables.get_mut(&id)?;
        let tab = &mut state.tab;
        match action {
            Action::Undo => tab.undo(),
            Action::Redo => tab.redo(),
            Action::Copy | Action::Cut => {
                if let Some(t) = tab.copy_text() {
                    ctx.copy_text(t);
                }
            }
            Action::Find | Action::Replace => tab.focus_find(),
            Action::Save | Action::SaveAs if workbook || action == Action::SaveAs => {
                tab.command(TableCmd::Export)
            }
            Action::Save if tab.is_dirty() => tab.command(TableCmd::Export),
            Action::ReloadFromDisk if workbook => {
                state.stamp = None;
                return Some(action);
            }
            a if a.recordable()
                || matches!(
                    a,
                    Action::TransformDialog(_) | Action::Paste | Action::SelectAll
                ) =>
            {
                self.notify(ctx, t!("table-text-command"), None);
            }
            other => return Some(other),
        }
        None
    }

    /// Draws tab `index` as a table. Returns true when it has the keyboard.
    pub(super) fn table_pane(&mut self, ui: &mut Ui, view: usize, index: usize) -> bool {
        let palette = self.palette();
        let font_size = self.settings.font_size;
        let doc = &mut self.docs[index];
        let id = doc.id;
        let Some(state) = self.tables.get_mut(&id) else {
            return false;
        };
        // The text was edited, or the workbook changed on disk: read again.
        if doc.workbook {
            if state.stamp != doc.disk
                && let Some(path) = &doc.path
                && let Ok(bytes) = std::fs::read(path)
            {
                state.tab.set_data(Data::Bytes(Arc::new(bytes)), None);
                state.stamp = doc.disk;
            }
        } else {
            let version = doc.editor.buffer.version();
            if state.tab.text_version != Some(version) {
                let text = doc.editor.text_snapshot();
                state.tab.set_data(Data::Text(text), Some(version));
            }
        }
        let events = state
            .tab
            .ui(ui, Id::new(("table", id, view)), &palette, font_size);
        doc.table_dirty = state.tab.is_dirty();
        let ctx = ui.ctx().clone();
        for event in events {
            self.table_event(&ctx, id, event);
        }
        false
    }

    fn table_event(&mut self, ctx: &egui::Context, id: u64, event: TableEvent) {
        match event {
            TableEvent::NewTab { text, syntax } => {
                self.new_tab();
                let doc = &mut self.docs[self.active];
                doc.editor = crate::editor::Editor::new(&text);
                doc.editor.buffer.mark_modified();
                if self.languages.iter().any(|l| l == &syntax) {
                    doc.syntax = syntax;
                }
                self.focus_editor = true;
            }
            TableEvent::SaveRecipe {
                name,
                description,
                job,
            } => {
                let Some(dir) = crate::recipes::recipes_dir() else {
                    return;
                };
                // Saving under a recipe's name again replaces its steps.
                let existing = self
                    .recipes
                    .iter()
                    .position(|r| r.name.eq_ignore_ascii_case(&name));
                let mut recipe = match existing {
                    Some(i) => self.recipes[i].clone(),
                    None => crate::recipes::Recipe::new(&name),
                };
                recipe.description = description;
                recipe.steps = vec![crate::recipes::Step::Table(job).into()];
                match crate::recipes::save(&mut recipe, &dir) {
                    Ok(()) => {
                        self.reload_recipes();
                        if let Some(state) = self.tables.get_mut(&id) {
                            state.tab.mark_saved();
                        }
                        self.notify(ctx, t!("table-recipe-saved", name = name), None);
                    }
                    Err(e) => self.errors.push(e),
                }
            }
            TableEvent::Message(m) => self.notify(ctx, m, None),
        }
    }

    /// The status bar's table part, when the active tab shows a table.
    pub(super) fn table_status(&mut self) -> Option<Vec<String>> {
        let id = self.docs.get(self.active)?.id;
        let state = self.tables.get_mut(&id).filter(|s| s.tab.shown)?;
        Some(state.tab.status())
    }

    /// Whether the active tab's table can undo or redo, for menus.
    pub(super) fn table_tab(&self) -> Option<&TableTab> {
        let id = self.docs.get(self.active)?.id;
        self.tables.get(&id).filter(|s| s.tab.shown).map(|s| &s.tab)
    }
}
