//! Drawing the editor and turning keyboard and mouse input into commands.
//!
//! Only the rows inside the scroll viewport are laid out, so the cost of a
//! frame depends on the window size, not the file size.

use std::collections::{BTreeSet, HashMap};
use std::ops::Range;
use std::sync::Arc;

use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{
    Align2, CursorIcon, Event, EventFilter, FontId, Galley, Id, Key, Modifiers, Pos2, Rect,
    ScrollArea, Sense, Stroke, Ui, Vec2,
};
use syntect::highlighting::Theme;
use syntect::parsing::{SyntaxReference, SyntaxSet};

use super::buffer::{CharEdit, LineEdit};
use super::fold::FoldMark;
use super::highlight::LineSpans;
use super::layout::{char_cols, wrap_points};
use super::selection::{Selection, Selections};
use super::smart::ColumnSel;
use super::{DragMode, Editor};
use crate::menu::{self, MenuEntry};
use crate::theme::Palette;

/// Lines longer than this (in chars) are drawn a window at a time when not wrapping.
const CLIP_LINE_CHARS: usize = 4000;
/// How many lines the highlighter may parse per frame before showing text unstyled.
const HIGHLIGHT_BUDGET: usize = 3000;
const TEXT_PAD: f32 = 6.0;
const MULTI_CLICK_SECS: f64 = 0.45;
/// Width of the bookmark column at the left of the gutter.
const BOOKMARK_MARGIN: f32 = 16.0;
/// Width of the fold-marker column at the right of the gutter.
const FOLD_MARGIN: f32 = 16.0;
/// Lines searched below a blank line for the indentation its guides follow.
const GUIDE_LOOKAHEAD: usize = 50;

pub struct ViewConfig<'a> {
    pub font: FontId,
    pub wrap: bool,
    pub show_line_numbers: bool,
    pub highlight_current_line: bool,
    pub palette: &'a Palette,
    pub syntax: &'a SyntaxReference,
    pub syntaxes: &'a Arc<SyntaxSet>,
    pub theme: &'a Theme,
    pub theme_name: &'a str,
    pub plain: bool,
    pub request_focus: bool,
    pub show_whitespace: bool,
    pub show_eol: bool,
    /// Label control characters, unusual spaces and zero-width characters.
    pub show_invisible: bool,
    /// Column mode: a plain drag or Shift+arrows select a rectangle.
    pub column_mode: bool,
    /// Shown at line ends when `show_eol` is on, such as "LF".
    pub eol_label: &'a str,
    pub indent_guides: bool,
    /// Draw a vertical line at this column.
    pub edge_column: Option<usize>,
    /// The right-click menu; when empty, a built-in Undo/Cut/Copy/Paste menu.
    pub context_menu: &'a [MenuEntry],
}

/// One laid-out visual row.
struct RowLayout {
    line: usize,
    line_start: usize,
    /// Char range of this row within the line.
    start: usize,
    end: usize,
    /// True for the first row of a line (where the line number goes).
    first_in_line: bool,
    /// True for the last row of a line (where the newline sits).
    last_in_line: bool,
    /// X of the first drawn char, relative to the text origin.
    x_offset: f32,
    galley: Arc<Galley>,
    /// Display-string char index for each char in `start..=end`.
    disp: Vec<usize>,
    /// Offsets (within the line) of tabs to draw an arrow across.
    tabs: Vec<usize>,
}

impl RowLayout {
    /// X of the boundary before char `offset` (within the line), relative to the text origin.
    fn x_of(&self, offset: usize) -> f32 {
        let i = offset.clamp(self.start, self.end) - self.start;
        let d = self.disp[i];
        self.x_offset + self.galley.pos_from_cursor(CCursor::new(d)).min.x
    }

    /// Char offset (within the line) nearest to `x`.
    fn offset_at(&self, x: f32) -> usize {
        let local = (x - self.x_offset).max(0.0);
        let d = self
            .galley
            .cursor_from_pos(Vec2::new(local, self.galley.size().y / 2.0))
            .index
            .0;
        // `disp` is sorted; find the char whose display span contains `d`.
        let i = match self.disp.binary_search(&d) {
            Ok(i) => i,
            Err(i) => {
                let i = i.saturating_sub(1);
                let span = self.disp.get(i + 1).copied().unwrap_or(d) - self.disp[i];
                if span > 0 && (d - self.disp[i]) * 2 >= span {
                    i + 1
                } else {
                    i
                }
            }
        };
        self.start + i.min(self.end - self.start)
    }
}

/// Lines and wrap points fetched during one frame.
#[derive(Default)]
struct FrameCache {
    lines: HashMap<usize, Arc<Vec<char>>>,
    wraps: HashMap<usize, Arc<Vec<usize>>>,
    rows: HashMap<usize, Arc<RowLayout>>,
}

/// Moves bookmarks through line edits. A bookmark stays on the line where an
/// edit starts; lines an edit joins into that line lose theirs.
fn map_bookmarks(marks: &BTreeSet<usize>, edits: &[LineEdit]) -> BTreeSet<usize> {
    let mut lines: Vec<usize> = marks.iter().copied().collect();
    for e in edits {
        let end = e.start_line + e.old_lines;
        lines.retain(|&l| l <= e.start_line || l >= end);
        for l in &mut lines {
            if *l >= end {
                *l = *l + e.new_lines - e.old_lines;
            }
        }
    }
    lines.into_iter().collect()
}

/// Moves marked ranges through char edits; a mark an edit touches is dropped.
/// Where char `pos` ends up after `edits` (applied in order). A position
/// inside replaced text moves to the end of the new text.
fn map_pos(mut pos: usize, edits: &[CharEdit]) -> usize {
    for e in edits {
        let end = e.start + e.removed;
        if pos >= end {
            pos = pos + e.inserted - e.removed;
        } else if pos > e.start {
            pos = e.start + e.inserted;
        }
    }
    pos
}

fn map_marks(marks: &[Range<usize>], edits: &[CharEdit]) -> Vec<Range<usize>> {
    let mut out = marks.to_vec();
    for e in edits {
        let end = e.start + e.removed;
        out.retain(|m| m.end <= e.start || m.start >= end);
        for m in &mut out {
            if m.start >= end {
                m.start = m.start + e.inserted - e.removed;
                m.end = m.end + e.inserted - e.removed;
            }
        }
    }
    out.sort_by_key(|m| (m.start, m.end));
    out
}

/// Fills the parts of `ranges` (sorted char ranges) that fall on the visible rows.
fn paint_ranges(
    painter: &egui::Painter,
    rows: &[(usize, Arc<RowLayout>)],
    ranges: &[Range<usize>],
    color: egui::Color32,
    text_x: f32,
    origin_y: f32,
    row_h: f32,
) {
    let (Some((_, first)), Some((_, last))) = (rows.first(), rows.last()) else {
        return;
    };
    let vis_start = first.line_start + first.start;
    let vis_end = last.line_start + last.end;
    let from = ranges.partition_point(|r| r.end < vis_start);
    for r in ranges[from..].iter().take_while(|r| r.start <= vis_end) {
        for (row_idx, row) in rows {
            let row_start = row.line_start + row.start;
            let row_end = row.line_start + row.end;
            if r.end <= row_start || r.start > row_end || (r.start == row_end && !row.last_in_line)
            {
                continue;
            }
            let x0 = row.x_of(r.start.max(row_start) - row.line_start);
            let x1 = row.x_of(r.end.min(row_end) - row.line_start);
            if x1 > x0 {
                let y = origin_y + *row_idx as f32 * row_h;
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(text_x + x0, y),
                        Pos2::new(text_x + x1, y + row_h),
                    ),
                    2.0,
                    color,
                );
            }
        }
    }
}

/// A completion entry: the word, with its kind or signature dimmed after it.
fn completion_label(
    item: &super::smart::CompletionItem,
    cfg: &ViewConfig,
) -> egui::text::LayoutJob {
    use super::smart::ItemKind;
    let palette = cfg.palette;
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &item.text,
        0.0,
        egui::TextFormat::simple(cfg.font.clone(), palette.text),
    );
    let note = match (&item.detail, item.kind) {
        (Some(d), _) => {
            let mut d = d.clone();
            if d.chars().count() > 48 {
                d = d.chars().take(47).collect::<String>() + "…";
            }
            Some(d)
        }
        (None, ItemKind::Keyword) => Some(t!("completion-keyword")),
        (None, ItemKind::Function) => Some(t!("completion-function")),
        (None, ItemKind::Word) => None,
    };
    if let Some(note) = note {
        job.append(
            &note,
            cfg.font.size,
            egui::TextFormat::simple(
                FontId::proportional((cfg.font.size * 0.85).max(10.0)),
                palette.gutter_fg,
            ),
        );
    }
    job
}

/// Font metrics for the editor grid.
#[derive(Clone, Copy)]
struct Metrics {
    row_h: f32,
    char_w: f32,
}

impl Editor {
    /// Applies pending edits to the row index, highlighter, bookmarks and marks.
    pub(super) fn sync_caches(&mut self) {
        let edits = self.buffer.take_line_edits();
        if let Some(first) = edits.iter().map(|e| e.start_line).min() {
            self.rows.apply_edits(&self.buffer, &edits);
            self.map_folds(&edits);
            self.fold_scan.edited(&edits, self.buffer.len_lines());
            self.highlighter.invalidate_from(first);
            if !self.bookmarks.is_empty() {
                self.bookmarks = map_bookmarks(&self.bookmarks, &edits);
            }
        }
        let char_edits = self.buffer.take_char_edits();
        if !char_edits.is_empty()
            && let Some(other) = self.other_view.as_deref_mut()
        {
            // The other view's carets move with edits made in this one.
            other.selections.map(|sel| {
                Selection::new(
                    map_pos(sel.anchor, &char_edits),
                    map_pos(sel.head, &char_edits),
                )
            });
            other.column = None;
            other.completion = None;
            other.hint = None;
        }
        if !char_edits.is_empty() && !self.marks.is_empty() {
            if char_edits.len().saturating_mul(self.marks.len()) > 5_000_000 {
                self.marks.clear();
            } else {
                self.marks = map_marks(&self.marks, &char_edits);
            }
        }
    }

    fn line_chars(&self, cache: &mut FrameCache, line: usize) -> Arc<Vec<char>> {
        cache
            .lines
            .entry(line)
            .or_insert_with(|| {
                Arc::new(
                    self.buffer
                        .rope()
                        .line(line)
                        .chars()
                        .filter(|&c| c != '\n')
                        .collect(),
                )
            })
            .clone()
    }

    fn line_wraps(&self, cache: &mut FrameCache, line: usize) -> Arc<Vec<usize>> {
        if let Some(w) = cache.wraps.get(&line) {
            return w.clone();
        }
        let chars = self.line_chars(cache, line);
        let w = Arc::new(wrap_points(&chars, self.rows.wrap_cols()));
        cache.wraps.insert(line, w.clone());
        w
    }

    /// Visual row and display column of char `pos`.
    fn visual_pos(&mut self, cache: &mut FrameCache, pos: usize) -> (usize, usize) {
        let line = self.buffer.char_to_line(pos);
        let offset = pos - self.buffer.line_start(line);
        let wraps = self.line_wraps(cache, line);
        let sub = wraps.iter().rposition(|&p| p <= offset).unwrap_or(0);
        let chars = self.line_chars(cache, line);
        let mut col = 0;
        for &c in &chars[wraps[sub]..offset.min(chars.len())] {
            col += char_cols(c, col);
        }
        (self.rows.line_to_row(line) + sub, col)
    }

    /// Char position on visual `row` closest to display column `col`.
    fn pos_at(&mut self, cache: &mut FrameCache, row: usize, col: usize) -> usize {
        let (line, sub) = self.rows.row_to_line(row);
        let wraps = self.line_wraps(cache, line);
        let chars = self.line_chars(cache, line);
        let start = wraps.get(sub).copied().unwrap_or(0);
        let end = wraps.get(sub + 1).copied().unwrap_or(chars.len());
        let mut c = 0;
        let mut i = start;
        while i < end {
            let w = char_cols(chars[i], c);
            if c + w > col {
                if col - c >= w.div_ceil(2) {
                    i += 1;
                }
                break;
            }
            c += w;
            i += 1;
        }
        // On a wrapped row (not the last), stay before the wrap point.
        if sub + 1 < wraps.len() && i >= end && end > start {
            i = end - 1;
        }
        self.buffer.line_start(line) + i.min(chars.len())
    }

    fn move_vertical(&mut self, cache: &mut FrameCache, rows: isize, extend: bool) {
        self.sync_caches();
        let total = self.rows.total_rows();
        let len = self.buffer.len_chars();
        let mut out = Vec::new();
        for s in self.selections.clone().iter() {
            let (row, col) = self.visual_pos(cache, s.head);
            let goal = s.goal_col.unwrap_or(col);
            let target = row as isize + rows;
            let pos = if target < 0 {
                0
            } else if target as usize >= total {
                len
            } else {
                self.pos_at(cache, target as usize, goal)
            };
            let mut moved = if !extend && !s.is_empty() && rows.abs() == 1 {
                Selection::caret(pos)
            } else {
                s.moved_to(pos, extend)
            };
            moved.goal_col = Some(goal);
            out.push(moved);
        }
        let primary = self.selections.primary_index();
        self.set_selections(Selections::from_vec(out, primary));
    }

    /// Adds a caret on the row above or below the primary one.
    fn add_caret_vertical(&mut self, cache: &mut FrameCache, down: bool) {
        self.sync_caches();
        let edge = if down {
            self.selections.iter().map(|s| s.head).max().unwrap_or(0)
        } else {
            self.selections.iter().map(|s| s.head).min().unwrap_or(0)
        };
        let (row, col) = self.visual_pos(cache, edge);
        let goal = self.selections.primary().goal_col.unwrap_or(col);
        let total = self.rows.total_rows();
        let target = if down { row + 1 } else { row.wrapping_sub(1) };
        if target >= total {
            return;
        }
        let pos = self.pos_at(cache, target, goal);
        let mut sels = self.selections.clone();
        sels.push(
            Selection {
                anchor: pos,
                head: pos,
                goal_col: Some(goal),
            },
            true,
        );
        self.set_selections(sels);
    }

    /// The rectangular selection to extend: the one being made, or a new one
    /// from the primary selection.
    fn column_state(&mut self, cache: &mut FrameCache) -> ColumnSel {
        if let Some((c, sels)) = &self.column
            && *sels == self.selections
        {
            return *c;
        }
        let p = self.selections.primary();
        let anchor = self.visual_pos(cache, p.anchor);
        let head = self.visual_pos(cache, p.head);
        ColumnSel { anchor, head }
    }

    /// Selects the columns between `c.anchor` and `c.head` on every row between them.
    fn set_column_selection(&mut self, cache: &mut FrameCache, c: ColumnSel) {
        let last_row = self.rows.total_rows().saturating_sub(1);
        let (r0, r1) = (
            c.anchor.0.min(c.head.0),
            c.anchor.0.max(c.head.0).min(last_row),
        );
        let sels: Vec<Selection> = (r0..=r1)
            .map(|r| {
                let a = self.pos_at(cache, r, c.anchor.1);
                let h = self.pos_at(cache, r, c.head.1);
                Selection::new(a, h)
            })
            .collect();
        let primary = if c.head.0 >= c.anchor.0 {
            sels.len() - 1
        } else {
            0
        };
        self.set_selections(Selections::from_vec(sels, primary));
        self.column = Some((c, self.selections.clone()));
    }

    fn extend_column_selection(&mut self, cache: &mut FrameCache, rows: isize, cols: isize) {
        self.sync_caches();
        let mut c = self.column_state(cache);
        let last_row = self.rows.total_rows().saturating_sub(1) as isize;
        c.head.0 = (c.head.0 as isize + rows).clamp(0, last_row) as usize;
        c.head.1 = (c.head.1 as isize + cols).max(0) as usize;
        self.set_column_selection(cache, c);
    }

    /// Display width of a line's indentation for its indent guides. A blank line
    /// takes the smaller indentation of the text lines around it.
    fn guide_indent(&self, cache: &mut FrameCache, line: usize) -> usize {
        let indent_of = |chars: &[char]| -> Option<usize> {
            let mut col = 0;
            for &c in chars {
                match c {
                    ' ' | '\t' => col += char_cols(c, col),
                    '\r' => return None,
                    _ => return Some(col),
                }
            }
            None
        };
        let chars = self.line_chars(cache, line);
        if let Some(i) = indent_of(&chars) {
            return i;
        }
        let n = self.buffer.len_lines();
        let next = (line + 1..n.min(line + GUIDE_LOOKAHEAD))
            .find_map(|l| indent_of(&self.line_chars(cache, l)));
        let prev = (line.saturating_sub(GUIDE_LOOKAHEAD)..line)
            .rev()
            .find_map(|l| indent_of(&self.line_chars(cache, l)));
        match (prev, next) {
            (Some(a), Some(b)) => a.min(b),
            _ => 0,
        }
    }

    /// Whole-word, case-sensitive occurrences of `word` on the visible lines,
    /// apart from the selection itself.
    fn word_hits(
        &self,
        cache: &mut FrameCache,
        rows: &[(usize, Arc<RowLayout>)],
        word: &str,
    ) -> Vec<Range<usize>> {
        let w: Vec<char> = word.chars().collect();
        let own = self.selections.primary().start();
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let mut out = Vec::new();
        let mut last_line = None;
        for (_, row) in rows {
            if last_line == Some(row.line) {
                continue;
            }
            last_line = Some(row.line);
            let chars = self.line_chars(cache, row.line);
            if chars.len() > CLIP_LINE_CHARS * 5 {
                continue;
            }
            let mut i = 0;
            while i + w.len() <= chars.len() {
                let bounded = (i == 0 || !is_word(chars[i - 1]))
                    && chars.get(i + w.len()).is_none_or(|&c| !is_word(c));
                if bounded && chars[i..i + w.len()] == w[..] {
                    let start = row.line_start + i;
                    if start != own {
                        out.push(start..start + w.len());
                    }
                    i += w.len();
                } else {
                    i += 1;
                }
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn build_row(
        &mut self,
        cache: &mut FrameCache,
        ui: &Ui,
        row: usize,
        cfg: &ViewConfig,
        metrics: Metrics,
        visible_cols: (f32, f32),
        budget: &mut usize,
    ) -> Arc<RowLayout> {
        if let Some(r) = cache.rows.get(&row) {
            return r.clone();
        }
        let (line, sub) = self.rows.row_to_line(row);
        let chars = self.line_chars(cache, line);
        let wraps = self.line_wraps(cache, line);
        let mut start = wraps.get(sub).copied().unwrap_or(0).min(chars.len());
        let mut end = wraps
            .get(sub + 1)
            .copied()
            .unwrap_or(chars.len())
            .min(chars.len());
        let last_in_line = sub + 1 >= wraps.len();

        // Very long unwrapped lines: lay out only the columns on screen.
        let mut start_col = 0usize;
        if !cfg.wrap && chars.len() > CLIP_LINE_CHARS {
            let first_col = (visible_cols.0 / metrics.char_w).floor().max(0.0) as usize;
            let last_col = (visible_cols.1 / metrics.char_w).ceil() as usize + 8;
            let mut col = 0;
            let mut i = 0;
            while i < chars.len() && col + char_cols(chars[i], col) <= first_col.saturating_sub(8) {
                col += char_cols(chars[i], col);
                i += 1;
            }
            start = i;
            start_col = col;
            while i < chars.len() && col < last_col {
                col += char_cols(chars[i], col);
                i += 1;
            }
            end = i;
        }

        let spans: Option<LineSpans> = if chars.len() > CLIP_LINE_CHARS && !cfg.wrap {
            None
        } else {
            self.highlighter
                .line_spans(line, &self.buffer, cfg.syntaxes, cfg.theme, budget)
        };

        let mut job = LayoutJob::default();
        let mut disp = Vec::with_capacity(end - start + 1);
        let mut text = String::new();
        let mut disp_len = 0usize;
        let mut byte: usize = chars[..start].iter().map(|c| c.len_utf8()).sum();
        let mut span_idx = 0;
        let mut col = if wraps.len() > 1 { 0 } else { start_col };
        let mut run_format: Option<TextFormat> = None;
        let base = TextFormat::simple(cfg.font.clone(), cfg.palette.editor_fg);
        let ws_format = TextFormat::simple(cfg.font.clone(), cfg.palette.whitespace);
        let mut label_format = TextFormat::simple(
            FontId::monospace(cfg.font.size * 0.7),
            cfg.palette.editor_fg,
        );
        label_format.background = cfg.palette.whitespace.gamma_multiply(0.7);
        label_format.valign = egui::Align::Center;
        let mut tabs = Vec::new();
        let flush = |job: &mut LayoutJob, text: &mut String, fmt: &Option<TextFormat>| {
            if !text.is_empty() {
                job.append(text, 0.0, fmt.clone().unwrap_or_else(|| base.clone()));
                text.clear();
            }
        };
        for (i, &c) in chars[start..end].iter().enumerate() {
            disp.push(disp_len);
            let format = match &spans {
                Some(spans) => {
                    while span_idx < spans.len() && (spans[span_idx].end as usize) <= byte {
                        span_idx += 1;
                    }
                    match spans.get(span_idx).filter(|s| s.start as usize <= byte) {
                        Some(s) => {
                            let mut f = TextFormat::simple(cfg.font.clone(), s.color);
                            f.italics = s.italic;
                            if s.underline {
                                f.underline = Stroke::new(1.0, s.color);
                            }
                            f
                        }
                        None => base.clone(),
                    }
                }
                None => base.clone(),
            };
            let label = if cfg.show_invisible {
                super::invisible::label(c)
            } else {
                None
            };
            let format = if label.is_some() {
                label_format.clone()
            } else if cfg.show_whitespace && (c == ' ' || c == '\t') {
                ws_format.clone()
            } else {
                format
            };
            if run_format.as_ref() != Some(&format) {
                flush(&mut job, &mut text, &run_format);
                run_format = Some(format);
            }
            let w = char_cols(c, col);
            match c {
                _ if label.is_some() => {
                    let label = label.unwrap_or_default();
                    text.push_str(label);
                    disp_len += label.len();
                }
                '\t' => {
                    // Shown as an arrow across the whole tab, drawn later.
                    if cfg.show_whitespace {
                        tabs.push(start + i);
                    }
                    text.extend(std::iter::repeat_n(' ', w));
                    disp_len += w;
                }
                ' ' if cfg.show_whitespace => {
                    text.push('·');
                    disp_len += 1;
                }
                c if c.is_control() => {
                    text.push(char::from_u32(0x2400 + (c as u32).min(0x21)).unwrap_or('\u{FFFD}'));
                    disp_len += 1;
                }
                c => {
                    text.push(c);
                    disp_len += 1;
                }
            }
            col += w;
            byte += c.len_utf8();
        }
        disp.push(disp_len);
        flush(&mut job, &mut text, &run_format);
        if job.sections.is_empty() {
            job.append("", 0.0, base.clone());
        }
        job.wrap.max_width = f32::INFINITY;
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let layout = Arc::new(RowLayout {
            line,
            line_start: self.buffer.line_start(line),
            start,
            end,
            first_in_line: sub == 0,
            last_in_line,
            x_offset: start_col as f32 * metrics.char_w,
            galley,
            disp,
            tabs,
        });
        cache.rows.insert(row, layout.clone());
        layout
    }

    /// Draws the editor and handles its input. Call once per frame. Returns
    /// the id of a clicked `cfg.context_menu` item.
    pub fn show(&mut self, ui: &mut Ui, id: Id, cfg: &ViewConfig) -> Option<usize> {
        let mut menu_clicked = None;
        let palette = cfg.palette;
        let metrics = Metrics {
            row_h: ui.fonts_mut(|f| f.row_height(&cfg.font)),
            char_w: ui.fonts_mut(|f| f.glyph_width(&cfg.font, '0')).max(1.0),
        };
        let number_font = FontId::monospace(cfg.font.size * 0.88);
        let digits = self.buffer.len_lines().max(999).to_string().len() as f32;
        let gutter_w = BOOKMARK_MARGIN
            + FOLD_MARGIN
            + if cfg.show_line_numbers {
                digits * ui.fonts_mut(|f| f.glyph_width(&number_font, '0')) + 8.0
            } else {
                0.0
            };
        let scrollbar = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_outer_margin + 4.0;
        let text_w =
            (ui.available_width() - gutter_w - TEXT_PAD - scrollbar).max(metrics.char_w * 10.0);
        let wrap_cols = cfg
            .wrap
            .then(|| ((text_w / metrics.char_w).floor() as usize).max(10));

        self.highlighter
            .configure(cfg.syntax, cfg.theme_name, cfg.theme, cfg.plain);
        self.fold_scan
            .configure(cfg.syntax, cfg.syntaxes, cfg.plain);
        self.sync_caches();
        self.fold_scan.poll(&self.buffer, ui.ctx());
        self.rows.set_wrap(&self.buffer, wrap_cols);

        let version_before = self.buffer.version();
        let mut cache = FrameCache::default();
        let mut changed = false;
        let has_focus = ui.memory(|m| m.has_focus(id));
        self.column_mode = cfg.column_mode;
        if has_focus {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                )
            });
            changed |= self.handle_keyboard(ui, &mut cache);
        }
        self.sync_caches();
        self.reveal_carets();
        if has_focus {
            self.update_hint(false);
        } else {
            self.completion = None;
            self.hint = None;
        }
        if changed {
            cache = FrameCache::default();
        }

        let total_rows = self.rows.total_rows();
        let content_w = if cfg.wrap {
            0.0
        } else {
            gutter_w + TEXT_PAD + (self.rows.max_width() as f32 + 4.0) * metrics.char_w
        };
        let content_h = (total_rows as f32 + 3.0) * metrics.row_h;

        let mut budget = HIGHLIGHT_BUDGET;
        let mut needs_repaint = false;
        let mut popup_at: Option<(Pos2, Pos2)> = None;
        let mut scroll = ScrollArea::both()
            .id_salt(id.with("scroll"))
            .auto_shrink(false);
        if let Some(row) = self.map_scroll.take() {
            scroll = scroll.vertical_scroll_offset(row * metrics.row_h);
        }
        let scroll = scroll.show_viewport(ui, |ui, viewport| {
            self.map_view = (
                viewport.min.y / metrics.row_h,
                viewport.height() / metrics.row_h,
            );
            let size = Vec2::new(
                content_w.max(viewport.width()),
                content_h.max(viewport.height()),
            );
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let response = ui.interact(rect, id, Sense::click_and_drag());
            let origin = rect.min;
            // An arrow over the gutter (fold markers, bookmarks, line numbers),
            // a text cursor over the text.
            let gutter_right = origin.x + viewport.min.x + gutter_w;
            if let Some(p) = response.hover_pos() {
                ui.ctx()
                    .set_cursor_icon(if p.x < gutter_right && !response.dragged() {
                        CursorIcon::Default
                    } else {
                        CursorIcon::Text
                    });
            }
            let text_x = origin.x + gutter_w + TEXT_PAD;
            let visible_cols = (viewport.min.x - gutter_w, viewport.max.x - gutter_w);
            self.page_rows = ((viewport.height() / metrics.row_h) as usize)
                .saturating_sub(1)
                .max(1);

            if cfg.request_focus {
                response.request_focus();
            }

            // Mouse.
            changed |= self.handle_mouse(
                ui,
                &response,
                &mut cache,
                cfg,
                metrics,
                origin,
                text_x,
                gutter_right,
                visible_cols,
                &mut budget,
            );
            if changed {
                self.sync_caches();
                cache = FrameCache::default();
            }

            // Rows on screen.
            let total_rows = self.rows.total_rows();
            let first =
                ((viewport.min.y / metrics.row_h).floor().max(0.0) as usize).min(total_rows);
            let last = ((viewport.max.y / metrics.row_h).ceil() as usize + 1).min(total_rows);
            let rows: Vec<(usize, Arc<RowLayout>)> = (first..last)
                .map(|r| {
                    (
                        r,
                        self.build_row(&mut cache, ui, r, cfg, metrics, visible_cols, &mut budget),
                    )
                })
                .collect();
            if budget == 0 {
                needs_repaint = true;
            }

            let painter = ui.painter();
            let row_y = |r: usize| origin.y + r as f32 * metrics.row_h;
            let caret_lines: Vec<usize> = self
                .selections
                .iter()
                .map(|s| self.buffer.char_to_line(s.head))
                .collect();

            // Current-line band.
            if cfg.highlight_current_line {
                for (r, row) in &rows {
                    if caret_lines.contains(&row.line) {
                        let band = Rect::from_min_max(
                            Pos2::new(origin.x + viewport.min.x, row_y(*r)),
                            Pos2::new(origin.x + viewport.max.x, row_y(*r) + metrics.row_h),
                        );
                        painter.rect_filled(band, 0.0, palette.line_highlight);
                    }
                }
            }

            // Long-line edge.
            if let Some(edge) = cfg.edge_column {
                let x = text_x + edge as f32 * metrics.char_w;
                painter.line_segment(
                    [
                        Pos2::new(x, origin.y + viewport.min.y),
                        Pos2::new(x, origin.y + viewport.max.y),
                    ],
                    Stroke::new(1.0, palette.guide.gamma_multiply(0.7)),
                );
            }

            // Indent guides, one per indentation level.
            if cfg.indent_guides {
                for (r, row) in &rows {
                    let indent = self.guide_indent(&mut cache, row.line);
                    let mut col = 0;
                    while col < indent {
                        let x = text_x + col as f32 * metrics.char_w;
                        painter.line_segment(
                            [
                                Pos2::new(x + 0.5, row_y(*r)),
                                Pos2::new(x + 0.5, row_y(*r) + metrics.row_h),
                            ],
                            Stroke::new(1.0, palette.guide.gamma_multiply(0.55)),
                        );
                        col += super::layout::tab_width();
                    }
                }
            }

            // Marks, find matches and other occurrences of the selected word.
            paint_ranges(
                painter,
                &rows,
                &self.marks,
                palette.mark,
                text_x,
                origin.y,
                metrics.row_h,
            );
            paint_ranges(
                painter,
                &rows,
                &self.search_hits,
                palette.find_hit,
                text_x,
                origin.y,
                metrics.row_h,
            );
            if let Some(word) = self.selected_word() {
                let hits = self.word_hits(&mut cache, &rows, &word);
                paint_ranges(
                    painter,
                    &rows,
                    &hits,
                    palette.word_hit,
                    text_x,
                    origin.y,
                    metrics.row_h,
                );
            }

            // Selections.
            for s in self.selections.iter().filter(|s| !s.is_empty()) {
                for (r, row) in &rows {
                    let row_start = row.line_start + row.start;
                    let row_end = row.line_start + row.end;
                    let sel_reaches_newline = row.last_in_line && s.end() > row_end;
                    if s.end() < row_start
                        || s.start() > row_end
                        || (s.end() == row_start && row.start != row.end)
                    {
                        continue;
                    }
                    let x0 = row.x_of(s.start().max(row_start) - row.line_start);
                    let mut x1 = row.x_of(s.end().min(row_end) - row.line_start);
                    if sel_reaches_newline {
                        x1 += metrics.char_w * 0.6;
                    }
                    if x1 > x0 {
                        let sel_rect = Rect::from_min_max(
                            Pos2::new(text_x + x0, row_y(*r)),
                            Pos2::new(text_x + x1, row_y(*r) + metrics.row_h),
                        );
                        painter.rect_filled(sel_rect, 2.0, palette.selection);
                    }
                }
            }

            // The bracket at the caret and its partner.
            if let Some(m) = self.brace_match() {
                let color = if m.matched {
                    palette.brace_match
                } else {
                    palette.brace_bad
                };
                paint_ranges(
                    painter,
                    &rows,
                    &m.ranges,
                    color,
                    text_x,
                    origin.y,
                    metrics.row_h,
                );
            }

            // Text.
            for (r, row) in &rows {
                painter.galley(
                    Pos2::new(text_x + row.x_offset, row_y(*r)),
                    row.galley.clone(),
                    palette.editor_fg,
                );
            }

            // Tabs: an arrow from one side of the tab to the other.
            let tab_stroke = Stroke::new(1.0, palette.whitespace);
            for (r, row) in &rows {
                for &t in &row.tabs {
                    let x0 = text_x + row.x_of(t) + metrics.char_w * 0.2;
                    let x1 = text_x + row.x_of(t + 1) - metrics.char_w * 0.2;
                    let y = (row_y(*r) + metrics.row_h / 2.0).round() + 0.5;
                    let head = (metrics.row_h * 0.18).clamp(2.5, 4.5).min(x1 - x0);
                    painter.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], tab_stroke);
                    painter.line_segment(
                        [Pos2::new(x1 - head, y - head), Pos2::new(x1, y)],
                        tab_stroke,
                    );
                    painter.line_segment(
                        [Pos2::new(x1 - head, y + head), Pos2::new(x1, y)],
                        tab_stroke,
                    );
                }
            }

            // End-of-line symbols and folded-region markers after the text.
            let last_line = self.buffer.len_lines() - 1;
            let small = FontId::monospace(cfg.font.size * 0.62);
            for (r, row) in &rows {
                if !row.last_in_line {
                    continue;
                }
                let mut x = text_x + row.x_of(row.end) + metrics.char_w * 0.4;
                let folded = self.is_folded(row.line);
                if cfg.show_eol && row.line < last_line {
                    let g = painter.layout_no_wrap(
                        cfg.eol_label.to_owned(),
                        small.clone(),
                        palette.whitespace,
                    );
                    let rect = Rect::from_min_size(
                        Pos2::new(x, row_y(*r) + (metrics.row_h - g.size().y - 2.0) / 2.0),
                        g.size() + Vec2::new(6.0, 2.0),
                    );
                    painter.rect_stroke(
                        rect,
                        2.0,
                        Stroke::new(1.0, palette.whitespace),
                        egui::StrokeKind::Inside,
                    );
                    painter.galley(rect.min + Vec2::new(3.0, 1.0), g, palette.whitespace);
                    x = rect.right() + metrics.char_w * 0.4;
                }
                if folded {
                    let g = painter.layout_no_wrap(
                        "...".to_owned(),
                        cfg.font.clone(),
                        palette.text_dim,
                    );
                    let rect = Rect::from_min_size(
                        Pos2::new(x, row_y(*r) + 1.0),
                        Vec2::new(g.size().x + 8.0, metrics.row_h - 2.0),
                    );
                    painter.rect_filled(rect, 3.0, palette.line_highlight);
                    painter.rect_stroke(
                        rect,
                        3.0,
                        Stroke::new(1.0, palette.guide),
                        egui::StrokeKind::Inside,
                    );
                    painter.galley(
                        Pos2::new(
                            rect.left() + 4.0,
                            row_y(*r) + (metrics.row_h - g.size().y) / 2.0,
                        ),
                        g,
                        palette.text_dim,
                    );
                    let y = row_y(*r) + metrics.row_h - 0.5;
                    painter.line_segment(
                        [
                            Pos2::new(origin.x + viewport.min.x + gutter_w, y),
                            Pos2::new(origin.x + viewport.max.x, y),
                        ],
                        Stroke::new(1.0, palette.guide.gamma_multiply(0.6)),
                    );
                }
            }

            // Carets and IME composition.
            let focused = response.has_focus();
            let mut primary_caret_rect = None;
            for (i, s) in self.selections.iter().enumerate() {
                let line = self.buffer.char_to_line(s.head);
                let offset = s.head - self.buffer.line_start(line);
                for (r, row) in &rows {
                    if row.line != line || offset < row.start || offset > row.end {
                        continue;
                    }
                    // At a wrap point the caret belongs to the next row.
                    if offset == row.end && !row.last_in_line {
                        continue;
                    }
                    let x = text_x + row.x_of(offset);
                    let caret = Rect::from_min_size(
                        Pos2::new(x - 1.0, row_y(*r)),
                        Vec2::new(2.0, metrics.row_h),
                    );
                    if focused {
                        painter.rect_filled(caret, 1.0, palette.accent);
                    }
                    if i == self.selections.primary_index() {
                        primary_caret_rect = Some(caret);
                        if let Some(pre) = self.preedit.as_ref().filter(|p| !p.is_empty()) {
                            let g = painter.layout_no_wrap(
                                pre.clone(),
                                cfg.font.clone(),
                                palette.editor_fg,
                            );
                            let bg = Rect::from_min_size(caret.min, g.size());
                            painter.rect_filled(bg, 0.0, palette.editor_bg);
                            painter.galley(caret.min, g, palette.editor_fg);
                            painter.line_segment(
                                [bg.left_bottom(), bg.right_bottom()],
                                Stroke::new(1.0, palette.accent),
                            );
                        }
                    }
                    break;
                }
            }

            // Gutter, drawn last so it stays put over horizontally scrolled text.
            let gutter = Rect::from_min_max(
                Pos2::new(origin.x + viewport.min.x, origin.y + viewport.min.y),
                Pos2::new(
                    origin.x + viewport.min.x + gutter_w,
                    origin.y + viewport.max.y,
                ),
            );
            painter.rect_filled(gutter, 0.0, palette.editor_bg);
            if cfg.highlight_current_line {
                for (r, row) in &rows {
                    if caret_lines.contains(&row.line) {
                        let band = Rect::from_min_max(
                            Pos2::new(gutter.left(), row_y(*r)),
                            Pos2::new(gutter.right(), row_y(*r) + metrics.row_h),
                        );
                        painter.rect_filled(band, 0.0, palette.line_highlight);
                    }
                }
            }
            if cfg.show_line_numbers {
                for (r, row) in &rows {
                    if !row.first_in_line {
                        continue;
                    }
                    let current = caret_lines.contains(&row.line);
                    painter.text(
                        Pos2::new(
                            gutter.right() - FOLD_MARGIN,
                            row_y(*r) + metrics.row_h / 2.0,
                        ),
                        Align2::RIGHT_CENTER,
                        (row.line + 1).to_string(),
                        number_font.clone(),
                        if current {
                            palette.gutter_active_fg
                        } else {
                            palette.gutter_fg
                        },
                    );
                }
            }

            // Fold markers: a box with − where a region can fold and + where
            // one is folded, joined by a line down to where the region ends.
            let fold_x = (gutter.right() - FOLD_MARGIN / 2.0).round() + 0.5;
            let lines: Vec<usize> = rows
                .iter()
                .filter(|(_, row)| row.first_in_line)
                .map(|(_, row)| row.line)
                .collect();
            let marks: HashMap<usize, FoldMark> =
                lines.iter().copied().zip(self.fold_marks(&lines)).collect();
            let line_stroke = Stroke::new(1.0, palette.gutter_fg);
            let box_size = (metrics.row_h * 0.62).clamp(9.0, 13.0).round();
            let half = box_size / 2.0;
            for (r, row) in &rows {
                let Some(mark) = marks.get(&row.line) else {
                    continue;
                };
                let top = row_y(*r);
                let bottom = top + metrics.row_h;
                if !row.first_in_line {
                    if mark.below {
                        painter.vline(fold_x, top..=bottom, line_stroke);
                    }
                    continue;
                }
                let mid = (top + metrics.row_h / 2.0).round() + 0.5;
                if mark.header {
                    let folded = self.is_folded(row.line);
                    let rect =
                        Rect::from_center_size(Pos2::new(fold_x, mid), Vec2::splat(box_size));
                    if mark.above {
                        painter.vline(fold_x, top..=rect.top(), line_stroke);
                    }
                    if mark.below {
                        painter.vline(fold_x, rect.bottom()..=bottom, line_stroke);
                    }
                    let color = if folded {
                        palette.accent
                    } else {
                        palette.gutter_active_fg
                    };
                    painter.rect(
                        rect,
                        2.5,
                        palette.editor_bg,
                        Stroke::new(1.0, color),
                        egui::StrokeKind::Inside,
                    );
                    let arm = half - 2.5;
                    let sign = Stroke::new(1.0, color);
                    painter.hline(fold_x - arm..=fold_x + arm, mid, sign);
                    if folded {
                        painter.vline(fold_x, mid - arm..=mid + arm, sign);
                    }
                } else {
                    if mark.above {
                        let to = if mark.end && !mark.below { mid } else { bottom };
                        painter.vline(fold_x, top..=to, line_stroke);
                    } else if mark.below {
                        painter.vline(fold_x, mid..=bottom, line_stroke);
                    }
                    if mark.end {
                        painter.hline(fold_x..=fold_x + half, mid, line_stroke);
                    }
                }
            }

            if !self.bookmarks.is_empty() {
                let radius = (metrics.row_h * 0.26).clamp(3.0, 6.0);
                for (r, row) in &rows {
                    if row.first_in_line && self.bookmarks.contains(&row.line) {
                        painter.circle_filled(
                            Pos2::new(
                                gutter.left() + BOOKMARK_MARGIN / 2.0 + 1.0,
                                row_y(*r) + metrics.row_h / 2.0,
                            ),
                            radius,
                            palette.accent,
                        );
                    }
                }
            }

            // Page Up / Page Down scroll the view along with the caret.
            let page_scroll = std::mem::take(&mut self.pending_scroll_rows);
            if page_scroll != 0 {
                ui.scroll_with_delta_animation(
                    Vec2::new(0.0, -(page_scroll as f32) * metrics.row_h),
                    egui::style::ScrollAnimation::none(),
                );
                self.scroll_to_caret = false;
            }

            // Keep the caret on screen after it moves.
            if std::mem::take(&mut self.scroll_to_caret) {
                let head = self.selections.primary().head;
                let (row, _) = self.visual_pos(&mut cache, head);
                let layout =
                    self.build_row(&mut cache, ui, row, cfg, metrics, visible_cols, &mut budget);
                let line_offset = head - layout.line_start;
                let x = text_x + layout.x_of(line_offset);
                let target = Rect::from_min_size(
                    Pos2::new(x - metrics.char_w * 4.0 - gutter_w, row_y(row)),
                    Vec2::new(metrics.char_w * 8.0 + gutter_w, metrics.row_h),
                );
                ui.scroll_to_rect_animation(
                    target.expand2(Vec2::new(0.0, metrics.row_h)),
                    None,
                    egui::style::ScrollAnimation::none(),
                );
            }

            if (self.completion.is_some() || self.hint.is_some())
                && let Some(caret) = primary_caret_rect
            {
                let to_global = ui
                    .ctx()
                    .layer_transform_to_global(ui.layer_id())
                    .unwrap_or_default();
                popup_at = Some((
                    to_global * caret.left_bottom(),
                    to_global * caret.left_top(),
                ));
            }

            if focused && let Some(caret) = primary_caret_rect {
                let to_global = ui
                    .ctx()
                    .layer_transform_to_global(ui.layer_id())
                    .unwrap_or_default();
                ui.output_mut(|o| {
                    o.ime = Some(egui::output::IMEOutput {
                        purpose: egui::IMEPurpose::Normal,
                        rect: to_global * viewport.translate(origin.to_vec2()),
                        cursor_rect: to_global * caret,
                        should_interrupt_composition: false,
                    });
                });
            }

            menu_clicked = self.context_menu(&response, cfg.context_menu);
        });
        let _ = scroll;
        if let Some((below, above)) = popup_at {
            self.completion_popup(ui, id, below, cfg);
            self.hint_popup(ui, id, above, cfg);
        }
        // Search highlights are recomputed next frame after the text changes.
        if needs_repaint || self.buffer.version() != version_before {
            ui.ctx().request_repaint();
        }
        menu_clicked
    }

    /// The word-completion list under the caret.
    fn completion_popup(&mut self, ui: &Ui, id: Id, at: Pos2, cfg: &ViewConfig) {
        let Some(c) = self.completion.clone() else {
            return;
        };
        let palette = cfg.palette;
        let mut chosen = None;
        egui::Area::new(id.with("completion"))
            .order(egui::Order::Foreground)
            .fixed_pos(at + Vec2::new(-4.0, 2.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(palette.chrome_bg)
                    .inner_margin(egui::Margin::same(3))
                    .show(ui, |ui| {
                        ui.set_min_width(160.0);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ScrollArea::vertical()
                            .max_height(220.0)
                            .auto_shrink([true, true])
                            .show(ui, |ui| {
                                for (i, item) in c.items.iter().enumerate() {
                                    let r = ui.add(egui::Button::selectable(
                                        i == c.selected,
                                        completion_label(item, cfg),
                                    ));
                                    if i == c.selected {
                                        r.scroll_to_me(None);
                                    }
                                    if r.clicked() {
                                        chosen = Some(i);
                                    }
                                }
                            });
                    });
            });
        if let Some(i) = chosen {
            let ctx = ui.ctx().clone();
            self.accept_completion(&ctx, Some(i));
            ui.memory_mut(|m| m.request_focus(id));
        }
    }

    /// The parameter hint above the caret: each signature, with the
    /// parameter being typed in bold.
    fn hint_popup(&mut self, ui: &Ui, id: Id, at: Pos2, cfg: &ViewConfig) {
        let Some(hint) = self.hint.clone() else {
            return;
        };
        let palette = cfg.palette;
        let mut sigs: Vec<_> = hint.signatures.iter().collect();
        // Overloads that take the argument being typed come first.
        sigs.sort_by_key(|s| hint.param_in(s).is_none());
        sigs.truncate(6);
        let small = FontId::proportional((cfg.font.size * 0.85).max(10.0));
        egui::Area::new(id.with("hint"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .fixed_pos(at + Vec2::new(-4.0, -2.0))
            .interactable(false)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(palette.chrome_bg)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.set_max_width(720.0);
                        for sig in sigs {
                            let mut job = egui::text::LayoutJob::default();
                            let plain = egui::TextFormat::simple(cfg.font.clone(), palette.text);
                            let mut bold =
                                egui::TextFormat::simple(cfg.font.clone(), palette.accent);
                            bold.underline = Stroke::new(1.0, palette.accent);
                            let active = hint.param_in(sig).map(|i| sig.params[i].clone());
                            match active {
                                Some(r) => {
                                    job.append(&sig.label[..r.start], 0.0, plain.clone());
                                    job.append(&sig.label[r.clone()], 0.0, bold);
                                    job.append(&sig.label[r.end..], 0.0, plain);
                                }
                                None => job.append(&sig.label, 0.0, plain),
                            }
                            job.wrap.max_width = 720.0;
                            ui.label(job);
                            if let Some(doc) = &sig.doc {
                                ui.label(
                                    egui::RichText::new(doc)
                                        .font(small.clone())
                                        .color(palette.gutter_fg),
                                );
                            }
                        }
                    });
            });
    }

    fn context_menu(&mut self, response: &egui::Response, extra: &[MenuEntry]) -> Option<usize> {
        let mut clicked = None;
        response.context_menu(|ui| {
            if !extra.is_empty() {
                clicked = menu::show(ui, extra);
                return;
            }
            let has_sel = self.selections.has_selection();
            let ctx = ui.ctx().clone();
            if ui
                .add_enabled(self.buffer.can_undo(), egui::Button::new(t!("undo")))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(self.buffer.can_redo(), egui::Button::new(t!("redo")))
                .clicked()
            {
                self.redo();
            }
            ui.separator();
            if ui
                .add_enabled(has_sel, egui::Button::new(t!("cut")))
                .clicked()
                && let Some(t) = self.cut(&ctx)
            {
                ctx.copy_text(t);
            }
            if ui
                .add_enabled(has_sel, egui::Button::new(t!("copy")))
                .clicked()
                && let Some(t) = self.selected_text()
            {
                ctx.copy_text(t);
            }
            if ui.button(t!("paste")).clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
            ui.separator();
            if ui.button(t!("select-all")).clicked() {
                self.select_all();
            }
        });
        clicked
    }

    /// Returns true if the text changed.
    fn handle_keyboard(&mut self, ui: &Ui, cache: &mut FrameCache) -> bool {
        let ctx = ui.ctx().clone();
        let events = ui.input(|i| i.events.clone());
        let mut edited = false;
        let mut typed_word = false;
        let mut open_hint = false;
        for event in events {
            match event {
                Event::Text(text) => {
                    if !text.is_empty() && !text.chars().any(|c| c.is_control()) {
                        self.type_text(&ctx, &text);
                        edited = true;
                        open_hint |= text == "(" && self.opts.parameter_hints;
                        typed_word |= text.chars().all(|c| c.is_alphanumeric() || c == '_');
                        if !typed_word {
                            self.completion = None;
                        }
                    }
                }
                Event::Paste(text) => {
                    self.completion = None;
                    self.paste(&ctx, &text);
                    edited = true;
                }
                Event::Copy => {
                    if let Some(t) = self.selected_text() {
                        ctx.copy_text(t);
                    }
                }
                Event::Cut => {
                    if let Some(t) = self.cut(&ctx) {
                        ctx.copy_text(t);
                        edited = true;
                    }
                }
                Event::Ime(ime) => match ime {
                    egui::ImeEvent::Preedit { text, .. } => self.preedit = Some(text),
                    egui::ImeEvent::Commit(text) => {
                        self.preedit = None;
                        if !text.is_empty() {
                            self.insert_text(&ctx, &text);
                            edited = true;
                        }
                    }
                    _ => {}
                },
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    edited |= self.handle_key(&ctx, cache, key, modifiers);
                }
                _ => {}
            }
        }
        if typed_word && (self.opts.completion || self.opts.language_completion) {
            self.update_completion(false);
        } else if typed_word {
            self.completion = None;
        }
        if open_hint {
            self.sync_caches();
            self.update_hint(true);
        }
        edited
    }

    /// Replays a recorded key press (macro playback).
    pub fn replay_key(&mut self, ctx: &egui::Context, key: Key, modifiers: Modifiers) {
        let mut cache = FrameCache::default();
        self.sync_caches();
        self.handle_key(ctx, &mut cache, key, modifiers);
        self.sync_caches();
    }

    fn handle_key(
        &mut self,
        ctx: &egui::Context,
        cache: &mut FrameCache,
        key: Key,
        m: Modifiers,
    ) -> bool {
        let word = m.command || (cfg!(target_os = "macos") && m.alt);
        let extend = m.shift;
        // The completion list takes the navigation keys while it is open.
        if let Some(c) = &mut self.completion {
            let n = c.items.len();
            match key {
                Key::ArrowDown => {
                    c.selected = (c.selected + 1) % n;
                    return false;
                }
                Key::ArrowUp => {
                    c.selected = (c.selected + n - 1) % n;
                    return false;
                }
                Key::PageDown => {
                    c.selected = (c.selected + 8).min(n - 1);
                    return false;
                }
                Key::PageUp => {
                    c.selected = c.selected.saturating_sub(8);
                    return false;
                }
                Key::Enter | Key::Tab if m.is_none() => {
                    self.accept_completion(ctx, None);
                    return true;
                }
                Key::Escape => {
                    self.completion = None;
                    return false;
                }
                Key::Backspace => {}
                _ => self.completion = None,
            }
        }
        // Alt+Shift+arrows grow a rectangular selection; in column mode
        // Shift+arrows do.
        if m.shift && !m.command && (m.alt || self.column_mode) {
            let step = match key {
                Key::ArrowUp => Some((-1, 0)),
                Key::ArrowDown => Some((1, 0)),
                Key::ArrowLeft => Some((0, -1)),
                Key::ArrowRight => Some((0, 1)),
                _ => None,
            };
            if let Some((dr, dc)) = step {
                self.extend_column_selection(cache, dr, dc);
                return false;
            }
        }
        match key {
            Key::Space if m.command && m.shift => {
                self.update_hint(true);
                return false;
            }
            Key::Space if m.command => {
                self.update_completion(true);
                return false;
            }
            Key::Escape if self.hint.is_some() => {
                self.hint = None;
                return false;
            }
            Key::ArrowLeft => self.move_horizontal(false, word, extend),
            Key::ArrowRight => self.move_horizontal(true, word, extend),
            Key::ArrowUp if m.command && m.alt => self.add_caret_vertical(cache, false),
            Key::ArrowDown if m.command && m.alt => self.add_caret_vertical(cache, true),
            Key::ArrowUp => self.move_vertical(cache, -1, extend),
            Key::ArrowDown => self.move_vertical(cache, 1, extend),
            Key::PageUp | Key::PageDown => {
                let rows = if key == Key::PageUp {
                    -(self.page_rows as isize)
                } else {
                    self.page_rows as isize
                };
                self.move_vertical(cache, rows, extend);
                self.pending_scroll_rows += rows;
            }
            Key::Home => self.move_home(extend, m.command),
            Key::End => self.move_end(extend, m.command),
            Key::Escape => self.collapse_selections(),
            Key::A if m.command => self.select_all(),
            Key::Z if m.command && m.shift => {
                self.redo();
                return true;
            }
            Key::Z if m.command => {
                self.undo();
                return true;
            }
            Key::Y if m.command => {
                self.redo();
                return true;
            }
            Key::Backspace => {
                self.delete(ctx, false, word);
                if self.completion.is_some() {
                    self.update_completion(false);
                }
                return true;
            }
            Key::Delete => {
                self.delete(ctx, true, word);
                return true;
            }
            Key::Enter if !m.command && !m.alt => {
                self.newline(ctx);
                return true;
            }
            Key::Tab if !m.command && !m.alt => {
                self.indent(ctx, m.shift);
                return true;
            }
            _ => {}
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_mouse(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        cache: &mut FrameCache,
        cfg: &ViewConfig,
        metrics: Metrics,
        origin: Pos2,
        text_x: f32,
        gutter_right: f32,
        visible_cols: (f32, f32),
        budget: &mut usize,
    ) -> bool {
        let (pressed, pointer, press_origin, modifiers, time) = ui.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.interact_pos(),
                i.pointer.press_origin(),
                i.modifiers,
                i.time,
            )
        });
        let Some(pointer) = pointer else { return false };
        let total_rows = self.rows.total_rows();
        let mut pos_at_pointer = |ed: &mut Editor| -> usize {
            let row = ((pointer.y - origin.y) / metrics.row_h).floor();
            if row < 0.0 {
                return 0;
            }
            let row = row as usize;
            if row >= total_rows {
                return ed.buffer.len_chars();
            }
            let layout = ed.build_row(cache, ui, row, cfg, metrics, visible_cols, budget);
            layout.line_start + layout.offset_at(pointer.x - text_x)
        };

        if response.clicked() || response.drag_started() {
            response.request_focus();
        }
        if pressed && response.hovered() {
            response.request_focus();
            // Use where the button went down, not where the pointer is by now.
            let pointer = press_origin.unwrap_or(pointer);
            let row = ((pointer.y - origin.y) / metrics.row_h).floor();
            // A click in the gutter toggles a fold (in the marker column) or a bookmark.
            if pointer.x < gutter_right {
                if row >= 0.0 && (row as usize) < total_rows {
                    let (line, _) = self.rows.row_to_line(row as usize);
                    if pointer.x < gutter_right - FOLD_MARGIN || !self.toggle_fold(line) {
                        self.toggle_bookmark(line);
                    }
                }
                self.drag_origin = None;
                return false;
            }
            let pos = if row < 0.0 {
                0
            } else if row as usize >= total_rows {
                self.buffer.len_chars()
            } else {
                let layout =
                    self.build_row(cache, ui, row as usize, cfg, metrics, visible_cols, budget);
                layout.line_start + layout.offset_at(pointer.x - text_x)
            };
            // Count rapid clicks in the same place: 1 = caret, 2 = word, 3 = line.
            let same_spot = self.last_click.is_some_and(|(t, p, _)| {
                time - t < MULTI_CLICK_SECS && (p - pointer).length() < 6.0
            });
            let clicks = if same_spot {
                self.last_click.map_or(1, |(_, _, n)| n % 3 + 1)
            } else {
                1
            };
            self.last_click = Some((time, pointer, clicks));

            let target = match clicks {
                2 => {
                    self.drag_mode = DragMode::Word;
                    let (a, b) = self.word_range(pos);
                    Selection::new(a, b)
                }
                3 => {
                    self.drag_mode = DragMode::Line;
                    let line = self.buffer.char_to_line(pos);
                    let start = self.buffer.line_start(line);
                    let end = if line + 1 < self.buffer.len_lines() {
                        self.buffer.line_start(line + 1)
                    } else {
                        self.buffer.len_chars()
                    };
                    Selection::new(start, end)
                }
                _ => {
                    self.drag_mode = DragMode::Char;
                    Selection::caret(pos)
                }
            };
            self.drag_origin = Some(target);
            self.completion = None;
            // Alt+drag (or any drag in column mode) makes a rectangular
            // selection from here.
            let column =
                modifiers.alt || (self.column_mode && !modifiers.command && !modifiers.shift);
            self.column_drag = (column && clicks == 1 && row >= 0.0).then(|| {
                let col = ((pointer.x - text_x) / metrics.char_w).round().max(0.0) as usize;
                (row as usize, col)
            });
            let mut sels = self.selections.clone();
            if (modifiers.alt || modifiers.command) && clicks == 1 {
                sels.push(target, true);
            } else if modifiers.shift && clicks == 1 {
                let p = sels.primary();
                sels = Selections::single(Selection::new(p.anchor, pos));
                self.drag_origin = Some(Selection::caret(p.anchor));
            } else {
                sels = Selections::single(target);
            }
            self.set_selections(sels);
            self.scroll_to_caret = false;
            return false;
        }

        if response.dragged_by(egui::PointerButton::Primary)
            && (modifiers.alt || self.column_mode)
            && let Some(anchor) = self.column_drag
        {
            let row = ((pointer.y - origin.y) / metrics.row_h).floor().max(0.0) as usize;
            let row = row.min(total_rows.saturating_sub(1));
            let col = ((pointer.x - text_x) / metrics.char_w).round().max(0.0) as usize;
            self.set_column_selection(
                cache,
                ColumnSel {
                    anchor,
                    head: (row, col),
                },
            );
            ui.scroll_to_rect(
                Rect::from_center_size(pointer, Vec2::splat(metrics.row_h)),
                None,
            );
        } else if response.dragged_by(egui::PointerButton::Primary)
            && let Some(origin_sel) = self.drag_origin
        {
            let pos = pos_at_pointer(self);
            let sel = match self.drag_mode {
                DragMode::Char => Selection::new(origin_sel.anchor, pos),
                DragMode::Word => {
                    let (a, b) = self.word_range(pos);
                    if pos < origin_sel.start() {
                        Selection::new(origin_sel.end(), a)
                    } else {
                        Selection::new(origin_sel.start(), b.max(origin_sel.end()))
                    }
                }
                DragMode::Line => {
                    let line = self.buffer.char_to_line(pos);
                    if pos < origin_sel.start() {
                        Selection::new(origin_sel.end(), self.buffer.line_start(line))
                    } else {
                        let end = if line + 1 < self.buffer.len_lines() {
                            self.buffer.line_start(line + 1)
                        } else {
                            self.buffer.len_chars()
                        };
                        Selection::new(origin_sel.start(), end.max(origin_sel.end()))
                    }
                }
            };
            let mut sels = self.selections.clone();
            if (modifiers.alt || modifiers.command) && sels.len() > 1 {
                // Ctrl- or Alt-drag edits the newest selection only.
                let primary = sels.primary_index();
                let mut v: Vec<Selection> = sels.iter().copied().collect();
                v[primary] = sel;
                sels = Selections::from_vec(v, primary);
            } else {
                sels = Selections::single(sel);
            }
            self.selections = sels;
            // Scroll when dragging past the edge.
            ui.scroll_to_rect(
                Rect::from_center_size(pointer, Vec2::splat(metrics.row_h)),
                None,
            );
        }
        if response.drag_stopped() {
            self.drag_origin = None;
            self.column_drag = None;
        }
        false
    }
}
