//! User-defined languages: a small JSON format (`*.lang.json` in the
//! languages folder) that Ettin Edit turns into a sublime-syntax grammar when
//! it loads. See `docs/languages.md` for the format.

use std::collections::BTreeMap;

use serde::Deserialize;

/// A commented starting point for a new `.lang.json` file. It is valid JSON:
/// keys that start with `_` are notes and are ignored.
pub const UDL_TEMPLATE: &str = r##"{
  "_about": "An Ettin Edit user-defined language. Keep it in the languages folder as <something>.lang.json, then choose Language > Reload Languages. Keys that start with _ are notes and are ignored.",

  "name": "My Language",
  "extensions": ["mylang"],

  "_first_line": "Optional regular expression tried on the first line of files whose extension is unknown, for example a shebang.",
  "first_line": "^#!.*\\bmylang\\b",

  "_case_sensitive": "false makes keywords match in any case (IF, If, if).",
  "case_sensitive": true,

  "line_comment": ["//", "#"],
  "block_comment": [["/*", "*/"]],

  "_strings": "Each entry is [open, close] or [open, close, escape]. Strings end at the end of the line unless multiline_strings is true.",
  "strings": [["\"", "\"", "\\"], ["'", "'", "\\"]],
  "multiline_strings": false,

  "_keywords": "Groups: keyword, type, constant, builtin, modifier, operator, variable, or a scope name such as support.class.",
  "keywords": {
    "keyword": ["if", "then", "else", "elif", "end", "while", "for", "in", "return", "break"],
    "type": ["int", "float", "string", "bool"],
    "constant": ["true", "false", "nil"],
    "builtin": ["print", "len", "open"]
  },

  "operators": ["+", "-", "*", "/", "%", "=", "==", "!=", "<", "<=", ">", ">=", "&&", "||", "!"],
  "numbers": true,

  "_folding": "Bracket pairs that open and close a block. They are coloured as brackets today and are kept for grammar-aware folding.",
  "folding": [["{", "}"]]
}
"##;

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn to_vec(&self) -> Vec<String> {
        match self {
            OneOrMany::One(s) => vec![s.clone()],
            OneOrMany::Many(v) => v.clone(),
        }
    }
}

impl Default for OneOrMany {
    fn default() -> Self {
        OneOrMany::Many(Vec::new())
    }
}

fn yes() -> bool {
    true
}

/// The `.lang.json` format.
#[derive(Deserialize)]
pub struct UserLanguage {
    pub name: String,
    #[serde(default)]
    extensions: OneOrMany,
    #[serde(default)]
    pub first_line: Option<String>,
    #[serde(default = "yes")]
    pub case_sensitive: bool,
    #[serde(default)]
    line_comment: OneOrMany,
    #[serde(default)]
    pub block_comment: Vec<[String; 2]>,
    /// `[open, close]` or `[open, close, escape]`.
    #[serde(default)]
    pub strings: Vec<Vec<String>>,
    #[serde(default)]
    pub multiline_strings: bool,
    #[serde(default)]
    pub keywords: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub operators: Vec<String>,
    #[serde(default = "yes")]
    pub numbers: bool,
    /// Bracket pairs that open and close a block.
    #[serde(default)]
    pub folding: Vec<[String; 2]>,
    /// Anything else; only `_notes` are allowed, so typos are reported.
    #[serde(flatten)]
    other: BTreeMap<String, serde_json::Value>,
}

/// The scope for a keyword group.
fn group_scope(group: &str) -> Result<String, String> {
    let scope = match group {
        "keyword" | "keywords" | "control" => "keyword.control",
        "type" | "types" => "storage.type",
        "constant" | "constants" => "constant.language",
        "builtin" | "builtins" | "function" | "functions" => "support.function",
        "modifier" | "modifiers" => "storage.modifier",
        "operator" | "operators" => "keyword.operator.word",
        "variable" | "variables" => "variable.language",
        _ if group.contains('.')
            && group
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')) =>
        {
            return Ok(group.to_owned());
        }
        _ => {
            return Err(format!(
                "unknown keyword group `{group}`; use keyword, type, constant, builtin, \
                 modifier, operator, variable or a scope name such as support.class"
            ));
        }
    };
    Ok(scope.to_owned())
}

/// Quotes text for a YAML single-quoted scalar.
fn yaml(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Text from the user that ends up in the grammar must stay on one line.
fn check_text(what: &str, s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(format!("{what} is empty"));
    }
    if s.chars().any(char::is_control) {
        return Err(format!(
            "{what} `{}` contains a control character",
            s.escape_debug()
        ));
    }
    Ok(())
}

/// A regex alternation matching any of `items` literally, longest first.
fn alternation(items: &[String]) -> String {
    let mut items: Vec<&String> = items.iter().collect();
    items.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    items.dedup();
    items
        .iter()
        .map(|w| regex::escape(w))
        .collect::<Vec<_>>()
        .join("|")
}

impl UserLanguage {
    fn slug(&self) -> String {
        let mut slug = String::new();
        for c in self.name.chars() {
            if c.is_ascii_alphanumeric() {
                slug.push(c.to_ascii_lowercase());
            } else if !slug.ends_with('-') {
                slug.push('-');
            }
        }
        let slug = slug.trim_matches('-');
        if slug.is_empty() {
            "user".to_owned()
        } else {
            slug.to_owned()
        }
    }

    fn validate(&self) -> Result<(), String> {
        check_text("name", self.name.trim())?;
        if let Some(key) = self.other.keys().find(|k| !k.starts_with('_')) {
            return Err(format!(
                "unknown setting `{key}` (start a key with _ to keep it as a note)"
            ));
        }
        Ok(())
    }

    /// The equivalent sublime-syntax grammar.
    pub fn to_sublime_syntax(&self) -> Result<String, String> {
        self.validate()?;
        let slug = self.slug();
        let mut out = String::from("%YAML 1.2\n---\n");
        out.push_str(&format!("name: {}\n", yaml(self.name.trim())));

        let mut exts = Vec::new();
        for ext in &self.extensions.to_vec() {
            let ext = ext.trim_start_matches('.');
            check_text("an extension", ext)?;
            exts.push(yaml(ext));
        }
        out.push_str(&format!("file_extensions: [{}]\n", exts.join(", ")));
        if let Some(first_line) = &self.first_line {
            check_text("first_line", first_line)?;
            if let Some(e) = syntect::parsing::Regex::try_compile(first_line) {
                return Err(format!("first_line is not a valid regex: {e}"));
            }
            out.push_str(&format!("first_line_match: {}\n", yaml(first_line)));
        }
        out.push_str(&format!("scope: source.user.{slug}\n"));
        out.push_str("contexts:\n  main:\n");

        let mut extra_contexts = String::new();
        let mut push_rule = |rule: String| {
            out.push_str(&rule);
        };

        for (i, [open, close]) in self.block_comment.iter().enumerate() {
            check_text("a block comment", open)?;
            check_text("a block comment", close)?;
            push_rule(format!(
                "    - match: {}\n      scope: punctuation.definition.comment.begin.{slug}\n      push: block-comment-{i}\n",
                yaml(&regex::escape(open))
            ));
            extra_contexts.push_str(&format!(
                "  block-comment-{i}:\n    - meta_scope: comment.block.{slug}\n    - match: {}\n      scope: punctuation.definition.comment.end.{slug}\n      pop: true\n",
                yaml(&regex::escape(close))
            ));
        }

        for lc in &self.line_comment.to_vec() {
            check_text("a line comment", lc)?;
            push_rule(format!(
                "    - match: {}\n      scope: comment.line.{slug}\n",
                yaml(&format!("{}.*", regex::escape(lc)))
            ));
        }

        for (i, parts) in self.strings.iter().enumerate() {
            let (open, close, escape) = match parts.as_slice() {
                [o, c] => (o, c, None),
                [o, c, e] if e.is_empty() => (o, c, None),
                [o, c, e] => (o, c, Some(e)),
                _ => {
                    return Err(
                        "each string needs [open, close] or [open, close, escape]".to_owned()
                    );
                }
            };
            check_text("a string delimiter", open)?;
            check_text("a string delimiter", close)?;
            push_rule(format!(
                "    - match: {}\n      scope: punctuation.definition.string.begin.{slug}\n      push: string-{i}\n",
                yaml(&regex::escape(open))
            ));
            let mut ctx = format!("  string-{i}:\n    - meta_scope: string.quoted.{slug}\n");
            if let Some(escape) = escape {
                check_text("a string escape", escape)?;
                // A doubled closing quote ('it''s') or an escape char before anything.
                let pattern = if escape == close {
                    format!("{0}{0}", regex::escape(close))
                } else {
                    format!("{}.", regex::escape(escape))
                };
                ctx.push_str(&format!(
                    "    - match: {}\n      scope: constant.character.escape.{slug}\n",
                    yaml(&pattern)
                ));
            }
            ctx.push_str(&format!(
                "    - match: {}\n      scope: punctuation.definition.string.end.{slug}\n      pop: true\n",
                yaml(&regex::escape(close))
            ));
            if !self.multiline_strings {
                ctx.push_str("    - match: '\\n'\n      pop: true\n");
            }
            extra_contexts.push_str(&ctx);
        }

        if self.numbers {
            push_rule(format!(
                "    - match: '(?<![\\w.])(?:0[xX][0-9A-Fa-f_]+|0[bB][01_]+|\\d[\\d_]*(?:\\.\\d[\\d_]*)?(?:[eE][+-]?\\d+)?)(?![\\w.])'\n      scope: constant.numeric.{slug}\n"
            ));
        }

        for (group, words) in &self.keywords {
            let scope = group_scope(group)?;
            if words.is_empty() {
                continue;
            }
            for w in words {
                check_text(&format!("a word in `{group}`"), w)?;
            }
            let alt = alternation(words);
            let body = if self.case_sensitive {
                format!("(?:{alt})")
            } else {
                format!("(?i:{alt})")
            };
            push_rule(format!(
                "    - match: {}\n      scope: {scope}.{slug}\n",
                yaml(&format!("(?<![\\w]){body}(?![\\w])"))
            ));
        }

        if !self.operators.is_empty() {
            for op in &self.operators {
                check_text("an operator", op)?;
            }
            push_rule(format!(
                "    - match: {}\n      scope: keyword.operator.{slug}\n",
                yaml(&alternation(&self.operators))
            ));
        }

        for [open, close] in &self.folding {
            check_text("a folding bracket", open)?;
            check_text("a folding bracket", close)?;
            push_rule(format!(
                "    - match: {}\n      scope: punctuation.section.block.begin.{slug}\n    - match: {}\n      scope: punctuation.section.block.end.{slug}\n",
                yaml(&regex::escape(open)),
                yaml(&regex::escape(close))
            ));
        }

        out.push_str(&extra_contexts);
        Ok(out)
    }
}

/// Parses a `.lang.json` document and returns the equivalent sublime-syntax grammar.
pub fn to_sublime_syntax(json: &str) -> Result<String, String> {
    let lang: UserLanguage = serde_json::from_str(json).map_err(|e| e.to_string())?;
    lang.to_sublime_syntax()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::Highlighter;
    use crate::highlight::tests::assert_scope;
    use syntect::parsing::SyntaxDefinition;

    fn load(json: &str) -> Result<SyntaxDefinition, String> {
        let yaml = to_sublime_syntax(json)?;
        SyntaxDefinition::load_from_str(&yaml, true, None).map_err(|e| format!("{e}\n{yaml}"))
    }

    fn highlighter_with(json: &str) -> Highlighter {
        let dir =
            std::env::temp_dir().join(format!("ettin-udl-{}-{}", std::process::id(), json.len()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t.lang.json"), json).unwrap();
        let (h, errors) = Highlighter::new(Some(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(errors.is_empty(), "{errors:#?}");
        h
    }

    #[test]
    fn template_is_valid_json_and_loads() {
        let value: serde_json::Value = serde_json::from_str(UDL_TEMPLATE).unwrap();
        assert_eq!(value["name"], "My Language");
        let def = load(UDL_TEMPLATE).unwrap();
        assert_eq!(def.name, "My Language");
        assert_eq!(def.file_extensions, vec!["mylang"]);
        assert!(def.first_line_match.is_some());
    }

    #[test]
    fn template_highlights() {
        let h = highlighter_with(UDL_TEMPLATE);
        let src = "# note\nif x == 0x1F then print(\"a\\\"b\") /* c */ end\n";
        let l = "My Language";
        assert_scope(&h, l, src, "if", "keyword.control.my-language");
        assert_scope(&h, l, src, "print", "support.function");
        assert_scope(&h, l, src, "0x1F", "constant.numeric");
        assert_scope(&h, l, src, "==", "keyword.operator");
        assert_scope(&h, l, src, "\\\"", "constant.character.escape");
        assert_scope(&h, l, src, "# note", "comment.line");
        assert_scope(&h, l, src, " c ", "comment.block");
    }

    #[test]
    fn words_are_escaped_and_case_folded() {
        let json = r#"{
            "name": "Odd (words)",
            "extensions": "odd",
            "case_sensitive": false,
            "keywords": { "keyword": ["c++", "a.b", "$x", "it's"], "support.class": ["Thing"] },
            "strings": [["'", "'", "'"]],
            "operators": ["**", "*", "|>"]
        }"#;
        let h = highlighter_with(json);
        let l = "Odd (words)";
        assert_scope(&h, l, "C++ + 1\n", "C++", "keyword.control.odd-words");
        assert_scope(&h, l, "A.B\n", "A.B", "keyword.control");
        assert_scope(&h, l, "$x\n", "$x", "keyword.control");
        // `a.b` must not match `axb` as an unescaped regex would.
        let toks = crate::highlight::tests::tokens(&h, l, "axb\n");
        assert!(toks.iter().all(|(_, s)| !s.contains("keyword")), "{toks:?}");
        assert_scope(&h, l, "x |> y\n", "|>", "keyword.operator");
        assert_scope(&h, l, "thing\n", "thing", "support.class");
        // A doubled quote stays inside the string.
        assert_scope(&h, l, "'it''s'\n", "''", "constant.character.escape");
    }

    #[test]
    fn bad_files_report_why() {
        assert!(to_sublime_syntax("{").is_err());
        assert!(
            to_sublime_syntax(r#"{"name": "X", "colour": 1}"#)
                .unwrap_err()
                .contains("colour")
        );
        assert!(
            to_sublime_syntax(r#"{"name": "X", "keywords": {"shiny": ["a"]}}"#)
                .unwrap_err()
                .contains("shiny")
        );
        assert!(to_sublime_syntax(r#"{"name": ""}"#).is_err());
        assert!(to_sublime_syntax(r#"{"name": "X", "strings": [["a"]]}"#).is_err());
        assert!(to_sublime_syntax(r#"{"name": "X", "operators": ["a\nb"]}"#).is_err());
        // A broken first-line regex is caught when the grammar loads.
        assert!(load(r#"{"name": "X", "first_line": "("}"#).is_err());
        // Notes are fine.
        assert!(to_sublime_syntax(r#"{"name": "X", "_why": "notes"}"#).is_ok());
    }
}
