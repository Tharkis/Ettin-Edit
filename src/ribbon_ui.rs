//! Draws the toolbar: a row of tab names, and below it the selected tab's
//! groups of buttons. Collapsed, only the names show and clicking one opens
//! its buttons over the text until a command runs or the user clicks away.

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Order, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
};
use egui_phosphor::regular as ph;

use crate::actions::Action;
use crate::fonts::icon_font;
use crate::ribbon::Size;
use crate::theme::Palette;

/// Height of a small button; a column holds three.
const SMALL_H: f32 = 24.0;
const ROWS: usize = 3;
const ROW_GAP: f32 = 2.0;
/// Height of the button area of a group.
const CONTENT_H: f32 = SMALL_H * ROWS as f32 + ROW_GAP * (ROWS as f32 - 1.0);
const LARGE_ICON: f32 = 24.0;
const SMALL_ICON: f32 = 16.0;
/// The widest a large button's label wraps to.
const LARGE_LABEL_W: f32 = 72.0;

#[derive(Clone)]
pub enum IconImage {
    Glyph(&'static str),
    /// A plugin's PNG icon.
    Texture(egui::TextureHandle),
}

/// A button ready to draw.
pub struct Button {
    pub action: Action,
    pub label: String,
    pub icon: IconImage,
    pub tooltip: String,
    pub enabled: bool,
    /// `Some` for a command that is turned on or off.
    pub checked: Option<bool>,
    pub size: Size,
    /// Where it is in the layout: tab, group, item.
    pub place: (usize, usize, usize),
}

pub struct GroupView {
    pub name: String,
    pub buttons: Vec<Button>,
}

pub struct TabView {
    /// Its index in the layout (hidden tabs are left out of the views).
    pub index: usize,
    pub name: String,
    pub groups: Vec<GroupView>,
}

pub enum BarEvent {
    Run(Action),
    /// Take a button off the toolbar.
    Remove(usize, usize, usize),
    Customise,
    ToggleCollapsed,
    Hide,
    ToggleMenuBar,
}

#[derive(Default)]
pub struct RibbonBar {
    /// The layout index of the selected tab.
    pub selected: usize,
    /// While collapsed: the selected tab's buttons are showing.
    popup: bool,
    /// The menu bar is showing, for the right-click menu's wording.
    pub menu_bar_shown: bool,
}

impl RibbonBar {
    /// Draws the bar into a top panel.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        tabs: &[TabView],
        collapsed: bool,
        palette: &Palette,
    ) -> Vec<BarEvent> {
        let mut events = Vec::new();
        if !tabs.iter().any(|t| t.index == self.selected)
            && let Some(first) = tabs.first()
        {
            self.selected = first.index;
        }
        if !collapsed {
            self.popup = false;
        }
        let strip = ui
            .horizontal(|ui| self.tab_strip(ui, tabs, collapsed, palette, &mut events))
            .response;
        let current = tabs.iter().find(|t| t.index == self.selected);
        if !collapsed {
            if let Some(tab) = current {
                body(ui, tab, palette, &mut events);
            } else {
                ui.label(egui::RichText::new(t!("toolbar-empty")).weak());
            }
        } else if self.popup
            && let Some(tab) = current
        {
            let area = egui::Area::new(Id::new("toolbar-popup"))
                .order(Order::Foreground)
                .fixed_pos(Pos2::new(strip.rect.left(), strip.rect.bottom() + 2.0))
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(palette.chrome_bg)
                        .show(ui, |ui| {
                            ui.set_max_width(strip.rect.width());
                            body(ui, tab, palette, &mut events)
                        });
                });
            let clicked_away = ui.input(|i| i.pointer.any_pressed())
                && !area.response.contains_pointer()
                && !strip.contains_pointer();
            let ran = events
                .iter()
                .any(|e| !matches!(e, BarEvent::ToggleCollapsed));
            if clicked_away || ran || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.popup = false;
            }
        }
        events
    }

    fn tab_strip(
        &mut self,
        ui: &mut Ui,
        tabs: &[TabView],
        collapsed: bool,
        palette: &Palette,
        events: &mut Vec<BarEvent>,
    ) {
        ui.spacing_mut().item_spacing.x = 2.0;
        for tab in tabs {
            let selected = tab.index == self.selected;
            let font = FontId::proportional(13.0);
            let galley = ui.painter().layout_no_wrap(
                tab.name.clone(),
                font,
                if selected {
                    palette.text
                } else {
                    palette.text_dim
                },
            );
            let size = galley.size() + Vec2::new(20.0, 8.0);
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            if response.hovered() {
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(4), palette.tab_hover_bg);
            }
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, palette.text);
            if selected && (!collapsed || self.popup) {
                let line = Rect::from_min_max(
                    Pos2::new(rect.left() + 6.0, rect.bottom() - 2.0),
                    Pos2::new(rect.right() - 6.0, rect.bottom()),
                );
                ui.painter().rect_filled(line, 1.0, palette.accent);
            }
            if response.double_clicked() {
                events.push(BarEvent::ToggleCollapsed);
            } else if response.clicked() {
                if collapsed {
                    self.popup = !(self.popup && selected);
                }
                self.selected = tab.index;
            }
            strip_menu(&response, collapsed, self.menu_bar_shown, events);
        }
        // The rest of the row: the collapse button at the far right.
        let rest = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(
            Rect::from_min_size(rest.min, Vec2::new(rest.width(), 24.0)),
            Sense::click(),
        );
        strip_menu(&response, collapsed, self.menu_bar_shown, events);
        let button = Rect::from_center_size(
            Pos2::new(rest.right() - 14.0, rest.top() + 12.0),
            Vec2::splat(22.0),
        );
        let b = ui.interact(button, Id::new("toolbar-collapse"), Sense::click());
        if b.hovered() {
            ui.painter()
                .rect_filled(button, CornerRadius::same(4), palette.tab_hover_bg);
        }
        ui.painter().text(
            button.center(),
            Align2::CENTER_CENTER,
            if collapsed {
                ph::CARET_DOWN
            } else {
                ph::CARET_UP
            },
            icon_font(14.0),
            palette.text_dim,
        );
        let tip = if collapsed {
            t!("toolbar-expand")
        } else {
            t!("toolbar-collapse")
        };
        if b.on_hover_text(tip).clicked() {
            events.push(BarEvent::ToggleCollapsed);
        }
    }
}

/// The right-click menu of the tab names.
fn strip_menu(response: &Response, collapsed: bool, menu_bar: bool, events: &mut Vec<BarEvent>) {
    response.context_menu(|ui| {
        if ui.button(t!("customise-toolbar-dots")).clicked() {
            events.push(BarEvent::Customise);
        }
        let label = if collapsed {
            t!("toolbar-expand")
        } else {
            t!("toolbar-collapse")
        };
        if ui.button(label).clicked() {
            events.push(BarEvent::ToggleCollapsed);
        }
        if ui.button(t!("toolbar-hide")).clicked() {
            events.push(BarEvent::Hide);
        }
        let label = if menu_bar {
            t!("menu-bar-hide")
        } else {
            t!("menu-bar-show-again")
        };
        if ui.button(label).clicked() {
            events.push(BarEvent::ToggleMenuBar);
        }
    });
}

/// The selected tab's groups, side by side; scrolls when the window is narrow.
fn body(ui: &mut Ui, tab: &TabView, palette: &Palette, events: &mut Vec<BarEvent>) {
    egui::ScrollArea::horizontal()
        .id_salt(("toolbar-body", tab.index))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, ROW_GAP);
                for (i, group) in tab.groups.iter().enumerate() {
                    if i > 0 {
                        let x = ui.cursor().left() + 3.0;
                        let top = ui.cursor().top() + 4.0;
                        ui.painter().vline(
                            x,
                            top..=top + CONTENT_H + 12.0,
                            Stroke::new(1.0, palette.border),
                        );
                        ui.add_space(8.0);
                    }
                    group_ui(ui, group, palette, events);
                }
                if tab.groups.iter().all(|g| g.buttons.is_empty()) {
                    ui.label(egui::RichText::new(t!("toolbar-empty")).weak());
                }
            });
        });
}

fn group_ui(ui: &mut Ui, group: &GroupView, palette: &Palette, events: &mut Vec<BarEvent>) {
    let name = ui.painter().layout_no_wrap(
        group.name.clone(),
        FontId::proportional(11.0),
        palette.text_dim,
    );
    ui.vertical(|ui| {
        let row = ui.horizontal_top(|ui| {
            ui.set_min_height(CONTENT_H);
            ui.set_min_width(name.size().x + 8.0);
            let mut i = 0;
            while i < group.buttons.len() {
                if group.buttons[i].size == Size::Large {
                    large_button(ui, &group.buttons[i], palette, events);
                    i += 1;
                    continue;
                }
                let column: Vec<&Button> = group.buttons[i..]
                    .iter()
                    .take_while(|b| b.size == Size::Small)
                    .take(ROWS)
                    .collect();
                i += column.len();
                let width = column
                    .iter()
                    .map(|b| small_width(ui, b))
                    .fold(0.0, f32::max);
                ui.vertical(|ui| {
                    for b in column {
                        small_button(ui, b, width, palette, events);
                    }
                });
            }
        });
        let width = row.response.rect.width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 14.0), Sense::hover());
        ui.painter()
            .galley(rect.center() - name.size() / 2.0, name, palette.text_dim);
    });
}

fn text_colour(b: &Button, palette: &Palette) -> Color32 {
    if b.enabled {
        palette.text
    } else {
        palette.text_dim.gamma_multiply(0.6)
    }
}

/// The background of a button: hovered, or turned on.
fn paint_frame(ui: &Ui, rect: Rect, response: &Response, b: &Button, palette: &Palette) {
    let radius = CornerRadius::same(4);
    if b.checked == Some(true) {
        ui.painter()
            .rect_filled(rect, radius, palette.accent.gamma_multiply(0.22));
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, palette.accent.gamma_multiply(0.7)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() && b.enabled {
        ui.painter().rect_filled(rect, radius, palette.tab_hover_bg);
    }
}

fn paint_icon(ui: &Ui, center: Pos2, size: f32, b: &Button, colour: Color32) {
    match &b.icon {
        IconImage::Glyph(g) => {
            ui.painter()
                .text(center, Align2::CENTER_CENTER, *g, icon_font(size), colour);
        }
        IconImage::Texture(tex) => {
            let tint = if b.enabled {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(100)
            };
            ui.painter().image(
                tex.id(),
                Rect::from_center_size(center, Vec2::splat(size)),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
        }
    }
}

fn finish(response: Response, b: &Button, events: &mut Vec<BarEvent>) {
    let response = response.on_hover_text(&b.tooltip);
    if response.clicked() && b.enabled {
        events.push(BarEvent::Run(b.action.clone()));
    }
    response.context_menu(|ui| {
        if ui.button(t!("toolbar-remove-button")).clicked() {
            let (t, g, i) = b.place;
            events.push(BarEvent::Remove(t, g, i));
        }
        if ui.button(t!("customise-toolbar-dots")).clicked() {
            events.push(BarEvent::Customise);
        }
        if ui.button(t!("toolbar-collapse")).clicked() {
            events.push(BarEvent::ToggleCollapsed);
        }
    });
}

fn large_button(ui: &mut Ui, b: &Button, palette: &Palette, events: &mut Vec<BarEvent>) {
    let colour = text_colour(b, palette);
    let mut job = egui::text::LayoutJob::simple(
        b.label.clone(),
        FontId::proportional(12.0),
        colour,
        LARGE_LABEL_W,
    );
    job.halign = egui::Align::Center;
    job.wrap.max_rows = 2;
    let galley = ui.painter().layout_job(job);
    let width = (galley.size().x + 12.0).max(44.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, CONTENT_H), Sense::click());
    paint_frame(ui, rect, &response, b, palette);
    paint_icon(
        ui,
        Pos2::new(rect.center().x, rect.top() + 6.0 + LARGE_ICON / 2.0),
        LARGE_ICON,
        b,
        colour,
    );
    // A centred job's galley is laid out around x = 0.
    let text_top = rect.top() + 12.0 + LARGE_ICON;
    ui.painter()
        .galley(Pos2::new(rect.center().x, text_top), galley, colour);
    finish(response, b, events);
}

fn small_galley(ui: &Ui, b: &Button, colour: Color32) -> std::sync::Arc<egui::Galley> {
    ui.painter()
        .layout_no_wrap(b.label.clone(), FontId::proportional(12.5), colour)
}

fn small_width(ui: &Ui, b: &Button) -> f32 {
    6.0 + SMALL_ICON + 6.0 + small_galley(ui, b, Color32::WHITE).size().x + 8.0
}

fn small_button(
    ui: &mut Ui,
    b: &Button,
    width: f32,
    palette: &Palette,
    events: &mut Vec<BarEvent>,
) {
    let colour = text_colour(b, palette);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, SMALL_H), Sense::click());
    paint_frame(ui, rect, &response, b, palette);
    paint_icon(
        ui,
        Pos2::new(rect.left() + 6.0 + SMALL_ICON / 2.0, rect.center().y),
        SMALL_ICON,
        b,
        colour,
    );
    let galley = small_galley(ui, b, colour);
    let pos = Pos2::new(
        rect.left() + 12.0 + SMALL_ICON,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, colour);
    finish(response, b, events);
}
