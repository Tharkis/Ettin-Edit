//! Split view: tabs live in the main (left) view or the other (right) view,
//! and a document can be cloned into both. Each view has its own active tab;
//! `NixPadApp::active` is always the active tab of the view with focus.

use egui::{Pos2, Rect, Sense, Stroke, Ui};

use super::NixPadApp;

pub(super) struct Split {
    /// Which view has focus: 0 is the main view, 1 the other view.
    pub focused: usize,
    /// The active tab of the view without focus, by document id.
    pub other_active: Option<u64>,
    /// Width of the main view as a share of the window.
    pub fraction: f32,
    /// Where the divider was drawn last frame, so the tab bar can line up with it.
    pub divider_x: Option<f32>,
}

impl Default for Split {
    fn default() -> Self {
        Self {
            focused: 0,
            other_active: None,
            fraction: 0.5,
            divider_x: None,
        }
    }
}

/// Width of the bar between the two views.
const DIVIDER: f32 = 5.0;

impl NixPadApp {
    /// True when both views have tabs.
    pub(super) fn is_split(&self) -> bool {
        self.docs.iter().any(|d| d.views[0]) && self.docs.iter().any(|d| d.views[1])
    }

    /// Indices of the tabs in `view`, in tab order.
    pub(super) fn tabs_in(&self, view: usize) -> Vec<usize> {
        (0..self.docs.len())
            .filter(|&i| self.docs[i].views[view])
            .collect()
    }

    /// The active tab of `view`.
    pub(super) fn active_in(&self, view: usize) -> Option<usize> {
        if view == self.split.focused {
            (self.active < self.docs.len()).then_some(self.active)
        } else {
            let id = self.split.other_active?;
            self.docs.iter().position(|d| d.id == id && d.views[view])
        }
    }

    /// The tab next to `index` in `view` (the one after it, else the one
    /// before), to show when `index` leaves that view.
    pub(super) fn neighbour_in(&self, view: usize, index: usize) -> Option<u64> {
        let tabs = self.tabs_in(view);
        let pos = tabs.iter().position(|&i| i == index)?;
        tabs.get(pos + 1)
            .or_else(|| pos.checked_sub(1).and_then(|p| tabs.get(p)))
            .map(|&i| self.docs[i].id)
    }

    /// Makes `index` the active tab of `view` and gives that view focus.
    pub(super) fn focus_view(&mut self, view: usize, index: usize) {
        if index >= self.docs.len() || !self.docs[index].views[view] {
            return;
        }
        // Going to a tab leaves the comparison (its tab brings it back).
        if let Some(c) = &mut self.compare {
            c.shown = false;
        }
        if let Some(old) = self.docs.get(self.active)
            && old.id != self.docs[index].id
        {
            self.previous_tab = Some(old.id);
        }
        if view != self.split.focused {
            let old = self.split.focused;
            self.split.other_active = self
                .docs
                .get(self.active)
                .filter(|d| d.views[old])
                .map(|d| d.id);
            self.split.focused = view;
        }
        self.active = index;
        self.make_live(index, view);
        self.focus_editor = true;
        self.scroll_to_active_tab = true;
    }

    /// Puts the carets of `view` into the editor of a document shown in both views.
    fn make_live(&mut self, index: usize, view: usize) {
        let doc = &mut self.docs[index];
        if doc.editor.has_other_view() && doc.live_view != view {
            doc.editor.swap_view();
            doc.live_view = view;
        }
    }

    /// Keeps the views consistent after tabs open, close or move: the main
    /// view is never empty, and each view's active tab is one of its own.
    pub(super) fn fix_views(&mut self) {
        if self.docs.is_empty() {
            self.split.other_active = None;
            self.split.focused = 0;
            return;
        }
        for doc in &mut self.docs {
            if !doc.views[0] && !doc.views[1] {
                doc.views[0] = true;
            }
            if doc.views[0] && doc.views[1] {
                if !doc.editor.has_other_view() {
                    doc.editor.add_other_view();
                }
            } else {
                if doc.editor.has_other_view() {
                    if doc.views[doc.live_view] {
                        doc.editor.remove_other_view();
                    } else {
                        doc.editor.swap_view();
                        doc.editor.remove_other_view();
                    }
                }
                doc.live_view = usize::from(doc.views[1]);
            }
        }
        // Everything left in the other view moves back to the main one.
        if !self.docs.iter().any(|d| d.views[0]) {
            for doc in &mut self.docs {
                doc.views = [true, false];
                doc.live_view = 0;
            }
        }
        if !self.is_split() {
            self.split.focused = 0;
            self.split.other_active = None;
        }
        let focused = self.split.focused;
        if self.active >= self.docs.len() || !self.docs[self.active].views[focused] {
            self.active = self.tabs_in(focused).first().copied().unwrap_or(0);
        }
        if self.is_split() && self.active_in(1 - focused).is_none() {
            self.split.other_active = self.tabs_in(1 - focused).first().map(|&i| self.docs[i].id);
        }
        self.make_live(self.active, focused);
    }

    /// Moves a tab to the other view (for a tab shown in both, closes it in `from`).
    pub(super) fn move_to_other_view(&mut self, index: usize, from: usize) {
        let Some(doc) = self.docs.get(index) else {
            return;
        };
        if !doc.views[from] {
            return;
        }
        if doc.views[1 - from] {
            self.close_in_view(index, from);
            return;
        }
        let to = 1 - from;
        let from_active = self.active_in(from);
        let replacement = if from_active == Some(index) {
            self.neighbour_in(from, index)
        } else {
            from_active.map(|i| self.docs[i].id)
        };
        let doc = &mut self.docs[index];
        doc.views = [false, false];
        doc.views[to] = true;
        doc.live_view = to;
        self.split.focused = to;
        self.active = index;
        self.split.other_active = replacement;
        self.focus_editor = true;
        self.scroll_to_active_tab = true;
        self.fix_views();
    }

    /// Shows a tab in the other view too, with its own carets and scrolling.
    pub(super) fn clone_to_other_view(&mut self, index: usize, from: usize) {
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        let to = 1 - from;
        if !doc.views[to] {
            if !doc.editor.has_other_view() {
                doc.live_view = from;
                doc.editor.add_other_view();
            }
            doc.views = [true, true];
        }
        // The view it came from keeps showing it.
        let from_id = doc.id;
        if self.split.focused == from {
            self.split.focused = to;
            self.split.other_active = Some(from_id);
        } else if self.active_in(from) != Some(index) {
            // `from` is the unfocused view: make this its active tab.
            self.split.other_active = Some(from_id);
            self.split.focused = to;
        }
        self.active = index;
        self.make_live(index, to);
        self.focus_editor = true;
        self.scroll_to_active_tab = true;
        self.fix_views();
    }

    /// Closes the copy of a tab shown in both views that is in `view`. Returns
    /// false (doing nothing) when the tab is only in one view.
    pub(super) fn close_in_view(&mut self, index: usize, view: usize) -> bool {
        let Some(doc) = self.docs.get(index) else {
            return false;
        };
        if !(doc.views[0] && doc.views[1]) {
            return false;
        }
        let was_active = self.active_in(view) == Some(index);
        let replacement = was_active.then(|| self.neighbour_in(view, index)).flatten();
        let other = 1 - view;
        let doc = &mut self.docs[index];
        if doc.live_view == view {
            doc.editor.swap_view();
            doc.live_view = other;
        }
        doc.editor.remove_other_view();
        doc.views[view] = false;
        if was_active {
            if view == self.split.focused {
                match replacement.and_then(|id| self.docs.iter().position(|d| d.id == id)) {
                    Some(i) => self.active = i,
                    // The view is empty now: focus moves to the tab in the other one.
                    None => {
                        self.split.focused = other;
                        self.active = index;
                        self.split.other_active = None;
                    }
                }
            } else {
                self.split.other_active = replacement;
            }
        }
        self.focus_editor = true;
        self.fix_views();
        true
    }

    /// Gives focus to the other view.
    pub(super) fn focus_other_view(&mut self) {
        let other = 1 - self.split.focused;
        if let Some(i) = self.active_in(other) {
            self.focus_view(other, i);
        }
    }

    /// Selects the next (`step` 1) or previous (`step` -1) tab of the focused view.
    pub(super) fn step_tab(&mut self, step: isize) {
        let view = self.split.focused;
        let tabs = self.tabs_in(view);
        let Some(pos) = tabs.iter().position(|&i| i == self.active) else {
            return;
        };
        let n = tabs.len() as isize;
        let next = tabs[(pos as isize + step).rem_euclid(n) as usize];
        self.focus_view(view, next);
    }

    /// The tab bar: one strip, or one per view lined up with the views below.
    pub(super) fn tab_bars(&mut self, ui: &mut Ui) {
        if !self.is_split() {
            self.tab_strip(ui, 0);
            return;
        }
        let rect = ui.available_rect_before_wrap();
        let split_x = self
            .split
            .divider_x
            .unwrap_or(rect.left() + rect.width() * self.split.fraction)
            .clamp(rect.left() + 40.0, rect.right() - 40.0);
        let height = 32.0;
        let left = Rect::from_min_max(rect.min, Pos2::new(split_x - DIVIDER, rect.top() + height));
        let right = Rect::from_min_max(
            Pos2::new(split_x + DIVIDER, rect.top()),
            Pos2::new(rect.right(), rect.top() + height),
        );
        for (view, r) in [(0, left), (1, right)] {
            ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
                ui.set_clip_rect(r.intersect(ui.clip_rect()));
                self.tab_strip(ui, view);
            });
        }
        ui.allocate_rect(
            Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.top() + height)),
            Sense::hover(),
        );
    }

    /// The editors: one view, or two side by side with a divider to drag.
    pub(super) fn editor_area(&mut self, ui: &mut Ui) {
        if self.compare.as_ref().is_some_and(|c| c.shown) {
            self.split.divider_x = None;
            self.compare_area(ui);
            return;
        }
        if !self.is_split() {
            self.split.divider_x = None;
            if self.active < self.docs.len() {
                self.editor_pane(ui, 0, self.active);
            }
            return;
        }
        let rect = ui.max_rect();
        let x = (rect.left() + rect.width() * self.split.fraction).round();
        self.split.divider_x = Some(x);
        let divider = Rect::from_min_max(
            Pos2::new(x - DIVIDER / 2.0, rect.top()),
            Pos2::new(x + DIVIDER / 2.0, rect.bottom()),
        );
        let response = ui.interact(divider, ui.id().with("split-divider"), Sense::drag());
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if response.dragged()
            && let Some(p) = response.interact_pointer_pos()
        {
            self.split.fraction = ((p.x - rect.left()) / rect.width()).clamp(0.15, 0.85);
        }
        let palette = self.palette();
        ui.painter().rect_filled(divider, 0.0, palette.chrome_bg);
        ui.painter().line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, palette.text_dim.gamma_multiply(0.25)),
        );
        let panes = [
            Rect::from_min_max(rect.min, Pos2::new(divider.left(), rect.bottom())),
            Rect::from_min_max(Pos2::new(divider.right(), rect.top()), rect.max),
        ];
        let mut focus = None;
        for (view, pane) in panes.into_iter().enumerate() {
            let Some(index) = self.active_in(view) else {
                continue;
            };
            ui.scope_builder(egui::UiBuilder::new().max_rect(pane), |ui| {
                ui.set_clip_rect(pane.intersect(ui.clip_rect()));
                if self.editor_pane(ui, view, index) && view != self.split.focused {
                    focus = Some((view, index));
                }
            });
        }
        // A click in the other view gives it focus.
        if let Some((view, index)) = focus {
            self.focus_view(view, index);
        }
        // The view without focus is dimmed a little.
        let unfocused = panes[1 - self.split.focused];
        ui.painter()
            .rect_filled(unfocused, 0.0, palette.chrome_bg.gamma_multiply(0.08));
    }
}
