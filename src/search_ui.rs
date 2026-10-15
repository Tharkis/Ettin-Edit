//! The search UI: the find/replace bar, the search results panel, and the
//! Go to Line and Find in Files dialogs. They only collect input and draw
//! results; the commands they trigger are [`Action`]s run by the app.

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::{Align, CursorIcon, FontId, Id, Key, Layout, Margin, Pos2, RichText, Sense, Ui, Vec2};

use crate::actions::Action;
use crate::search::{FileSearchEvent, Hit, Query, SearchOptions};
use crate::theme::Palette;

const FIND_FIELD: &str = "find-field";
const REPLACE_FIELD: &str = "replace-field";

/// Puts keyboard focus in a text field and selects all its text.
fn focus_and_select_all(ui: &Ui, response: &egui::Response, len: usize) {
    response.request_focus();
    if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), response.id) {
        state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(len))));
        state.store(ui.ctx(), response.id);
    }
}

/// A small on/off button for a search option.
fn option_toggle(ui: &mut Ui, label: &str, on: &mut bool, tip: &str) -> bool {
    let response = ui
        .add(
            egui::Button::new(RichText::new(label).monospace().size(12.5))
                .selected(*on)
                .min_size(Vec2::new(26.0, 22.0)),
        )
        .on_hover_text(tip);
    if response.clicked() {
        *on = !*on;
        return true;
    }
    false
}

pub fn icon_button(ui: &mut Ui, label: &str, tip: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(label).size(14.0)).min_size(Vec2::new(26.0, 22.0)))
        .on_hover_text(tip)
        .clicked()
}

// -------------------------------------------------------------------------
// Find / replace bar

#[derive(Default)]
pub struct FindBar {
    pub open: bool,
    pub replace_mode: bool,
    pub find: String,
    pub replace: String,
    pub in_selection: bool,
    /// Focus the find (or replace) field on the next frame.
    focus: Option<&'static str>,
    /// A note shown next to the count, such as "Wrapped around" or "7 replaced".
    pub message: Option<String>,
    /// The document state a replace message describes; the message clears once the text changes again.
    pub message_at: Option<(u64, u64)>,
    /// Where search-as-you-type starts from: the document and char offset.
    pub origin: Option<(u64, usize)>,
    /// What the highlights were computed for: (doc id, buffer version, text, options).
    hits_key: Option<(u64, u64, String, SearchOptions)>,
    /// Number of matches and whether there are more than were counted.
    pub count: Option<(usize, bool)>,
    /// Index of the selected match among them, if it is one.
    pub current: Option<usize>,
}

impl FindBar {
    /// Opens the bar. `seed` is the selected text to search for, if any.
    pub fn show_bar(&mut self, replace: bool, seed: Option<String>, multi_line_selection: bool) {
        self.open = true;
        self.replace_mode = replace;
        if let Some(seed) = seed {
            self.find = seed;
        }
        self.in_selection = multi_line_selection;
        self.focus = Some(FIND_FIELD);
        self.message = None;
        self.hits_key = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.message = None;
        self.hits_key = None;
        self.count = None;
    }

    /// Compiles the current search text. Errors are shown in the bar.
    pub fn query(&mut self, opts: &SearchOptions) -> Option<Query> {
        match Query::new(&self.find, opts) {
            Ok(q) => q,
            Err(e) => {
                self.message = Some(format!("Invalid regex: {e}"));
                None
            }
        }
    }

    /// True when the highlights need recomputing for this document state.
    pub fn hits_stale(&self, doc: u64, version: u64, opts: &SearchOptions) -> bool {
        self.hits_key
            .as_ref()
            .is_none_or(|(d, v, t, o)| *d != doc || *v != version || *t != self.find || o != opts)
    }

    pub fn set_hits_key(&mut self, doc: u64, version: u64, opts: SearchOptions) {
        self.hits_key = Some((doc, version, self.find.clone(), opts));
    }

    /// Draws the bar. Returns true when the search text or an option changed
    /// (so search-as-you-type should run).
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        opts: &mut SearchOptions,
        palette: &Palette,
        actions: &mut Vec<Action>,
    ) -> bool {
        let mut changed = false;
        let w = &mut ui.visuals_mut().widgets;
        w.inactive.bg_stroke = egui::Stroke::new(1.0, palette.border);
        w.inactive.bg_fill = palette.editor_bg;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let arrow = if self.replace_mode { "⏷" } else { "⏵" };
            if ui
                .add(
                    egui::Button::new(arrow)
                        .frame(false)
                        .min_size(Vec2::new(18.0, 22.0)),
                )
                .on_hover_text("Toggle replace")
                .clicked()
            {
                self.replace_mode = !self.replace_mode;
            }

            let error = self
                .message
                .as_deref()
                .is_some_and(|m| m.starts_with("Invalid"));
            let field = egui::TextEdit::singleline(&mut self.find)
                .id(Id::new(FIND_FIELD))
                .hint_text("Find")
                .desired_width(280.0)
                .margin(Margin::symmetric(6, 3))
                .text_color_opt(error.then_some(palette.modified));
            let response = ui.add(field);
            if self.focus == Some(FIND_FIELD) {
                focus_and_select_all(ui, &response, self.find.chars().count());
                self.focus = None;
            }
            if response.changed() {
                changed = true;
            }
            let (enter, shift, escape) = ui.input(|i| {
                (
                    i.key_pressed(Key::Enter),
                    i.modifiers.shift,
                    i.key_pressed(Key::Escape),
                )
            });
            if response.lost_focus() {
                if enter {
                    actions.push(if shift {
                        Action::FindPrev
                    } else {
                        Action::FindNext
                    });
                    response.request_focus();
                } else if escape {
                    actions.push(Action::CloseFindBar);
                }
            }

            changed |= option_toggle(ui, "Aa", &mut opts.match_case, "Match case");
            changed |= option_toggle(ui, "W", &mut opts.whole_word, "Whole word");
            changed |= option_toggle(ui, ".*", &mut opts.regex, "Regular expression");

            let count = match self.count {
                _ if self.find.is_empty() => None,
                Some((0, _)) => Some("No results".to_owned()),
                Some((n, more)) => {
                    let total = if more { format!("{n}+") } else { n.to_string() };
                    Some(match self.current {
                        Some(i) => format!("{} of {total}", i + 1),
                        None => format!("{total} found"),
                    })
                }
                None => None,
            };
            let message = self.message.clone();
            let bad = |m: &str| {
                m.starts_with("Invalid")
                    || matches!(m, "Not found" | "No results" | "No more matches")
            };
            let warn = message
                .as_deref()
                .map_or(matches!(self.count, Some((0, _))), bad);
            let status = match (count, message) {
                (_, Some(m)) if bad(&m) => m,
                (Some(c), Some(m)) if c != m => format!("{c}, {}", m.to_lowercase()),
                (c, m) => m.or(c).unwrap_or_default(),
            };
            ui.add_sized(
                [170.0, 22.0],
                egui::Label::new(RichText::new(status).size(12.0).color(if warn {
                    palette.modified
                } else {
                    palette.text_dim
                }))
                .truncate(),
            );

            if icon_button(ui, "↑", "Previous match (Shift+F3)") {
                actions.push(Action::FindPrev);
            }
            if icon_button(ui, "↓", "Next match (F3)") {
                actions.push(Action::FindNext);
            }
            ui.menu_button(RichText::new("…").size(14.0), |ui| {
                ui.checkbox(&mut opts.wrap, "Wrap around");
                ui.separator();
                for (label, action) in [
                    ("Find All in Current Document", Action::FindAllInDoc),
                    ("Find All in Open Tabs", Action::FindAllInTabs),
                    ("Select All Matches", Action::SelectAllMatches),
                ] {
                    if ui.button(label).clicked() {
                        actions.push(action);
                    }
                }
                ui.separator();
                if ui.button("Mark All").clicked() {
                    actions.push(Action::MarkAll { bookmark: false });
                }
                if ui.button("Mark All and Bookmark Lines").clicked() {
                    actions.push(Action::MarkAll { bookmark: true });
                }
                if ui.button("Clear All Marks").clicked() {
                    actions.push(Action::ClearMarks);
                }
                ui.separator();
                if ui.button("Find in Files…").clicked() {
                    actions.push(Action::FindInFiles);
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, "×", "Close (Esc)") {
                    actions.push(Action::CloseFindBar);
                }
            });
        });

        if self.replace_mode {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(22.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.replace)
                        .id(Id::new(REPLACE_FIELD))
                        .hint_text(if opts.regex {
                            r"Replace  ($1 or \1 for groups)"
                        } else {
                            "Replace"
                        })
                        .desired_width(280.0)
                        .margin(Margin::symmetric(6, 3)),
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    actions.push(Action::ReplaceOne);
                    response.request_focus();
                } else if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
                    actions.push(Action::CloseFindBar);
                }
                if ui
                    .button("Replace")
                    .on_hover_text("Replace this match and find the next")
                    .clicked()
                {
                    actions.push(Action::ReplaceOne);
                }
                if ui.button("Replace All").clicked() {
                    actions.push(Action::ReplaceAll);
                }
                ui.checkbox(&mut self.in_selection, "In selection");
            });
        }
        changed
    }
}

// -------------------------------------------------------------------------
// Results panel

#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// An open document, by id.
    Doc(u64),
    File(PathBuf),
}

pub struct ResultGroup {
    pub source: Source,
    pub label: String,
    pub hits: Vec<Hit>,
    pub collapsed: bool,
}

/// A running Find in Files search.
pub struct FileJob {
    pub rx: Receiver<FileSearchEvent>,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct Results {
    pub open: bool,
    /// What was searched for, and where.
    pub title: String,
    pub groups: Vec<ResultGroup>,
    /// Shown after the counts, e.g. "searched 120 files" or "stopped".
    pub note: String,
    pub truncated: bool,
    /// What the rows are called, when they aren't search hits (e.g. "problem").
    pub unit: Option<&'static str>,
    pub job: Option<FileJob>,
    /// Flattened (group, hit) rows; rebuilt when groups change or collapse.
    rows: Vec<(usize, Option<usize>)>,
    rows_dirty: bool,
}

/// Where a click in the results panel wants to go.
pub struct Jump {
    pub source: Source,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

impl Results {
    pub fn start(&mut self, title: String) {
        self.stop();
        self.open = true;
        self.title = title;
        self.groups.clear();
        self.note.clear();
        self.truncated = false;
        self.unit = None;
        self.rows_dirty = true;
    }

    pub fn add_group(&mut self, source: Source, label: String, hits: Vec<Hit>) {
        self.groups.push(ResultGroup {
            source,
            label,
            hits,
            collapsed: false,
        });
        self.rows_dirty = true;
    }

    pub fn stop(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel.store(true, Ordering::Relaxed);
            self.note = "stopped".to_owned();
        }
    }

    /// Takes in what the background search has found so far. Returns true while it runs.
    pub fn poll(&mut self) -> bool {
        let Some(job) = &self.job else { return false };
        let mut finished = None;
        let mut new_groups = Vec::new();
        for event in job.rx.try_iter() {
            match event {
                FileSearchEvent::File { path, hits } => new_groups.push((path, hits)),
                FileSearchEvent::Done {
                    files_searched,
                    truncated,
                } => finished = Some((files_searched, truncated)),
            }
        }
        for (path, hits) in new_groups {
            let label = path.display().to_string();
            self.add_group(Source::File(path), label, hits);
        }
        if let Some((files, truncated)) = finished {
            self.job = None;
            self.truncated = truncated;
            self.note = format!("searched {files} file{}", if files == 1 { "" } else { "s" });
            return false;
        }
        true
    }

    fn total_hits(&self) -> usize {
        self.groups
            .iter()
            .flat_map(|g| &g.hits)
            .map(|h| h.matches)
            .sum()
    }

    fn rebuild_rows(&mut self) {
        self.rows.clear();
        for (gi, g) in self.groups.iter().enumerate() {
            self.rows.push((gi, None));
            if !g.collapsed {
                self.rows.extend((0..g.hits.len()).map(|hi| (gi, Some(hi))));
            }
        }
        self.rows_dirty = false;
    }

    pub fn ui(&mut self, ui: &mut Ui, palette: &Palette) -> Option<Jump> {
        if self.rows_dirty {
            self.rebuild_rows();
        }
        let mut jump = None;
        ui.horizontal(|ui| {
            let hits = self.total_hits();
            let groups = self.groups.len();
            let unit = self.unit.unwrap_or("hit");
            let mut summary = format!(
                "{hits}{} {unit}{} in {groups} {}",
                if self.truncated { "+" } else { "" },
                if hits == 1 { "" } else { "s" },
                if self
                    .groups
                    .first()
                    .is_some_and(|g| matches!(g.source, Source::File(_)))
                    || self.job.is_some()
                {
                    if groups == 1 { "file" } else { "files" }
                } else if groups == 1 {
                    "document"
                } else {
                    "documents"
                }
            );
            if !self.note.is_empty() {
                summary += &format!(" ({})", self.note);
            }
            ui.label(RichText::new(&self.title).strong().size(12.5));
            ui.label(RichText::new(summary).size(12.0).color(palette.text_dim));
            if self.job.is_some() {
                ui.spinner();
                if ui.small_button("Stop").clicked() {
                    self.stop();
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, "×", "Close results") {
                    self.stop();
                    self.open = false;
                }
                if ui.small_button("Collapse All").clicked() {
                    self.groups.iter_mut().for_each(|g| g.collapsed = true);
                    self.rows_dirty = true;
                }
                if ui.small_button("Expand All").clicked() {
                    self.groups.iter_mut().for_each(|g| g.collapsed = false);
                    self.rows_dirty = true;
                }
            });
        });
        ui.separator();

        let font = FontId::monospace(12.5);
        let row_h = ui.fonts_mut(|f| f.row_height(&font)) + 4.0;
        let mut toggle = None;
        egui::ScrollArea::both().auto_shrink(false).show_rows(
            ui,
            row_h,
            self.rows.len(),
            |ui, range| {
                for &(gi, hit) in &self.rows[range] {
                    let group = &self.groups[gi];
                    let job = match hit {
                        None => {
                            let mut job = LayoutJob::default();
                            let arrow = if group.collapsed { "▸ " } else { "▾ " };
                            job.append(
                                arrow,
                                0.0,
                                TextFormat::simple(font.clone(), palette.text_dim),
                            );
                            job.append(
                                &group.label,
                                0.0,
                                TextFormat::simple(font.clone(), palette.accent),
                            );
                            job.append(
                                &format!(
                                    "  ({})",
                                    group.hits.iter().map(|h| h.matches).sum::<usize>()
                                ),
                                0.0,
                                TextFormat::simple(font.clone(), palette.text_dim),
                            );
                            job
                        }
                        Some(hi) => hit_job(&group.hits[hi], &font, palette),
                    };
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
                    let width = (galley.size().x + 24.0).max(ui.available_width());
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, row_h), Sense::click());
                    if response.hovered() {
                        ui.painter().rect_filled(rect, 2.0, palette.line_highlight);
                    }
                    ui.painter().galley(
                        Pos2::new(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0),
                        galley,
                        palette.text,
                    );
                    let response = response.on_hover_cursor(CursorIcon::PointingHand);
                    if response.clicked() {
                        match hit {
                            None => toggle = Some(gi),
                            Some(hi) => {
                                let h = &group.hits[hi];
                                jump = Some(Jump {
                                    source: group.source.clone(),
                                    line: h.line,
                                    col: h.col,
                                    len: h.len,
                                });
                            }
                        }
                    }
                }
            },
        );
        if let Some(gi) = toggle {
            self.groups[gi].collapsed = !self.groups[gi].collapsed;
            self.rows_dirty = true;
        }
        jump
    }
}

/// "  123  text with the matches highlighted"
fn hit_job(hit: &Hit, font: &FontId, palette: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        &format!("    {:>6}  ", hit.line + 1),
        0.0,
        TextFormat::simple(font.clone(), palette.gutter_fg),
    );
    let plain = TextFormat::simple(font.clone(), palette.text);
    let mut marked = plain.clone();
    marked.background = palette.find_hit;
    let mut at = 0;
    for r in &hit.highlights {
        let Range { start, end } = r.clone();
        if start < at || end > hit.text.len() {
            continue;
        }
        job.append(
            &hit.text[at..start].replace('\t', "    "),
            0.0,
            plain.clone(),
        );
        job.append(
            &hit.text[start..end].replace('\t', "    "),
            0.0,
            marked.clone(),
        );
        at = end;
    }
    job.append(&hit.text[at..].replace('\t', "    "), 0.0, plain);
    job
}

// -------------------------------------------------------------------------
// Go to Line

#[derive(Default)]
pub struct GotoDialog {
    pub open: bool,
    text: String,
    error: bool,
    focus: bool,
}

impl GotoDialog {
    pub fn show_dialog(&mut self) {
        self.open = true;
        self.text.clear();
        self.error = false;
        self.focus = true;
    }

    /// Returns the 0-based line and column to go to once confirmed.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        lines: usize,
        palette: &Palette,
    ) -> Option<(usize, usize)> {
        if !self.open {
            return None;
        }
        let mut result = None;
        let mut close = false;
        let response = egui::Modal::new(Id::new("goto-line")).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.heading("Go to Line");
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("Line number (1–{lines}), or line:column"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.text)
                    .desired_width(f32::INFINITY)
                    .margin(Margin::symmetric(6, 4)),
            );
            if self.focus {
                field.request_focus();
                self.focus = false;
            }
            if field.changed() {
                self.error = false;
            }
            if self.error {
                ui.label(
                    RichText::new("Enter a line number")
                        .color(palette.modified)
                        .size(12.0),
                );
            }
            let enter = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 26.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button("Go").clicked() || enter {
                        match parse_line_col(&self.text) {
                            Some((l, c)) => {
                                result = Some((l.saturating_sub(1), c.saturating_sub(1)));
                                close = true;
                            }
                            None => {
                                self.error = true;
                                self.focus = true;
                            }
                        }
                    }
                },
            );
        });
        if close || response.should_close() {
            self.open = false;
        }
        result
    }
}

/// "12" or "12:5" (also "12,5" or "12 5").
fn parse_line_col(s: &str) -> Option<(usize, usize)> {
    let mut parts = s
        .split(|c: char| c == ':' || c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty());
    let line = parts.next()?.parse().ok()?;
    let col = match parts.next() {
        Some(c) => c.parse().ok()?,
        None => 1,
    };
    parts.next().is_none().then_some((line, col))
}

// -------------------------------------------------------------------------
// Find in Files

pub struct FifRequest {
    pub find: String,
    pub dir: PathBuf,
    pub filters: String,
    pub subfolders: bool,
    pub hidden: bool,
    pub opts: SearchOptions,
}

#[derive(Default)]
pub struct FifDialog {
    pub open: bool,
    pub find: String,
    pub dir: String,
    pub filters: String,
    pub subfolders: bool,
    pub hidden: bool,
    error: Option<String>,
    focus: bool,
}

impl FifDialog {
    pub fn show_dialog(&mut self, seed: Option<String>, dir: Option<PathBuf>) {
        self.open = true;
        if let Some(s) = seed {
            self.find = s;
        }
        if let Some(d) = dir {
            self.dir = d.display().to_string();
        }
        self.error = None;
        self.focus = true;
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        opts: &mut SearchOptions,
        palette: &Palette,
    ) -> Option<FifRequest> {
        if !self.open {
            return None;
        }
        let mut request = None;
        let mut close = false;
        let response = egui::Modal::new(Id::new("find-in-files")).show(ctx, |ui| {
            ui.set_width(520.0);
            ui.heading("Find in Files");
            ui.add_space(8.0);
            let mut enter = false;
            egui::Grid::new("fif-grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Find what");
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.find)
                            .desired_width(f32::INFINITY)
                            .margin(Margin::symmetric(6, 4)),
                    );
                    if self.focus {
                        focus_and_select_all(ui, &field, self.find.chars().count());
                        self.focus = false;
                    }
                    enter |= field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    ui.end_row();

                    ui.label("Folder");
                    ui.horizontal(|ui| {
                        let browse_w = 80.0;
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut self.dir)
                                .desired_width(ui.available_width() - browse_w)
                                .margin(Margin::symmetric(6, 4)),
                        );
                        enter |= field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                        if ui.button("Browse…").clicked() {
                            let mut dialog = rfd::FileDialog::new().set_title("Search in folder");
                            if !self.dir.is_empty() {
                                dialog = dialog.set_directory(&self.dir);
                            }
                            if let Some(d) = dialog.pick_folder() {
                                self.dir = d.display().to_string();
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Files");
                    let field = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.filters)
                                .hint_text("*.rs *.toml   (empty for all files, !name to skip)")
                                .desired_width(f32::INFINITY)
                                .margin(Margin::symmetric(6, 4)),
                        )
                        .on_hover_text("File name patterns separated by spaces or semicolons. Start one with ! to leave those files out.");
                    enter |= field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    ui.end_row();

                    ui.label("");
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut opts.match_case, "Match case");
                        ui.checkbox(&mut opts.whole_word, "Whole word");
                        ui.checkbox(&mut opts.regex, "Regular expression");
                    });
                    ui.end_row();
                    ui.label("");
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.subfolders, "Include subfolders");
                        ui.checkbox(&mut self.hidden, "Include hidden files and folders");
                    });
                    ui.end_row();
                });
            if let Some(e) = &self.error {
                ui.add_space(4.0);
                ui.label(RichText::new(e).color(palette.modified).size(12.0));
            }
            ui.add_space(10.0);
            ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 26.0), Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                let go = ui.add(
                    egui::Button::new("Find All").fill(palette.accent.gamma_multiply(0.6)),
                );
                if go.clicked() || enter {
                    let dir = PathBuf::from(self.dir.trim());
                    if self.find.is_empty() {
                        self.error = Some("Enter something to search for.".into());
                    } else if !dir.is_dir() {
                        self.error = Some("That folder doesn't exist.".into());
                    } else if let Err(e) = Query::new(&self.find, opts) {
                        self.error = Some(format!("Invalid regex: {e}"));
                    } else {
                        request = Some(FifRequest {
                            find: self.find.clone(),
                            dir,
                            filters: self.filters.clone(),
                            subfolders: self.subfolders,
                            hidden: self.hidden,
                            opts: *opts,
                        });
                        close = true;
                    }
                }
            });
        });
        if close || response.should_close() {
            self.open = false;
        }
        request
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_line_and_column() {
        assert_eq!(parse_line_col("12"), Some((12, 1)));
        assert_eq!(parse_line_col(" 12:5 "), Some((12, 5)));
        assert_eq!(parse_line_col("12,5"), Some((12, 5)));
        assert_eq!(parse_line_col("x"), None);
        assert_eq!(parse_line_col("1:2:3"), None);
        assert_eq!(parse_line_col(""), None);
    }
}
