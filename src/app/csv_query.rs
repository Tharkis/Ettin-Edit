//! CsvQuery: which tabs are tables, sending queries to the panel's worker,
//! re-running when the current tab changes, and the panel itself.

use egui::{Frame, Margin, Ui};

use super::NixPadApp;
use crate::actions::Action;
use crate::csv_query::{Active, Job, PanelAction, ROW_LIMIT, Source, View};
use crate::tools::csv;
use crate::tools::sql::table_name;

/// Seconds to wait after an edit before querying again.
const RERUN_DELAY: f64 = 0.5;
/// Text looked at to guess a tab's format.
const DETECT_CHARS: usize = 64 * 1024;

impl NixPadApp {
    /// Every tab's table name, in tab order: its file name made into an SQL
    /// name, with `_2`, `_3`... when two tabs would share one.
    fn csv_table_names(&self) -> Vec<String> {
        let mut used = std::collections::HashSet::new();
        self.docs
            .iter()
            .map(|d| {
                let base = table_name(&d.title());
                let mut name = base.clone();
                let mut n = 2;
                while !used.insert(name.to_lowercase()) {
                    name = format!("{base}_{n}");
                    n += 1;
                }
                name
            })
            .collect()
    }

    /// Guesses the format of tab `index` again when its text changed.
    fn csv_detect(&mut self, index: usize) -> csv::CsvFormat {
        let doc = &self.docs[index];
        let version = doc.editor.buffer.version();
        let settings = self.csv.settings.entry(doc.id).or_default();
        if let Some((v, f)) = settings.detected
            && v == version
        {
            return f;
        }
        let len = doc.editor.buffer.len_chars();
        let sample = doc.editor.buffer.slice_string(0..len.min(DETECT_CHARS));
        let ext = doc
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .map(|e| e.to_string_lossy().to_lowercase());
        let format = csv::detect(&sample, ext.as_deref());
        settings.detected = Some((version, format));
        format
    }

    /// The key of tab `index`'s table: changes with its text and format.
    fn csv_key(&mut self, index: usize) -> u64 {
        let detected = self.csv_detect(index);
        let doc = &self.docs[index];
        let version = doc.editor.buffer.version();
        self.csv.settings[&doc.id].key(version, detected)
    }

    /// Sends the query in the SQL box. The current tab is `this`; other tabs
    /// are loaded when the query names them.
    pub(super) fn run_csv_query(&mut self, export: bool) {
        if self.docs.is_empty() {
            return;
        }
        let names = self.csv_table_names();
        let sql = self.csv.sql.clone();
        let words: std::collections::HashSet<String> = sql
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect();
        let mut sources = Vec::new();
        for (i, name) in names.iter().enumerate() {
            if i != self.active && !words.contains(&name.to_lowercase()) {
                continue;
            }
            let key = self.csv_key(i);
            let detected = self.csv.settings[&self.docs[i].id].detected.map(|d| d.1);
            let settings = &self.csv.settings[&self.docs[i].id];
            let format = settings.format(detected.unwrap_or_default());
            let types = settings.types.clone();
            let text = (self.csv.sent.get(name) != Some(&key))
                .then(|| self.docs[i].editor.text_snapshot());
            sources.push(Source {
                name: name.clone(),
                key,
                text,
                format,
                types,
            });
        }
        let active_key = self.csv_key(self.active);
        self.csv.last_run = Some((self.docs[self.active].id, active_key));
        self.csv.stale_since = None;
        let id = self.csv.next_job_id();
        let stats = self.csv.view == View::Columns;
        self.csv.send(Job {
            id,
            sources,
            this: Some(names[self.active].clone()),
            sql,
            limit: ROW_LIMIT,
            stats,
            export,
        });
    }

    pub(super) fn run_csv_action(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::ToggleCsvQuery => {
                self.csv.toggle();
                if !self.csv.open {
                    self.focus_editor = true;
                }
            }
            Action::RunCsvQuery => {
                if !self.csv.open {
                    self.csv.toggle();
                }
                self.run_csv_query(false);
            }
            Action::CopyCsvResults => self.csv.copy_results(ctx),
            Action::CsvResultsToNewTab => self.run_csv_query(true),
            _ => {}
        }
    }

    /// The CsvQuery panel at the bottom of the window.
    pub(super) fn csv_query_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if !self.csv.open {
            return;
        }
        if self.csv.poll() {
            let export = self.csv.retry_was_export();
            self.run_csv_query(export);
        }
        if let Some(text) = self.csv.exported.take() {
            self.new_tab();
            let doc = &mut self.docs[self.active];
            doc.editor = crate::editor::Editor::new(&text);
            doc.editor.buffer.mark_modified();
        }
        // Query again when the current tab changes or is edited.
        if !self.docs.is_empty() && !self.csv.is_running() {
            let id = self.docs[self.active].id;
            let key = self.csv_key(self.active);
            if self.csv.last_run != Some((id, key)) {
                let now = ctx.input(|i| i.time);
                let same_tab = self.csv.last_run.is_some_and(|(d, _)| d == id);
                let since = *self.csv.stale_since.get_or_insert(now);
                if !same_tab || now - since >= RERUN_DELAY {
                    self.run_csv_query(false);
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f64(RERUN_DELAY));
                }
            }
        }
        if self.csv.is_running() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        let palette = self.palette();
        let names = self.csv_table_names();
        let all: Vec<(String, String)> = names
            .iter()
            .zip(&self.docs)
            .map(|(n, d)| (n.clone(), d.title()))
            .collect();
        let view_before = self.csv.view;
        let mut action = None;
        let active = self.docs.get(self.active).map(|d| d.id);
        let detected = active.map(|_| self.csv_detect(self.active));
        egui::Panel::bottom("csvquery")
            .resizable(true)
            .default_size(280.0)
            .min_size(120.0)
            .frame(
                Frame::new()
                    .fill(palette.chrome_bg)
                    .inner_margin(Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| {
                let csv = &mut self.csv;
                let mut settings = active.map(|id| csv.settings.remove(&id).unwrap_or_default());
                let a = settings
                    .as_mut()
                    .zip(detected)
                    .map(|(settings, detected)| Active {
                        table: &names[self.active],
                        detected,
                        settings,
                    });
                action = csv.ui(ui, &palette, a, &all);
                if let (Some(id), Some(s)) = (active, settings) {
                    csv.settings.insert(id, s);
                }
            });
        if !self.csv.open {
            self.focus_editor = true;
        }
        if self.csv.view != view_before && self.csv.view == View::Columns {
            self.csv.last_run = None;
        }
        match action {
            Some(PanelAction::Run) => self.run_csv_query(false),
            Some(PanelAction::ToNewTab) => self.run_csv_query(true),
            Some(PanelAction::InsertTable(name)) => {
                let sql = &mut self.csv.sql;
                if !sql.is_empty() && !sql.ends_with(char::is_whitespace) {
                    sql.push(' ');
                }
                sql.push_str(&name);
                self.csv.focus_sql = true;
            }
            None => {}
        }
    }
}
