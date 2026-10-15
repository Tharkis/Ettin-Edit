//! The CsvQuery panel: an SQL box over the current tab read as a table, a
//! sortable results grid, and per-column statistics. Queries run on a
//! background thread that keeps the tables loaded between queries.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{Align, Color32, FontId, Key, Layout, Modifiers, RichText, Sense, TextEdit, Ui, Vec2};
use egui_extras::{Column, TableBuilder};

use crate::search_ui::icon_button;
use crate::theme::Palette;
use crate::tools::csv::{self, ColType, CsvFormat};
use crate::tools::sql::{self, ColumnStats, Db, QueryResult, Value};

/// Rows shown in the grid; a query returning more keeps the first ones.
pub const ROW_LIMIT: usize = 100_000;
const HISTORY: usize = 30;
pub const DEFAULT_SQL: &str = "SELECT * FROM this";

/// One tab's table, as sent to the worker.
pub struct Source {
    pub name: String,
    /// Changes whenever the text, the format or the column types change.
    pub key: u64,
    /// The tab's text, sent only when the worker doesn't have this key yet.
    pub text: Option<Arc<String>>,
    pub format: CsvFormat,
    pub types: Vec<Option<ColType>>,
}

pub struct Job {
    pub id: u64,
    pub sources: Vec<Source>,
    /// The table `this` points at.
    pub this: Option<String>,
    pub sql: String,
    pub limit: usize,
    /// Also work out column statistics of `this`.
    pub stats: bool,
    /// Return the whole result as CSV, for a new tab.
    pub export: bool,
}

/// A loaded table's shape.
#[derive(Clone, Debug, Default)]
pub struct TableInfo {
    pub columns: Vec<String>,
    /// Types guessed from the values.
    pub inferred: Vec<ColType>,
    pub rows: usize,
}

struct Reply {
    id: u64,
    result: Result<QueryResult, String>,
    stats: Option<Result<Vec<ColumnStats>, String>>,
    tables: Vec<(String, u64, Result<TableInfo, String>)>,
    export: Option<String>,
    /// Tables the worker needs the text of (it doesn't have that key).
    missing: Vec<String>,
}

struct Worker {
    jobs: Sender<Job>,
    replies: Receiver<Reply>,
}

impl Worker {
    fn spawn() -> Self {
        let (jobs, job_rx) = channel::<Job>();
        let (reply_tx, replies) = channel();
        std::thread::Builder::new()
            .name("csvquery".into())
            .spawn(move || {
                let mut db = match Db::new() {
                    Ok(db) => db,
                    Err(e) => {
                        for job in job_rx {
                            let _ = reply_tx.send(Reply::error(job.id, e.clone()));
                        }
                        return;
                    }
                };
                let mut loaded: HashMap<String, (u64, TableInfo)> = HashMap::new();
                while let Ok(mut job) = job_rx.recv() {
                    // Only the latest query matters.
                    while let Ok(newer) = job_rx.try_recv() {
                        job = newer;
                    }
                    let reply = run_job(&mut db, &mut loaded, job);
                    if reply_tx.send(reply).is_err() {
                        return;
                    }
                }
            })
            .expect("start the CsvQuery thread");
        Self { jobs, replies }
    }
}

impl Reply {
    fn error(id: u64, e: String) -> Self {
        Reply {
            id,
            result: Err(e),
            stats: None,
            tables: Vec::new(),
            export: None,
            missing: Vec::new(),
        }
    }
}

fn run_job(db: &mut Db, loaded: &mut HashMap<String, (u64, TableInfo)>, job: Job) -> Reply {
    let mut reply = Reply::error(job.id, String::new());
    let mut load_errors = Vec::new();
    for src in &job.sources {
        if loaded.get(&src.name).is_some_and(|(k, _)| *k == src.key) && db.has_table(&src.name) {
            continue;
        }
        let Some(text) = &src.text else {
            reply.missing.push(src.name.clone());
            continue;
        };
        let info = csv::parse(text, &src.format)
            .map_err(|e| e.message)
            .and_then(|table| {
                let inferred = csv::infer_types(&table, src.format.delimiter);
                let types: Vec<ColType> = inferred
                    .iter()
                    .enumerate()
                    .map(|(i, t)| src.types.get(i).copied().flatten().unwrap_or(*t))
                    .collect();
                db.load(&src.name, &table, &types, src.format.delimiter)?;
                Ok(TableInfo {
                    columns: table.columns,
                    inferred,
                    rows: table.rows.len(),
                })
            });
        match &info {
            Ok(i) => {
                loaded.insert(src.name.clone(), (src.key, i.clone()));
            }
            Err(e) => {
                loaded.remove(&src.name);
                load_errors.push(format!("{}: {e}", src.name));
            }
        }
        reply.tables.push((src.name.clone(), src.key, info));
    }
    if !reply.missing.is_empty() {
        return reply;
    }
    let names: Vec<&str> = job.sources.iter().map(|s| s.name.as_str()).collect();
    db.retain(&names);
    loaded.retain(|k, _| names.contains(&k.as_str()));
    if let Some(this) = &job.this
        && db.has_table(this)
        && let Err(e) = db.set_this(this)
    {
        reply.result = Err(e);
        return reply;
    }
    let limit = if job.export { usize::MAX } else { job.limit };
    reply.result = db.query(&job.sql, limit).map_err(|mut e| {
        for l in &load_errors {
            e.push_str(&format!("\n{l}"));
        }
        e
    });
    if job.stats
        && let Some(this) = &job.this
        && db.has_table(this)
    {
        reply.stats = Some(db.stats(this));
    }
    if job.export
        && let Ok(r) = &reply.result
    {
        let rows: Vec<Vec<String>> = r
            .rows
            .iter()
            .map(|row| row.iter().map(Value::display).collect())
            .collect();
        reply.export = Some(csv::write_csv(&r.columns, &rows));
    }
    reply
}

/// The user's choices for one tab; `None` means "as detected".
#[derive(Clone, Debug, Default)]
pub struct DocSettings {
    pub detected: Option<(u64, CsvFormat)>,
    pub delimiter: Option<u8>,
    pub quote: Option<Option<u8>>,
    pub header: Option<bool>,
    pub types: Vec<Option<ColType>>,
}

impl DocSettings {
    pub fn format(&self, detected: CsvFormat) -> CsvFormat {
        CsvFormat {
            delimiter: self.delimiter.unwrap_or(detected.delimiter),
            quote: self.quote.unwrap_or(detected.quote),
            header: self.header.unwrap_or(detected.header),
        }
    }

    pub fn key(&self, version: u64, detected: CsvFormat) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        version.hash(&mut h);
        self.format(detected).hash(&mut h);
        self.types.hash(&mut h);
        h.finish()
    }

    fn is_custom(&self) -> bool {
        self.delimiter.is_some()
            || self.quote.is_some()
            || self.header.is_some()
            || self.types.iter().any(Option::is_some)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Results,
    Columns,
}

/// The tab shown as `this`, for the panel to draw.
pub struct Active<'a> {
    pub table: &'a str,
    pub detected: CsvFormat,
    pub settings: &'a mut DocSettings,
}

pub enum PanelAction {
    Run,
    ToNewTab,
    InsertTable(String),
}

#[derive(Default)]
pub struct CsvQueryPanel {
    pub open: bool,
    pub sql: String,
    history: Vec<String>,
    worker: Option<Worker>,
    next_id: u64,
    /// The job being run, and whether it is an export.
    running: Option<(u64, bool)>,
    /// What the worker has, by table name.
    pub sent: HashMap<String, u64>,
    pub result: Option<QueryResult>,
    pub error: Option<String>,
    /// Sorted row order of `result`.
    order: Vec<usize>,
    sort: Option<(usize, bool)>,
    selected: Option<usize>,
    pub view: View,
    pub tables: HashMap<String, TableInfo>,
    pub stats: Option<Vec<ColumnStats>>,
    pub settings: HashMap<u64, DocSettings>,
    /// The tab and key the last query ran on.
    pub last_run: Option<(u64, u64)>,
    pub stale_since: Option<f64>,
    pub focus_sql: bool,
    /// CSV of a finished export, for the app to open in a new tab.
    pub exported: Option<String>,
    /// The job last sent, kept to send again with text the worker lacked.
    pending_retry: bool,
}

impl CsvQueryPanel {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            if self.sql.trim().is_empty() {
                self.sql = DEFAULT_SQL.into();
            }
            self.focus_sql = true;
            self.last_run = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    pub fn next_job_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn send(&mut self, job: Job) {
        if !job.sql.trim().is_empty() {
            let sql = job.sql.trim().to_owned();
            self.history.retain(|h| h != &sql);
            self.history.insert(0, sql);
            self.history.truncate(HISTORY);
        }
        for s in &job.sources {
            if s.text.is_some() {
                self.sent.insert(s.name.clone(), s.key);
            }
        }
        self.running = Some((job.id, job.export));
        let worker = self.worker.get_or_insert_with(Worker::spawn);
        if worker.jobs.send(job).is_err() {
            self.worker = None;
            self.running = None;
            self.error = Some(t!("query-engine-stopped"));
        }
    }

    /// Takes a finished query. Returns true when the panel should run the
    /// job again with the text of tables the worker didn't have.
    pub fn poll(&mut self) -> bool {
        let Some(worker) = &self.worker else {
            return false;
        };
        let replies: Vec<Reply> = worker.replies.try_iter().collect();
        let mut retry = false;
        for reply in replies {
            if self.running.map(|r| r.0) != Some(reply.id) {
                continue;
            }
            let export = self.running.is_some_and(|r| r.1);
            self.running = None;
            for (name, key, info) in reply.tables {
                match info {
                    Ok(info) => {
                        self.sent.insert(name.clone(), key);
                        self.tables.insert(name, info);
                    }
                    Err(_) => {
                        self.sent.remove(&name);
                        self.tables.remove(&name);
                    }
                }
            }
            if !reply.missing.is_empty() {
                for m in &reply.missing {
                    self.sent.remove(m);
                }
                retry = true;
                self.pending_retry = export;
                continue;
            }
            if export {
                match reply.export {
                    Some(csv) => self.exported = Some(csv),
                    None => {
                        self.error = reply.result.err();
                    }
                }
                continue;
            }
            if let Some(stats) = reply.stats {
                self.stats = stats.ok();
            }
            match reply.result {
                Ok(r) => {
                    self.error = None;
                    let keep_sort = self
                        .result
                        .as_ref()
                        .is_some_and(|old| old.columns == r.columns);
                    self.result = Some(r);
                    if !keep_sort {
                        self.sort = None;
                        self.selected = None;
                    }
                    self.sort_rows();
                }
                Err(e) => self.error = Some(e),
            }
        }
        retry
    }

    /// Whether the job to send again (after `poll` asked for it) was an export.
    pub fn retry_was_export(&self) -> bool {
        self.pending_retry
    }

    fn sort_rows(&mut self) {
        let Some(r) = &self.result else {
            self.order.clear();
            return;
        };
        self.order = (0..r.rows.len()).collect();
        if let Some((col, asc)) = self.sort
            && col < r.columns.len()
        {
            self.order.sort_by(|&a, &b| {
                let o = r.rows[a][col].sort_cmp(&r.rows[b][col]);
                if asc { o } else { o.reverse() }
            });
        }
    }

    /// The shown rows, in the shown order, as text.
    fn shown_rows(&self) -> Vec<Vec<String>> {
        let Some(r) = &self.result else {
            return Vec::new();
        };
        self.order
            .iter()
            .map(|&i| r.rows[i].iter().map(Value::display).collect())
            .collect()
    }

    /// Copies the results as tab-separated text, for spreadsheets.
    pub fn copy_results(&self, ctx: &egui::Context) {
        if let Some(r) = &self.result {
            ctx.copy_text(csv::write_tsv(&r.columns, &self.shown_rows()));
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        active: Option<Active>,
        all_tables: &[(String, String)],
    ) -> Option<PanelAction> {
        let mut action = None;
        let mut active = active;
        // Header: format controls for `this`.
        ui.horizontal(|ui| {
            ui.label(RichText::new("CsvQuery").strong().size(12.5));
            match active.as_mut() {
                Some(a) => {
                    let shape = self.tables.get(a.table).map_or(String::new(), |t| {
                        format!(
                            " · {}",
                            t!(
                                "csv-shape",
                                rows = thousands(t.rows),
                                columns = t.columns.len()
                            )
                        )
                    });
                    ui.label(
                        RichText::new(format!("this = {}{shape}", a.table))
                            .size(12.0)
                            .color(palette.text_dim),
                    )
                    .on_hover_text(t!("csv-this-hover"));
                    format_controls(ui, a.detected, a.settings);
                }
                None => {
                    ui.label(
                        RichText::new(t!("csv-open-file"))
                            .size(12.0)
                            .color(palette.text_dim),
                    );
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, "×", &t!("close-csvquery")) {
                    self.open = false;
                }
                ui.menu_button(t!("tables"), |ui| {
                    ui.label(
                        RichText::new(t!("click-to-insert-name"))
                            .size(11.5)
                            .color(palette.text_dim),
                    );
                    for (name, title) in all_tables {
                        if ui.button(format!("{name}    ({title})")).clicked() {
                            action = Some(PanelAction::InsertTable(name.clone()));
                            ui.close();
                        }
                    }
                });
                ui.menu_button(t!("recent"), |ui| {
                    if self.history.is_empty() {
                        ui.label(t!("no-queries-yet"));
                    }
                    for h in self.history.clone() {
                        let short: String = h.chars().take(80).collect();
                        if ui
                            .button(short.replace('\n', " "))
                            .on_hover_text(&h)
                            .clicked()
                        {
                            self.sql = h;
                            action = Some(PanelAction::Run);
                            ui.close();
                        }
                    }
                });
            });
        });

        // SQL box and Run.
        ui.horizontal(|ui| {
            let run_w = 70.0;
            let edit = TextEdit::multiline(&mut self.sql)
                .id_salt("csvquery-sql")
                .font(FontId::monospace(13.0))
                .desired_rows(2)
                .hint_text(DEFAULT_SQL)
                .desired_width(ui.available_width() - run_w - 8.0);
            let response = ui.add(edit);
            if self.focus_sql {
                response.request_focus();
                self.focus_sql = false;
            }
            if response.has_focus()
                && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
            {
                action = Some(PanelAction::Run);
            }
            ui.vertical(|ui| {
                let run = ui
                    .add_sized(Vec2::new(run_w, 26.0), egui::Button::new(t!("run")))
                    .on_hover_text(t!("run-query-hover"));
                if run.clicked() {
                    action = Some(PanelAction::Run);
                }
            });
        });

        // Status line and result buttons.
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Results, t!("results"));
            ui.selectable_value(&mut self.view, View::Columns, t!("columns"));
            ui.separator();
            if self.running.is_some() {
                ui.spinner();
            }
            if let Some(e) = &self.error {
                let mut e = e.clone();
                if e.contains("no such table") {
                    let names: Vec<&str> = all_tables.iter().map(|t| t.0.as_str()).collect();
                    e.push_str(&format!(
                        ". {}",
                        t!("csv-tables-list", names = names.join(", "))
                    ));
                }
                ui.label(
                    RichText::new(e)
                        .size(12.0)
                        .color(Color32::from_rgb(230, 100, 100)),
                );
            } else if let Some(r) = &self.result {
                let n = r.rows.len();
                let text = if let Some(c) = r.changed {
                    t!("csv-rows-changed", count = c)
                } else if r.truncated {
                    t!(
                        "csv-first-rows",
                        rows = thousands(n),
                        ms = r.elapsed.as_millis() as u64
                    )
                } else {
                    t!(
                        "csv-rows",
                        count = n,
                        rows = thousands(n),
                        ms = r.elapsed.as_millis() as u64
                    )
                };
                ui.label(RichText::new(text).size(12.0).color(palette.text_dim));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let has = self.result.as_ref().is_some_and(|r| !r.columns.is_empty());
                if ui
                    .add_enabled(has, egui::Button::new(t!("open-in-new-tab")).small())
                    .on_hover_text(t!("open-in-new-tab-hover"))
                    .clicked()
                {
                    action = Some(PanelAction::ToNewTab);
                }
                if ui
                    .add_enabled(has, egui::Button::new(t!("copy")).small())
                    .on_hover_text(t!("copy-results-hover"))
                    .clicked()
                {
                    self.copy_results(ui.ctx());
                }
            });
        });
        ui.add_space(2.0);
        // The panel keeps its height when there are few or no rows.
        ui.set_min_height(ui.available_height());

        match self.view {
            View::Results => self.results_grid(ui),
            View::Columns => {
                let info = active
                    .as_ref()
                    .and_then(|a| self.tables.get(a.table).cloned());
                let settings = active.map(|a| a.settings);
                self.columns_view(ui, palette, info.as_ref(), settings)
            }
        }
        action
    }

    fn results_grid(&mut self, ui: &mut Ui) {
        let Some(r) = &self.result else {
            return;
        };
        if r.columns.is_empty() {
            return;
        }
        let font = FontId::proportional(12.5);
        let row_h = ui.fonts_mut(|f| f.row_height(&font)) + 6.0;
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0')).max(1.0);
        // Start each column wide enough for its name and first values.
        let widths: Vec<f32> = (0..r.columns.len())
            .map(|c| {
                let longest = r
                    .rows
                    .iter()
                    .take(200)
                    .map(|row| row[c].display().chars().count())
                    .max()
                    .unwrap_or(0)
                    .max(r.columns[c].chars().count() + 2);
                (longest as f32 * char_w + 20.0).clamp(50.0, 360.0)
            })
            .collect();
        let numeric: Vec<bool> = (0..r.columns.len())
            .map(|c| r.rows.iter().take(200).any(|row| row[c].is_number()))
            .collect();
        let mut clicked_header = None;
        let mut clicked_row = None;
        let mut copy = None;
        let selected = self.selected;
        let order = &self.order;
        let sort = self.sort;
        egui::ScrollArea::horizontal()
            .id_salt("csvquery-grid-h")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut table = TableBuilder::new(ui)
                    .id_salt("csvquery-grid")
                    .striped(true)
                    .resizable(true)
                    .sense(Sense::click())
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .auto_shrink([false, false])
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::INFINITY);
                for w in &widths {
                    table = table.column(Column::initial(*w).at_least(30.0).clip(true));
                }
                table
                    .header(row_h, |mut header| {
                        for (c, name) in r.columns.iter().enumerate() {
                            header.col(|ui| {
                                let arrow = match sort {
                                    Some((s, true)) if s == c => " ▲",
                                    Some((s, false)) if s == c => " ▼",
                                    _ => "",
                                };
                                let resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("{name}{arrow}")).strong().size(12.5),
                                    )
                                    .sense(Sense::click())
                                    .truncate(),
                                );
                                if resp.on_hover_text(t!("click-to-sort")).clicked() {
                                    clicked_header = Some(c);
                                }
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(row_h, order.len(), |mut row| {
                            let i = order[row.index()];
                            row.set_selected(selected == Some(i));
                            for (c, v) in r.rows[i].iter().enumerate() {
                                let (_, resp) = row.col(|ui| {
                                    let text = v.display();
                                    let rich = if matches!(v, Value::Null) {
                                        RichText::new("").size(12.5)
                                    } else {
                                        RichText::new(text).size(12.5)
                                    };
                                    if numeric[c] && v.is_number() {
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.add(
                                                    egui::Label::new(rich)
                                                        .truncate()
                                                        .selectable(false),
                                                );
                                            },
                                        );
                                    } else {
                                        ui.add(egui::Label::new(rich).truncate().selectable(false));
                                    }
                                });
                                resp.context_menu(|ui| {
                                    if ui.button(t!("copy-value")).clicked() {
                                        copy = Some(v.display());
                                        ui.close();
                                    }
                                    if ui.button(t!("copy-row")).clicked() {
                                        copy = Some(csv::write_tsv(
                                            &[],
                                            &[r.rows[i].iter().map(Value::display).collect()],
                                        ));
                                        ui.close();
                                    }
                                });
                            }
                            if row.response().clicked() {
                                clicked_row = Some(i);
                            }
                        });
                    });
            });
        if let Some(text) = copy {
            ui.ctx().copy_text(text.trim_end_matches('\n').to_owned());
        }
        if let Some(i) = clicked_row {
            self.selected = Some(i);
        }
        if let Some(c) = clicked_header {
            self.sort = match self.sort {
                Some((s, true)) if s == c => Some((c, false)),
                Some((s, false)) if s == c => None,
                _ => Some((c, true)),
            };
            self.sort_rows();
        }
    }

    /// Each column of `this`: its type (which can be changed), and how many
    /// values it has, how many are empty or distinct, and the smallest,
    /// largest and average value.
    fn columns_view(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        info: Option<&TableInfo>,
        settings: Option<&mut DocSettings>,
    ) {
        let (Some(info), Some(settings)) = (info, settings) else {
            ui.label(
                RichText::new(t!("csv-run-for-columns"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
            return;
        };
        settings.types.resize(info.columns.len(), None);
        let stats = self
            .stats
            .as_ref()
            .filter(|s| s.len() == info.columns.len());
        let font = FontId::proportional(12.5);
        let row_h = ui.fonts_mut(|f| f.row_height(&font)) + 8.0;
        let headers = [
            t!("csv-column"),
            t!("csv-type"),
            t!("csv-filled"),
            t!("csv-empty"),
            t!("csv-distinct"),
            t!("csv-min"),
            t!("csv-max"),
            t!("csv-average"),
        ];
        egui::ScrollArea::horizontal()
            .id_salt("csvquery-cols-h")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut table = TableBuilder::new(ui)
                    .id_salt("csvquery-cols")
                    .striped(true)
                    .resizable(true)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .auto_shrink([false, false])
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::INFINITY);
                for i in 0..headers.len() {
                    let w = match i {
                        0 => 180.0,
                        1 => 150.0,
                        5 | 6 => 160.0,
                        _ => 80.0,
                    };
                    table = table.column(Column::initial(w).at_least(40.0).clip(true));
                }
                table
                    .header(row_h, |mut header| {
                        for (i, h) in headers.iter().enumerate() {
                            header.col(|ui| {
                                let text = RichText::new(h).strong().size(12.5);
                                let r = ui.label(text);
                                if i == 1 {
                                    r.on_hover_text(t!("csv-type-hover"));
                                }
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(row_h, info.columns.len(), |mut row| {
                            let c = row.index();
                            row.col(|ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(&info.columns[c]).size(12.5))
                                        .truncate(),
                                );
                            });
                            row.col(|ui| {
                                let inferred =
                                    info.inferred.get(c).copied().unwrap_or(ColType::Text);
                                let label = |t: Option<ColType>| match t {
                                    None => t!("auto-with", detected = inferred.label()),
                                    Some(t) => t.label(),
                                };
                                egui::ComboBox::from_id_salt(("csvquery-type", c))
                                    .selected_text(label(settings.types[c]))
                                    .width(130.0)
                                    .show_ui(ui, |ui| {
                                        for t in [
                                            None,
                                            Some(ColType::Text),
                                            Some(ColType::Integer),
                                            Some(ColType::Real),
                                        ] {
                                            ui.selectable_value(
                                                &mut settings.types[c],
                                                t,
                                                label(t),
                                            );
                                        }
                                    });
                            });
                            let Some(s) = stats.map(|s| &s[c]) else {
                                return;
                            };
                            let cells = [
                                s.filled.to_string(),
                                s.empty.to_string(),
                                s.distinct.to_string(),
                                s.min.clone(),
                                s.max.clone(),
                                s.mean.map(sql::format_number).unwrap_or_default(),
                            ];
                            for (i, text) in cells.into_iter().enumerate() {
                                row.col(|ui| {
                                    let label =
                                        egui::Label::new(RichText::new(text).size(12.5)).truncate();
                                    if i <= 2 || i == 5 {
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.add(label);
                                            },
                                        );
                                    } else {
                                        ui.add(label);
                                    }
                                });
                            }
                        });
                    });
            });
    }
}

/// `1234567` → `1,234,567`.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Delimiter, quote, header and column-type choices for the current tab.
fn format_controls(ui: &mut Ui, detected: CsvFormat, settings: &mut DocSettings) {
    let current = settings.format(detected);
    let delim_text = |d: Option<u8>| match d {
        None => t!(
            "auto-with",
            detected = csv::delimiter_name(detected.delimiter)
        ),
        Some(d) => csv::delimiter_name(d),
    };
    egui::ComboBox::from_id_salt("csvquery-delimiter")
        .selected_text(delim_text(settings.delimiter))
        .width(130.0)
        .show_ui(ui, |ui| {
            for d in [
                None,
                Some(b','),
                Some(b';'),
                Some(b'\t'),
                Some(b'|'),
                Some(b' '),
            ] {
                ui.selectable_value(&mut settings.delimiter, d, delim_text(d));
            }
        })
        .response
        .on_hover_text(t!("delimiter"));
    let quote_text = |q: Option<Option<u8>>| match q {
        None => t!("auto-quotes"),
        Some(Some(c)) => t!("quotes-char", char = (c as char).to_string()),
        Some(None) => t!("no-quotes"),
    };
    egui::ComboBox::from_id_salt("csvquery-quote")
        .selected_text(quote_text(settings.quote))
        .width(100.0)
        .show_ui(ui, |ui| {
            for q in [None, Some(Some(b'"')), Some(Some(b'\'')), Some(None)] {
                ui.selectable_value(&mut settings.quote, q, quote_text(q));
            }
        })
        .response
        .on_hover_text(t!("quote-character"));
    let mut header = current.header;
    if ui
        .checkbox(&mut header, t!("header-row"))
        .on_hover_text(t!("header-row-hover"))
        .changed()
    {
        settings.header = (header != detected.header).then_some(header);
    }
    if settings.is_custom()
        && ui
            .small_button(t!("reset"))
            .on_hover_text(t!("reset-format-hover"))
            .clicked()
    {
        let detected = settings.detected;
        *settings = DocSettings {
            detected,
            ..DocSettings::default()
        };
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn thousands() {
        assert_eq!(super::thousands(0), "0");
        assert_eq!(super::thousands(999), "999");
        assert_eq!(super::thousands(100000), "100,000");
        assert_eq!(super::thousands(1234567), "1,234,567");
    }
}
