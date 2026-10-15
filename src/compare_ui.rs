//! The side-by-side compare view: two texts aligned row by row, with
//! differences coloured, the changed words inside changed lines marked, an
//! overview bar of where the differences are, and difference navigation.

use std::collections::HashMap;
use std::ops::Range;

use egui::{
    Align, Align2, Color32, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, Ui,
    Vec2,
};
use unicode_width::UnicodeWidthChar;

use crate::compare::{self, CompareOptions, Comparison, Row, RowKind};
use crate::search_ui::icon_button;
use crate::theme::Palette;

/// Where one side's text comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// An open tab; the comparison follows its edits.
    Doc(u64),
    /// Fixed text: a saved file, the clipboard or the last commit.
    Text,
}

pub struct Side {
    pub source: Source,
    pub title: String,
    pub lines: Vec<String>,
    /// For a tab, the buffer version `lines` were taken at.
    pub version: u64,
    /// Line number (0-based) of the first line, for a part of a file.
    pub first_line: usize,
}

impl Side {
    pub fn new(source: Source, title: String, text: &str, version: u64) -> Self {
        Self {
            source,
            title,
            lines: compare::split_lines(text),
            version,
            first_line: 0,
        }
    }
}

/// The changed char ranges of a changed line: (left, right).
type InlineChanges = (Vec<Range<usize>>, Vec<Range<usize>>);

pub enum CompareAction {
    /// Open this tab at this line (0-based).
    Jump {
        doc: u64,
        line: usize,
    },
    Swap,
    ExportPatch,
    Close,
}

pub struct CompareView {
    pub left: Side,
    pub right: Side,
    pub opts: CompareOptions,
    pub result: Comparison,
    /// Changed words of `Changed` rows, worked out as rows come into view.
    inline: HashMap<usize, InlineChanges>,
    /// The difference last moved to.
    pub current: Option<usize>,
    /// Scroll so this row is in view on the next frame.
    scroll_to: Option<usize>,
    /// How far both sides are scrolled to the right, in points.
    h_scroll: f32,
    /// Shown instead of the editors. Hidden while a tab is being edited.
    pub shown: bool,
    /// When a tab being compared changed, to compare again shortly after.
    pub stale_since: Option<f64>,
    /// Height of the rows area last frame.
    view_h: f32,
    /// The sides were swapped: the diff still runs in the original
    /// direction and is mirrored, so swapping only flips what is shown.
    swapped: bool,
}

/// Colours of the differences, readable on dark and light themes.
struct DiffColors {
    added: Color32,
    removed: Color32,
    changed: Color32,
    moved: Color32,
    added_words: Color32,
    removed_words: Color32,
    filler: Color32,
}

impl DiffColors {
    fn new(palette: &Palette) -> Self {
        let a = if palette.dark { 46 } else { 60 };
        let w = if palette.dark { 110 } else { 120 };
        Self {
            added: Color32::from_rgba_unmultiplied(70, 190, 110, a),
            removed: Color32::from_rgba_unmultiplied(225, 85, 85, a),
            changed: Color32::from_rgba_unmultiplied(80, 145, 235, a),
            moved: Color32::from_rgba_unmultiplied(170, 115, 235, a),
            added_words: Color32::from_rgba_unmultiplied(70, 190, 110, w),
            removed_words: Color32::from_rgba_unmultiplied(225, 85, 85, w),
            filler: palette.chrome_bg.gamma_multiply(0.7),
        }
    }

    fn row(&self, row: &Row) -> Option<Color32> {
        match (row.kind, row.moved) {
            (RowKind::Added | RowKind::Removed, true) => Some(self.moved),
            (RowKind::Added, _) => Some(self.added),
            (RowKind::Removed, _) => Some(self.removed),
            (RowKind::Changed, _) => Some(self.changed),
            _ => None,
        }
    }
}

/// Longest line drawn; the rest of a very long line is cut off.
const MAX_LINE_CHARS: usize = 4000;
const OVERVIEW_W: f32 = 14.0;

impl CompareView {
    pub fn new(left: Side, right: Side, opts: CompareOptions) -> Self {
        let mut view = Self {
            left,
            right,
            opts,
            result: Comparison::default(),
            inline: HashMap::new(),
            current: None,
            scroll_to: None,
            h_scroll: 0.0,
            shown: true,
            stale_since: None,
            view_h: 400.0,
            swapped: false,
        };
        view.recompute();
        if !view.result.hunks.is_empty() {
            view.go_to(0);
        }
        view
    }

    /// Compares the two sides again (after an edit or an option change).
    pub fn recompute(&mut self) {
        self.result = if self.swapped {
            compare::compare(&self.right.lines, &self.left.lines, &self.opts).mirrored()
        } else {
            compare::compare(&self.left.lines, &self.right.lines, &self.opts)
        };
        self.inline.clear();
        self.stale_since = None;
        if let Some(c) = self.current
            && c >= self.result.hunks.len()
        {
            self.current = self.result.hunks.len().checked_sub(1);
        }
    }

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.left, &mut self.right);
        self.swapped = !self.swapped;
        self.result = std::mem::take(&mut self.result).mirrored();
        self.inline.clear();
    }

    /// Moves to difference `i`.
    pub fn go_to(&mut self, i: usize) {
        if let Some(h) = self.result.hunks.get(i) {
            self.current = Some(i);
            self.scroll_to = Some(h.start);
        }
    }

    /// Moves to the next (`forward`) or previous difference.
    pub fn step(&mut self, forward: bool) {
        let n = self.result.hunks.len();
        if n == 0 {
            return;
        }
        let next = match (self.current, forward) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(c), true) => (c + 1).min(n - 1),
            (Some(c), false) => c.saturating_sub(1),
        };
        self.go_to(next);
    }

    pub fn summary_text(&self) -> String {
        let s = self.result.summary;
        if s.is_empty() {
            return t!("no-differences");
        }
        let mut parts = Vec::new();
        for (n, id) in [
            (s.changed, "lines-changed"),
            (s.added, "lines-added"),
            (s.removed, "lines-removed"),
            (s.moved, "lines-moved"),
        ] {
            if n > 0 {
                let mut args = crate::i18n::Args::new();
                args.set("count", n);
                parts.push(crate::i18n::tr_args(id, &args));
            }
        }
        t!("compare-lines-summary", parts = parts.join(", "))
    }

    pub fn ui(&mut self, ui: &mut Ui, palette: &Palette, font_size: f32) -> Option<CompareAction> {
        let mut action = None;
        let colors = DiffColors::new(palette);
        // Toolbar
        egui::Frame::new()
            .fill(palette.chrome_bg)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let n = self.result.hunks.len();
                    ui.add_enabled_ui(n > 0, |ui| {
                        if ui
                            .button(t!("first"))
                            .on_hover_text(t!("first-difference"))
                            .clicked()
                        {
                            self.go_to(0);
                        }
                        if icon_button(ui, "↑", &t!("previous-difference-hover")) {
                            self.step(false);
                        }
                        if icon_button(ui, "↓", &t!("next-difference-hover")) {
                            self.step(true);
                        }
                        if ui
                            .button(t!("last"))
                            .on_hover_text(t!("last-difference"))
                            .clicked()
                        {
                            self.go_to(n.saturating_sub(1));
                        }
                    });
                    let position = match (self.current, n) {
                        (_, 0) => t!("identical"),
                        (Some(c), _) => t!("difference-of", current = c + 1, total = n),
                        (None, _) => t!("differences-count", count = n),
                    };
                    ui.label(RichText::new(position).color(palette.text));
                    ui.label(RichText::new(self.summary_text()).color(palette.text_dim));
                    if self.result.timed_out {
                        ui.label(RichText::new(t!("compare-approximate")).color(palette.text_dim));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, "×", &t!("close-comparison-hover")) {
                            action = Some(CompareAction::Close);
                        }
                        if ui
                            .button(t!("export-patch"))
                            .on_hover_text(t!("export-patch-hover"))
                            .clicked()
                        {
                            action = Some(CompareAction::ExportPatch);
                        }
                        if ui.button(t!("swap-sides")).clicked() {
                            action = Some(CompareAction::Swap);
                        }
                        ui.menu_button(t!("options"), |ui| {
                            let before = self.opts;
                            ui.checkbox(
                                &mut self.opts.ignore_whitespace,
                                t!("compare-ignore-whitespace"),
                            );
                            ui.checkbox(&mut self.opts.ignore_case, t!("compare-ignore-case"));
                            ui.checkbox(
                                &mut self.opts.ignore_empty_lines,
                                t!("compare-ignore-empty-lines"),
                            );
                            ui.checkbox(&mut self.opts.detect_moves, t!("compare-detect-moves"));
                            if self.opts != before {
                                self.recompute();
                            }
                        });
                    });
                });
            });

        let font = FontId::monospace(font_size);
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0')).max(1.0);
        let row_h = ui.fonts_mut(|f| f.row_height(&font)).ceil() + 2.0;
        let digits = (self.left.lines.len() + self.left.first_line)
            .max(self.right.lines.len() + self.right.first_line)
            .max(99)
            .to_string()
            .len() as f32;
        let gutter_w = digits * char_w + 14.0;

        // Titles
        let full = ui.available_rect_before_wrap();
        let body_w = full.width() - OVERVIEW_W;
        let col_w = (body_w / 2.0).floor();
        let title_h = 24.0;
        let (title_rect, _) =
            ui.allocate_exact_size(Vec2::new(full.width(), title_h), Sense::hover());
        for (k, side) in [&self.left, &self.right].into_iter().enumerate() {
            let x = title_rect.left() + k as f32 * col_w + 10.0;
            let lines = side.lines.len();
            let text = format!("{}   {}", side.title, t!("line-count", count = lines));
            ui.painter()
                .with_clip_rect(Rect::from_min_size(
                    Pos2::new(title_rect.left() + k as f32 * col_w, title_rect.top()),
                    Vec2::new(col_w, title_h),
                ))
                .text(
                    Pos2::new(x, title_rect.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(13.0),
                    palette.text,
                );
        }
        ui.painter().line_segment(
            [title_rect.left_bottom(), title_rect.right_bottom()],
            Stroke::new(1.0, palette.border),
        );

        // Rows
        let rows_rect = ui.available_rect_before_wrap();
        let n_rows = self.result.rows.len();
        let mut scroll = ScrollArea::vertical()
            .id_salt("compare-rows")
            .auto_shrink([false, false]);
        if let Some(row) = self.scroll_to.take() {
            let target = (row as f32 * row_h - self.view_h / 3.0).max(0.0);
            scroll = scroll.vertical_scroll_offset(target);
        }
        let mut visible = 0..0;
        let mut jump = None;
        let mut max_cols = 0usize;
        let output = scroll.show_rows(ui, row_h, n_rows, |ui, range| {
            visible = range.clone();
            for i in range {
                let row = self.result.rows[i];
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(body_w.max(0.0), row_h), Sense::click());
                if row.kind == RowKind::Changed && !self.inline.contains_key(&i) {
                    let (l, r) = (row.left.unwrap_or(0), row.right.unwrap_or(0));
                    let changes = compare::inline_changes(
                        &self.left.lines[l],
                        &self.right.lines[r],
                        &self.opts,
                    );
                    self.inline.insert(i, changes);
                }
                let inline = self.inline.get(&i);
                let current = self
                    .current
                    .and_then(|c| self.result.hunks.get(c))
                    .is_some_and(|h| h.contains(&i));
                for (k, (line, side)) in [(row.left, &self.left), (row.right, &self.right)]
                    .into_iter()
                    .enumerate()
                {
                    let col = Rect::from_min_size(
                        Pos2::new(rect.left() + k as f32 * col_w, rect.top()),
                        Vec2::new(col_w - if k == 0 { 1.0 } else { 0.0 }, row_h),
                    );
                    let painter = ui.painter().with_clip_rect(col.intersect(ui.clip_rect()));
                    let Some(line) = line else {
                        // No line on this side: a filler row keeps both sides lined up.
                        painter.rect_filled(col, 0.0, colors.filler);
                        continue;
                    };
                    if let Some(bg) = colors.row(&row) {
                        painter.rect_filled(col, 0.0, bg);
                    }
                    if current {
                        painter.rect_filled(
                            Rect::from_min_size(col.min, Vec2::new(3.0, row_h)),
                            0.0,
                            palette.accent,
                        );
                    }
                    painter.text(
                        Pos2::new(col.left() + gutter_w - 8.0, col.center().y),
                        Align2::RIGHT_CENTER,
                        (side.first_line + line + 1).to_string(),
                        font.clone(),
                        palette.gutter_fg,
                    );
                    let text_x = col.left() + gutter_w - self.h_scroll;
                    let text_clip = Rect::from_min_max(
                        Pos2::new(col.left() + gutter_w - 2.0, col.top()),
                        col.max,
                    )
                    .intersect(ui.clip_rect());
                    let painter = painter.with_clip_rect(text_clip);
                    let (display, cols) = display_line(&side.lines[line]);
                    max_cols = max_cols.max(*cols.last().unwrap_or(&0));
                    if let Some((l, r)) = inline {
                        let (ranges, color) = if k == 0 {
                            (l, colors.removed_words)
                        } else {
                            (r, colors.added_words)
                        };
                        for range in ranges {
                            let a = cols[range.start.min(cols.len() - 1)];
                            let b = cols[range.end.min(cols.len() - 1)];
                            painter.rect_filled(
                                Rect::from_min_max(
                                    Pos2::new(text_x + a as f32 * char_w, col.top() + 1.0),
                                    Pos2::new(text_x + b as f32 * char_w, col.bottom() - 1.0),
                                ),
                                2.0,
                                color,
                            );
                        }
                    }
                    painter.text(
                        Pos2::new(text_x, col.center().y),
                        Align2::LEFT_CENTER,
                        display,
                        font.clone(),
                        palette.editor_fg,
                    );
                }
                // A double-click opens the line in its tab.
                if response.double_clicked()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    let k = usize::from(pos.x >= rect.left() + col_w);
                    let (line, side) = if k == 0 {
                        (row.left, &self.left)
                    } else {
                        (row.right, &self.right)
                    };
                    if let (Some(line), Source::Doc(doc)) = (line, &side.source) {
                        jump = Some(CompareAction::Jump { doc: *doc, line });
                    }
                }
            }
        });
        self.view_h = output.inner_rect.height();
        if jump.is_some() {
            action = jump;
        }
        // The divider between the sides.
        let mid = (rows_rect.left() + col_w).round() - 0.5;
        ui.painter().line_segment(
            [
                Pos2::new(mid, rows_rect.top()),
                Pos2::new(mid, rows_rect.bottom()),
            ],
            Stroke::new(1.0, palette.border),
        );

        // Shift + wheel (or a sideways swipe) scrolls both sides sideways.
        if ui.rect_contains_pointer(rows_rect) {
            let dx = ui.input(|i| i.smooth_scroll_delta.x);
            let text_w = col_w - gutter_w - 8.0;
            let max = (max_cols as f32 * char_w - text_w + 4.0 * char_w).max(0.0);
            self.h_scroll = (self.h_scroll - dx).clamp(0.0, max);
        }

        // Overview bar: where the differences are, and which rows are in view.
        let bar = Rect::from_min_max(
            Pos2::new(rows_rect.right() - OVERVIEW_W, rows_rect.top()),
            rows_rect.max,
        );
        let painter = ui.painter_at(bar);
        painter.rect_filled(bar, 0.0, palette.chrome_bg);
        if n_rows > 0 {
            let y_of = |row: usize| bar.top() + bar.height() * row as f32 / n_rows as f32;
            for (i, h) in self.result.hunks.iter().enumerate() {
                let first = self.result.rows[h.start];
                let color = colors.row(&first).unwrap_or(palette.accent);
                let top = y_of(h.start);
                let bottom = y_of(h.end).max(top + 2.0);
                let solid = Color32::from_rgb(color.r(), color.g(), color.b());
                let color = if Some(i) == self.current {
                    palette.accent
                } else {
                    solid.gamma_multiply(if palette.dark { 2.2 } else { 1.0 })
                };
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(bar.left() + 3.0, top),
                        Pos2::new(bar.right() - 3.0, bottom),
                    ),
                    1.0,
                    color,
                );
            }
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(bar.left() + 1.0, y_of(visible.start)),
                    Pos2::new(
                        bar.right() - 1.0,
                        y_of(visible.end).max(y_of(visible.start) + 4.0),
                    ),
                ),
                2.0,
                Stroke::new(1.0, palette.text_dim),
                egui::StrokeKind::Inside,
            );
            let response = ui.interact(
                bar,
                ui.id().with("compare-overview"),
                Sense::click_and_drag(),
            );
            if (response.clicked() || response.dragged())
                && let Some(p) = response.interact_pointer_pos()
            {
                let frac = ((p.y - bar.top()) / bar.height()).clamp(0.0, 1.0);
                self.scroll_to = Some((frac * n_rows as f32) as usize);
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        action
    }
}

/// A line as drawn (tabs expanded, very long lines cut) and the display
/// column where each char starts, plus one entry for the end.
fn display_line(line: &str) -> (String, Vec<usize>) {
    let mut out = String::new();
    let mut cols = Vec::with_capacity(line.len().min(MAX_LINE_CHARS) + 1);
    let mut col = 0;
    for c in line.chars().take(MAX_LINE_CHARS) {
        cols.push(col);
        if c == '\t' {
            let n = crate::editor::layout::tab_width() - col % crate::editor::layout::tab_width();
            out.extend(std::iter::repeat_n(' ', n));
            col += n;
        } else {
            out.push(c);
            col += c.width().unwrap_or(0);
        }
    }
    cols.push(col);
    (out, cols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_columns_expand_tabs() {
        let (s, cols) = display_line("a\tb");
        assert_eq!(s, "a   b");
        assert_eq!(cols, vec![0, 1, 4, 5]);
    }

    #[test]
    fn stepping_through_differences() {
        let side = |t: &str| Side::new(Source::Text, "x".into(), t, 0);
        let mut v = CompareView::new(
            side("a\nb\nc\nd\ne"),
            side("a\nB\nc\nd\nE"),
            CompareOptions::default(),
        );
        assert_eq!(v.result.hunks.len(), 2);
        assert_eq!(v.current, Some(0));
        v.step(true);
        assert_eq!(v.current, Some(1));
        v.step(true);
        assert_eq!(v.current, Some(1));
        v.step(false);
        assert_eq!(v.current, Some(0));
        assert_eq!(v.summary_text(), "Lines: 2 changed");
        v.swap();
        assert_eq!(v.left.lines[1], "B");
    }
}
