//! A pop-up list to pick from by typing: the command palette (F1) and the
//! tab list.

use egui::{Id, Key, RichText};

use crate::theme::Palette;

pub struct Item<T> {
    pub label: String,
    /// Shown dim and smaller after the label, such as the menu a command
    /// is in.
    pub context: String,
    /// Shown dimmed on the right, such as a shortcut or a folder.
    pub detail: String,
    /// A word shown before the detail, such as "recent".
    pub tag: String,
    /// Greyed out and sorted last when false; can't be picked.
    pub enabled: bool,
    /// Its place among the commands used recently (0 is the latest).
    pub recent: Option<usize>,
    pub value: T,
}

impl<T> Item<T> {
    pub fn new(label: String, detail: String, value: T) -> Self {
        Self {
            label,
            context: String::new(),
            detail,
            tag: String::new(),
            enabled: true,
            recent: None,
            value,
        }
    }
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

/// Every word of the query appears in the text (in any order, any case).
fn matches(text: &str, query: &str) -> bool {
    let text = text.to_lowercase();
    query
        .split_whitespace()
        .all(|w| text.contains(&w.to_lowercase()))
}

/// How well an item matches, best first: used recently, the name starts
/// with the query, a word in the name starts with it, the name contains it,
/// and last only the menu path (or shortcut) matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    Recent,
    Starts,
    WordStarts,
    Contains,
    Elsewhere,
}

fn tier(label: &str, recent: bool, query: &str) -> Tier {
    if recent {
        return Tier::Recent;
    }
    let q = query.trim().to_lowercase();
    let name = label.to_lowercase();
    if q.is_empty() || name.starts_with(&q) {
        return Tier::Starts;
    }
    let word_start = name
        .match_indices(&q)
        .any(|(i, _)| name[..i].ends_with(|c: char| !c.is_alphanumeric()));
    if word_start {
        Tier::WordStarts
    } else if name.contains(&q) || matches(&name, &q) {
        Tier::Contains
    } else {
        Tier::Elsewhere
    }
}

/// The items that match `query`, best first. Items that can't run now go
/// below the ones that can; within a tier recent ones go by how recent,
/// then shorter names (closer matches) first, then list order.
fn ranked<'a, T>(items: &'a [Item<T>], query: &str) -> Vec<&'a Item<T>> {
    let mut shown: Vec<(usize, &Item<T>)> = items
        .iter()
        .enumerate()
        .filter(|(_, i)| matches(&format!("{} {} {}", i.label, i.context, i.detail), query))
        .collect();
    shown.sort_by_key(|(n, i)| {
        let t = tier(&i.label, i.recent.is_some(), query);
        let tie = if query.trim().is_empty() {
            0
        } else {
            i.label.chars().count()
        };
        (!i.enabled, t, i.recent.unwrap_or(usize::MAX), tie, *n)
    });
    shown.into_iter().map(|(_, i)| i).collect()
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
        let shown = ranked(items, &self.query);
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
            picked = shown
                .get(self.selected)
                .filter(|i| i.enabled)
                .map(|i| i.value.clone());
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
                                ui.label(RichText::new(t!("nothing-matches")).weak());
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
                                let text = if item.enabled {
                                    palette.text
                                } else {
                                    palette.text_dim
                                };
                                let right = if item.tag.is_empty() {
                                    item.detail.clone()
                                } else if item.detail.is_empty() {
                                    item.tag.clone()
                                } else {
                                    format!("{}    {}", item.tag, item.detail)
                                };
                                let detail = ui.painter().layout_no_wrap(
                                    right,
                                    egui::FontId::proportional(12.0),
                                    palette.text_dim,
                                );
                                let label_w = rect.width() - detail.size().x - 24.0;
                                let mut job = egui::text::LayoutJob::default();
                                job.append(
                                    &item.label,
                                    0.0,
                                    egui::TextFormat::simple(
                                        egui::FontId::proportional(14.0),
                                        text,
                                    ),
                                );
                                if !item.context.is_empty() {
                                    job.append(
                                        &item.context,
                                        8.0,
                                        egui::TextFormat {
                                            valign: egui::Align::Center,
                                            ..egui::TextFormat::simple(
                                                egui::FontId::proportional(11.5),
                                                palette.text_dim,
                                            )
                                        },
                                    );
                                }
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
                                if r.clicked() && item.enabled {
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
    }

    fn item(label: &str, context: &str) -> Item<String> {
        Item {
            context: context.into(),
            ..Item::new(label.into(), String::new(), label.into())
        }
    }

    fn top(items: &[Item<String>], query: &str) -> Vec<String> {
        ranked(items, query)
            .iter()
            .map(|i| i.value.clone())
            .collect()
    }

    /// Roughly what the palette lists for "json" on a plain-text tab.
    fn json_commands() -> Vec<Item<String>> {
        let mut list = vec![
            item("Convert to JSON", "Transform › Convert to"),
            item("Read as JSON", "Transform › Convert to › Read as"),
            item("Format JSON", "Tools › JSON and JavaScript"),
            item("Minify JSON", "Tools › JSON and JavaScript"),
            item("Validate JSON", "Tools › JSON and JavaScript"),
            item("Sort JSON Keys", "Tools › JSON and JavaScript"),
            item("JSON Tree", "Tools › JSON and JavaScript"),
            item("Save", "File"),
            item("Clean API dump", "Recipe · 3 steps"),
        ];
        list[0].enabled = false;
        list
    }

    #[test]
    fn ranks_by_recent_then_how_the_name_matches() {
        let mut items = json_commands();
        let order = top(&items, "json");
        assert!(order[..3].contains(&"Format JSON".to_string()), "{order:?}");
        assert_eq!(order[0], "JSON Tree");
        // Commands that can't run now go last.
        assert_eq!(order.last().unwrap(), "Convert to JSON");
        // Run Minify JSON once: it comes first.
        items[3].recent = Some(0);
        assert_eq!(top(&items, "json")[0], "Minify JSON");
        // Only the context matches: after the names that do.
        let order = top(&items, "tools");
        assert_eq!(order.len(), 5);
    }

    #[test]
    fn tiers() {
        assert_eq!(tier("Duplicate", false, "dup"), Tier::Starts);
        assert_eq!(
            tier("Remove Duplicate Lines", false, "dup"),
            Tier::WordStarts
        );
        assert_eq!(tier("Undup", false, "dup"), Tier::Contains);
        assert_eq!(tier("Lines dup", false, "dup lines"), Tier::Contains);
        assert_eq!(tier("Save", false, "file"), Tier::Elsewhere);
        assert_eq!(tier("Save", true, "file"), Tier::Recent);
    }

    #[test]
    fn empty_query_puts_recent_first() {
        let mut items = json_commands();
        items[7].recent = Some(1);
        items[2].recent = Some(0);
        let order = top(&items, "");
        assert_eq!(order[..2], ["Format JSON".to_string(), "Save".to_string()]);
    }
}
