//! Names for screen readers. egui reads a button's text out, so a button
//! that shows only an icon or a symbol (like `Aa` or `×`) needs its real
//! name given here; the same text is its tooltip.

use egui::{Response, WidgetInfo, WidgetType};

/// Names a button that shows only an icon or a symbol.
pub fn name(response: Response, label: &str) -> Response {
    let enabled = response.enabled();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    response.on_hover_text(label)
}

/// Names an icon-only button that turns something on or off.
pub fn toggle(response: Response, on: bool, label: &str) -> Response {
    let enabled = response.enabled();
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, on, label));
    response.on_hover_text(label)
}

#[cfg(test)]
pub mod tests {
    use egui::accesskit::{Role, TreeUpdate};

    /// Runs one frame with accesskit on and returns the tree.
    pub fn tree(mut ui: impl FnMut(&mut egui::Ui)) -> TreeUpdate {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let _ = crate::fonts::install(&ctx, &crate::fonts::EditorFont::default());
        // The first frame lays out; the second has every widget in place.
        let mut update = None;
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), &mut ui);
            output.textures_delta.clear();
            update = output.platform_output.accesskit_update;
        }
        update.expect("accesskit is on")
    }

    /// The labels of the buttons in a tree.
    pub fn button_labels(tree: &TreeUpdate) -> Vec<String> {
        tree.nodes
            .iter()
            .filter(|(_, n)| matches!(n.role(), Role::Button | Role::CheckBox))
            .filter_map(|(_, n)| n.label().map(str::to_owned))
            .collect()
    }

    #[test]
    fn find_bar_toggles_and_recipe_buttons_have_names() {
        let tree = tree(|ui| {
            let mut on = false;
            crate::search_ui::option_toggle(ui, "Aa", &mut on, "Match case");
            crate::search_ui::option_toggle(ui, ".*", &mut on, "Regular expression");
            crate::search_ui::icon_button(ui, egui_phosphor::regular::PLUS, "New recipe");
        });
        let labels = button_labels(&tree);
        for name in ["Match case", "Regular expression", "New recipe"] {
            assert!(labels.contains(&name.to_owned()), "{name}: {labels:?}");
        }
    }

    #[test]
    fn named_buttons_are_read_by_name() {
        let tree = tree(|ui| {
            super::name(ui.button("×"), "Clear the search");
            super::toggle(ui.button("Aa"), true, "Match case");
        });
        let labels = button_labels(&tree);
        assert!(
            labels.contains(&"Clear the search".to_owned()),
            "{labels:?}"
        );
        assert!(labels.contains(&"Match case".to_owned()), "{labels:?}");
        assert!(!labels.iter().any(|l| l == "×" || l == "Aa"), "{labels:?}");
    }
}
