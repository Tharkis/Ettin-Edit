//! Panels that plugins describe in JSON: text, buttons, form fields, a tree
//! or a table. Ettin Edit draws them; clicks and changes go back to the plugin.

use std::collections::{BTreeMap, HashMap};

use egui::{Align, Frame, Layout, Margin, RichText, ScrollArea, Sense, Ui};
use egui_extras::{Column, TableBuilder};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::theme::Palette;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    #[default]
    Right,
    Bottom,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Item {
    Heading {
        text: String,
    },
    Label {
        text: String,
        #[serde(default)]
        weak: bool,
        #[serde(default)]
        mono: bool,
    },
    Separator,
    Button {
        id: String,
        label: String,
    },
    Input {
        id: String,
        #[serde(default)]
        label: String,
        #[serde(default)]
        value: String,
        #[serde(default)]
        hint: String,
        #[serde(default)]
        multiline: bool,
    },
    Checkbox {
        id: String,
        label: String,
        #[serde(default)]
        value: bool,
    },
    Select {
        id: String,
        #[serde(default)]
        label: String,
        options: Vec<String>,
        #[serde(default)]
        value: String,
    },
    Row {
        items: Vec<Item>,
    },
    Tree {
        id: String,
        nodes: Vec<Node>,
        /// A node id to highlight, open and scroll to.
        #[serde(default)]
        selected: Option<String>,
    },
    Table {
        id: String,
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct Node {
    /// Sent back when the node is clicked; the label when empty.
    #[serde(default)]
    pub id: String,
    pub label: String,
    /// Shown dimmed after the label.
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub children: Vec<Node>,
    #[serde(default)]
    pub open: Option<bool>,
}

impl Node {
    fn key(&self) -> &str {
        if self.id.is_empty() {
            &self.label
        } else {
            &self.id
        }
    }
}

/// Something the user did in a panel.
#[derive(Debug, PartialEq)]
pub struct PanelEvent {
    /// The item's id; empty for the panel itself (closing it).
    pub item: String,
    /// `click`, `change`, `submit`, `select` or `close`.
    pub kind: &'static str,
    pub value: Value,
}

pub struct Panel {
    pub plugin: String,
    pub id: String,
    pub title: String,
    pub position: Position,
    items: Vec<Item>,
    /// Current values of inputs, checkboxes and selects.
    values: BTreeMap<String, Value>,
    /// The values the plugin last sent, so a refresh that doesn't change an
    /// item keeps what the user typed.
    sent: BTreeMap<String, Value>,
    /// Selected tree node or table row, per item.
    selected: HashMap<String, String>,
    /// Table sort, per item: (column, ascending).
    sort: HashMap<String, (usize, bool)>,
    /// Trees whose selected node the plugin just changed: show it next frame.
    reveal: HashMap<String, String>,
    /// Scroll back to the top next frame.
    to_top: bool,
}

impl Panel {
    pub fn new(plugin: &str, id: &str) -> Self {
        Self {
            plugin: plugin.to_owned(),
            id: id.to_owned(),
            title: String::new(),
            position: Position::Right,
            items: Vec::new(),
            values: BTreeMap::new(),
            sent: BTreeMap::new(),
            selected: HashMap::new(),
            sort: HashMap::new(),
            reveal: HashMap::new(),
            to_top: false,
        }
    }

    /// Replaces the panel's contents.
    pub fn set_items(&mut self, items: Vec<Item>) {
        let mut sent = BTreeMap::new();
        collect_values(&items, &mut sent);
        for (id, value) in &sent {
            if self.sent.get(id) != Some(value) || !self.values.contains_key(id) {
                self.values.insert(id.clone(), value.clone());
            }
        }
        self.values.retain(|id, _| sent.contains_key(id));
        self.sent = sent;
        reveal_selected(&items, &mut self.selected, &mut self.reveal);
        self.items = items;
    }

    pub fn scroll_to_top(&mut self) {
        self.to_top = true;
    }

    /// Every input, checkbox and select value, by id.
    pub fn values(&self) -> Value {
        Value::Object(self.values.clone().into_iter().collect())
    }

    /// Draws the panel; returns what the user did. `closed` is set when the
    /// panel's close button was clicked.
    pub fn ui(&mut self, ui: &mut Ui, palette: &Palette) -> Vec<PanelEvent> {
        let mut events = Vec::new();
        ui.horizontal(|ui| {
            ui.label(RichText::new(&self.title).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("×").frame(false))
                    .on_hover_text(t!("close-panel"))
                    .clicked()
                {
                    events.push(PanelEvent {
                        item: String::new(),
                        kind: "close",
                        value: Value::Null,
                    });
                }
            });
        });
        ui.separator();
        let items = std::mem::take(&mut self.items);
        let salt = format!("plugin-panel-{}-{}", self.plugin, self.id);
        let mut scroll = ScrollArea::vertical()
            .id_salt(&salt)
            .auto_shrink([false, false]);
        if std::mem::take(&mut self.to_top) {
            scroll = scroll.vertical_scroll_offset(0.0);
        }
        scroll.show(ui, |ui| {
            for item in &items {
                self.item_ui(ui, item, palette, &mut events);
            }
        });
        self.items = items;
        events
    }

    fn item_ui(
        &mut self,
        ui: &mut Ui,
        item: &Item,
        palette: &Palette,
        events: &mut Vec<PanelEvent>,
    ) {
        match item {
            Item::Heading { text } => {
                ui.add_space(4.0);
                ui.label(RichText::new(text).strong().size(14.0));
            }
            Item::Label { text, weak, mono } => {
                let mut rich = RichText::new(text);
                if *weak {
                    rich = rich.color(palette.text_dim);
                }
                if *mono {
                    rich = rich.monospace();
                }
                ui.add(egui::Label::new(rich).wrap().selectable(true));
            }
            Item::Separator => {
                ui.separator();
            }
            Item::Button { id, label } => {
                if ui.button(label).clicked() {
                    events.push(PanelEvent {
                        item: id.clone(),
                        kind: "click",
                        value: Value::Null,
                    });
                }
            }
            Item::Input {
                id,
                label,
                hint,
                multiline,
                ..
            } => {
                let mut text = self
                    .values
                    .get(id)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let mut submit = false;
                let mut field = |ui: &mut Ui| {
                    let edit = if *multiline {
                        egui::TextEdit::multiline(&mut text).desired_rows(3)
                    } else {
                        egui::TextEdit::singleline(&mut text)
                    };
                    let resp = ui.add(
                        edit.hint_text(hint.as_str())
                            .id_salt((&self.plugin, &self.id, id))
                            .desired_width(f32::INFINITY),
                    );
                    if !multiline
                        && resp.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        submit = true;
                    }
                };
                if label.is_empty() {
                    field(ui);
                } else {
                    ui.label(label);
                    field(ui);
                }
                self.values.insert(id.clone(), json!(text));
                if submit {
                    events.push(PanelEvent {
                        item: id.clone(),
                        kind: "submit",
                        value: json!(text),
                    });
                }
            }
            Item::Checkbox { id, label, .. } => {
                let mut on = self
                    .values
                    .get(id)
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if ui.checkbox(&mut on, label).changed() {
                    self.values.insert(id.clone(), json!(on));
                    events.push(PanelEvent {
                        item: id.clone(),
                        kind: "change",
                        value: json!(on),
                    });
                }
            }
            Item::Select {
                id, label, options, ..
            } => {
                let current = self
                    .values
                    .get(id)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let mut picked = None;
                ui.horizontal(|ui| {
                    if !label.is_empty() {
                        ui.label(label);
                    }
                    egui::ComboBox::from_id_salt((&self.plugin, &self.id, id))
                        .selected_text(&current)
                        .show_ui(ui, |ui| {
                            for option in options {
                                if ui.selectable_label(*option == current, option).clicked()
                                    && *option != current
                                {
                                    picked = Some(option.clone());
                                }
                            }
                        });
                });
                if let Some(option) = picked {
                    self.values.insert(id.clone(), json!(option));
                    events.push(PanelEvent {
                        item: id.clone(),
                        kind: "change",
                        value: json!(option),
                    });
                }
            }
            Item::Row { items } => {
                ui.horizontal_wrapped(|ui| {
                    for item in items {
                        self.item_ui(ui, item, palette, events);
                    }
                });
            }
            Item::Tree { id, nodes, .. } => {
                let selected = self.selected.get(id).cloned();
                let reveal = self
                    .reveal
                    .remove(id)
                    .and_then(|key| find_node(nodes, &key));
                let mut clicked = None;
                Frame::new().inner_margin(Margin::same(2)).show(ui, |ui| {
                    for (i, node) in nodes.iter().enumerate() {
                        let path = format!("{}/{}/{id}/{i}", self.plugin, self.id);
                        let reveal = reveal
                            .as_deref()
                            .filter(|r| r.first() == Some(&i))
                            .map(|r| &r[1..]);
                        let mut tree = Tree {
                            selected: selected.as_deref(),
                            palette,
                            clicked: &mut clicked,
                        };
                        tree.node(ui, node, &path, reveal);
                    }
                });
                if let Some(key) = clicked {
                    self.selected.insert(id.clone(), key.clone());
                    events.push(PanelEvent {
                        item: id.clone(),
                        kind: "select",
                        value: json!(key),
                    });
                }
            }
            Item::Table { id, columns, rows } => {
                if let Some(row) = self.table_ui(ui, id, columns, rows) {
                    self.selected.insert(id.clone(), row.to_string());
                    events.push(PanelEvent {
                        item: id.clone(),
                        kind: "select",
                        value: json!(row),
                    });
                }
            }
        }
    }

    /// Draws a table; returns the index (in the plugin's order) of a clicked row.
    fn table_ui(
        &mut self,
        ui: &mut Ui,
        id: &str,
        columns: &[String],
        rows: &[Vec<Value>],
    ) -> Option<usize> {
        if columns.is_empty() {
            return None;
        }
        let mut order: Vec<usize> = (0..rows.len()).collect();
        let sort = self.sort.get(id).copied();
        if let Some((col, asc)) = sort {
            order.sort_by(|&a, &b| {
                let ord = compare_cells(rows[a].get(col), rows[b].get(col));
                if asc { ord } else { ord.reverse() }
            });
        }
        let selected: Option<usize> = self.selected.get(id).and_then(|s| s.parse().ok());
        let row_h = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
        let font = egui::TextStyle::Body.resolve(ui.style());
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0')).max(1.0);
        // Start each column wide enough for its name and first values.
        let widths: Vec<f32> = (0..columns.len())
            .map(|c| {
                let longest = rows
                    .iter()
                    .take(200)
                    .map(|row| cell_text(row.get(c)).chars().count())
                    .max()
                    .unwrap_or(0)
                    .max(columns[c].chars().count() + 2);
                (longest as f32 * char_w + 16.0).clamp(40.0, 320.0)
            })
            .collect();
        let mut clicked_header = None;
        let mut clicked_row = None;
        ui.push_id((&self.plugin, &self.id, id), |ui| {
            // No scrolling of its own: the panel scrolls.
            let mut table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .vscroll(false)
                .sense(Sense::click())
                .cell_layout(Layout::left_to_right(Align::Center));
            for (c, w) in widths.iter().enumerate() {
                let col = if c + 1 == columns.len() {
                    Column::remainder().at_least(*w)
                } else {
                    Column::initial(*w).at_least(30.0)
                };
                table = table.column(col.clip(true));
            }
            table
                .header(row_h, |mut header| {
                    for (c, name) in columns.iter().enumerate() {
                        header.col(|ui| {
                            let arrow = match sort {
                                Some((s, true)) if s == c => " ▲",
                                Some((s, false)) if s == c => " ▼",
                                _ => "",
                            };
                            let resp = ui.add(
                                egui::Label::new(RichText::new(format!("{name}{arrow}")).strong())
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
                        for c in 0..columns.len() {
                            let value = rows[i].get(c);
                            let (_, resp) = row.col(|ui| {
                                let text = cell_text(value);
                                if value.is_some_and(Value::is_number) {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.add_space(8.0);
                                        ui.add(egui::Label::new(text).truncate().selectable(false));
                                    });
                                } else {
                                    ui.add(egui::Label::new(text).truncate().selectable(false));
                                }
                            });
                            if resp.clicked() {
                                clicked_row = Some(i);
                            }
                        }
                        if row.response().clicked() {
                            clicked_row = Some(i);
                        }
                    });
                });
        });
        if let Some(c) = clicked_header {
            let asc = !matches!(sort, Some((s, true)) if s == c);
            self.sort.insert(id.to_owned(), (c, asc));
        }
        clicked_row
    }
}

/// What drawing a tree needs at every level.
struct Tree<'a> {
    selected: Option<&'a str>,
    palette: &'a Palette,
    clicked: &'a mut Option<String>,
}

impl Tree<'_> {
    /// Draws a node and its children. `reveal` is the path, below this node,
    /// to a node to open and scroll to; empty when it is this node.
    fn node(&mut self, ui: &mut Ui, node: &Node, path: &str, reveal: Option<&[usize]>) {
        let key = node.key();
        let palette = self.palette;
        let selected = self.selected;
        let clicked = &mut *self.clicked;
        let mut label = |ui: &mut Ui| {
            let mut job = egui::text::LayoutJob::default();
            let font = egui::TextStyle::Body.resolve(ui.style());
            job.append(
                &node.label,
                0.0,
                egui::TextFormat::simple(font.clone(), ui.visuals().text_color()),
            );
            if !node.detail.is_empty() {
                job.append(
                    &node.detail,
                    8.0,
                    egui::TextFormat::simple(font, palette.text_dim),
                );
            }
            // Long labels end in … at the panel's edge; hovering shows all of it.
            // (Laid out here: a button would lay a job out again without the limit.)
            let padding = ui.spacing().button_padding.x * 2.0 + 2.0;
            job.wrap = egui::text::TextWrapping::truncate_at_width(ui.available_width() - padding);
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            let mut response = ui.selectable_label(selected == Some(key), galley);
            if node.label.chars().count() + node.detail.chars().count() > 40 {
                response = response.on_hover_text(if node.detail.is_empty() {
                    node.label.clone()
                } else {
                    format!("{}\n{}", node.label, node.detail)
                });
            }
            if reveal.is_some_and(<[usize]>::is_empty) {
                response.scroll_to_me(Some(Align::Center));
            }
            if response.clicked() {
                *clicked = Some(key.to_owned());
            }
        };
        if node.children.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().icon_width + ui.spacing().icon_spacing);
                label(ui);
            });
            return;
        }
        let id = ui.make_persistent_id(path);
        let open = node.open.unwrap_or(false);
        let mut state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open);
        if reveal.is_some_and(|r| !r.is_empty()) {
            state.set_open(true);
        }
        state.show_header(ui, label).body(|ui| {
            for (i, child) in node.children.iter().enumerate() {
                let reveal = reveal.filter(|r| r.first() == Some(&i)).map(|r| &r[1..]);
                self.node(ui, child, &format!("{path}/{i}"), reveal);
            }
        });
    }
}

/// The indexes leading to the node with this key, from the top level.
fn find_node(nodes: &[Node], key: &str) -> Option<Vec<usize>> {
    for (i, node) in nodes.iter().enumerate() {
        if node.key() == key {
            return Some(vec![i]);
        }
        if let Some(mut rest) = find_node(&node.children, key) {
            rest.insert(0, i);
            return Some(rest);
        }
    }
    None
}

/// Takes each tree's `selected` from the plugin, and marks it to be shown
/// when it changed.
fn reveal_selected(
    items: &[Item],
    selected: &mut HashMap<String, String>,
    reveal: &mut HashMap<String, String>,
) {
    for item in items {
        match item {
            Item::Tree {
                id,
                selected: Some(key),
                ..
            } => {
                if selected.get(id) != Some(key) {
                    selected.insert(id.clone(), key.clone());
                    reveal.insert(id.clone(), key.clone());
                }
            }
            Item::Row { items } => reveal_selected(items, selected, reveal),
            _ => {}
        }
    }
}

fn collect_values(items: &[Item], out: &mut BTreeMap<String, Value>) {
    for item in items {
        match item {
            Item::Input { id, value, .. } | Item::Select { id, value, .. } => {
                out.insert(id.clone(), json!(value));
            }
            Item::Checkbox { id, value, .. } => {
                out.insert(id.clone(), json!(value));
            }
            Item::Row { items } => collect_values(items, out),
            _ => {}
        }
    }
}

fn cell_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}

/// Numbers sort as numbers, before text; text compares case-insensitively.
fn compare_cells(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a.and_then(Value::as_f64), b.and_then(Value::as_f64)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => cell_text(a)
            .to_lowercase()
            .cmp(&cell_text(b).to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(v: Value) -> Vec<Item> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn parses_every_item_type() {
        let parsed = items(json!([
            {"type": "heading", "text": "Stats"},
            {"type": "label", "text": "hi", "weak": true},
            {"type": "separator"},
            {"type": "button", "id": "go", "label": "Go"},
            {"type": "input", "id": "q", "label": "Query"},
            {"type": "checkbox", "id": "c", "label": "Case", "value": true},
            {"type": "select", "id": "s", "options": ["a", "b"], "value": "b"},
            {"type": "row", "items": [{"type": "button", "id": "x", "label": "X"}]},
            {"type": "tree", "id": "t", "nodes": [{"label": "root", "children": [{"id": "k", "label": "leaf"}]}]},
            {"type": "table", "id": "g", "columns": ["a", "b"], "rows": [[1, "x"], [2, null]]}
        ]));
        assert_eq!(parsed.len(), 10);
        assert!(serde_json::from_value::<Vec<Item>>(json!([{"type": "nope"}])).is_err());
    }

    #[test]
    fn refresh_keeps_values_the_user_changed() {
        let mut p = Panel::new("p", "main");
        p.set_items(items(json!([{"type": "input", "id": "q", "value": "a"}])));
        assert_eq!(p.values(), json!({"q": "a"}));
        // The user types; the plugin refreshes with the same value.
        p.values.insert("q".into(), json!("typed"));
        p.set_items(items(json!([{"type": "input", "id": "q", "value": "a"}])));
        assert_eq!(p.values(), json!({"q": "typed"}));
        // The plugin sets a new value: it wins.
        p.set_items(items(json!([{"type": "input", "id": "q", "value": "b"}])));
        assert_eq!(p.values(), json!({"q": "b"}));
        // Items that are gone lose their values.
        p.set_items(items(
            json!([{"type": "checkbox", "id": "c", "label": "C"}]),
        ));
        assert_eq!(p.values(), json!({"c": false}));
    }

    #[test]
    fn cells_sort_numbers_first() {
        let (one, two, a) = (json!(1), json!(2.5), json!("a"));
        assert!(compare_cells(Some(&one), Some(&two)).is_lt());
        assert!(compare_cells(Some(&two), Some(&a)).is_lt());
        assert!(compare_cells(Some(&json!("B")), Some(&a)).is_gt());
        assert_eq!(cell_text(Some(&json!(null))), "");
        assert_eq!(cell_text(Some(&json!(true))), "true");
    }
}
