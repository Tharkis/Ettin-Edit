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
        /// Shown when the pointer rests on it (also while it is greyed out).
        hint: Option<String>,
    },
    Separator,
    /// A dim line that says what the menu is for, as written.
    Note(String),
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
                hint,
            } => {
                let mut button = Button::new(label.as_str());
                if let Some(checked) = checked {
                    button = button.selected(*checked);
                }
                if let Some(shortcut) = shortcut {
                    button = button.shortcut_text(shortcut.as_str());
                }
                let mut response = ui.add_enabled(*enabled, button);
                if let Some(hint) = hint {
                    response = response
                        .on_hover_text(hint.as_str())
                        .on_disabled_hover_text(hint.as_str());
                }
                if response.clicked() {
                    clicked = Some(*id);
                }
            }
            MenuEntry::Separator => {
                ui.separator();
            }
            MenuEntry::Note(text) => {
                ui.label(egui::RichText::new(text).small().weak());
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
