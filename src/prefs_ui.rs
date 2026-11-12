//! The Preferences window (Settings > Preferences): every setting in six
//! sections, a search box that looks through all of them, and Undo Changes
//! / Reset Section. Settings change as they are edited.

use std::sync::mpsc::{Receiver, channel};

/// The width of the Preferences window.
const PREFS_WIDTH: f32 = 760.0;
/// The tallest the Preferences window gets.
const PREFS_HEIGHT: f32 = 500.0;
/// The width of the labels in front of controls.
const LABEL_W: f32 = 190.0;

use egui::{Id, RichText, ScrollArea, Ui};

use crate::document::LineEnding;
use crate::encoding::{TextEncoding, legacy_groups};
use crate::fonts::{EditorFont, SystemFont};
use crate::settings::{FormatIndent, HashResult, LanguageIndent, Settings, ToolbarMode};
use crate::theme::Palette;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Section {
    #[default]
    Appearance,
    Editing,
    Languages,
    Transform,
    Files,
    Plugins,
}

impl Section {
    pub const ALL: [Section; 6] = [
        Section::Appearance,
        Section::Editing,
        Section::Languages,
        Section::Transform,
        Section::Files,
        Section::Plugins,
    ];

    fn label(self) -> String {
        match self {
            Section::Appearance => t!("prefs-appearance"),
            Section::Editing => t!("prefs-editing"),
            Section::Languages => t!("prefs-languages"),
            Section::Transform => t!("menu-transform"),
            Section::Files => t!("prefs-general"),
            Section::Plugins => t!("menu-plugins"),
        }
    }
}

/// Runs `$m!(field, …)` with the settings a section holds.
macro_rules! section_fields {
    ($section:expr, $m:ident) => {
        match $section {
            Section::Appearance => $m!(
                theme,
                follow_system_theme,
                light_theme,
                dark_theme,
                ui_scale,
                ui_language,
                show_toolbar,
                toolbar_mode,
                show_menu_bar,
                markdown_web_images,
                editor_font,
                default_font_size
            ),
            Section::Editing => $m!(
                word_wrap,
                show_line_numbers,
                highlight_current_line,
                show_whitespace,
                show_eol,
                show_invisible,
                indent_guides,
                show_edge,
                edge_column,
                auto_close,
                close_tags,
                smart_indent,
                word_completion,
                language_completion,
                parameter_hints
            ),
            Section::Languages => $m!(tab_width, use_spaces, detect_indent, language_overrides),
            Section::Transform => $m!(
                hash_whole_document,
                recipe_commands_ask,
                shell,
                format_indent,
                suggestions,
                dismissed_suggestions
            ),
            Section::Files => $m!(
                restore_session,
                remember_unsaved,
                single_instance,
                auto_reload,
                xml_check_on_save,
                new_line_ending,
                new_encoding,
                new_language,
                print_line_numbers,
                print_colours,
                check_updates
            ),
            Section::Plugins => $m!(python_path, node_path),
        }
    };
}

/// Copies a section's settings from `from` into `to`.
pub fn copy_section(section: Section, to: &mut Settings, from: &Settings) {
    macro_rules! copy {
        ($($f:ident),*) => {{ $( to.$f = from.$f.clone(); )* }};
    }
    section_fields!(section, copy);
    if section == Section::Appearance {
        to.font_size = to.default_font_size;
    }
}

/// True when two settings agree on everything in `section`.
pub fn same_section(section: Section, a: &Settings, b: &Settings) -> bool {
    macro_rules! same {
        ($($f:ident),*) => { true $( && a.$f == b.$f )* };
    }
    section_fields!(section, same)
}

/// What the dialog asks the app to do.
pub enum PrefsEvent {
    /// Open the shortcut mapper.
    Shortcuts,
    /// Open the theme colour editor.
    Styles,
    /// Open Customise Toolbar.
    Toolbar,
    /// Open the folder of the user's translations.
    TranslationsFolder,
    /// Read the translations again.
    ReloadTranslations,
    ExportSettings,
    ImportSettings,
    SettingsFolder,
}

#[derive(Default)]
pub struct PrefsDialog {
    pub open: bool,
    section: Section,
    search: String,
    focus_search: bool,
    /// The settings when the window opened, for Undo Changes.
    snapshot: Option<Box<Settings>>,
    /// System fonts, read in the background the first time they're needed.
    fonts: Option<Vec<SystemFont>>,
    fonts_rx: Option<Receiver<Vec<SystemFont>>>,
    font_filter: String,
    all_fonts: bool,
    /// The interface languages, read when the dialog opens.
    languages: Vec<crate::i18n::Language>,
    /// The language picked to add a per-language setting for.
    add_language: String,
}

/// Draws one setting at a time, leaving out those the search doesn't match.
struct Form {
    /// The search words, lower case; empty shows everything.
    words: Vec<String>,
    /// A heading to draw before the next setting that is shown.
    heading: Option<String>,
    /// The section, shown in headings while searching.
    section: String,
    shown: usize,
}

impl Form {
    fn new(search: &str, section: Section) -> Self {
        Self {
            words: search
                .split_whitespace()
                .map(|w| w.to_lowercase())
                .collect(),
            heading: None,
            section: section.label(),
            shown: 0,
        }
    }

    fn searching(&self) -> bool {
        !self.words.is_empty()
    }

    /// True when the search finds `label` or one of `keywords`.
    fn matches(&self, label: &str, keywords: &str) -> bool {
        let text = format!("{label} {keywords}").to_lowercase();
        self.words.iter().all(|w| text.contains(w.as_str()))
    }

    fn heading(&mut self, text: String) {
        self.heading = Some(text);
    }

    /// Draws the waiting heading, if any; true when the setting is shown.
    fn show(&mut self, ui: &mut Ui, label: &str, keywords: &str) -> bool {
        if self.searching() && !self.matches(label, keywords) {
            return false;
        }
        if let Some(h) = self.heading.take()
            && (self.searching() || !h.is_empty())
        {
            // An empty heading: no heading, except the section's name while searching.
            let text = match (self.searching(), h.is_empty()) {
                (true, true) => self.section.clone(),
                (true, false) => format!("{} › {h}", self.section),
                _ => h,
            };
            ui.add_space(6.0);
            ui.label(RichText::new(text).strong());
            ui.add_space(2.0);
        }
        self.shown += 1;
        true
    }

    /// A label and the controls that set it, on one row.
    fn row(&mut self, ui: &mut Ui, label: &str, keywords: &str, add: impl FnOnce(&mut Ui)) {
        if self.show(ui, label, keywords) {
            ui.horizontal(|ui| {
                row_label(ui, label);
                add(ui);
            });
        }
    }

    fn check(&mut self, ui: &mut Ui, value: &mut bool, label: &str, keywords: &str) {
        if self.show(ui, label, keywords) {
            ui.checkbox(value, label);
        }
    }
}

/// A setting's name in the label column.
fn row_label(ui: &mut Ui, label: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(LABEL_W, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(LABEL_W);
            ui.add(egui::Label::new(label).truncate());
        },
    );
}

/// The interface sizes offered, in percent.
const SCALES: [u32; 9] = [80, 90, 100, 110, 125, 150, 175, 200, 0];

impl PrefsDialog {
    pub fn show(&mut self, settings: &Settings) {
        if !self.open {
            self.snapshot = Some(Box::new(settings.clone()));
            self.search.clear();
        }
        self.open = true;
        self.focus_search = true;
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
        themes: &[String],
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
        let mut close = false;
        egui::Window::new(t!("preferences"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .id(Id::new("preferences"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            // One size for every section, so the window doesn't jump; at
            // most the size of the main window.
            .fixed_size([
                PREFS_WIDTH,
                (ctx.content_rect().height() - 90.0).clamp(300.0, PREFS_HEIGHT),
            ])
            .show(ctx, |ui| {
                let body = egui::vec2(ui.available_width(), ui.available_height() - 34.0);
                ui.allocate_ui(body, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(170.0);
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut self.search)
                                    .hint_text(t!("prefs-search-hint"))
                                    .desired_width(166.0),
                            );
                            if std::mem::take(&mut self.focus_search) {
                                r.request_focus();
                            }
                            ui.add_space(4.0);
                            let searching = !self.search.trim().is_empty();
                            for section in Section::ALL {
                                if ui
                                    .selectable_label(
                                        !searching && self.section == section,
                                        section.label(),
                                    )
                                    .clicked()
                                {
                                    self.section = section;
                                    self.search.clear();
                                }
                            }
                            ui.add_space(10.0);
                            ui.label(RichText::new(t!("prefs-also")).small().weak());
                            if ui.button(t!("shortcut-mapper-dots")).clicked() {
                                events.push(PrefsEvent::Shortcuts);
                            }
                            if ui.button(t!("theme-colours-dots")).clicked() {
                                events.push(PrefsEvent::Styles);
                            }
                            if ui.button(t!("customise-toolbar-dots")).clicked() {
                                events.push(PrefsEvent::Toolbar);
                            }
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            // Long lines wrap instead of widening the window.
                            ui.set_max_width(ui.available_width());
                            ScrollArea::vertical()
                                .id_salt("prefs-scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    self.sections(
                                        ui,
                                        settings,
                                        languages,
                                        themes,
                                        palette,
                                        &mut events,
                                    )
                                });
                        });
                    })
                });
                ui.separator();
                close = self.footer(ui, settings);
            });
        if !open || close {
            self.open = false;
            self.snapshot = None;
        }
        events
    }

    fn sections(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        languages: &[String],
        themes: &[String],
        palette: &Palette,
        events: &mut Vec<PrefsEvent>,
    ) {
        let search = self.search.trim().to_owned();
        let list: Vec<Section> = if search.is_empty() {
            vec![self.section]
        } else {
            Section::ALL.to_vec()
        };
        let mut shown = 0;
        for section in list {
            let mut form = Form::new(&search, section);
            match section {
                Section::Appearance => {
                    self.appearance(ui, &mut form, settings, themes, palette, events)
                }
                Section::Editing => editing(ui, &mut form, settings),
                Section::Languages => self.languages_section(ui, &mut form, settings, languages),
                Section::Transform => transform(ui, &mut form, settings),
                Section::Files => files(ui, &mut form, settings, languages, events),
                Section::Plugins => plugins(ui, &mut form, settings),
            }
            shown += form.shown;
        }
        if !search.is_empty() {
            // The windows Preferences links to.
            let mut form = Form::new(&search, Section::Appearance);
            form.section = t!("prefs-also");
            form.heading(t!("prefs-other-windows"));
            for (label, keywords, event) in [
                (
                    t!("shortcut-mapper-dots"),
                    "keyboard keys hotkeys keymap vs code visual studio notepad++ shortcuts",
                    PrefsEvent::Shortcuts,
                ),
                (
                    t!("theme-colours-dots"),
                    "colours colors syntax highlighting theme scope",
                    PrefsEvent::Styles,
                ),
                (
                    t!("customise-toolbar-dots"),
                    "ribbon toolbar buttons groups tabs",
                    PrefsEvent::Toolbar,
                ),
            ] {
                if form.show(ui, &label, keywords) && ui.button(&label).clicked() {
                    events.push(event);
                }
            }
            shown += form.shown;
            if shown == 0 {
                ui.add_space(8.0);
                ui.label(RichText::new(t!("prefs-no-match", search = search)).weak());
            }
        }
    }

    /// Undo Changes, Reset Section and Close. True when Close was pressed.
    fn footer(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut close = false;
        let searching = !self.search.trim().is_empty();
        let changed = self.snapshot.as_ref().is_some_and(|s| {
            Section::ALL
                .iter()
                .any(|&sec| !same_section(sec, s, settings))
        });
        let defaults = Settings::default();
        let at_default = same_section(self.section, settings, &defaults);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("changes-apply-live")).small().weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(t!("close")).clicked() {
                    close = true;
                }
                let reset = ui
                    .add_enabled(
                        !searching && !at_default,
                        egui::Button::new(t!("reset-section")),
                    )
                    .on_hover_text(t!("reset-section-tip", section = self.section.label()));
                if reset.clicked() {
                    copy_section(self.section, settings, &defaults);
                }
                let undo = ui
                    .add_enabled(changed, egui::Button::new(t!("undo-changes")))
                    .on_hover_text(t!("undo-changes-tip"));
                if undo.clicked()
                    && let Some(s) = &self.snapshot
                {
                    for sec in Section::ALL {
                        copy_section(sec, settings, s);
                    }
                }
            });
        });
        close
    }

    fn appearance(
        &mut self,
        ui: &mut Ui,
        form: &mut Form,
        settings: &mut Settings,
        themes: &[String],
        palette: &Palette,
        events: &mut Vec<PrefsEvent>,
    ) {
        form.heading(String::new());
        let theme_combo = |ui: &mut Ui, id: &str, value: &mut String| {
            egui::ComboBox::from_id_salt(id)
                .selected_text(value.as_str())
                .width(170.0)
                .height(320.0)
                .show_ui(ui, |ui| {
                    for name in themes {
                        ui.selectable_value(value, name.clone(), name);
                    }
                });
        };
        form.row(
            ui,
            &t!("prefs-theme"),
            &format!(
                "colour color dark light scheme mode night system desktop automatic {}",
                t!("prefs-follow-system")
            ),
            |ui| {
                ui.add_enabled_ui(!settings.follow_system_theme, |ui| {
                    theme_combo(ui, "prefs-theme", &mut settings.theme)
                });
                ui.checkbox(&mut settings.follow_system_theme, t!("prefs-follow-system"));
            },
        );
        form.row(
            ui,
            &t!("prefs-light-dark-pair"),
            "theme follow system dark light mode night",
            |ui| {
                ui.add_enabled_ui(settings.follow_system_theme, |ui| {
                    theme_combo(ui, "prefs-light-theme", &mut settings.light_theme);
                    theme_combo(ui, "prefs-dark-theme", &mut settings.dark_theme);
                });
            },
        );
        form.heading(t!("prefs-interface"));
        form.row(
            ui,
            &t!("prefs-interface-size"),
            "zoom scale bigger smaller dpi menus buttons dialogs",
            |ui| {
                egui::ComboBox::from_id_salt("prefs-scale")
                    .selected_text(format!("{} %", settings.ui_scale))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for pct in SCALES.into_iter().filter(|&p| p > 0) {
                            ui.selectable_value(&mut settings.ui_scale, pct, format!("{pct} %"));
                        }
                    });
                ui.add(
                    egui::DragValue::new(&mut settings.ui_scale)
                        .range(80..=200)
                        .suffix(" %"),
                );
            },
        );
        if form.show(
            ui,
            &t!("prefs-interface-language"),
            "language translation locale english deutsch français español",
        ) {
            self.interface_language(ui, settings, events);
        }
        form.row(
            ui,
            &t!("prefs-toolbar"),
            "ribbon simplified hide hidden",
            |ui| {
                let mut mode = if !settings.show_toolbar {
                    None
                } else {
                    Some(settings.toolbar_mode)
                };
                ui.radio_value(&mut mode, Some(ToolbarMode::Full), t!("prefs-toolbar-full"));
                ui.radio_value(
                    &mut mode,
                    Some(ToolbarMode::Simplified),
                    t!("toolbar-mode-simplified"),
                );
                ui.radio_value(&mut mode, None, t!("prefs-toolbar-hidden"));
                match mode {
                    Some(m) => {
                        settings.show_toolbar = true;
                        settings.toolbar_mode = m;
                    }
                    None => settings.show_toolbar = false,
                }
            },
        );
        form.row(
            ui,
            &t!("prefs-menu-bar"),
            "menus file edit hide alt",
            |ui| {
                ui.checkbox(&mut settings.show_menu_bar, t!("prefs-show"));
            },
        );
        form.row(
            ui,
            &t!("prefs-markdown-preview"),
            "markdown preview images web internet remote privacy http",
            |ui| {
                ui.checkbox(
                    &mut settings.markdown_web_images,
                    t!("prefs-markdown-web-images"),
                )
                .on_hover_text(t!("prefs-markdown-web-images-tip"));
            },
        );
        form.heading(t!("prefs-font"));
        let current = if settings.editor_font.family.is_empty() {
            t!("built-in-font")
        } else {
            settings.editor_font.family.clone()
        };
        form.row(
            ui,
            &t!("editor-font-label"),
            "font typeface monospace",
            |ui| {
                ui.label(RichText::new(&current).color(palette.accent));
                if !settings.editor_font.family.is_empty()
                    && ui.button(t!("use-built-in")).clicked()
                {
                    settings.editor_font = EditorFont::default();
                }
            },
        );
        form.row(
            ui,
            &t!("font-size-label"),
            "font text size zoom bigger",
            |ui| {
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
            },
        );
        if form.searching() {
            return;
        }
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
                                        settings.editor_font = f.to_editor_font();
                                    }
                                }
                            });
                    });
            }
        }
    }

    /// The language picker: follow the system, or one of the languages.
    fn interface_language(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        events: &mut Vec<PrefsEvent>,
    ) {
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
        ui.horizontal(|ui| {
            row_label(ui, &t!("prefs-interface-language"));
            egui::ComboBox::from_id_salt("ui-language")
                .selected_text(selected)
                .width(170.0)
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
                events.push(PrefsEvent::ReloadTranslations);
            }
        });
        ui.label(RichText::new(t!("prefs-translations-hint")).small().weak());
    }

    fn languages_section(
        &mut self,
        ui: &mut Ui,
        form: &mut Form,
        settings: &mut Settings,
        languages: &[String],
    ) {
        form.heading(t!("prefs-indentation"));
        form.row(
            ui,
            &t!("prefs-tab-width"),
            "tab stop width columns indent size",
            |ui| {
                ui.add(egui::DragValue::new(&mut settings.tab_width).range(1..=16));
                ui.label(t!("prefs-columns"));
            },
        );
        form.row(
            ui,
            &t!("prefs-tab-key"),
            "tabs spaces indent with spaces tab key",
            |ui| {
                ui.radio_value(&mut settings.use_spaces, false, t!("prefs-indent-tabs"));
                ui.radio_value(&mut settings.use_spaces, true, t!("prefs-indent-spaces"));
            },
        );
        form.check(
            ui,
            &mut settings.detect_indent,
            &t!("prefs-detect-indent"),
            "detect indentation guess tabs spaces width open",
        );
        form.heading(t!("prefs-per-language"));
        if form.show(
            ui,
            &t!("prefs-per-language"),
            "tab width tabs spaces language json makefile python override indentation",
        ) {
            language_overrides(ui, settings, languages, &mut self.add_language);
        }
    }
}

/// The table of per-language tab widths and tabs/spaces.
fn language_overrides(
    ui: &mut Ui,
    settings: &mut Settings,
    languages: &[String],
    add_language: &mut String,
) {
    ui.label(RichText::new(t!("prefs-per-language-hint")).small().weak());
    let mut remove = None;
    egui::Grid::new("language-overrides")
        .num_columns(4)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            ui.label(RichText::new(t!("menu-language")).small().weak());
            ui.label(RichText::new(t!("prefs-tab-width")).small().weak());
            ui.label(RichText::new(t!("prefs-tab-key")).small().weak());
            ui.end_row();
            for (key, o) in settings.language_overrides.iter_mut() {
                let name = languages
                    .iter()
                    .find(|l| l.to_lowercase() == *key)
                    .cloned()
                    .unwrap_or_else(|| key.clone());
                ui.label(name);
                ui.horizontal(|ui| {
                    let mut set = o.tab_width.is_some();
                    if ui.checkbox(&mut set, "").changed() {
                        o.tab_width = set.then_some(settings.tab_width);
                    }
                    if let Some(w) = &mut o.tab_width {
                        ui.add(egui::DragValue::new(w).range(1..=16));
                    } else {
                        ui.label(RichText::new(t!("prefs-default")).weak());
                    }
                });
                egui::ComboBox::from_id_salt(("lang-tabs", key.as_str()))
                    .selected_text(match o.use_tabs {
                        None => t!("prefs-default"),
                        Some(true) => t!("prefs-tabs"),
                        Some(false) => t!("prefs-spaces"),
                    })
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut o.use_tabs, None, t!("prefs-default"));
                        ui.selectable_value(&mut o.use_tabs, Some(true), t!("prefs-tabs"));
                        ui.selectable_value(&mut o.use_tabs, Some(false), t!("prefs-spaces"));
                    });
                if crate::a11y::name(ui.small_button("×"), &t!("prefs-remove-language")).clicked()
                {
                    remove = Some(key.clone());
                }
                ui.end_row();
            }
        });
    if let Some(key) = remove {
        settings.language_overrides.remove(&key);
    }
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("lang-add")
            .selected_text(if add_language.is_empty() {
                t!("prefs-pick-language")
            } else {
                add_language.clone()
            })
            .width(170.0)
            .height(320.0)
            .show_ui(ui, |ui| {
                for name in languages {
                    if !settings
                        .language_overrides
                        .contains_key(&name.to_lowercase())
                    {
                        ui.selectable_value(add_language, name.clone(), name);
                    }
                }
            });
        if ui
            .add_enabled(!add_language.is_empty(), egui::Button::new(t!("prefs-add")))
            .clicked()
        {
            settings.language_overrides.insert(
                add_language.to_lowercase(),
                LanguageIndent {
                    tab_width: Some(settings.tab_width),
                    use_tabs: Some(!settings.use_spaces),
                },
            );
            add_language.clear();
        }
    });
}

fn editing(ui: &mut Ui, form: &mut Form, settings: &mut Settings) {
    form.heading(t!("prefs-show"));
    form.check(
        ui,
        &mut settings.word_wrap,
        &t!("prefs-word-wrap"),
        "wrap long lines view",
    );
    form.check(
        ui,
        &mut settings.show_line_numbers,
        &t!("prefs-line-numbers"),
        "gutter margin",
    );
    form.check(
        ui,
        &mut settings.highlight_current_line,
        &t!("prefs-highlight-line"),
        "caret cursor",
    );
    form.check(
        ui,
        &mut settings.show_whitespace,
        &t!("prefs-whitespace"),
        "show symbols spaces tabs dots view",
    );
    form.check(
        ui,
        &mut settings.show_eol,
        &t!("prefs-eol"),
        "show symbols line endings crlf lf view",
    );
    form.check(
        ui,
        &mut settings.show_invisible,
        &t!("prefs-invisible"),
        "show symbols zero width control",
    );
    form.check(
        ui,
        &mut settings.indent_guides,
        &t!("prefs-indent-guides"),
        "indentation lines",
    );
    form.row(
        ui,
        &t!("prefs-edge"),
        "edge ruler margin column long line",
        |ui| {
            ui.checkbox(&mut settings.show_edge, "");
            ui.add(egui::DragValue::new(&mut settings.edge_column).range(10..=400));
        },
    );
    form.heading(t!("prefs-typing"));
    form.check(
        ui,
        &mut settings.auto_close,
        &t!("prefs-auto-close"),
        "brackets quotes pairs",
    );
    form.check(
        ui,
        &mut settings.close_tags,
        &t!("prefs-close-tags"),
        "html xml tags",
    );
    form.check(
        ui,
        &mut settings.smart_indent,
        &t!("prefs-smart-indent"),
        "auto indent",
    );
    form.check(
        ui,
        &mut settings.word_completion,
        &t!("prefs-word-completion"),
        "autocomplete completion suggestions",
    );
    form.check(
        ui,
        &mut settings.language_completion,
        &t!("prefs-language-completion"),
        "autocomplete completion keywords",
    );
    form.check(
        ui,
        &mut settings.parameter_hints,
        &t!("prefs-parameter-hints"),
        "function call tips",
    );
}

fn transform(ui: &mut Ui, form: &mut Form, settings: &mut Settings) {
    form.heading(String::new());
    form.row(
        ui,
        &t!("prefs-hash-no-selection"),
        "hash md5 sha checksum whole document clipboard",
        |ui| {
            egui::ComboBox::from_id_salt("prefs-hash")
                .selected_text(match settings.hash_whole_document {
                    HashResult::Clipboard => t!("prefs-hash-clipboard"),
                    HashResult::NewTab => t!("prefs-hash-new-tab"),
                })
                .width(220.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut settings.hash_whole_document,
                        HashResult::Clipboard,
                        t!("prefs-hash-clipboard"),
                    );
                    ui.selectable_value(
                        &mut settings.hash_whole_document,
                        HashResult::NewTab,
                        t!("prefs-hash-new-tab"),
                    );
                });
        },
    );
    form.row(
        ui,
        &t!("prefs-recipe-commands"),
        "recipe shell command steps ask confirm run security",
        |ui| {
            egui::ComboBox::from_id_salt("prefs-recipe-commands")
                .selected_text(if settings.recipe_commands_ask {
                    t!("prefs-ask-first")
                } else {
                    t!("prefs-run-without-asking")
                })
                .width(220.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut settings.recipe_commands_ask,
                        true,
                        t!("prefs-ask-first"),
                    );
                    ui.selectable_value(
                        &mut settings.recipe_commands_ask,
                        false,
                        t!("prefs-run-without-asking"),
                    );
                });
        },
    );
    form.row(
        ui,
        &t!("prefs-shell"),
        "shell bash sh cmd powershell pwsh filter through command pipe",
        |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut settings.shell)
                    .hint_text(if cfg!(windows) { "cmd /C" } else { "sh -c" })
                    .font(egui::TextStyle::Monospace)
                    .desired_width(220.0),
            )
            .on_hover_text(t!("prefs-shell-tip"));
        },
    );
    form.row(
        ui,
        &t!("prefs-format-indent"),
        "json xml format pretty print indent convert spaces tabs",
        |ui| {
            let label = |f: FormatIndent| match f {
                FormatIndent::Document => t!("prefs-indent-document"),
                FormatIndent::Two => t!("prefs-indent-n-spaces", count = 2),
                FormatIndent::Four => t!("prefs-indent-n-spaces", count = 4),
                FormatIndent::Tab => t!("prefs-tabs"),
            };
            egui::ComboBox::from_id_salt("prefs-format-indent")
                .selected_text(label(settings.format_indent))
                .width(220.0)
                .show_ui(ui, |ui| {
                    for f in [
                        FormatIndent::Document,
                        FormatIndent::Two,
                        FormatIndent::Four,
                        FormatIndent::Tab,
                    ] {
                        ui.selectable_value(&mut settings.format_indent, f, label(f));
                    }
                });
        },
    );
    form.heading(t!("prefs-suggestions-heading"));
    form.check(
        ui,
        &mut settings.suggestions,
        &t!("prefs-suggestions"),
        "suggestion bar hints",
    );
    if !settings.dismissed_suggestions.is_empty() {
        let label = t!(
            "prefs-suggestions-reset",
            count = settings.dismissed_suggestions.len()
        );
        if form.show(ui, &label, "suggestion dismissed") && ui.button(&label).clicked() {
            settings.dismissed_suggestions.clear();
        }
    }
}

fn files(
    ui: &mut Ui,
    form: &mut Form,
    settings: &mut Settings,
    languages: &[String],
    events: &mut Vec<PrefsEvent>,
) {
    form.heading(t!("prefs-startup"));
    form.check(
        ui,
        &mut settings.restore_session,
        &t!("prefs-restore-session"),
        "session reopen restore",
    );
    form.check(
        ui,
        &mut settings.remember_unsaved,
        &t!("prefs-remember-unsaved"),
        "unsaved quit exit backup",
    );
    form.check(
        ui,
        &mut settings.single_instance,
        &t!("prefs-single-instance"),
        "single instance window",
    );
    form.heading(t!("prefs-files"));
    form.check(
        ui,
        &mut settings.auto_reload,
        &t!("prefs-auto-reload"),
        "reload changed disk",
    );
    form.check(
        ui,
        &mut settings.xml_check_on_save,
        &t!("prefs-xml-check"),
        "xml well-formed save",
    );
    form.heading(t!("prefs-new-documents"));
    form.row(
        ui,
        &t!("prefs-line-ending"),
        "new document line ending eol crlf lf windows unix",
        |ui| {
            egui::ComboBox::from_id_salt("new-line-ending")
                .selected_text(match settings.new_line_ending {
                    None => t!(
                        "prefs-system-line-ending",
                        ending = LineEnding::platform_default().short_label()
                    ),
                    Some(le) => le.label(),
                })
                .width(220.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut settings.new_line_ending,
                        None,
                        t!(
                            "prefs-system-line-ending",
                            ending = LineEnding::platform_default().short_label()
                        ),
                    );
                    for le in [LineEnding::Lf, LineEnding::CrLf, LineEnding::Cr] {
                        ui.selectable_value(&mut settings.new_line_ending, Some(le), le.label());
                    }
                });
        },
    );
    form.row(
        ui,
        &t!("menu-encoding"),
        "new document encoding utf-8 charset",
        |ui| {
            let current = if settings.new_encoding.is_empty() {
                "UTF-8".to_owned()
            } else {
                settings.new_encoding.clone()
            };
            egui::ComboBox::from_id_salt("new-encoding")
                .selected_text(&current)
                .width(220.0)
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
        },
    );
    form.row(
        ui,
        &t!("menu-language"),
        "new document language syntax plain text",
        |ui| {
            egui::ComboBox::from_id_salt("new-language")
                .selected_text(&settings.new_language)
                .width(220.0)
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
        },
    );
    form.heading(t!("prefs-printing"));
    form.check(
        ui,
        &mut settings.print_line_numbers,
        &t!("prefs-print-line-numbers"),
        "print",
    );
    form.check(
        ui,
        &mut settings.print_colours,
        &t!("prefs-print-colours"),
        "print colors",
    );
    form.heading(t!("prefs-updates"));
    form.check(
        ui,
        &mut settings.check_updates,
        &t!("prefs-check-updates"),
        "update version new release",
    );
    form.heading(t!("prefs-settings-files"));
    if form.show(
        ui,
        &t!("prefs-settings-files"),
        "export import backup move computer sync settings folder",
    ) {
        ui.horizontal(|ui| {
            if ui.button(t!("export-settings-dots")).clicked() {
                events.push(PrefsEvent::ExportSettings);
            }
            if ui.button(t!("import-settings-dots")).clicked() {
                events.push(PrefsEvent::ImportSettings);
            }
            if ui.button(t!("open-settings-folder")).clicked() {
                events.push(PrefsEvent::SettingsFolder);
            }
        });
        ui.label(
            RichText::new(t!("prefs-settings-files-hint"))
                .small()
                .weak(),
        );
    }
}

fn plugins(ui: &mut Ui, form: &mut Form, settings: &mut Settings) {
    form.heading(t!("prefs-plugin-programs"));
    form.row(ui, "Python 3", "plugins python interpreter path", |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut settings.python_path)
                .hint_text("python3")
                .desired_width(300.0),
        );
    });
    form.row(ui, "Node.js", "plugins node javascript path", |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut settings.node_path)
                .hint_text("node")
                .desired_width(300.0),
        );
    });
    if !form.searching() {
        ui.label(RichText::new(t!("prefs-plugin-path-hint")).weak());
        ui.label(RichText::new(t!("prefs-plugin-reload-hint")).weak());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_and_reset_work_on_sections() {
        let opened = Settings::default();
        let mut now = opened.clone();
        now.theme = "Ocean Dark".into();
        now.ui_scale = 150;
        now.tab_width = 8;
        now.shell = "bash -c".into();
        // Not a preference: kept by Undo Changes.
        now.recent_files.push("/tmp/a.txt".into());
        assert!(!same_section(Section::Appearance, &opened, &now));
        assert!(same_section(Section::Editing, &opened, &now));
        for s in Section::ALL {
            copy_section(s, &mut now, &opened);
        }
        assert_eq!(now.theme, "Ettin Dark");
        assert_eq!(now.ui_scale, 100);
        assert_eq!(now.tab_width, 4);
        assert!(now.shell.is_empty());
        assert_eq!(now.recent_files.len(), 1);

        // Reset Section only touches its own section.
        now.tab_width = 2;
        now.language_overrides.clear();
        now.word_wrap = true;
        copy_section(Section::Languages, &mut now, &Settings::default());
        assert_eq!(now.tab_width, 4);
        assert!(now.language_overrides.contains_key("json"));
        assert!(now.word_wrap);
    }

    #[test]
    fn search_matches_labels_and_keywords() {
        let form = Form::new("tab", Section::Languages);
        assert!(form.matches("Tab width:", ""));
        assert!(form.matches("Tab key", "tabs spaces"));
        assert!(form.matches("Per-language settings", "tab width tabs spaces"));
        assert!(!form.matches("Word wrap", "wrap long lines view"));
        let form = Form::new("wrap", Section::Editing);
        assert!(form.matches("Word wrap", "wrap long lines view"));
        // Every word has to match.
        let form = Form::new("tab json", Section::Languages);
        assert!(form.matches("Per-language settings", "tab width json"));
        assert!(!form.matches("Tab width:", ""));
    }

    /// Searching "tab" shows the tab settings; "wrap" finds word wrap
    /// (which used to be only in the View menu).
    #[test]
    fn searching_finds_settings_across_sections() {
        let palette = crate::theme::load_themes(None).0[0].palette;
        let found = |search: &str| -> Vec<String> {
            let ctx = egui::Context::default();
            let mut prefs = PrefsDialog::default();
            let mut settings = Settings::default();
            prefs.show(&settings);
            prefs.search = search.to_owned();
            prefs.fonts = Some(Vec::new());
            let mut draw = |ui: &mut Ui| {
                let mut events = Vec::new();
                prefs.sections(
                    ui,
                    &mut settings,
                    &["JSON".into()],
                    &[],
                    &palette,
                    &mut events,
                );
            };
            let mut output = ctx.run_ui(Default::default(), &mut draw);
            output.textures_delta.clear();
            let mut output = ctx.run_ui(Default::default(), &mut draw);
            output.textures_delta.clear();
            fn texts(shape: &egui::Shape, out: &mut Vec<String>) {
                match shape {
                    egui::Shape::Text(t) => out.push(t.galley.text().to_owned()),
                    egui::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
                    _ => {}
                }
            }
            let mut out = Vec::new();
            for shape in &output.shapes {
                texts(&shape.shape, &mut out);
            }
            out
        };
        let tab = found("tab");
        assert!(tab.iter().any(|t| t == "Tab width:"), "{tab:?}");
        assert!(tab.iter().any(|t| t == "Indent with spaces"), "{tab:?}");
        assert!(tab.iter().any(|t| t.contains("Per-language")), "{tab:?}");
        assert!(!tab.iter().any(|t| t == "Word wrap"), "{tab:?}");
        let wrap = found("wrap");
        assert!(wrap.iter().any(|t| t == "Word wrap"), "{wrap:?}");
    }
}
