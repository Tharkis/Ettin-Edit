//! The theme colour editor (Theme > Edit Theme Colours): change a theme's
//! editor and syntax colours, see them at once, and save them to its file.

use std::io::Cursor;

use egui::color_picker::{Alpha, color_edit_button_srgba};
use egui::{Color32, Id, RichText, ScrollArea, Ui};
use plist::{Dictionary, Value};

use crate::theme::{Palette, parse_hex, to_hex};

/// The editor colours a theme can set, with the message ids of their names.
const GLOBALS: &[(&str, &str)] = &[
    ("background", "style-background"),
    ("foreground", "style-text"),
    ("caret", "style-caret"),
    ("selection", "style-selection"),
    ("lineHighlight", "style-current-line"),
    ("gutterForeground", "style-line-numbers"),
    ("guide", "style-indent-guides"),
    ("invisibles", "style-whitespace"),
    ("findHighlight", "style-search-matches"),
    ("accent", "style-accent"),
];

pub enum StyleEvent {
    /// Show the edited theme now (its `.tmTheme` text).
    Preview { name: String, source: String },
    /// Write the theme to `<name>.tmTheme` and use it.
    Save { name: String, source: String },
    /// Go back to the theme as saved.
    Discard,
    /// Edit another theme (and switch to it).
    Switch(String),
}

#[derive(Default)]
pub struct StyleEditor {
    pub open: bool,
    /// The theme being edited.
    name: String,
    doc: Option<Value>,
    dirty: bool,
    error: Option<String>,
    /// The name typed for Save As New Theme.
    new_name: Option<String>,
    filter: String,
    /// The theme as last saved, for Discard Changes.
    saved: String,
    focus_name: bool,
}

fn color_of(dict: &Dictionary, key: &str) -> Option<Color32> {
    dict.get(key).and_then(Value::as_string).and_then(parse_hex)
}

fn set_color(dict: &mut Dictionary, key: &str, color: Option<Color32>) {
    match color {
        Some(c) => {
            dict.insert(key.to_owned(), Value::String(to_hex(c)));
        }
        None => {
            dict.remove(key);
        }
    }
}

/// The theme's `settings` array: the editor colours first, then one entry per scope rule.
fn entries(doc: &mut Value) -> Option<&mut Vec<Value>> {
    doc.as_dictionary_mut()?.get_mut("settings")?.as_array_mut()
}

fn is_global(v: &Value) -> bool {
    v.as_dictionary().is_some_and(|d| !d.contains_key("scope"))
}

/// A colour button that can also be empty (the theme doesn't set it).
fn optional_color(ui: &mut Ui, color: &mut Option<Color32>, fallback: Color32) -> bool {
    let mut changed = false;
    match color {
        Some(c) => {
            changed |= color_edit_button_srgba(ui, c, Alpha::OnlyBlend).changed();
            if ui
                .small_button("×")
                .on_hover_text(t!("unset-colour"))
                .clicked()
            {
                *color = None;
                changed = true;
            }
        }
        None => {
            if ui
                .button(RichText::new(t!("set-colour")).small())
                .on_hover_text(t!("choose-colour"))
                .clicked()
            {
                *color = Some(fallback);
                changed = true;
            }
        }
    }
    changed
}

impl StyleEditor {
    /// Opens the editor on `name`, reading its `.tmTheme` text.
    pub fn show(&mut self, name: &str, source: &str) {
        self.open = true;
        self.load(name, source);
    }

    pub fn load(&mut self, name: &str, source: &str) {
        self.name = name.to_owned();
        self.saved = source.to_owned();
        self.dirty = false;
        self.new_name = None;
        match Value::from_reader_xml(Cursor::new(source.as_bytes())) {
            Ok(doc) => {
                self.doc = Some(doc);
                self.error = None;
            }
            Err(e) => {
                self.doc = None;
                self.error = Some(t!("theme-read-failed", error = e.to_string()));
            }
        }
    }

    fn source(&self) -> Option<String> {
        let mut out = Vec::new();
        self.doc.as_ref()?.to_writer_xml(&mut out).ok()?;
        String::from_utf8(out).ok()
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        themes: &[(String, bool)],
        palette: &Palette,
    ) -> Vec<StyleEvent> {
        let mut events = Vec::new();
        if !self.open {
            return events;
        }
        let mut open = true;
        let mut changed = false;
        egui::Window::new(t!("theme-colours"))
            .id(Id::new("style-editor"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([760.0, 560.0])
            .show(ctx, |ui| {
                self.toolbar(ui, themes, palette, &mut events);
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(palette.brace_bad.to_opaque()));
                }
                ui.separator();
                let Some(doc) = self.doc.as_mut() else {
                    return;
                };
                ScrollArea::vertical()
                    .id_salt("style-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        changed |= globals(ui, doc, palette);
                        ui.add_space(8.0);
                        changed |= rules(ui, doc, &mut self.filter, palette);
                    });
            });
        if changed {
            self.dirty = true;
            if let Some(source) = self.source() {
                events.push(StyleEvent::Preview {
                    name: self.name.clone(),
                    source,
                });
            }
        }
        if !open {
            self.open = false;
            if self.dirty {
                events.push(StyleEvent::Discard);
                self.dirty = false;
            }
        }
        events
    }

    fn toolbar(
        &mut self,
        ui: &mut Ui,
        themes: &[(String, bool)],
        palette: &Palette,
        events: &mut Vec<StyleEvent>,
    ) {
        ui.horizontal(|ui| {
            ui.label(t!("theme-label"));
            egui::ComboBox::from_id_salt("style-theme")
                .selected_text(&self.name)
                .width(180.0)
                .show_ui(ui, |ui| {
                    for (name, _) in themes {
                        if ui.selectable_label(*name == self.name, name).clicked()
                            && *name != self.name
                        {
                            if self.dirty {
                                events.push(StyleEvent::Discard);
                            }
                            events.push(StyleEvent::Switch(name.clone()));
                        }
                    }
                });
            let save = ui.add_enabled(
                self.dirty,
                egui::Button::new(t!("save")).fill(if self.dirty {
                    palette.accent.gamma_multiply(0.6)
                } else {
                    Color32::TRANSPARENT
                }),
            );
            if save.clicked()
                && let Some(source) = self.source()
            {
                self.saved = source.clone();
                events.push(StyleEvent::Save {
                    name: self.name.clone(),
                    source,
                });
                self.dirty = false;
            }
            if ui
                .add_enabled(self.dirty, egui::Button::new(t!("discard-changes")))
                .clicked()
            {
                events.push(StyleEvent::Discard);
                let (name, saved) = (self.name.clone(), self.saved.clone());
                self.load(&name, &saved);
            }
            if ui.button(t!("save-as-new-theme")).clicked() {
                self.new_name = Some(String::new());
                self.focus_name = true;
            }
        });
        if let Some(new_name) = &mut self.new_name {
            let mut done = None;
            ui.horizontal(|ui| {
                ui.label(t!("new-theme-name"));
                let r = ui.add(
                    egui::TextEdit::singleline(new_name)
                        .hint_text(t!("new-theme-name-hint"))
                        .desired_width(220.0),
                );
                if std::mem::take(&mut self.focus_name) {
                    r.request_focus();
                }
                let name = new_name.trim().to_owned();
                let taken = themes.iter().any(|(t, _)| t.eq_ignore_ascii_case(&name));
                let bad = name.is_empty()
                    || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']);
                let ok = !bad && !taken;
                if ui.add_enabled(ok, egui::Button::new(t!("save"))).clicked()
                    || (ok && r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    done = Some(Some(name.clone()));
                }
                if ui.button(t!("cancel")).clicked() {
                    done = Some(None);
                }
                if taken {
                    ui.label(RichText::new(t!("theme-name-taken")).weak());
                }
            });
            match done {
                Some(Some(name)) => {
                    if let Some(dict) = self.doc.as_mut().and_then(Value::as_dictionary_mut) {
                        dict.insert("name".to_owned(), Value::String(name.clone()));
                    }
                    if let Some(source) = self.source() {
                        if self.dirty {
                            events.push(StyleEvent::Discard);
                        }
                        self.saved = source.clone();
                        events.push(StyleEvent::Save {
                            name: name.clone(),
                            source,
                        });
                        self.name = name;
                        self.dirty = false;
                    }
                    self.new_name = None;
                }
                Some(None) => self.new_name = None,
                None => {}
            }
        }
        ui.label(RichText::new(t!("theme-colours-hint")).small().weak());
    }
}

/// The editor colours. Returns true when one changed.
fn globals(ui: &mut Ui, doc: &mut Value, palette: &Palette) -> bool {
    let Some(list) = entries(doc) else {
        return false;
    };
    if !list.iter().any(is_global) {
        list.insert(0, Value::Dictionary(Dictionary::new()));
    }
    let Some(global) = list
        .iter_mut()
        .find(|v| is_global(v))
        .and_then(Value::as_dictionary_mut)
    else {
        return false;
    };
    if !matches!(global.get("settings"), Some(Value::Dictionary(_))) {
        global.insert("settings".to_owned(), Value::Dictionary(Dictionary::new()));
    }
    let Some(settings) = global
        .get_mut("settings")
        .and_then(Value::as_dictionary_mut)
    else {
        return false;
    };
    let mut changed = false;
    ui.label(RichText::new(t!("style-editor")).strong());
    egui::Grid::new("style-globals")
        .num_columns(4)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            for (i, (key, label)) in GLOBALS.iter().enumerate() {
                ui.label(crate::i18n::tr(label));
                ui.horizontal(|ui| {
                    let mut c = color_of(settings, key);
                    if optional_color(ui, &mut c, palette.text) {
                        set_color(settings, key, c);
                        changed = true;
                    }
                });
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    changed
}

/// The syntax colours, one row per rule. Returns true when one changed.
fn rules(ui: &mut Ui, doc: &mut Value, filter: &mut String, palette: &Palette) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("style-syntax")).strong());
        ui.add_space(12.0);
        ui.label(t!("font-find-label"));
        ui.add(
            egui::TextEdit::singleline(filter)
                .hint_text(t!("rule-filter-hint"))
                .desired_width(280.0),
        );
    });
    ui.label(RichText::new(t!("rules-hint")).small().weak());
    let Some(list) = entries(doc) else {
        return false;
    };
    let needle = filter.to_lowercase();
    let mut remove = None;
    egui::Grid::new("style-rules")
        .num_columns(6)
        .striped(true)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(RichText::new(t!("rule-name")).weak());
            ui.label(RichText::new(t!("rule-scope")).weak());
            ui.label(RichText::new(t!("style-text")).weak());
            ui.label(RichText::new(t!("style-background")).weak());
            ui.label(RichText::new(t!("rule-style")).weak());
            ui.label("");
            ui.end_row();
            for (i, entry) in list.iter_mut().enumerate() {
                if is_global(entry) {
                    continue;
                }
                let Some(dict) = entry.as_dictionary_mut() else {
                    continue;
                };
                let mut name = dict
                    .get("name")
                    .and_then(Value::as_string)
                    .unwrap_or_default()
                    .to_owned();
                let mut scope = dict
                    .get("scope")
                    .and_then(Value::as_string)
                    .unwrap_or_default()
                    .to_owned();
                if !needle.is_empty()
                    && !name.to_lowercase().contains(&needle)
                    && !scope.to_lowercase().contains(&needle)
                {
                    continue;
                }
                if ui
                    .add_sized(
                        [150.0, 20.0],
                        egui::TextEdit::singleline(&mut name).hint_text(t!("no-name")),
                    )
                    .changed()
                {
                    dict.insert("name".to_owned(), Value::String(name));
                    changed = true;
                }
                if ui
                    .add_sized([260.0, 20.0], egui::TextEdit::singleline(&mut scope))
                    .on_hover_text(scope.as_str())
                    .changed()
                {
                    dict.insert("scope".to_owned(), Value::String(scope));
                    changed = true;
                }
                if !matches!(dict.get("settings"), Some(Value::Dictionary(_))) {
                    dict.insert("settings".to_owned(), Value::Dictionary(Dictionary::new()));
                }
                let Some(settings) = dict.get_mut("settings").and_then(Value::as_dictionary_mut)
                else {
                    continue;
                };
                for key in ["foreground", "background"] {
                    ui.horizontal(|ui| {
                        let mut c = color_of(settings, key);
                        let fallback = if key == "foreground" {
                            palette.text
                        } else {
                            palette.editor_bg
                        };
                        if optional_color(ui, &mut c, fallback) {
                            set_color(settings, key, c);
                            changed = true;
                        }
                    });
                }
                ui.horizontal(|ui| {
                    let style = settings
                        .get("fontStyle")
                        .and_then(Value::as_string)
                        .unwrap_or_default()
                        .to_owned();
                    let mut words: Vec<&str> = style.split_whitespace().collect();
                    for (word, label) in [("bold", "B"), ("italic", "I"), ("underline", "U")] {
                        let on = words.contains(&word);
                        let mut text = RichText::new(label);
                        text = match word {
                            "bold" => text.strong(),
                            "italic" => text.italics(),
                            _ => text.underline(),
                        };
                        if ui
                            .add(
                                egui::Button::new(text)
                                    .selected(on)
                                    .min_size(egui::vec2(22.0, 0.0)),
                            )
                            .on_hover_text(word)
                            .clicked()
                        {
                            if on {
                                words.retain(|w| *w != word);
                            } else {
                                words.push(word);
                            }
                            let joined = words.join(" ");
                            if joined.is_empty() {
                                settings.remove("fontStyle");
                            } else {
                                settings.insert("fontStyle".to_owned(), Value::String(joined));
                            }
                            changed = true;
                        }
                    }
                });
                if ui
                    .small_button("🗑")
                    .on_hover_text(t!("delete-rule"))
                    .clicked()
                {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        list.remove(i);
        changed = true;
    }
    if ui.button(t!("add-rule")).clicked() {
        let mut settings = Dictionary::new();
        settings.insert(
            "foreground".to_owned(),
            Value::String(to_hex(palette.accent)),
        );
        let mut rule = Dictionary::new();
        rule.insert("name".to_owned(), Value::String(t!("new-rule")));
        rule.insert(
            "scope".to_owned(),
            Value::String(if filter.trim().is_empty() {
                "keyword".to_owned()
            } else {
                filter.trim().to_owned()
            }),
        );
        rule.insert("settings".to_owned(), Value::Dictionary(settings));
        list.push(Value::Dictionary(rule));
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEME: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict><key>name</key><string>T</string><key>settings</key><array>
<dict><key>settings</key><dict><key>background</key><string>#101010</string></dict></dict>
<dict><key>name</key><string>Comment</string><key>scope</key><string>comment</string>
<key>settings</key><dict><key>foreground</key><string>#808080</string></dict></dict>
</array></dict></plist>"#;

    #[test]
    fn reads_and_writes_a_theme() {
        let mut ed = StyleEditor::default();
        ed.show("T", THEME);
        let doc = ed.doc.as_mut().unwrap();
        let list = entries(doc).unwrap();
        assert_eq!(list.len(), 2);
        assert!(is_global(&list[0]));
        let global = list[0]
            .as_dictionary_mut()
            .and_then(|d| d.get_mut("settings"))
            .and_then(Value::as_dictionary_mut)
            .unwrap();
        assert_eq!(
            color_of(global, "background"),
            Some(Color32::from_rgb(16, 16, 16))
        );
        set_color(global, "caret", Some(Color32::from_rgb(255, 0, 0)));
        let source = ed.source().unwrap();
        assert!(source.contains("#FF0000"));
        // syntect can read what we wrote.
        let theme = crate::theme::from_source("T", &source, true).unwrap();
        assert!(theme.syntax.settings.caret.is_some());
        assert_eq!(theme.syntax.scopes.len(), 1);
    }
}
