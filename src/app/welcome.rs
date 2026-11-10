//! The Welcome tab: what Ettin Edit is for, the four things to try first,
//! What's new in this version, and a two-step guided tour on sample text.

use egui::{Align, Frame, Layout, Margin, RichText, Sense, Ui};

use super::EttinApp;
use crate::actions::Action;
use crate::tools::convert::DataFormat;
use crate::tools::transform::TransformOp;
use crate::transform_ui::DialogKind;

include!(concat!(env!("OUT_DIR"), "/release_notes.rs"));

/// The version running now.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The text "Try it on sample text" opens: log lines and a JSON blob.
pub const SAMPLE: &str = "\
2026-11-10 09:00:01 ERROR disk full on /var
2026-11-10 09:00:02 INFO  service started
2026-11-10 09:00:05 WARN  low memory
2026-11-10 09:00:09 ERROR timeout talking to the database
2026-11-10 09:00:12 INFO  request done
{\"user\": \"ada\", \"roles\": [\"admin\", \"dev\"], \"active\": true}
";

/// The log lines of [`SAMPLE`] (the first five).
const SAMPLE_LOG_LINES: usize = 5;

/// What the Welcome tab opens on at start, if anything.
#[derive(Debug, PartialEq, Eq)]
pub enum Startup {
    /// A fresh profile: the Welcome tab.
    Welcome,
    /// After an update: the Welcome tab, headed What's new.
    WhatsNew,
    Nothing,
}

/// Decides what to show at start. `shown` is the version the tab was last
/// shown for (empty when never), `existing` whether the settings came from
/// a file (someone who used an older version), and `always` the "Show this
/// tab when Ettin Edit starts" setting.
pub fn startup(shown: &str, existing: bool, always: bool, version: &str) -> Startup {
    if shown.is_empty() {
        if existing {
            Startup::WhatsNew
        } else {
            Startup::Welcome
        }
    } else if shown != version {
        Startup::WhatsNew
    } else if always {
        Startup::Welcome
    } else {
        Startup::Nothing
    }
}

/// The release notes for `version` as Markdown, if the metainfo has them.
pub fn notes_for(version: &str) -> Option<&'static str> {
    RELEASE_NOTES
        .iter()
        .find(|(v, _)| *v == version)
        .map(|(_, notes)| *notes)
}

/// The guided tour on the sample text.
pub struct Tour {
    /// The sample tab.
    pub doc: u64,
    /// 0: Keep Matching Lines; 1: Convert to; 2: done.
    pub step: u8,
    /// The text's version when "Show me" was pressed, to notice the step
    /// was carried out.
    pub version: Option<u64>,
}

/// What the Welcome tab's state is, kept by the app.
#[derive(Default)]
pub struct WelcomeState {
    /// Opened after an update, so it is headed What's new.
    pub whats_new: bool,
    pub notes: crate::manual::MarkdownView,
    pub intro: crate::manual::MarkdownView,
}

/// A card for one of the things to try first.
fn card(
    ui: &mut Ui,
    palette: &crate::theme::Palette,
    title: &str,
    keys: Option<&str>,
    text: &str,
) -> bool {
    let response = Frame::new()
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(6.0)
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(title).strong().color(palette.text));
                if let Some(keys) = keys {
                    ui.label(
                        RichText::new(format!(" {keys} "))
                            .monospace()
                            .size(11.0)
                            .background_color(palette.chrome_bg),
                    );
                }
                ui.label(RichText::new(format!("· {text}")).color(palette.text_dim));
            });
        })
        .response;
    let response = ui.interact(response.rect, response.id.with("card"), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter().rect_stroke(
            response.rect,
            6.0,
            egui::Stroke::new(1.0, palette.accent),
            egui::StrokeKind::Inside,
        );
    }
    response.clicked()
}

impl EttinApp {
    /// Opens the Welcome tab, or selects it if it is open.
    pub(super) fn open_welcome(&mut self, whats_new: bool) {
        self.welcome.whats_new = whats_new;
        if let Some(i) = self.docs.iter().position(|d| d.welcome) {
            self.select(i);
            return;
        }
        let id = self.alloc_id();
        let mut doc = crate::document::Document::new_untitled(id, 0);
        doc.welcome = true;
        doc.editor.buffer.read_only = true;
        self.add_doc(doc);
    }

    /// At start: the Welcome tab on a fresh profile, What's new after an
    /// update, or the Welcome tab when asked for every time.
    pub(super) fn startup_welcome(&mut self) {
        let what = startup(
            &self.settings.welcome_shown_version,
            self.settings.loaded_from_file,
            self.settings.show_welcome,
            VERSION,
        );
        self.settings.welcome_shown_version = VERSION.to_owned();
        match what {
            Startup::Welcome => self.open_welcome(false),
            Startup::WhatsNew => self.open_welcome(true),
            Startup::Nothing => {}
        }
    }

    /// Draws the Welcome tab.
    pub(super) fn welcome_pane(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let ctx = ui.ctx().clone();
        let mut actions = Vec::new();
        let mut try_sample = false;
        let mut page: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("welcome")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::new()
                    .inner_margin(Margin::symmetric(40, 28))
                    .show(ui, |ui| {
                        let wide = ui.available_width() > 820.0;
                        let left_w = if wide {
                            (ui.available_width() * 0.55).min(560.0)
                        } else {
                            ui.available_width()
                        };
                        let mut left = |ui: &mut Ui| {
                            ui.set_width(left_w);
                            let title = if self.welcome.whats_new {
                                t!("welcome-whats-new-title", version = VERSION)
                            } else {
                                t!("welcome-title")
                            };
                            ui.label(RichText::new(title).size(24.0).strong().color(palette.text));
                            ui.add_space(2.0);
                            let intro = t!("welcome-intro");
                            if let Some(link) = self.welcome.intro.ui(ui, &intro, &palette, None) {
                                page = Some(link);
                            }
                            ui.add_space(10.0);
                            ui.spacing_mut().item_spacing.y = 8.0;
                            let keys = |a: &Action| crate::actions::shortcut_label(&ctx, a);
                            if card(
                                ui,
                                &palette,
                                &t!("menu-transform"),
                                None,
                                &t!("welcome-card-transform"),
                            ) {
                                page = Some("transform.md".into());
                            }
                            if card(
                                ui,
                                &palette,
                                &t!("welcome-card-palette-title"),
                                keys(&Action::CommandPalette).as_deref(),
                                &t!("welcome-card-palette"),
                            ) {
                                actions.push(Action::CommandPalette);
                            }
                            if card(
                                ui,
                                &palette,
                                &t!("recipes-title"),
                                keys(&Action::Recipes).as_deref(),
                                &t!("welcome-card-recipes"),
                            ) {
                                actions.push(Action::Recipes);
                            }
                            let pipe = Action::TransformDialog(DialogKind::Pipe);
                            if card(
                                ui,
                                &palette,
                                &t!("pipe-title"),
                                keys(&pipe).as_deref(),
                                &t!("welcome-card-pipe"),
                            ) {
                                actions.push(pipe);
                            }
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let first = egui::Button::new(
                                    RichText::new(t!("welcome-try-sample")).color(egui::Color32::WHITE),
                                )
                                .fill(palette.accent);
                                if ui.add(first).clicked() {
                                    try_sample = true;
                                }
                                if ui.button(t!("welcome-open-manual")).clicked() {
                                    actions.push(Action::UserManual);
                                }
                                if ui.button(t!("welcome-open-file")).clicked() {
                                    actions.push(Action::Open);
                                }
                            });
                            ui.checkbox(&mut self.settings.show_welcome, t!("welcome-show-at-start"));
                        };
                        let mut right = |ui: &mut Ui| {
                            ui.label(
                                RichText::new(t!("welcome-whats-new", version = VERSION))
                                    .strong()
                                    .color(palette.text),
                            );
                            Frame::new()
                                .stroke(egui::Stroke::new(1.0, palette.border))
                                .corner_radius(6.0)
                                .inner_margin(Margin::symmetric(12, 8))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    let notes = notes_for(VERSION).unwrap_or("");
                                    self.welcome.notes.ui(ui, notes, &palette, None);
                                    if ui.link(t!("welcome-full-notes")).clicked() {
                                        let url = format!(
                                            "https://github.com/Tharkis/Ettin-Edit/releases/tag/v{VERSION}"
                                        );
                                        if let Err(e) = crate::system::open_url(&url) {
                                            self.errors.push(e);
                                        }
                                    }
                                });
                            ui.label(
                                RichText::new(t!("welcome-whats-new-hint"))
                                    .size(11.0)
                                    .color(palette.text_dim),
                            );
                        };
                        if wide {
                            ui.horizontal_top(|ui| {
                                ui.vertical(&mut left);
                                ui.add_space(30.0);
                                ui.with_layout(Layout::top_down(Align::Min), |ui| right(ui));
                            });
                        } else {
                            left(ui);
                            ui.add_space(20.0);
                            right(ui);
                        }
                    });
            });
        if try_sample {
            self.start_tour();
        }
        if let Some(link) = page {
            if link.ends_with(".md") {
                self.manual.show(&link, None);
            } else if let Err(e) = crate::system::open_url(&link) {
                self.errors.push(e);
            }
        }
        self.pending.extend(actions);
    }

    /// Opens the sample text in a new tab and starts the guided tour.
    fn start_tour(&mut self) {
        self.new_tab();
        let doc = &mut self.docs[self.active];
        doc.editor = crate::editor::Editor::new(SAMPLE);
        self.tour = Some(Tour {
            doc: doc.id,
            step: 0,
            version: None,
        });
        self.focus_editor = true;
    }

    /// The guided tour's bar above the sample tab: what to try next, and a
    /// Show me button that does it.
    pub(super) fn tour_bar(&mut self, ui: &mut Ui) {
        let Some(tour) = &mut self.tour else {
            return;
        };
        let Some(index) = self.docs.iter().position(|d| d.id == tour.doc) else {
            self.tour = None;
            return;
        };
        // A step is done once the text changed after Show me.
        let version = self.docs[index].editor.buffer.version();
        if tour.version.is_some_and(|v| v != version) && !self.transform_dialog.is_open() {
            tour.step += 1;
            tour.version = None;
        }
        let step = tour.step;
        if index != self.active {
            return;
        }
        let palette = self.palette();
        let text = match step {
            0 => t!("tour-step-keep"),
            1 => t!("tour-step-convert"),
            _ => t!("tour-done"),
        };
        let mut show = false;
        let mut end = false;
        egui::Panel::top("tour-bar")
            .frame(
                Frame::new()
                    .fill(palette.accent.gamma_multiply(0.18))
                    .inner_margin(Margin::symmetric(10, 5)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(egui_phosphor::regular::LIGHTBULB).color(palette.accent),
                    );
                    if step < 2 {
                        ui.label(
                            RichText::new(t!("tour-step-of", step = step + 1, steps = 2)).strong(),
                        );
                    }
                    ui.label(text);
                    if step < 2 && ui.button(t!("tour-show-me")).clicked() {
                        show = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if step < 2 {
                            t!("tour-end")
                        } else {
                            t!("close")
                        };
                        if ui.button(label).clicked() {
                            end = true;
                        }
                    });
                });
            });
        if end {
            self.tour = None;
            return;
        }
        if show {
            self.tour_show_me(index, step, version);
        }
    }

    /// Does the tour's current step: selects the right lines and opens the
    /// dialog or runs the conversion.
    fn tour_show_me(&mut self, index: usize, step: u8, version: u64) {
        let editor = &mut self.docs[index].editor;
        let buf = &editor.buffer;
        let lines = buf.len_lines();
        match step {
            0 => {
                let end = buf.line_start(SAMPLE_LOG_LINES.min(lines) - 1)
                    + buf.line_len(SAMPLE_LOG_LINES.min(lines) - 1);
                editor.select_range(0..end);
                self.open_transform_dialog(DialogKind::FilterLines { keep: true });
                self.transform_dialog.set_pattern("ERROR");
            }
            _ => {
                // The JSON line, wherever the first step left it.
                let json = (0..lines).find(|&l| buf.line_text(l).trim_start().starts_with('{'));
                if let Some(l) = json {
                    let start = buf.line_start(l);
                    let end = start + buf.line_len(l);
                    editor.select_range(start..end);
                    self.pending.push(Action::Transform(TransformOp::ConvertTo {
                        to: DataFormat::Yaml,
                        from: Some(DataFormat::Json),
                    }));
                }
            }
        }
        if let Some(tour) = &mut self.tour {
            tour.version = Some(version);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_shows_once_and_whats_new_once_per_version() {
        // A fresh profile shows Welcome; the next start shows nothing.
        assert_eq!(startup("", false, false, "2.2.0"), Startup::Welcome);
        assert_eq!(startup("2.2.0", true, false, "2.2.0"), Startup::Nothing);
        // An update shows What's new once.
        assert_eq!(startup("2.2.0", true, false, "2.3.0"), Startup::WhatsNew);
        assert_eq!(startup("2.3.0", true, false, "2.3.0"), Startup::Nothing);
        // Someone coming from a version without the Welcome tab.
        assert_eq!(startup("", true, false, "2.2.0"), Startup::WhatsNew);
        // "Show this tab when Ettin Edit starts".
        assert_eq!(startup("2.2.0", true, true, "2.2.0"), Startup::Welcome);
    }

    #[test]
    fn this_version_has_release_notes() {
        let notes = notes_for(VERSION).expect("a <release> for this version in the metainfo");
        assert!(notes.contains("- "), "{notes}");
        assert!(!notes.contains('<'), "{notes}");
        assert!(notes.starts_with(char::is_uppercase), "{notes}");
    }

    #[test]
    fn sample_has_two_error_lines_and_json() {
        let lines: Vec<&str> = SAMPLE.lines().collect();
        let errors: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.contains("ERROR"))
            .map(|(i, _)| i + 1)
            .collect();
        assert_eq!(errors, vec![1, 4]);
        assert!(serde_json::from_str::<serde_json::Value>(lines[SAMPLE_LOG_LINES]).is_ok());
    }
}
