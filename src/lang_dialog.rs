//! Language > Define Language…: a form for a simple user-defined language,
//! with a live preview. It writes the same `.lang.json` files that
//! [`crate::user_lang`] reads, so "Edit JSON instead" is always possible.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{Id, RichText, ScrollArea, Ui};
use serde_json::{Map, Value, json};
use syntect::highlighting::Theme as SyntectTheme;
use syntect::parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};

use crate::theme::Palette;
use crate::theme_simple::{self as simple, Simple};

/// One named list of words and the Simple colour they get.
#[derive(Clone, Debug, PartialEq)]
pub struct WordList {
    pub name: String,
    pub colour: Simple,
    /// The words, separated by spaces.
    pub words: String,
}

/// What the dialog edits.
#[derive(Clone, Debug, PartialEq)]
pub struct LangForm {
    pub name: String,
    /// Extensions separated by spaces.
    pub extensions: String,
    /// Line comment markers separated by spaces.
    pub line_comment: String,
    pub block_open: String,
    pub block_close: String,
    pub double_quotes: bool,
    pub single_quotes: bool,
    /// Backslash escapes inside strings.
    pub escape: bool,
    pub multiline_strings: bool,
    pub numbers: bool,
    pub ignore_case: bool,
    pub lists: Vec<WordList>,
    /// Operators separated by spaces.
    pub operators: String,
    /// Folding pairs: "open close", separated by commas.
    pub folding: String,
}

impl Default for LangForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            extensions: String::new(),
            line_comment: "#".to_owned(),
            block_open: String::new(),
            block_close: String::new(),
            double_quotes: true,
            single_quotes: false,
            escape: true,
            multiline_strings: false,
            numbers: true,
            ignore_case: false,
            lists: vec![WordList {
                name: t!("lang-keywords-list"),
                colour: Simple::Keywords,
                words: String::new(),
            }],
            operators: String::new(),
            folding: String::new(),
        }
    }
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

/// Colours a keyword list can have (not Default text).
const LIST_COLOURS: [Simple; 9] = [
    Simple::Keywords,
    Simple::Functions,
    Simple::Types,
    Simple::Numbers,
    Simple::Strings,
    Simple::Comments,
    Simple::Tags,
    Simple::Attributes,
    Simple::Errors,
];

impl LangForm {
    fn folding_pairs(&self) -> Vec<[String; 2]> {
        self.folding
            .split(',')
            .filter_map(|pair| {
                let w = words(pair);
                (w.len() == 2).then(|| [w[0].clone(), w[1].clone()])
            })
            .collect()
    }

    /// The `.lang.json` text for the form.
    pub fn to_json(&self) -> String {
        let mut m = Map::new();
        m.insert(
            "_about".into(),
            json!("Made with Language > Define Language. Open it there again to change it."),
        );
        m.insert("name".into(), json!(self.name.trim()));
        m.insert(
            "extensions".into(),
            json!(
                words(&self.extensions)
                    .iter()
                    .map(|e| e.trim_start_matches('.').to_owned())
                    .collect::<Vec<_>>()
            ),
        );
        m.insert("case_sensitive".into(), json!(!self.ignore_case));
        m.insert("line_comment".into(), json!(words(&self.line_comment)));
        let (open, close) = (self.block_open.trim(), self.block_close.trim());
        if !open.is_empty() && !close.is_empty() {
            m.insert("block_comment".into(), json!([[open, close]]));
        } else {
            m.insert("block_comment".into(), json!([]));
        }
        let mut strings = Vec::new();
        for (on, q) in [(self.double_quotes, "\""), (self.single_quotes, "'")] {
            if on {
                if self.escape {
                    strings.push(json!([q, q, "\\"]));
                } else {
                    strings.push(json!([q, q]));
                }
            }
        }
        m.insert("strings".into(), Value::Array(strings));
        m.insert("multiline_strings".into(), json!(self.multiline_strings));
        m.insert("numbers".into(), json!(self.numbers));
        let lists: Vec<Value> = self
            .lists
            .iter()
            .filter(|l| !words(&l.words).is_empty())
            .map(|l| {
                json!({
                    "name": l.name.trim(),
                    "colour": l.colour.id(),
                    "words": words(&l.words),
                })
            })
            .collect();
        m.insert("keyword_lists".into(), Value::Array(lists));
        m.insert("operators".into(), json!(words(&self.operators)));
        m.insert("folding".into(), json!(self.folding_pairs()));
        // One setting per line, lists on one line, so the file is easy to edit by hand.
        let mut text = String::from("{\n");
        let last = m.len().saturating_sub(1);
        for (i, (key, value)) in m.iter().enumerate() {
            let value = match value {
                Value::Array(items) if key == "keyword_lists" && !items.is_empty() => {
                    let rows: Vec<String> = items
                        .iter()
                        .map(|v| format!("    {}", compact(v)))
                        .collect();
                    format!("[\n{}\n  ]", rows.join(",\n"))
                }
                v => compact(v),
            };
            let comma = if i == last { "" } else { "," };
            text.push_str(&format!("  {}: {value}{comma}\n", compact(&json!(key))));
        }
        text.push_str("}\n");
        text
    }

    /// Reads a `.lang.json` file into the form. When it uses something the
    /// form can't show, returns what (to offer opening the JSON instead).
    pub fn from_json(text: &str) -> Result<Self, Vec<String>> {
        let value: Value = serde_json::from_str(text).map_err(|e| vec![e.to_string()])?;
        let Some(obj) = value.as_object() else {
            return Err(vec![t!("lang-not-an-object")]);
        };
        let mut form = LangForm {
            lists: Vec::new(),
            line_comment: String::new(),
            double_quotes: false,
            escape: false,
            ..LangForm::default()
        };
        let mut problems = Vec::new();
        let one_or_many = |v: &Value| -> Option<Vec<String>> {
            match v {
                Value::String(s) => Some(vec![s.clone()]),
                Value::Array(a) => a.iter().map(|x| x.as_str().map(str::to_owned)).collect(),
                _ => None,
            }
        };
        let simple_word = |w: &str| !w.is_empty() && !w.chars().any(char::is_whitespace);
        let mut escapes = Vec::new();
        for (key, v) in obj {
            if key.starts_with('_') {
                continue;
            }
            match key.as_str() {
                "name" => form.name = v.as_str().unwrap_or_default().to_owned(),
                "extensions" | "line_comment" | "operators" => match one_or_many(v) {
                    Some(list) if list.iter().all(|w| simple_word(w)) => {
                        let joined = list.join(" ");
                        match key.as_str() {
                            "extensions" => form.extensions = joined,
                            "line_comment" => form.line_comment = joined,
                            _ => form.operators = joined,
                        }
                    }
                    _ => problems.push(t!("lang-unsupported-spaces", setting = key.clone())),
                },
                "case_sensitive" => form.ignore_case = !v.as_bool().unwrap_or(true),
                "multiline_strings" => form.multiline_strings = v.as_bool().unwrap_or(false),
                "numbers" => form.numbers = v.as_bool().unwrap_or(true),
                "block_comment" => {
                    let pairs = v.as_array().cloned().unwrap_or_default();
                    match pairs.as_slice() {
                        [] => {}
                        [pair] => {
                            let p = one_or_many(pair).unwrap_or_default();
                            if p.len() == 2 && p.iter().all(|w| simple_word(w)) {
                                form.block_open = p[0].clone();
                                form.block_close = p[1].clone();
                            } else {
                                problems.push(t!("lang-unsupported-block"));
                            }
                        }
                        _ => problems.push(t!("lang-unsupported-block")),
                    }
                }
                "strings" => {
                    for s in v.as_array().cloned().unwrap_or_default() {
                        let parts = one_or_many(&s).unwrap_or_default();
                        let (open, close, escape) = match parts.as_slice() {
                            [o, c] => (o.as_str(), c.as_str(), ""),
                            [o, c, e] => (o.as_str(), c.as_str(), e.as_str()),
                            _ => ("", "-", ""),
                        };
                        match (open, close, escape) {
                            ("\"", "\"", e) if e.is_empty() || e == "\\" => {
                                form.double_quotes = true;
                                escapes.push(!e.is_empty());
                            }
                            ("'", "'", e) if e.is_empty() || e == "\\" => {
                                form.single_quotes = true;
                                escapes.push(!e.is_empty());
                            }
                            _ => problems.push(t!("lang-unsupported-strings")),
                        }
                    }
                }
                "keywords" => {
                    for (group, list) in v.as_object().cloned().unwrap_or_default() {
                        // Only groups whose scope is the one a list colour writes.
                        let colour = match group.as_str() {
                            "keyword" | "keywords" | "control" => Some(Simple::Keywords),
                            _ => None,
                        };
                        let list = one_or_many(&list).unwrap_or_default();
                        match colour {
                            Some(colour) if list.iter().all(|w| simple_word(w)) => {
                                form.lists.push(WordList {
                                    name: group.clone(),
                                    colour,
                                    words: list.join(" "),
                                })
                            }
                            _ => problems.push(t!("lang-unsupported-group", group = group.clone())),
                        }
                    }
                }
                "keyword_lists" => {
                    for l in v.as_array().cloned().unwrap_or_default() {
                        let name = l["name"].as_str().unwrap_or_default().to_owned();
                        let colour = l["colour"]
                            .as_str()
                            .and_then(Simple::from_id)
                            .filter(|c| *c != Simple::Text);
                        let list = one_or_many(&l["words"]).unwrap_or_default();
                        match colour {
                            Some(colour) if list.iter().all(|w| simple_word(w)) => {
                                form.lists.push(WordList {
                                    name,
                                    colour,
                                    words: list.join(" "),
                                })
                            }
                            _ => problems.push(t!("lang-unsupported-group", group = name)),
                        }
                    }
                }
                "folding" => {
                    let mut pairs = Vec::new();
                    for pair in v.as_array().cloned().unwrap_or_default() {
                        let p = one_or_many(&pair).unwrap_or_default();
                        if p.len() == 2 && p.iter().all(|w| simple_word(w) && !w.contains(',')) {
                            pairs.push(format!("{} {}", p[0], p[1]));
                        } else {
                            problems.push(t!("lang-unsupported-folding"));
                        }
                    }
                    form.folding = pairs.join(", ");
                }
                "first_line" => problems.push(t!("lang-unsupported-first-line")),
                other => problems.push(t!("lang-unsupported-setting", setting = other.to_owned())),
            }
        }
        // One escape setting for all strings.
        if escapes.iter().any(|e| *e) && escapes.iter().any(|e| !e) {
            problems.push(t!("lang-unsupported-strings"));
        }
        form.escape = escapes.first().copied().unwrap_or(true);
        if problems.is_empty() {
            Ok(form)
        } else {
            problems.dedup();
            Err(problems)
        }
    }

    /// Sample text that shows off what the form defines.
    pub fn sample(&self) -> String {
        let mut out = String::new();
        if let Some(c) = words(&self.line_comment).first() {
            out.push_str(&format!("{c} {}\n", t!("lang-sample-comment")));
        }
        let mut n = 10;
        for list in &self.lists {
            let w = words(&list.words);
            if w.is_empty() {
                continue;
            }
            let mut line: Vec<String> = Vec::new();
            for word in w.iter().take(4) {
                line.push(format!("{word} {n}"));
                n += 10;
            }
            out.push_str(&line.join(" "));
            out.push('\n');
        }
        let quote = if self.double_quotes {
            Some('"')
        } else if self.single_quotes {
            Some('\'')
        } else {
            None
        };
        if let Some(q) = quote {
            let esc = if self.escape { "\\n" } else { "" };
            out.push_str(&format!("x = {q}{}{esc}{q}", t!("lang-sample-text")));
            if let Some(op) = words(&self.operators).first() {
                out.push_str(&format!(" {op} 1.5"));
            }
            out.push('\n');
        }
        let (open, close) = (self.block_open.trim(), self.block_close.trim());
        if !open.is_empty() && !close.is_empty() {
            out.push_str(&format!("{open} {} {close}\n", t!("lang-sample-block")));
        }
        out
    }
}

/// JSON on one line, with a space after each comma and colon.
fn compact(v: &Value) -> String {
    match v {
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(compact).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(map) => format!(
            "{{ {} }}",
            map.iter()
                .map(|(k, v)| format!("{}: {}", compact(&json!(k)), compact(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// What the dialog asks the app to do.
pub enum LangEvent {
    /// A language file was written: reload the languages.
    Saved { path: PathBuf, name: String },
    /// Open this `.lang.json` file in a tab.
    OpenJson(PathBuf),
    /// Make a new language from the JSON template (Language > New Language…).
    NewJson,
}

/// The preview's grammar for the current form.
struct Preview {
    json: String,
    result: Result<(Arc<SyntaxSet>, String), String>,
}

#[derive(Default)]
pub struct LangDialog {
    pub open: bool,
    form: LangForm,
    /// The file being edited, or None for a new language.
    path: Option<PathBuf>,
    sample: String,
    /// True once the user typed or pasted their own sample.
    own_sample: bool,
    preview: Option<Preview>,
    /// User languages in the languages folder: (file name, path).
    existing: Vec<(String, PathBuf)>,
    /// A file the dialog can't show, and why.
    unsupported: Option<(PathBuf, Vec<String>)>,
    error: Option<String>,
    raise: u8,
}

/// The `.lang.json` files in `dir`, sorted.
fn language_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter_map(|p| {
                    let name = p.file_name()?.to_str()?.to_owned();
                    name.to_lowercase()
                        .ends_with(".lang.json")
                        .then(|| (name[..name.len() - 10].to_owned(), p))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|(n, _)| n.to_lowercase());
    out
}

/// A file name for a new language called `name` in `dir`, not used yet.
pub fn new_file(dir: &Path, name: &str) -> PathBuf {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() {
            slug.extend(c.to_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "my-language" } else { slug };
    (1..)
        .map(|n| {
            if n == 1 {
                dir.join(format!("{slug}.lang.json"))
            } else {
                dir.join(format!("{slug}-{n}.lang.json"))
            }
        })
        .find(|p| !p.exists())
        .expect("an unused name")
}

impl LangDialog {
    /// Opens the dialog on a new language, or on `path` when given.
    pub fn show(&mut self, dir: Option<&Path>, path: Option<&Path>) {
        self.open = true;
        self.raise = 2;
        self.error = None;
        self.unsupported = None;
        self.existing = dir.map(language_files).unwrap_or_default();
        self.new_language();
        if let Some(path) = path {
            self.load(path);
        }
    }

    fn new_language(&mut self) {
        self.form = LangForm::default();
        self.path = None;
        self.own_sample = false;
        self.unsupported = None;
    }

    /// Loads an existing language into the form, if the form can show it.
    fn load(&mut self, path: &Path) {
        self.unsupported = None;
        match std::fs::read_to_string(path) {
            Ok(text) => match LangForm::from_json(&text) {
                Ok(form) => {
                    self.form = form;
                    self.path = Some(path.to_owned());
                    self.own_sample = false;
                }
                Err(why) => self.unsupported = Some((path.to_owned(), why)),
            },
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn update_preview(&mut self) {
        if !self.own_sample {
            self.sample = self.form.sample();
        }
        // The preview works before the language has a name.
        let mut form = self.form.clone();
        if form.name.trim().is_empty() {
            form.name = t!("lang-new");
        }
        let json = form.to_json();
        if self.preview.as_ref().is_some_and(|p| p.json == json) {
            return;
        }
        let result = crate::user_lang::to_sublime_syntax(&json)
            .and_then(|yaml| {
                SyntaxDefinition::load_from_str(&yaml, true, None).map_err(|e| e.to_string())
            })
            .map(|def| {
                let name = def.name.clone();
                let mut b = SyntaxSetBuilder::new();
                b.add(def);
                (Arc::new(b.build()), name)
            });
        self.preview = Some(Preview { json, result });
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        palette: &Palette,
        theme: &SyntectTheme,
        active_text: Option<&str>,
    ) -> Vec<LangEvent> {
        let mut events = Vec::new();
        if !self.open {
            return events;
        }
        self.update_preview();
        let mut open = true;
        let mut close = false;
        let shown = egui::Window::new(t!("define-language"))
            .id(Id::new("define-language"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([980.0, 560.0])
            .show(ctx, |ui| {
                self.top_row(ui, &mut events);
                if let Some((path, why)) = self.unsupported.clone() {
                    ui.label(
                        RichText::new(t!("lang-cant-show", reasons = why.join("; ")))
                            .color(palette.modified),
                    );
                    ui.horizontal(|ui| {
                        if ui.button(t!("open-json")).clicked() {
                            events.push(LangEvent::OpenJson(path));
                            close = true;
                        }
                        if ui.button(t!("cancel")).clicked() {
                            self.unsupported = None;
                        }
                    });
                }
                ui.separator();
                let body_h = (ui.available_height() - 40.0).max(160.0);
                egui::Panel::right("lang-preview")
                    .resizable(true)
                    .default_size(420.0)
                    .min_size(240.0)
                    .show(ui, |ui| {
                        ui.set_max_height(body_h);
                        self.preview_ui(ui, palette, theme, active_text);
                    });
                ScrollArea::vertical()
                    .id_salt("lang-form")
                    .auto_shrink([false, false])
                    .max_height(body_h)
                    .show(ui, |ui| self.form_ui(ui));
                ui.separator();
                close |= self.footer(ui, palette, &mut events);
            });
        if self.raise > 0
            && let Some(shown) = shown
        {
            ctx.move_to_top(shown.response.layer_id);
            self.raise -= 1;
        }
        if !open || close {
            self.open = false;
        }
        events
    }

    fn top_row(&mut self, ui: &mut Ui, events: &mut Vec<LangEvent>) {
        ui.horizontal(|ui| {
            ui.label(t!("lang-editing"));
            let current = match &self.path {
                Some(p) => p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                None => t!("lang-new"),
            };
            let mut pick = None;
            egui::ComboBox::from_id_salt("lang-existing")
                .selected_text(current)
                .width(220.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.path.is_none(), t!("lang-new"))
                        .clicked()
                    {
                        pick = Some(None);
                    }
                    for (name, path) in &self.existing {
                        if ui
                            .selectable_label(self.path.as_ref() == Some(path), name)
                            .clicked()
                        {
                            pick = Some(Some(path.clone()));
                        }
                    }
                });
            match pick {
                Some(None) => self.new_language(),
                Some(Some(path)) => self.load(&path),
                None => {}
            }
            if ui
                .link(t!("edit-json-instead"))
                .on_hover_text(t!("edit-json-instead-tip"))
                .clicked()
            {
                match &self.path {
                    Some(path) => events.push(LangEvent::OpenJson(path.clone())),
                    None => events.push(LangEvent::NewJson),
                }
                self.open = false;
            }
        });
    }

    fn form_ui(&mut self, ui: &mut Ui) {
        let f = &mut self.form;
        let field = |ui: &mut Ui, text: &mut String, hint: String, width: f32| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .hint_text(hint)
                    .desired_width(width),
            )
        };
        egui::Grid::new("lang-form-grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(t!("lang-name"));
                field(ui, &mut f.name, t!("lang-name-hint"), 260.0);
                ui.end_row();
                ui.label(t!("lang-extensions"));
                field(ui, &mut f.extensions, t!("lang-extensions-hint"), 260.0);
                ui.end_row();
                ui.label(t!("lang-line-comment"));
                ui.horizontal(|ui| {
                    field(ui, &mut f.line_comment, "#".into(), 60.0);
                    ui.label(t!("lang-block-comment"));
                    field(ui, &mut f.block_open, "/*".into(), 50.0);
                    field(ui, &mut f.block_close, "*/".into(), 50.0);
                });
                ui.end_row();
                ui.label(t!("lang-strings"));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut f.double_quotes, "\"…\"");
                    ui.checkbox(&mut f.single_quotes, "'…'");
                    ui.checkbox(&mut f.escape, t!("lang-escape"));
                    ui.checkbox(&mut f.multiline_strings, t!("lang-multiline"));
                });
                ui.end_row();
                ui.label(t!("lang-numbers"));
                ui.checkbox(&mut f.numbers, t!("lang-colour-numbers"));
                ui.end_row();
                ui.label(t!("lang-case"));
                ui.checkbox(&mut f.ignore_case, t!("lang-ignore-case"));
                ui.end_row();
                ui.label(t!("lang-operators"));
                field(ui, &mut f.operators, "+ - * / =".into(), 260.0);
                ui.end_row();
                ui.label(t!("lang-folding"));
                field(ui, &mut f.folding, t!("lang-folding-hint"), 260.0)
                    .on_hover_text(t!("lang-folding-tip"));
                ui.end_row();
            });
        ui.add_space(6.0);
        ui.label(RichText::new(t!("lang-keyword-lists")).weak());
        let mut remove = None;
        for (i, list) in f.lists.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [100.0, 20.0],
                    egui::TextEdit::singleline(&mut list.name)
                        .id_salt(("lang-list-name", i))
                        .hint_text(t!("lang-list-name")),
                );
                ui.add_sized(
                    [240.0, 20.0],
                    egui::TextEdit::singleline(&mut list.words)
                        .id_salt(("lang-list-words", i))
                        .hint_text(t!("lang-list-words"))
                        .font(egui::TextStyle::Monospace),
                );
                egui::ComboBox::from_id_salt(("lang-list-colour", i))
                    .selected_text(list.colour.label())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for c in LIST_COLOURS {
                            ui.selectable_value(&mut list.colour, c, c.label());
                        }
                    });
                if crate::a11y::name(ui.small_button("🗑"), &t!("lang-remove-list")).clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            f.lists.remove(i);
        }
        if ui.button(t!("lang-add-list")).clicked() {
            f.lists.push(WordList {
                name: String::new(),
                colour: Simple::Functions,
                words: String::new(),
            });
        }
    }

    fn preview_ui(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        theme: &SyntectTheme,
        active_text: Option<&str>,
    ) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("lang-preview")).weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.own_sample && ui.small_button(t!("lang-sample-reset")).clicked() {
                    self.own_sample = false;
                    self.sample = self.form.sample();
                }
                if let Some(text) = active_text
                    && ui
                        .small_button(t!("lang-sample-from-tab"))
                        .on_hover_text(t!("lang-sample-from-tab-tip"))
                        .clicked()
                {
                    self.sample = text.lines().take(60).collect::<Vec<_>>().join("\n");
                    self.own_sample = true;
                }
            });
        });
        let Some(preview) = &self.preview else {
            return;
        };
        let bg = theme
            .settings
            .background
            .map(|c| egui::Color32::from_rgb(c.r, c.g, c.b))
            .unwrap_or(palette.editor_bg);
        let fg = theme
            .settings
            .foreground
            .map(|c| egui::Color32::from_rgb(c.r, c.g, c.b))
            .unwrap_or(palette.editor_fg);
        match &preview.result {
            Ok((set, name)) => {
                let set = set.clone();
                let name = name.clone();
                let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
                    let text = buf.as_str();
                    let syntax = set
                        .find_syntax_by_name(&name)
                        .unwrap_or_else(|| set.find_syntax_plain_text());
                    let tokens = simple::highlight(text, syntax, &set, theme);
                    let job =
                        simple::layout(text, &tokens, egui::FontId::monospace(13.0), fg, None);
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                egui::Frame::new()
                    .fill(bg)
                    .corner_radius(4.0)
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("lang-sample")
                            .max_height(ui.available_height() - 40.0)
                            .show(ui, |ui| {
                                let r = ui.add(
                                    egui::TextEdit::multiline(&mut self.sample)
                                        .frame(egui::Frame::NONE)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(12)
                                        .layouter(&mut layouter),
                                );
                                if r.changed() {
                                    self.own_sample = true;
                                }
                            });
                    });
                ui.label(RichText::new(t!("lang-sample-hint")).small().weak());
            }
            Err(e) => {
                ui.label(
                    RichText::new(t!("lang-preview-error", error = e.clone()))
                        .color(palette.brace_bad.to_opaque()),
                );
            }
        }
    }

    /// Where it saves, Cancel and Save Language. True when the dialog should close.
    fn footer(&mut self, ui: &mut Ui, palette: &Palette, events: &mut Vec<LangEvent>) -> bool {
        let mut close = false;
        let dir = crate::highlight::languages_dir();
        let target = self
            .path
            .clone()
            .or_else(|| dir.as_deref().map(|d| new_file(d, &self.form.name)));
        ui.horizontal(|ui| {
            if let Some(t) = &target {
                ui.label(
                    RichText::new(t!("lang-saves-to", path = t.display().to_string()))
                        .small()
                        .weak(),
                );
            }
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(palette.brace_bad.to_opaque()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let valid = !self.form.name.trim().is_empty()
                    && self.preview.as_ref().is_some_and(|p| p.result.is_ok());
                let save = ui
                    .add_enabled(valid, egui::Button::new(t!("save-language")))
                    .on_disabled_hover_text(t!("save-language-disabled"));
                if save.clicked()
                    && let Some(path) = target.clone()
                {
                    let written = path
                        .parent()
                        .map(std::fs::create_dir_all)
                        .unwrap_or(Ok(()))
                        .and_then(|_| crate::watch::write(&path, self.form.to_json()));
                    match written {
                        Ok(()) => {
                            events.push(LangEvent::Saved {
                                path,
                                name: self.form.name.trim().to_owned(),
                            });
                            close = true;
                        }
                        Err(e) => {
                            self.error = Some(t!(
                                "write-failed",
                                path = path.display().to_string(),
                                error = e.to_string()
                            ))
                        }
                    }
                }
                if ui.button(t!("cancel")).clicked() {
                    close = true;
                }
            });
        });
        close
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::Highlighter;
    use crate::highlight::tests::assert_scope;

    fn gcode() -> LangForm {
        LangForm {
            name: "Gcode".into(),
            extensions: "gcode nc".into(),
            line_comment: ";".into(),
            block_open: "(".into(),
            block_close: ")".into(),
            double_quotes: true,
            single_quotes: false,
            escape: true,
            multiline_strings: false,
            numbers: true,
            ignore_case: true,
            lists: vec![
                WordList {
                    name: "Motion".into(),
                    colour: Simple::Keywords,
                    words: "G0 G1 G2 G3".into(),
                },
                WordList {
                    name: "Machine".into(),
                    colour: Simple::Functions,
                    words: "M3 M5 M30".into(),
                },
            ],
            operators: String::new(),
            folding: String::new(),
        }
    }

    /// The Gcode example from the mockup: built in the dialog, saved, it
    /// highlights a .gcode file, and opening it again gives the same form.
    #[test]
    fn gcode_round_trip() {
        let form = gcode();
        let json = form.to_json();
        assert_eq!(LangForm::from_json(&json).unwrap(), form);

        let dir = std::env::temp_dir().join(format!("ettin-langdlg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = new_file(&dir, &form.name);
        assert_eq!(path.file_name().unwrap(), "gcode.lang.json");
        std::fs::write(&path, &json).unwrap();
        let (h, errors) = Highlighter::new(Some(&dir));
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            h.detect(Some(Path::new("part.gcode")), ""),
            "Gcode",
            "the extension picks the language"
        );
        let src = "; warm up\nG0 X10 Y20\nm3 S1200 (spindle on)\nG1 Z-1.5 F300\nM30\n";
        assert_scope(&h, "Gcode", src, "G0", "keyword.control");
        assert_scope(&h, "Gcode", src, "m3", "entity.name.function");
        assert_scope(&h, "Gcode", src, "; warm up", "comment.line");
        assert_scope(&h, "Gcode", src, "spindle on", "comment.block");
        assert_scope(&h, "Gcode", src, "1.5", "constant.numeric");

        // Reopening the saved file in the dialog loads the same values.
        let mut dlg = LangDialog::default();
        dlg.show(Some(&dir), Some(&path));
        assert!(dlg.unsupported.is_none());
        assert_eq!(dlg.form, form);
        assert_eq!(dlg.existing.len(), 1);
        assert_eq!(
            new_file(&dir, "Gcode").file_name().unwrap(),
            "gcode-2.lang.json"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_option_round_trips() {
        let mut form = gcode();
        form.single_quotes = true;
        form.escape = false;
        form.multiline_strings = true;
        form.ignore_case = false;
        form.numbers = false;
        form.operators = "+ - == != , :".into();
        form.folding = "{ }, if end".into();
        form.block_open = String::new();
        form.block_close = String::new();
        form.lists.push(WordList {
            name: "Kinds".into(),
            colour: Simple::Types,
            words: "int real".into(),
        });
        let json = form.to_json();
        assert_eq!(LangForm::from_json(&json).unwrap(), form);
        crate::user_lang::to_sublime_syntax(&json).unwrap();
    }

    #[test]
    fn json_only_features_are_reported() {
        // The JSON template has type and constant groups and a first-line pattern.
        let why = LangForm::from_json(crate::user_lang::UDL_TEMPLATE).unwrap_err();
        assert!(why.iter().any(|w| w.contains("type")), "{why:?}");
        assert!(LangForm::from_json("{").is_err());
        // Older files with a plain keyword group load.
        let form = LangForm::from_json(
            r#"{"name": "Old", "extensions": "old", "line_comment": "//",
                "keywords": {"keyword": ["if", "end"]}}"#,
        )
        .unwrap();
        assert_eq!(form.lists.len(), 1);
        assert_eq!(form.lists[0].colour, Simple::Keywords);
        assert_eq!(form.line_comment, "//");
    }

    #[test]
    fn sample_uses_the_form() {
        let s = gcode().sample();
        assert!(s.starts_with("; "));
        assert!(s.contains("G0 10"));
        assert!(s.contains("( "));
    }
}
