//! The document map: the whole document drawn small beside the editor, with
//! the visible part marked. Clicking or dragging on it scrolls the editor.

use egui::{Color32, Pos2, Rect, Sense, Ui, Vec2};
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;

use super::Editor;
use super::layout::char_cols;
use crate::theme::Palette;

/// Height of one row in the map, in points.
const ROW_H: f32 = 2.0;
/// Width of one column in the map.
const COL_W: f32 = 1.0;
/// Lines highlighted per frame for the map, beyond what the editor has done.
const MAP_BUDGET: usize = 300;

fn blend(bg: Color32, fg: Color32, t: f32) -> Color32 {
    let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(l(bg.r(), fg.r()), l(bg.g(), fg.g()), l(bg.b(), fg.b()))
}

impl Editor {
    /// Draws the map into `ui`, which should be a narrow side panel.
    pub fn show_map(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        syntaxes: &SyntaxSet,
        theme: &Theme,
    ) {
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, palette.editor_bg);

        let total = self.rows.total_rows().max(1);
        let (top, shown) = self.map_view;
        let map_rows = (rect.height() / ROW_H).floor().max(1.0) as usize;
        // When the whole document doesn't fit, the map scrolls with the editor.
        let scrollable = total.saturating_sub(map_rows) as f32;
        let progress = if total as f32 > shown {
            (top / (total as f32 - shown).max(1.0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let first = (scrollable * progress).round() as usize;

        let mut budget = MAP_BUDGET;
        let mut missing = false;
        let max_cols = ((rect.width() - 6.0) / COL_W).max(1.0) as usize;
        let text_bg = palette.editor_bg;
        let plain = blend(text_bg, palette.editor_fg, 0.55);
        let mut last_line = usize::MAX;
        let mut line_chars: Vec<char> = Vec::new();
        let mut spans = None;
        let mut wraps: Vec<usize> = vec![0];
        for i in 0..map_rows.min(total.saturating_sub(first)) {
            let row = first + i;
            let (line, sub) = self.rows.row_to_line(row);
            if line != last_line {
                last_line = line;
                line_chars = self
                    .buffer
                    .rope()
                    .line(line)
                    .chars()
                    .filter(|&c| c != '\n' && c != '\r')
                    .take(4000)
                    .collect();
                wraps = super::layout::wrap_points(&line_chars, self.rows.wrap_cols());
                spans =
                    self.highlighter
                        .line_spans(line, &self.buffer, syntaxes, theme, &mut budget);
                if spans.is_none() && !self.highlighter.is_plain() {
                    missing = true;
                }
            }
            let start = wraps.get(sub).copied().unwrap_or(0);
            let end = wraps.get(sub + 1).copied().unwrap_or(line_chars.len());
            let y = rect.top() + i as f32 * ROW_H;
            // Byte offsets for matching the highlighter's spans.
            let mut byte = line_chars[..start]
                .iter()
                .map(|c| c.len_utf8())
                .sum::<usize>();
            let mut col = 0usize;
            let mut run: Option<(usize, Color32)> = None;
            let flush = |run: &mut Option<(usize, Color32)>, to: usize| {
                if let Some((from, color)) = run.take()
                    && to > from
                {
                    let x0 = rect.left() + 3.0 + from as f32 * COL_W;
                    let x1 = (rect.left() + 3.0 + to as f32 * COL_W).min(rect.right() - 3.0);
                    if x1 > x0 {
                        painter.rect_filled(
                            Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + ROW_H - 0.5)),
                            0.0,
                            color,
                        );
                    }
                }
            };
            for &c in &line_chars[start..end.min(line_chars.len())] {
                if col >= max_cols {
                    break;
                }
                let w = char_cols(c, col);
                if c.is_whitespace() {
                    flush(&mut run, col);
                } else {
                    let color = spans
                        .as_ref()
                        .and_then(|s| {
                            s.iter()
                                .find(|s| (s.start as usize) <= byte && byte < s.end as usize)
                        })
                        .map(|s| blend(text_bg, s.color, 0.75))
                        .unwrap_or(plain);
                    match run {
                        Some((_, c0)) if c0 == color => {}
                        _ => {
                            flush(&mut run, col);
                            run = Some((col, color));
                        }
                    }
                }
                col += w;
                byte += c.len_utf8();
            }
            flush(&mut run, col);
        }
        if missing {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(60));
        }

        // The part of the document the editor shows.
        let y0 = rect.top() + (top - first as f32) * ROW_H;
        let view = Rect::from_min_size(
            Pos2::new(rect.left(), y0),
            Vec2::new(rect.width(), (shown * ROW_H).max(6.0)),
        );
        painter.rect_filled(
            view,
            0.0,
            Color32::from_rgba_unmultiplied(
                palette.editor_fg.r(),
                palette.editor_fg.g(),
                palette.editor_fg.b(),
                if palette.dark { 22 } else { 30 },
            ),
        );
        painter.rect_stroke(
            view,
            0.0,
            egui::Stroke::new(1.0, palette.guide),
            egui::StrokeKind::Inside,
        );

        if let Some(p) = response.interact_pointer_pos()
            && (response.clicked() || response.dragged())
        {
            let row = first as f32 + (p.y - rect.top()) / ROW_H;
            self.map_scroll = Some((row - shown / 2.0).max(0.0));
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }
}
