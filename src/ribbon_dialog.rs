//! Settings > Customise Toolbar: pick commands on the left and put them on
//! the toolbar's tabs and groups on the right, by dragging or with the
//! buttons between the lists. Changes apply when the user presses OK or Apply.

use std::sync::Arc;

use egui::{Align2, CornerRadius, Id, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::fonts::icon_font;
use crate::ribbon::{self, Group, Item, Layout, Name, Size, Tab};
use crate::ribbon_ui::IconImage;
use crate::theme::Palette;

/// A command the toolbar can show.
pub struct CommandEntry {
    /// [`crate::actions::command_key`] of the command.
    pub key: String,
    /// Where it is in the menus, like "Edit › Line Operations › Duplicate".
    pub path: String,
    /// The button label it gets by default.
    pub label: String,
    pub icon: IconImage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Sel {
    Tab(usize),
    Group(usize, usize),
    Item(usize, usize, usize),
}

/// What is being dragged.
#[derive(Clone, Debug)]
enum Drag {
    Command(String),
    Node(Sel),
}

pub enum DialogEvent {
    /// Use and save this layout.
    Save(Layout),
}

#[derive(Default)]
pub struct CustomiseDialog {
    pub open: bool,
    draft: Option<Layout>,
    filter: String,
    /// The command picked in the list on the left.
    picked: Option<String>,
    sel: Option<Sel>,
    choosing_icon: bool,
    icon_filter: String,
}

const ROW_H: f32 = 22.0;
const LIST_W: f32 = 300.0;
const TREE_W: f32 = 300.0;
const LIST_H: f32 = 330.0;

impl CustomiseDialog {
    pub fn show(&mut self, layout: &Layout) {
        self.open = true;
        self.draft = Some(layout.clone());
        self.sel = None;
        self.choosing_icon = false;
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        commands: &[CommandEntry],
        palette: &Palette,
    ) -> Vec<DialogEvent> {
        let mut events = Vec::new();
        if !self.open {
            return events;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new(t!("customise-toolbar"))
            .id(Id::new("customise-toolbar"))
            // Anchored at the top, so it grows downwards when the icon picker opens.
            .pivot(Align2::CENTER_TOP)
            .default_pos(ctx.content_rect().center_top() + Vec2::new(0.0, 60.0))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let Some(mut draft) = self.draft.take() else {
                    return;
                };
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(LIST_W);
                        self.command_list(ui, &mut draft, commands, palette);
                    });
                    ui.vertical(|ui| {
                        ui.set_width(110.0);
                        ui.add_space(60.0);
                        self.middle_buttons(ui, &mut draft);
                    });
                    ui.vertical(|ui| {
                        ui.set_width(TREE_W);
                        self.tree(ui, &mut draft, commands, palette);
                    });
                });
                ui.separator();
                self.properties(ui, &mut draft, commands, palette);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(t!("toolbar-reset")).clicked() {
                        draft = Layout::default();
                        self.sel = None;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t!("cancel")).clicked() {
                            close = true;
                        }
                        if ui.button(t!("apply")).clicked() {
                            events.push(DialogEvent::Save(draft.clone()));
                        }
                        if ui.button(t!("ok")).clicked() {
                            events.push(DialogEvent::Save(draft.clone()));
                            close = true;
                        }
                    });
                });
                self.draft = Some(draft);
            });
        drag_preview(ctx, commands, self.draft.as_ref(), palette);
        if !open || close {
            self.open = false;
            self.draft = None;
        }
        events
    }

    fn command_list(
        &mut self,
        ui: &mut Ui,
        draft: &mut Layout,
        commands: &[CommandEntry],
        palette: &Palette,
    ) {
        ui.label(RichText::new(t!("toolbar-commands")).strong());
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .hint_text(t!("toolbar-filter-hint"))
                .desired_width(LIST_W),
        );
        let words: Vec<String> = self
            .filter
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let shown: Vec<&CommandEntry> = commands
            .iter()
            .filter(|c| {
                let path = c.path.to_lowercase();
                words.iter().all(|w| path.contains(w.as_str()))
            })
            .collect();
        egui::Frame::new()
            .stroke(Stroke::new(1.0, palette.border))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("toolbar-commands")
                    .max_height(LIST_H)
                    .min_scrolled_height(LIST_H)
                    .auto_shrink([false, false])
                    .show_rows(ui, ROW_H, shown.len(), |ui, range| {
                        for c in &shown[range] {
                            let picked = self.picked.as_deref() == Some(c.key.as_str());
                            let response = row(ui, 0.0, &c.icon, &c.path, picked, true, palette)
                                .on_hover_text(&c.path);
                            if response.clicked() {
                                self.picked = Some(c.key.clone());
                            }
                            if response.double_clicked() {
                                self.sel = add(draft, self.sel, &c.key);
                            }
                            if response.drag_started() {
                                self.picked = Some(c.key.clone());
                                response.dnd_set_drag_payload(Drag::Command(c.key.clone()));
                            }
                        }
                    });
            });
    }

    fn middle_buttons(&mut self, ui: &mut Ui, draft: &mut Layout) {
        let width = 104.0;
        let can_add = self.picked.is_some() && self.sel.is_some();
        if ui
            .add_enabled(
                can_add,
                egui::Button::new(t!("toolbar-add")).min_size(Vec2::new(width, 0.0)),
            )
            .on_hover_text(t!("toolbar-add-tip"))
            .clicked()
            && let Some(key) = self.picked.clone()
        {
            self.sel = add(draft, self.sel, &key);
        }
        if ui
            .add_enabled(
                self.sel.is_some(),
                egui::Button::new(t!("toolbar-remove")).min_size(Vec2::new(width, 0.0)),
            )
            .clicked()
            && let Some(sel) = self.sel
        {
            self.sel = remove(draft, sel);
        }
        ui.add_space(12.0);
        for (label, up) in [("\u{25B2}", true), ("\u{25BC}", false)] {
            let tip = if up {
                t!("toolbar-move-up")
            } else {
                t!("toolbar-move-down")
            };
            if ui
                .add_enabled(
                    self.sel.is_some(),
                    egui::Button::new(label).min_size(Vec2::new(width, 0.0)),
                )
                .on_hover_text(tip)
                .clicked()
                && let Some(sel) = self.sel
            {
                self.sel = Some(step(draft, sel, up));
            }
        }
        ui.add_space(12.0);
        if ui
            .add(egui::Button::new(t!("toolbar-new-tab")).min_size(Vec2::new(width, 0.0)))
            .clicked()
        {
            self.sel = Some(new_tab(draft, self.sel));
        }
        if ui
            .add_enabled(
                !draft.tabs.is_empty(),
                egui::Button::new(t!("toolbar-new-group")).min_size(Vec2::new(width, 0.0)),
            )
            .clicked()
        {
            self.sel = new_group(draft, self.sel);
        }
    }

    fn tree(
        &mut self,
        ui: &mut Ui,
        draft: &mut Layout,
        commands: &[CommandEntry],
        palette: &Palette,
    ) {
        ui.label(RichText::new(t!("toolbar-layout")).strong());
        ui.label(RichText::new(t!("toolbar-drag-hint")).small().weak());
        let mut dropped: Option<(Drag, Sel)> = None;
        egui::Frame::new()
            .stroke(Stroke::new(1.0, palette.border))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("toolbar-tree")
                    .max_height(LIST_H - 4.0)
                    .min_scrolled_height(LIST_H - 4.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for t in 0..draft.tabs.len() {
                            let tab = &mut draft.tabs[t];
                            let sel = Sel::Tab(t);
                            let name = tab.name.show();
                            let response = ui
                                .horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    ui.checkbox(&mut tab.visible, "")
                                        .on_hover_text(t!("toolbar-tab-visible"));
                                    row(
                                        ui,
                                        0.0,
                                        &IconImage::Glyph(egui_phosphor::regular::TABS),
                                        &name,
                                        self.sel == Some(sel),
                                        true,
                                        palette,
                                    )
                                })
                                .inner;
                            self.tree_row(&response, sel, &mut dropped, palette);
                            for g in 0..draft.tabs[t].groups.len() {
                                let group = &draft.tabs[t].groups[g];
                                let sel = Sel::Group(t, g);
                                let response = row(
                                    ui,
                                    18.0,
                                    &IconImage::Glyph(egui_phosphor::regular::SQUARES_FOUR),
                                    &group.name.show(),
                                    self.sel == Some(sel),
                                    true,
                                    palette,
                                );
                                self.tree_row(&response, sel, &mut dropped, palette);
                                for i in 0..group.items.len() {
                                    let item = &group.items[i];
                                    let sel = Sel::Item(t, g, i);
                                    let entry = commands.iter().find(|c| c.key == item.command);
                                    let (icon, label) = item_look(item, entry);
                                    let response = row(
                                        ui,
                                        36.0,
                                        &icon,
                                        &label,
                                        self.sel == Some(sel),
                                        entry.is_some(),
                                        palette,
                                    );
                                    let response = match entry {
                                        Some(e) => response.on_hover_text(&e.path),
                                        None => response.on_hover_text(t!("toolbar-unavailable")),
                                    };
                                    self.tree_row(&response, sel, &mut dropped, palette);
                                }
                            }
                        }
                        if draft.tabs.is_empty() {
                            ui.label(RichText::new(t!("toolbar-no-tabs")).weak());
                        }
                    });
            });
        if let Some((drag, target)) = dropped {
            self.sel = match drag {
                Drag::Command(key) => add_before(draft, target, &key),
                Drag::Node(from) => move_node(draft, from, target),
            };
        }
    }

    /// Selection, dragging and dropping for one row of the layout.
    fn tree_row(
        &mut self,
        response: &egui::Response,
        sel: Sel,
        dropped: &mut Option<(Drag, Sel)>,
        palette: &Palette,
    ) {
        if response.clicked() {
            self.sel = Some(sel);
            self.choosing_icon = false;
        }
        if response.drag_started() {
            self.sel = Some(sel);
            response.dnd_set_drag_payload(Drag::Node(sel));
        }
        if let Some(payload) = response.dnd_hover_payload::<Drag>() {
            let rect = response.rect;
            let into = drops_into(&payload, sel);
            let painter = response.ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                Id::new("toolbar-drop"),
            ));
            if into {
                painter.rect_stroke(
                    rect,
                    CornerRadius::same(3),
                    Stroke::new(1.5, palette.accent),
                    egui::StrokeKind::Inside,
                );
            } else {
                painter.hline(rect.x_range(), rect.top(), Stroke::new(2.0, palette.accent));
            }
        }
        if let Some(payload) = response.dnd_release_payload::<Drag>() {
            *dropped = Some((Arc::unwrap_or_clone(payload), sel));
        }
    }

    fn properties(
        &mut self,
        ui: &mut Ui,
        draft: &mut Layout,
        commands: &[CommandEntry],
        palette: &Palette,
    ) {
        ui.set_min_height(44.0);
        match self.sel {
            None => {
                ui.label(RichText::new(t!("toolbar-select-hint")).weak());
            }
            Some(Sel::Tab(t)) => {
                if let Some(tab) = draft.tabs.get_mut(t) {
                    name_field(ui, &t!("toolbar-tab-name"), &mut tab.name);
                }
            }
            Some(Sel::Group(t, g)) => {
                if let Some(group) = draft.tabs.get_mut(t).and_then(|t| t.groups.get_mut(g)) {
                    name_field(ui, &t!("toolbar-group-name"), &mut group.name);
                }
            }
            Some(Sel::Item(t, g, i)) => {
                let Some(item) = draft
                    .tabs
                    .get_mut(t)
                    .and_then(|t| t.groups.get_mut(g))
                    .and_then(|g| g.items.get_mut(i))
                else {
                    return;
                };
                let entry = commands.iter().find(|c| c.key == item.command);
                self.item_properties(ui, item, entry, palette);
            }
        }
    }

    fn item_properties(
        &mut self,
        ui: &mut Ui,
        item: &mut Item,
        entry: Option<&CommandEntry>,
        palette: &Palette,
    ) {
        let default_label = entry.map(|e| e.label.clone()).unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(t!("toolbar-label"));
            let mut text = item.label.clone().unwrap_or_default();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut text)
                        .hint_text(&default_label)
                        .desired_width(200.0),
                )
                .changed()
            {
                item.label = (!text.trim().is_empty()).then_some(text);
            }
            ui.add_space(12.0);
            ui.label(t!("toolbar-size"));
            ui.radio_value(&mut item.size, Size::Large, t!("toolbar-size-large"));
            ui.radio_value(&mut item.size, Size::Small, t!("toolbar-size-small"));
            ui.add_space(12.0);
            ui.label(t!("toolbar-icon"));
            let (icon, _) = item_look(item, entry);
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
            paint(ui, rect.center(), 18.0, &icon, palette.text);
            if response.on_hover_text(t!("toolbar-choose-icon")).clicked()
                || ui.button(t!("toolbar-change-icon")).clicked()
            {
                self.choosing_icon = !self.choosing_icon;
            }
            if item.icon.is_some() && ui.button(t!("toolbar-default-icon")).clicked() {
                item.icon = None;
            }
        });
        if self.choosing_icon {
            self.icon_picker(ui, item, palette);
        }
        if let Some(e) = entry {
            ui.label(RichText::new(&e.path).small().weak());
        } else {
            ui.label(RichText::new(t!("toolbar-unavailable")).small().weak());
        }
    }

    fn icon_picker(&mut self, ui: &mut Ui, item: &mut Item, palette: &Palette) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.icon_filter)
                    .hint_text(t!("toolbar-icon-filter-hint"))
                    .desired_width(200.0),
            );
            ui.label(RichText::new(t!("toolbar-icon-credit")).small().weak());
        });
        let filter = self.icon_filter.trim().to_lowercase();
        let icons: Vec<(String, &'static str)> = ribbon::icon_names()
            .into_iter()
            .filter(|(n, _)| filter.is_empty() || n.contains(&filter))
            .collect();
        let cell = 30.0;
        let per_row = ((ui.available_width() - 16.0) / cell).floor().max(1.0) as usize;
        let rows = icons.len().div_ceil(per_row);
        ScrollArea::vertical()
            .id_salt("toolbar-icons")
            .max_height(cell * 3.0)
            .auto_shrink([false, true])
            .show_rows(ui, cell, rows, |ui, range| {
                for r in range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for (name, glyph) in icons.iter().skip(r * per_row).take(per_row) {
                            let (rect, response) =
                                ui.allocate_exact_size(Vec2::splat(cell), Sense::click());
                            let chosen = item.icon.as_deref() == Some(name.as_str());
                            if chosen {
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(4),
                                    palette.accent.gamma_multiply(0.3),
                                );
                            } else if response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(4),
                                    palette.tab_hover_bg,
                                );
                            }
                            ui.painter().text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                *glyph,
                                icon_font(18.0),
                                palette.text,
                            );
                            if response.on_hover_text(name).clicked() {
                                item.icon = Some(name.clone());
                            }
                        }
                    });
                }
            });
    }
}

fn name_field(ui: &mut Ui, label: &str, name: &mut Name) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut text = name.show();
        if ui
            .add(egui::TextEdit::singleline(&mut text).desired_width(240.0))
            .changed()
        {
            *name = Name::user(&text);
        }
    });
}

/// The icon and label an item shows in the dialog.
fn item_look(item: &Item, entry: Option<&CommandEntry>) -> (IconImage, String) {
    let icon = match item.icon.as_deref().and_then(ribbon::glyph) {
        Some(g) => IconImage::Glyph(g),
        None => entry
            .map(|e| e.icon.clone())
            .unwrap_or(IconImage::Glyph(ribbon::GENERIC_ICON)),
    };
    let label = match (&item.label, entry) {
        (Some(l), _) => l.clone(),
        (None, Some(e)) => e.label.clone(),
        (None, None) => unavailable_name(&item.command),
    };
    (icon, label)
}

/// A readable name for a command that isn't available now (a plugin that
/// isn't running, or a command from another version).
fn unavailable_name(key: &str) -> String {
    match serde_json::from_str::<crate::actions::Action>(key) {
        Ok(crate::actions::Action::PluginCommand { plugin, command }) => {
            format!("{plugin}: {command}")
        }
        _ => key.trim_matches('"').to_owned(),
    }
}

fn paint(ui: &Ui, center: Pos2, size: f32, icon: &IconImage, colour: egui::Color32) {
    match icon {
        IconImage::Glyph(g) => {
            ui.painter()
                .text(center, Align2::CENTER_CENTER, *g, icon_font(size), colour);
        }
        IconImage::Texture(tex) => {
            ui.painter().image(
                tex.id(),
                Rect::from_center_size(center, Vec2::splat(size)),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }
}

/// One row of a list: icon and text, indented, full width.
fn row(
    ui: &mut Ui,
    indent: f32,
    icon: &IconImage,
    text: &str,
    selected: bool,
    available: bool,
    palette: &Palette,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click_and_drag());
    if selected {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(3),
            palette.accent.gamma_multiply(0.3),
        );
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(3), palette.tab_hover_bg);
    }
    let colour = if available {
        palette.text
    } else {
        palette.text_dim.gamma_multiply(0.7)
    };
    let x = rect.left() + indent + 6.0;
    paint(ui, Pos2::new(x + 8.0, rect.center().y), 15.0, icon, colour);
    let galley = ui.painter().layout(
        text.to_owned(),
        egui::FontId::proportional(13.0),
        colour,
        f32::INFINITY,
    );
    let clip = Rect::from_min_max(Pos2::new(x + 22.0, rect.top()), rect.max);
    ui.painter().with_clip_rect(clip).galley(
        Pos2::new(x + 22.0, rect.center().y - galley.size().y / 2.0),
        galley,
        colour,
    );
    response
}

/// Shows what is being dragged next to the pointer.
fn drag_preview(
    ctx: &egui::Context,
    commands: &[CommandEntry],
    draft: Option<&Layout>,
    palette: &Palette,
) {
    let Some(payload) = egui::DragAndDrop::payload::<Drag>(ctx) else {
        return;
    };
    let Some(pos) = ctx.pointer_interact_pos() else {
        return;
    };
    let text = match &*payload {
        Drag::Command(key) => commands
            .iter()
            .find(|c| &c.key == key)
            .map(|c| c.label.clone())
            .unwrap_or_default(),
        Drag::Node(sel) => draft
            .map(|d| node_name(d, *sel, commands))
            .unwrap_or_default(),
    };
    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        Id::new("toolbar-drag"),
    ));
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(13.0), palette.text);
    let rect = Rect::from_min_size(pos + Vec2::new(14.0, 6.0), galley.size() + Vec2::splat(8.0));
    painter.rect_filled(rect, CornerRadius::same(4), palette.chrome_bg);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, palette.accent),
        egui::StrokeKind::Inside,
    );
    painter.galley(rect.min + Vec2::splat(4.0), galley, palette.text);
}

fn node_name(layout: &Layout, sel: Sel, commands: &[CommandEntry]) -> String {
    match sel {
        Sel::Tab(t) => layout.tabs.get(t).map(|t| t.name.show()),
        Sel::Group(t, g) => layout
            .tabs
            .get(t)
            .and_then(|t| t.groups.get(g))
            .map(|g| g.name.show()),
        Sel::Item(t, g, i) => layout
            .tabs
            .get(t)
            .and_then(|t| t.groups.get(g))
            .and_then(|g| g.items.get(i))
            .map(|item| item_look(item, commands.iter().find(|c| c.key == item.command)).1),
    }
    .unwrap_or_default()
}

// -------------------------------------------------------------------------
// Changing the layout. These keep indexes in range and return what to select.

fn new_group_named() -> Group {
    Group {
        name: Name::user(&t!("toolbar-new-group-name")),
        items: Vec::new(),
    }
}

/// Whether dropping `drag` on the row `target` puts it inside that row
/// (a button into a group) rather than before it.
fn drops_into(drag: &Drag, target: Sel) -> bool {
    matches!(
        (drag, target),
        (
            Drag::Command(_) | Drag::Node(Sel::Item(..)),
            Sel::Tab(_) | Sel::Group(..)
        ) | (Drag::Node(Sel::Group(..)), Sel::Tab(_))
    )
}

/// Adds a command after the selection; returns the new button's place.
fn add(layout: &mut Layout, sel: Option<Sel>, key: &str) -> Option<Sel> {
    let (t, g, i) = match sel? {
        Sel::Item(t, g, i) => (t, g, i + 1),
        Sel::Group(t, g) => (t, g, layout.tabs.get(t)?.groups.get(g)?.items.len()),
        Sel::Tab(t) => {
            let tab = layout.tabs.get_mut(t)?;
            if tab.groups.is_empty() {
                tab.groups.push(new_group_named());
            }
            let g = tab.groups.len() - 1;
            (t, g, tab.groups[g].items.len())
        }
    };
    insert_item(layout, (t, g, i), new_item(key))
}

fn new_item(key: &str) -> Item {
    Item {
        command: key.to_owned(),
        label: None,
        icon: None,
        size: Size::Small,
    }
}

/// Where a button dropped on `target` goes: (tab, group, index).
fn item_slot(layout: &mut Layout, target: Sel) -> Option<(usize, usize, usize)> {
    Some(match target {
        Sel::Item(t, g, i) => (t, g, i),
        Sel::Group(t, g) => (t, g, layout.tabs.get(t)?.groups.get(g)?.items.len()),
        Sel::Tab(t) => {
            let tab = layout.tabs.get_mut(t)?;
            if tab.groups.is_empty() {
                tab.groups.push(new_group_named());
            }
            let g = tab.groups.len() - 1;
            (t, g, tab.groups[g].items.len())
        }
    })
}

fn add_before(layout: &mut Layout, target: Sel, key: &str) -> Option<Sel> {
    let slot = item_slot(layout, target)?;
    insert_item(layout, slot, new_item(key))
}

fn insert_item(layout: &mut Layout, (t, g, i): (usize, usize, usize), item: Item) -> Option<Sel> {
    let items = &mut layout.tabs.get_mut(t)?.groups.get_mut(g)?.items;
    let i = i.min(items.len());
    items.insert(i, item);
    Some(Sel::Item(t, g, i))
}

fn remove(layout: &mut Layout, sel: Sel) -> Option<Sel> {
    match sel {
        Sel::Item(t, g, i) => {
            let items = &mut layout.tabs.get_mut(t)?.groups.get_mut(g)?.items;
            if i < items.len() {
                items.remove(i);
            }
            if items.is_empty() {
                Some(Sel::Group(t, g))
            } else {
                Some(Sel::Item(t, g, i.min(items.len() - 1)))
            }
        }
        Sel::Group(t, g) => {
            let groups = &mut layout.tabs.get_mut(t)?.groups;
            if g < groups.len() {
                groups.remove(g);
            }
            if groups.is_empty() {
                Some(Sel::Tab(t))
            } else {
                Some(Sel::Group(t, g.min(groups.len() - 1)))
            }
        }
        Sel::Tab(t) => {
            if t < layout.tabs.len() {
                layout.tabs.remove(t);
            }
            (!layout.tabs.is_empty()).then(|| Sel::Tab(t.min(layout.tabs.len() - 1)))
        }
    }
}

/// Moves the selection one place up or down. A button at the end of its
/// group moves into the next group.
fn step(layout: &mut Layout, sel: Sel, up: bool) -> Sel {
    match sel {
        Sel::Tab(t) => {
            let to = if up { t.checked_sub(1) } else { Some(t + 1) };
            match to {
                Some(to) if to < layout.tabs.len() => {
                    layout.tabs.swap(t, to);
                    Sel::Tab(to)
                }
                _ => sel,
            }
        }
        Sel::Group(t, g) => {
            let groups = &mut layout.tabs[t].groups;
            let to = if up { g.checked_sub(1) } else { Some(g + 1) };
            match to {
                Some(to) if to < groups.len() => {
                    groups.swap(g, to);
                    Sel::Group(t, to)
                }
                _ => sel,
            }
        }
        Sel::Item(t, g, i) => {
            let groups = &mut layout.tabs[t].groups;
            let len = groups[g].items.len();
            if up && i > 0 {
                groups[g].items.swap(i, i - 1);
                Sel::Item(t, g, i - 1)
            } else if !up && i + 1 < len {
                groups[g].items.swap(i, i + 1);
                Sel::Item(t, g, i + 1)
            } else if up && g > 0 {
                let item = groups[g].items.remove(i);
                groups[g - 1].items.push(item);
                Sel::Item(t, g - 1, groups[g - 1].items.len() - 1)
            } else if !up && g + 1 < groups.len() {
                let item = groups[g].items.remove(i);
                groups[g + 1].items.insert(0, item);
                Sel::Item(t, g + 1, 0)
            } else {
                sel
            }
        }
    }
}

fn new_tab(layout: &mut Layout, sel: Option<Sel>) -> Sel {
    let at = match sel {
        Some(Sel::Tab(t) | Sel::Group(t, _) | Sel::Item(t, _, _)) => t + 1,
        None => layout.tabs.len(),
    }
    .min(layout.tabs.len());
    layout.tabs.insert(
        at,
        Tab {
            name: Name::user(&t!("toolbar-new-tab-name")),
            visible: true,
            groups: vec![new_group_named()],
        },
    );
    Sel::Tab(at)
}

fn new_group(layout: &mut Layout, sel: Option<Sel>) -> Option<Sel> {
    let (t, at) = match sel {
        Some(Sel::Group(t, g) | Sel::Item(t, g, _)) => (t, g + 1),
        Some(Sel::Tab(t)) => (t, layout.tabs.get(t)?.groups.len()),
        None => (0, layout.tabs.first()?.groups.len()),
    };
    let groups = &mut layout.tabs.get_mut(t)?.groups;
    let at = at.min(groups.len());
    groups.insert(at, new_group_named());
    Some(Sel::Group(t, at))
}

/// Moves a tab, group or button to where it was dropped.
fn move_node(layout: &mut Layout, from: Sel, target: Sel) -> Option<Sel> {
    match from {
        Sel::Item(ft, fg, fi) => {
            let (mut t, mut g, mut i) = item_slot(layout, target)?;
            let items = &mut layout.tabs.get_mut(ft)?.groups.get_mut(fg)?.items;
            if fi >= items.len() {
                return None;
            }
            let item = items.remove(fi);
            if (t, g) == (ft, fg) && fi < i {
                i -= 1;
            }
            if layout.tabs.get(t).and_then(|x| x.groups.get(g)).is_none() {
                (t, g, i) = (ft, fg, fi);
            }
            insert_item(layout, (t, g, i), item)
        }
        Sel::Group(ft, fg) => {
            let (t, mut g) = match target {
                Sel::Tab(t) => (t, layout.tabs.get(t)?.groups.len()),
                Sel::Group(t, g) | Sel::Item(t, g, _) => (t, g),
            };
            let groups = &mut layout.tabs.get_mut(ft)?.groups;
            if fg >= groups.len() {
                return None;
            }
            let group = groups.remove(fg);
            if t == ft && fg < g {
                g -= 1;
            }
            let groups = &mut layout.tabs.get_mut(t)?.groups;
            let g = g.min(groups.len());
            groups.insert(g, group);
            Some(Sel::Group(t, g))
        }
        Sel::Tab(ft) => {
            let (Sel::Tab(mut t) | Sel::Group(mut t, _) | Sel::Item(mut t, _, _)) = target;
            if ft >= layout.tabs.len() {
                return None;
            }
            let tab = layout.tabs.remove(ft);
            if ft < t {
                t -= 1;
            }
            let t = t.min(layout.tabs.len());
            layout.tabs.insert(t, tab);
            Some(Sel::Tab(t))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(layout: &Layout, t: usize, g: usize) -> Vec<String> {
        layout.tabs[t].groups[g]
            .items
            .iter()
            .map(|i| i.command.clone())
            .collect()
    }

    fn sample() -> Layout {
        let item = |k: &str| new_item(k);
        Layout {
            version: 1,
            tabs: vec![
                Tab {
                    name: Name::user("A"),
                    visible: true,
                    groups: vec![
                        Group {
                            name: Name::user("a1"),
                            items: vec![item("1"), item("2"), item("3")],
                        },
                        Group {
                            name: Name::user("a2"),
                            items: vec![item("4")],
                        },
                    ],
                },
                Tab {
                    name: Name::user("B"),
                    visible: true,
                    groups: vec![],
                },
            ],
        }
    }

    #[test]
    fn adds_after_the_selection() {
        let mut l = sample();
        assert_eq!(
            add(&mut l, Some(Sel::Item(0, 0, 0)), "x"),
            Some(Sel::Item(0, 0, 1))
        );
        assert_eq!(keys(&l, 0, 0), ["1", "x", "2", "3"]);
        assert_eq!(
            add(&mut l, Some(Sel::Group(0, 1)), "y"),
            Some(Sel::Item(0, 1, 1))
        );
        // A tab without groups gets one.
        assert_eq!(
            add(&mut l, Some(Sel::Tab(1)), "z"),
            Some(Sel::Item(1, 0, 0))
        );
        assert_eq!(keys(&l, 1, 0), ["z"]);
        assert_eq!(add(&mut l, None, "w"), None);
    }

    #[test]
    fn moves_buttons_within_and_between_groups() {
        let mut l = sample();
        // Drop "1" on "3": it goes before "3".
        assert_eq!(
            move_node(&mut l, Sel::Item(0, 0, 0), Sel::Item(0, 0, 2)),
            Some(Sel::Item(0, 0, 1))
        );
        assert_eq!(keys(&l, 0, 0), ["2", "1", "3"]);
        // Drop "3" on the second group: it goes to its end.
        move_node(&mut l, Sel::Item(0, 0, 2), Sel::Group(0, 1));
        assert_eq!(keys(&l, 0, 1), ["4", "3"]);
        // Step down from the end of a group moves into the next one.
        let mut l = sample();
        assert_eq!(step(&mut l, Sel::Item(0, 0, 2), false), Sel::Item(0, 1, 0));
        assert_eq!(keys(&l, 0, 1), ["3", "4"]);
    }

    #[test]
    fn moves_groups_and_tabs() {
        let mut l = sample();
        assert_eq!(
            move_node(&mut l, Sel::Group(0, 0), Sel::Tab(1)),
            Some(Sel::Group(1, 0))
        );
        assert_eq!(l.tabs[0].groups.len(), 1);
        assert_eq!(keys(&l, 1, 0), ["1", "2", "3"]);
        assert_eq!(
            move_node(&mut l, Sel::Tab(1), Sel::Tab(0)),
            Some(Sel::Tab(0))
        );
        assert_eq!(l.tabs[0].name.show(), "B");
    }

    #[test]
    fn removing_keeps_a_sensible_selection() {
        let mut l = sample();
        assert_eq!(remove(&mut l, Sel::Item(0, 0, 2)), Some(Sel::Item(0, 0, 1)));
        assert_eq!(remove(&mut l, Sel::Item(0, 1, 0)), Some(Sel::Group(0, 1)));
        assert_eq!(remove(&mut l, Sel::Group(0, 1)), Some(Sel::Group(0, 0)));
        assert_eq!(remove(&mut l, Sel::Tab(1)), Some(Sel::Tab(0)));
        assert_eq!(remove(&mut l, Sel::Tab(0)), None);
    }

    #[test]
    fn new_tabs_and_groups_go_after_the_selection() {
        let mut l = sample();
        assert_eq!(new_tab(&mut l, Some(Sel::Item(0, 0, 0))), Sel::Tab(1));
        assert_eq!(l.tabs.len(), 3);
        assert_eq!(l.tabs[1].groups.len(), 1);
        assert_eq!(
            new_group(&mut l, Some(Sel::Group(0, 0))),
            Some(Sel::Group(0, 1))
        );
        assert_eq!(l.tabs[0].groups.len(), 3);
    }
}
