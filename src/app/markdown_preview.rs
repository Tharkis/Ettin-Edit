//! View > Markdown Preview: the Markdown tab you are editing, rendered in a
//! side panel that follows the tab with focus, updates as you type and
//! scrolls along with the text. Drawn like the user manual.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

use super::EttinApp;
use crate::plugins::panel::{Item, Panel, PanelEvent};
use crate::text_tools_ui::Notice;

/// The "plugin" the preview panel belongs to: no plugin has an empty id.
pub(super) const OWNER: &str = "";
const PANEL: &str = "markdown-preview";
/// Bigger tabs aren't previewed, so typing stays smooth.
const MAX_CHARS: usize = 2_000_000;
/// Wait this long after typing before updating.
const DELAY: f64 = 0.3;
const EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mdx"];

pub(super) struct MarkdownPreview {
    pub shown: bool,
    /// Scroll along with the text.
    sync: bool,
    /// What is shown: (tab, version, sync).
    showing: Option<(u64, u64, bool)>,
    /// When the shown tab last changed, until the preview catches up.
    changed: Option<f64>,
}

impl Default for MarkdownPreview {
    fn default() -> Self {
        Self {
            shown: false,
            sync: true,
            showing: None,
            changed: None,
        }
    }
}

/// True for tabs in the Markdown language or with a Markdown file name.
pub(super) fn is_markdown(language: &str, path: Option<&Path>) -> bool {
    language.to_lowercase().starts_with("markdown")
        || path
            .and_then(|p| p.extension())
            .is_some_and(|e| EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

/// Drops a YAML front matter block (`---` … `---` at the very top).
/// Returns the rest and the line (from 0) it starts on.
pub(super) fn split_front_matter(text: &str) -> (&str, usize) {
    let Some(body) = text.strip_prefix("---\n") else {
        return (text, 0);
    };
    let Some(end) = body.find("\n---") else {
        return (text, 0);
    };
    let mut rest = &body[end + 4..];
    // Only a line of just "---" closes it.
    let line_end = rest.find('\n').unwrap_or(rest.len());
    if !rest[..line_end].trim().is_empty() {
        return (text, 0);
    }
    rest = rest.get(line_end + 1..).unwrap_or("");
    let skipped = &text[..text.len() - rest.len()];
    (rest, skipped.matches('\n').count())
}

impl EttinApp {
    pub(super) fn toggle_markdown_preview(&mut self) {
        let p = &mut self.markdown_preview;
        p.shown = !p.shown;
        p.showing = None;
        if !p.shown {
            self.plugins
                .panels
                .retain(|panel| !(panel.plugin == OWNER && panel.id == PANEL));
        }
    }

    /// Keeps the preview level with the active tab. Called each frame.
    pub(super) fn markdown_preview_frame(&mut self, ctx: &egui::Context) {
        if !self.markdown_preview.shown {
            return;
        }
        let Some(doc) = self.docs.get(self.active) else {
            return;
        };
        let now = ctx.input(|i| i.time);
        let p = &mut self.markdown_preview;
        let key = (doc.id, doc.editor.buffer.version(), p.sync);
        let same_tab = p
            .showing
            .is_some_and(|(id, _, sync)| id == key.0 && sync == key.2);
        let panel_there = self
            .plugins
            .panels
            .iter()
            .any(|panel| panel.plugin == OWNER && panel.id == PANEL);
        if p.showing == Some(key) && panel_there {
            return;
        }
        if same_tab && panel_there {
            // Typing: update a moment after it stops.
            let since = *p.changed.get_or_insert(now);
            if now - since < DELAY {
                ctx.request_repaint_after(Duration::from_secs_f64(DELAY));
                return;
            }
        }
        p.changed = None;
        p.showing = Some(key);
        let to_top = !same_tab;
        let title = t!("markdown-preview-title", title = doc.title());
        let items = self.markdown_preview_items();
        let index = match self
            .plugins
            .panels
            .iter()
            .position(|panel| panel.plugin == OWNER && panel.id == PANEL)
        {
            Some(i) => i,
            None => {
                self.plugins.panels.push(Panel::new(OWNER, PANEL));
                self.plugins.panels.len() - 1
            }
        };
        let panel = &mut self.plugins.panels[index];
        panel.title = title;
        panel.set_items(items);
        if to_top {
            panel.scroll_to_top();
        }
    }

    fn markdown_preview_items(&self) -> Vec<Item> {
        let doc = &self.docs[self.active];
        let note = |text: String| {
            vec![Item::Label {
                text,
                weak: true,
                mono: false,
            }]
        };
        if !is_markdown(&doc.syntax, doc.path.as_deref()) {
            return note(t!("markdown-preview-not-markdown"));
        }
        if doc.editor.buffer.len_chars() > MAX_CHARS {
            return note(t!("markdown-preview-too-big"));
        }
        let all = doc.editor.buffer.text();
        let (text, line) = split_front_matter(&all);
        if text.trim().is_empty() {
            return note(t!("markdown-preview-empty"));
        }
        let sync = self.markdown_preview.sync;
        vec![
            Item::Checkbox {
                id: "sync".into(),
                label: t!("markdown-preview-sync"),
                value: sync,
            },
            Item::Markdown {
                id: "page".into(),
                text: text.to_owned(),
                base: doc
                    .path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(|p| p.display().to_string()),
                doc: sync.then_some(doc.id),
                line,
            },
        ]
    }

    /// Something the user did in the preview panel.
    pub(super) fn markdown_preview_event(&mut self, event: PanelEvent) {
        match (event.kind, event.item.as_str()) {
            ("close", _) => self.toggle_markdown_preview(),
            ("change", "sync") => {
                self.markdown_preview.sync = event.value.as_bool().unwrap_or(true);
                self.markdown_preview.showing = None;
            }
            ("link", _) => {
                if let Value::String(href) = event.value {
                    self.follow_markdown_link(&href);
                }
            }
            _ => {}
        }
    }

    /// Opens a link to another file, relative to the previewed one.
    fn follow_markdown_link(&mut self, href: &str) {
        let href = href.strip_prefix("file://").unwrap_or(href);
        let target = href.split('#').next().unwrap_or("");
        if target.is_empty() {
            return;
        }
        let mut path = PathBuf::from(target);
        if path.is_relative() {
            match self.docs.get(self.active).and_then(|d| d.path.as_deref()) {
                Some(file) => path = file.parent().unwrap_or(Path::new("")).join(path),
                None => {
                    return self.preview_notice(t!("markdown-preview-save-first"));
                }
            }
        }
        if path.exists() {
            self.open_path(&path, true);
        } else {
            self.preview_notice(t!(
                "markdown-preview-missing",
                path = path.display().to_string()
            ));
        }
    }

    fn preview_notice(&mut self, text: String) {
        self.notice = Some(Notice {
            text,
            error: true,
            since: None,
            jump: None,
            open: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_tabs() {
        assert!(is_markdown("Markdown", None));
        assert!(is_markdown("Plain Text", Some(Path::new("/a/README.MD"))));
        assert!(!is_markdown("Plain Text", Some(Path::new("/a/notes.txt"))));
        assert!(!is_markdown("Plain Text", None));
    }

    #[test]
    fn front_matter_is_left_out() {
        assert_eq!(split_front_matter("# Hi\n"), ("# Hi\n", 0));
        assert_eq!(
            split_front_matter("---\ntitle: x\n---\n# Hi\n"),
            ("# Hi\n", 3)
        );
        assert_eq!(split_front_matter("---\nno end\n"), ("---\nno end\n", 0));
        // A rule further down isn't front matter's end unless it's alone.
        assert_eq!(
            split_front_matter("---\na\n----x\n"),
            ("---\na\n----x\n", 0)
        );
    }
}
