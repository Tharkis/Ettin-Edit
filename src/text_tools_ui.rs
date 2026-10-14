//! Panels and notices for the text tools: the JSON tree, the XPath panel,
//! and the short notice that reports a tool's result or error.

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{
    Align, Align2, CursorIcon, FontId, Frame, Key, Layout, Margin, Pos2, RichText, Sense,
    TextFormat, Ui, Vec2,
};

use crate::search_ui::icon_button;
use crate::theme::Palette;
use crate::tools::ToolError;
use crate::tools::json::{self, JsonDoc, Kind};

// ----------------------------------------------------------------------
// Notice

/// A short message about what a tool did, shown at the bottom right.
pub struct Notice {
    pub text: String,
    pub error: bool,
    /// When it was first shown (egui time), to hide it after a while.
    pub since: Option<f64>,
    /// A place in a document the message is about: (doc id, char offset).
    pub jump: Option<(u64, usize)>,
}

pub enum NoticeResponse {
    Close,
    Jump(u64, usize),
}

impl Notice {
    fn lifetime(&self) -> f64 {
        if self.error { 12.0 } else { 4.0 }
    }

    pub fn ui(&self, ctx: &egui::Context, palette: &Palette) -> Option<NoticeResponse> {
        let now = ctx.input(|i| i.time);
        let left = self.lifetime() - (now - self.since.unwrap_or(now));
        if left <= 0.0 {
            return Some(NoticeResponse::Close);
        }
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
        let mut response = None;
        egui::Area::new(egui::Id::new("tool-notice"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-16.0, -36.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::popup(ui.style())
                    .inner_margin(Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_max_width(460.0);
                        ui.horizontal(|ui| {
                            let (mark, color) = if self.error {
                                ("●", palette.modified)
                            } else {
                                ("✓", palette.accent)
                            };
                            ui.label(RichText::new(mark).color(color));
                            ui.add(egui::Label::new(&self.text).wrap());
                            if let Some((doc, pos)) = self.jump
                                && ui.small_button("Go to").clicked()
                            {
                                response = Some(NoticeResponse::Jump(doc, pos));
                            }
                            if ui
                                .add(egui::Button::new("×").frame(false))
                                .on_hover_text("Dismiss")
                                .clicked()
                            {
                                response = Some(NoticeResponse::Close);
                            }
                        });
                    });
            });
        response
    }
}

// ----------------------------------------------------------------------
// JSON tree

/// Documents bigger than this aren't shown as a tree.
const TREE_MAX_BYTES: usize = 32 << 20;
/// While typing, the tree is rebuilt at most this often (seconds).
const TREE_REBUILD_DELAY: f64 = 0.3;
const MAX_FILTER_ROWS: usize = 5000;

pub enum TreeAction {
    /// Select this byte range of the document text.
    Select(Range<usize>),
    Copy(String),
}

pub struct JsonTree {
    pub open: bool,
    /// The document and version the tree was built from.
    source: Option<(u64, u64)>,
    built_at: f64,
    text: Arc<String>,
    doc: Result<JsonDoc, ToolError>,
    expanded: HashSet<usize>,
    filter: String,
    /// Matches for the filter, for the filter and source they were found for.
    filtered: Option<(String, Vec<usize>, bool)>,
    selected: Option<usize>,
}

impl Default for JsonTree {
    fn default() -> Self {
        Self {
            open: false,
            source: None,
            built_at: f64::NEG_INFINITY,
            text: Arc::default(),
            doc: Err(ToolError::new("")),
            expanded: HashSet::new(),
            filter: String::new(),
            filtered: None,
            selected: None,
        }
    }
}

fn kind_color(kind: Kind, palette: &Palette) -> egui::Color32 {
    match kind {
        Kind::Object | Kind::Array => palette.text_dim,
        Kind::String => palette.editor_fg,
        _ => palette.accent,
    }
}

impl JsonTree {
    /// Rebuilds the tree if the document changed. Returns true if a rebuild
    /// is waiting (the caller should repaint soon).
    pub fn update(
        &mut self,
        doc_id: u64,
        version: u64,
        now: f64,
        text: impl FnOnce() -> Arc<String>,
    ) -> bool {
        if self.source == Some((doc_id, version)) {
            return false;
        }
        let same_doc = self.source.is_some_and(|(d, _)| d == doc_id);
        if same_doc && now - self.built_at < TREE_REBUILD_DELAY {
            return true;
        }
        self.text = text();
        self.built_at = now;
        self.source = Some((doc_id, version));
        self.filtered = None;
        self.doc = if self.text.len() > TREE_MAX_BYTES {
            Err(ToolError::new(
                "This document is too large to show as a tree.",
            ))
        } else {
            json::parse(&self.text)
        };
        if !same_doc {
            self.selected = None;
            self.expanded.clear();
            self.expanded.insert(0);
        }
        if let Ok(doc) = &self.doc {
            self.selected = self.selected.filter(|&s| s < doc.nodes.len());
        }
        false
    }

    /// Selects the innermost value at a byte offset and opens its parents.
    pub fn reveal(&mut self, offset: usize) {
        let Ok(doc) = &self.doc else { return };
        if let Some(id) = doc.node_at(offset) {
            let mut cur = doc.nodes[id].parent;
            while let Some(p) = cur {
                self.expanded.insert(p);
                cur = doc.nodes[p].parent;
            }
            self.selected = Some(id);
        }
    }

    fn select_range(doc: &JsonDoc, id: usize) -> Range<usize> {
        let n = &doc.nodes[id];
        n.key_span.as_ref().map_or(n.span.start, |k| k.start)..n.span.end
    }

    pub fn ui(&mut self, ui: &mut Ui, palette: &Palette) -> Option<TreeAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new("JSON").strong().size(12.5));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, "×", "Close the JSON tree") {
                    self.open = false;
                }
                if let Ok(doc) = &self.doc {
                    if ui.small_button("⊟").on_hover_text("Collapse all").clicked() {
                        self.expanded.clear();
                        self.expanded.insert(0);
                    }
                    if ui.small_button("⊞").on_hover_text("Expand all").clicked() {
                        self.expanded = (0..doc.nodes.len())
                            .filter(|&i| !doc.nodes[i].children.is_empty())
                            .collect();
                    }
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .id(egui::Id::new("json-tree-filter"))
                        .hint_text("Filter keys and values")
                        .desired_width(f32::INFINITY),
                );
            });
        });
        ui.separator();

        let doc = match &self.doc {
            Ok(doc) => doc,
            Err(e) => {
                if self.source.is_some() {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Not valid JSON").color(palette.modified));
                    if let Some(offset) = e.offset {
                        let (line, col) = crate::tools::line_col(&self.text, offset);
                        ui.add(
                            egui::Label::new(format!("Line {line}, column {col}: {}", e.message))
                                .wrap(),
                        );
                        if ui.button("Go to Error").clicked() {
                            action = Some(TreeAction::Select(offset..offset));
                        }
                    } else {
                        ui.add(egui::Label::new(&e.message).wrap());
                    }
                }
                return action;
            }
        };

        // The rows to show: matches for the filter, or the expanded tree.
        let filter = self.filter.trim().to_lowercase();
        let rows: Vec<(usize, usize)> = if filter.is_empty() {
            let mut rows = Vec::new();
            let mut stack = vec![(0usize, 0usize)];
            while let Some((id, depth)) = stack.pop() {
                rows.push((id, depth));
                if self.expanded.contains(&id) {
                    for &c in doc.nodes[id].children.iter().rev() {
                        stack.push((c, depth + 1));
                    }
                }
            }
            rows
        } else {
            if self.filtered.as_ref().is_none_or(|(f, _, _)| *f != filter) {
                let mut hits = Vec::new();
                let mut more = false;
                for id in 0..doc.nodes.len() {
                    let n = &doc.nodes[id];
                    let key_hit = n
                        .key
                        .as_ref()
                        .is_some_and(|k| k.to_lowercase().contains(&filter));
                    let value_hit = !matches!(n.kind, Kind::Object | Kind::Array)
                        && doc
                            .copy_value(id, &self.text)
                            .to_lowercase()
                            .contains(&filter);
                    if key_hit || value_hit {
                        if hits.len() == MAX_FILTER_ROWS {
                            more = true;
                            break;
                        }
                        hits.push(id);
                    }
                }
                self.filtered = Some((filter.clone(), hits, more));
            }
            let (_, hits, more) = self.filtered.as_ref().expect("filtered just set");
            if hits.is_empty() {
                ui.label(RichText::new("No matches").color(palette.text_dim));
            } else if *more {
                ui.label(
                    RichText::new(format!("First {MAX_FILTER_ROWS} matches"))
                        .size(11.5)
                        .color(palette.text_dim),
                );
            }
            hits.iter().map(|&id| (id, 0)).collect()
        };

        let font = FontId::monospace(12.5);
        let row_h = ui.fonts_mut(|f| f.row_height(&font)) + 4.0;
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, ' '));
        let filtering = !filter.is_empty();
        let mut toggle = None;
        egui::ScrollArea::both().auto_shrink(false).show_rows(
            ui,
            row_h,
            rows.len(),
            |ui, range| {
                for &(id, depth) in &rows[range] {
                    let node = &doc.nodes[id];
                    let container = !node.children.is_empty();
                    let mut job = LayoutJob::default();
                    let dim = TextFormat::simple(font.clone(), palette.text_dim);
                    let indent = "  ".repeat(depth);
                    let arrow = match (filtering, container, self.expanded.contains(&id)) {
                        (false, true, true) => "▾ ",
                        (false, true, false) => "▸ ",
                        _ => "  ",
                    };
                    job.append(&format!("{indent}{arrow}"), 0.0, dim.clone());
                    let label = if filtering {
                        doc.path(id)
                    } else {
                        match (&node.key, doc.index_in_parent(id)) {
                            (Some(k), _) => k.clone(),
                            (None, Some(i)) => format!("[{i}]"),
                            (None, None) => "$".to_owned(),
                        }
                    };
                    job.append(
                        &label,
                        0.0,
                        TextFormat::simple(font.clone(), palette.accent),
                    );
                    job.append(": ", 0.0, dim.clone());
                    job.append(
                        &doc.preview(id, &self.text),
                        0.0,
                        TextFormat::simple(font.clone(), kind_color(node.kind, palette)),
                    );
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
                    let width = (galley.size().x + 16.0).max(ui.available_width());
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, row_h), Sense::click());
                    if self.selected == Some(id) {
                        ui.painter().rect_filled(rect, 2.0, palette.selection);
                    } else if response.hovered() {
                        ui.painter().rect_filled(rect, 2.0, palette.line_highlight);
                    }
                    ui.painter().galley(
                        Pos2::new(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0),
                        galley,
                        palette.text,
                    );
                    let response = response.on_hover_cursor(CursorIcon::PointingHand);
                    // Clicking the arrow (or double-clicking) opens and closes.
                    let on_arrow = response.interact_pointer_pos().is_some_and(|p| {
                        p.x < rect.left() + 4.0 + (depth as f32 + 1.0) * 2.0 * char_w
                    });
                    if response.clicked() {
                        self.selected = Some(id);
                        if container && !filtering && on_arrow {
                            toggle = Some(id);
                        } else {
                            action = Some(TreeAction::Select(Self::select_range(doc, id)));
                        }
                    }
                    if response.double_clicked() && container && !filtering {
                        toggle = Some(id);
                    }
                    response.context_menu(|ui| {
                        if ui.button("Copy Path").clicked() {
                            action = Some(TreeAction::Copy(doc.path(id)));
                            ui.close();
                        }
                        if let Some(k) = &node.key
                            && ui.button("Copy Key").clicked()
                        {
                            action = Some(TreeAction::Copy(k.clone()));
                            ui.close();
                        }
                        if ui.button("Copy Value").clicked() {
                            action = Some(TreeAction::Copy(doc.copy_value(id, &self.text)));
                            ui.close();
                        }
                        if container && !filtering {
                            ui.separator();
                            if ui.button("Expand All Inside").clicked() {
                                let mut stack = vec![id];
                                while let Some(n) = stack.pop() {
                                    if !doc.nodes[n].children.is_empty() {
                                        self.expanded.insert(n);
                                        stack.extend(&doc.nodes[n].children);
                                    }
                                }
                                ui.close();
                            }
                        }
                    });
                }
            },
        );
        if let Some(id) = toggle
            && !self.expanded.remove(&id)
        {
            self.expanded.insert(id);
        }
        action
    }
}

// ----------------------------------------------------------------------
// XPath panel

/// A node the expression selected, placed in the document.
pub struct XPathRow {
    /// Char range in the document.
    pub range: Range<usize>,
    /// 0-based line.
    pub line: usize,
    pub label: String,
    /// The value on one line, shortened.
    pub value: String,
    /// The whole string value, for copying.
    pub full: String,
}

pub enum XPathAction {
    Evaluate,
    Select(Range<usize>),
    Copy(String),
}

#[derive(Default)]
pub struct XPathPanel {
    pub open: bool,
    pub expr: String,
    pub focus: bool,
    pub rows: Vec<XPathRow>,
    /// The value of an expression that doesn't select nodes.
    pub value: Option<String>,
    pub error: Option<String>,
    /// The document (id, version) the results came from.
    pub source: Option<(u64, u64)>,
    pub history: Vec<String>,
}

impl XPathPanel {
    pub fn show_panel(&mut self) {
        self.open = true;
        self.focus = true;
    }

    pub fn remember(&mut self) {
        let expr = self.expr.trim().to_owned();
        self.history.retain(|e| *e != expr);
        self.history.insert(0, expr);
        self.history.truncate(20);
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        current: Option<(u64, u64)>,
    ) -> Option<XPathAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new("XPath").strong().size(12.5));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, "×", "Close the XPath panel") {
                    self.open = false;
                }
                if !self.rows.is_empty() && ui.small_button("Copy Results").clicked() {
                    let text: Vec<String> = self.rows.iter().map(|r| r.full.clone()).collect();
                    action = Some(XPathAction::Copy(text.join("\n")));
                }
                if ui.button("Evaluate").clicked() {
                    action = Some(XPathAction::Evaluate);
                }
                if !self.history.is_empty() {
                    ui.menu_button("Recent", |ui| {
                        for e in self.history.clone() {
                            if ui.button(&e).clicked() {
                                self.expr = e;
                                action = Some(XPathAction::Evaluate);
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Recent expressions");
                }
                // A fixed id keeps focus when the buttons beside it come and go.
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.expr)
                        .id(egui::Id::new("xpath-expression"))
                        .hint_text("//book[price > 30]/title")
                        .font(FontId::monospace(13.0))
                        .desired_width(f32::INFINITY),
                );
                if std::mem::take(&mut self.focus) {
                    edit.request_focus();
                }
                if edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    action = Some(XPathAction::Evaluate);
                    edit.request_focus();
                }
            });
        });
        let stale = self.source.is_some() && self.source != current;
        if let Some(e) = &self.error {
            ui.label(RichText::new(e).color(palette.modified));
            return action;
        }
        if let Some(v) = &self.value {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Result:").color(palette.text_dim));
                ui.label(RichText::new(v).monospace());
                if ui.small_button("Copy").clicked() {
                    action = Some(XPathAction::Copy(v.clone()));
                }
            });
            return action;
        }
        if self.source.is_none() {
            return action;
        }
        ui.label(
            RichText::new(format!(
                "{} node{}{}",
                self.rows.len(),
                if self.rows.len() == 1 { "" } else { "s" },
                if stale {
                    " (the document changed since; evaluate again to refresh)"
                } else {
                    ""
                }
            ))
            .size(12.0)
            .color(palette.text_dim),
        );
        let font = FontId::monospace(12.5);
        let row_h = ui.fonts_mut(|f| f.row_height(&font)) + 4.0;
        egui::ScrollArea::both().auto_shrink(false).show_rows(
            ui,
            row_h,
            self.rows.len(),
            |ui, range| {
                for row in &self.rows[range] {
                    let mut job = LayoutJob::default();
                    job.append(
                        &format!("{:>6}  ", row.line + 1),
                        0.0,
                        TextFormat::simple(font.clone(), palette.gutter_fg),
                    );
                    job.append(
                        &row.label,
                        0.0,
                        TextFormat::simple(font.clone(), palette.accent),
                    );
                    job.append(
                        &format!("  {}", row.value),
                        0.0,
                        TextFormat::simple(font.clone(), palette.text),
                    );
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
                    let width = (galley.size().x + 16.0).max(ui.available_width());
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
                        action = Some(XPathAction::Select(row.range.clone()));
                    }
                    response.context_menu(|ui| {
                        if ui.button("Copy Value").clicked() {
                            action = Some(XPathAction::Copy(row.full.clone()));
                            ui.close();
                        }
                    });
                }
            },
        );
        action
    }
}
