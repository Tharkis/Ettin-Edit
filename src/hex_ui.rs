//! Drawing the hex view and turning keys and clicks into byte edits.

use egui::{
    Align2, Event, EventFilter, FontId, Id, Key, Pos2, Rect, ScrollArea, Sense, Stroke, Ui, Vec2,
};

use crate::encoding::TextEncoding;
use crate::hex::{HexView, ROW, format_hex, parse_hex};
use crate::theme::Palette;

/// Character columns: the offset, then the hex bytes (with a gap after the
/// eighth), then the bytes as text.
const HEX_COL: f32 = 10.0;
const TEXT_COL: f32 = HEX_COL + ROW as f32 * 3.0 + 3.0;
const WIDTH_COLS: f32 = TEXT_COL + ROW as f32 + 1.0;

/// What the user asked the hex view for.
#[derive(Debug, PartialEq)]
pub enum HexOutcome {
    /// New bytes for the whole file, to turn back into text.
    Bytes(Vec<u8>),
    Undo,
    Redo,
}

fn hex_x(i: usize) -> f32 {
    HEX_COL + i as f32 * 3.0 + if i >= ROW / 2 { 1.0 } else { 0.0 }
}

fn printable(b: u8) -> Option<char> {
    (0x20..0x7F).contains(&b).then_some(b as char)
}

impl HexView {
    /// Draws the view. `encoding` is the tab's, for typing in the text column.
    #[allow(clippy::too_many_arguments)]
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        id: Id,
        font_size: f32,
        palette: &Palette,
        encoding: TextEncoding,
        read_only: bool,
        request_focus: bool,
    ) -> Option<HexOutcome> {
        let font = FontId::monospace(font_size);
        let small = FontId::proportional(12.0);
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0')).max(1.0);
        let row_h = ui.fonts_mut(|f| f.row_height(&font)).ceil() + 2.0;
        let full = ui.max_rect();
        let bar_h = 24.0;
        let header = Rect::from_min_size(full.min, Vec2::new(full.width(), row_h + 4.0));
        let body = Rect::from_min_max(
            Pos2::new(full.left(), header.bottom()),
            Pos2::new(full.right(), full.bottom() - bar_h),
        );
        let bar = Rect::from_min_max(Pos2::new(full.left(), body.bottom()), full.max);
        let pad = 8.0;
        let x0 = full.left() + pad;

        // Column numbers.
        let painter = ui.painter_at(full);
        painter.rect_filled(header, 0.0, palette.chrome_bg);
        for i in 0..ROW {
            painter.text(
                Pos2::new(x0 + hex_x(i) * char_w, header.top() + 2.0),
                Align2::LEFT_TOP,
                format!("{i:02X}"),
                font.clone(),
                palette.text_dim,
            );
        }
        painter.text(
            Pos2::new(x0 + TEXT_COL * char_w, header.top() + 2.0),
            Align2::LEFT_TOP,
            "0123456789ABCDEF",
            font.clone(),
            palette.text_dim,
        );

        let response = ui.interact(body, id, Sense::click_and_drag());
        if request_focus || response.clicked() || response.drag_started() {
            response.request_focus();
        }
        let focused = response.has_focus();
        let mut outcome = None;
        if focused {
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
            let page = ((body.height() / row_h) as usize).max(1) * ROW;
            outcome = self.keys(ui, page, encoding, read_only);
        }

        // Rows.
        let rows = self.bytes.len() / ROW + 1;
        let mut area = ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false]);
        if std::mem::take(&mut self.scroll_to_cursor) {
            let top = (self.cursor / ROW) as f32 * row_h;
            let (offset, height) = self.scroll;
            if top < offset {
                area = area.vertical_scroll_offset(top);
            } else if top + row_h > offset + height {
                area = area.vertical_scroll_offset(top + row_h - height);
            }
        }
        let selection = self.selection();
        let mut clicked: Option<(usize, bool)> = None;
        let pointer = response.interact_pointer_pos();
        let output = ui
            .scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                area.show_rows(ui, row_h, rows, |ui, range| {
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(
                            ui.available_width().max(WIDTH_COLS * char_w + 2.0 * pad),
                            row_h * range.len() as f32,
                        ),
                        Sense::hover(),
                    );
                    let painter = ui.painter();
                    for (n, row) in range.clone().enumerate() {
                        let y = rect.top() + n as f32 * row_h + 1.0;
                        let start = row * ROW;
                        painter.text(
                            Pos2::new(x0, y),
                            Align2::LEFT_TOP,
                            format!("{start:08X}"),
                            font.clone(),
                            palette.gutter_fg,
                        );
                        for i in 0..ROW {
                            let at = start + i;
                            if at > self.bytes.len() {
                                break;
                            }
                            let hx = x0 + hex_x(i) * char_w;
                            let tx = x0 + (TEXT_COL + i as f32) * char_w;
                            let hex_cell = Rect::from_min_size(
                                Pos2::new(hx - char_w * 0.25, y - 1.0),
                                Vec2::new(char_w * 2.5, row_h),
                            );
                            let text_cell = Rect::from_min_size(
                                Pos2::new(tx, y - 1.0),
                                Vec2::new(char_w, row_h),
                            );
                            if selection.contains(&at) {
                                painter.rect_filled(hex_cell, 2.0, palette.selection);
                                painter.rect_filled(text_cell, 0.0, palette.selection);
                            }
                            if at == self.cursor {
                                let (main, other) = if self.in_text {
                                    (text_cell, hex_cell)
                                } else {
                                    (hex_cell, text_cell)
                                };
                                let color = if focused {
                                    palette.accent
                                } else {
                                    palette.text_dim
                                };
                                painter.rect_stroke(
                                    main,
                                    2.0,
                                    Stroke::new(1.5, color),
                                    egui::StrokeKind::Inside,
                                );
                                painter.rect_stroke(
                                    other,
                                    2.0,
                                    Stroke::new(1.0, color.gamma_multiply(0.5)),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            let Some(&b) = self.bytes.get(at) else {
                                continue;
                            };
                            let (hex, color) = match self.pending {
                                Some(d) if at == self.cursor => (format!("{d:X}_"), palette.accent),
                                _ if b == 0 => ("00".to_owned(), palette.text_dim),
                                _ => (format!("{b:02X}"), palette.editor_fg),
                            };
                            painter.text(
                                Pos2::new(hx, y),
                                Align2::LEFT_TOP,
                                hex,
                                font.clone(),
                                color,
                            );
                            let (c, color) = match printable(b) {
                                Some(c) => (c, palette.editor_fg),
                                None => ('.', palette.whitespace),
                            };
                            painter.text(
                                Pos2::new(tx, y),
                                Align2::LEFT_TOP,
                                c,
                                font.clone(),
                                color,
                            );
                        }
                    }
                    // Which byte the pointer is on.
                    if let Some(p) = pointer.filter(|p| body.contains(*p) || response.dragged()) {
                        let row =
                            range.start as isize + ((p.y - rect.top()) / row_h).floor() as isize;
                        let col = (p.x - x0) / char_w;
                        let (i, text) = if col >= TEXT_COL - 1.0 {
                            ((col - TEXT_COL).floor().max(0.0) as usize, true)
                        } else {
                            let i = (0..ROW).rev().find(|&i| col >= hex_x(i) - 0.5).unwrap_or(0);
                            (i, false)
                        };
                        let at = row.max(0) as usize * ROW + i.min(ROW - 1);
                        clicked = Some((at, text));
                    }
                })
            })
            .inner;
        self.scroll = (output.state.offset.y, output.inner_rect.height());
        if let Some((at, text)) = clicked {
            if response.drag_started() || response.clicked() {
                let extend = ui.input(|i| i.modifiers.shift);
                self.in_text = text;
                self.move_to(at, extend);
                self.scroll_to_cursor = false;
            } else if response.dragged() {
                self.move_to(at, true);
            }
        }

        // The status line.
        let painter = ui.painter_at(full);
        painter.rect_filled(bar, 0.0, palette.chrome_bg);
        let mut info = vec![t!(
            "hex-offset",
            hex = format!("{:X}", self.cursor),
            dec = self.cursor
        )];
        if let Some(&b) = self.bytes.get(self.cursor) {
            info.push(format!("0x{b:02X} = {b} = 0b{b:08b}"));
        }
        if !selection.is_empty() {
            info.push(t!("hex-selected", count = selection.len()));
        }
        info.push(t!("hex-size", count = self.bytes.len()));
        let text = info.join("    ");
        painter.text(
            Pos2::new(bar.left() + pad, bar.center().y),
            Align2::LEFT_CENTER,
            text,
            small.clone(),
            palette.text_dim,
        );
        if let Some(m) = &self.message {
            painter.text(
                Pos2::new(bar.right() - pad, bar.center().y),
                Align2::RIGHT_CENTER,
                m,
                small,
                palette.modified,
            );
        }
        outcome
    }

    /// Handles this frame's keys. `page` is how many bytes fit on screen.
    fn keys(
        &mut self,
        ui: &Ui,
        page: usize,
        encoding: TextEncoding,
        read_only: bool,
    ) -> Option<HexOutcome> {
        let mut outcome = None;
        self.before_edit = None;
        let events = ui.input(|i| i.events.clone());
        for event in events {
            let len = self.bytes.len();
            match event {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers: m,
                    ..
                } => {
                    let c = self.cursor;
                    let row_start = c - c % ROW;
                    let to = match key {
                        Key::ArrowLeft => Some(c.saturating_sub(1)),
                        Key::ArrowRight => Some(c + 1),
                        Key::ArrowUp => Some(c.saturating_sub(ROW)),
                        Key::ArrowDown => Some((c + ROW).min(len)),
                        Key::PageUp => Some(c.saturating_sub(page)),
                        Key::PageDown => Some((c + page).min(len)),
                        Key::Home if m.command => Some(0),
                        Key::Home => Some(row_start),
                        Key::End if m.command => Some(len),
                        Key::End => Some((row_start + ROW - 1).min(len)),
                        _ => None,
                    };
                    if let Some(to) = to {
                        self.move_to(to, m.shift);
                        continue;
                    }
                    match key {
                        Key::Tab => {
                            self.in_text = !self.in_text;
                            self.pending = None;
                        }
                        Key::Z if m.command && m.shift => outcome = Some(HexOutcome::Redo),
                        Key::Z if m.command => outcome = Some(HexOutcome::Undo),
                        Key::Y if m.command => outcome = Some(HexOutcome::Redo),
                        Key::Escape => {
                            self.pending = None;
                            self.anchor = self.cursor;
                        }
                        Key::Delete | Key::Backspace | Key::Insert if read_only => {
                            self.message = Some(t!("hex-read-only"));
                        }
                        Key::Delete | Key::Backspace => {
                            let sel = self.selection();
                            let range = if !sel.is_empty() {
                                sel
                            } else if key == Key::Delete {
                                c..(c + 1).min(len)
                            } else {
                                c.saturating_sub(1)..c
                            };
                            if !range.is_empty() {
                                outcome = Some(self.edit(range.clone(), &[], range.start));
                            }
                        }
                        Key::Insert => {
                            outcome = Some(self.edit(c..c, &[0], c));
                        }
                        _ => {}
                    }
                }
                Event::Text(text) if read_only => {
                    let _ = text;
                    self.message = Some(t!("hex-read-only"));
                }
                Event::Text(text) => {
                    for ch in text.chars() {
                        if self.in_text {
                            let bom = encoding.encode("").map(|b| b.len()).unwrap_or(0);
                            match encoding.encode(&ch.to_string()) {
                                Ok(bytes) => {
                                    let bytes = &bytes[bom..];
                                    let c = self.cursor;
                                    let end = (c + bytes.len()).min(self.bytes.len());
                                    outcome = Some(self.edit(c..end, bytes, c + bytes.len()));
                                }
                                Err(_) => {
                                    self.message = Some(t!(
                                        "hex-cant-encode",
                                        char = ch.to_string(),
                                        encoding = encoding.name()
                                    ));
                                }
                            }
                        } else if let Some(d) = ch.to_digit(16) {
                            let d = d as u8;
                            match self.pending.take() {
                                None => self.pending = Some(d),
                                Some(high) => {
                                    let c = self.cursor;
                                    let end = (c + 1).min(self.bytes.len());
                                    outcome = Some(self.edit(c..end, &[high << 4 | d], c + 1));
                                }
                            }
                        }
                    }
                }
                Event::Copy | Event::Cut => {
                    let sel = self.selection();
                    let range = if sel.is_empty() {
                        self.cursor..(self.cursor + 1).min(len)
                    } else {
                        sel
                    };
                    ui.ctx().copy_text(format_hex(&self.bytes[range]));
                }
                Event::Paste(text) if read_only => {
                    let _ = text;
                    self.message = Some(t!("hex-read-only"));
                }
                Event::Paste(text) => match parse_hex(&text) {
                    Some(bytes) => {
                        let c = self.cursor;
                        let sel = self.selection();
                        let range = if sel.is_empty() {
                            c..(c + bytes.len()).min(len)
                        } else {
                            sel
                        };
                        let after = range.start + bytes.len();
                        outcome = Some(self.edit(range, &bytes, after));
                    }
                    None => self.message = Some(t!("hex-paste-hint")),
                },
                _ => {}
            }
        }
        outcome
    }

    /// Replaces `range` with `with` and puts the cursor at `cursor`.
    fn edit(&mut self, range: std::ops::Range<usize>, with: &[u8], cursor: usize) -> HexOutcome {
        let bytes = self.replaced(range, with);
        if self.before_edit.is_none() {
            self.before_edit = Some((self.cursor, self.anchor));
        }
        self.bytes = std::sync::Arc::new(bytes.clone());
        self.message = None;
        self.move_to(cursor, false);
        HexOutcome::Bytes(bytes)
    }
}
