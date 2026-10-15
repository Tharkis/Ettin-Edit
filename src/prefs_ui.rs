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

    fn label(self) -> String {
        match self {
            Section::Editor => t!("prefs-editing"),
            Section::Display => t!("prefs-display"),
            Section::NewDocument => t!("prefs-new-documents"),
            Section::General => t!("prefs-general"),
            Section::Plugins => t!("menu-plugins"),
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
    /// Show the interface in the language now in the settings.
    Language,
    /// Open the folder of the user's translations.
    TranslationsFolder,
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
    /// The interface languages, read when the dialog opens.
    languages: Vec<crate::i18n::Language>,
}

fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).strong());
    ui.add_space(2.0);
}

impl PrefsDialog {
    pub fn show(&mut self) {
        self.open = true;
        self.languages = crate::i18n::available();
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
        egui::Window::new(t!("preferences"))
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
                        if ui.button(t!("shortcut-mapper-dots")).clicked() {
                            events.push(PrefsEvent::Shortcuts);
                        }
                        if ui.button(t!("theme-colours-dots")).clicked() {
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
                                Section::General => {
                                    self.interface_language(ui, settings, &mut events);
                                    general(ui, settings)
                                }
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
        heading(ui, &t!("prefs-font"));
        let current = if settings.editor_font.family.is_empty() {
            t!("built-in-font")
        } else {
            settings.editor_font.family.clone()
        };
        ui.horizontal(|ui| {
            ui.label(t!("editor-font-label"));
            ui.label(RichText::new(&current).color(palette.accent));
            if !settings.editor_font.family.is_empty() && ui.button(t!("use-built-in")).clicked() {
                events.push(PrefsEvent::Font(EditorFont::default()));
            }
        });
        ui.horizontal(|ui| {
            ui.label(t!("font-size-label"));
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
            ui.label(RichText::new(t!("font-size-hint")).weak());
        });
        self.load_fonts(ui.ctx());
        ui.horizontal(|ui| {
            ui.label(t!("font-find-label"));
            ui.add(
                egui::TextEdit::singleline(&mut self.font_filter)
                    .hint_text(t!("font-name-hint"))
                    .desired_width(200.0),
            );
            ui.checkbox(&mut self.all_fonts, t!("show-proportional-fonts"));
        });
        match &self.fonts {
            None => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(t!("looking-for-fonts"));
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
                                    ui.label(RichText::new(t!("no-fonts-match")).weak());
                                }
                                for f in shown {
                                    let selected = settings.editor_font.path.as_ref()
                                        == Some(&f.path)
                                        && settings.editor_font.index == f.index;
                                    let mut label = f.family.clone();
                                    if !f.monospace {
                                        label += &format!("  ({})", t!("proportional"));
                                    }
                                    if ui.selectable_label(selected, label).clicked() {
                                        events.push(PrefsEvent::Font(f.to_editor_font()));
                                    }
                                }
                            });
                    });
            }
        }
        heading(ui, &t!("prefs-show"));
        ui.checkbox(&mut settings.word_wrap, t!("prefs-word-wrap"));
        ui.checkbox(&mut settings.show_line_numbers, t!("prefs-line-numbers"));
        ui.checkbox(
            &mut settings.highlight_current_line,
            t!("prefs-highlight-line"),
        );
        ui.checkbox(&mut settings.show_whitespace, t!("prefs-whitespace"));
        ui.checkbox(&mut settings.show_eol, t!("prefs-eol"));
        ui.checkbox(&mut settings.indent_guides, t!("prefs-indent-guides"));
        ui.horizontal(|ui| {
            ui.checkbox(&mut settings.show_edge, t!("prefs-edge"));
            ui.add(egui::DragValue::new(&mut settings.edge_column).range(10..=400));
        });
    }
}

fn editing(ui: &mut Ui, settings: &mut Settings) {
    heading(ui, &t!("prefs-indentation"));
    ui.horizontal(|ui| {
        ui.label(t!("prefs-tab-width"));
        ui.add(egui::DragValue::new(&mut settings.tab_width).range(1..=16));
        ui.label(t!("prefs-columns"));
    });
    ui.radio_value(&mut settings.use_spaces, false, t!("prefs-indent-tabs"));
    ui.radio_value(&mut settings.use_spaces, true, t!("prefs-indent-spaces"));
    ui.checkbox(&mut settings.smart_indent, t!("prefs-smart-indent"));
    heading(ui, &t!("prefs-typing"));
    ui.checkbox(&mut settings.auto_close, t!("prefs-auto-close"));
    ui.checkbox(&mut settings.close_tags, t!("prefs-close-tags"));
    ui.checkbox(&mut settings.word_completion, t!("prefs-word-completion"));
    ui.checkbox(
        &mut settings.language_completion,
        t!("prefs-language-completion"),
    );
    ui.checkbox(&mut settings.parameter_hints, t!("prefs-parameter-hints"));
}

fn new_document(ui: &mut Ui, settings: &mut Settings, languages: &[String]) {
    heading(ui, &t!("prefs-line-ending"));
    ui.radio_value(
        &mut settings.new_line_ending,
        None,
        t!(
            "prefs-system-line-ending",
            ending = LineEnding::platform_default().short_label()
        ),
    );
    for le in [LineEnding::Lf, LineEnding::CrLf, LineEnding::Cr] {
        ui.radio_value(&mut settings.new_line_ending, Some(le), le.label());
    }
    heading(ui, &t!("menu-encoding"));
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
                ui.label(RichText::new(crate::i18n::tr(group)).small().weak());
                for (label, _) in list {
                    if ui.selectable_label(current == label, label).clicked() {
                        settings.new_encoding = label.to_owned();
                    }
                }
            }
        });
    heading(ui, &t!("menu-language"));
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

impl PrefsDialog {
    /// The language picker: follow the system, or one of the languages.
    fn interface_language(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        events: &mut Vec<PrefsEvent>,
    ) {
        heading(ui, &t!("prefs-interface-language"));
        let name_of = |code: &str| {
            self.languages
                .iter()
                .find(|l| l.code == code)
                .map(|l| l.name.clone())
                .unwrap_or_else(|| code.to_owned())
        };
        let system = name_of(&crate::i18n::resolve("", &self.languages));
        let system_label = t!("prefs-system-language", language = system);
        let selected = if settings.ui_language.is_empty() {
            system_label.clone()
        } else {
            name_of(&settings.ui_language)
        };
        let before = settings.ui_language.clone();
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("ui-language")
                .selected_text(selected)
                .width(240.0)
                .height(320.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.ui_language, String::new(), system_label);
                    ui.separator();
                    for l in &self.languages {
                        let mut label = l.name.clone();
                        if l.name != l.code {
                            label += &format!("  ({})", l.code);
                        }
                        ui.selectable_value(&mut settings.ui_language, l.code.clone(), label);
                    }
                });
            if ui.button(t!("open-translations-folder")).clicked() {
                events.push(PrefsEvent::TranslationsFolder);
            }
            if ui.button(t!("reload-translations")).clicked() {
                self.languages = crate::i18n::available();
                events.push(PrefsEvent::Language);
            }
        });
        ui.label(RichText::new(t!("prefs-translations-hint")).weak());
        if settings.ui_language != before {
            events.push(PrefsEvent::Language);
        }
    }
}

fn general(ui: &mut Ui, settings: &mut Settings) {
    heading(ui, &t!("prefs-startup"));
    ui.checkbox(&mut settings.restore_session, t!("prefs-restore-session"));
    ui.checkbox(&mut settings.remember_unsaved, t!("prefs-remember-unsaved"));
    ui.checkbox(&mut settings.single_instance, t!("prefs-single-instance"));
    heading(ui, &t!("prefs-files"));
    ui.checkbox(&mut settings.auto_reload, t!("prefs-auto-reload"));
    ui.checkbox(&mut settings.xml_check_on_save, t!("prefs-xml-check"));
    heading(ui, &t!("prefs-printing"));
    ui.checkbox(
        &mut settings.print_line_numbers,
        t!("prefs-print-line-numbers"),
    );
    ui.checkbox(&mut settings.print_colours, t!("prefs-print-colours"));
    heading(ui, &t!("prefs-updates"));
    ui.checkbox(&mut settings.check_updates, t!("prefs-check-updates"));
}

fn plugins(ui: &mut Ui, settings: &mut Settings) {
    heading(ui, &t!("prefs-plugin-programs"));
    ui.label(RichText::new(t!("prefs-plugin-path-hint")).weak());
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
    ui.label(RichText::new(t!("prefs-plugin-reload-hint")).weak());
}
