//! Drawing the editor and turning keyboard and mouse input into commands.
//!
//! Only the rows inside the scroll viewport are laid out, so the cost of a
//! frame depends on the window size, not the file size.

use std::collections::HashMap;
use std::sync::Arc;

use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{
    Align2, CursorIcon, Event, EventFilter, FontId, Galley, Id, Key, Modifiers, Pos2, Rect,
    ScrollArea, Sense, Stroke, Ui, Vec2,
};
use syntect::highlighting::Theme;
use syntect::parsing::{SyntaxReference, SyntaxSet};

use super::highlight::LineSpans;
use super::layout::{char_cols, wrap_points};
use super::selection::{Selection, Selections};
use super::{DragMode, Editor};
use crate::theme::Palette;

/// Lines longer than this (in chars) are drawn a window at a time when not wrapping.
const CLIP_LINE_CHARS: usize = 4000;
/// How many lines the highlighter may parse per frame before showing text unstyled.
const HIGHLIGHT_BUDGET: usize = 3000;
const TEXT_PAD: f32 = 6.0;
const MULTI_CLICK_SECS: f64 = 0.45;

pub struct ViewConfig<'a> {
    pub font: FontId,
    pub wrap: bool,
    pub show_line_numbers: bool,
    pub highlight_current_line: bool,
    pub palette: &'a Palette,
    pub syntax: &'a SyntaxReference,
    pub syntaxes: &'a SyntaxSet,
    pub theme: &'a Theme,
    pub theme_name: &'a str,
    pub plain: bool,
    pub request_focus: bool,
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

/// Font metrics for the editor grid.
#[derive(Clone, Copy)]
struct Metrics {
    row_h: f32,
    char_w: f32,
}

impl Editor {
    /// Applies pending edits to the row index and highlighter.
    fn sync_caches(&mut self) {
        let edits = self.buffer.take_line_edits();
        if let Some(first) = edits.iter().map(|e| e.start_line).min() {
            self.rows.apply_edits(&self.buffer, &edits);
            self.highlighter.invalidate_from(first);
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
        let flush = |job: &mut LayoutJob, text: &mut String, fmt: &Option<TextFormat>| {
            if !text.is_empty() {
                job.append(text, 0.0, fmt.clone().unwrap_or_else(|| base.clone()));
                text.clear();
            }
        };
        for &c in &chars[start..end] {
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
            if run_format.as_ref() != Some(&format) {
                flush(&mut job, &mut text, &run_format);
                run_format = Some(format);
            }
            let w = char_cols(c, col);
            match c {
                '\t' => {
                    for _ in 0..w {
                        text.push(' ');
                    }
                    disp_len += w;
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
        });
        cache.rows.insert(row, layout.clone());
        layout
    }

    /// Draws the editor and handles its input. Call once per frame.
    pub fn show(&mut self, ui: &mut Ui, id: Id, cfg: &ViewConfig) {
        let palette = cfg.palette;
        let metrics = Metrics {
            row_h: ui.fonts_mut(|f| f.row_height(&cfg.font)),
            char_w: ui.fonts_mut(|f| f.glyph_width(&cfg.font, '0')).max(1.0),
        };
        let number_font = FontId::monospace(cfg.font.size * 0.88);
        let digits = self.buffer.len_lines().max(999).to_string().len() as f32;
        let gutter_w = if cfg.show_line_numbers {
            digits * ui.fonts_mut(|f| f.glyph_width(&number_font, '0')) + 28.0
        } else {
            10.0
        };
        let scrollbar = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_outer_margin + 4.0;
        let text_w =
            (ui.available_width() - gutter_w - TEXT_PAD - scrollbar).max(metrics.char_w * 10.0);
        let wrap_cols = cfg
            .wrap
            .then(|| ((text_w / metrics.char_w).floor() as usize).max(10));

        self.highlighter
            .configure(cfg.syntax, cfg.theme_name, cfg.theme, cfg.plain);
        self.sync_caches();
        self.rows.set_wrap(&self.buffer, wrap_cols);

        let mut cache = FrameCache::default();
        let mut changed = false;
        let has_focus = ui.memory(|m| m.has_focus(id));
        if has_focus {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                )
            });
            changed |= self.handle_keyboard(ui, &mut cache);
        }
        self.sync_caches();
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
        let scroll = ScrollArea::both()
            .id_salt(id.with("scroll"))
            .auto_shrink(false)
            .show_viewport(ui, |ui, viewport| {
                let size = Vec2::new(
                    content_w.max(viewport.width()),
                    content_h.max(viewport.height()),
                );
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                let response = ui
                    .interact(rect, id, Sense::click_and_drag())
                    .on_hover_cursor(CursorIcon::Text);
                let origin = rect.min;
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
                            self.build_row(
                                &mut cache,
                                ui,
                                r,
                                cfg,
                                metrics,
                                visible_cols,
                                &mut budget,
                            ),
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

                // Text.
                for (r, row) in &rows {
                    painter.galley(
                        Pos2::new(text_x + row.x_offset, row_y(*r)),
                        row.galley.clone(),
                        palette.editor_fg,
                    );
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
                            Pos2::new(gutter.right() - 14.0, row_y(*r) + metrics.row_h / 2.0),
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
                    let layout = self.build_row(
                        &mut cache,
                        ui,
                        row,
                        cfg,
                        metrics,
                        visible_cols,
                        &mut budget,
                    );
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

                self.context_menu(&response);
            });
        let _ = scroll;
        if needs_repaint {
            ui.ctx().request_repaint();
        }
    }

    fn context_menu(&mut self, response: &egui::Response) {
        response.context_menu(|ui| {
            let has_sel = self.selections.has_selection();
            let ctx = ui.ctx().clone();
            if ui
                .add_enabled(self.buffer.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(self.buffer.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.redo();
            }
            ui.separator();
            if ui.add_enabled(has_sel, egui::Button::new("Cut")).clicked()
                && let Some(t) = self.cut(&ctx)
            {
                ctx.copy_text(t);
            }
            if ui.add_enabled(has_sel, egui::Button::new("Copy")).clicked()
                && let Some(t) = self.selected_text()
            {
                ctx.copy_text(t);
            }
            if ui.button("Paste").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
            ui.separator();
            if ui.button("Select All").clicked() {
                self.select_all();
            }
        });
    }

    /// Returns true if the text changed.
    fn handle_keyboard(&mut self, ui: &Ui, cache: &mut FrameCache) -> bool {
        let ctx = ui.ctx().clone();
        let events = ui.input(|i| i.events.clone());
        let mut edited = false;
        for event in events {
            match event {
                Event::Text(text) => {
                    if !text.is_empty() && !text.chars().any(|c| c.is_control()) {
                        self.insert_text(&ctx, &text);
                        edited = true;
                    }
                }
                Event::Paste(text) => {
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
        edited
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
        match key {
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
            let mut sels = self.selections.clone();
            if modifiers.alt && clicks == 1 {
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
            if modifiers.alt && sels.len() > 1 {
                // Alt-drag edits the newest caret only.
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
        }
        false
    }
}
