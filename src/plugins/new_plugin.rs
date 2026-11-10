//! Plugins > New Plugin…: writes a working plugin to start from, in Python
//! or JavaScript, with a command, a transform or a panel.

use std::path::{Path, PathBuf};

use egui::{RichText, Ui};

use crate::theme::Palette;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    Python,
    JavaScript,
}

/// What the new plugin starts with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Start {
    Command,
    #[default]
    Transform,
    Panel,
}

/// A folder name for the plugin: lower case letters, digits and dashes.
pub fn id_for(name: &str) -> String {
    let mut id = String::new();
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id = id.trim_end_matches('-').to_owned();
    if id.is_empty() {
        "my-plugin".into()
    } else {
        id
    }
}

/// The main file's name.
pub fn main_file(lang: Lang) -> &'static str {
    match lang {
        Lang::Python => "main.py",
        Lang::JavaScript => "main.js",
    }
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into())
}

/// The new plugin's files: (name, contents).
pub fn files(name: &str, lang: Lang, start: Start) -> Vec<(&'static str, String)> {
    let manifest = serde_json::json!({
        "name": name.trim(),
        "version": "0.1.0",
        "description": "",
        "author": "",
        "main": main_file(lang),
    });
    let mut out = vec![(
        "plugin.json",
        serde_json::to_string_pretty(&manifest).unwrap_or_default() + "\n",
    )];
    let title = quoted(name.trim());
    // For comments: nothing that could end one.
    let name = name.trim().replace(['"', '\\', '\n', '\r', '*', '/'], "");
    let main = match (lang, start) {
        (Lang::Python, Start::Command) => format!(
            r#""""{name}: an Ettin Edit plugin. See Plugins > Plugin Guide."""

import ettin


@ettin.command({title})
def hello():
    doc = ettin.active()
    ettin.message("Hello from %s! The tab has %d lines." % ({title}, doc.line_count))


ettin.run()
"#
        ),
        (Lang::Python, Start::Transform) => format!(
            r#""""{name}: an Ettin Edit plugin. See Plugins > Plugin Guide.

The transform appears in Transform > Plugin Transforms, works on each
selection (or the whole tab) and can be a recipe step."""

import ettin


@ettin.transform({title}, options={{"upper": True}})
def run(text, options):
    return text.upper() if options["upper"] else text.lower()


ettin.run()
"#
        ),
        (Lang::Python, Start::Panel) => format!(
            r#""""{name}: an Ettin Edit plugin. See Plugins > Plugin Guide."""

import ettin
from ettin import ui

PANEL = "main"


def render(doc=None):
    doc = doc or ettin.active()
    ettin.show_panel(PANEL, {title}, [
        ui.label(doc.title, weak=True),
        ui.label("%d lines" % doc.line_count),
        ui.button("refresh", "Refresh"),
    ])


@ettin.command("Show " + {title})
def show():
    render()


@ettin.on_panel(PANEL)
def panel(event):
    if event["item"] == "refresh":
        render()


@ettin.on("activated")
def activated(doc):
    render(doc)


ettin.run()
"#
        ),
        (Lang::JavaScript, Start::Command) => format!(
            r#"// {name}: an Ettin Edit plugin. See Plugins > Plugin Guide.
'use strict';

const ettin = require('ettin');

ettin.command({title}, async () => {{
  const doc = await ettin.activeDocument();
  await ettin.message(`Hello from ${{{title}}}! The tab has ${{doc.lineCount}} lines.`);
}});
"#
        ),
        (Lang::JavaScript, Start::Transform) => format!(
            r#"// {name}: an Ettin Edit plugin. See Plugins > Plugin Guide.
//
// The transform appears in Transform > Plugin Transforms, works on each
// selection (or the whole tab) and can be a recipe step.
'use strict';

const ettin = require('ettin');

ettin.transform({title}, {{ upper: true }}, (text, options) =>
  options.upper ? text.toUpperCase() : text.toLowerCase());
"#
        ),
        (Lang::JavaScript, Start::Panel) => format!(
            r#"// {name}: an Ettin Edit plugin. See Plugins > Plugin Guide.
'use strict';

const ettin = require('ettin');
const {{ ui }} = ettin;

const PANEL = 'main';

async function render(doc) {{
  doc = doc || await ettin.activeDocument();
  await ettin.showPanel(PANEL, {{
    title: {title},
    items: [
      ui.label(doc.title, {{ weak: true }}),
      ui.label(`${{doc.lineCount}} lines`),
      ui.button('refresh', 'Refresh'),
    ],
  }});
}}

ettin.command('Show ' + {title}, () => render());
ettin.onPanel(PANEL, (event) => {{
  if (event.item === 'refresh') return render();
}});
ettin.on('activated', render);
"#
        ),
    };
    out.push((main_file(lang), main));
    match lang {
        Lang::Python => out.push(("ettin.pyi", super::SDK_PYTHON_STUBS.to_owned())),
        Lang::JavaScript => out.push(("ettin.d.ts", super::SDK_JS_TYPES.to_owned())),
    }
    out
}

/// Writes the plugin into a new folder in `root`. Returns its id and the
/// main file.
pub fn create(
    root: &Path,
    name: &str,
    lang: Lang,
    start: Start,
) -> Result<(String, PathBuf), String> {
    let id = id_for(name);
    let dir = root.join(&id);
    if dir.exists() {
        return Err(t!("new-plugin-exists", folder = dir.display().to_string()));
    }
    let written = std::fs::create_dir_all(&dir).and_then(|_| {
        files(name, lang, start)
            .iter()
            .try_for_each(|(file, text)| std::fs::write(dir.join(file), text))
    });
    written.map_err(|e| {
        t!(
            "new-plugin-failed",
            folder = dir.display().to_string(),
            error = e.to_string()
        )
    })?;
    Ok((id, dir.join(main_file(lang))))
}

/// The New Plugin dialog.
pub struct NewPluginDialog {
    pub open: bool,
    pub name: String,
    pub lang: Lang,
    pub start: Start,
    pub reload: bool,
    pub error: Option<String>,
}

impl Default for NewPluginDialog {
    fn default() -> Self {
        Self {
            open: false,
            name: String::new(),
            lang: Lang::default(),
            start: Start::default(),
            reload: true,
            error: None,
        }
    }
}

impl NewPluginDialog {
    pub fn show(&mut self) {
        self.open = true;
        self.error = None;
        if self.name.is_empty() {
            self.name = t!("new-plugin-default-name");
        }
    }

    /// Draws the dialog; true when Create was clicked.
    pub fn ui(&mut self, ctx: &egui::Context, root: Option<&Path>, palette: &Palette) -> bool {
        if !self.open {
            return false;
        }
        let mut create = false;
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("new-plugin-dialog")).show(ctx, |ui| {
            ui.set_width(440.0);
            ui.heading(t!("new-plugin-title"));
            ui.add_space(6.0);
            egui::Grid::new("new-plugin-grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(t!("new-plugin-name"));
                    let r = ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(260.0));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        create = true;
                    }
                    ui.end_row();
                    ui.label(t!("new-plugin-language"));
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.lang, Lang::Python, "Python");
                        ui.radio_value(&mut self.lang, Lang::JavaScript, "JavaScript");
                    });
                    ui.end_row();
                    ui.label(t!("new-plugin-starts-with"));
                    ui.vertical(|ui| {
                        ui.radio_value(
                            &mut self.start,
                            Start::Transform,
                            t!("new-plugin-transform"),
                        );
                        ui.radio_value(&mut self.start, Start::Command, t!("new-plugin-command"));
                        ui.radio_value(&mut self.start, Start::Panel, t!("new-plugin-panel"));
                    });
                    ui.end_row();
                });
            ui.add_space(4.0);
            ui.checkbox(&mut self.reload, t!("reload-on-change"))
                .on_hover_text(t!("reload-on-change-hover"));
            ui.add_space(4.0);
            folder_hint(ui, root, &self.name, palette);
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(palette.modified).size(12.0));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ok = !self.name.trim().is_empty() && root.is_some();
                if ui
                    .add_enabled(ok, egui::Button::new(t!("new-plugin-create")))
                    .clicked()
                {
                    create = true;
                }
                if ui.button(t!("cancel")).clicked() {
                    close = true;
                }
            });
        });
        if modal.should_close() || close {
            self.open = false;
        }
        create && !self.name.trim().is_empty() && root.is_some()
    }
}

fn folder_hint(ui: &mut Ui, root: Option<&Path>, name: &str, palette: &Palette) {
    let text = match root {
        Some(root) => t!(
            "new-plugin-folder",
            folder = root.join(id_for(name)).display().to_string()
        ),
        None => t!("no-plugins-folder"),
    };
    ui.label(RichText::new(text).size(11.0).color(palette.text_dim));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names() {
        assert_eq!(id_for("My Plugin!"), "my-plugin");
        assert_eq!(id_for("  CSV  tools 2 "), "csv-tools-2");
        assert_eq!(id_for("***"), "my-plugin");
    }

    #[test]
    fn every_skeleton_has_a_manifest_and_main_file() {
        for lang in [Lang::Python, Lang::JavaScript] {
            for start in [Start::Command, Start::Transform, Start::Panel] {
                let files = files("Say \"Hi\"", lang, start);
                let manifest: serde_json::Value = serde_json::from_str(&files[0].1).unwrap();
                assert_eq!(manifest["name"], "Say \"Hi\"");
                assert_eq!(manifest["main"], main_file(lang));
                assert_eq!(files[1].0, main_file(lang));
                assert!(
                    files[1].1.contains("\"Say \\\"Hi\\\"\""),
                    "{lang:?} {start:?}"
                );
            }
        }
    }
}
