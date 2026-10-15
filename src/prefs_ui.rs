//! The Preferences dialog (Settings > Preferences).

use std::sync::mpsc::{Receiver, channel};

use egui::{Id, RichText, ScrollArea, Ui};

use crate::document::LineEnding;
use crate::encoding::{TextEncoding, legacy_groups};
use crate::fonts::{EditorFont, SystemFont};
use crate::settings::Settings;
use crate::theme::Palette;

#[derive(Clone, Copy, Default, PartialEq)]
enum Section {
    #[default]
    Editor,
    Display,
    NewDocument,
    General,
    Plugins,
}

impl Section {
    const ALL: [Section; 5] = [
        Section::Editor,
        Section::Display,
        Section::NewDocument,
        Section::General,
        Section::Plugins,
    ];

    fn label(self) -> &'static str {
        match self {
            Section::Editor => "Editing",
            Section::Display => "Display",
            Section::NewDocument => "New Documents",
            Section::General => "Files and Startup",
            Section::Plugins => "Plugins",
        }
    }
}

/// What the dialog asks the app to do.
pub enum PrefsEvent {
    /// Load this editor font.
    Font(EditorFont),
    /// Open the shortcut mapper.
    Shortcuts,
    /// Open the theme colour editor.
    Styles,
}

#[derive(Default)]
pub struct PrefsDialog {
    pub open: bool,
    section: Section,
    /// System fonts, read in the background the first time they're needed.
    fonts: Option<Vec<SystemFont>>,
    fonts_rx: Option<Receiver<Vec<SystemFont>>>,
    font_filter: String,
    all_fonts: bool,
}

fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).strong());
    ui.add_space(2.0);
}

impl PrefsDialog {
    pub fn show(&mut self) {
        self.open = true;
    }

    fn load_fonts(&mut self, ctx: &egui::Context) {
        if self.fonts.is_some() || self.fonts_rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(crate::fonts::system_fonts());
            ctx.request_repaint();
        });
        self.fonts_rx = Some(rx);
    }

    /// Draws the dialog. Settings change as they're edited.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        settings: &mut Settings,
        languages: &[String],
        palette: &Palette,
    ) -> Vec<PrefsEvent> {
        let mut events = Vec::new();
        if !self.open {
            return events;
        }
        if let Some(rx) = &self.fonts_rx
            && let Ok(fonts) = rx.try_recv()
        {
            self.fonts = Some(fonts);
            self.fonts_rx = None;
        }
        let mut open = true;
        egui::Window::new("Preferences")
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .id(Id::new("preferences"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([640.0, 460.0])
            .min_width(520.0)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(150.0);
                        for section in Section::ALL {
                            if ui
                                .selectable_label(self.section == section, section.label())
                                .clicked()
                            {
                                self.section = section;
                            }
                        }
                        ui.add_space(12.0);
                        if ui.button("Shortcut Mapper…").clicked() {
                            events.push(PrefsEvent::Shortcuts);
                        }
                        if ui.button("Theme Colours…").clicked() {
                            events.push(PrefsEvent::Styles);
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ScrollArea::vertical()
                            .id_salt("prefs-scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.section {
                                Section::Editor => editing(ui, settings),
                                Section::Display => {
                                    self.display(ui, settings, palette, &mut events)
                                }
                                Section::NewDocument => new_document(ui, settings, languages),
                                Section::General => general(ui, settings),
                                Section::Plugins => plugins(ui, settings),
                            });
                    });
                });
            });
        if !open {
            self.open = false;
        }
        events
    }

    fn display(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        palette: &Palette,
        events: &mut Vec<PrefsEvent>,
    ) {
        heading(ui, "Font");
        let current = if settings.editor_font.family.is_empty() {
            "Built-in (Hack)".to_owned()
        } else {
            settings.editor_font.family.clone()
        };
        ui.horizontal(|ui| {
            ui.label("Editor font:");
            ui.label(RichText::new(&current).color(palette.accent));
            if !settings.editor_font.family.is_empty() && ui.button("Use Built-in").clicked() {
                events.push(PrefsEvent::Font(EditorFont::default()));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Size:");
            let mut size = settings.default_font_size;
            if ui
                .add(
                    egui::DragValue::new(&mut size)
                        .range(8.0..=48.0)
                        .speed(0.25),
                )
                .changed()
            {
                settings.default_font_size = size;
                settings.font_size = size;
            }
            ui.label(RichText::new("Ctrl+0 returns to this size after zooming").weak());
        });
        self.load_fonts(ui.ctx());
        ui.horizontal(|ui| {
            ui.label("Find:");
            ui.add(
                egui::TextEdit::singleline(&mut self.font_filter)
                    .hint_text("Font name")
                    .desired_width(200.0),
            );
            ui.checkbox(&mut self.all_fonts, "Show proportional fonts");
        });
        match &self.fonts {
            None => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Looking for installed fonts…");
                });
            }
            Some(fonts) => {
                let filter = self.font_filter.to_lowercase();
                let shown: Vec<&SystemFont> = fonts
                    .iter()
                    .filter(|f| self.all_fonts || f.monospace)
                    .filter(|f| filter.is_empty() || f.family.to_lowercase().contains(&filter))
                    .collect();
                egui::Frame::new()
                    .fill(palette.editor_bg)
                    .stroke(egui::Stroke::new(1.0, palette.border))
                    .corner_radius(4)
                    .inner_margin(4)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("font-list")
                            .max_height(150.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                if shown.is_empty() {
                                    ui.label(RichText::new("No fonts match").weak());
                                }
                                for f in shown {
                                    let selected = settings.editor_font.path.as_ref()
                                        == Some(&f.path)
                                        && settings.editor_font.index == f.index;
                                    let mut label = f.family.clone();
                                    if !f.monospace {
                                        label += "  (proportional)";
                                    }
                                    if ui.selectable_label(selected, label).clicked() {
                                        events.push(PrefsEvent::Font(f.to_editor_font()));
                                    }
                                }
                            });
                    });
            }
        }
        heading(ui, "Show");
        ui.checkbox(&mut settings.word_wrap, "Word wrap");
        ui.checkbox(&mut settings.show_line_numbers, "Line numbers");
        ui.checkbox(
            &mut settings.highlight_current_line,
            "Highlight the current line",
        );
        ui.checkbox(&mut settings.show_whitespace, "Whitespace symbols");
        ui.checkbox(&mut settings.show_eol, "End-of-line symbols");
        ui.checkbox(&mut settings.indent_guides, "Indent guides");
        ui.horizontal(|ui| {
            ui.checkbox(&mut settings.show_edge, "Long-line marker at column");
            ui.add(egui::DragValue::new(&mut settings.edge_column).range(10..=400));
        });
    }
}

fn editing(ui: &mut Ui, settings: &mut Settings) {
    heading(ui, "Indentation");
    ui.horizontal(|ui| {
        ui.label("Tab width:");
        ui.add(egui::DragValue::new(&mut settings.tab_width).range(1..=16));
        ui.label("columns");
    });
    ui.radio_value(&mut settings.use_spaces, false, "Indent with tabs");
    ui.radio_value(&mut settings.use_spaces, true, "Indent with spaces");
    ui.checkbox(
        &mut settings.smart_indent,
        "Smart indentation (keep the indent, and indent after an opening bracket)",
    );
    heading(ui, "Typing");
    ui.checkbox(
        &mut settings.auto_close,
        "Close brackets and quotes automatically",
    );
    ui.checkbox(
        &mut settings.close_tags,
        "Close HTML and XML tags automatically",
    );
    ui.checkbox(
        &mut settings.word_completion,
        "Suggest words from the document while typing",
    );
    ui.checkbox(
        &mut settings.language_completion,
        "Suggest the language's keywords and functions",
    );
    ui.checkbox(
        &mut settings.parameter_hints,
        "Show a function's parameters while typing a call",
    );
}

fn new_document(ui: &mut Ui, settings: &mut Settings, languages: &[String]) {
    heading(ui, "Line ending");
    ui.radio_value(
        &mut settings.new_line_ending,
        None,
        format!(
            "This system's default ({})",
            LineEnding::platform_default().short_label()
        ),
    );
    for le in [LineEnding::Lf, LineEnding::CrLf, LineEnding::Cr] {
        ui.radio_value(&mut settings.new_line_ending, Some(le), le.label());
    }
    heading(ui, "Encoding");
    let current = if settings.new_encoding.is_empty() {
        "UTF-8".to_owned()
    } else {
        settings.new_encoding.clone()
    };
    egui::ComboBox::from_id_salt("new-encoding")
        .selected_text(&current)
        .width(240.0)
        .height(320.0)
        .show_ui(ui, |ui| {
            for enc in TextEncoding::UNICODE {
                let name = enc.name();
                if ui.selectable_label(current == name, &name).clicked() {
                    settings.new_encoding = name;
                }
            }
            for (group, list) in legacy_groups() {
                ui.label(RichText::new(group).small().weak());
                for (label, _) in list {
                    if ui.selectable_label(current == label, label).clicked() {
                        settings.new_encoding = label.to_owned();
                    }
                }
            }
        });
    heading(ui, "Language");
    egui::ComboBox::from_id_salt("new-language")
        .selected_text(&settings.new_language)
        .width(240.0)
        .height(320.0)
        .show_ui(ui, |ui| {
            for name in languages {
                if ui
                    .selectable_label(settings.new_language == *name, name)
                    .clicked()
                {
                    settings.new_language = name.clone();
                }
            }
        });
}

fn general(ui: &mut Ui, settings: &mut Settings) {
    heading(ui, "Startup and exit");
    ui.checkbox(
        &mut settings.restore_session,
        "Reopen the files that were open last time",
    );
    ui.checkbox(
        &mut settings.remember_unsaved,
        "Quit without asking about unsaved changes, and bring them back next time",
    );
    ui.checkbox(
        &mut settings.single_instance,
        "Open files in the running Ettin Edit window",
    );
    heading(ui, "Files");
    ui.checkbox(
        &mut settings.auto_reload,
        "Reload files changed on disk automatically when they have no unsaved changes",
    );
    ui.checkbox(
        &mut settings.xml_check_on_save,
        "Check that XML is well-formed each time it's saved",
    );
    heading(ui, "Printing");
    ui.checkbox(&mut settings.print_line_numbers, "Print line numbers");
    ui.checkbox(
        &mut settings.print_colours,
        "Print syntax colours (darkened to read well on paper)",
    );
    heading(ui, "Updates");
    ui.checkbox(
        &mut settings.check_updates,
        "Check for a new version of Ettin Edit once a week",
    );
}

fn plugins(ui: &mut Ui, settings: &mut Settings) {
    heading(ui, "Programs that run plugins");
    ui.label(RichText::new("Leave empty to find them on the PATH.").weak());
    egui::Grid::new("plugin-paths")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Python 3:");
            ui.add(
                egui::TextEdit::singleline(&mut settings.python_path)
                    .hint_text("python3")
                    .desired_width(300.0),
            );
            ui.end_row();
            ui.label("Node.js:");
            ui.add(
                egui::TextEdit::singleline(&mut settings.node_path)
                    .hint_text("node")
                    .desired_width(300.0),
            );
            ui.end_row();
        });
    ui.label(RichText::new("Changes apply when plugins are reloaded.").weak());
}
