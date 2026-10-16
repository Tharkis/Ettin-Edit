//! The toolbar's link to the rest of the app: which commands it can show,
//! their labels, icons and state, and what its buttons and dialog ask for.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use egui::{Frame, Margin, Ui};

use super::EttinApp;
use crate::actions::{self, Action};
use crate::encoding::{TextEncoding, legacy_groups};
use crate::highlight::PLAIN_TEXT;
use crate::ribbon::{self, Layout};
use crate::ribbon_dialog::{CommandEntry, CustomiseDialog, DialogEvent};
use crate::ribbon_ui::{BarEvent, Button, GroupView, IconImage, RibbonBar, TabView};

/// Seconds the list of commands is kept before it is built again (plugins
/// add commands while they run; the language can change).
const CACHE_SECONDS: f64 = 1.0;

/// The toolbar's state in the app.
pub struct Toolbar {
    pub layout: Layout,
    pub bar: RibbonBar,
    pub dialog: CustomiseDialog,
    /// Every command it can show, by command key, and when it was built.
    cache: Option<(f64, Vec<(Action, CommandEntry)>)>,
    /// Plugin PNG icons by file; `None` for one that couldn't be read.
    images: HashMap<PathBuf, Option<egui::TextureHandle>>,
}

impl Toolbar {
    pub fn load() -> (Self, Option<String>) {
        let (layout, error) = Layout::load();
        (
            Self {
                layout,
                bar: RibbonBar::default(),
                dialog: CustomiseDialog::default(),
                cache: None,
                images: HashMap::new(),
            },
            error,
        )
    }

    /// Builds the command list again on next use.
    pub fn refresh(&mut self) {
        self.cache = None;
    }
}

/// A plugin icon: a Phosphor name, or a PNG file in the plugin's folder.
fn plugin_icon(
    images: &mut HashMap<PathBuf, Option<egui::TextureHandle>>,
    ctx: &egui::Context,
    dir: &Path,
    icon: &str,
) -> Option<IconImage> {
    if !icon.to_ascii_lowercase().ends_with(".png") {
        return ribbon::glyph(icon).map(IconImage::Glyph);
    }
    let path = dir.join(icon);
    images
        .entry(path.clone())
        .or_insert_with(|| {
            let bytes = std::fs::read(&path).ok()?;
            let image = eframe::icon_data::from_png_bytes(&bytes).ok()?;
            let size = [image.width as usize, image.height as usize];
            Some(ctx.load_texture(
                format!("plugin-icon:{}", path.display()),
                egui::ColorImage::from_rgba_unmultiplied(size, &image.rgba),
                egui::TextureOptions::LINEAR,
            ))
        })
        .clone()
        .map(IconImage::Texture)
}

impl EttinApp {
    /// Every command the toolbar can show: the command palette's, plus
    /// themes, languages and encodings from the menus.
    fn toolbar_commands(&mut self, ctx: &egui::Context) -> &[(Action, CommandEntry)] {
        let now = ctx.input(|i| i.time);
        let fresh = self
            .toolbar
            .cache
            .as_ref()
            .is_some_and(|(at, _)| now - at < CACHE_SECONDS && *at <= now);
        if !fresh {
            let list = self.build_toolbar_commands(ctx);
            self.toolbar.cache = Some((now, list));
        }
        &self.toolbar.cache.as_ref().expect("just built").1
    }

    fn build_toolbar_commands(&mut self, ctx: &egui::Context) -> Vec<(Action, CommandEntry)> {
        let mut list: Vec<(Action, String)> = self
            .command_catalog(ctx)
            .into_iter()
            .map(|c| (c.action, c.path))
            .collect();
        let theme = t!("menu-theme");
        for t in &self.themes {
            list.push((
                Action::SetTheme(t.name.clone()),
                format!("{theme} › {}", t.name),
            ));
        }
        let language = t!("menu-language");
        for l in &self.languages {
            let path = if l == PLAIN_TEXT {
                format!("{language} › {l}")
            } else {
                let letter = l.chars().next().unwrap_or(' ').to_ascii_uppercase();
                format!("{language} › {letter} › {l}")
            };
            list.push((Action::SetLanguage(l.clone()), path));
        }
        let encoding = t!("menu-encoding");
        // (menu path below the Encoding menu, encoding name)
        let mut encodings: Vec<(String, String)> = TextEncoding::UNICODE
            .iter()
            .map(|e| (e.name(), e.name()))
            .collect();
        for (group, names) in legacy_groups() {
            for (label, enc) in names {
                encodings.push((
                    format!("{} › {label}", crate::i18n::tr(group)),
                    TextEncoding::Legacy(enc).name(),
                ));
            }
        }
        for (shown, name) in &encodings {
            list.push((
                Action::ConvertEncoding(name.clone()),
                format!("{encoding} › {} › {shown}", t!("encoding-save-as")),
            ));
        }
        for (shown, name) in &encodings {
            list.push((
                Action::ReopenWithEncoding(name.clone()),
                format!("{encoding} › {} › {shown}", t!("reopen-with")),
            ));
        }
        let plugins: Vec<(String, String, PathBuf, Option<String>)> = self
            .plugins
            .plugins
            .iter()
            .flat_map(|p| {
                p.commands
                    .iter()
                    .map(|c| (p.id.clone(), c.id.clone(), p.dir.clone(), c.icon.clone()))
            })
            .collect();
        list.into_iter()
            .map(|(action, path)| {
                let label = ribbon::button_label(path.rsplit(" › ").next().unwrap_or(&path));
                let icon = match &action {
                    Action::PluginCommand { plugin, command } => plugins
                        .iter()
                        .find(|(p, c, _, _)| p == plugin && c == command)
                        .and_then(|(_, _, dir, icon)| {
                            plugin_icon(&mut self.toolbar.images, ctx, dir, icon.as_deref()?)
                        }),
                    _ => None,
                }
                .unwrap_or(IconImage::Glyph(ribbon::default_icon(&action)));
                let entry = CommandEntry {
                    key: actions::command_key(&action),
                    path,
                    label,
                    icon,
                };
                (action, entry)
            })
            .collect()
    }

    /// Whether a command can run now, and whether it is turned on (for
    /// commands that turn something on or off).
    fn command_state(&self, action: &Action) -> (bool, Option<bool>) {
        let s = &self.settings;
        let doc = self.docs.get(self.active);
        let editor = doc.map(|d| &d.editor);
        let saved = doc.is_some_and(|d| d.path.is_some());
        let compare = self.compare.is_some();
        let recording = self.recorder.is_recording();
        let has_macro = !self.recorder.current.is_empty();
        let on = |b: bool| (true, Some(b));
        match action {
            Action::Undo => (editor.is_some_and(|e| e.buffer.can_undo()), None),
            Action::Redo => (editor.is_some_and(|e| e.buffer.can_redo()), None),
            Action::Cut | Action::Copy => {
                (editor.is_some_and(|e| e.selections.has_selection()), None)
            }
            Action::ReloadFromDisk
            | Action::RenameFile
            | Action::MoveToTrash
            | Action::OpenContainingFolder
            | Action::CompareToSaved
            | Action::CompareToGitHead => (saved, None),
            Action::ReopenWithEncoding(_) => (saved, None),
            Action::NextDifference
            | Action::PrevDifference
            | Action::ShowComparison
            | Action::ExportComparePatch
            | Action::CloseComparison => (compare, None),
            Action::CopyCsvResults | Action::CsvResultsToNewTab => {
                (self.csv.result.is_some(), None)
            }
            Action::PlayMacro | Action::RunMacroMultiple | Action::SaveMacro => {
                (!recording && has_macro, None)
            }
            Action::PlaySavedMacro(_) => (!recording, None),
            Action::FocusOtherView => (self.is_split(), None),
            Action::CloseWorkspace => (s.workspace.is_some(), None),
            Action::ToggleMacroRecording => on(recording),
            Action::ToggleWordWrap => on(s.word_wrap),
            Action::ToggleLineNumbers => on(s.show_line_numbers),
            Action::ToggleCurrentLine => on(s.highlight_current_line),
            Action::ToggleWhitespace => on(s.show_whitespace),
            Action::ToggleEol => on(s.show_eol),
            Action::ToggleInvisible => on(s.show_invisible),
            Action::ToggleAllChars => on(self.all_chars_shown()),
            Action::ToggleColumnMode => on(self.column_mode),
            Action::ToggleCharPanel => on(s.char_panel),
            Action::ToggleHexView => on(self.hex_shown()),
            Action::ToggleIndentGuides => on(s.indent_guides),
            Action::ToggleEdge => on(s.show_edge),
            Action::ToggleFunctionList => on(s.function_list),
            Action::ToggleDocMap => on(s.doc_map),
            Action::ToggleFolderPanel => on(s.folder_panel),
            Action::ToggleDistractionFree => on(self.distraction_free),
            Action::ToggleAutoClose => on(s.auto_close),
            Action::ToggleSmartIndent => on(s.smart_indent),
            Action::ToggleUseSpaces => on(s.use_spaces),
            Action::ToggleCompletion => on(s.word_completion),
            Action::ToggleLanguageCompletion => on(s.language_completion),
            Action::ToggleParameterHints => on(s.parameter_hints),
            Action::ToggleSingleInstance => on(s.single_instance),
            Action::ToggleRememberUnsaved => on(s.remember_unsaved),
            Action::ToggleAutoReload => on(s.auto_reload),
            Action::ToggleUpdateCheck => on(s.check_updates),
            Action::ToggleCloseTags => on(s.close_tags),
            Action::ToggleXmlCheckOnSave => on(s.xml_check_on_save),
            Action::ToggleJsonTree => on(self.json_tree.open),
            Action::ToggleCsvQuery => on(self.csv.open),
            Action::ToggleXPathPanel => on(self.xpath.open),
            Action::ToggleReadOnly => on(editor.is_some_and(|e| e.buffer.read_only)),
            Action::ToggleCompareOption(opt) => on(s.compare.get(*opt)),
            Action::ToggleToolbar => on(s.show_toolbar),
            Action::SetTheme(name) => on(self
                .themes
                .get(self.theme_idx)
                .is_some_and(|t| &t.name == name)),
            Action::SetLanguage(name) => on(doc.is_some_and(|d| &d.syntax == name)),
            _ => (true, None),
        }
    }

    /// The visible tabs, with each button's label, icon and state. Buttons
    /// for commands that aren't available (a plugin that isn't running) are
    /// left out.
    fn toolbar_views(&mut self, ctx: &egui::Context) -> Vec<TabView> {
        self.toolbar_commands(ctx);
        let layout = self.toolbar.layout.clone();
        let commands = &self.toolbar.cache.as_ref().expect("built above").1;
        let by_key: HashMap<&str, &(Action, CommandEntry)> =
            commands.iter().map(|c| (c.1.key.as_str(), c)).collect();
        let all_shortcuts = actions::all_shortcuts();
        let plugin_shortcuts = self.plugins.shortcuts();
        let mut views = Vec::new();
        for (t, tab) in layout.tabs.iter().enumerate().filter(|(_, t)| t.visible) {
            let mut groups = Vec::new();
            for (g, group) in tab.groups.iter().enumerate() {
                let mut buttons = Vec::new();
                for (i, item) in group.items.iter().enumerate() {
                    let Some((action, entry)) = by_key.get(item.command.as_str()) else {
                        continue;
                    };
                    let (enabled, checked) = self.command_state(action);
                    let shortcut = actions::effective_shortcut(action, None)
                        .or_else(|| {
                            all_shortcuts
                                .iter()
                                .chain(plugin_shortcuts.iter())
                                .find(|(_, a)| a == action)
                                .map(|(s, _)| *s)
                        })
                        .map(|s| ctx.format_shortcut(&s));
                    let tooltip = match shortcut {
                        Some(s) => format!("{} ({s})", entry.path),
                        None => entry.path.clone(),
                    };
                    let icon = match item.icon.as_deref().and_then(ribbon::glyph) {
                        Some(g) => IconImage::Glyph(g),
                        None => entry.icon.clone(),
                    };
                    buttons.push(Button {
                        action: action.clone(),
                        label: item.label.clone().unwrap_or_else(|| entry.label.clone()),
                        icon,
                        tooltip,
                        enabled,
                        checked,
                        size: item.size,
                        place: (t, g, i),
                    });
                }
                if !buttons.is_empty() {
                    groups.push(GroupView {
                        name: group.name.show(),
                        buttons,
                    });
                }
            }
            views.push(TabView {
                index: t,
                name: tab.name.show(),
                groups,
            });
        }
        views
    }

    /// Draws the toolbar under the menu bar.
    pub(super) fn toolbar_panel(&mut self, ui: &mut Ui) {
        if !self.settings.show_toolbar {
            return;
        }
        let palette = self.palette();
        let ctx = ui.ctx().clone();
        let views = self.toolbar_views(&ctx);
        let collapsed = self.settings.toolbar_collapsed;
        let mut events = Vec::new();
        egui::Panel::top("toolbar")
            .frame(Frame::new().fill(palette.chrome_bg).inner_margin(Margin {
                left: 6,
                right: 6,
                top: 0,
                bottom: if collapsed { 2 } else { 4 },
            }))
            .show_separator_line(false)
            .show(ui, |ui| {
                events = self.toolbar.bar.ui(ui, &views, collapsed, &palette);
            });
        for event in events {
            match event {
                BarEvent::Run(action) => self.pending.push(action),
                BarEvent::Remove(t, g, i) => {
                    self.toolbar.layout.remove(t, g, i);
                    self.save_toolbar();
                }
                BarEvent::Customise => self.pending.push(Action::CustomiseToolbar),
                BarEvent::ToggleCollapsed => self.pending.push(Action::CollapseToolbar),
                BarEvent::Hide => self.pending.push(Action::ToggleToolbar),
            }
        }
    }

    fn save_toolbar(&mut self) {
        if let Err(e) = self.toolbar.layout.save() {
            self.errors.push(e);
        }
    }

    /// The Customise Toolbar dialog, while it is open.
    pub(super) fn toolbar_dialog(&mut self, ctx: &egui::Context) {
        if !self.toolbar.dialog.open {
            return;
        }
        let palette = self.palette();
        self.toolbar_commands(ctx);
        let commands: Vec<CommandEntry> = self
            .toolbar
            .cache
            .as_ref()
            .map(|(_, list)| {
                list.iter()
                    .map(|(_, e)| CommandEntry {
                        key: e.key.clone(),
                        path: e.path.clone(),
                        label: e.label.clone(),
                        icon: e.icon.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        for event in self.toolbar.dialog.ui(ctx, &commands, &palette) {
            match event {
                DialogEvent::Save(layout) => {
                    self.toolbar.layout = layout;
                    self.settings.show_toolbar = true;
                    self.save_toolbar();
                }
            }
        }
    }

    /// Runs the toolbar's own commands.
    pub(super) fn run_toolbar(&mut self, action: &Action) {
        match action {
            Action::CustomiseToolbar => {
                self.toolbar.refresh();
                let layout = self.toolbar.layout.clone();
                self.toolbar.dialog.show(&layout);
            }
            Action::ToggleToolbar => self.settings.show_toolbar = !self.settings.show_toolbar,
            Action::CollapseToolbar => {
                if self.settings.show_toolbar {
                    self.settings.toolbar_collapsed = !self.settings.toolbar_collapsed;
                } else {
                    self.settings.show_toolbar = true;
                    self.settings.toolbar_collapsed = false;
                }
            }
            _ => {}
        }
    }
}
