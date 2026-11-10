//! Draws the toolbar: a row of tab names, and below it the selected tab's
//! groups of buttons. Collapsed, only the names show and clicking one opens
//! its buttons over the text until a command runs or the user clicks away.
//! Simplified, the buttons are one row of small ones.

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Order, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
};
use egui_phosphor::regular as ph;

use crate::actions::Action;
use crate::fonts::icon_font;
use crate::menu::{self, MenuEntry};
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
/// The widest a collapsed group's name wraps to.
const COLLAPSED_LABEL_W: f32 = 80.0;
/// A small button without its label.
const ICON_ONLY_W: f32 = 6.0 + SMALL_ICON + 6.0;
/// Room for the ▾ of a button that opens a menu.
const CARET_W: f32 = 12.0;

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
    /// The shortcut as shown in menus, like `Ctrl+Alt+K`.
    pub shortcut: Option<String>,
    pub enabled: bool,
    /// `Some` for a command that is turned on or off.
    pub checked: Option<bool>,
    pub size: Size,
    /// Where it is in the layout: tab, group, item.
    pub place: (usize, usize, usize),
    /// A drop-down button's menu; clicking it opens this instead of
    /// running `action`.
    pub menu: Option<Vec<MenuEntry>>,
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
    /// An item of a drop-down button's menu (or the ? menu) was clicked.
    Menu(usize),
    /// Switch between the full and the simplified toolbar.
    ToggleMode,
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
    /// `help` is the menu of the ? button at the right of the tab names.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        tabs: &[TabView],
        help: &[MenuEntry],
        collapsed: bool,
        simplified: bool,
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
            .horizontal(|ui| {
                self.tab_strip(ui, tabs, help, collapsed, simplified, palette, &mut events)
            })
            .response;
        let current = tabs.iter().find(|t| t.index == self.selected);
        let body = |ui: &mut Ui, tab: &TabView, events: &mut Vec<BarEvent>| {
            if simplified {
                simple_body(ui, tab, palette, events)
            } else {
                body(ui, tab, palette, events)
            }
        };
        if !collapsed {
            if let Some(tab) = current {
                body(ui, tab, &mut events);
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
                            body(ui, tab, &mut events)
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

    #[allow(clippy::too_many_arguments)]
    fn tab_strip(
        &mut self,
        ui: &mut Ui,
        tabs: &[TabView],
        help: &[MenuEntry],
        collapsed: bool,
        simplified: bool,
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
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, &tab.name)
            });
            if response.hovered() {
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(4), palette.tab_hover_bg);
            }
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, palette.text);
            focus_ring(ui, rect, &response, palette);
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
        // The rest of the row: ?, the toolbar style and the collapse button
        // at the far right.
        let rest = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(
            Rect::from_min_size(rest.min, Vec2::new(rest.width(), 24.0)),
            Sense::click(),
        );
        strip_menu(&response, collapsed, self.menu_bar_shown, events);
        let mode_text = format!(
            "{}  {}",
            ph::LIST,
            if simplified {
                t!("toolbar-mode-full")
            } else {
                t!("toolbar-mode-simplified")
            }
        );
        let mode_galley =
            ui.painter()
                .layout_no_wrap(mode_text, FontId::proportional(12.5), palette.text_dim);
        let mode = Rect::from_min_size(
            Pos2::new(
                rest.right() - 32.0 - mode_galley.size().x - 12.0,
                rest.top() + 1.0,
            ),
            Vec2::new(mode_galley.size().x + 12.0, 22.0),
        );
        let m = ui.interact(mode, Id::new("toolbar-mode"), Sense::click());
        if m.hovered() {
            ui.painter()
                .rect_filled(mode, CornerRadius::same(4), palette.tab_hover_bg);
        }
        focus_ring(ui, mode, &m, palette);
        let tip = if simplified {
            t!("toolbar-mode-full-tip")
        } else {
            t!("toolbar-mode-simplified-tip")
        };
        m.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &tip));
        ui.painter().galley(
            mode.center() - mode_galley.size() / 2.0,
            mode_galley,
            palette.text_dim,
        );
        if m.on_hover_text(&tip).clicked() {
            events.push(BarEvent::ToggleMode);
        }
        let help_rect = Rect::from_center_size(
            Pos2::new(mode.left() - 16.0, rest.top() + 12.0),
            Vec2::splat(22.0),
        );
        let h = ui.interact(help_rect, Id::new("toolbar-help"), Sense::click());
        if h.hovered() {
            ui.painter()
                .rect_filled(help_rect, CornerRadius::same(4), palette.tab_hover_bg);
        }
        focus_ring(ui, help_rect, &h, palette);
        ui.painter().text(
            help_rect.center(),
            Align2::CENTER_CENTER,
            ph::QUESTION,
            icon_font(15.0),
            palette.text_dim,
        );
        let tip = t!("toolbar-help");
        h.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &tip));
        let h = h.on_hover_text(&tip);
        if let Some(id) = egui::Popup::menu(&h)
            .show(|ui| menu::show(ui, help))
            .and_then(|r| r.inner)
        {
            events.push(BarEvent::Menu(id));
        }
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
        b.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &tip));
        if b.on_hover_text(&tip).clicked() {
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

/// How much room a group takes; a narrow window steps groups down from
/// `Large` towards `Collapsed`, rightmost group first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupState {
    /// As laid out: large and small buttons.
    Large,
    /// Large buttons become small ones, three to a column.
    Small,
    /// Small buttons without their labels (the tooltip still names them).
    IconOnly,
    /// One large button with the group's name that opens a menu of its commands.
    Collapsed,
}

const STATES: [GroupState; 4] = [
    GroupState::Large,
    GroupState::Small,
    GroupState::IconOnly,
    GroupState::Collapsed,
];

/// How a tab's groups are drawn in the width there is.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    /// The state of each group that is shown.
    pub states: Vec<GroupState>,
    /// Groups from this index on go in the » menu.
    pub shown: usize,
}

/// Space between two groups (the divider and its margins).
const GROUP_GAP: f32 = 10.0;
/// The » button and the space before it.
const OVERFLOW_W: f32 = 34.0;
/// Room kept free so rounding never pushes the last group off the edge.
const SLACK: f32 = 4.0;

/// Fits groups into `available` width. `widths[g][s]` is group `g`'s width in
/// state `s` (`STATES` order). Starting with the rightmost group, each group
/// is stepped down one state at a time until the tab fits (a state that
/// isn't narrower is passed over); when every group is as narrow as it gets
/// and it still doesn't fit, groups move from the right into the » menu.
pub fn fit(widths: &[[f32; 4]], gap: f32, available: f32, overflow: f32) -> Fit {
    let n = widths.len();
    let mut states = vec![0usize; n];
    let total = |states: &[usize], count: usize| -> f32 {
        (0..count).map(|g| widths[g][states[g]]).sum::<f32>() + gap * count.saturating_sub(1) as f32
    };
    let as_fit = |states: &[usize], shown: usize| Fit {
        states: states[..shown].iter().map(|&s| STATES[s]).collect(),
        shown,
    };
    for g in (0..n).rev() {
        while total(&states, n) > available {
            // The next state that is narrower; a group whose collapsed
            // button would be wider than its icons stays as icons.
            let now = widths[g][states[g]];
            match (states[g] + 1..4).find(|&s| widths[g][s] < now) {
                Some(s) => states[g] = s,
                None => break,
            }
        }
    }
    if total(&states, n) <= available {
        return as_fit(&states, n);
    }
    let mut shown = n;
    while shown > 0 && total(&states, shown) + overflow > available {
        shown -= 1;
    }
    as_fit(&states, shown)
}

/// The selected tab's groups, side by side, shrunk to fit the window.
fn body(ui: &mut Ui, tab: &TabView, palette: &Palette, events: &mut Vec<BarEvent>) {
    if tab.groups.iter().all(|g| g.buttons.is_empty()) {
        ui.label(egui::RichText::new(t!("toolbar-empty")).weak());
        return;
    }
    let widths: Vec<[f32; 4]> = tab
        .groups
        .iter()
        .map(|g| STATES.map(|s| group_width(ui, g, s)))
        .collect();
    let available = ui.available_width() - SLACK;
    let layout = fit(&widths, GROUP_GAP, available, OVERFLOW_W);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, ROW_GAP);
        for (i, (group, state)) in tab.groups.iter().zip(&layout.states).enumerate() {
            if i > 0 {
                let x = ui.cursor().left() + 3.0;
                let top = ui.cursor().top() + 4.0;
                ui.painter().vline(
                    x,
                    top..=top + CONTENT_H + 12.0,
                    Stroke::new(1.0, palette.border),
                );
                ui.add_space(GROUP_GAP - 2.0);
            }
            group_ui(ui, (tab.index, i), group, *state, palette, events);
        }
        if layout.shown < tab.groups.len() {
            overflow_button(
                ui,
                tab,
                &tab.groups[layout.shown..],
                CONTENT_H,
                palette,
                events,
            );
        }
    });
}

/// A group's columns: one large button, or up to three small ones.
enum Column<'a> {
    Large(&'a Button),
    Small(Vec<&'a Button>),
}

fn columns(group: &GroupView, state: GroupState) -> Vec<Column<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    let all_small = state != GroupState::Large;
    while i < group.buttons.len() {
        if !all_small && group.buttons[i].size == Size::Large {
            out.push(Column::Large(&group.buttons[i]));
            i += 1;
            continue;
        }
        let column: Vec<&Button> = group.buttons[i..]
            .iter()
            .take_while(|b| all_small || b.size == Size::Small)
            .take(ROWS)
            .collect();
        i += column.len();
        out.push(Column::Small(column));
    }
    out
}

fn name_galley(ui: &Ui, group: &GroupView, palette: &Palette) -> std::sync::Arc<egui::Galley> {
    ui.painter().layout_no_wrap(
        group.name.clone(),
        FontId::proportional(11.0),
        palette.text_dim,
    )
}

/// The width a group takes in a state, without drawing it.
fn group_width(ui: &Ui, group: &GroupView, state: GroupState) -> f32 {
    let name = ui
        .painter()
        .layout_no_wrap(
            group.name.clone(),
            FontId::proportional(11.0),
            Color32::WHITE,
        )
        .size()
        .x
        + 8.0;
    if state == GroupState::IconOnly && !icons_differ(group) {
        return f32::INFINITY;
    }
    let content = if state == GroupState::Collapsed {
        collapsed_size(ui, &group.name)
    } else {
        let cols = columns(group, state);
        let gaps = 2.0 * cols.len().saturating_sub(1) as f32;
        cols.iter()
            .map(|c| match c {
                Column::Large(b) => large_width(ui, b),
                Column::Small(bs) => small_column_width(ui, bs, state),
            })
            .sum::<f32>()
            + gaps
    };
    content.max(name)
}

/// Whether every button in the group has its own icon; without labels,
/// buttons that share an icon can't be told apart, so such a group skips
/// the icon-only state.
fn icons_differ(group: &GroupView) -> bool {
    let key = |b: &Button| match &b.icon {
        IconImage::Glyph(g) => (Some(*g), None),
        IconImage::Texture(t) => (None, Some(t.id())),
    };
    let keys: Vec<_> = group.buttons.iter().map(key).collect();
    keys.iter().enumerate().all(|(i, k)| !keys[..i].contains(k))
}

fn small_column_width(ui: &Ui, column: &[&Button], state: GroupState) -> f32 {
    if state == GroupState::IconOnly {
        return column.iter().map(|b| icon_width(b)).fold(0.0, f32::max);
    }
    column
        .iter()
        .map(|b| small_width(ui, b))
        .fold(0.0, f32::max)
}

fn group_ui(
    ui: &mut Ui,
    id: (usize, usize),
    group: &GroupView,
    state: GroupState,
    palette: &Palette,
    events: &mut Vec<BarEvent>,
) {
    let name = name_galley(ui, group, palette);
    ui.vertical(|ui| {
        let row = ui.horizontal_top(|ui| {
            ui.set_min_height(CONTENT_H);
            ui.set_min_width(name.size().x + 8.0);
            if state == GroupState::Collapsed {
                collapsed_button(ui, id, group, palette, events);
                return;
            }
            for column in columns(group, state) {
                match column {
                    Column::Large(b) => large_button(ui, b, palette, events),
                    Column::Small(column) => {
                        let width = small_column_width(ui, &column, state);
                        ui.vertical(|ui| {
                            for b in column {
                                if state == GroupState::IconOnly {
                                    icon_button(ui, b, palette, events);
                                } else {
                                    small_button(ui, b, width, palette, events);
                                }
                            }
                        });
                    }
                }
            }
        });
        let width = row.response.rect.width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 14.0), Sense::hover());
        ui.painter()
            .galley(rect.center() - name.size() / 2.0, name, palette.text_dim);
    });
}

/// The commands of a group as menu items, with their state.
fn menu_items(ui: &mut Ui, group: &GroupView, events: &mut Vec<BarEvent>) {
    for b in &group.buttons {
        if let Some(entries) = &b.menu {
            if let Some(id) = ui
                .menu_button(b.label.as_str(), |ui| menu::show(ui, entries))
                .inner
                .flatten()
            {
                events.push(BarEvent::Menu(id));
            }
            continue;
        }
        let icon: egui::Atom<'_> = match &b.icon {
            IconImage::Glyph(g) => egui::RichText::new(*g).font(icon_font(14.0)).into(),
            IconImage::Texture(tex) => egui::Image::new((tex.id(), Vec2::splat(14.0))).into(),
        };
        let mut button =
            egui::Button::new((icon, b.label.as_str())).selected(b.checked == Some(true));
        if let Some(sc) = &b.shortcut {
            button = button.shortcut_text(sc.as_str());
        }
        if ui.add_enabled(b.enabled, button).clicked() {
            events.push(BarEvent::Run(b.action.clone()));
            ui.close();
        }
    }
}

/// The label of a collapsed group's button: wraps once, then the caret.
fn collapsed_galley(ui: &Ui, name: &str, colour: Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(
        name.to_owned(),
        FontId::proportional(12.0),
        colour,
        COLLAPSED_LABEL_W,
    );
    job.halign = egui::Align::Center;
    job.wrap.max_rows = 2;
    ui.painter().layout_job(job)
}

fn collapsed_size(ui: &Ui, name: &str) -> f32 {
    (collapsed_galley(ui, name, Color32::WHITE).size().x + 12.0).max(48.0)
}

fn collapsed_button(
    ui: &mut Ui,
    id: (usize, usize),
    group: &GroupView,
    palette: &Palette,
    events: &mut Vec<BarEvent>,
) {
    let Some(first) = group.buttons.first() else {
        return;
    };
    let any_enabled = group.buttons.iter().any(|b| b.enabled);
    let colour = if any_enabled {
        palette.text
    } else {
        palette.text_dim.gamma_multiply(0.6)
    };
    let galley = collapsed_galley(ui, &group.name, colour);
    let width = (galley.size().x + 12.0).max(48.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, CONTENT_H), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, any_enabled, &group.name)
    });
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    let any_checked = group.buttons.iter().any(|b| b.checked == Some(true));
    let radius = CornerRadius::same(4);
    if open || response.hovered() {
        ui.painter().rect_filled(rect, radius, palette.tab_hover_bg);
    }
    focus_ring(ui, rect, &response, palette);
    if any_checked {
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, palette.accent.gamma_multiply(0.7)),
            egui::StrokeKind::Inside,
        );
    }
    paint_icon(
        ui,
        Pos2::new(rect.center().x, rect.top() + 6.0 + LARGE_ICON / 2.0),
        LARGE_ICON,
        &first.icon,
        any_enabled,
        colour,
    );
    let text_top = rect.top() + 12.0 + LARGE_ICON;
    let rows = galley.rows.len();
    let last = galley.rows.last().map(|r| r.rect()).unwrap_or(Rect::ZERO);
    let label_h = galley.size().y;
    ui.painter()
        .galley(Pos2::new(rect.center().x, text_top), galley, colour);
    let caret = if rows <= 1 {
        Pos2::new(rect.center().x, text_top + label_h + 7.0)
    } else {
        Pos2::new(
            rect.center().x + last.right() + 7.0,
            text_top + last.center().y,
        )
    };
    ui.painter().text(
        caret,
        Align2::CENTER_CENTER,
        ph::CARET_DOWN,
        icon_font(10.0),
        palette.text_dim,
    );
    let response = response.on_hover_text(&group.name);
    egui::Popup::menu(&response)
        .id(Id::new(("toolbar-group-menu", id)))
        .show(|ui| menu_items(ui, group, events));
}

/// The » button at the right end that lists the groups with no room.
fn overflow_button(
    ui: &mut Ui,
    tab: &TabView,
    hidden: &[GroupView],
    height: f32,
    palette: &Palette,
    events: &mut Vec<BarEvent>,
) {
    let rest = ui.available_rect_before_wrap();
    let side = height.min(28.0);
    let rect = Rect::from_min_size(
        Pos2::new(rest.right() - side, rest.top() + (height - side) / 2.0),
        Vec2::splat(side),
    );
    let response = ui.interact(
        rect,
        Id::new(("toolbar-overflow", tab.index)),
        Sense::click(),
    );
    let label = t!("toolbar-more");
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    let radius = CornerRadius::same(4);
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    if response.hovered() {
        ui.painter().rect_filled(rect, radius, palette.tab_hover_bg);
    }
    focus_ring(ui, rect, &response, palette);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ph::CARET_DOUBLE_RIGHT,
        icon_font(15.0),
        palette.text,
    );
    let response = response.on_hover_text(&label);
    egui::Popup::menu(&response).show(|ui| {
        ui.label(
            egui::RichText::new(t!("toolbar-hidden-groups"))
                .small()
                .weak(),
        );
        for group in hidden {
            ui.menu_button(&group.name, |ui| menu_items(ui, group, events));
        }
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
    focus_ring(ui, rect, response, palette);
}

/// The outline of the button that has keyboard focus (Tab moves it).
fn focus_ring(ui: &Ui, rect: Rect, response: &Response, palette: &Palette) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(1.5, palette.accent),
            egui::StrokeKind::Inside,
        );
    }
}

fn paint_icon(ui: &Ui, center: Pos2, size: f32, icon: &IconImage, enabled: bool, colour: Color32) {
    match icon {
        IconImage::Glyph(g) => {
            ui.painter()
                .text(center, Align2::CENTER_CENTER, *g, icon_font(size), colour);
        }
        IconImage::Texture(tex) => {
            let tint = if enabled {
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
    // Screen readers read the label (an icon-only button has none on screen).
    response.widget_info(|| {
        let mut info = egui::WidgetInfo::labeled(egui::WidgetType::Button, b.enabled, &b.label);
        info.selected = b.checked;
        info
    });
    let response = response.on_hover_text(&b.tooltip);
    if let Some(entries) = &b.menu {
        if b.enabled
            && let Some(id) = egui::Popup::menu(&response)
                // Not the default id: the right-click menu below uses that
                // one, and closes it on a left click.
                .id(Id::new(("toolbar-drop-down", b.place)))
                .show(|ui| menu::show(ui, entries))
                .and_then(|r| r.inner)
        {
            events.push(BarEvent::Menu(id));
        }
    } else if response.clicked() && b.enabled {
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

fn large_galley(ui: &Ui, b: &Button, colour: Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(
        b.label.clone(),
        FontId::proportional(12.0),
        colour,
        LARGE_LABEL_W,
    );
    job.halign = egui::Align::Center;
    job.wrap.max_rows = 2;
    ui.painter().layout_job(job)
}

fn large_width(ui: &Ui, b: &Button) -> f32 {
    (large_galley(ui, b, Color32::WHITE).size().x + 12.0).max(44.0)
}

fn large_button(ui: &mut Ui, b: &Button, palette: &Palette, events: &mut Vec<BarEvent>) {
    let colour = text_colour(b, palette);
    let galley = large_galley(ui, b, colour);
    let width = (galley.size().x + 12.0).max(44.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, CONTENT_H), Sense::click());
    paint_frame(ui, rect, &response, b, palette);
    paint_icon(
        ui,
        Pos2::new(rect.center().x, rect.top() + 6.0 + LARGE_ICON / 2.0),
        LARGE_ICON,
        &b.icon,
        b.enabled,
        colour,
    );
    // A centred job's galley is laid out around x = 0.
    let text_top = rect.top() + 12.0 + LARGE_ICON;
    let label_h = galley.size().y;
    ui.painter()
        .galley(Pos2::new(rect.center().x, text_top), galley, colour);
    if b.menu.is_some() {
        caret(
            ui,
            Pos2::new(rect.center().x, text_top + label_h + 7.0),
            palette,
        );
    }
    finish(response, b, events);
}

/// The ▾ of a drop-down button.
fn caret(ui: &Ui, center: Pos2, palette: &Palette) {
    ui.painter().text(
        center,
        Align2::CENTER_CENTER,
        ph::CARET_DOWN,
        icon_font(10.0),
        palette.text_dim,
    );
}

/// The extra width of a drop-down button's ▾.
fn caret_w(b: &Button) -> f32 {
    if b.menu.is_some() { CARET_W } else { 0.0 }
}

fn small_galley(ui: &Ui, b: &Button, colour: Color32) -> std::sync::Arc<egui::Galley> {
    ui.painter()
        .layout_no_wrap(b.label.clone(), FontId::proportional(12.5), colour)
}

fn small_width(ui: &Ui, b: &Button) -> f32 {
    6.0 + SMALL_ICON + 6.0 + small_galley(ui, b, Color32::WHITE).size().x + 8.0 + caret_w(b)
}

fn icon_width(b: &Button) -> f32 {
    ICON_ONLY_W + caret_w(b)
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
        &b.icon,
        b.enabled,
        colour,
    );
    let galley = small_galley(ui, b, colour);
    let pos = Pos2::new(
        rect.left() + 12.0 + SMALL_ICON,
        rect.center().y - galley.size().y / 2.0,
    );
    let label_w = galley.size().x;
    ui.painter().galley(pos, galley, colour);
    if b.menu.is_some() {
        caret(
            ui,
            Pos2::new(pos.x + label_w + 2.0 + CARET_W / 2.0, rect.center().y),
            palette,
        );
    }
    finish(response, b, events);
}

/// A small button without its label; the tooltip names it.
fn icon_button(ui: &mut Ui, b: &Button, palette: &Palette, events: &mut Vec<BarEvent>) {
    let colour = text_colour(b, palette);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(icon_width(b), SMALL_H), Sense::click());
    paint_frame(ui, rect, &response, b, palette);
    paint_icon(
        ui,
        Pos2::new(rect.left() + ICON_ONLY_W / 2.0, rect.center().y),
        SMALL_ICON,
        &b.icon,
        b.enabled,
        colour,
    );
    if b.menu.is_some() {
        caret(
            ui,
            Pos2::new(rect.right() - CARET_W / 2.0 - 1.0, rect.center().y),
            palette,
        );
    }
    finish(response, b, events);
}

// -------------------------------------------------------------------------
// The simplified toolbar: one row of small buttons

/// A group's width in the simplified toolbar, by state (`STATES` order):
/// large buttons with labels and small ones as icons; (no second state);
/// every button as an icon; one drop-down with the group's name.
fn simple_widths(ui: &Ui, group: &GroupView) -> [f32; 4] {
    let gaps = 2.0 * group.buttons.len().saturating_sub(1) as f32;
    let mixed = group
        .buttons
        .iter()
        .map(|b| match b.size {
            Size::Large => small_width(ui, b),
            Size::Small => icon_width(b),
        })
        .sum::<f32>()
        + gaps;
    let icons = if icons_differ(group) {
        group.buttons.iter().map(icon_width).sum::<f32>() + gaps
    } else {
        f32::INFINITY
    };
    [
        mixed,
        f32::INFINITY,
        icons,
        simple_collapsed_width(ui, &group.name),
    ]
}

fn simple_collapsed_galley(ui: &Ui, name: &str, colour: Color32) -> std::sync::Arc<egui::Galley> {
    ui.painter()
        .layout_no_wrap(name.to_owned(), FontId::proportional(12.5), colour)
}

fn simple_collapsed_width(ui: &Ui, name: &str) -> f32 {
    8.0 + simple_collapsed_galley(ui, name, Color32::WHITE).size().x + 4.0 + CARET_W
}

/// The selected tab's groups in one row, shrunk to fit the window.
fn simple_body(ui: &mut Ui, tab: &TabView, palette: &Palette, events: &mut Vec<BarEvent>) {
    if tab.groups.iter().all(|g| g.buttons.is_empty()) {
        ui.label(egui::RichText::new(t!("toolbar-empty")).weak());
        return;
    }
    let widths: Vec<[f32; 4]> = tab.groups.iter().map(|g| simple_widths(ui, g)).collect();
    let available = ui.available_width() - SLACK;
    let layout = fit(&widths, GROUP_GAP, available, OVERFLOW_W);
    ui.horizontal(|ui| {
        ui.set_min_height(SMALL_H + 6.0);
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
        for (i, (group, state)) in tab.groups.iter().zip(&layout.states).enumerate() {
            if i > 0 {
                let x = ui.cursor().left() + 3.0;
                let mid = ui.min_rect().center().y;
                ui.painter()
                    .vline(x, mid - 8.0..=mid + 8.0, Stroke::new(1.0, palette.border));
                ui.add_space(GROUP_GAP - 2.0);
            }
            match state {
                GroupState::Collapsed => {
                    simple_collapsed(ui, (tab.index, i), group, palette, events)
                }
                GroupState::IconOnly => {
                    for b in &group.buttons {
                        icon_button(ui, b, palette, events);
                    }
                }
                _ => {
                    for b in &group.buttons {
                        match b.size {
                            Size::Large => {
                                let w = small_width(ui, b);
                                small_button(ui, b, w, palette, events);
                            }
                            Size::Small => icon_button(ui, b, palette, events),
                        }
                    }
                }
            }
        }
        if layout.shown < tab.groups.len() {
            overflow_button(
                ui,
                tab,
                &tab.groups[layout.shown..],
                SMALL_H,
                palette,
                events,
            );
        }
    });
}

/// A simplified group with no room for its buttons: its name and ▾.
fn simple_collapsed(
    ui: &mut Ui,
    id: (usize, usize),
    group: &GroupView,
    palette: &Palette,
    events: &mut Vec<BarEvent>,
) {
    let any_enabled = group.buttons.iter().any(|b| b.enabled);
    let colour = if any_enabled {
        palette.text
    } else {
        palette.text_dim.gamma_multiply(0.6)
    };
    let galley = simple_collapsed_galley(ui, &group.name, colour);
    let width = simple_collapsed_width(ui, &group.name);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, SMALL_H), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, any_enabled, &group.name)
    });
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), palette.tab_hover_bg);
    }
    focus_ring(ui, rect, &response, palette);
    ui.painter().galley(
        Pos2::new(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
        galley,
        colour,
    );
    caret(
        ui,
        Pos2::new(rect.right() - CARET_W / 2.0 - 2.0, rect.center().y),
        palette,
    );
    let response = response.on_hover_text(&group.name);
    egui::Popup::menu(&response)
        .id(Id::new(("toolbar-group-menu", id)))
        .show(|ui| menu_items(ui, group, events));
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: [f32; 4] = [100.0, 80.0, 40.0, 50.0];

    #[test]
    fn everything_fits_at_full_size() {
        let f = fit(&[W, W, W], 10.0, 400.0, 30.0);
        assert_eq!(f.shown, 3);
        assert!(f.states.iter().all(|s| *s == GroupState::Large));
    }

    #[test]
    fn rightmost_groups_shrink_first() {
        // 100 + 100 + 100 + 20 gaps = 320; 300 needs 20 less.
        let f = fit(&[W, W, W], 10.0, 300.0, 30.0);
        assert_eq!(
            f.states,
            vec![GroupState::Large, GroupState::Large, GroupState::Small]
        );
        // 260: the last group goes all the way (to icon-only), then the middle.
        let f = fit(&[W, W, W], 10.0, 245.0, 30.0);
        assert_eq!(
            f.states,
            vec![GroupState::Large, GroupState::Small, GroupState::IconOnly]
        );
        assert_eq!(f.shown, 3);
    }

    #[test]
    fn a_wider_state_is_passed_over() {
        // Collapsed (50) is wider than icon-only (40), so the group stays
        // as icons.
        let f = fit(&[W], 0.0, 30.0, 0.0);
        assert_eq!(f.shown, 0);
        let f = fit(&[W, W], 10.0, 130.0, 0.0);
        assert_eq!(f.states, vec![GroupState::Small, GroupState::IconOnly]);
        let f = fit(&[W, W], 10.0, 75.0, 30.0);
        assert_eq!(f.states, vec![GroupState::IconOnly]);
    }

    #[test]
    fn everything_collapsed_and_still_too_wide_overflows() {
        const C: [f32; 4] = [100.0, 80.0, 60.0, 50.0];
        // All collapsed: 4 × 50 + 3 × 10 = 230.
        let f = fit(&[C, C, C, C], 10.0, 150.0, 30.0);
        assert!(f.states.iter().all(|s| *s == GroupState::Collapsed));
        // Two groups (110) + the » button (30) fit in 150; three don't.
        assert_eq!(f.shown, 2);
        assert_eq!(f.states.len(), 2);
    }

    #[test]
    fn a_group_without_an_icon_only_state_collapses() {
        let f = fit(&[[100.0, 80.0, f32::INFINITY, 50.0]], 0.0, 60.0, 30.0);
        assert_eq!(f.states, vec![GroupState::Collapsed]);
    }

    #[test]
    fn icon_only_and_collapsed_buttons_have_names() {
        let palette = crate::theme::load_themes(None).0[0].palette;
        let button = |label: &str, icon| Button {
            action: Action::Undo,
            label: label.to_owned(),
            icon: IconImage::Glyph(icon),
            tooltip: label.to_owned(),
            shortcut: None,
            enabled: true,
            checked: None,
            size: Size::Small,
            place: (0, 0, 0),
            menu: None,
        };
        let group = GroupView {
            name: "History".to_owned(),
            buttons: vec![
                button("Undo", ph::ARROW_U_UP_LEFT),
                button("Redo", ph::ARROW_U_UP_RIGHT),
            ],
        };
        let tree = crate::a11y::tests::tree(|ui| {
            let mut events = Vec::new();
            group_ui(
                ui,
                (0, 0),
                &group,
                GroupState::IconOnly,
                &palette,
                &mut events,
            );
            group_ui(
                ui,
                (0, 1),
                &group,
                GroupState::Collapsed,
                &palette,
                &mut events,
            );
        });
        let labels: Vec<String> = tree
            .nodes
            .iter()
            .filter_map(|(_, n)| n.label().map(str::to_owned))
            .collect();
        for name in ["Undo", "Redo", "History"] {
            assert!(labels.contains(&name.to_owned()), "{name}: {labels:?}");
        }
    }

    #[test]
    fn no_room_at_all_hides_every_group() {
        let f = fit(&[W, W], 10.0, 20.0, 30.0);
        assert_eq!(f.shown, 0);
        assert!(f.states.is_empty());
    }
}
