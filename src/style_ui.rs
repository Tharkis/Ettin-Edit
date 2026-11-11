//! The theme colour editor (Theme > Edit Theme Colours): change a theme's
//! editor and syntax colours, see them at once, and save them to its file.
//! The Simple view shows about ten named colours; Advanced shows every rule.

use std::io::Cursor;

use egui::color_picker::{Alpha, color_edit_button_srgba};
use egui::{Color32, Id, RichText, ScrollArea, Ui};
use plist::{Dictionary, Value};
use syntect::highlighting::Theme as SyntectTheme;
use syntect::parsing::{Scope, SyntaxSet};

use crate::theme::{Palette, parse_hex, to_hex};
use crate::theme_simple::{self as simple, Simple};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum View {
    #[default]
    Simple,
    Advanced,
}

/// What "What colours this?" found.
struct Picked {
    stack: Vec<Scope>,
    simple: Simple,
    /// The rule (index into the theme's settings) that colours it.
    rule: Option<usize>,
    /// Where it is in the preview, when picked there.
    in_preview: Option<std::ops::Range<usize>>,
}

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
    ("ettinTabStrip", "style-tab-strip"),
    ("ettinPanels", "style-panels"),
    ("ettinStatusBar", "style-status-bar"),
    ("ettinRibbon", "style-ribbon"),
];

pub enum StyleEvent {
    /// Show the edited theme now (its `.tmTheme` text).
    Preview { name: String, source: String },
    /// Write the theme to `<name>.tmTheme` (it is already shown).
    Write { name: String, source: String },
    /// Write a new theme and switch to it.
    SaveAsNew { name: String, source: String },
    /// Edit another theme (and switch to it).
    Switch(String),
}

/// Seconds without changes before an edit is written to the theme file.
const WRITE_AFTER: f64 = 0.5;

#[derive(Default)]
pub struct StyleEditor {
    pub open: bool,
    /// The theme being edited.
    name: String,
    doc: Option<Value>,
    error: Option<String>,
    /// The name typed for Save As New Theme.
    new_name: Option<String>,
    filter: String,
    /// The theme as it was when the window opened it, for Undo Changes.
    original: String,
    /// When the last change not yet written to the file was made.
    unwritten: Option<f64>,
    focus_name: bool,
    /// Frames left to keep the window on top after it opens (it can open
    /// from Preferences, which would otherwise stay in front).
    raise: u8,
    view: View,
    /// "What colours this?" is waiting for a click in the preview or the editor.
    picking: bool,
    picked: Option<Picked>,
    /// The Simple row to show as selected.
    selected: Option<Simple>,
    /// Which preview sample is shown.
    sample: usize,
    /// Scroll the Advanced view to the picked rule once.
    scroll_to_rule: bool,
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
            if crate::a11y::name(ui.small_button("×"), &t!("unset-colour")).clicked() {
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

/// The built-in copy of a theme, for Reset.
fn builtin_source(name: &str) -> Option<&'static str> {
    crate::theme::BUILTIN
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| *s)
}

impl StyleEditor {
    /// Opens the editor on `name`, reading its `.tmTheme` text.
    pub fn show(&mut self, name: &str, source: &str) {
        self.open = true;
        self.raise = 2;
        self.load(name, source);
    }

    /// Edits `name`; Undo Changes goes back to `source`.
    pub fn load(&mut self, name: &str, source: &str) {
        self.original = source.to_owned();
        self.unwritten = None;
        self.new_name = None;
        self.set_doc(name, source);
    }

    /// The theme being edited.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// True while "What colours this?" waits for a click; a click in the
    /// editor then goes to [`Self::pick`].
    pub fn picking(&self) -> bool {
        self.open && self.picking
    }

    /// Shows what colours text with these scopes (a click in the editor or the preview).
    pub fn pick(&mut self, stack: Vec<Scope>, in_preview: Option<std::ops::Range<usize>>) {
        self.picking = false;
        let which = simple::simple_for(&stack);
        let rule = self
            .doc
            .as_ref()
            .and_then(|d| simple::winning_rule(d, &stack));
        self.selected = Some(which);
        self.scroll_to_rule = true;
        self.raise = 2;
        self.picked = Some(Picked {
            stack,
            simple: which,
            rule,
            in_preview,
        });
    }

    fn set_doc(&mut self, name: &str, source: &str) {
        self.name = name.to_owned();
        self.picked = None;
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

    /// How many colours and styles differ from when the window opened.
    fn changes(&self) -> usize {
        match (self.doc.as_ref(), saved_doc(&self.original)) {
            (Some(now), Some(then)) => count_changes(&then, now),
            _ => 0,
        }
    }

    /// Writes a change that hasn't been written yet.
    fn flush(&mut self, events: &mut Vec<StyleEvent>) {
        if self.unwritten.take().is_some()
            && let Some(source) = self.source()
        {
            events.push(StyleEvent::Write {
                name: self.name.clone(),
                source,
            });
        }
    }

    /// Shows `source` and writes it at once (Undo Changes, Reset).
    fn replace_with(&mut self, source: &str, events: &mut Vec<StyleEvent>) {
        let name = self.name.clone();
        self.set_doc(&name, source);
        self.unwritten = None;
        // The text itself, so a reset file is byte for byte the built-in one.
        events.push(StyleEvent::Preview {
            name: name.clone(),
            source: source.to_owned(),
        });
        events.push(StyleEvent::Write {
            name,
            source: source.to_owned(),
        });
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        themes: &[(String, bool)],
        palette: &Palette,
        theme: Option<&SyntectTheme>,
        syntaxes: &SyntaxSet,
    ) -> Vec<StyleEvent> {
        let mut events = Vec::new();
        if !self.open {
            return events;
        }
        let mut open = true;
        let mut changed = false;
        let mut close = false;
        let shown = egui::Window::new(t!("theme-colours"))
            .id(Id::new("style-editor"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([980.0, 600.0])
            .show(ctx, |ui| {
                self.toolbar(ui, themes, &mut events);
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(palette.brace_bad.to_opaque()));
                }
                self.view_switch(ui);
                ui.separator();
                let body_h = (ui.available_height() - 36.0).max(120.0);
                if self.doc.is_some() {
                    egui::Panel::right("style-preview")
                        .resizable(true)
                        .default_size(330.0)
                        .min_size(220.0)
                        .show_separator_line(true)
                        .show(ui, |ui| {
                            ScrollArea::vertical()
                                .id_salt("style-preview-scroll")
                                .auto_shrink([false, false])
                                .max_height(body_h)
                                .show(ui, |ui| {
                                    if let Some(theme) = theme {
                                        self.preview(ui, theme, syntaxes, palette);
                                    }
                                });
                        });
                }
                let view = self.view;
                let selected = self.selected;
                let highlight = self.picked.as_ref().and_then(|p| p.rule);
                let scroll = std::mem::take(&mut self.scroll_to_rule);
                if let Some(doc) = self.doc.as_mut() {
                    ScrollArea::both()
                        .id_salt("style-scroll")
                        .auto_shrink([false, false])
                        .max_height(body_h)
                        .show(ui, |ui| match view {
                            View::Simple => {
                                if let Some(theme) = theme {
                                    changed |=
                                        simple_rows(ui, doc, theme, selected, &mut self.selected);
                                }
                                ui.add_space(10.0);
                                changed |= window_rows(ui, doc, palette);
                            }
                            View::Advanced => {
                                changed |= globals(ui, doc, palette);
                                ui.add_space(8.0);
                                changed |=
                                    rules(ui, doc, &mut self.filter, palette, highlight, scroll);
                            }
                        });
                }
                ui.separator();
                close = self.footer(ui, &mut events);
            });
        if self.raise > 0
            && let Some(shown) = shown
        {
            ctx.move_to_top(shown.response.layer_id);
            self.raise -= 1;
        }
        let now = ctx.input(|i| i.time);
        if changed {
            self.unwritten = Some(now);
            if let Some(source) = self.source() {
                events.push(StyleEvent::Preview {
                    name: self.name.clone(),
                    source,
                });
            }
        }
        if let Some(at) = self.unwritten {
            let wait = WRITE_AFTER - (now - at);
            if wait <= 0.0 {
                self.flush(&mut events);
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
            }
        }
        if !open || close {
            // Changes are kept: write the last one now.
            self.flush(&mut events);
            self.open = false;
            self.picking = false;
        }
        events
    }

    /// Simple, Advanced and "What colours this?".
    fn view_switch(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Simple, t!("theme-view-simple"))
                .on_hover_text(t!("theme-view-simple-tip"));
            ui.selectable_value(&mut self.view, View::Advanced, t!("theme-view-advanced"))
                .on_hover_text(t!("theme-view-advanced-tip"));
            ui.add_space(12.0);
            let pick = ui
                .selectable_label(self.picking, t!("what-colours-this"))
                .on_hover_text(t!("what-colours-this-tip"));
            if pick.clicked() {
                self.picking = !self.picking;
            }
            if self.picking {
                ui.label(
                    RichText::new(t!("what-colours-this-waiting"))
                        .small()
                        .weak(),
                );
            }
        });
    }

    /// The highlighted sample on the right, which "What colours this?" can be used on.
    fn preview(
        &mut self,
        ui: &mut Ui,
        theme: &SyntectTheme,
        syntaxes: &SyntaxSet,
        palette: &Palette,
    ) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("preview")).weak());
            egui::ComboBox::from_id_salt("style-sample")
                .selected_text(simple::SAMPLES[self.sample.min(simple::SAMPLES.len() - 1)].0)
                .show_ui(ui, |ui| {
                    for (i, (lang, _)) in simple::SAMPLES.iter().enumerate() {
                        if ui.selectable_value(&mut self.sample, i, *lang).changed() {
                            self.picked = None;
                        }
                    }
                });
        });
        let (lang, text) = simple::SAMPLES[self.sample.min(simple::SAMPLES.len() - 1)];
        let syntax = syntaxes
            .find_syntax_by_name(lang)
            .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
        let tokens = simple::highlight(text, syntax, syntaxes, theme);
        let marked = self.picked.as_ref().and_then(|p| p.in_preview.clone());
        let bg = theme
            .settings
            .background
            .map(|c| Color32::from_rgb(c.r, c.g, c.b))
            .unwrap_or(palette.editor_bg);
        let fg = theme
            .settings
            .foreground
            .map(|c| Color32::from_rgb(c.r, c.g, c.b))
            .unwrap_or(palette.editor_fg);
        let job = simple::layout(text, &tokens, egui::FontId::monospace(13.0), fg, marked);
        egui::Frame::new()
            .fill(bg)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click());
                if self.picking && response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                if response.clicked()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    let char_index = galley.cursor_from_pos(pos - rect.min).index.0;
                    let byte = text
                        .char_indices()
                        .nth(char_index)
                        .map(|(b, _)| b)
                        .unwrap_or(text.len());
                    if let Some(token) = tokens.iter().find(|t| t.range.contains(&byte)) {
                        let (stack, range) = (token.stack.clone(), token.range.clone());
                        self.pick(stack, Some(range));
                    }
                }
                ui.painter().galley(rect.min, galley, fg);
            });
        if !self.picking && self.picked.is_none() {
            ui.label(RichText::new(t!("what-colours-this-hint")).small().weak());
        }
        self.picked_note(ui, palette);
    }

    /// What "What colours this?" found, under the preview.
    fn picked_note(&mut self, ui: &mut Ui, palette: &Palette) {
        let Some(p) = &self.picked else {
            return;
        };
        let Some(doc) = self.doc.as_ref() else {
            return;
        };
        ui.add_space(6.0);
        ui.label(
            RichText::new(t!("picked-colour", name = p.simple.label()))
                .color(palette.accent)
                .strong(),
        );
        let scope = p.stack.last().map(|s| s.build_string()).unwrap_or_default();
        ui.label(RichText::new(t!("picked-scope", scope = scope)).small());
        ui.label(
            RichText::new(simple::stack_text(&p.stack))
                .font(egui::FontId::monospace(11.0))
                .weak(),
        )
        .on_hover_text(t!("picked-stack-tip"));
        match p.rule {
            Some(rule) => {
                ui.label(
                    RichText::new(t!("picked-rule", rule = simple::rule_label(doc, rule))).small(),
                );
                if p.simple != Simple::Text && !simple::rule_follows(doc, rule, p.simple) {
                    let note = if simple::follows_after_change(doc, &p.stack, p.simple) {
                        t!("picked-rule-shared", name = p.simple.label())
                    } else {
                        t!("picked-rule-separate", name = p.simple.label())
                    };
                    ui.label(RichText::new(note).small().weak());
                }
            }
            None => {
                ui.label(RichText::new(t!("picked-default")).small());
            }
        }
        if self.view == View::Simple
            && p.rule.is_some()
            && ui.small_button(t!("show-in-advanced")).clicked()
        {
            self.view = View::Advanced;
            self.scroll_to_rule = true;
        }
    }

    /// Undo Changes, Reset and Close. True when Close was pressed.
    fn footer(&mut self, ui: &mut Ui, events: &mut Vec<StyleEvent>) -> bool {
        let mut close = false;
        let changes = self.changes();
        let builtin = builtin_source(&self.name);
        let at_builtin = builtin.is_some_and(|b| {
            saved_doc(b)
                .zip(self.doc.as_ref())
                .is_some_and(|(b, now)| count_changes(&b, now) == 0)
        });
        ui.horizontal(|ui| {
            let hint = if changes == 0 {
                t!("changes-apply-live")
            } else {
                t!("theme-changes-count", count = changes)
            };
            ui.label(RichText::new(hint).small().weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(t!("close")).clicked() {
                    close = true;
                }
                let reset = ui
                    .add_enabled(
                        builtin.is_some() && !at_builtin,
                        egui::Button::new(t!("reset")),
                    )
                    .on_hover_text(if builtin.is_some() {
                        t!("theme-reset-tip", name = self.name.clone())
                    } else {
                        t!("theme-reset-custom-tip")
                    })
                    .on_disabled_hover_text(if builtin.is_some() {
                        t!("theme-reset-tip", name = self.name.clone())
                    } else {
                        t!("theme-reset-custom-tip")
                    });
                if reset.clicked()
                    && let Some(b) = builtin
                {
                    self.replace_with(b, events);
                }
                if ui
                    .add_enabled(changes > 0, egui::Button::new(t!("undo-changes")))
                    .on_hover_text(t!("undo-changes-tip"))
                    .clicked()
                {
                    let original = self.original.clone();
                    self.replace_with(&original, events);
                }
            });
        });
        close
    }

    fn toolbar(&mut self, ui: &mut Ui, themes: &[(String, bool)], events: &mut Vec<StyleEvent>) {
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
                            self.flush(events);
                            events.push(StyleEvent::Switch(name.clone()));
                        }
                    }
                });
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
                Some(Some(name)) => self.save_as_new(name, events),
                Some(None) => self.new_name = None,
                None => {}
            }
        }
        ui.label(RichText::new(t!("theme-colours-hint")).small().weak());
    }

    /// Saves the edited colours as a new theme and puts the theme they
    /// were made from back as it was when the window opened.
    fn save_as_new(&mut self, name: String, events: &mut Vec<StyleEvent>) {
        if let Some(dict) = self.doc.as_mut().and_then(Value::as_dictionary_mut) {
            dict.insert("name".to_owned(), Value::String(name.clone()));
        }
        let Some(source) = self.source() else {
            return;
        };
        self.unwritten = None;
        if self.changes() > 0 || saved_doc(&self.original).is_some() {
            events.push(StyleEvent::Preview {
                name: self.name.clone(),
                source: self.original.clone(),
            });
            events.push(StyleEvent::Write {
                name: self.name.clone(),
                source: self.original.clone(),
            });
        }
        events.push(StyleEvent::SaveAsNew {
            name: name.clone(),
            source: source.clone(),
        });
        self.load(&name, &source);
    }
}

fn saved_doc(source: &str) -> Option<Value> {
    Value::from_reader_xml(Cursor::new(source.as_bytes())).ok()
}

/// How many settings (colours and font styles) differ between two themes.
fn count_changes(a: &Value, b: &Value) -> usize {
    match (a, b) {
        (Value::Dictionary(a), Value::Dictionary(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter()
                .map(|k| match (a.get(k), b.get(k)) {
                    (Some(x), Some(y)) => count_changes(x, y),
                    _ => 1,
                })
                .sum()
        }
        (Value::Array(a), Value::Array(b)) => {
            // Theme rules are matched by scope, so a rule added at the top
            // counts once rather than shifting every rule after it.
            let key = |v: &Value| {
                v.as_dictionary()
                    .map(|d| d.get("scope").and_then(Value::as_string).unwrap_or(""))
                    .map(str::to_owned)
            };
            if a.iter().chain(b.iter()).all(|v| key(v).is_some()) {
                let mut unmatched: Vec<&Value> = b.iter().collect();
                let mut total = 0;
                for x in a {
                    match unmatched.iter().position(|y| key(y) == key(x)) {
                        Some(i) => total += count_changes(x, unmatched.remove(i)),
                        None => total += 1,
                    }
                }
                return total + unmatched.len();
            }
            let shared: usize = a.iter().zip(b).map(|(x, y)| count_changes(x, y)).sum();
            shared + a.len().abs_diff(b.len())
        }
        (x, y) => usize::from(x != y),
    }
}

/// The editor colours entry of a theme (the one without a scope), made if missing.
fn global_settings(doc: &mut Value) -> Option<&mut Dictionary> {
    let list = entries(doc)?;
    if !list.iter().any(is_global) {
        list.insert(0, Value::Dictionary(Dictionary::new()));
    }
    let global = list
        .iter_mut()
        .find(|v| is_global(v))
        .and_then(Value::as_dictionary_mut)?;
    if !matches!(global.get("settings"), Some(Value::Dictionary(_))) {
        global.insert("settings".to_owned(), Value::Dictionary(Dictionary::new()));
    }
    global
        .get_mut("settings")
        .and_then(Value::as_dictionary_mut)
}

/// The editor colours. Returns true when one changed.
fn globals(ui: &mut Ui, doc: &mut Value, palette: &Palette) -> bool {
    let Some(settings) = global_settings(doc) else {
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

/// A B, I or U toggle. Returns true when clicked.
fn style_toggle(ui: &mut Ui, on: &mut bool, label: &str, name: &str) -> bool {
    let mut text = RichText::new(label);
    text = match label {
        "B" => text.strong(),
        "I" => text.italics(),
        _ => text.underline(),
    };
    let clicked = crate::a11y::toggle(
        ui.add(
            egui::Button::new(text)
                .selected(*on)
                .min_size(egui::vec2(22.0, 0.0)),
        ),
        *on,
        name,
    )
    .clicked();
    if clicked {
        *on = !*on;
    }
    clicked
}

/// The Simple view's named colours. Returns true when one changed.
fn simple_rows(
    ui: &mut Ui,
    doc: &mut Value,
    theme: &SyntectTheme,
    selected: Option<Simple>,
    select: &mut Option<Simple>,
) -> bool {
    let mut changed = false;
    ui.label(RichText::new(t!("theme-simple-hint")).small().weak());
    ui.add_space(4.0);
    egui::Grid::new("style-simple")
        .num_columns(3)
        .spacing([16.0, 6.0])
        .min_col_width(150.0)
        .show(ui, |ui| {
            for which in simple::ALL {
                let mut look = simple::look(theme, which);
                let label = ui
                    .selectable_label(
                        selected == Some(which),
                        RichText::new(which.label()).strong(),
                    )
                    .on_hover_text(if which == Simple::Text {
                        t!("simple-text-tip")
                    } else {
                        t!("simple-scopes-tip", scopes = which.scopes().join(", "))
                    });
                if label.clicked() {
                    *select = Some(which);
                }
                if selected == Some(which) {
                    label.scroll_to_me(None);
                }
                let mut edited = false;
                ui.horizontal(|ui| {
                    edited |= crate::a11y::name(
                        color_edit_button_srgba(ui, &mut look.foreground, Alpha::Opaque),
                        &t!("simple-colour-name", name = which.label()),
                    )
                    .changed();
                    if let Some(bg) = look.background.as_mut() {
                        edited |= crate::a11y::name(
                            color_edit_button_srgba(ui, bg, Alpha::OnlyBlend),
                            &t!("simple-background-name", name = which.label()),
                        )
                        .on_hover_text(t!("style-background"))
                        .changed();
                    }
                });
                ui.horizontal(|ui| {
                    if which != Simple::Text {
                        edited |= style_toggle(ui, &mut look.bold, "B", &t!("style-bold"));
                        edited |= style_toggle(ui, &mut look.italic, "I", &t!("style-italic"));
                        edited |=
                            style_toggle(ui, &mut look.underline, "U", &t!("style-underline"));
                    }
                });
                ui.end_row();
                if edited {
                    simple::set_look(doc, which, &look);
                    *select = Some(which);
                    changed = true;
                }
            }
        });
    changed
}

/// Tab strip, panels, status bar, ribbon and selection. Returns true when one changed.
fn window_rows(ui: &mut Ui, doc: &mut Value, palette: &Palette) -> bool {
    let Some(settings) = global_settings(doc) else {
        return false;
    };
    let mut changed = false;
    ui.label(RichText::new(t!("style-window")).strong());
    ui.label(RichText::new(t!("style-window-hint")).small().weak());
    egui::Grid::new("style-window")
        .num_columns(2)
        .spacing([16.0, 6.0])
        .min_col_width(150.0)
        .show(ui, |ui| {
            for (key, label) in simple::WINDOW {
                ui.label(crate::i18n::tr(label));
                let fallback = match *key {
                    "ettinTabStrip" => palette.tab_strip_bg,
                    "ettinStatusBar" => palette.status_bg,
                    "ettinRibbon" => palette.ribbon_bg,
                    "selection" => palette.selection,
                    _ => palette.chrome_bg,
                };
                ui.horizontal(|ui| {
                    let mut c = color_of(settings, key);
                    if optional_color(ui, &mut c, fallback) {
                        set_color(settings, key, c);
                        changed = true;
                    }
                    if c.is_none() {
                        ui.label(RichText::new(t!("style-window-automatic")).small().weak());
                    }
                });
                ui.end_row();
            }
        });
    changed
}

/// The syntax colours, one row per rule. Returns true when one changed.
fn rules(
    ui: &mut Ui,
    doc: &mut Value,
    filter: &mut String,
    palette: &Palette,
    highlight: Option<usize>,
    scroll: bool,
) -> bool {
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
        .num_columns(7)
        .striped(true)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("");
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
                let picked = highlight == Some(i);
                if !picked
                    && !needle.is_empty()
                    && !name.to_lowercase().contains(&needle)
                    && !scope.to_lowercase().contains(&needle)
                {
                    continue;
                }
                let marker = ui.label(if picked {
                    RichText::new("▶").color(palette.accent)
                } else {
                    RichText::new(" ")
                });
                if picked {
                    marker.on_hover_text(t!("picked-rule-marker"));
                    if scroll {
                        ui.scroll_to_cursor(Some(egui::Align::Center));
                    }
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
                    for (word, label, name) in [
                        ("bold", "B", t!("style-bold")),
                        ("italic", "I", t!("style-italic")),
                        ("underline", "U", t!("style-underline")),
                    ] {
                        let on = words.contains(&word);
                        let mut text = RichText::new(label);
                        text = match word {
                            "bold" => text.strong(),
                            "italic" => text.italics(),
                            _ => text.underline(),
                        };
                        if crate::a11y::toggle(
                            ui.add(
                                egui::Button::new(text)
                                    .selected(on)
                                    .min_size(egui::vec2(22.0, 0.0)),
                            ),
                            on,
                            &name,
                        )
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
                if crate::a11y::name(ui.small_button("🗑"), &t!("delete-rule")).clicked() {
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

    #[test]
    fn counts_changed_settings() {
        let saved = saved_doc(THEME).unwrap();
        let mut ed = StyleEditor::default();
        ed.show("T", THEME);
        assert_eq!(count_changes(&saved, ed.doc.as_ref().unwrap()), 0);
        let list = entries(ed.doc.as_mut().unwrap()).unwrap();
        let global = list[0]
            .as_dictionary_mut()
            .and_then(|d| d.get_mut("settings"))
            .and_then(Value::as_dictionary_mut)
            .unwrap();
        set_color(global, "background", Some(Color32::from_rgb(1, 2, 3)));
        set_color(global, "caret", Some(Color32::from_rgb(255, 0, 0)));
        assert_eq!(count_changes(&saved, ed.doc.as_ref().unwrap()), 2);
    }

    /// Undo Changes puts back the theme as it was when the window opened
    /// and writes it to the file.
    #[test]
    fn undo_changes_goes_back_and_writes() {
        let mut ed = StyleEditor::default();
        ed.show("T", THEME);
        let list = entries(ed.doc.as_mut().unwrap()).unwrap();
        let global = list[0]
            .as_dictionary_mut()
            .and_then(|d| d.get_mut("settings"))
            .and_then(Value::as_dictionary_mut)
            .unwrap();
        set_color(global, "caret", Some(Color32::from_rgb(255, 0, 0)));
        assert_eq!(ed.changes(), 1);
        let mut events = Vec::new();
        ed.unwritten = Some(0.0);
        ed.flush(&mut events);
        assert!(
            matches!(&events[0], StyleEvent::Write { source, .. } if source.contains("#FF0000"))
        );
        events.clear();
        let original = ed.original.clone();
        ed.replace_with(&original, &mut events);
        assert_eq!(ed.changes(), 0);
        assert!(
            matches!(&events[1], StyleEvent::Write { source, .. } if !source.contains("#FF0000"))
        );
        // Closing keeps the changes: nothing waits to be written.
        assert!(ed.unwritten.is_none());
    }

    #[test]
    fn reset_knows_the_built_in_themes() {
        assert!(builtin_source("Ettin Dark").is_some());
        assert!(builtin_source("Mine").is_none());
    }
}
