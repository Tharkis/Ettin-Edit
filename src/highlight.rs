//! Syntax highlighting: turns document text into an egui `LayoutJob` using
//! syntect grammars and colour schemes.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme as SyntectTheme};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Files larger than this are shown without highlighting to keep typing responsive.
pub const MAX_HIGHLIGHT_BYTES: usize = 2 * 1024 * 1024;

pub const PLAIN_TEXT: &str = "Plain Text";

/// The highlighted layout of a document, reused until the text, language,
/// theme or font size changes.
#[derive(Default)]
pub struct HighlightCache {
    key: u64,
    job: Option<LayoutJob>,
}

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

    fn syntax(&self, name: &str) -> &SyntaxReference {
        self.syntaxes
            .find_syntax_by_name(name)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text())
    }

    /// Returns the highlighted layout for `text`, rebuilding it only when something changed.
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &self,
        cache: &mut HighlightCache,
        text: &str,
        syntax: &str,
        theme_name: &str,
        theme: &SyntectTheme,
        font: &FontId,
        fallback_color: Color32,
    ) -> LayoutJob {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        syntax.hash(&mut hasher);
        theme_name.hash(&mut hasher);
        font.size.to_bits().hash(&mut hasher);
        let key = hasher.finish();

        if cache.key != key || cache.job.is_none() {
            cache.job = Some(self.build_job(text, syntax, theme, font, fallback_color));
            cache.key = key;
        }
        cache.job.clone().unwrap_or_default()
    }

    fn build_job(
        &self,
        text: &str,
        syntax: &str,
        theme: &SyntectTheme,
        font: &FontId,
        fallback_color: Color32,
    ) -> LayoutJob {
        let mut job = LayoutJob::default();
        if syntax == PLAIN_TEXT || text.len() > MAX_HIGHLIGHT_BYTES {
            job.append(text, 0.0, TextFormat::simple(font.clone(), fallback_color));
            return job;
        }

        let mut highlighter = HighlightLines::new(self.syntax(syntax), theme);
        for line in LinesWithEndings::from(text) {
            match highlighter.highlight_line(line, &self.syntaxes) {
                Ok(ranges) => {
                    for (style, piece) in ranges {
                        let fg = style.foreground;
                        let mut format = TextFormat::simple(
                            font.clone(),
                            Color32::from_rgba_unmultiplied(fg.r, fg.g, fg.b, fg.a),
                        );
                        format.italics = style.font_style.contains(FontStyle::ITALIC);
                        if style.font_style.contains(FontStyle::UNDERLINE) {
                            format.underline = egui::Stroke::new(1.0, format.color);
                        }
                        job.append(piece, 0.0, format);
                    }
                }
                // A grammar error shouldn't lose text: show the line unstyled.
                Err(_) => job.append(line, 0.0, TextFormat::simple(font.clone(), fallback_color)),
            }
        }
        job
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
