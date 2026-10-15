//! File > Print and File > Export as HTML.

use std::path::PathBuf;

use super::EttinApp;
use crate::print::{HtmlOptions, to_html};
use crate::system;

/// A file name made from a tab's title.
fn file_stem(title: &str) -> String {
    let stem: String = title
        .trim_start_matches('*')
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "._- ".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if stem.trim().is_empty() {
        "document".to_owned()
    } else {
        stem.trim().to_owned()
    }
}

impl EttinApp {
    /// Prints the current document through the browser, or (`export`) saves it as HTML.
    pub(super) fn print(&mut self, export: bool) {
        let doc = &mut self.docs[self.active];
        let text = doc.editor.text_snapshot();
        let title = doc.title();
        let current = &self.themes[self.theme_idx];
        // Paper is white: a dark theme's colours are swapped for a light theme's.
        let theme = if !export && current.palette.dark {
            self.themes
                .iter()
                .find(|t| t.name == "Ettin Light")
                .or_else(|| self.themes.iter().find(|t| !t.palette.dark))
                .unwrap_or(current)
        } else {
            current
        };
        let syntax = self.highlighter.syntax(&doc.syntax);
        let opts = HtmlOptions {
            title: &title,
            line_numbers: if export {
                self.settings.show_line_numbers
            } else {
                self.settings.print_line_numbers
            },
            colours: export || self.settings.print_colours,
            for_print: !export,
            tab_width: crate::editor::layout::tab_width(),
            font_size: if export { 14.0 } else { 11.0 },
        };
        let html = to_html(
            &text,
            syntax,
            &self.highlighter.syntaxes,
            &theme.syntax,
            &opts,
        );
        let path = if export {
            let mut dialog = rfd::FileDialog::new()
                .set_title("Export as HTML")
                .set_file_name(format!("{}.html", file_stem(&title)))
                .add_filter("HTML", &["html", "htm"]);
            if let Some(dir) = doc.path.as_deref().and_then(|p| p.parent()) {
                dialog = dialog.set_directory(dir);
            }
            let Some(path) = dialog.save_file() else {
                return;
            };
            path
        } else {
            let dir = std::env::temp_dir().join("ettin-edit-print");
            if let Err(e) = std::fs::create_dir_all(&dir) {
                self.errors
                    .push(format!("Couldn't make {}: {e}", dir.display()));
                return;
            }
            dir.join(format!("{}.html", file_stem(&title)))
        };
        if let Err(e) = std::fs::write(&path, html) {
            self.errors
                .push(format!("Couldn't write {}: {e}", path.display()));
            return;
        }
        if !export && let Err(e) = system::open_url(&file_url(&path)) {
            self.errors.push(e);
        }
    }
}

/// A `file://` URL for a local file, so browsers open it rather than an editor.
fn file_url(path: &std::path::Path) -> String {
    let path: PathBuf = path.to_path_buf();
    let text = path.display().to_string().replace('\\', "/");
    let text = text.replace(' ', "%20").replace('#', "%23");
    if text.starts_with('/') {
        format!("file://{text}")
    } else {
        format!("file:///{text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_safe_file_names_and_urls() {
        assert_eq!(file_stem("*notes: v2?.md"), "notes_ v2_.md");
        assert_eq!(file_stem("***"), "document");
        assert_eq!(
            file_url(std::path::Path::new("/tmp/a b#1.html")),
            "file:///tmp/a%20b%231.html"
        );
    }
}
