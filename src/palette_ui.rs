//! A pop-up list to pick from by typing: the command palette (F1) and the
//! tab list.

use egui::{Id, Key, RichText};

use crate::theme::Palette;

pub struct Item<T> {
    pub label: String,
    /// Shown dimmed on the right, such as a shortcut or a folder.
    pub detail: String,
    pub value: T,
}

#[derive(Default)]
pub struct Picker {
    pub open: bool,
    title: String,
    hint: String,
    query: String,
    selected: usize,
    focus: bool,
}

/// Every word of the query appears in the label (in any order, any case).
fn matches(label: &str, query: &str) -> bool {
    let label = label.to_lowercase();
    query
        .split_whitespace()
        .all(|w| label.contains(&w.to_lowercase()))
}

/// Lower is better: labels whose last part starts with the query come first.
fn rank(label: &str, query: &str) -> usize {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 0;
    }
    let last = label
        .rsplit('›')
        .next()
        .unwrap_or(label)
        .trim()
        .to_lowercase();
    if last.starts_with(&q) {
        0
    } else if last.contains(&q) {
        1
    } else {
        2
    }
}

impl Picker {
    pub fn show(&mut self, title: &str, hint: &str) {
        self.open = true;
        self.title = title.to_owned();
        self.hint = hint.to_owned();
        self.query.clear();
        self.selected = 0;
        self.focus = true;
    }

    /// Draws the list; returns the picked item's value.
    pub fn ui<T: Clone>(
        &mut self,
        ctx: &egui::Context,
        items: &[Item<T>],
        palette: &Palette,
    ) -> Option<T> {
        if !self.open {
            return None;
        }
        let mut shown: Vec<&Item<T>> = items
            .iter()
            .filter(|i| matches(&i.label, &self.query) || matches(&i.detail, &self.query))
            .collect();
        shown.sort_by_key(|i| rank(&i.label, &self.query));
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            self.selected = (self.selected + 1).min(shown.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            self.selected = self.selected.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(Key::PageDown)) {
            self.selected = (self.selected + 10).min(shown.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(Key::PageUp)) {
            self.selected = self.selected.saturating_sub(10);
        }
        self.selected = self.selected.min(shown.len().saturating_sub(1));
        let mut picked = None;
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            picked = shown.get(self.selected).map(|i| i.value.clone());
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.open = false;
            return None;
        }

        let screen = ctx.content_rect();
        let width = (screen.width() * 0.6).clamp(360.0, 640.0);
        let area = egui::Area::new(Id::new("picker"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(
                screen.center().x - width / 2.0,
                screen.top() + 60.0,
            ));
        let response = area.show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.label(RichText::new(&self.title).small().weak());
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text(self.hint.as_str())
                            .desired_width(f32::INFINITY),
                    );
                    if std::mem::take(&mut self.focus) {
                        edit.request_focus();
                    }
                    if edit.changed() {
                        self.selected = 0;
                    }
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(380.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            if shown.is_empty() {
                                ui.label(RichText::new("Nothing matches").weak());
                            }
                            for (i, item) in shown.iter().enumerate() {
                                let selected = i == self.selected;
                                let (rect, r) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 24.0),
                                    egui::Sense::click(),
                                );
                                if selected || r.hovered() {
                                    let fill = if selected {
                                        palette.selection
                                    } else {
                                        palette.tab_hover_bg
                                    };
                                    ui.painter().rect_filled(rect, 4.0, fill);
                                }
                                let font = egui::FontId::proportional(14.0);
                                let detail = ui.painter().layout_no_wrap(
                                    item.detail.clone(),
                                    egui::FontId::proportional(12.0),
                                    palette.text_dim,
                                );
                                let label_w = rect.width() - detail.size().x - 24.0;
                                let mut job = egui::text::LayoutJob::simple_singleline(
                                    item.label.clone(),
                                    font,
                                    palette.text,
                                );
                                job.wrap = egui::text::TextWrapping::truncate_at_width(label_w);
                                let label = ui.fonts_mut(|f| f.layout_job(job));
                                ui.painter().galley(
                                    egui::pos2(
                                        rect.left() + 8.0,
                                        rect.center().y - label.size().y / 2.0,
                                    ),
                                    label,
                                    palette.text,
                                );
                                ui.painter().galley(
                                    egui::pos2(
                                        rect.right() - 8.0 - detail.size().x,
                                        rect.center().y - detail.size().y / 2.0,
                                    ),
                                    detail,
                                    palette.text_dim,
                                );
                                if selected
                                    && ui.input(|i| {
                                        i.key_pressed(Key::ArrowDown)
                                            || i.key_pressed(Key::ArrowUp)
                                            || i.key_pressed(Key::PageDown)
                                            || i.key_pressed(Key::PageUp)
                                    })
                                {
                                    r.scroll_to_me(None);
                                }
                                if r.clicked() {
                                    picked = Some(item.value.clone());
                                }
                            }
                        });
                });
        });
        // A click outside closes it.
        if ctx.input(|i| i.pointer.any_pressed())
            && !response.response.contains_pointer()
            && picked.is_none()
        {
            self.open = false;
        }
        if picked.is_some() {
            self.open = false;
        }
        picked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_every_word_in_any_order() {
        assert!(matches("Edit › Line Operations › Duplicate", "dup line"));
        assert!(matches("Edit › Line Operations › Duplicate", ""));
        assert!(!matches("Edit › Line Operations › Duplicate", "dup xml"));
        assert!(
            rank("Edit › Duplicate", "dup")
                < rank("Edit › Line Operations › Remove Duplicate Lines", "dup")
        );
    }
}
