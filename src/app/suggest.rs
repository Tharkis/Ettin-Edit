//! The suggestion bar above a tab: "This JSON is on one line (48 KB).
//! Format · Show as Tree". The detectors are in `crate::suggest`.

use egui::{Align, Frame, Id, Layout, Margin, RichText, Ui};

use super::EttinApp;
use crate::actions::Action;
use crate::document::LineEnding;
use crate::suggest::{self, Facts, Kind};
use crate::tools::json::JsonOp;
use crate::tools::mime::MimeOp;
use crate::tools::transform::TransformOp;
use crate::tools::xml::XmlOp;

/// A suggestion shown for a tab.
pub(crate) struct Suggestion {
    pub kind: Kind,
    /// The text's version when it was made; editing the tab hides it.
    pub version: u64,
    /// The text's size, for the message.
    pub bytes: usize,
}

/// "48 KB", "1.2 MB", "900 bytes".
fn size_text(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{} KB", (bytes as f64 / 1024.0).round())
    } else {
        t!("size-bytes", bytes = bytes)
    }
}

impl EttinApp {
    /// The kinds of suggestion turned off for good.
    fn dismissed(&self) -> Vec<Kind> {
        self.settings
            .dismissed_suggestions
            .iter()
            .filter_map(|k| Kind::from_key(k))
            .collect()
    }

    /// Looks for something to suggest for tab `index`: after it was opened,
    /// or after a big paste.
    pub(super) fn check_suggestions(&mut self, index: usize) {
        if !self.settings.suggestions {
            return;
        }
        let dismissed = self.dismissed();
        let Some(doc) = self.docs.get(index) else {
            return;
        };
        if doc.welcome || doc.binary.is_some() {
            return;
        }
        let rope = doc.editor.buffer.rope();
        let full = rope.len_bytes();
        let end = rope.byte_to_char(full.min(suggest::SCAN_BYTES));
        let text = rope.slice(..end).to_string();
        let facts = Facts {
            language: &doc.syntax,
            mixed_line_endings: doc.mixed_line_endings,
            full_len: full,
        };
        let found = suggest::detect(suggest::scan_part(&text), &facts, &dismissed);
        let (id, version) = (doc.id, doc.editor.buffer.version());
        match found {
            Some(kind) => {
                self.suggestions.insert(
                    id,
                    Suggestion {
                        kind,
                        version,
                        bytes: full,
                    },
                );
            }
            None => {
                self.suggestions.remove(&id);
            }
        }
    }

    /// After a paste of more than 1 KB into the active tab, looks again.
    pub(super) fn check_paste(&mut self) {
        let Some(doc) = self.docs.get_mut(self.active) else {
            return;
        };
        let pasted = std::mem::take(&mut doc.editor.pasted);
        if pasted > suggest::PASTE_BYTES {
            self.check_suggestions(self.active);
        }
    }

    /// The bar above tab `index` in `view`, when it has a suggestion.
    pub(super) fn suggestion_bar(&mut self, ui: &mut Ui, view: usize, index: usize) {
        let doc = &self.docs[index];
        let Some(s) = self.suggestions.get(&doc.id) else {
            return;
        };
        // Editing the tab, or turning suggestions off, puts it away.
        if s.version != doc.editor.buffer.version() || !self.settings.suggestions {
            self.suggestions.remove(&doc.id);
            return;
        }
        let (kind, bytes, id) = (s.kind, s.bytes, doc.id);
        let palette = self.palette();
        let message = match kind {
            Kind::MinifiedJson => t!("suggest-minified-json", size = size_text(bytes)),
            Kind::MinifiedXml => t!("suggest-minified-xml", size = size_text(bytes)),
            Kind::Jwt => t!("suggest-jwt"),
            Kind::Base64 => t!("suggest-base64"),
            Kind::Csv => t!("suggest-csv"),
            Kind::MixedLineEndings => t!("suggest-mixed-line-endings"),
            Kind::MixedIndentation => t!("suggest-mixed-indentation"),
        };
        let mut picked: Vec<Action> = Vec::new();
        let mut close = false;
        let mut never = false;
        egui::Panel::top(Id::new(("suggestion-bar", view)))
            .frame(
                Frame::new()
                    .fill(palette.accent.gamma_multiply(0.14))
                    .inner_margin(Margin::symmetric(10, 4)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(egui_phosphor::regular::LIGHTBULB).color(palette.accent),
                    );
                    ui.label(message);
                    let mut button = |ui: &mut Ui, label: String, action: Action| {
                        if ui.button(label).clicked() {
                            picked.push(action);
                        }
                    };
                    match kind {
                        Kind::MinifiedJson => {
                            button(ui, t!("suggest-format"), Action::Json(JsonOp::Format));
                            if !self.json_tree.open {
                                button(ui, t!("suggest-show-tree"), Action::ToggleJsonTree);
                            }
                        }
                        Kind::MinifiedXml => {
                            button(ui, t!("suggest-format"), Action::Xml(XmlOp::PrettyPrint))
                        }
                        Kind::Jwt => button(
                            ui,
                            t!("suggest-decode"),
                            Action::Transform(TransformOp::JwtDecode),
                        ),
                        Kind::Base64 => {
                            button(ui, t!("suggest-decode"), Action::Mime(MimeOp::Base64Decode))
                        }
                        Kind::Csv => {
                            if !self.csv.open {
                                button(ui, t!("suggest-open-csvquery"), Action::ToggleCsvQuery);
                            }
                            button(ui, t!("suggest-show-table"), Action::RunCsvQuery);
                        }
                        Kind::MixedLineEndings | Kind::MixedIndentation => {
                            ui.menu_button(t!("suggest-fix"), |ui| {
                                if kind == Kind::MixedLineEndings {
                                    if ui.button(t!("suggest-fix-lf")).clicked() {
                                        picked.push(Action::SetLineEnding(LineEnding::Lf));
                                    }
                                    if ui.button(t!("suggest-fix-crlf")).clicked() {
                                        picked.push(Action::SetLineEnding(LineEnding::CrLf));
                                    }
                                } else {
                                    if ui.button(t!("suggest-fix-spaces")).clicked() {
                                        picked.push(Action::TabsToSpaces);
                                    }
                                    if ui.button(t!("suggest-fix-tabs")).clicked() {
                                        picked.push(Action::SpacesToTabs { all: false });
                                    }
                                }
                            });
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if crate::search_ui::icon_button(
                            ui,
                            egui_phosphor::regular::X,
                            &t!("suggest-close"),
                        ) {
                            close = true;
                        }
                        if ui
                            .add(
                                egui::Label::new(
                                    RichText::new(t!("suggest-never"))
                                        .size(12.0)
                                        .color(palette.text_dim),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            never = true;
                        }
                    });
                });
            });
        if never {
            let key = kind.key().to_owned();
            if !self.settings.dismissed_suggestions.contains(&key) {
                self.settings.dismissed_suggestions.push(key);
            }
            // Other tabs with the same suggestion lose it too.
            self.suggestions.retain(|_, s| s.kind != kind);
            self.settings.save();
        }
        if close || !picked.is_empty() {
            self.suggestions.remove(&id);
        }
        if !picked.is_empty() {
            if index != self.active {
                self.focus_view(view, index);
            }
            if kind == Kind::MixedLineEndings {
                // The file is written with one kind of break from now on.
                self.docs[index].mixed_line_endings = false;
            }
            self.pending.extend(picked);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_well() {
        assert_eq!(size_text(48 * 1024 + 100), "48 KB");
        assert_eq!(size_text(3 * 1024 * 1024 / 2), "1.5 MB");
    }
}
