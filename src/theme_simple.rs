//! The Simple view of Theme Colours: about ten named colours (Comments,
//! Strings, Keywords…), each standing for a list of scopes, so a theme can be
//! changed without knowing TextMate scopes. Also finds what colours a spot of
//! text ("What colours this?") and highlights small samples for previews.

use std::ops::Range;
use std::str::FromStr;

use egui::Color32;
use egui::text::{LayoutJob, TextFormat};
use plist::{Dictionary, Value};
use syntect::highlighting::{FontStyle, Highlighter, ScopeSelectors, Style, Theme};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};

use crate::theme::{parse_hex, to_hex};

/// A named colour of the Simple view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Simple {
    Text,
    Comments,
    Strings,
    Numbers,
    Keywords,
    Functions,
    Types,
    Tags,
    Attributes,
    Errors,
}

pub const ALL: [Simple; 10] = [
    Simple::Text,
    Simple::Comments,
    Simple::Strings,
    Simple::Numbers,
    Simple::Keywords,
    Simple::Functions,
    Simple::Types,
    Simple::Tags,
    Simple::Attributes,
    Simple::Errors,
];

impl Simple {
    /// The id used in files (`.lang.json` keyword lists).
    pub fn id(self) -> &'static str {
        match self {
            Simple::Text => "text",
            Simple::Comments => "comments",
            Simple::Strings => "strings",
            Simple::Numbers => "numbers",
            Simple::Keywords => "keywords",
            Simple::Functions => "functions",
            Simple::Types => "types",
            Simple::Tags => "tags",
            Simple::Attributes => "attributes",
            Simple::Errors => "errors",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        ALL.into_iter()
            .find(|s| s.id().eq_ignore_ascii_case(id.trim()))
    }

    /// The message id of its name.
    pub fn label_key(self) -> &'static str {
        match self {
            Simple::Text => "simple-text",
            Simple::Comments => "simple-comments",
            Simple::Strings => "simple-strings",
            Simple::Numbers => "simple-numbers",
            Simple::Keywords => "simple-keywords",
            Simple::Functions => "simple-functions",
            Simple::Types => "simple-types",
            Simple::Tags => "simple-tags",
            Simple::Attributes => "simple-attributes",
            Simple::Errors => "simple-errors",
        }
    }

    pub fn label(self) -> String {
        crate::i18n::tr(self.label_key())
    }

    /// The English name, used for the rule the Simple view adds to a theme.
    fn rule_name(self) -> &'static str {
        match self {
            Simple::Text => "Default text",
            Simple::Comments => "Comments",
            Simple::Strings => "Strings",
            Simple::Numbers => "Numbers",
            Simple::Keywords => "Keywords",
            Simple::Functions => "Functions",
            Simple::Types => "Types",
            Simple::Tags => "Tags",
            Simple::Attributes => "Attributes",
            Simple::Errors => "Errors",
        }
    }

    /// The scopes it colours. A scope that starts with one of these (such as
    /// `keyword.control.rust`) belongs to it too. Default text has none: it
    /// is the theme's text colour.
    pub fn scopes(self) -> &'static [&'static str] {
        match self {
            Simple::Text => &[],
            Simple::Comments => &["comment", "punctuation.definition.comment"],
            Simple::Strings => &["string"],
            Simple::Numbers => &["constant.numeric"],
            Simple::Keywords => &[
                "keyword.control",
                "keyword.other",
                "keyword.declaration",
                "storage.type",
                "storage.modifier",
                "constant.language",
            ],
            Simple::Functions => &[
                "entity.name.function",
                "support.function",
                "variable.function",
            ],
            Simple::Types => &[
                "entity.name.type",
                "entity.name.class",
                "entity.name.struct",
                "entity.name.enum",
                "entity.name.trait",
                "entity.name.impl",
                "entity.other.inherited-class",
                "support.type",
                "support.class",
            ],
            Simple::Tags => &["entity.name.tag"],
            Simple::Attributes => &["entity.other.attribute-name"],
            Simple::Errors => &["invalid"],
        }
    }

    /// A typical scope of this kind of text, to show its colour.
    fn sample_scope(self) -> Option<&'static str> {
        Some(match self {
            Simple::Text => return None,
            Simple::Comments => "comment.line",
            Simple::Strings => "string.quoted.double",
            Simple::Numbers => "constant.numeric",
            Simple::Keywords => "keyword.control",
            Simple::Functions => "entity.name.function",
            Simple::Types => "entity.name.class",
            Simple::Tags => "entity.name.tag",
            Simple::Attributes => "entity.other.attribute-name",
            Simple::Errors => "invalid.illegal",
        })
    }

    /// The one scope a `.lang.json` keyword list gets when it uses this
    /// colour: the one the Simple view shows, so the list looks like its swatch.
    pub fn lang_scope(self) -> &'static str {
        self.sample_scope().unwrap_or("variable.language")
    }

    fn scope_list(self) -> Vec<Scope> {
        self.scopes()
            .iter()
            .filter_map(|s| Scope::new(s).ok())
            .collect()
    }

    /// True when `scope` is one of this colour's scopes or inside one.
    pub fn covers(self, scope: Scope) -> bool {
        self.scope_list().iter().any(|c| c.is_prefix_of(scope))
    }
}

/// The named colour a scope stack belongs to: the innermost scope that one
/// of them covers, or Default text.
pub fn simple_for(stack: &[Scope]) -> Simple {
    for scope in stack.iter().rev() {
        if let Some(s) = ALL.into_iter().find(|s| s.covers(*scope)) {
            return s;
        }
    }
    Simple::Text
}

/// The window colours a theme can set, beyond the editor's. Other tmTheme
/// readers ignore these keys.
pub const WINDOW: &[(&str, &str)] = &[
    ("ettinTabStrip", "style-tab-strip"),
    ("ettinPanels", "style-panels"),
    ("ettinStatusBar", "style-status-bar"),
    ("ettinRibbon", "style-ribbon"),
    ("selection", "style-selection"),
];

/// How a named colour looks in a theme now.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Look {
    pub foreground: Color32,
    /// Set when the colour has its own background (Errors usually do).
    pub background: Option<Color32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

fn rgba(c: syntect::highlighting::Color) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

/// How `simple` looks in `theme`.
pub fn look(theme: &Theme, simple: Simple) -> Look {
    let h = Highlighter::new(theme);
    let default = h.get_default();
    let style = match simple.sample_scope().and_then(|s| Scope::new(s).ok()) {
        Some(scope) => h.style_for_stack(&[scope]),
        None => default,
    };
    Look {
        foreground: rgba(style.foreground),
        background: (style.background != default.background).then(|| rgba(style.background)),
        bold: style.font_style.contains(FontStyle::BOLD),
        italic: style.font_style.contains(FontStyle::ITALIC),
        underline: style.font_style.contains(FontStyle::UNDERLINE),
    }
}

fn rules(doc: &mut Value) -> Option<&mut Vec<Value>> {
    doc.as_dictionary_mut()?.get_mut("settings")?.as_array_mut()
}

fn scope_text(rule: &Value) -> Option<&str> {
    rule.as_dictionary()?.get("scope")?.as_string()
}

/// True when every selector of a rule's scope is a plain scope inside
/// `simple` (so the rule colours only that kind of text).
fn rule_is_only(scope: &str, simple: Simple) -> bool {
    let Ok(selectors) = ScopeSelectors::from_str(scope) else {
        return false;
    };
    !selectors.selectors.is_empty()
        && selectors.selectors.iter().all(|sel| {
            sel.extract_single_scope()
                .is_some_and(|scope| simple.covers(scope))
        })
}

fn settings_of(rule: &mut Value) -> Option<&mut Dictionary> {
    let dict = rule.as_dictionary_mut()?;
    if !matches!(dict.get("settings"), Some(Value::Dictionary(_))) {
        dict.insert("settings".to_owned(), Value::Dictionary(Dictionary::new()));
    }
    dict.get_mut("settings")?.as_dictionary_mut()
}

fn write_look(settings: &mut Dictionary, look: &Look) {
    settings.insert(
        "foreground".to_owned(),
        Value::String(to_hex(look.foreground)),
    );
    match look.background {
        Some(bg) => {
            settings.insert("background".to_owned(), Value::String(to_hex(bg)));
        }
        None => {
            settings.remove("background");
        }
    }
    let mut words = Vec::new();
    if look.bold {
        words.push("bold");
    }
    if look.italic {
        words.push("italic");
    }
    if look.underline {
        words.push("underline");
    }
    if words.is_empty() {
        settings.remove("fontStyle");
    } else {
        settings.insert("fontStyle".to_owned(), Value::String(words.join(" ")));
    }
}

/// Changes a named colour in a theme document (a parsed `.tmTheme`): every
/// rule that colours only that kind of text gets the new look, and a rule
/// for all of its scopes (added at the top if missing) makes sure the rest
/// follows. Rules for special places, such as JSON keys, keep their colours.
pub fn set_look(doc: &mut Value, simple: Simple, look: &Look) {
    let Some(list) = rules(doc) else {
        return;
    };
    if simple == Simple::Text {
        let global = match list
            .iter()
            .position(|v| v.as_dictionary().is_some_and(|d| !d.contains_key("scope")))
        {
            Some(i) => i,
            None => {
                list.insert(0, Value::Dictionary(Dictionary::new()));
                0
            }
        };
        if let Some(settings) = settings_of(&mut list[global]) {
            settings.insert(
                "foreground".to_owned(),
                Value::String(to_hex(look.foreground)),
            );
        }
        return;
    }
    let ours = simple.scopes().join(", ");
    let mut found = false;
    for rule in list.iter_mut() {
        let Some(scope) = scope_text(rule).map(str::to_owned) else {
            continue;
        };
        let mine = scope == ours;
        if mine || rule_is_only(&scope, simple) {
            found |= mine;
            if let Some(settings) = settings_of(rule) {
                write_look(settings, look);
            }
        }
    }
    if !found {
        let mut settings = Dictionary::new();
        write_look(&mut settings, look);
        let mut rule = Dictionary::new();
        rule.insert(
            "name".to_owned(),
            Value::String(simple.rule_name().to_owned()),
        );
        rule.insert("scope".to_owned(), Value::String(ours));
        rule.insert("settings".to_owned(), Value::Dictionary(settings));
        // First among the rules: on a tie syntect keeps the earlier rule.
        let at = list
            .iter()
            .position(|v| v.as_dictionary().is_some_and(|d| d.contains_key("scope")))
            .unwrap_or(list.len());
        list.insert(at, Value::Dictionary(rule));
    }
}

/// A window colour (see [`WINDOW`]) from a theme document's editor colours.
pub fn window_colour(doc: &Value, key: &str) -> Option<Color32> {
    doc.as_dictionary()?
        .get("settings")?
        .as_array()?
        .iter()
        .filter_map(Value::as_dictionary)
        .find(|d| !d.contains_key("scope"))?
        .get("settings")?
        .as_dictionary()?
        .get(key)?
        .as_string()
        .and_then(parse_hex)
}

/// Which rule colours text with this scope stack: the index into the
/// theme's `settings` array of the best-matching rule that sets a text
/// colour, or `None` when the theme's default text colour applies.
pub fn winning_rule(doc: &Value, stack: &[Scope]) -> Option<usize> {
    let list = doc.as_dictionary()?.get("settings")?.as_array()?;
    let mut best: Option<(syntect::parsing::MatchPower, usize)> = None;
    for (i, rule) in list.iter().enumerate() {
        let Some(dict) = rule.as_dictionary() else {
            continue;
        };
        let has_fg = dict
            .get("settings")
            .and_then(Value::as_dictionary)
            .is_some_and(|s| s.contains_key("foreground"));
        let Some(scope) = dict.get("scope").and_then(Value::as_string) else {
            continue;
        };
        if !has_fg {
            continue;
        }
        let Ok(selectors) = ScopeSelectors::from_str(scope) else {
            continue;
        };
        if let Some(power) = selectors.does_match(stack)
            && best.is_none_or(|(b, _)| power > b)
        {
            best = Some((power, i));
        }
    }
    best.map(|(_, i)| i)
}

/// A rule's name, or its scope when it has none.
pub fn rule_label(doc: &Value, index: usize) -> String {
    let Some(dict) = doc
        .as_dictionary()
        .and_then(|d| d.get("settings"))
        .and_then(Value::as_array)
        .and_then(|l| l.get(index))
        .and_then(Value::as_dictionary)
    else {
        return String::new();
    };
    dict.get("name")
        .and_then(Value::as_string)
        .filter(|n| !n.is_empty())
        .or_else(|| dict.get("scope").and_then(Value::as_string))
        .unwrap_or_default()
        .to_owned()
}

/// True when the rule at `index` belongs to the Simple view's `simple`
/// (changing that colour changes this rule).
pub fn rule_follows(doc: &Value, index: usize, simple: Simple) -> bool {
    let Some(scope) = doc
        .as_dictionary()
        .and_then(|d| d.get("settings"))
        .and_then(Value::as_array)
        .and_then(|l| l.get(index))
        .and_then(scope_text)
    else {
        return false;
    };
    scope == simple.scopes().join(", ") || rule_is_only(scope, simple)
}

/// True when changing `simple` would recolour text with this scope stack
/// (its rule follows the colour now, or the Simple view's rule would win).
pub fn follows_after_change(doc: &Value, stack: &[Scope], simple: Simple) -> bool {
    if simple == Simple::Text {
        return true;
    }
    let mut copy = doc.clone();
    let look = Look {
        foreground: Color32::from_rgb(1, 2, 3),
        background: None,
        bold: false,
        italic: false,
        underline: false,
    };
    set_look(&mut copy, simple, &look);
    winning_rule(&copy, stack).is_some_and(|i| rule_follows(&copy, i, simple))
}

/// A scope stack written out, outermost first.
pub fn stack_text(stack: &[Scope]) -> String {
    stack
        .iter()
        .map(|s| s.build_string())
        .collect::<Vec<_>>()
        .join(" › ")
}

/// One highlighted piece of a sample.
pub struct Token {
    /// Byte range in the sample.
    pub range: Range<usize>,
    pub stack: Vec<Scope>,
    pub style: Style,
}

/// Highlights `text` as `syntax`, keeping each token's scopes.
pub fn highlight(
    text: &str,
    syntax: &SyntaxReference,
    syntaxes: &SyntaxSet,
    theme: &Theme,
) -> Vec<Token> {
    let h = Highlighter::new(theme);
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut out = Vec::new();
    let mut base = 0;
    for line in text.split_inclusive('\n') {
        let ops = state.parse_line(line, syntaxes).unwrap_or_default();
        let mut pos = 0;
        let mut push = |from: usize, to: usize, stack: &ScopeStack| {
            if to > from {
                out.push(Token {
                    range: base + from..base + to,
                    stack: stack.as_slice().to_vec(),
                    style: h.style_for_stack(stack.as_slice()),
                });
            }
        };
        for (i, op) in ops {
            push(pos, i, &stack);
            pos = pos.max(i);
            let _ = stack.apply(&op);
        }
        push(pos, line.len(), &stack);
        base += line.len();
    }
    out
}

/// The scopes at byte `at` of a highlighted sample.
#[cfg(test)]
pub fn stack_at(tokens: &[Token], at: usize) -> Option<&[Scope]> {
    tokens
        .iter()
        .find(|t| t.range.contains(&at))
        .or_else(|| tokens.iter().rev().find(|t| t.range.end == at))
        .map(|t| t.stack.as_slice())
}

/// The scopes at line `line`, char `col` of a document: parses from a little
/// above it (big files) or the start.
pub fn stack_in_document(
    buffer: &crate::editor::buffer::Buffer,
    line: usize,
    col: usize,
    syntax: &SyntaxReference,
    syntaxes: &SyntaxSet,
) -> Vec<Scope> {
    const BEFORE: usize = 2000;
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let from = line.saturating_sub(BEFORE);
    let mut text = String::new();
    for l in from..=line.min(buffer.len_lines().saturating_sub(1)) {
        text.clear();
        text.push_str(&buffer.line_text(l));
        text.push('\n');
        let ops = state.parse_line(&text, syntaxes).unwrap_or_default();
        if l == line {
            let byte = text
                .char_indices()
                .nth(col)
                .map(|(b, _)| b)
                .unwrap_or(text.len() - 1);
            for (i, op) in ops {
                if i > byte {
                    break;
                }
                let _ = stack.apply(&op);
            }
            return stack.as_slice().to_vec();
        }
        for (_, op) in ops {
            let _ = stack.apply(&op);
        }
    }
    stack.as_slice().to_vec()
}

/// A layout of highlighted sample text, with `marked` (a byte range) boxed.
pub fn layout(
    text: &str,
    tokens: &[Token],
    font: egui::FontId,
    fallback: Color32,
    marked: Option<Range<usize>>,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut covered = 0;
    let format = |color: Color32, style: Option<&Style>, range: &Range<usize>| {
        let mut f = TextFormat::simple(font.clone(), color);
        if let Some(s) = style {
            f.italics = s.font_style.contains(FontStyle::ITALIC);
            if s.font_style.contains(FontStyle::UNDERLINE) {
                f.underline = egui::Stroke::new(1.0, color);
            }
        }
        if marked
            .as_ref()
            .is_some_and(|m| m.start <= range.start && range.end <= m.end)
        {
            f.background = Color32::from_rgba_unmultiplied(255, 196, 0, 70);
        }
        f
    };
    for t in tokens {
        if t.range.start > covered {
            let r = covered..t.range.start;
            job.append(&text[r.clone()], 0.0, format(fallback, None, &r));
        }
        let color = rgba(t.style.foreground);
        job.append(
            &text[t.range.clone()],
            0.0,
            format(color, Some(&t.style), &t.range),
        );
        covered = t.range.end;
    }
    if covered < text.len() {
        let r = covered..text.len();
        job.append(&text[r.clone()], 0.0, format(fallback, None, &r));
    }
    job
}

/// The preview samples of Theme Colours: (language, text).
pub const SAMPLES: &[(&str, &str)] = &[
    (
        "JavaScript",
        "// compute a total\nfunction total(items) {\n  const tax = 0.2;\n  return items.length * 2 + \"ok\" + true;\n}\nclass Cart extends Base {}\n",
    ),
    (
        "Python",
        "# compute a total\ndef total(items):\n    \"\"\"Add them up.\"\"\"\n    if not items:\n        return None\n    return len(items) * 2\n\nclass Cart(Base):\n    pass\n",
    ),
    (
        "Rust",
        "/// Compute a total.\npub fn total(items: &[u32]) -> u32 {\n    let tax = 0.2;\n    if items.is_empty() { return 0; }\n    items.len() as u32 * 2 // \"ok\"\n}\n",
    ),
    (
        "HTML",
        "<!-- a page -->\n<div class=\"cart\" id=\"main\">\n  <p>Total: <b>42</b></p>\n</div>\n",
    ),
    (
        "JSON",
        "{\n  \"name\": \"cart\",\n  \"items\": [1, 2, 3],\n  \"paid\": true,\n  \"note\": null\n}\n",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn doc(source: &str) -> Value {
        Value::from_reader_xml(Cursor::new(source.as_bytes())).unwrap()
    }

    fn source(doc: &Value) -> String {
        let mut out = Vec::new();
        doc.to_writer_xml(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn theme_of(doc: &Value) -> Theme {
        crate::theme::from_source("T", &source(doc), true)
            .unwrap()
            .syntax
    }

    /// The colour of the first token equal to `word` in `text`.
    fn colour_of(theme: &Theme, language: &str, text: &str, word: &str) -> Color32 {
        let h = crate::highlight::tests::shared();
        let syntax = h.syntaxes.find_syntax_by_name(language).unwrap();
        let tokens = highlight(text, syntax, &h.syntaxes, theme);
        let t = tokens
            .iter()
            .find(|t| &text[t.range.clone()] == word)
            .unwrap_or_else(|| panic!("no token {word} in {language}"));
        rgba(t.style.foreground)
    }

    const PINK: Color32 = Color32::from_rgb(0xFF, 0x00, 0xAA);

    /// Changing Keywords recolours keywords in Rust, Python and JSON, in every built-in theme.
    #[test]
    fn keywords_recolour_every_language() {
        for (name, src) in crate::theme::BUILTIN {
            let mut d = doc(src);
            let before = theme_of(&d);
            let mut l = look(&before, Simple::Keywords);
            l.foreground = PINK;
            set_look(&mut d, Simple::Keywords, &l);
            let theme = theme_of(&d);
            for (lang, text, word) in [
                ("Rust", "pub fn main() { if x { return } }\n", "fn"),
                ("Rust", "pub fn main() { if x { return } }\n", "if"),
                ("Python", "def f():\n    return None\n", "def"),
                ("Python", "def f():\n    return None\n", "return"),
                ("JSON", "{\"a\": true}\n", "true"),
                ("JavaScript", SAMPLES[0].1, "function"),
            ] {
                assert_eq!(
                    colour_of(&theme, lang, text, word),
                    PINK,
                    "{name}: {lang} {word}"
                );
            }
            assert_eq!(look(&theme, Simple::Keywords).foreground, PINK);
            // Strings and JSON keys keep their colours.
            assert_eq!(
                colour_of(&theme, "JSON", "{\"a\": \"b\"}\n", "a"),
                colour_of(&before, "JSON", "{\"a\": \"b\"}\n", "a"),
                "{name}"
            );
            assert_eq!(
                look(&theme, Simple::Strings),
                look(&before, Simple::Strings),
                "{name}"
            );
        }
    }

    #[test]
    fn every_simple_colour_can_be_changed() {
        let (_, src) = crate::theme::BUILTIN[0];
        for simple in ALL {
            let mut d = doc(src);
            let mut l = look(&theme_of(&d), simple);
            l.foreground = PINK;
            l.bold = true;
            set_look(&mut d, simple, &l);
            let after = look(&theme_of(&d), simple);
            assert_eq!(after.foreground, PINK, "{simple:?}");
            if simple != Simple::Text {
                assert!(after.bold, "{simple:?}");
            }
            // Changing it again doesn't add another rule.
            let count = rules(&mut d).unwrap().len();
            set_look(&mut d, simple, &l);
            assert_eq!(rules(&mut d).unwrap().len(), count);
        }
    }

    #[test]
    fn what_colours_this() {
        let h = crate::highlight::tests::shared();
        let (_, src) = crate::theme::BUILTIN[0];
        let d = doc(src);
        let theme = theme_of(&d);
        let (lang, text) = SAMPLES[0];
        let syntax = h.syntaxes.find_syntax_by_name(lang).unwrap();
        let tokens = highlight(text, syntax, &h.syntaxes, &theme);
        let at = |word: &str| stack_at(&tokens, text.find(word).unwrap() + 1).unwrap();
        assert_eq!(simple_for(at("\"ok\"")), Simple::Strings);
        assert_eq!(simple_for(at("function")), Simple::Keywords);
        assert_eq!(simple_for(at("total(")), Simple::Functions);
        assert_eq!(simple_for(at("// compute")), Simple::Comments);
        assert_eq!(simple_for(at("0.2")), Simple::Numbers);
        assert_eq!(simple_for(at("Cart")), Simple::Types);
        // Ettin Dark colours strings with a rule shared with symbols; once
        // Strings is changed, the Simple view's own rule wins.
        let rule = winning_rule(&d, at("\"ok\"")).unwrap();
        assert!(!rule_follows(&d, rule, Simple::Strings));
        assert!(follows_after_change(&d, at("\"ok\""), Simple::Strings));
        let mut d = d;
        set_look(&mut d, Simple::Strings, &look(&theme, Simple::Strings));
        let rule = winning_rule(&d, at("\"ok\"")).unwrap();
        assert!(rule_follows(&d, rule, Simple::Strings));
        assert_eq!(rule_label(&d, rule), "Strings");

        // In a document, from the editor.
        let buffer = crate::editor::buffer::Buffer::new("let a = \"x\";\nlet b = 'y';\n");
        let js = h.syntaxes.find_syntax_by_name("JavaScript").unwrap();
        let stack = stack_in_document(&buffer, 1, 9, js, &h.syntaxes);
        assert_eq!(
            simple_for(&stack),
            Simple::Strings,
            "{}",
            stack_text(&stack)
        );
        let stack = stack_in_document(&buffer, 0, 1, js, &h.syntaxes);
        assert_eq!(simple_for(&stack), Simple::Keywords);
    }

    #[test]
    fn built_in_rules_have_names() {
        for (name, src) in crate::theme::BUILTIN {
            let d = doc(src);
            let list = d
                .as_dictionary()
                .unwrap()
                .get("settings")
                .unwrap()
                .as_array()
                .unwrap();
            for rule in list.iter().filter_map(Value::as_dictionary) {
                if rule.contains_key("scope") {
                    let n = rule.get("name").and_then(Value::as_string).unwrap_or("");
                    assert!(!n.is_empty(), "{name}: {:?}", rule.get("scope"));
                }
            }
        }
    }

    #[test]
    fn window_colours_are_read() {
        let src = crate::theme::BUILTIN[0].1.replace(
            "<key>accent</key>",
            "<key>ettinStatusBar</key><string>#112233</string><key>accent</key>",
        );
        let d = doc(&src);
        assert_eq!(
            window_colour(&d, "ettinStatusBar"),
            Some(Color32::from_rgb(0x11, 0x22, 0x33))
        );
        assert_eq!(window_colour(&d, "ettinRibbon"), None);
    }
}
