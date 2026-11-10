//! Menus described as data, so the same menu can be drawn in more than one
//! place (the Tools menu is in the menu bar and the editor's right-click menu).

use egui::{Button, Ui};

pub enum MenuEntry {
    Item {
        label: String,
        shortcut: Option<String>,
        enabled: bool,
        /// `Some` for an item that can be ticked.
        checked: Option<bool>,
        /// Returned by [`show`] when the item is clicked.
        id: usize,
    },
    Separator,
    /// A line of dim text that says something about the items below it.
    Heading(String),
    /// A heading with a spinner, while something is being worked out.
    Busy(String),
    Submenu {
        label: String,
        entries: Vec<MenuEntry>,
    },
}

/// Draws the entries into an open menu; returns the id of the clicked item.
pub fn show(ui: &mut Ui, entries: &[MenuEntry]) -> Option<usize> {
    let mut clicked = None;
    for entry in entries {
        match entry {
            MenuEntry::Item {
                label,
                shortcut,
                enabled,
                checked,
                id,
            } => {
                let mut button = Button::new(label.as_str());
                if let Some(checked) = checked {
                    button = button.selected(*checked);
                }
                if let Some(shortcut) = shortcut {
                    button = button.shortcut_text(shortcut.as_str());
                }
                if ui.add_enabled(*enabled, button).clicked() {
                    clicked = Some(*id);
                }
            }
            MenuEntry::Separator => {
                ui.separator();
            }
            MenuEntry::Heading(text) => {
                ui.label(egui::RichText::new(text.to_uppercase()).small().weak());
            }
            MenuEntry::Busy(text) => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(12.0));
                    ui.label(egui::RichText::new(text).weak());
                });
            }
            MenuEntry::Submenu { label, entries } => {
                if let Some(id) = ui
                    .menu_button(label.as_str(), |ui| show(ui, entries))
                    .inner
                    .flatten()
                {
                    clicked = Some(id);
                }
            }
        }
    }
    clicked
}
