//! The shortcut mapper (Settings > Shortcut Mapper): see every command and
//! change, add or remove its keyboard shortcuts.

use std::collections::BTreeMap;

use egui::{Id, Key, KeyboardShortcut, RichText, Ui};

use crate::actions::{self, Action};
use crate::plugins::shortcut::{self, Chord};
use crate::settings::Keymap;
use crate::theme::Palette;

/// A command as the mapper shows it.
pub struct Row {
    pub path: String,
    pub action: Action,
    /// The shortcuts it has without the user's changes.
    pub defaults: Vec<KeyboardShortcut>,
    /// Its two-key chords without the user's changes.
    pub default_chords: Vec<Chord>,
    /// Handled by the text box itself (Undo, Copy…), so it can't be changed.
    pub fixed: bool,
}

#[derive(Default)]
pub struct ShortcutMapper {
    pub open: bool,
    filter: String,
    only_assigned: bool,
    selected: Option<Action>,
    /// Waiting for a key press: replace the shortcuts, or add one.
    capture: Option<Capture>,
    /// A captured shortcut another command already uses.
    conflict: Option<Conflict>,
    error: Option<String>,
    /// A key press taken at the start of the frame, before the editor sees it.
    grabbed: Option<Result<KeyboardShortcut, String>>,
    /// Frames left to keep the window on top after it opens (it can open
    /// from Preferences, which would otherwise stay in front).
    raise: u8,
    focus_filter: bool,
    /// The shortcuts and keymap when the window opened, for Undo Changes.
    opened: Option<(BTreeMap<String, Vec<String>>, Keymap)>,
}

#[derive(Clone, Copy, PartialEq)]
enum Capture {
    Replace,
    Add,
    /// A chord's first key, then its second.
    ChordFirst,
    ChordSecond(KeyboardShortcut),
}

struct Conflict {
    shortcut: KeyboardShortcut,
    mode: Capture,
    other: Action,
    other_path: String,
}

/// The Keymap drop-down. True when it changed.
fn keymap_picker(ui: &mut Ui, keymap: &mut Keymap) -> bool {
    let before = *keymap;
    let label = |k: Keymap| match k {
        Keymap::Ettin => t!("keymap-ettin"),
        Keymap::VsCode => t!("keymap-vs-code"),
    };
    ui.horizontal(|ui| {
        ui.label(t!("keymap-label"));
        egui::ComboBox::from_id_salt("keymap")
            .selected_text(label(*keymap))
            .width(200.0)
            .show_ui(ui, |ui| {
                for k in [Keymap::Ettin, Keymap::VsCode] {
                    ui.selectable_value(keymap, k, label(k));
                }
            });
        ui.label(RichText::new(t!("keymap-hint")).small().weak());
    });
    *keymap != before
}

/// The shortcuts a command has now.
pub fn current(row: &Row, saved: &BTreeMap<String, Vec<String>>) -> Vec<KeyboardShortcut> {
    match saved.get(&actions::command_key(&row.action)) {
        Some(list) if !row.fixed => list
            .iter()
            .filter(|t| !shortcut::is_chord(t))
            .filter_map(|t| shortcut::parse(t).ok())
            .collect(),
        _ => row.defaults.clone(),
    }
}

/// The two-key chords a command has now.
pub fn current_chords(row: &Row, saved: &BTreeMap<String, Vec<String>>) -> Vec<Chord> {
    match saved.get(&actions::command_key(&row.action)) {
        Some(list) if !row.fixed => list
            .iter()
            .filter(|t| shortcut::is_chord(t))
            .filter_map(|t| shortcut::parse_chord(t).ok())
            .collect(),
        _ => row.default_chords.clone(),
    }
}

/// Saves a command's shortcuts and chords; back to its defaults, nothing
/// is saved for it.
pub fn store(
    saved: &mut BTreeMap<String, Vec<String>>,
    row: &Row,
    list: &[KeyboardShortcut],
    chords: &[Chord],
) {
    let key = actions::command_key(&row.action);
    let same_as_default = list.len() == row.defaults.len()
        && list
            .iter()
            .zip(&row.defaults)
            .all(|(a, b)| shortcut::same(a, b))
        && chords.len() == row.default_chords.len()
        && chords
            .iter()
            .zip(&row.default_chords)
            .all(|(a, b)| shortcut::same_chord(a, b));
    if same_as_default {
        saved.remove(&key);
    } else {
        let mut texts: Vec<String> = list.iter().map(shortcut::to_text).collect();
        texts.extend(chords.iter().map(shortcut::chord_to_text));
        saved.insert(key, texts);
    }
}

/// The command's shortcuts and chords as the table shows them.
fn keys_text(ctx: &egui::Context, row: &Row, saved: &BTreeMap<String, Vec<String>>) -> String {
    current(row, saved)
        .iter()
        .map(|s| ctx.format_shortcut(s))
        .chain(
            current_chords(row, saved)
                .iter()
                .map(|c| shortcut::format_chord(ctx, c)),
        )
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn is_modifier(key: Key) -> bool {
    matches!(
        key,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight
            | Key::SuperLeft
            | Key::SuperRight
    )
}

/// Reads one shortcut from this frame's key presses.
pub fn captured(ctx: &egui::Context) -> Option<Result<KeyboardShortcut, String>> {
    ctx.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if (*key == Key::Escape && modifiers.is_none()) || is_modifier(*key) {
                    return None;
                }
                let s = KeyboardShortcut::new(
                    egui::Modifiers {
                        command: modifiers.command || modifiers.ctrl,
                        ctrl: false,
                        mac_cmd: false,
                        ..*modifiers
                    },
                    *key,
                );
                Some(shortcut::parse(&shortcut::to_text(&s)))
            }
            egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_) => {
                Some(Err(t!("shortcut-reserved-clipboard")))
            }
            _ => None,
        })
    })
}

/// Reads one key press of this frame, with or without modifiers (a
/// chord's second key).
fn captured_any(ctx: &egui::Context) -> Option<KeyboardShortcut> {
    ctx.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if !is_modifier(*key) => Some(KeyboardShortcut::new(
                egui::Modifiers {
                    command: modifiers.command || modifiers.ctrl,
                    ctrl: false,
                    mac_cmd: false,
                    ..*modifiers
                },
                *key,
            )),
            _ => None,
        })
    })
}

/// Takes this frame's key presses away, so nothing else acts on them while
/// a shortcut is being picked.
pub fn swallow_keys(ctx: &egui::Context) {
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            !matches!(
                e,
                egui::Event::Key { .. }
                    | egui::Event::Text(_)
                    | egui::Event::Copy
                    | egui::Event::Cut
                    | egui::Event::Paste(_)
            )
        })
    });
}

/// The command (other than `action`) that already uses `s`, if any.
pub fn used_by<'a>(
    rows: &'a [Row],
    saved: &BTreeMap<String, Vec<String>>,
    action: &Action,
    s: &KeyboardShortcut,
) -> Option<&'a Row> {
    rows.iter()
        .find(|r| &r.action != action && current(r, saved).iter().any(|c| shortcut::same(c, s)))
}

impl ShortcutMapper {
    pub fn show(&mut self, saved: &BTreeMap<String, Vec<String>>, keymap: Keymap) {
        if !self.open {
            self.opened = Some((saved.clone(), keymap));
        }
        self.open = true;
        self.raise = 2;
        self.focus_filter = true;
        self.capture = None;
        self.conflict = None;
        self.error = None;
    }

    /// While waiting for a shortcut, takes this frame's key presses so
    /// nothing else acts on them. Call before anything else reads input.
    pub fn grab_keys(&mut self, ctx: &egui::Context) {
        if !self.open || self.capture.is_none() {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape) && i.modifiers.is_none()) {
            self.capture = None;
        } else if let Some(Capture::ChordSecond(_)) = self.capture {
            if let Some(s) = captured_any(ctx) {
                self.grabbed = Some(shortcut::parse_second(&shortcut::to_text(&s)));
            }
        } else if let Some(result) = captured(ctx) {
            self.grabbed = Some(result);
        }
        swallow_keys(ctx);
    }

    /// Draws the mapper. Returns true when the saved shortcuts changed.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        rows: &[Row],
        saved: &mut BTreeMap<String, Vec<String>>,
        keymap: &mut Keymap,
        palette: &Palette,
    ) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;

        if let Some(mode) = self.capture
            && let Some(result) = self.grabbed.take()
        {
            if matches!(mode, Capture::ChordFirst | Capture::ChordSecond(_)) {
                changed |= self.chord_key(rows, saved, mode, result);
            } else {
                self.capture = None;
                match result {
                    Err(e) => self.error = Some(e),
                    Ok(s) => {
                        let selected = self.selected.clone();
                        let other = rows.iter().find(|r| {
                            Some(&r.action) != selected.as_ref()
                                && current(r, saved).iter().any(|c| shortcut::same(c, &s))
                        });
                        match other {
                            Some(r) => {
                                self.conflict = Some(Conflict {
                                    shortcut: s,
                                    mode,
                                    other: r.action.clone(),
                                    other_path: r.path.clone(),
                                })
                            }
                            None => {
                                changed |= self.assign(rows, saved, s, mode);
                            }
                        }
                    }
                }
            }
        }

        let mut open = true;
        let mut close = false;
        let shown = egui::Window::new(t!("shortcut-mapper"))
            .default_rect(crate::dialog::centered(ctx, egui::vec2(680.0, 520.0)))
            .id(Id::new("shortcut-mapper"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .min_size([520.0, 360.0])
            .show(ctx, |ui| {
                changed |= keymap_picker(ui, keymap);
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(t!("font-find-label"));
                    let find = ui.add(
                        egui::TextEdit::singleline(&mut self.filter)
                            .hint_text(t!("command-or-shortcut"))
                            .desired_width(260.0),
                    );
                    if std::mem::take(&mut self.focus_filter) {
                        find.request_focus();
                    }
                    ui.checkbox(&mut self.only_assigned, t!("only-assigned"));
                });
                ui.add_space(4.0);
                changed |= self.buttons(ui, rows, saved, palette);
                ui.separator();
                // The table takes the height the window has (Ettin Edit 2.6).
                let table_h = crate::dialog::body_height(ui, Id::new("shortcut-mapper"), 34.0);
                self.table(ui, rows, saved, palette, table_h);
                crate::dialog::footer(ui, Id::new("shortcut-mapper"), |ui| {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t!("changes-apply-live")).small().weak());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t!("close")).clicked() {
                                close = true;
                            }
                            let differs = self
                                .opened
                                .as_ref()
                                .is_some_and(|(s, k)| s != saved || k != keymap);
                            if ui
                                .add_enabled(differs, egui::Button::new(t!("undo-changes")))
                                .on_hover_text(t!("undo-changes-tip"))
                                .clicked()
                            {
                                changed |= self.undo_changes(saved, keymap);
                            }
                        });
                    });
                });
            });
        if self.raise > 0
            && let Some(shown) = shown
        {
            ctx.move_to_top(shown.response.layer_id);
            self.raise -= 1;
        }
        if !open || close {
            self.open = false;
            self.capture = None;
            self.conflict = None;
            self.opened = None;
        }
        changed
    }

    /// A chord key was pressed: remember the first, or give the selected
    /// command the chord.
    fn chord_key(
        &mut self,
        rows: &[Row],
        saved: &mut BTreeMap<String, Vec<String>>,
        mode: Capture,
        key: Result<KeyboardShortcut, String>,
    ) -> bool {
        let key = match key {
            Ok(k) => k,
            Err(e) => {
                self.capture = None;
                self.error = Some(e);
                return false;
            }
        };
        let Capture::ChordSecond(first) = mode else {
            // The first key can't also be a command's own shortcut.
            if let Some(r) = rows
                .iter()
                .find(|r| current(r, saved).iter().any(|c| shortcut::same(c, &key)))
            {
                self.capture = None;
                self.error = Some(t!(
                    "chord-first-in-use",
                    shortcut = shortcut::to_text(&key),
                    command = r.path.clone()
                ));
                return false;
            }
            self.capture = Some(Capture::ChordSecond(key));
            return false;
        };
        self.capture = None;
        let chord = (first, key);
        let Some(row) = self
            .selected
            .as_ref()
            .and_then(|a| rows.iter().find(|r| &r.action == a))
        else {
            return false;
        };
        if let Some(other) = rows.iter().find(|r| {
            r.action != row.action
                && current_chords(r, saved)
                    .iter()
                    .any(|c| shortcut::same_chord(c, &chord))
        }) {
            self.error = Some(t!(
                "chord-in-use",
                chord = shortcut::chord_to_text(&chord),
                command = other.path.clone()
            ));
            return false;
        }
        let mut chords = current_chords(row, saved);
        if !chords.iter().any(|c| shortcut::same_chord(c, &chord)) {
            chords.push(chord);
        }
        store(saved, row, &current(row, saved), &chords);
        self.error = None;
        true
    }

    /// Puts back the shortcuts and keymap from when the window opened.
    fn undo_changes(
        &mut self,
        saved: &mut BTreeMap<String, Vec<String>>,
        keymap: &mut Keymap,
    ) -> bool {
        let Some((s, k)) = &self.opened else {
            return false;
        };
        *saved = s.clone();
        *keymap = *k;
        self.capture = None;
        self.conflict = None;
        true
    }

    /// Gives the selected command `s`, replacing or adding to its shortcuts.
    fn assign(
        &mut self,
        rows: &[Row],
        saved: &mut BTreeMap<String, Vec<String>>,
        s: KeyboardShortcut,
        mode: Capture,
    ) -> bool {
        let Some(row) = self
            .selected
            .as_ref()
            .and_then(|a| rows.iter().find(|r| &r.action == a))
        else {
            return false;
        };
        // A chord's first key waits for a second, so it can't run a command.
        if let Some(r) = rows.iter().find(|r| {
            current_chords(r, saved)
                .iter()
                .any(|c| shortcut::same(&c.0, &s))
        }) {
            self.error = Some(t!(
                "shortcut-starts-chord",
                shortcut = shortcut::to_text(&s),
                command = r.path.clone()
            ));
            return false;
        }
        let mut list = match mode {
            Capture::Replace => Vec::new(),
            _ => current(row, saved),
        };
        if !list.iter().any(|c| shortcut::same(c, &s)) {
            list.push(s);
        }
        store(saved, row, &list, &current_chords(row, saved));
        self.error = None;
        true
    }

    fn buttons(
        &mut self,
        ui: &mut Ui,
        rows: &[Row],
        saved: &mut BTreeMap<String, Vec<String>>,
        palette: &Palette,
    ) -> bool {
        let mut changed = false;
        let row = self
            .selected
            .as_ref()
            .and_then(|a| rows.iter().find(|r| &r.action == a));

        if let Some(conflict) = &self.conflict {
            let text = t!(
                "shortcut-in-use",
                shortcut = ui.ctx().format_shortcut(&conflict.shortcut),
                command = conflict.other_path.clone()
            );
            let mut decided = None;
            egui::Frame::new()
                .fill(palette.find_hit)
                .corner_radius(4)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(text);
                        if ui.button(t!("move-it-here")).clicked() {
                            decided = Some(true);
                        }
                        if ui.button(t!("cancel")).clicked() {
                            decided = Some(false);
                        }
                    });
                });
            if let Some(take) = decided {
                let conflict = self.conflict.take().expect("checked above");
                if take {
                    if let Some(other) = rows.iter().find(|r| r.action == conflict.other) {
                        let list: Vec<KeyboardShortcut> = current(other, saved)
                            .into_iter()
                            .filter(|c| !shortcut::same(c, &conflict.shortcut))
                            .collect();
                        store(saved, other, &list, &current_chords(other, saved));
                    }
                    changed |= self.assign(rows, saved, conflict.shortcut, conflict.mode);
                }
            }
            return changed;
        }

        ui.horizontal(|ui| match row {
            None => {
                ui.label(RichText::new(t!("select-command")).weak());
            }
            Some(row) if row.fixed => {
                ui.label(RichText::new(t!("shortcut-fixed", command = row.path.clone())).weak());
            }
            Some(row) => {
                if let Some(mode) = self.capture {
                    let text = match mode {
                        Capture::ChordFirst => t!("press-chord-first"),
                        Capture::ChordSecond(first) => {
                            t!("press-chord-second", key = ui.ctx().format_shortcut(&first))
                        }
                        _ => t!("press-shortcut"),
                    };
                    ui.label(RichText::new(text).color(palette.accent).strong());
                    return;
                }
                let has = !current(row, saved).is_empty();
                let has_chord = !current_chords(row, saved).is_empty();
                if ui
                    .button(if has {
                        t!("change-shortcut")
                    } else {
                        t!("set-shortcut")
                    })
                    .clicked()
                {
                    self.capture = Some(Capture::Replace);
                    self.error = None;
                }
                if has && ui.button(t!("add-another-shortcut")).clicked() {
                    self.capture = Some(Capture::Add);
                    self.error = None;
                }
                if ui
                    .button(t!("add-chord"))
                    .on_hover_text(t!("add-chord-tip"))
                    .clicked()
                {
                    self.capture = Some(Capture::ChordFirst);
                    self.error = None;
                }
                if ui
                    .add_enabled(has || has_chord, egui::Button::new(t!("remove")))
                    .clicked()
                {
                    store(saved, row, &[], &[]);
                    changed = true;
                }
                let modified = saved.contains_key(&actions::command_key(&row.action));
                if ui
                    .add_enabled(modified, egui::Button::new(t!("reset-to-default")))
                    .clicked()
                {
                    saved.remove(&actions::command_key(&row.action));
                    changed = true;
                }
            }
        });
        if let Some(e) = &self.error {
            ui.label(RichText::new(e).color(palette.brace_bad.to_opaque()));
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !saved.is_empty(),
                    egui::Button::new(t!("reset-all-shortcuts")),
                )
                .clicked()
            {
                saved.clear();
                changed = true;
            }
            ui.label(RichText::new(t!("shortcut-rule")).weak().small());
        });
        changed
    }

    fn table(
        &mut self,
        ui: &mut Ui,
        rows: &[Row],
        saved: &BTreeMap<String, Vec<String>>,
        palette: &Palette,
        height: f32,
    ) {
        let ctx = ui.ctx().clone();
        let filter = self.filter.to_lowercase();
        let shown: Vec<(&Row, String, bool)> = rows
            .iter()
            .map(|r| {
                let keys = keys_text(&ctx, r, saved);
                let modified = saved.contains_key(&actions::command_key(&r.action));
                (r, keys, modified)
            })
            .filter(|(r, keys, _)| {
                (!self.only_assigned || !keys.is_empty())
                    && (filter.is_empty()
                        || r.path.to_lowercase().contains(&filter)
                        || keys.to_lowercase().contains(&filter))
            })
            .collect();
        let row_h = 22.0;
        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .auto_shrink([false, false])
            .min_scrolled_height(height - 28.0)
            // The window's height, so the window doesn't grow to fit it.
            .max_scroll_height(height - 28.0)
            .sense(egui::Sense::click())
            .column(egui_extras::Column::remainder().at_least(260.0).clip(true))
            .column(egui_extras::Column::exact(200.0))
            .header(row_h, |mut h| {
                h.col(|ui| {
                    ui.strong(t!("command"));
                });
                h.col(|ui| {
                    ui.strong(t!("shortcut"));
                });
            })
            .body(|body| {
                body.rows(row_h, shown.len(), |mut table_row| {
                    let (row, keys, modified) = &shown[table_row.index()];
                    let selected = self.selected.as_ref() == Some(&row.action);
                    table_row.set_selected(selected);
                    table_row.col(|ui| {
                        let text = RichText::new(&row.path);
                        ui.label(if row.fixed { text.weak() } else { text });
                    });
                    table_row.col(|ui| {
                        let mut text = RichText::new(keys.as_str());
                        if *modified {
                            text = text.color(palette.accent);
                        }
                        ui.label(text);
                    });
                    let response = table_row.response();
                    if response.clicked() {
                        self.selected = Some(row.action.clone());
                        self.capture = None;
                        self.error = None;
                    }
                    if response.double_clicked() && !row.fixed {
                        self.selected = Some(row.action.clone());
                        self.capture = Some(Capture::Replace);
                    }
                    response.context_menu(|ui| {
                        if ui
                            .button(t!("copy-command-id"))
                            .on_hover_text(t!("copy-command-id-hover"))
                            .clicked()
                        {
                            ui.ctx().copy_text(actions::command_id(&row.action));
                            ui.close();
                        }
                    });
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_changes_restores_shortcuts_and_keymap() {
        let mut m = ShortcutMapper::default();
        let mut saved = BTreeMap::from([("\"Save\"".to_owned(), vec!["F12".to_owned()])]);
        let mut keymap = Keymap::Ettin;
        m.show(&saved, keymap);
        saved.clear();
        keymap = Keymap::VsCode;
        assert!(m.undo_changes(&mut saved, &mut keymap));
        assert_eq!(keymap, Keymap::Ettin);
        assert_eq!(saved.len(), 1);
    }
}
