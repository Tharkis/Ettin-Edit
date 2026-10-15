//! XML Tools > Validate and XSL Transform: finding the schema or stylesheet,
//! running it, and showing what it found.

use std::path::{Path, PathBuf};

use super::EttinApp;
use crate::search::Hit;
use crate::search_ui::Source;
use crate::tools::{Problem, dtd, xsd, xslt};

/// Rows shown in the results panel at most.
const MAX_PROBLEMS: usize = 5000;

fn is_url(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("ftp://")
}

/// A schema or DTD reference from the document, as a file next to it when it exists.
fn local_file(reference: &str, doc_dir: Option<&Path>) -> Option<PathBuf> {
    let reference = reference.strip_prefix("file://").unwrap_or(reference);
    if is_url(reference) {
        return None;
    }
    let path = Path::new(reference);
    let path = match doc_dir {
        Some(dir) if path.is_relative() => dir.join(path),
        _ => path.to_path_buf(),
    };
    path.is_file().then_some(path)
}

fn is_dtd(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("dtd"))
}

/// What to validate against.
enum Schema {
    File(PathBuf),
    /// The document's own DOCTYPE, with only its internal subset.
    InternalDtd,
}

impl EttinApp {
    /// Validates the current document. `choose` asks for the schema even when
    /// one is known.
    pub(super) fn validate_schema(&mut self, ctx: &egui::Context, choose: bool) {
        let doc = &mut self.docs[self.active];
        let text = doc.editor.text_snapshot();
        let doc_path = doc.path.clone();
        let doc_dir = doc_path.as_deref().and_then(Path::parent);
        let key = doc_path.as_ref().map(|p| p.display().to_string());

        let mut schema = None;
        if !choose {
            if let Some(saved) = key.as_ref().and_then(|k| self.settings.xml_schemas.get(k))
                && saved.is_file()
            {
                schema = Some(Schema::File(saved.clone()));
            } else if let Some(found) =
                xsd::schema_location(&text).and_then(|r| local_file(&r, doc_dir))
            {
                schema = Some(Schema::File(found));
            } else if text.contains("<!DOCTYPE") {
                schema = Some(
                    match dtd::doctype_system_id(&text).and_then(|r| local_file(&r, doc_dir)) {
                        Some(file) => Schema::File(file),
                        None => Schema::InternalDtd,
                    },
                );
            }
        }
        let schema = match schema {
            Some(s) => s,
            None => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title(t!("dialog-validate-against"))
                    .add_filter(t!("filter-schema"), &["xsd", "dtd"])
                    .add_filter(t!("filter-all-files"), &["*"]);
                if let Some(dir) = doc_dir {
                    dialog = dialog.set_directory(dir);
                }
                let Some(file) = dialog.pick_file() else {
                    return;
                };
                if let Some(k) = key {
                    self.settings.xml_schemas.insert(k, file.clone());
                }
                Schema::File(file)
            }
        };

        let (name, result) = match &schema {
            Schema::InternalDtd => (t!("the-doctype"), dtd::validate_dtd(&text, None)),
            Schema::File(path) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                let source = match std::fs::read_to_string(path) {
                    Ok(s) => s,
                    Err(e) => {
                        self.tool_error(
                            ctx,
                            &t!("xml-validate"),
                            &t!(
                                "read-failed",
                                path = path.display().to_string(),
                                error = e.to_string()
                            ),
                            None,
                            false,
                        );
                        return;
                    }
                };
                let result = if is_dtd(path) {
                    dtd::validate_dtd(&text, Some(&source))
                } else {
                    xsd::validate_xsd(&text, &source, path.parent())
                };
                (name, result)
            }
        };
        match result {
            Err(e) => self.tool_error(
                ctx,
                &t!("validate-against", schema = name.clone()),
                &e.message,
                None,
                false,
            ),
            Ok(problems) if problems.is_empty() => {
                self.results.open = false;
                self.notify(ctx, t!("valid-against", schema = name), None);
            }
            Ok(problems) => self.show_problems(&name, &problems),
        }
    }

    fn show_problems(&mut self, schema: &str, problems: &[Problem]) {
        let doc = &self.docs[self.active];
        let buffer = &doc.editor.buffer;
        let rope = buffer.rope();
        let hits: Vec<Hit> = problems
            .iter()
            .take(MAX_PROBLEMS)
            .map(|p| {
                let pos = rope.byte_to_char(p.offset.min(rope.len_bytes()));
                let line = buffer.char_to_line(pos);
                Hit {
                    line,
                    col: pos - buffer.line_start(line),
                    len: 0,
                    text: p.message.clone(),
                    highlights: Vec::new(),
                    matches: 1,
                }
            })
            .collect();
        let label = doc.title();
        let id = doc.id;
        self.results.start(t!("validate-against", schema = schema));
        self.results.unit = Some("results-problems");
        self.results.truncated = problems.len() > MAX_PROBLEMS;
        self.results.add_group(Source::Doc(id), label, hits);
    }

    /// Runs a stylesheet over the current document into a new tab.
    pub(super) fn xsl_transform(&mut self, ctx: &egui::Context) {
        let doc = &mut self.docs[self.active];
        let text = doc.editor.text_snapshot();
        let start = self
            .settings
            .last_xsl
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .or_else(|| {
                doc.path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf)
            });
        let mut dialog = rfd::FileDialog::new()
            .set_title(t!("xsl-transform"))
            .add_filter(t!("filter-xsl"), &["xsl", "xslt"])
            .add_filter(t!("filter-all-files"), &["*"]);
        if let Some(dir) = start {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };
        self.settings.last_xsl = Some(path.clone());
        let stylesheet = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                let message = t!(
                    "read-failed",
                    path = path.display().to_string(),
                    error = e.to_string()
                );
                self.tool_error(ctx, &t!("xsl-transform"), &message, None, false);
                return;
            }
        };
        match xslt::transform_with_messages(&text, &stylesheet) {
            Err(e) => self.tool_error(ctx, &t!("xsl-transform"), &e.message, None, false),
            Ok(out) => {
                let head = out
                    .output
                    .trim_start()
                    .get(..200)
                    .unwrap_or(out.output.trim_start());
                let lower = head.to_ascii_lowercase();
                let syntax = if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
                    "HTML"
                } else if lower.starts_with('<') {
                    "XML"
                } else {
                    crate::highlight::PLAIN_TEXT
                };
                self.new_tab();
                let doc = &mut self.docs[self.active];
                doc.editor = crate::editor::Editor::new(&out.output);
                doc.editor.buffer.mark_modified();
                doc.syntax = syntax.into();
                if !out.messages.is_empty() {
                    let text = format!("xsl:message: {}", out.messages.join(" | "));
                    self.notify(ctx, text, None);
                }
            }
        }
    }
}
