//! Language detection and the bundled syntax definitions. The highlighting
//! itself lives in `editor::highlight`.

use std::path::Path;

use syntect::parsing::{SyntaxReference, SyntaxSet};

pub const PLAIN_TEXT: &str = "Plain Text";

pub struct Highlighter {
    pub syntaxes: SyntaxSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
        }
    }

    /// Language names for the Language menu, sorted case-insensitively.
    pub fn language_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .syntaxes
            .syntaxes()
            .iter()
            .filter(|s| !s.hidden)
            .map(|s| s.name.clone())
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup();
        names
    }

    /// Guesses the language from the file name, then from the first line (shebangs, `<?xml`).
    pub fn detect(&self, path: Option<&Path>, text: &str) -> String {
        let by_path = path.and_then(|p| {
            let name = p.file_name()?.to_str()?;
            self.syntaxes
                .find_syntax_by_extension(name)
                .or_else(|| {
                    let ext = p.extension()?.to_str()?;
                    self.syntaxes
                        .find_syntax_by_extension(ext)
                        .or_else(|| self.syntaxes.find_syntax_by_extension(&ext.to_lowercase()))
                })
                .or_else(|| self.extra_extension(name))
        });
        let syntax = by_path.or_else(|| {
            let first_line = text.lines().next().unwrap_or("");
            self.syntaxes.find_syntax_by_first_line(first_line)
        });
        syntax
            .map(|s| s.name.clone())
            .unwrap_or_else(|| PLAIN_TEXT.to_owned())
    }

    /// Common extensions the bundled grammars don't claim, mapped to a close match.
    fn extra_extension(&self, file_name: &str) -> Option<&SyntaxReference> {
        let ext = file_name.rsplit('.').next()?.to_lowercase();
        let target = match ext.as_str() {
            "ts" | "tsx" | "mjs" | "cjs" | "jsx" => "JavaScript",
            "xsd" | "xsl" | "xslt" | "svg" | "csproj" | "vcxproj" | "plist" => "XML",
            "jsonc" | "json5" => "JSON",
            "htm" | "xhtml" => "HTML",
            "zsh" | "fish" => "Bourne Again Shell (bash)",
            _ => return None,
        };
        self.syntaxes.find_syntax_by_name(target)
    }

    pub fn syntax(&self, name: &str) -> &SyntaxReference {
        self.syntaxes
            .find_syntax_by_name(name)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_languages() {
        let h = Highlighter::new();
        assert_eq!(h.detect(Some(Path::new("main.rs")), ""), "Rust");
        assert_eq!(h.detect(Some(Path::new("a.py")), ""), "Python");
        assert_eq!(h.detect(Some(Path::new("data.xsd")), ""), "XML");
        assert_eq!(h.detect(None, "#!/usr/bin/env python3\n"), "Python");
        assert_eq!(h.detect(Some(Path::new("notes")), "hello"), PLAIN_TEXT);
    }
}
