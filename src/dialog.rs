//! Helpers for resizable windows (Ettin Edit 2.6): the body takes the room
//! the window has, less the footer's height, so dragging an edge makes the
//! window bigger or smaller and nothing grows it on its own.

use egui::{Id, Ui};

/// The height left for a window's body above its footer. The footer's
/// height is the one [`footer`] measured last frame (`guess` before that).
pub fn body_height(ui: &Ui, id: Id, guess: f32) -> f32 {
    let footer = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(id.with("footer-h")))
        .unwrap_or(guess);
    (ui.available_height() - footer - ui.spacing().item_spacing.y).max(80.0)
}

/// Draws a window's footer and remembers its height for [`body_height`].
pub fn footer<R>(ui: &mut Ui, id: Id, add: impl FnOnce(&mut Ui) -> R) -> R {
    let top = ui.cursor().top();
    let r = add(ui);
    let h = (ui.min_rect().bottom() - top).max(0.0);
    let key = id.with("footer-h");
    let before = ui.ctx().data(|d| d.get_temp::<f32>(key));
    if before.is_none_or(|b| (b - h).abs() > 0.5) {
        ui.ctx().data_mut(|d| d.insert_temp(key, h));
        ui.ctx().request_repaint();
    }
    r
}

/// Where a maximized window goes: the main window, less a margin.
pub fn maximized_rect(ctx: &egui::Context) -> egui::Rect {
    ctx.content_rect().shrink(12.0)
}

/// A title-bar-less maximize toggle, drawn as an icon button. True when
/// clicked.
pub fn maximize_button(ui: &mut Ui, maximized: bool) -> bool {
    let (icon, tip) = if maximized {
        (
            egui_phosphor::regular::CORNERS_IN,
            crate::i18n::tr("window-restore"),
        )
    } else {
        (
            egui_phosphor::regular::CORNERS_OUT,
            crate::i18n::tr("window-maximize"),
        )
    };
    ui.add(egui::Button::new(icon).frame(false))
        .on_hover_text(tip)
        .clicked()
}

/// Where a window of `size` starts: the middle of the main window. Used
/// with `default_rect` rather than a centre pivot, so dragging the corner
/// moves only that corner.
pub fn centered(ctx: &egui::Context, size: egui::Vec2) -> egui::Rect {
    let area = ctx.content_rect();
    let size = size
        .min(area.size() - egui::vec2(24.0, 24.0))
        .max(egui::vec2(200.0, 150.0));
    egui::Rect::from_center_size(area.center(), size)
}
