//! The dialog for a plugin's transform: its options as a small form, a live
//! preview of what it makes of the text (run by the plugin itself), and
//! where the result goes. Transforms without options run straight away and
//! skip it.

use egui::{Align, Id, Key, Layout, RichText, Ui, Vec2};
use serde_json::{Map, Value};

use crate::theme::Palette;
use crate::transform_ui::{Dest, PREVIEW_LINES, Source, first_lines, pane};

/// Wait this long after the options or text change before previewing.
const PREVIEW_DELAY: f64 = 0.3;

/// How a text option is shown: line breaks and tabs as `\n` and `\t`.
fn shown(text: &str) -> String {
    text.replace('\n', "\\n").replace('\t', "\\t")
}

/// The text option back from how it is shown.
fn unshown(text: &str) -> String {
    text.replace("\\n", "\n").replace("\\t", "\t")
}

/// A form for a transform's options: a check box for true/false, a box
/// for numbers and text. True when a value changed.
pub fn options_form(ui: &mut Ui, id: Id, options: &mut Map<String, Value>) -> bool {
    let mut changed = false;
    for (key, value) in options.iter_mut() {
        ui.horizontal(|ui| {
            ui.add_sized([140.0, 20.0], egui::Label::new(key.as_str()).truncate());
            match value {
                Value::Bool(b) => changed |= ui.checkbox(b, "").changed(),
                Value::Number(n) => {
                    let mut text = n.to_string();
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut text)
                            .id(id.with(("opt", key.as_str())))
                            .desired_width(100.0),
                    );
                    if r.changed()
                        && let Ok(v) = text.trim().parse::<f64>()
                    {
                        let new = if v.fract() == 0.0 && v.abs() < 9e15 {
                            Value::from(v as i64)
                        } else {
                            serde_json::Number::from_f64(v).map_or(Value::Null, Value::Number)
                        };
                        if !new.is_null() {
                            *value = new;
                            changed = true;
                        }
                    }
                }
                Value::String(s) => {
                    let mut text = shown(s);
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut text)
                            .id(id.with(("opt", key.as_str())))
                            .desired_width(220.0),
                    );
                    if r.changed() {
                        *s = unshown(&text);
                        changed = true;
                    }
                }
                other => {
                    ui.label(RichText::new(other.to_string()).monospace().weak());
                }
            }
        });
    }
    if options.values().any(Value::is_string) {
        ui.label(
            RichText::new(t!("plugin-options-escapes"))
                .size(11.0)
                .weak(),
        );
    }
    changed
}

/// The transform the dialog is for.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub plugin: String,
    pub transform: String,
    pub title: String,
    pub plugin_name: String,
}

/// What the dialog asks the app to do.
pub enum PluginDialogAction {
    /// Run the transform on this text for the preview; tell the dialog the
    /// request's id with [`PluginTransformDialog::previewing`].
    Preview {
        input: String,
        key: u64,
    },
    Run(Map<String, Value>, Dest),
    Stop,
}

#[derive(Default)]
pub struct PluginTransformDialog {
    pub target: Option<Target>,
    pub options: Map<String, Value>,
    defaults: Map<String, Value>,
    pub dest: Dest,
    /// The transform is running on the text now.
    pub running: bool,
    pub error: Option<String>,
    /// What the preview should be for (text and options) and since when.
    wanted: Option<(u64, f64)>,
    /// The preview request waiting for an answer: (request id, key, input).
    pub waiting: Option<(u64, u64, String)>,
    /// The last preview: key, input and output (or the error).
    preview: Option<(u64, String, Result<String, String>)>,
}

impl PluginTransformDialog {
    pub fn show(&mut self, target: Target, defaults: Map<String, Value>, dest: Option<Dest>) {
        if self.target.as_ref() != Some(&target) {
            self.options = defaults.clone();
            self.preview = None;
        }
        self.target = Some(target);
        self.defaults = defaults;
        self.dest = dest.unwrap_or_default();
        self.running = false;
        self.error = None;
        self.wanted = None;
        self.waiting = None;
    }

    pub fn is_open(&self) -> bool {
        self.target.is_some()
    }

    pub fn close(&mut self) {
        self.target = None;
        self.waiting = None;
        self.running = false;
    }

    /// The app sent the preview request `id` for `key`.
    pub fn previewing(&mut self, id: u64, key: u64, input: String) {
        self.waiting = Some((id, key, input));
    }

    /// The plugin answered the preview.
    pub fn preview_done(&mut self, result: Result<String, String>) {
        if let Some((_, key, input)) = self.waiting.take() {
            self.preview = Some((key, input, result));
        }
    }

    /// What the preview is for: the text and the options.
    fn key(&self, source: &Source) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        source.key.hash(&mut h);
        Value::Object(self.options.clone()).to_string().hash(&mut h);
        h.finish()
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        palette: &Palette,
        source: &Source,
    ) -> Option<PluginDialogAction> {
        let target = self.target.clone()?;
        let now = ctx.input(|i| i.time);
        let mut result = None;
        let mut close = false;
        // Preview a moment after the text or the options stop changing.
        let key = self.key(source);
        let have = self.preview.as_ref().is_some_and(|(k, ..)| *k == key)
            || self.waiting.as_ref().is_some_and(|(_, k, _)| *k == key);
        if !have && !self.running {
            match self.wanted {
                Some((k, since)) if k == key => {
                    if now - since >= PREVIEW_DELAY && self.waiting.is_none() {
                        let input = first_lines(&(source.text)(), PREVIEW_LINES).to_owned();
                        result = Some(PluginDialogAction::Preview { input, key });
                    } else {
                        ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                            PREVIEW_DELAY,
                        ));
                    }
                }
                _ => {
                    self.wanted = Some((key, now));
                    ctx.request_repaint_after(std::time::Duration::from_secs_f64(PREVIEW_DELAY));
                }
            }
        }
        if self.waiting.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        let modal = egui::Modal::new(Id::new("plugin-transform-dialog")).show(ctx, |ui| {
            ui.set_width(600.0);
            ui.heading(&target.title);
            ui.label(
                RichText::new(t!(
                    "plugin-transform-from",
                    plugin = target.plugin_name.clone()
                ))
                .size(12.0)
                .color(palette.text_dim),
            );
            ui.add_space(6.0);
            options_form(ui, Id::new("plugin-transform-options"), &mut self.options);
            if ui
                .add_enabled(
                    self.options != self.defaults,
                    egui::Button::new(t!("plugin-options-reset")).small(),
                )
                .clicked()
            {
                self.options = self.defaults.clone();
            }
            ui.add_space(6.0);
            let (input, output) = match &self.preview {
                Some((_, input, out)) => (input.as_str(), Some(out)),
                None => ("", None),
            };
            let input_title = if source.selection {
                t!("pipe-input-selection")
            } else {
                t!("pipe-input-tab")
            };
            let half = (ui.available_width() - 8.0) / 2.0;
            ui.horizontal_top(|ui| {
                ui.allocate_ui(Vec2::new(half, 160.0), |ui| {
                    pane(ui, "plugin-in", &input_title, input, None, palette);
                });
                ui.allocate_ui(Vec2::new(half, 160.0), |ui| {
                    let stale = self.preview.as_ref().is_some_and(|(k, ..)| *k != key);
                    let title = if self.waiting.is_some() {
                        t!("plugin-previewing")
                    } else if stale {
                        t!("pipe-preview-stale")
                    } else {
                        t!("pipe-output")
                    };
                    match output {
                        Some(Ok(out)) => pane(ui, "plugin-out", &title, out, None, palette),
                        Some(Err(e)) => {
                            pane(ui, "plugin-out", &title, e, Some(palette.modified), palette)
                        }
                        None => pane(ui, "plugin-out", &title, "", None, palette),
                    }
                });
            });
            if source.lines > PREVIEW_LINES {
                ui.label(
                    RichText::new(t!("plugin-preview-first-lines", count = PREVIEW_LINES))
                        .size(11.0)
                        .color(palette.text_dim),
                );
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(t!("result-goes-to"));
                ui.radio_value(&mut self.dest, Dest::ThisTab, t!("result-this-tab"));
                ui.radio_value(&mut self.dest, Dest::NewTab, t!("result-new-tab"));
                ui.radio_value(&mut self.dest, Dest::Clipboard, t!("result-clipboard"));
            });
            if let Some(e) = &self.error {
                ui.add_space(4.0);
                ui.label(RichText::new(e).color(palette.modified).size(12.0));
            }
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 26.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if self.running {
                        if ui.button(t!("stop")).clicked() {
                            result = Some(PluginDialogAction::Stop);
                        }
                        ui.spinner();
                        ui.label(
                            RichText::new(t!("pipe-running", command = target.title.clone()))
                                .color(palette.text_dim),
                        );
                        return;
                    }
                    if ui.button(t!("cancel")).clicked() {
                        close = true;
                    }
                    let run =
                        egui::Button::new(t!("pipe-run")).fill(palette.accent.gamma_multiply(0.55));
                    if ui.add(run).clicked() || ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.error = None;
                        result = Some(PluginDialogAction::Run(self.options.clone(), self.dest));
                    }
                },
            );
        });
        if modal.should_close() && !self.running {
            close = true;
        }
        if close {
            self.close();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_options_show_line_breaks_and_tabs() {
        assert_eq!(shown("a\nb\tc"), "a\\nb\\tc");
        assert_eq!(unshown("a\\nb\\tc"), "a\nb\tc");
        for s in ["", "\n", "x\\", "\t\t"] {
            assert_eq!(unshown(&shown(s)), s);
        }
    }

    #[test]
    fn showing_another_transform_starts_from_its_defaults() {
        let mut d = PluginTransformDialog::default();
        let t = |id: &str| Target {
            plugin: "p".into(),
            transform: id.into(),
            title: id.into(),
            plugin_name: "P".into(),
        };
        let mut defaults = Map::new();
        defaults.insert("sep".into(), Value::from("|"));
        d.show(t("a"), defaults.clone(), None);
        d.options.insert("sep".into(), Value::from(","));
        // The same transform again keeps what was typed.
        d.show(t("a"), defaults.clone(), Some(Dest::NewTab));
        assert_eq!(d.options["sep"], ",");
        assert_eq!(d.dest, Dest::NewTab);
        d.show(t("b"), defaults, None);
        assert_eq!(d.options["sep"], "|");
        d.previewing(7, 1, "in".into());
        d.preview_done(Ok("out".into()));
        assert!(d.waiting.is_none());
        d.close();
        assert!(!d.is_open());
    }
}
