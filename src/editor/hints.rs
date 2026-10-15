//! Parameter hints: while the caret is inside a function call's parentheses,
//! the function's signatures are shown with the parameter being typed
//! picked out. Signatures come from the language's API list and from the
//! functions defined in the document itself.

use std::collections::HashMap;
use std::sync::Arc;

use crate::function_list::{self, SymbolKind};
use crate::lang_api::{Signature, matching_paren};

use super::Editor;

/// How far back from the caret to look for the call's `(`.
const CALL_LOOKBACK: usize = 4000;
/// Documents bigger than this aren't searched for their own functions.
const MAX_DOC_CHARS: usize = 2_000_000;

/// The parameter hint shown above the caret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallHint {
    /// Where the call's `(` is.
    pub open: usize,
    pub name: String,
    pub signatures: Vec<Arc<Signature>>,
    /// The parameter the caret is in (0-based).
    pub active: usize,
}

impl CallHint {
    /// The parameter to pick out in `sig`: the active one, or the last when
    /// it takes any number of arguments (`*args`, `...rest`).
    pub fn param_in(&self, sig: &Signature) -> Option<usize> {
        if self.active < sig.params.len() {
            return Some(self.active);
        }
        let last = sig.params.len().checked_sub(1)?;
        let text = &sig.label[sig.params[last].clone()];
        (text.contains("...") || text.starts_with('*') || text.contains("params ")).then_some(last)
    }
}

/// Signatures of the functions a document defines, by name.
pub(super) type DocSignatures = HashMap<String, Vec<Arc<Signature>>>;

impl Editor {
    /// Sets the language's keywords and functions (the app does this every frame).
    pub fn set_api(&mut self, api: Arc<crate::lang_api::LanguageApi>) {
        if !Arc::ptr_eq(&self.api, &api) {
            self.api = api;
            self.doc_signatures = None;
            self.hint = None;
        }
    }

    /// Opens (`open`) or refreshes the parameter hint for the call around the
    /// caret. Refreshing follows the caret from call to call and closes the
    /// hint when the caret leaves every call.
    pub fn update_hint(&mut self, open: bool) {
        if !open && self.hint.is_none() {
            return;
        }
        let primary = self.selections.primary();
        if self.selections.len() != 1 || !primary.is_empty() {
            self.hint = None;
            return;
        }
        let Some((paren, name, active)) = self.call_at(primary.head) else {
            self.hint = None;
            return;
        };
        if let Some(h) = &mut self.hint
            && h.open == paren
        {
            h.active = active;
            return;
        }
        let signatures = self.signatures_for(&name);
        self.hint = (!signatures.is_empty()).then_some(CallHint {
            open: paren,
            name,
            signatures,
            active,
        });
    }

    /// The innermost unclosed `(` before `pos`, the name called there and how
    /// many of its arguments come before `pos`.
    fn call_at(&self, pos: usize) -> Option<(usize, String, usize)> {
        let rope = self.buffer.rope();
        let from = pos.saturating_sub(CALL_LOOKBACK);
        let from = rope.line_to_char(rope.char_to_line(from));
        // Open brackets: (char, position, commas inside it).
        let mut stack: Vec<(char, usize, usize)> = Vec::new();
        let mut quote: Option<char> = None;
        let mut escaped = false;
        for (i, c) in rope.slice(from..pos).chars().enumerate() {
            if let Some(q) = quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q || (c == '\n' && q != '`') {
                    quote = None;
                }
                continue;
            }
            match c {
                '"' | '\'' | '`' => {
                    // An apostrophe inside a word (Rust lifetimes, English) isn't a quote.
                    let prev = (from + i > 0).then(|| rope.char(from + i - 1));
                    if c != '\'' || !prev.is_some_and(|p| p.is_alphanumeric()) {
                        quote = Some(c);
                    }
                }
                '(' | '[' | '{' => stack.push((c, from + i, 0)),
                ')' | ']' | '}' => {
                    stack.pop();
                }
                ',' => {
                    if let Some(top) = stack.last_mut() {
                        top.2 += 1;
                    }
                }
                _ => {}
            }
        }
        let &(_, paren, commas) = stack.iter().rev().find(|(c, ..)| *c == '(')?;
        let name = self.called_name(paren)?;
        if self
            .api
            .is_keyword(name.rsplit(['.', ':']).next().unwrap_or(&name))
            && self.api.signatures(&name).is_empty()
        {
            return None;
        }
        Some((paren, name, commas))
    }

    /// The (possibly qualified) name before the `(` at `paren`: `foo`,
    /// `os.path.join`, `std::mem::swap`, `println!`.
    fn called_name(&self, paren: usize) -> Option<String> {
        let rope = self.buffer.rope();
        let mut end = paren;
        while end > 0 && matches!(rope.char(end - 1), ' ' | '\t') {
            end -= 1;
        }
        let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
        let mut start = end;
        if start > 0 && rope.char(start - 1) == '!' {
            start -= 1;
        }
        let name_end = start;
        while start > 0 && word(rope.char(start - 1)) {
            start -= 1;
        }
        if start == name_end || rope.char(start).is_ascii_digit() {
            return None;
        }
        // Up to three qualifiers: `a.b`, `a::b`, `a->b`, `a:b` (Lua methods).
        for _ in 0..3 {
            let sep = ["::", "->", ".", ":"].into_iter().find(|sep| {
                let n = sep.chars().count();
                start >= n && rope.slice(start - n..start) == *sep
            });
            let Some(sep) = sep else {
                break;
            };
            let mut s = start - sep.chars().count();
            let before = s;
            while s > 0 && word(rope.char(s - 1)) {
                s -= 1;
            }
            if s == before {
                break;
            }
            start = s;
        }
        Some(rope.slice(start..end).to_string())
    }

    /// Signatures for a call to `name`: the language's own, then any the
    /// document defines.
    fn signatures_for(&mut self, name: &str) -> Vec<Arc<Signature>> {
        let mut out = self.api.signatures(name);
        let short = name
            .rsplit(['.', ':', '>'])
            .next()
            .unwrap_or(name)
            .trim_end_matches('!')
            .to_owned();
        let stale = self
            .doc_signatures
            .as_ref()
            .is_none_or(|(v, _)| *v != self.buffer.version());
        let known = self
            .doc_signatures
            .as_ref()
            .is_some_and(|(_, m)| m.contains_key(&short));
        // Rebuild only for a name that isn't known yet; a function that was
        // already found is still found after small edits elsewhere.
        if stale && !known && self.buffer.len_chars() <= MAX_DOC_CHARS {
            let text = self.text_snapshot();
            let sigs = document_signatures(&self.api.language, &text);
            self.doc_signatures = Some((self.buffer.version(), Arc::new(sigs)));
        }
        if let Some((_, map)) = &self.doc_signatures
            && let Some(list) = map.get(&short)
        {
            for sig in list {
                if !out.iter().any(|s| s.label == sig.label) {
                    out.push(sig.clone());
                }
            }
        }
        out
    }
}

/// The signatures of the functions `text` defines, read from their definitions.
pub(super) fn document_signatures(language: &str, text: &str) -> DocSignatures {
    let mut out = DocSignatures::new();
    let symbols = function_list::symbols(language, text);
    if symbols.is_empty() {
        return out;
    }
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    for sym in symbols {
        if !matches!(sym.kind, SymbolKind::Function | SymbolKind::Method) {
            continue;
        }
        let Some(&line_start) = starts.get(sym.line) else {
            continue;
        };
        let name = sym
            .name
            .rsplit(['.', ':'])
            .next()
            .unwrap_or(&sym.name)
            .to_owned();
        // The definition may wrap its parameters over a few lines.
        let end = starts
            .get(sym.line + 12)
            .copied()
            .unwrap_or(text.len())
            .min(line_start + 3000);
        let def = &text[line_start..end];
        if let Some(sig) = signature_from_definition(def, &name)
            && !out
                .get(&name)
                .is_some_and(|l| l.iter().any(|s| s.label == sig.label))
        {
            out.entry(name).or_default().push(Arc::new(sig));
        }
    }
    out
}

/// Builds `name(params) -> ret` from the text of a definition.
fn signature_from_definition(def: &str, name: &str) -> Option<Signature> {
    let first_line = def.lines().next()?;
    let mut search = 0;
    let open = loop {
        let at = first_line[search..].find(name)? + search;
        let before_ok = first_line[..at]
            .chars()
            .last()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let after = &first_line[at + name.len()..];
        // Between the name and `(` may come generics, `=`, `function`, `async`.
        if before_ok && let Some(p) = after.find('(') {
            let between = &after[..p];
            let ok = between.len() <= 40
                && !between.contains([';', ')', '"', '\''])
                && between
                    .chars()
                    .next()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
            if ok {
                break at + name.len() + p;
            }
        }
        search = at + name.len();
    };
    let close = matching_paren(def, open)?;
    let params: String = def[open + 1..close]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let rest_end = def[close + 1..]
        .find(['{', '\n', ';', '='])
        .map_or(def.len(), |i| close + 1 + i);
    let mut rest = def[close + 1..rest_end].trim().trim_end_matches(':').trim();
    if rest.len() > 60 || rest.starts_with("where") {
        rest = "";
    }
    let label = match rest {
        "" => format!("{name}({params})"),
        r if r.starts_with(':') => format!("{name}({params}){r}"),
        r => format!("{name}({params}) {r}"),
    };
    Signature::parse(&label, None).map(|(_, s)| s)
}

#[cfg(test)]
mod tests {
    use super::super::selection::Selections;
    use super::*;

    fn editor(language: &str, text: &str, caret: usize) -> Editor {
        let mut e = Editor::new(text);
        let (mut lib, _) = crate::lang_api::ApiLibrary::load(None, Default::default());
        e.api = lib.get(language);
        e.selections = Selections::caret(caret);
        e
    }

    #[test]
    fn hint_follows_the_argument() {
        let text = "print(a, f(x), 'b,c', ";
        let mut e = editor("Python", text, text.chars().count());
        e.update_hint(true);
        let h = e.hint.clone().unwrap();
        assert_eq!(h.name, "print");
        assert_eq!(h.active, 3);
        assert_eq!(h.param_in(&h.signatures[0]), Some(3));
        // Inside the nested call the hint switches to `f`, which is unknown.
        e.selections = Selections::caret(11);
        e.update_hint(false);
        assert!(e.hint.is_none());
    }

    #[test]
    fn qualified_names_and_keywords() {
        let text = "x = os.path.join(a, ";
        let mut e = editor("Python", text, text.chars().count());
        e.update_hint(true);
        let h = e.hint.clone().unwrap();
        assert_eq!(h.name, "os.path.join");
        assert_eq!(h.signatures[0].label, "os.path.join(path, *paths) -> str");
        assert_eq!(h.active, 1);
        let mut e = editor("JavaScript", "if (a, ", 7);
        e.update_hint(true);
        assert!(e.hint.is_none());
        let mut e = editor("Rust", "println!(\"{}\", ", 15);
        e.update_hint(true);
        assert_eq!(e.hint.unwrap().active, 1);
    }

    #[test]
    fn functions_defined_in_the_document() {
        let text = "fn area(&self, width: f64,\n        height: f64) -> f64 {\n}\nfn main() {\n    area(1.0, ";
        let mut e = editor("Rust", text, text.chars().count());
        e.update_hint(true);
        let h = e.hint.clone().unwrap();
        let sig = &h.signatures[0];
        assert_eq!(sig.label, "area(&self, width: f64, height: f64) -> f64");
        let params: Vec<&str> = sig.params.iter().map(|r| &sig.label[r.clone()]).collect();
        assert_eq!(params, ["width: f64", "height: f64"]);
        assert_eq!(h.active, 1);

        let py = "def greet(name, greeting='hi'):\n    pass\n\ngreet(";
        let mut e = editor("Python", py, py.chars().count());
        e.update_hint(true);
        assert_eq!(
            e.hint.unwrap().signatures[0].label,
            "greet(name, greeting='hi')"
        );
        let js = "const add = (a, b) => a + b;\nfunction mul(a, b): number {\n}\nmul(";
        let sigs = document_signatures("TypeScript", js);
        assert_eq!(sigs["mul"][0].label, "mul(a, b): number");
    }
}
