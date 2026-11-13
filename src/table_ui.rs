//! The table tab (2.7): a grid of typed columns with sort and filter on the
//! headers, a Steps list beside it, and small dialogs that each add one
//! step. Every change is a step, so the whole job can be saved as a recipe
//! and replayed on the next file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use egui::{
    Align, Color32, Id, Key, Layout, Margin, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui,
    Vec2,
};
use egui_extras::{Column as GridColumn, TableBuilder};
use serde::{Deserialize, Serialize};

use crate::i18n::tr;
use crate::table::export::{Export, ExportKind, Output, QuoteMode};
use crate::table::import::{Input, Source};
use crate::table::job::JobStep;
use crate::table::steps::{Condition, SortKey, TextOp};
use crate::table::{Kind, Table, TableJob, TableStep};
use crate::theme::Palette;

/// Commands for the table tab, from the Table menu, the toolbar, the
/// command palette and shortcuts. They act on the selected cell's column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableCmd {
    /// File > Open as Table…
    OpenAsTable,
    /// Shows a text tab (CSV, JSON) as a table.
    ShowAsTable,
    /// Shows the table tab's text again.
    ShowAsText,
    ImportOptions,
    Refresh,
    RenameColumn,
    ChangeType,
    ChooseColumns,
    RemoveColumn,
    DuplicateColumn,
    MoveLeft,
    MoveRight,
    ReplaceValues,
    Text(TextOp),
    Filter,
    SortAscending,
    SortDescending,
    Sort,
    RemoveDuplicates,
    RemoveEmptyRows,
    RemoveRow,
    EditCell,
    ColumnStats,
    Export,
    SaveAsRecipe,
}

impl TableCmd {
    /// Commands that need a table tab.
    pub fn needs_table(self) -> bool {
        !matches!(self, TableCmd::OpenAsTable | TableCmd::ShowAsTable)
    }

    pub fn label(self) -> String {
        match self {
            TableCmd::Text(op) => op.label(),
            other => tr(match other {
                TableCmd::OpenAsTable => "table-open-as-table",
                TableCmd::ShowAsTable => "table-show-as-table",
                TableCmd::ShowAsText => "table-show-as-text",
                TableCmd::ImportOptions => "table-import-options",
                TableCmd::Refresh => "table-refresh",
                TableCmd::RenameColumn => "table-rename-column",
                TableCmd::ChangeType => "table-change-type",
                TableCmd::ChooseColumns => "table-choose-columns",
                TableCmd::RemoveColumn => "table-remove-column",
                TableCmd::DuplicateColumn => "table-duplicate-column",
                TableCmd::MoveLeft => "table-move-left",
                TableCmd::MoveRight => "table-move-right",
                TableCmd::ReplaceValues => "table-replace-values",
                TableCmd::Filter => "table-filter",
                TableCmd::SortAscending => "table-sort-ascending",
                TableCmd::SortDescending => "table-sort-descending",
                TableCmd::Sort => "table-sort",
                TableCmd::RemoveDuplicates => "table-remove-duplicates",
                TableCmd::RemoveEmptyRows => "table-remove-empty-rows",
                TableCmd::RemoveRow => "table-remove-row",
                TableCmd::EditCell => "table-edit-cell",
                TableCmd::ColumnStats => "table-column-stats",
                TableCmd::Export => "table-export",
                TableCmd::SaveAsRecipe => "table-save-as-recipe",
                TableCmd::Text(_) => unreachable!(),
            }),
        }
    }
}

/// What the table is read from: the tab's text, or a workbook's bytes.
#[derive(Clone)]
pub enum Data {
    Text(Arc<String>),
    Bytes(Arc<Vec<u8>>),
}

impl Data {
    fn input(&self) -> Input<'_> {
        match self {
            Data::Text(t) => Input::Text(t),
            Data::Bytes(b) => Input::Bytes(b),
        }
    }
}

/// What the table tab asks the app to do.
pub enum TableEvent {
    /// Open the export in a new tab.
    NewTab { text: String, syntax: String },
    SaveRecipe {
        name: String,
        description: String,
        job: TableJob,
    },
    /// A message for the status line.
    Message(String),
}

/// The table after the source (stage 0) or after a step (stage i + 1).
#[derive(Clone)]
struct Stage {
    /// The table at this stage, or the last one that worked.
    table: Arc<Table>,
    /// Why this stage's step failed.
    error: Option<String>,
    /// An earlier step failed, so this one didn't run.
    blocked: bool,
}

struct CellEdit {
    row: usize,
    col: usize,
    text: String,
    focus: bool,
}

/// An open dialog, and the step it changes (`None` adds a step).
pub struct OpenDialog {
    kind: Dialog,
    editing: Option<usize>,
    preview: Option<(String, Result<Arc<Table>, String>)>,
}

enum Dialog {
    Import {
        source: Source,
        sheets: Vec<String>,
    },
    Rename {
        column: String,
        to: String,
    },
    Duplicate {
        column: String,
        to: String,
    },
    ChangeType {
        column: String,
        kind: Kind,
        format: String,
        decimal_comma: bool,
    },
    Replace {
        column: String,
        all: bool,
        find: String,
        replace: String,
        whole_cell: bool,
        match_case: bool,
        regex: bool,
    },
    Filter {
        column: String,
        condition: Condition,
        value: String,
        values: BTreeSet<String>,
        match_case: bool,
        search: String,
        /// The column's values and how often each comes up.
        distinct: Vec<(String, usize)>,
    },
    Sort {
        keys: Vec<SortKey>,
    },
    Dedupe {
        columns: Vec<(String, bool)>,
    },
    Choose {
        columns: Vec<(String, bool)>,
    },
    Export {
        export: Export,
        dest: Dest,
    },
    Recipe {
        name: String,
        description: String,
        /// Some steps pick rows by number, which may not suit new data.
        row_numbers: bool,
    },
    Stats {
        column: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dest {
    NewTab,
    File,
    Clipboard,
}

pub struct TableTab {
    pub job: TableJob,
    /// Shown as a table (false: the tab's text shows).
    pub shown: bool,
    pub data: Data,
    /// The text version the data was read from, for text tabs.
    pub text_version: Option<u64>,
    /// The file the data came from, for names of exports and recipes.
    pub path: Option<PathBuf>,
    /// The job as last exported or saved as a recipe.
    saved: TableJob,
    stages: Vec<Stage>,
    /// The stage shown; the last when `None`.
    view: Option<usize>,
    /// The selected cell: row and column of the shown table.
    pub cell: Option<(usize, usize)>,
    edit: Option<CellEdit>,
    /// The Edit Cell step the open cell edit changes.
    editing_cell_step: Option<usize>,
    find: String,
    find_focus: bool,
    scroll_to: Option<usize>,
    undo: Vec<(TableJob, Option<usize>)>,
    redo: Vec<(TableJob, Option<usize>)>,
    pub dialog: Option<OpenDialog>,
    /// A problem to show above the grid.
    pub message: Option<String>,
    /// Column widths by name, guessed from the first rows.
    widths: BTreeMap<String, f32>,
}

const ROW_LIMIT_PREVIEW: usize = 8;

impl TableTab {
    pub fn new(
        job: TableJob,
        data: Data,
        text_version: Option<u64>,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            saved: job.clone(),
            job,
            shown: true,
            data,
            text_version,
            path,
            stages: Vec::new(),
            view: None,
            cell: None,
            edit: None,
            editing_cell_step: None,
            find: String::new(),
            find_focus: false,
            scroll_to: None,
            undo: Vec::new(),
            redo: Vec::new(),
            dialog: None,
            message: None,
            widths: BTreeMap::new(),
        }
    }

    /// The job has changed since it was last exported or saved as a recipe.
    pub fn is_dirty(&self) -> bool {
        self.job != self.saved
    }

    pub fn mark_saved(&mut self) {
        self.saved = self.job.clone();
    }

    /// Reads new data (the file changed, or the tab's text was edited).
    pub fn set_data(&mut self, data: Data, text_version: Option<u64>) {
        self.data = data;
        self.text_version = text_version;
        self.stages.clear();
    }

    /// Runs whatever hasn't run yet.
    fn compute(&mut self) {
        if self.stages.len() > self.job.steps.len() + 1 {
            self.stages.truncate(self.job.steps.len() + 1);
        }
        if self.stages.is_empty() {
            let stage = match self.job.read.read(self.data.input()) {
                Ok(t) => Stage {
                    table: Arc::new(t),
                    error: None,
                    blocked: false,
                },
                Err(e) => Stage {
                    table: Arc::new(Table::default()),
                    error: Some(e),
                    blocked: false,
                },
            };
            self.stages.push(stage);
        }
        while self.stages.len() <= self.job.steps.len() {
            let i = self.stages.len() - 1;
            let prev = self.stages[i].clone();
            let step = &self.job.steps[i];
            let stage = if prev.error.is_some() || prev.blocked {
                Stage {
                    table: prev.table,
                    error: None,
                    blocked: true,
                }
            } else if !step.enabled {
                prev
            } else {
                match step.op.apply(&prev.table) {
                    Ok(t) => Stage {
                        table: Arc::new(t),
                        error: None,
                        blocked: false,
                    },
                    Err(e) => Stage {
                        table: prev.table,
                        error: Some(e),
                        blocked: false,
                    },
                }
            };
            self.stages.push(stage);
        }
    }

    /// Forgets the stages from `stage` on.
    fn invalidate(&mut self, stage: usize) {
        self.stages.truncate(stage);
    }

    fn view_stage(&self) -> usize {
        self.view
            .unwrap_or(self.job.steps.len())
            .min(self.job.steps.len())
    }

    /// The table that shows.
    pub fn table(&mut self) -> Arc<Table> {
        self.compute();
        self.stages[self.view_stage()].table.clone()
    }

    /// The table after every step.
    pub fn result(&mut self) -> Result<Arc<Table>, String> {
        self.compute();
        for (i, s) in self.stages.iter().enumerate() {
            if let Some(e) = &s.error {
                return Err(match i {
                    0 => e.clone(),
                    _ => t!("table-step-failed", step = i, message = e.clone()),
                });
            }
        }
        Ok(self.stages.last().expect("computed").table.clone())
    }

    /// The table before step `i` runs.
    fn before(&mut self, i: usize) -> Arc<Table> {
        self.compute();
        self.stages[i.min(self.stages.len() - 1)].table.clone()
    }

    fn remember(&mut self) {
        self.undo.push((self.job.clone(), self.view));
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) {
        if let Some((job, view)) = self.undo.pop() {
            self.redo
                .push((std::mem::replace(&mut self.job, job), self.view));
            self.view = view;
            self.after_job_change();
        }
    }

    pub fn redo(&mut self) {
        if let Some((job, view)) = self.redo.pop() {
            self.undo
                .push((std::mem::replace(&mut self.job, job), self.view));
            self.view = view;
            self.after_job_change();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn after_job_change(&mut self) {
        self.stages.clear();
        self.edit = None;
        self.clamp_cell();
    }

    fn clamp_cell(&mut self) {
        let t = self.table();
        self.cell = self.cell.and_then(|(r, c)| {
            (t.rows > 0 && !t.columns.is_empty())
                .then(|| (r.min(t.rows - 1), c.min(t.columns.len() - 1)))
        });
    }

    /// Adds a step after the one showing; it then shows.
    pub fn add_step(&mut self, step: TableStep) {
        self.remember();
        let at = self.view_stage();
        if let TableStep::RenameColumn { column, to } = &step {
            for later in &mut self.job.steps[at..] {
                later.op.rename_column(column, to);
            }
        }
        self.job.steps.insert(at, JobStep::from(step));
        self.view = if at + 1 == self.job.steps.len() {
            None
        } else {
            Some(at + 1)
        };
        self.invalidate(at + 1);
        self.clamp_cell();
    }

    fn replace_step(&mut self, i: usize, step: TableStep) {
        if self.job.steps.get(i).is_none_or(|s| s.op == step) {
            return;
        }
        self.remember();
        if let (TableStep::RenameColumn { to: old, .. }, TableStep::RenameColumn { to: new, .. }) =
            (&self.job.steps[i].op, &step)
        {
            let (old, new) = (old.clone(), new.clone());
            for later in &mut self.job.steps[i + 1..] {
                later.op.rename_column(&old, &new);
            }
        }
        self.job.steps[i].op = step;
        self.invalidate(i + 1);
        self.clamp_cell();
    }

    fn remove_step(&mut self, i: usize) {
        if i >= self.job.steps.len() {
            return;
        }
        self.remember();
        self.job.steps.remove(i);
        self.view = self.view.map(|v| if v > i { v - 1 } else { v });
        if self.view == Some(self.job.steps.len()) {
            self.view = None;
        }
        self.invalidate(i + 1);
        self.clamp_cell();
    }

    fn toggle_step(&mut self, i: usize) {
        self.remember();
        self.job.steps[i].enabled = !self.job.steps[i].enabled;
        self.invalidate(i + 1);
        self.clamp_cell();
    }

    fn move_step(&mut self, i: usize, up: bool) {
        let j = if up { i.wrapping_sub(1) } else { i + 1 };
        if j >= self.job.steps.len() {
            return;
        }
        self.remember();
        self.job.steps.swap(i, j);
        self.invalidate(i.min(j) + 1);
        self.clamp_cell();
    }

    fn set_source(&mut self, source: Source) {
        if source == self.job.read {
            return;
        }
        self.remember();
        self.job.read = source;
        self.stages.clear();
        self.clamp_cell();
    }

    /// The selected column's name in the shown table.
    fn selected_column(&mut self) -> Option<(usize, String)> {
        let t = self.table();
        let c = self.cell.map_or(0, |c| c.1);
        t.columns.get(c).map(|col| (c, col.name.clone()))
    }

    fn open(&mut self, kind: Dialog) {
        self.dialog = Some(OpenDialog {
            kind,
            editing: None,
            preview: None,
        });
    }

    /// Runs a command from a menu, the toolbar or a shortcut.
    pub fn command(&mut self, cmd: TableCmd) {
        self.message = None;
        let table = self.table();
        let column = self.selected_column();
        let need_column = |c: &Option<(usize, String)>| c.clone();
        match cmd {
            TableCmd::OpenAsTable | TableCmd::ShowAsTable | TableCmd::ShowAsText => {}
            TableCmd::ImportOptions => {
                let sheets = match &self.data {
                    Data::Bytes(b) => crate::table::import::sheet_names(b).unwrap_or_default(),
                    Data::Text(_) => Vec::new(),
                };
                self.open(Dialog::Import {
                    source: self.job.read.clone(),
                    sheets,
                });
            }
            TableCmd::Refresh => {
                self.stages.clear();
                self.clamp_cell();
            }
            TableCmd::Sort => {
                let keys = vec![SortKey {
                    column: column.map(|c| c.1).unwrap_or_default(),
                    descending: false,
                }];
                self.open(Dialog::Sort { keys });
            }
            TableCmd::RemoveDuplicates => {
                let columns = table.names().into_iter().map(|n| (n, true)).collect();
                self.open(Dialog::Dedupe { columns });
            }
            TableCmd::ChooseColumns => {
                let columns = table.names().into_iter().map(|n| (n, true)).collect();
                self.open(Dialog::Choose { columns });
            }
            TableCmd::RemoveEmptyRows => self.add_step(TableStep::RemoveEmptyRows),
            TableCmd::RemoveRow => {
                if let Some((r, _)) = self.cell {
                    self.add_step(TableStep::RemoveRows { rows: vec![r] });
                }
            }
            TableCmd::EditCell => {
                if let Some((row, col)) = self.cell {
                    self.edit = Some(CellEdit {
                        row,
                        col,
                        text: table.get(row, col).to_owned(),
                        focus: true,
                    });
                }
            }
            TableCmd::Export => {
                let export = self.job.write.clone();
                let dest = if export.is_text() {
                    Dest::NewTab
                } else {
                    Dest::File
                };
                self.open(Dialog::Export { export, dest });
            }
            TableCmd::SaveAsRecipe => {
                let name = self
                    .path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| tr("table-default-recipe-name"));
                let row_numbers = self.job.has_row_number_steps();
                self.open(Dialog::Recipe {
                    name,
                    description: String::new(),
                    row_numbers,
                });
            }
            _ => {
                let Some((c, name)) = need_column(&column) else {
                    self.message = Some(tr("table-no-columns"));
                    return;
                };
                match cmd {
                    TableCmd::RenameColumn => self.open(Dialog::Rename {
                        to: name.clone(),
                        column: name,
                    }),
                    TableCmd::DuplicateColumn => self.open(Dialog::Duplicate {
                        to: table.free_name(&name),
                        column: name,
                    }),
                    TableCmd::ChangeType => self.open(Dialog::ChangeType {
                        kind: table.columns[c].kind,
                        column: name,
                        format: String::new(),
                        decimal_comma: false,
                    }),
                    TableCmd::RemoveColumn => self.add_step(TableStep::RemoveColumns {
                        columns: vec![name],
                    }),
                    TableCmd::MoveLeft | TableCmd::MoveRight => {
                        let to = if cmd == TableCmd::MoveLeft {
                            c.saturating_sub(1)
                        } else {
                            (c + 1).min(table.columns.len().saturating_sub(1))
                        };
                        if to != c {
                            self.add_step(TableStep::MoveColumn { column: name, to });
                            self.cell = self.cell.map(|(r, _)| (r, to));
                        }
                    }
                    TableCmd::ReplaceValues => self.open(Dialog::Replace {
                        column: name,
                        all: false,
                        find: String::new(),
                        replace: String::new(),
                        whole_cell: false,
                        match_case: false,
                        regex: false,
                    }),
                    TableCmd::Text(op) => self.add_step(TableStep::Text {
                        column: name,
                        text: op,
                    }),
                    TableCmd::Filter => {
                        let distinct = distinct_values(&table, c);
                        self.open(Dialog::Filter {
                            column: name,
                            condition: Condition::Equals,
                            value: self
                                .cell
                                .map(|(r, _)| table.get(r, c).to_owned())
                                .unwrap_or_default(),
                            values: BTreeSet::new(),
                            match_case: false,
                            search: String::new(),
                            distinct,
                        })
                    }
                    TableCmd::SortAscending | TableCmd::SortDescending => {
                        self.add_step(TableStep::SortRows {
                            keys: vec![SortKey {
                                column: name,
                                descending: cmd == TableCmd::SortDescending,
                            }],
                        })
                    }
                    TableCmd::ColumnStats => self.open(Dialog::Stats { column: name }),
                    _ => {}
                }
            }
        }
    }

    /// Opens the dialog of step `i` to change it.
    fn edit_step(&mut self, i: usize) {
        let Some(step) = self.job.steps.get(i).map(|s| s.op.clone()) else {
            return;
        };
        let before = self.before(i);
        let kind = match step {
            TableStep::RenameColumn { column, to } => Dialog::Rename { column, to },
            TableStep::DuplicateColumn { column, to } => Dialog::Duplicate { column, to },
            TableStep::ChangeType {
                column,
                kind,
                format,
            } => Dialog::ChangeType {
                column,
                kind,
                decimal_comma: format == ",",
                format: if format == "," { String::new() } else { format },
            },
            TableStep::ReplaceValues {
                column,
                find,
                replace,
                whole_cell,
                match_case,
                regex,
            } => Dialog::Replace {
                all: column.is_empty(),
                column,
                find,
                replace,
                whole_cell,
                match_case,
                regex,
            },
            TableStep::FilterRows {
                column,
                condition,
                value,
                values,
                match_case,
            } => {
                let distinct = before
                    .find(&column)
                    .map(|c| distinct_values(&before, c))
                    .unwrap_or_default();
                Dialog::Filter {
                    column,
                    condition,
                    value,
                    values: values.into_iter().collect(),
                    match_case,
                    search: String::new(),
                    distinct,
                }
            }
            TableStep::SortRows { keys } => Dialog::Sort { keys },
            TableStep::RemoveDuplicates { columns } => Dialog::Dedupe {
                columns: before
                    .names()
                    .into_iter()
                    .map(|n| {
                        let on = columns.is_empty() || columns.contains(&n);
                        (n, on)
                    })
                    .collect(),
            },
            TableStep::ChooseColumns { columns } => {
                let mut list: Vec<(String, bool)> =
                    columns.iter().map(|c| (c.clone(), true)).collect();
                for n in before.names() {
                    if !columns.contains(&n) {
                        list.push((n, false));
                    }
                }
                Dialog::Choose { columns: list }
            }
            TableStep::EditCell { row, column, value } => {
                self.view = Some(i);
                let col = before.find(&column).unwrap_or(0);
                self.cell = Some((row, col));
                self.edit = Some(CellEdit {
                    row,
                    col,
                    text: value,
                    focus: true,
                });
                self.editing_cell_step = Some(i);
                return;
            }
            _ => return,
        };
        self.dialog = Some(OpenDialog {
            kind,
            editing: Some(i),
            preview: None,
        });
    }
}

/// Each value of column `c` and how many rows have it, most common first
/// (at most 1,000).
fn distinct_values(t: &Table, c: usize) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for v in t.columns[c].cells.iter() {
        *counts.entry(v).or_default() += 1;
    }
    let mut list: Vec<(String, usize)> =
        counts.into_iter().map(|(v, n)| (v.to_owned(), n)).collect();
    let kind = t.columns[c].kind;
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| kind.compare(&a.0, &b.0)));
    list.truncate(1000);
    list
}

// ------------------------------------------------------------------ drawing

impl TableTab {
    /// Draws the tab: Steps on the right, filters and find above the grid.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        id: Id,
        palette: &Palette,
        font_size: f32,
    ) -> Vec<TableEvent> {
        let mut events = Vec::new();
        self.compute();
        egui::Panel::right(id.with("steps"))
            .resizable(true)
            .default_size(270.0)
            .size_range(170.0..=560.0)
            .frame(
                egui::Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(6, 4)),
            )
            .show(ui, |ui| self.steps_panel(ui, palette));
        self.top_bar(ui, palette);
        self.grid(ui, id, palette, font_size);
        self.keys(ui);
        self.dialogs(ui.ctx(), id, palette, &mut events);
        events
    }

    /// The status bar's text: size, hidden rows and problems.
    pub fn status(&mut self) -> Vec<String> {
        let t = self.table();
        let mut out = vec![t!(
            "table-status-size",
            rows = thousands(t.rows),
            columns = t.columns.len()
        )];
        let source_rows = self.stages[0].table.rows;
        if source_rows > t.rows {
            out.push(t!(
                "table-status-removed",
                rows = thousands(source_rows - t.rows)
            ));
        }
        let problems = t.problems();
        if problems > 0 {
            out.push(t!("table-status-problems", count = problems));
        }
        if let Some((r, c)) = self.cell
            && let Some(col) = t.columns.get(c)
        {
            out.push(t!(
                "table-status-cell",
                row = r + 1,
                column = col.name.clone()
            ));
        }
        out
    }

    fn steps_panel(&mut self, ui: &mut Ui, palette: &Palette) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("table-steps")).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                use egui_phosphor::regular as ph;
                if ui
                    .add_enabled(
                        self.can_redo(),
                        egui::Button::new(ph::ARROW_CLOCKWISE).frame(false),
                    )
                    .on_hover_text(tr("redo"))
                    .clicked()
                {
                    self.redo();
                }
                if ui
                    .add_enabled(
                        self.can_undo(),
                        egui::Button::new(ph::ARROW_COUNTER_CLOCKWISE).frame(false),
                    )
                    .on_hover_text(tr("undo"))
                    .clicked()
                {
                    self.undo();
                }
            });
        });
        ui.separator();
        let shown = self.view_stage();
        let last = self.job.steps.len();
        let mut select = None;
        let mut edit = None;
        let mut toggle = None;
        let mut remove = None;
        let mut moved = None;
        let mut import = false;
        let footer_h = 92.0;
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height((ui.available_height() - footer_h).max(60.0))
            .show(ui, |ui| {
                let row = |ui: &mut Ui, on: bool, add: &mut dyn FnMut(&mut Ui)| {
                    let fill = if on {
                        palette.selection
                    } else {
                        Color32::TRANSPARENT
                    };
                    egui::Frame::new()
                        .fill(fill)
                        .corner_radius(3.0)
                        .inner_margin(Margin::symmetric(4, 3))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            add(ui)
                        });
                };
                let source_error = self.stages.first().and_then(|s| s.error.clone());
                row(ui, shown == 0, &mut |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("1").color(palette.text_dim).monospace());
                        ui.vertical(|ui| {
                            let r = ui.add(
                                egui::Label::new(RichText::new(tr("table-step-source")).strong())
                                    .sense(Sense::click()),
                            );
                            let d = ui.add(
                                egui::Label::new(
                                    RichText::new(self.job.read.summary())
                                        .size(11.5)
                                        .color(palette.text_dim),
                                )
                                .wrap()
                                .sense(Sense::click()),
                            );
                            for r in [r, d] {
                                if r.clicked() {
                                    select = Some(0);
                                }
                                if r.double_clicked() {
                                    import = true;
                                }
                                r.on_hover_text(tr("table-step-source-tip"));
                            }
                            if let Some(e) = &source_error {
                                ui.label(RichText::new(e).size(11.5).color(palette.brace_bad));
                            }
                        });
                    });
                });
                for i in 0..last {
                    let stage = self.stages.get(i + 1).cloned();
                    let step = &self.job.steps[i];
                    let on = shown == i + 1;
                    row(ui, on, &mut |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new((i + 2).to_string())
                                    .color(palette.text_dim)
                                    .monospace(),
                            );
                            let mut enabled = step.enabled;
                            if ui
                                .checkbox(&mut enabled, "")
                                .on_hover_text(tr("table-step-enabled-tip"))
                                .changed()
                            {
                                toggle = Some(i);
                            }
                            ui.vertical(|ui| {
                                let mut text = RichText::new(step.op.summary());
                                if !step.enabled {
                                    text = text.color(palette.text_dim).strikethrough();
                                }
                                let r = ui.add(egui::Label::new(text).wrap().sense(Sense::click()));
                                if r.clicked() {
                                    select = Some(i + 1);
                                }
                                if r.double_clicked() {
                                    edit = Some(i);
                                }
                                r.on_hover_text(tr("table-step-tip"));
                                if step.op.by_row_number() {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} {}",
                                            egui_phosphor::regular::WARNING,
                                            tr("table-step-row-number")
                                        ))
                                        .size(11.0)
                                        .color(palette.text_dim),
                                    )
                                    .on_hover_text(tr("table-step-row-number-tip"));
                                }
                                if let Some(st) = &stage {
                                    if let Some(e) = &st.error {
                                        ui.label(
                                            RichText::new(e).size(11.5).color(palette.brace_bad),
                                        );
                                    } else if st.blocked {
                                        ui.label(
                                            RichText::new(tr("table-step-blocked"))
                                                .size(11.5)
                                                .color(palette.text_dim),
                                        );
                                    }
                                }
                                if on {
                                    ui.horizontal(|ui| {
                                        use egui_phosphor::regular as ph;
                                        let small = |s: &str| egui::Button::new(s).small();
                                        if ui
                                            .add_enabled(i > 0, small(ph::ARROW_UP))
                                            .on_hover_text(tr("table-step-up"))
                                            .clicked()
                                        {
                                            moved = Some((i, true));
                                        }
                                        if ui
                                            .add_enabled(i + 1 < last, small(ph::ARROW_DOWN))
                                            .on_hover_text(tr("table-step-down"))
                                            .clicked()
                                        {
                                            moved = Some((i, false));
                                        }
                                        if ui
                                            .add(small(ph::PENCIL_SIMPLE))
                                            .on_hover_text(tr("table-step-edit"))
                                            .clicked()
                                        {
                                            edit = Some(i);
                                        }
                                        if ui
                                            .add(small(ph::TRASH))
                                            .on_hover_text(tr("table-step-delete"))
                                            .clicked()
                                        {
                                            remove = Some(i);
                                        }
                                    });
                                }
                            });
                        });
                    });
                }
            });
        ui.separator();
        ui.label(
            RichText::new(tr("table-steps-hint"))
                .size(11.0)
                .color(palette.text_dim),
        );
        ui.horizontal(|ui| {
            if ui.button(tr("table-export-dots")).clicked() {
                self.command(TableCmd::Export);
            }
            if ui.button(tr("table-save-as-recipe-dots")).clicked() {
                self.command(TableCmd::SaveAsRecipe);
            }
        });
        if let Some(s) = select {
            self.view = if s == last { None } else { Some(s) };
            self.clamp_cell();
        }
        if import {
            self.command(TableCmd::ImportOptions);
        }
        if let Some(i) = toggle {
            self.toggle_step(i);
        }
        if let Some((i, up)) = moved {
            self.move_step(i, up);
            let j = if up { i - 1 } else { i + 1 };
            self.view = if j + 1 == last { None } else { Some(j + 1) };
        }
        if let Some(i) = remove {
            self.remove_step(i);
        }
        if let Some(i) = edit {
            self.edit_step(i);
        }
    }

    fn top_bar(&mut self, ui: &mut Ui, palette: &Palette) {
        let filters: Vec<(usize, String)> = self
            .job
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.enabled && matches!(s.op, TableStep::FilterRows { .. }))
            .map(|(i, s)| (i, chip_text(&s.op)))
            .collect();
        let mut remove = None;
        let mut find_next = false;
        let mut show_result = false;
        egui::Frame::new()
            .inner_margin(Margin::symmetric(6, 4))
            .show(ui, |ui| {
                if let Some(m) = &self.message {
                    ui.label(RichText::new(m).color(palette.brace_bad));
                }
                if self.view.is_some_and(|v| v < self.job.steps.len()) {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(t!("table-viewing-step", step = self.view_stage() + 1))
                                .color(palette.text_dim),
                        );
                        if ui.link(tr("table-show-result")).clicked() {
                            show_result = true;
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if !filters.is_empty() {
                        ui.label(RichText::new(tr("table-filters")).color(palette.text_dim));
                        for (i, text) in &filters {
                            let b = egui::Button::new(
                                RichText::new(format!("{text}  {}", egui_phosphor::regular::X))
                                    .size(12.0),
                            )
                            .corner_radius(10.0)
                            .fill(palette.selection);
                            if ui.add(b).on_hover_text(tr("table-remove-filter")).clicked() {
                                remove = Some(*i);
                            }
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let id = Id::new("table-find").with(self as *const _ as usize);
                        let r = ui.add(
                            TextEdit::singleline(&mut self.find)
                                .id(id)
                                .hint_text(tr("table-find-hint"))
                                .desired_width(180.0),
                        );
                        if std::mem::take(&mut self.find_focus) {
                            r.request_focus();
                        }
                        if r.changed()
                            || (r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                        {
                            find_next = true;
                            if r.lost_focus() {
                                r.request_focus();
                            }
                        }
                        ui.label(egui_phosphor::regular::MAGNIFYING_GLASS);
                    });
                });
            });
        if let Some(i) = remove {
            self.remove_step(i);
        }
        if show_result {
            self.view = None;
            self.clamp_cell();
        }
        if find_next {
            self.find_next();
        }
    }

    /// Puts the find box in focus.
    pub fn focus_find(&mut self) {
        self.find_focus = true;
    }

    /// Moves to the next cell (after the selected one) that contains the
    /// find text, ignoring case.
    fn find_next(&mut self) {
        let needle = self.find.to_lowercase();
        if needle.is_empty() {
            return;
        }
        let t = self.table();
        let cols = t.columns.len();
        if cols == 0 || t.rows == 0 {
            return;
        }
        let total = t.rows * cols;
        let start = self.cell.map_or(0, |(r, c)| r * cols + c + 1);
        for k in 0..total {
            let at = (start + k) % total;
            let (r, c) = (at / cols, at % cols);
            if t.get(r, c).to_lowercase().contains(&needle) {
                self.cell = Some((r, c));
                self.scroll_to = Some(r);
                return;
            }
        }
        self.message = Some(t!("table-not-found", text = self.find.clone()));
    }

    /// The selected cell's value, for Copy.
    pub fn copy_text(&mut self) -> Option<String> {
        let t = self.table();
        let (r, c) = self.cell?;
        Some(t.columns.get(c)?.cells.get(r).to_owned())
    }

    fn column_width(&mut self, t: &Table, c: usize, char_w: f32) -> f32 {
        let col = &t.columns[c];
        *self.widths.entry(col.name.clone()).or_insert_with(|| {
            let longest = col
                .cells
                .iter()
                .take(200)
                .map(|v| v.chars().count())
                .max()
                .unwrap_or(0)
                .max(col.name.chars().count() + 6);
            (longest as f32 * char_w + 20.0).clamp(60.0, 320.0)
        })
    }

    fn grid(&mut self, ui: &mut Ui, id: Id, palette: &Palette, font_size: f32) {
        let t = self.table();
        if t.columns.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(tr("table-empty")).color(palette.text_dim));
            });
            return;
        }
        let font = egui::FontId::monospace(font_size);
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
        let row_h = ui.fonts_mut(|f| f.row_height(&font)) + 6.0;
        let widths: Vec<f32> = (0..t.columns.len())
            .map(|c| self.column_width(&t, c, char_w))
            .collect();
        let number_w = (thousands(t.rows).len() as f32 * char_w + 16.0).max(36.0);
        let sorted = self.sorted_columns();
        let needle = self.find.to_lowercase();
        let selected = self.cell;
        let mut edit = self.edit.take();
        let mut picked: Option<(usize, TableCmd)> = None;
        let mut clicked: Option<(usize, usize)> = None;
        let mut start_edit: Option<(usize, usize)> = None;
        let mut commit: Option<CellEdit> = None;
        let mut cancel = false;
        let mut filter_value: Option<(usize, String, bool)> = None;
        let mut copy: Option<String> = None;
        let shape: u64 = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            t.names().hash(&mut h);
            h.finish()
        };
        let scroll_to = self.scroll_to.take();
        ScrollArea::horizontal()
            .id_salt(id.with("grid-h"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut tb = TableBuilder::new(ui)
                    .id_salt(id.with(("grid", shape)))
                    .striped(true)
                    .resizable(true)
                    .sense(Sense::click())
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .auto_shrink([false, false])
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::INFINITY)
                    .column(GridColumn::exact(number_w));
                if let Some(r) = scroll_to {
                    tb = tb.scroll_to_row(r, Some(Align::Center));
                }
                for w in &widths {
                    tb = tb.column(GridColumn::initial(*w).at_least(40.0).clip(true));
                }
                tb.header(row_h + 4.0, |mut header| {
                    header.col(|_| {});
                    for (c, col) in t.columns.iter().enumerate() {
                        header.col(|ui| {
                            let badge = RichText::new(col.kind.badge())
                                .monospace()
                                .size(10.5)
                                .color(palette.text_dim)
                                .background_color(palette.line_highlight);
                            ui.label(badge).on_hover_text(col.kind.label());
                            use egui_phosphor::regular as ph;
                            let arrow = match sorted.get(&col.name) {
                                Some(true) => format!(" {}", ph::ARROW_DOWN),
                                Some(false) => format!(" {}", ph::ARROW_UP),
                                None => String::new(),
                            };
                            let r = ui.button(
                                RichText::new(format!("{}{arrow} {}", col.name, ph::CARET_DOWN))
                                    .strong(),
                            );
                            // Not the default popup id: the right-click
                            // menu uses that one and closes on a left click.
                            if let Some(Some(cmd)) = egui::Popup::menu(&r)
                                .id(id.with(("column-menu", c)))
                                .show(column_menu)
                                .map(|r| r.inner)
                            {
                                picked = Some((c, cmd));
                            }
                            r.context_menu(|ui| {
                                if let Some(cmd) = column_menu(ui) {
                                    picked = Some((c, cmd));
                                }
                            });
                        });
                    }
                })
                .body(|body| {
                    body.rows(row_h, t.rows, |mut row| {
                        let r = row.index();
                        row.col(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new((r + 1).to_string())
                                        .monospace()
                                        .size(font_size * 0.85)
                                        .color(palette.gutter_fg),
                                );
                            });
                        });
                        for (c, col) in t.columns.iter().enumerate() {
                            let v = col.cells.get(r);
                            let is_sel = selected == Some((r, c));
                            let editing = edit.as_ref().is_some_and(|e| e.row == r && e.col == c);
                            let (_, resp) = row.col(|ui| {
                                let rect = ui.max_rect().expand2(Vec2::new(4.0, 2.0));
                                if is_sel {
                                    ui.painter().rect_filled(rect, 0.0, palette.selection);
                                    ui.painter().rect_stroke(
                                        rect.shrink(1.0),
                                        0.0,
                                        Stroke::new(1.5, palette.accent),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                if col.is_problem(r) {
                                    ui.painter().rect_filled(
                                        rect,
                                        0.0,
                                        palette.brace_bad.gamma_multiply(0.25),
                                    );
                                } else if !needle.is_empty() && v.to_lowercase().contains(&needle) {
                                    ui.painter().rect_filled(rect, 0.0, palette.find_hit);
                                }
                                if editing {
                                    let e = edit.as_mut().expect("editing");
                                    let eid = Id::new("table-cell-edit");
                                    let te = ui.add(
                                        TextEdit::singleline(&mut e.text)
                                            .id(eid)
                                            .font(font.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                    if std::mem::take(&mut e.focus) {
                                        te.request_focus();
                                    }
                                    if ui.input(|i| i.key_pressed(Key::Escape)) {
                                        cancel = true;
                                    } else if te.lost_focus() {
                                        commit = edit.take();
                                    }
                                    return;
                                }
                                let text =
                                    RichText::new(v).font(font.clone()).color(palette.editor_fg);
                                if col.kind.is_number() {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.add(egui::Label::new(text).truncate().selectable(false));
                                    });
                                } else {
                                    ui.add(egui::Label::new(text).truncate().selectable(false));
                                }
                            });
                            if resp.clicked() {
                                clicked = Some((r, c));
                            }
                            if resp.double_clicked() {
                                start_edit = Some((r, c));
                            }
                            if resp.secondary_clicked() {
                                clicked = Some((r, c));
                            }
                            resp.context_menu(|ui| {
                                if ui.button(tr("table-copy-value")).clicked() {
                                    copy = Some(v.to_owned());
                                    ui.close();
                                }
                                if ui.button(tr("table-copy-row")).clicked() {
                                    copy = Some(t.row(r).join("\t"));
                                    ui.close();
                                }
                                if ui.button(tr("table-edit-cell")).clicked() {
                                    start_edit = Some((r, c));
                                    ui.close();
                                }
                                ui.separator();
                                let short: String = v.chars().take(30).collect();
                                if ui
                                    .button(t!("table-filter-is", value = short.clone()))
                                    .clicked()
                                {
                                    filter_value = Some((c, v.to_owned(), true));
                                    ui.close();
                                }
                                if ui
                                    .button(t!("table-filter-is-not", value = short))
                                    .clicked()
                                {
                                    filter_value = Some((c, v.to_owned(), false));
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button(tr("table-sort-ascending")).clicked() {
                                    picked = Some((c, TableCmd::SortAscending));
                                    ui.close();
                                }
                                if ui.button(tr("table-sort-descending")).clicked() {
                                    picked = Some((c, TableCmd::SortDescending));
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button(tr("table-remove-row")).clicked() {
                                    picked = Some((c, TableCmd::RemoveRow));
                                    ui.close();
                                }
                            });
                        }
                    });
                });
            });
        if !cancel && commit.is_none() {
            self.edit = edit;
        } else if cancel {
            self.editing_cell_step = None;
        }
        if let Some(text) = copy {
            ui.ctx().copy_text(text);
        }
        if let Some(cell) = clicked {
            self.cell = Some(cell);
            self.message = None;
        }
        if let Some((r, c)) = start_edit {
            self.cell = Some((r, c));
            self.command(TableCmd::EditCell);
        }
        if let Some(e) = commit {
            self.commit_edit(e);
        }
        if let Some((c, value, is)) = filter_value
            && let Some(col) = t.columns.get(c)
        {
            self.add_step(TableStep::FilterRows {
                column: col.name.clone(),
                condition: if is {
                    Condition::Equals
                } else {
                    Condition::NotEquals
                },
                value,
                values: Vec::new(),
                match_case: true,
            });
        }
        if let Some((c, cmd)) = picked {
            let r = self.cell.map_or(0, |c| c.0);
            self.cell = Some((r, c));
            self.command(cmd);
        }
    }

    fn commit_edit(&mut self, e: CellEdit) {
        let t = self.table();
        let Some(col) = t.columns.get(e.col) else {
            return;
        };
        let step = TableStep::EditCell {
            row: e.row,
            column: col.name.clone(),
            value: e.text,
        };
        match self.editing_cell_step.take() {
            Some(i) => self.replace_step(i, step),
            None if t.get(e.row, e.col) != step_value(&step) => self.add_step(step),
            None => {}
        }
    }

    /// The columns the last enabled sort step orders by: name → descending.
    fn sorted_columns(&self) -> BTreeMap<String, bool> {
        let upto = self.view_stage();
        self.job.steps[..upto]
            .iter()
            .rev()
            .filter(|s| s.enabled)
            .find_map(|s| match &s.op {
                TableStep::SortRows { keys } => Some(
                    keys.iter()
                        .map(|k| (k.column.clone(), k.descending))
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Arrow keys, Enter and Delete move around and edit the grid while no
    /// text box has the keyboard.
    fn keys(&mut self, ui: &mut Ui) {
        if self.dialog.is_some()
            || self.edit.is_some()
            || ui.ctx().memory(|m| m.focused().is_some())
        {
            return;
        }
        let t = self.table();
        if t.rows == 0 || t.columns.is_empty() {
            return;
        }
        let (mut r, mut c) = self.cell.unwrap_or((0, 0));
        let page = 20;
        let mut moved = false;
        let mut edit = false;
        let mut clear = false;
        let (mut undo, mut redo, mut copy) = (false, false, false);
        ui.input(|i| {
            for e in &i.events {
                match e {
                    egui::Event::Copy | egui::Event::Cut => copy = true,
                    egui::Event::Key {
                        key: Key::Z,
                        pressed: true,
                        modifiers,
                        ..
                    } if modifiers.command => {
                        if modifiers.shift {
                            redo = true;
                        } else {
                            undo = true;
                        }
                    }
                    egui::Event::Key {
                        key: Key::Y,
                        pressed: true,
                        modifiers,
                        ..
                    } if modifiers.command => redo = true,
                    _ => {}
                }
            }
            if i.modifiers.any() && !i.modifiers.shift_only() && !i.modifiers.command {
                return;
            }
            let cmd = i.modifiers.command;
            if i.key_pressed(Key::ArrowDown) {
                r = if cmd {
                    t.rows - 1
                } else {
                    (r + 1).min(t.rows - 1)
                };
                moved = true;
            }
            if i.key_pressed(Key::ArrowUp) {
                r = if cmd { 0 } else { r.saturating_sub(1) };
                moved = true;
            }
            if i.key_pressed(Key::ArrowRight) {
                c = (c + 1).min(t.columns.len() - 1);
                moved = true;
            }
            if i.key_pressed(Key::ArrowLeft) {
                c = c.saturating_sub(1);
                moved = true;
            }
            if i.key_pressed(Key::PageDown) {
                r = (r + page).min(t.rows - 1);
                moved = true;
            }
            if i.key_pressed(Key::PageUp) {
                r = r.saturating_sub(page);
                moved = true;
            }
            if i.key_pressed(Key::Home) {
                c = 0;
                if cmd {
                    r = 0;
                }
                moved = true;
            }
            if i.key_pressed(Key::End) {
                c = t.columns.len() - 1;
                if cmd {
                    r = t.rows - 1;
                }
                moved = true;
            }
            if !cmd && (i.key_pressed(Key::Enter) || i.key_pressed(Key::F2)) {
                edit = true;
            }
            if !cmd && i.key_pressed(Key::Delete) {
                clear = true;
            }
        });
        if undo {
            self.undo();
            return;
        }
        if redo {
            self.redo();
            return;
        }
        if copy && let Some(text) = self.copy_text() {
            ui.ctx().copy_text(text);
        }
        if moved {
            self.cell = Some((r, c));
            self.scroll_to = Some(r);
        }
        if edit && self.cell.is_some() {
            self.command(TableCmd::EditCell);
        }
        if clear
            && let Some((r, c)) = self.cell
            && !t.get(r, c).is_empty()
        {
            self.add_step(TableStep::EditCell {
                row: r,
                column: t.columns[c].name.clone(),
                value: String::new(),
            });
        }
    }
}

fn step_value(step: &TableStep) -> &str {
    match step {
        TableStep::EditCell { value, .. } => value,
        _ => "",
    }
}

/// A filter step as a short chip: `Sex = F`.
fn chip_text(step: &TableStep) -> String {
    match step {
        TableStep::FilterRows {
            column,
            condition,
            value,
            values,
            ..
        } => {
            let v = match condition {
                Condition::OneOf => {
                    let mut s = values
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                    if values.len() > 3 {
                        s.push_str(", …");
                    }
                    s
                }
                c if c.needs_value() => value.clone(),
                _ => String::new(),
            };
            format!("{column} {} {v}", condition.label())
                .trim_end()
                .to_owned()
        }
        other => other.summary(),
    }
}

/// The column header's menu. Returns the command clicked.
fn column_menu(ui: &mut Ui) -> Option<TableCmd> {
    let mut out = None;
    let mut item = |ui: &mut Ui, cmd: TableCmd, dots: bool| {
        let label = if dots {
            format!("{}…", cmd.label())
        } else {
            cmd.label()
        };
        if ui.button(label).clicked() {
            out = Some(cmd);
            ui.close();
        }
    };
    item(ui, TableCmd::SortAscending, false);
    item(ui, TableCmd::SortDescending, false);
    item(ui, TableCmd::Filter, true);
    ui.separator();
    item(ui, TableCmd::RenameColumn, true);
    item(ui, TableCmd::ChangeType, true);
    item(ui, TableCmd::DuplicateColumn, true);
    item(ui, TableCmd::MoveLeft, false);
    item(ui, TableCmd::MoveRight, false);
    item(ui, TableCmd::RemoveColumn, false);
    item(ui, TableCmd::ChooseColumns, true);
    ui.separator();
    item(ui, TableCmd::ReplaceValues, true);
    ui.menu_button(tr("table-text-menu"), |ui| {
        for op in TextOp::ALL {
            item(ui, TableCmd::Text(op), false);
        }
    });
    ui.separator();
    item(ui, TableCmd::RemoveDuplicates, true);
    item(ui, TableCmd::ColumnStats, false);
    out
}

/// `1234567` → `1,234,567`.
pub fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

// ------------------------------------------------------------------ dialogs

/// What a dialog's form makes.
enum Built {
    Step(TableStep),
    Source(Source),
    /// A form with nothing to apply (Statistics), or one handled itself.
    Nothing,
    Invalid(String),
}

impl TableTab {
    fn dialogs(
        &mut self,
        ctx: &egui::Context,
        id: Id,
        palette: &Palette,
        events: &mut Vec<TableEvent>,
    ) {
        let Some(mut open) = self.dialog.take() else {
            return;
        };
        let base = match open.editing {
            Some(i) => self.before(i),
            None => self.table(),
        };
        let source = self.stages.first().map(|s| s.table.clone());
        let wid = id.with("table-dialog");
        let (title, size) = dialog_title(&open.kind);
        let mut keep = true;
        let mut ok = false;
        let mut cancel = false;
        let mut built = Built::Nothing;
        let ok_label = match (&open.kind, open.editing) {
            (Dialog::Export { .. }, _) => {
                tr("table-export").trim_end_matches(['…', '.']).to_owned()
            }
            (Dialog::Recipe { .. }, _) => tr("save"),
            (Dialog::Stats { .. }, _) => tr("close"),
            (_, Some(_)) => tr("table-change-step"),
            _ => tr("ok"),
        };
        let data = self.data.clone();
        egui::Window::new(title)
            .id(wid)
            .collapsible(false)
            .resizable(true)
            .default_rect(crate::dialog::centered(ctx, size))
            .open(&mut keep)
            .show(ctx, |ui| {
                let body_h = crate::dialog::body_height(ui, wid, 40.0);
                ScrollArea::vertical()
                    .id_salt(wid.with("body"))
                    .max_height(body_h)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        built = form(ui, &mut open.kind, &base, source.as_deref(), &data, palette);
                        let preview_step = match &built {
                            Built::Step(s) => Some(s.clone()),
                            _ => None,
                        };
                        if let Some(step) = preview_step {
                            let key = serde_json::to_string(&step).unwrap_or_default();
                            if open.preview.as_ref().is_none_or(|p| p.0 != key) {
                                let r = step.apply(&base).map(Arc::new);
                                open.preview = Some((key, r));
                            }
                            ui.add_space(6.0);
                            ui.label(RichText::new(tr("table-preview")).color(palette.text_dim));
                            match &open.preview.as_ref().expect("set").1 {
                                Ok(t) => {
                                    let focus = step_column(&step);
                                    preview_grid(ui, t, focus.as_deref(), palette);
                                    ui.label(
                                        RichText::new(t!(
                                            "table-status-size",
                                            rows = thousands(t.rows),
                                            columns = t.columns.len()
                                        ))
                                        .size(11.5)
                                        .color(palette.text_dim),
                                    );
                                }
                                Err(e) => {
                                    ui.label(RichText::new(e).color(palette.brace_bad));
                                }
                            }
                        }
                        if let Built::Source(src) = &built {
                            let key = serde_json::to_string(src).unwrap_or_default();
                            if open.preview.as_ref().is_none_or(|p| p.0 != key) {
                                let r = src.read(data.input()).map(Arc::new);
                                open.preview = Some((key, r));
                            }
                            ui.add_space(6.0);
                            ui.label(RichText::new(tr("table-preview")).color(palette.text_dim));
                            match &open.preview.as_ref().expect("set").1 {
                                Ok(t) => {
                                    preview_grid(ui, t, None, palette);
                                    ui.label(
                                        RichText::new(t!(
                                            "table-status-size",
                                            rows = thousands(t.rows),
                                            columns = t.columns.len()
                                        ))
                                        .size(11.5)
                                        .color(palette.text_dim),
                                    );
                                }
                                Err(e) => {
                                    ui.label(RichText::new(e).color(palette.brace_bad));
                                }
                            }
                        }
                        if let Built::Invalid(e) = &built
                            && !e.is_empty()
                        {
                            ui.label(RichText::new(e).color(palette.brace_bad));
                        }
                    });
                crate::dialog::footer(ui, wid, |ui| {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let stats = matches!(open.kind, Dialog::Stats { .. });
                            if !stats && ui.button(tr("cancel")).clicked() {
                                cancel = true;
                            }
                            let can = !matches!(built, Built::Invalid(_));
                            if ui.add_enabled(can, egui::Button::new(&ok_label)).clicked() {
                                ok = true;
                            }
                        });
                    });
                });
                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    cancel = true;
                }
                if ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.command)
                    && !matches!(built, Built::Invalid(_))
                {
                    ok = true;
                }
            });
        if !keep || cancel {
            return;
        }
        if !ok {
            self.dialog = Some(open);
            return;
        }
        match built {
            Built::Step(step) => match open.editing {
                Some(i) => self.replace_step(i, step),
                None => {
                    if let TableStep::ChooseColumns { columns } = &step
                        && *columns == base.names()
                    {
                        return;
                    }
                    self.add_step(step)
                }
            },
            Built::Source(src) => self.set_source(src),
            Built::Invalid(_) => self.dialog = Some(open),
            Built::Nothing => match open.kind {
                Dialog::Export { export, dest } => self.export(ctx, export, dest, events),
                Dialog::Recipe {
                    name, description, ..
                } => events.push(TableEvent::SaveRecipe {
                    name: name.trim().to_owned(),
                    description,
                    job: self.job.clone(),
                }),
                _ => {}
            },
        }
    }

    /// Writes the result where the Export dialog says.
    fn export(
        &mut self,
        ctx: &egui::Context,
        export: Export,
        dest: Dest,
        events: &mut Vec<TableEvent>,
    ) {
        let table = match self.result() {
            Ok(t) => t,
            Err(e) => {
                self.message = Some(e);
                return;
            }
        };
        let out = match export.write(&table) {
            Ok(o) => o,
            Err(e) => {
                self.message = Some(e);
                return;
            }
        };
        let stem = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| tr("table-default-file-name"));
        let file_name = format!("{stem}.{}", export.extension());
        match (dest, out) {
            (Dest::NewTab, Output::Text(text)) => events.push(TableEvent::NewTab {
                text,
                syntax: export.syntax().to_owned(),
            }),
            (Dest::Clipboard, Output::Text(text)) => {
                ctx.copy_text(text);
                events.push(TableEvent::Message(tr("table-copied")));
            }
            (_, out) => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title(tr("table-export"))
                    .set_file_name(&file_name);
                if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
                    dialog = dialog.set_directory(dir);
                }
                let Some(path) = dialog.save_file() else {
                    return;
                };
                match std::fs::write(&path, out.into_bytes()) {
                    Ok(()) => events.push(TableEvent::Message(t!(
                        "table-exported-to",
                        path = path.display().to_string()
                    ))),
                    Err(e) => {
                        self.message = Some(t!(
                            "table-export-failed",
                            path = path.display().to_string(),
                            error = e.to_string()
                        ));
                        return;
                    }
                }
            }
        }
        self.job.write = export;
        self.mark_saved();
    }
}

fn dialog_title(kind: &Dialog) -> (String, Vec2) {
    let (id, size) = match kind {
        Dialog::Import { .. } => ("table-import-options", Vec2::new(760.0, 560.0)),
        Dialog::Rename { .. } => ("table-rename-column", Vec2::new(520.0, 380.0)),
        Dialog::Duplicate { .. } => ("table-duplicate-column", Vec2::new(520.0, 380.0)),
        Dialog::ChangeType { .. } => ("table-change-type", Vec2::new(600.0, 480.0)),
        Dialog::Replace { .. } => ("table-replace-values", Vec2::new(620.0, 500.0)),
        Dialog::Filter { .. } => ("table-filter", Vec2::new(620.0, 580.0)),
        Dialog::Sort { .. } => ("table-sort", Vec2::new(560.0, 460.0)),
        Dialog::Dedupe { .. } => ("table-remove-duplicates", Vec2::new(560.0, 520.0)),
        Dialog::Choose { .. } => ("table-choose-columns", Vec2::new(560.0, 560.0)),
        Dialog::Export { .. } => ("table-export", Vec2::new(720.0, 560.0)),
        Dialog::Recipe { .. } => ("table-save-as-recipe", Vec2::new(520.0, 320.0)),
        Dialog::Stats { .. } => ("table-column-stats", Vec2::new(460.0, 460.0)),
    };
    (tr(id).trim_end_matches(['…', '.']).to_owned(), size)
}

/// The column a step changes, to show first in its preview.
fn step_column(step: &TableStep) -> Option<String> {
    match step {
        TableStep::RenameColumn { to, .. } | TableStep::DuplicateColumn { to, .. } => {
            Some(to.clone())
        }
        TableStep::ChangeType { column, .. }
        | TableStep::ReplaceValues { column, .. }
        | TableStep::Text { column, .. }
        | TableStep::FilterRows { column, .. } => Some(column.clone()),
        TableStep::SortRows { keys } => keys.first().map(|k| k.column.clone()),
        _ => None,
    }
}

fn column_combo(ui: &mut Ui, id: Id, names: &[String], value: &mut String) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(value.clone())
        .width(220.0)
        .show_ui(ui, |ui| {
            for n in names {
                changed |= ui.selectable_value(value, n.clone(), n).changed();
            }
        });
    changed
}

/// Draws a dialog's form; returns what it makes.
fn form(
    ui: &mut Ui,
    kind: &mut Dialog,
    base: &Table,
    source: Option<&Table>,
    data: &Data,
    palette: &Palette,
) -> Built {
    let names = base.names();
    let hint = |ui: &mut Ui, s: String| {
        ui.label(RichText::new(s).size(11.5).color(palette.text_dim));
    };
    let grid = |ui: &mut Ui, label: &str, add: &mut dyn FnMut(&mut Ui)| {
        ui.horizontal(|ui| {
            ui.add_sized([130.0, 20.0], egui::Label::new(label));
            add(ui);
        });
    };
    match kind {
        Dialog::Import { source, sheets } => {
            let is_bytes = matches!(data, Data::Bytes(_));
            if !is_bytes {
                ui.horizontal(|ui| {
                    ui.label(tr("table-read-as"));
                    let csv = matches!(source, Source::Csv { .. });
                    if ui.radio(csv, tr("table-read-delimited")).clicked() && !csv {
                        *source = Source::default();
                    }
                    let json = matches!(source, Source::Json { .. });
                    if ui.radio(json, tr("table-read-json")).clicked() && !json {
                        *source = Source::Json {
                            rows: String::new(),
                            detect_types: true,
                        };
                    }
                });
            }
            match source {
                Source::Csv {
                    delimiter,
                    quote,
                    header,
                    ..
                } => {
                    grid(ui, &tr("table-delimiter"), &mut |ui| {
                        delimiter_combo(ui, Id::new("table-import-delim"), delimiter);
                    });
                    grid(ui, &tr("table-quote"), &mut |ui| {
                        egui::ComboBox::from_id_salt("table-import-quote")
                            .selected_text(match quote {
                                Some(q) => q.to_string(),
                                None => tr("table-quote-none"),
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(quote, Some('"'), "\"");
                                ui.selectable_value(quote, Some('\''), "'");
                                ui.selectable_value(quote, None, tr("table-quote-none"));
                            });
                    });
                    ui.checkbox(header, tr("table-first-row-headers"));
                }
                Source::Excel {
                    sheet, header_row, ..
                } => {
                    grid(ui, &tr("table-sheet"), &mut |ui| {
                        let shown = if sheet.is_empty() {
                            sheets.first().cloned().unwrap_or_default()
                        } else {
                            sheet.clone()
                        };
                        egui::ComboBox::from_id_salt("table-import-sheet")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                for s in sheets.iter() {
                                    ui.selectable_value(sheet, s.clone(), s);
                                }
                            });
                    });
                    grid(ui, &tr("table-header-row"), &mut |ui| {
                        ui.add(egui::DragValue::new(header_row).range(0..=1_048_576));
                    });
                    hint(ui, tr("table-header-row-hint"));
                }
                Source::Json { rows, .. } => {
                    grid(ui, &tr("table-json-rows"), &mut |ui| {
                        ui.add(
                            TextEdit::singleline(rows)
                                .hint_text(tr("table-json-rows-hint"))
                                .desired_width(260.0),
                        );
                    });
                    hint(ui, tr("table-json-flatten-hint"));
                }
            }
            let mut detect = match source {
                Source::Csv { detect_types, .. }
                | Source::Excel { detect_types, .. }
                | Source::Json { detect_types, .. } => *detect_types,
            };
            if ui.checkbox(&mut detect, tr("table-detect-types")).changed() {
                source.set_detect_types(detect);
            }
            Built::Source(source.clone())
        }
        Dialog::Rename { column, to } | Dialog::Duplicate { column, to } => {
            let rename = true;
            let _ = rename;
            grid(ui, &tr("table-column"), &mut |ui| {
                column_combo(ui, Id::new("table-rename-col"), &names, column);
            });
            let mut r = None;
            grid(ui, &tr("table-new-name"), &mut |ui| {
                r = Some(ui.add(TextEdit::singleline(to).desired_width(260.0)));
            });
            if let Some(r) = r
                && !r.has_focus()
                && ui.memory(|m| m.focused().is_none())
            {
                r.request_focus();
            }
            if to.trim().is_empty() {
                return Built::Invalid(String::new());
            }
            match kind {
                Dialog::Rename { column, to } => Built::Step(TableStep::RenameColumn {
                    column: column.clone(),
                    to: to.trim().to_owned(),
                }),
                Dialog::Duplicate { column, to } => Built::Step(TableStep::DuplicateColumn {
                    column: column.clone(),
                    to: to.trim().to_owned(),
                }),
                _ => unreachable!(),
            }
        }
        Dialog::ChangeType {
            column,
            kind: k,
            format,
            decimal_comma,
        } => {
            grid(ui, &tr("table-column"), &mut |ui| {
                column_combo(ui, Id::new("table-type-col"), &names, column);
            });
            grid(ui, &tr("table-type"), &mut |ui| {
                egui::ComboBox::from_id_salt("table-type-kind")
                    .selected_text(k.label())
                    .show_ui(ui, |ui| {
                        for kind in Kind::ALL {
                            ui.selectable_value(k, kind, kind.label());
                        }
                    });
            });
            let fmt = match k {
                Kind::Date | Kind::DateTime => {
                    grid(ui, &tr("table-date-pattern"), &mut |ui| {
                        ui.add(
                            TextEdit::singleline(format)
                                .hint_text("DD/MM/YYYY")
                                .desired_width(200.0),
                        );
                    });
                    hint(ui, tr("table-date-pattern-hint"));
                    format.trim().to_owned()
                }
                Kind::Integer | Kind::Decimal => {
                    ui.checkbox(decimal_comma, tr("table-decimal-comma"));
                    if *decimal_comma {
                        ",".to_owned()
                    } else {
                        String::new()
                    }
                }
                _ => String::new(),
            };
            hint(ui, tr("table-type-hint"));
            Built::Step(TableStep::ChangeType {
                column: column.clone(),
                kind: *k,
                format: fmt,
            })
        }
        Dialog::Replace {
            column,
            all,
            find,
            replace,
            whole_cell,
            match_case,
            regex,
        } => {
            grid(ui, &tr("table-column"), &mut |ui| {
                ui.add_enabled_ui(!*all, |ui| {
                    column_combo(ui, Id::new("table-replace-col"), &names, column);
                });
                ui.checkbox(all, tr("table-all-columns-check"));
            });
            grid(ui, &tr("table-find"), &mut |ui| {
                ui.add(TextEdit::singleline(find).desired_width(300.0));
            });
            grid(ui, &tr("table-replace-with"), &mut |ui| {
                ui.add(TextEdit::singleline(replace).desired_width(300.0));
            });
            ui.horizontal(|ui| {
                ui.checkbox(whole_cell, tr("table-whole-cell"));
                ui.checkbox(match_case, tr("match-case"));
                ui.checkbox(regex, tr("table-regex"));
            });
            if find.is_empty() {
                return Built::Invalid(String::new());
            }
            Built::Step(TableStep::ReplaceValues {
                column: if *all { String::new() } else { column.clone() },
                find: find.clone(),
                replace: replace.clone(),
                whole_cell: *whole_cell,
                match_case: *match_case,
                regex: *regex,
            })
        }
        Dialog::Filter {
            column,
            condition,
            value,
            values,
            match_case,
            search,
            distinct,
        } => {
            let before = column.clone();
            grid(ui, &tr("table-column"), &mut |ui| {
                column_combo(ui, Id::new("table-filter-col"), &names, column);
            });
            if *column != before
                && let Some(c) = base.find(column)
            {
                *distinct = distinct_values(base, c);
                values.clear();
            }
            grid(ui, &tr("table-keep-rows-where"), &mut |ui| {
                egui::ComboBox::from_id_salt("table-filter-cond")
                    .selected_text(condition.label())
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for c in Condition::ALL {
                            ui.selectable_value(condition, c, c.label());
                        }
                    });
            });
            if condition.needs_value() {
                grid(ui, &tr("table-value"), &mut |ui| {
                    ui.add(TextEdit::singleline(value).desired_width(300.0));
                });
            }
            if *condition == Condition::OneOf {
                ui.horizontal(|ui| {
                    ui.add(
                        TextEdit::singleline(search)
                            .hint_text(tr("table-search-values"))
                            .desired_width(200.0),
                    );
                    if ui.button(tr("table-select-shown")).clicked() {
                        let s = search.to_lowercase();
                        for (v, _) in distinct.iter() {
                            if v.to_lowercase().contains(&s) {
                                values.insert(v.clone());
                            }
                        }
                    }
                    if ui.button(tr("table-select-none")).clicked() {
                        values.clear();
                    }
                });
                ScrollArea::vertical()
                    .id_salt("table-filter-values")
                    .max_height(220.0)
                    .show(ui, |ui| {
                        let s = search.to_lowercase();
                        for (v, n) in distinct
                            .iter()
                            .filter(|(v, _)| v.to_lowercase().contains(&s))
                        {
                            let mut on = values.contains(v);
                            let label = if v.is_empty() {
                                format!("{}  ({n})", tr("table-empty-value"))
                            } else {
                                format!("{v}  ({n})")
                            };
                            if ui.checkbox(&mut on, label).changed() {
                                if on {
                                    values.insert(v.clone());
                                } else {
                                    values.remove(v);
                                }
                            }
                        }
                    });
            }
            ui.checkbox(match_case, tr("match-case"));
            if *condition == Condition::OneOf && values.is_empty() {
                return Built::Invalid(tr("table-pick-values"));
            }
            let mut sorted: Vec<String> = values.iter().cloned().collect();
            if let Some(c) = base.find(column) {
                let k = base.columns[c].kind;
                sorted.sort_by(|a, b| k.compare(a, b));
            }
            Built::Step(TableStep::FilterRows {
                column: column.clone(),
                condition: *condition,
                value: if condition.needs_value() {
                    value.clone()
                } else {
                    String::new()
                },
                values: if *condition == Condition::OneOf {
                    sorted
                } else {
                    Vec::new()
                },
                match_case: *match_case,
            })
        }
        Dialog::Sort { keys } => {
            let mut remove = None;
            for (i, key) in keys.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(if i == 0 {
                        tr("table-sort-by")
                    } else {
                        tr("table-then-by")
                    });
                    column_combo(ui, Id::new(("table-sort-col", i)), &names, &mut key.column);
                    ui.radio_value(&mut key.descending, false, tr("table-ascending"));
                    ui.radio_value(&mut key.descending, true, tr("table-descending"));
                    if i > 0 && ui.small_button(egui_phosphor::regular::X).clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                keys.remove(i);
            }
            if ui.button(tr("table-add-sort-key")).clicked() {
                let next = names
                    .iter()
                    .find(|n| !keys.iter().any(|k| &k.column == *n))
                    .cloned()
                    .unwrap_or_default();
                keys.push(SortKey {
                    column: next,
                    descending: false,
                });
            }
            hint(ui, tr("table-sort-hint"));
            Built::Step(TableStep::SortRows { keys: keys.clone() })
        }
        Dialog::Dedupe { columns } => {
            hint(ui, tr("table-dedupe-hint"));
            ui.horizontal(|ui| {
                if ui.button(tr("table-select-all")).clicked() {
                    columns.iter_mut().for_each(|c| c.1 = true);
                }
                if ui.button(tr("table-select-none")).clicked() {
                    columns.iter_mut().for_each(|c| c.1 = false);
                }
            });
            for (name, on) in columns.iter_mut() {
                ui.checkbox(on, name.as_str());
            }
            let chosen: Vec<String> = columns
                .iter()
                .filter(|c| c.1)
                .map(|c| c.0.clone())
                .collect();
            if chosen.is_empty() {
                return Built::Invalid(tr("table-pick-columns"));
            }
            Built::Step(TableStep::RemoveDuplicates {
                columns: if chosen.len() == columns.len() {
                    Vec::new()
                } else {
                    chosen
                },
            })
        }
        Dialog::Choose { columns } => {
            hint(ui, tr("table-choose-hint"));
            ui.horizontal(|ui| {
                if ui.button(tr("table-select-all")).clicked() {
                    columns.iter_mut().for_each(|c| c.1 = true);
                }
                if ui.button(tr("table-select-none")).clicked() {
                    columns.iter_mut().for_each(|c| c.1 = false);
                }
            });
            let mut swap = None;
            let n = columns.len();
            for (i, (name, on)) in columns.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    use egui_phosphor::regular as ph;
                    if ui
                        .add_enabled(i > 0, egui::Button::new(ph::ARROW_UP).small())
                        .clicked()
                    {
                        swap = Some((i, i - 1));
                    }
                    if ui
                        .add_enabled(i + 1 < n, egui::Button::new(ph::ARROW_DOWN).small())
                        .clicked()
                    {
                        swap = Some((i, i + 1));
                    }
                    ui.checkbox(on, name.as_str());
                });
            }
            if let Some((a, b)) = swap {
                columns.swap(a, b);
            }
            let chosen: Vec<String> = columns
                .iter()
                .filter(|c| c.1)
                .map(|c| c.0.clone())
                .collect();
            if chosen.is_empty() {
                return Built::Invalid(tr("table-choose-none"));
            }
            Built::Step(TableStep::ChooseColumns { columns: chosen })
        }
        Dialog::Export { export, dest } => {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(190.0);
                    for k in ExportKind::ALL {
                        if ui.selectable_label(export.kind() == k, k.label()).clicked()
                            && export.kind() != k
                        {
                            *export = k.default_export();
                            if !export.is_text() {
                                *dest = Dest::File;
                            }
                        }
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    export_options(ui, export);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(tr("table-result-goes-to"));
                        let text = export.is_text();
                        ui.add_enabled_ui(text, |ui| {
                            ui.radio_value(dest, Dest::NewTab, tr("result-new-tab"));
                        });
                        ui.radio_value(dest, Dest::File, tr("table-dest-file"));
                        ui.add_enabled_ui(text, |ui| {
                            ui.radio_value(dest, Dest::Clipboard, tr("result-clipboard"));
                        });
                    });
                    ui.label(RichText::new(tr("table-preview")).color(palette.text_dim));
                    let mut sample = base.clone();
                    let keep: Vec<usize> = (0..base.rows.min(30)).collect();
                    sample = sample.pick_rows(&keep);
                    let text = match export.write(&sample) {
                        Ok(Output::Text(t)) => t,
                        Ok(Output::Bytes(_)) => {
                            t!("table-workbook-preview", rows = thousands(base.rows))
                        }
                        Err(e) => e,
                    };
                    ScrollArea::both()
                        .id_salt("table-export-preview")
                        .max_height(240.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(RichText::new(text).monospace().size(12.0))
                                    .extend(),
                            );
                        });
                });
            });
            Built::Nothing
        }
        Dialog::Recipe {
            name,
            description,
            row_numbers,
        } => {
            grid(ui, &tr("recipe-name"), &mut |ui| {
                ui.add(TextEdit::singleline(name).desired_width(300.0));
            });
            grid(ui, &tr("recipe-description"), &mut |ui| {
                ui.add(TextEdit::singleline(description).desired_width(300.0));
            });
            hint(ui, tr("table-recipe-hint"));
            if *row_numbers {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        egui_phosphor::regular::WARNING,
                        tr("table-step-row-number-tip")
                    ))
                    .size(11.5)
                    .color(palette.brace_bad),
                );
            }
            if name.trim().is_empty() {
                return Built::Invalid(String::new());
            }
            Built::Nothing
        }
        Dialog::Stats { column } => {
            grid(ui, &tr("table-column"), &mut |ui| {
                column_combo(ui, Id::new("table-stats-col"), &names, column);
            });
            if let Some(c) = base.find(column) {
                column_stats(ui, base, c, source, palette);
            }
            Built::Nothing
        }
    }
}

fn delimiter_combo(ui: &mut Ui, id: Id, delimiter: &mut char) {
    let name = |d: char| crate::tools::csv::delimiter_name(d as u8);
    egui::ComboBox::from_id_salt(id)
        .selected_text(name(*delimiter))
        .show_ui(ui, |ui| {
            for d in [',', ';', '\t', '|', ' '] {
                ui.selectable_value(delimiter, d, name(d));
            }
        });
    let mut other = if [',', ';', '\t', '|', ' '].contains(delimiter) {
        String::new()
    } else {
        delimiter.to_string()
    };
    if ui
        .add(
            TextEdit::singleline(&mut other)
                .hint_text(tr("table-other-delimiter"))
                .desired_width(60.0)
                .char_limit(1),
        )
        .changed()
        && let Some(c) = other.chars().next()
    {
        *delimiter = c;
    }
}

fn export_options(ui: &mut Ui, export: &mut Export) {
    match export {
        Export::Csv {
            delimiter,
            quote,
            header,
            crlf,
        } => {
            ui.horizontal(|ui| {
                ui.label(tr("table-delimiter"));
                delimiter_combo(ui, Id::new("table-export-delim"), delimiter);
            });
            ui.horizontal(|ui| {
                ui.label(tr("table-quote-values"));
                egui::ComboBox::from_id_salt("table-export-quote")
                    .selected_text(quote.label())
                    .show_ui(ui, |ui| {
                        for q in QuoteMode::ALL {
                            ui.selectable_value(quote, q, q.label());
                        }
                    });
            });
            ui.checkbox(header, tr("table-header-row-out"));
            ui.checkbox(crlf, tr("table-windows-line-ends"));
        }
        Export::Excel { sheet } => {
            ui.horizontal(|ui| {
                ui.label(tr("table-sheet-name"));
                ui.add(
                    TextEdit::singleline(sheet)
                        .desired_width(200.0)
                        .char_limit(31),
                );
            });
        }
        Export::Json {
            pretty,
            typed,
            empty_as_null,
        } => {
            ui.checkbox(pretty, tr("table-json-pretty"));
            ui.checkbox(typed, tr("table-json-typed"));
            ui.checkbox(empty_as_null, tr("table-json-null"));
        }
        Export::JsonLines {
            typed,
            empty_as_null,
        } => {
            ui.checkbox(typed, tr("table-json-typed"));
            ui.checkbox(empty_as_null, tr("table-json-null"));
        }
        Export::Markdown => {}
    }
}

/// Counts, emptiness and range of a column, and its most common values.
fn column_stats(ui: &mut Ui, t: &Table, c: usize, _source: Option<&Table>, palette: &Palette) {
    let col = &t.columns[c];
    let empty = col.cells.iter().filter(|v| v.trim().is_empty()).count();
    let distinct = distinct_values(t, c);
    let mut values: Vec<&str> = col.cells.iter().filter(|v| !v.is_empty()).collect();
    values.sort_by(|a, b| col.kind.compare(a, b));
    let lines = [
        (tr("table-type"), col.kind.label()),
        (tr("table-stats-rows"), thousands(t.rows)),
        (tr("table-stats-filled"), thousands(t.rows - empty)),
        (tr("table-stats-empty"), thousands(empty)),
        (
            tr("table-stats-distinct"),
            if distinct.len() >= 1000 {
                "1,000+".to_owned()
            } else {
                thousands(distinct.len())
            },
        ),
        (tr("table-stats-problems"), thousands(col.problems())),
        (
            tr("table-stats-min"),
            values.first().map(|s| s.to_string()).unwrap_or_default(),
        ),
        (
            tr("table-stats-max"),
            values.last().map(|s| s.to_string()).unwrap_or_default(),
        ),
    ];
    for (k, v) in lines {
        ui.horizontal(|ui| {
            ui.add_sized(
                [130.0, 18.0],
                egui::Label::new(RichText::new(k).color(palette.text_dim)),
            );
            ui.label(v);
        });
    }
    ui.add_space(6.0);
    ui.label(RichText::new(tr("table-stats-common")).color(palette.text_dim));
    for (v, n) in distinct.iter().take(10) {
        let shown = if v.is_empty() {
            tr("table-empty-value")
        } else {
            v.clone()
        };
        ui.label(
            RichText::new(format!("{n:>8}  {shown}"))
                .monospace()
                .size(12.0),
        );
    }
}

/// The first rows of a table as aligned text, the focus column first.
fn preview_grid(ui: &mut Ui, t: &Table, focus: Option<&str>, palette: &Palette) {
    let mut order: Vec<usize> = (0..t.columns.len()).collect();
    if let Some(f) = focus.and_then(|f| t.find(f)) {
        order.retain(|&c| c != f);
        order.insert(0, f);
    }
    order.truncate(8);
    let rows = t.rows.min(ROW_LIMIT_PREVIEW);
    let width = |c: usize| {
        (0..rows)
            .map(|r| t.get(r, c).chars().count())
            .chain([t.columns[c].name.chars().count()])
            .max()
            .unwrap_or(0)
            .min(24)
    };
    let widths: Vec<usize> = order.iter().map(|&c| width(c)).collect();
    let cell = |s: &str, w: usize| {
        let mut v: String = s.chars().take(w).collect();
        if s.chars().count() > w {
            v.pop();
            v.push('…');
        }
        let pad = w - v.chars().count();
        format!("{v}{}", " ".repeat(pad))
    };
    let mut text = String::new();
    let header: Vec<String> = order
        .iter()
        .zip(&widths)
        .map(|(&c, &w)| cell(&t.columns[c].name, w))
        .collect();
    text.push_str(&header.join(" │ "));
    text.push('\n');
    for r in 0..rows {
        let line: Vec<String> = order
            .iter()
            .zip(&widths)
            .map(|(&c, &w)| cell(t.get(r, c), w))
            .collect();
        text.push_str(&line.join(" │ "));
        text.push('\n');
    }
    egui::Frame::new()
        .fill(palette.editor_bg)
        .corner_radius(4.0)
        .inner_margin(Margin::same(6))
        .show(ui, |ui| {
            ScrollArea::horizontal()
                .id_salt("table-preview-grid")
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(text)
                                .monospace()
                                .size(12.0)
                                .color(palette.editor_fg),
                        )
                        .extend(),
                    );
                });
        });
}
