//! The shortcut mapper (Settings > Shortcut Mapper): see every command and
//! change, add or remove its keyboard shortcuts.

use std::collections::BTreeMap;

use egui::{Id, Key, KeyboardShortcut, RichText, Ui};

use crate::actions::{self, Action};
use crate::plugins::shortcut;
use crate::theme::Palette;

/// A command as the mapper shows it.
pub struct Row {
    pub path: String,
    pub action: Action,
    /// The shortcuts it has without the user's changes.
    pub defaults: Vec<KeyboardShortcut>,
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
}

#[derive(Clone, Copy, PartialEq)]
enum Capture {
    Replace,
    Add,
}

struct Conflict {
    shortcut: KeyboardShortcut,
    mode: Capture,
    other: Action,
    other_path: String,
}

/// The shortcuts a command has now.
pub fn current(row: &Row, saved: &BTreeMap<String, Vec<String>>) -> Vec<KeyboardShortcut> {
    match saved.get(&actions::command_key(&row.action)) {
        Some(list) if !row.fixed => list
            .iter()
            .filter_map(|t| shortcut::parse(t).ok())
            .collect(),
        _ => row.defaults.clone(),
    }
}

fn store(
    saved: &mut BTreeMap<String, Vec<String>>,
    row_defaults: &[KeyboardShortcut],
    action: &Action,
    list: &[KeyboardShortcut],
) {
    let key = actions::command_key(action);
    let same_as_default = list.len() == row_defaults.len()
        && list
            .iter()
            .zip(row_defaults)
            .all(|(a, b)| shortcut::same(a, b));
    if same_as_default {
        saved.remove(&key);
    } else {
        saved.insert(key, list.iter().map(shortcut::to_text).collect());
    }
}

fn is_modifier(key: Key) -> bool {
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
fn captured(ctx: &egui::Context) -> Option<Result<KeyboardShortcut, String>> {
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

impl ShortcutMapper {
    pub fn show(&mut self) {
        self.open = true;
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
        } else if let Some(result) = captured(ctx) {
            self.grabbed = Some(result);
        }
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

    /// Draws the mapper. Returns true when the saved shortcuts changed.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        rows: &[Row],
        saved: &mut BTreeMap<String, Vec<String>>,
        palette: &Palette,
    ) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;

        if let Some(mode) = self.capture
            && let Some(result) = self.grabbed.take()
        {
            {
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
        egui::Window::new(t!("shortcut-mapper"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .id(Id::new("shortcut-mapper"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([680.0, 520.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(t!("font-find-label"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.filter)
                            .hint_text(t!("command-or-shortcut"))
                            .desired_width(260.0),
                    );
                    ui.checkbox(&mut self.only_assigned, t!("only-assigned"));
                });
                ui.add_space(4.0);
                changed |= self.buttons(ui, rows, saved, palette);
                ui.separator();
                self.table(ui, rows, saved, palette);
            });
        if !open {
            self.open = false;
            self.capture = None;
            self.conflict = None;
        }
        changed
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
        let mut list = match mode {
            Capture::Replace => Vec::new(),
            Capture::Add => current(row, saved),
        };
        if !list.iter().any(|c| shortcut::same(c, &s)) {
            list.push(s);
        }
        store(saved, &row.defaults, &row.action, &list);
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
                        store(saved, &other.defaults, &other.action, &list);
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
                if self.capture.is_some() {
                    ui.label(
                        RichText::new(t!("press-shortcut"))
                            .color(palette.accent)
                            .strong(),
                    );
                    return;
                }
                let has = !current(row, saved).is_empty();
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
                    .add_enabled(has, egui::Button::new(t!("remove")))
                    .clicked()
                {
                    store(saved, &row.defaults, &row.action, &[]);
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
    ) {
        let ctx = ui.ctx().clone();
        let filter = self.filter.to_lowercase();
        let shown: Vec<(&Row, String, bool)> = rows
            .iter()
            .map(|r| {
                let keys = current(r, saved)
                    .iter()
                    .map(|s| ctx.format_shortcut(s))
                    .collect::<Vec<_>>()
                    .join(", ");
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
            .min_scrolled_height(340.0)
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
                });
            });
    }
}
