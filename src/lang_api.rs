//! Language knowledge for completion and parameter hints: each language's
//! keywords and the signatures of its common functions.
//!
//! Built-in lists live in `assets/api/*.json`; files in the user's `api`
//! folder add to them (see `docs/completion.md`). Languages without a list get
//! the keywords their grammar highlights. Functions defined in the document
//! itself are found by the editor, using the function list's parsers.

use std::collections::{BTreeSet, HashMap};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use regex::Regex;
use serde::Deserialize;
use syntect::parsing::SyntaxDefinition;
use syntect::parsing::syntax_definition::Pattern;

/// Ettin's own lists: (file name, source).
const BUILTIN_APIS: &[(&str, &str)] = &[
    ("python.json", include_str!("../assets/api/python.json")),
    (
        "javascript.json",
        include_str!("../assets/api/javascript.json"),
    ),
    ("rust.json", include_str!("../assets/api/rust.json")),
    ("c.json", include_str!("../assets/api/c.json")),
    ("cpp.json", include_str!("../assets/api/cpp.json")),
    ("csharp.json", include_str!("../assets/api/csharp.json")),
    ("java.json", include_str!("../assets/api/java.json")),
    ("go.json", include_str!("../assets/api/go.json")),
    ("php.json", include_str!("../assets/api/php.json")),
    ("lua.json", include_str!("../assets/api/lua.json")),
    ("ruby.json", include_str!("../assets/api/ruby.json")),
    ("shell.json", include_str!("../assets/api/shell.json")),
    (
        "powershell.json",
        include_str!("../assets/api/powershell.json"),
    ),
    ("sql.json", include_str!("../assets/api/sql.json")),
    ("html.json", include_str!("../assets/api/html.json")),
    ("css.json", include_str!("../assets/api/css.json")),
    ("kotlin.json", include_str!("../assets/api/kotlin.json")),
    ("swift.json", include_str!("../assets/api/swift.json")),
];

/// The commented example written to a new completion lists folder.
pub const API_TEMPLATE: &str = r#"{
  "_about": "Keywords and function signatures for completion and parameter hints. Copy this to a .json file in this folder, then choose Language > Reload Languages. Keys that start with _ are notes and are ignored. Lists add to the built-in ones.",
  "languages": ["Python"],
  "_ignore_case": "true for languages like SQL, where IF and if are the same word.",
  "ignore_case": false,
  "keywords": ["my_keyword"],
  "functions": [
    "my_function(first, second=2) -> int",
    {"signature": "mymodule.helper(path, *, verbose=False)", "doc": "Shown under the signature."}
  ]
}
"#;

/// One way to call a function: `label` is the whole signature as shown,
/// `params` the byte range of each parameter within it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub label: String,
    pub params: Vec<Range<usize>>,
    pub doc: Option<String>,
}

impl Signature {
    /// Parses `name(a, b = 1) -> T`. `None` without a parameter list.
    pub fn parse(label: &str, doc: Option<String>) -> Option<(String, Self)> {
        let label = label.trim();
        let open = param_open(label)?;
        // `fn map<U>(…)` is called as `map`: drop generics and leading words.
        let mut depth = 0i32;
        let bare: String = label[..open]
            .chars()
            .filter(|&c| {
                match c {
                    '<' => depth += 1,
                    '>' => depth -= 1,
                    _ => return depth == 0,
                }
                false
            })
            .collect();
        let name = bare
            .split_whitespace()
            .last()
            .unwrap_or("")
            .trim_start_matches(['*', '&'])
            .trim_end_matches(':');
        if name.is_empty() {
            return None;
        }
        let close = matching_paren(label, open)?;
        let mut params = split_params(label, open + 1, close);
        // A method's receiver isn't passed between the parentheses.
        if params
            .first()
            .is_some_and(|r| is_receiver(&label[r.clone()]))
        {
            params.remove(0);
        }
        Some((
            name.to_owned(),
            Self {
                label: label.to_owned(),
                params,
                doc: doc.filter(|d| !d.trim().is_empty()),
            },
        ))
    }
}

/// `self`, `&mut self`, `this` and the like.
pub fn is_receiver(param: &str) -> bool {
    let p = param.trim_start_matches('&').trim();
    let p = p.strip_prefix("mut ").unwrap_or(p).trim();
    let p = p.strip_prefix('\'').map_or(p, |rest| {
        rest.split_once(' ').map_or(rest, |(_, after)| after).trim()
    });
    matches!(p, "self" | "cls" | "this") || p.starts_with("self:")
}

/// The `(` that starts the parameter list: the first one outside `<…>` generics.
fn param_open(label: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut prev = ' ';
    for (i, c) in label.char_indices() {
        match c {
            '<' => depth += 1,
            '>' if prev != '-' && prev != '=' => depth -= 1,
            '(' if depth <= 0 => return Some(i),
            _ => {}
        }
        prev = c;
    }
    None
}

/// The byte index of the `)` that closes the `(` at `open`.
pub fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in text[open..].char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' | '`' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return (c == ')').then_some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// The trimmed byte ranges of the comma-separated parameters in `text[from..to]`.
fn split_params(text: &str, from: usize, to: usize) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut start = from;
    let push = |a: usize, b: usize, out: &mut Vec<Range<usize>>| {
        let s = &text[a..b];
        let lead = s.len() - s.trim_start().len();
        let trail = s.len() - s.trim_end().len();
        if a + lead < b - trail {
            out.push(a + lead..b - trail);
        }
    };
    for (i, c) in text[from..to].char_indices() {
        let i = from + i;
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            // `->` and `=>` are arrows, not closing brackets.
            '>' if !matches!(text[..i].chars().last(), Some('-' | '=')) => depth -= 1,
            ',' if depth <= 0 => {
                push(start, i, &mut out);
                start = i + 1;
            }
            _ => {}
        }
    }
    push(start, to, &mut out);
    out
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FunctionEntry {
    Plain(String),
    Documented {
        #[serde(alias = "sig")]
        signature: String,
        #[serde(default)]
        doc: Option<String>,
    },
}

#[derive(Deserialize)]
struct ApiFile {
    languages: Vec<String>,
    #[serde(default)]
    ignore_case: bool,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    functions: Vec<FunctionEntry>,
}

/// What completion and parameter hints know about one language.
#[derive(Debug, Default)]
pub struct LanguageApi {
    /// The language's name as in the Language menu.
    pub language: String,
    pub ignore_case: bool,
    pub keywords: Vec<String>,
    /// Function names for completion, sorted.
    pub function_names: Vec<String>,
    /// Signatures by name: the full name (`os.path.join`) and the last part (`join`).
    signatures: HashMap<String, Vec<Arc<Signature>>>,
}

impl LanguageApi {
    fn key(&self, name: &str) -> String {
        if self.ignore_case {
            name.to_lowercase()
        } else {
            name.to_owned()
        }
    }

    fn add_function(&mut self, name: &str, sig: Signature) {
        let sig = Arc::new(sig);
        let short = last_segment(name);
        let mut keys = vec![self.key(name)];
        if short != name {
            keys.push(self.key(short));
        }
        for key in keys {
            let list = self.signatures.entry(key).or_default();
            if !list.iter().any(|s| s.label == sig.label) {
                list.push(sig.clone());
            }
        }
        if !self.function_names.iter().any(|n| n == short) {
            self.function_names.push(short.to_owned());
        }
    }

    /// Signatures for a call to `name` (which may be qualified, like
    /// `console.log` or `std::mem::swap`): the most qualified match wins.
    pub fn signatures(&self, name: &str) -> Vec<Arc<Signature>> {
        let mut rest = name;
        loop {
            if let Some(list) = self.signatures.get(&self.key(rest)) {
                return list.clone();
            }
            match next_segment(rest) {
                Some(r) => rest = r,
                None => return Vec::new(),
            }
        }
    }

    pub fn is_keyword(&self, word: &str) -> bool {
        if self.ignore_case {
            self.keywords.iter().any(|k| k.eq_ignore_ascii_case(word))
        } else {
            self.keywords.iter().any(|k| k == word)
        }
    }

    /// The first signature of `name`, for the completion list.
    pub fn detail(&self, name: &str) -> Option<&str> {
        self.signatures
            .get(&self.key(name))
            .and_then(|l| l.first())
            .map(|s| s.label.as_str())
    }
}

const SEPARATORS: &[&str] = &["::", "->", ".", ":"];

/// `a.b.c` → `b.c`; `None` for a plain name.
fn next_segment(name: &str) -> Option<&str> {
    SEPARATORS
        .iter()
        .filter_map(|sep| name.find(sep).map(|i| (i, sep.len())))
        .min()
        .map(|(i, len)| &name[i + len..])
}

fn last_segment(name: &str) -> &str {
    let mut rest = name;
    while let Some(r) = next_segment(rest) {
        rest = r;
    }
    rest
}

/// Every language's API, built on first use.
#[derive(Default)]
pub struct ApiLibrary {
    /// Parsed list files by language name.
    files: HashMap<String, Vec<Arc<ApiFileData>>>,
    /// Keywords from each grammar, for languages without a list.
    grammar_words: HashMap<String, Vec<String>>,
    built: HashMap<String, Arc<LanguageApi>>,
}

struct ApiFileData {
    ignore_case: bool,
    keywords: Vec<String>,
    functions: Vec<(String, Signature)>,
}

/// Where the user's own lists live.
pub fn api_dir() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("api"))
}

impl ApiLibrary {
    /// Loads the built-in lists, then the user's from `user_dir`. Returns a
    /// message for each user file that could not be read.
    pub fn load(
        user_dir: Option<&Path>,
        grammar_words: HashMap<String, Vec<String>>,
    ) -> (Self, Vec<String>) {
        let mut lib = Self {
            grammar_words,
            ..Self::default()
        };
        for (file, src) in BUILTIN_APIS {
            if let Err(e) = lib.add_source(src) {
                // Covered by tests; never stop the editor over it.
                eprintln!("Ettin Edit: built-in API list {file} failed to load: {e}");
            }
        }
        let mut errors = Vec::new();
        if let Some(dir) = user_dir
            && let Ok(entries) = std::fs::read_dir(dir)
        {
            let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();
            for path in paths {
                if path
                    .extension()
                    .is_none_or(|e| !e.eq_ignore_ascii_case("json"))
                {
                    continue;
                }
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                let result = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|src| lib.add_source(&src));
                if let Err(e) = result {
                    errors.push(format!(
                        "Skipped the API file {}: {e}",
                        name.unwrap_or_default()
                    ));
                }
            }
        }
        (lib, errors)
    }

    fn add_source(&mut self, src: &str) -> Result<(), String> {
        let file: ApiFile = serde_json::from_str(src).map_err(|e| e.to_string())?;
        if file.languages.is_empty() {
            return Err("`languages` names no language".to_owned());
        }
        let functions = file
            .functions
            .into_iter()
            .filter_map(|f| match f {
                FunctionEntry::Plain(s) => Signature::parse(&s, None),
                FunctionEntry::Documented { signature, doc } => Signature::parse(&signature, doc),
            })
            .collect();
        let data = Arc::new(ApiFileData {
            ignore_case: file.ignore_case,
            keywords: file.keywords,
            functions,
        });
        for lang in file.languages {
            self.files.entry(lang).or_default().push(data.clone());
        }
        Ok(())
    }

    /// The API for `language` (empty for one nothing is known about).
    pub fn get(&mut self, language: &str) -> Arc<LanguageApi> {
        if let Some(api) = self.built.get(language) {
            return api.clone();
        }
        let mut api = LanguageApi {
            language: language.to_owned(),
            ..LanguageApi::default()
        };
        let mut keywords = BTreeSet::new();
        match self.files.get(language) {
            Some(files) => {
                for f in files {
                    api.ignore_case |= f.ignore_case;
                    keywords.extend(f.keywords.iter().cloned());
                    for (name, sig) in &f.functions {
                        api.add_function(name, sig.clone());
                    }
                }
            }
            None => {
                if let Some(words) = self.grammar_words.get(language) {
                    keywords.extend(words.iter().cloned());
                }
            }
        }
        api.keywords = keywords.into_iter().collect();
        api.function_names.sort();
        let api = Arc::new(api);
        self.built.insert(language.to_owned(), api.clone());
        api
    }
}

/// Words a grammar highlights as keywords, types, constants or built-ins:
/// the plain-word alternatives of its keyword patterns, such as `\b(if|else)\b`.
pub fn grammar_keywords(def: &SyntaxDefinition) -> Vec<String> {
    static GROUP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\((?:\?:|\?i:|)((?:[A-Za-z_][A-Za-z0-9_]*\|)+[A-Za-z_][A-Za-z0-9_]*)\)")
            .expect("valid regex")
    });
    const SCOPES: &[&str] = &[
        "keyword",
        "storage",
        "constant.language",
        "support",
        "variable.language",
    ];
    let mut words = BTreeSet::new();
    for context in def.contexts.values() {
        for pattern in &context.patterns {
            let Pattern::Match(m) = pattern else {
                continue;
            };
            let mut scopes = m.scope.clone();
            if let Some(captures) = &m.captures {
                for (_, s) in captures {
                    scopes.extend(s.iter().copied());
                }
            }
            let wanted = scopes.iter().any(|s| {
                let s = s.build_string();
                SCOPES
                    .iter()
                    .any(|p| s == *p || s.starts_with(&format!("{p}.")))
            });
            if !wanted {
                continue;
            }
            let re = m.regex.regex_str();
            for cap in GROUP.captures_iter(re) {
                let whole = cap.get(0).expect("group 0");
                let before = &re[..whole.start()];
                let after = &re[whole.end()..];
                // Reject groups that are only part of a word, like `a(?:bstract|s)`.
                let ok_before = before.is_empty()
                    || before.ends_with("\\b")
                    || before.chars().last().is_some_and(|c| !c.is_alphanumeric());
                let ok_after = after
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_alphanumeric() && !"?*+{".contains(c));
                if ok_before && ok_after {
                    words.extend(
                        cap[1]
                            .split('|')
                            .filter(|w| w.len() >= 2)
                            .map(str::to_owned),
                    );
                }
            }
        }
    }
    words.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_parse_parameters() {
        let (name, sig) = Signature::parse("print(*objects, sep=' ', end='\\n')", None).unwrap();
        assert_eq!(name, "print");
        let params: Vec<&str> = sig.params.iter().map(|r| &sig.label[r.clone()]).collect();
        assert_eq!(params, ["*objects", "sep=' '", "end='\\n'"]);
        let (_, sig) =
            Signature::parse("fn map<U, F: Fn(T) -> U>(self, f: F) -> Option<U>", None).unwrap();
        let params: Vec<&str> = sig.params.iter().map(|r| &sig.label[r.clone()]).collect();
        assert_eq!(params, ["f: F"]);
        let (_, sig) = Signature::parse("Map<K, V>.get(key: K): V", None).unwrap();
        assert_eq!(sig.params.len(), 1);
        assert!(Signature::parse("noparens", None).is_none());
        let (_, empty) = Signature::parse("now()", None).unwrap();
        assert!(empty.params.is_empty());
    }

    #[test]
    fn builtin_lists_load() {
        for (file, src) in BUILTIN_APIS {
            let mut lib = ApiLibrary::default();
            lib.add_source(src)
                .unwrap_or_else(|e| panic!("{file}: {e}"));
            let data: ApiFile = serde_json::from_str(src).unwrap();
            for f in &data.functions {
                let s = match f {
                    FunctionEntry::Plain(s) => s,
                    FunctionEntry::Documented { signature, .. } => signature,
                };
                assert!(
                    Signature::parse(s, None).is_some(),
                    "{file}: bad signature {s}"
                );
            }
            for lang in &data.languages {
                let api = lib.get(lang);
                assert!(!api.keywords.is_empty(), "{file}: no keywords for {lang}");
            }
        }
    }

    #[test]
    fn qualified_lookup() {
        let mut lib = ApiLibrary::default();
        lib.add_source(
            r#"{"languages": ["Test"], "keywords": ["if"], "functions": [
                "os.path.join(path, *paths)", "join(sep, items)",
                {"signature": "len(s)", "doc": "Length"}]}"#,
        )
        .unwrap();
        let api = lib.get("Test");
        assert_eq!(
            api.signatures("os.path.join")[0].label,
            "os.path.join(path, *paths)"
        );
        assert_eq!(api.signatures("join").len(), 2);
        assert_eq!(api.signatures("x.len")[0].doc.as_deref(), Some("Length"));
        assert!(api.signatures("nothing").is_empty());
        assert_eq!(api.function_names, ["join", "len"]);
        assert!(api.is_keyword("if"));
    }

    #[test]
    fn grammar_keywords_for_unlisted_languages() {
        let h = crate::highlight::tests::shared();
        let words = &h.grammar_words["Go"];
        assert!(words.iter().any(|w| w == "func"));
        let mut lib = ApiLibrary::load(None, h.grammar_words.clone()).0;
        assert!(lib.get("Go").keywords.iter().any(|w| w == "func"));
    }
}
