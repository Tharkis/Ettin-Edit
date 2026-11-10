//! Every command with a name, for the shortcut mapper and the command palette.

use super::EttinApp;
use super::menus::MenuModel;
use crate::actions::Action;
use crate::menu::MenuEntry;
use crate::palette_ui::Item;

/// What sort of command it is, for the command palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Menu,
    Recipe,
    Plugin,
}

/// A command and where it is in the menus, like "Edit › Line Operations › Duplicate".
#[derive(Clone)]
pub struct Command {
    pub path: String,
    pub action: Action,
    /// The shortcut a plugin gave its command.
    pub plugin_shortcut: Option<egui::KeyboardShortcut>,
    /// Its name in the command palette, when that isn't the last part of
    /// the path ("Convert to YAML" rather than "YAML").
    pub name: Option<String>,
    /// What the palette shows after the name instead of the menu path,
    /// such as "Recipe · 3 steps".
    pub context: Option<String>,
    pub kind: Kind,
    /// False for a menu item that is greyed out now.
    pub enabled: bool,
}

impl Command {
    fn menu(path: String, action: Action) -> Self {
        Self {
            path,
            action,
            plugin_shortcut: None,
            name: None,
            context: None,
            kind: Kind::Menu,
            enabled: true,
        }
    }

    /// The name the palette shows.
    pub fn palette_name(&self) -> String {
        match &self.name {
            Some(n) => n.clone(),
            None => self
                .path
                .rsplit(" › ")
                .next()
                .unwrap_or(&self.path)
                .to_owned(),
        }
    }

    /// What the palette shows after the name: the menu it is in, or the
    /// context set for it.
    pub fn palette_context(&self) -> String {
        if let Some(c) = &self.context {
            return c.clone();
        }
        match self.path.rsplit_once(" › ") {
            Some((menus, _)) => menus.to_owned(),
            None => String::new(),
        }
    }
}

fn flatten(prefix: &str, entries: &[MenuEntry], m: &MenuModel, out: &mut Vec<Command>) {
    for entry in entries {
        match entry {
            MenuEntry::Item {
                label, id, enabled, ..
            } => {
                if let Some(action) = m.action(*id)
                    && !matches!(action, Action::CompareWith(_))
                {
                    out.push(Command {
                        name: m.name(*id).map(str::to_owned),
                        enabled: *enabled,
                        ..Command::menu(format!("{prefix} › {label}"), action)
                    });
                }
            }
            MenuEntry::Submenu { label, entries } => {
                flatten(&format!("{prefix} › {label}"), entries, m, out)
            }
            MenuEntry::Separator | MenuEntry::Heading(_) | MenuEntry::Busy(_) => {}
        }
    }
}

/// A running plugin's commands: (id, title, shortcut).
pub struct PluginCommands {
    pub id: String,
    pub name: String,
    pub commands: Vec<(String, String, Option<egui::KeyboardShortcut>)>,
}

/// Marks the recipes in `out` (from Transform › Recipes) as recipes, adds
/// any recipe that isn't there, and adds every plugin command.
fn add_recipes_and_plugins(
    out: &mut Vec<Command>,
    recipes: &[crate::recipes::Recipe],
    plugins: &[PluginCommands],
) {
    for r in recipes {
        let action = Action::RunRecipe(r.id.clone());
        match out.iter_mut().find(|c| c.action == action) {
            Some(c) => {
                c.kind = Kind::Recipe;
                c.context = Some(recipe_context(r));
            }
            None => out.push(Command {
                kind: Kind::Recipe,
                context: Some(recipe_context(r)),
                ..Command::menu(
                    format!(
                        "{} › {} › {}",
                        t!("menu-transform"),
                        t!("menu-recipes"),
                        r.name
                    ),
                    action,
                )
            }),
        }
    }
    for p in plugins {
        for (id, title, shortcut) in &p.commands {
            out.push(Command {
                plugin_shortcut: *shortcut,
                name: Some(title.clone()),
                context: Some(plugin_context(&p.name)),
                kind: Kind::Plugin,
                ..Command::menu(
                    format!("{} › {} › {}", t!("menu-plugins"), p.name, title),
                    Action::PluginCommand {
                        plugin: p.id.clone(),
                        command: id.clone(),
                    },
                )
            });
        }
    }
}

/// "Recipe · 3 steps", shown after a recipe's name in the palette.
pub fn recipe_context(recipe: &crate::recipes::Recipe) -> String {
    t!("palette-recipe", steps = recipe.steps.len())
}

/// "Plugin · jsonl-tools", shown after a plugin command's name.
pub fn plugin_context(plugin: &str) -> String {
    t!("palette-plugin", name = plugin.to_owned())
}

/// Commands from the menus drawn directly in the menu bar.
fn fixed_commands() -> Vec<(&'static [&'static str], Action)> {
    use Action::*;
    vec![
        (&["menu-file", "new"], NewTab),
        (&["menu-file", "open-dots"], Open),
        (&["menu-file", "save"], Save),
        (&["menu-file", "save-as"], SaveAs),
        (&["menu-file", "save-all"], SaveAll),
        (&["menu-file", "reload-from-disk"], ReloadFromDisk),
        (&["menu-file", "rename-dots"], RenameFile),
        (&["menu-file", "move-to-trash"], MoveToTrash),
        (
            &["menu-file", "open-containing-folder"],
            OpenContainingFolder,
        ),
        (&["menu-file", "open-terminal-here"], OpenTerminalHere),
        (&["menu-file", "print-dots"], Print),
        (&["menu-file", "export-html"], ExportHtml),
        (
            &["menu-file", "open-folder-as-workspace"],
            OpenFolderAsWorkspace,
        ),
        (&["menu-file", "close-workspace"], CloseWorkspace),
        (&["menu-file", "close"], CloseTab),
        (&["menu-file", "close-all"], CloseAll),
        (&["menu-file", "close-others"], CloseOthers),
        (&["menu-file", "clear-recent-files"], ClearRecent),
        (&["menu-file", "exit"], Quit),
        (&["menu-search", "find-dots"], Find),
        (&["menu-search", "replace-dots"], Replace),
        (&["menu-search", "find-next"], FindNext),
        (&["menu-search", "find-previous"], FindPrev),
        (&["menu-search", "find-selected-next"], FindSelectedNext),
        (&["menu-search", "find-selected-previous"], FindSelectedPrev),
        (&["menu-search", "find-in-files"], FindInFiles),
        (&["menu-search", "find-all-in-document"], FindAllInDoc),
        (&["menu-search", "find-all-in-tabs"], FindAllInTabs),
        (&["menu-search", "select-all-matches"], SelectAllMatches),
        (&["menu-search", "mark-all"], MarkAll { bookmark: false }),
        (&["menu-search", "clear-all-marks"], ClearMarks),
        (&["menu-search", "go-to-line"], GoToLine),
        (
            &["menu-search", "menu-bookmark", "toggle-bookmark"],
            ToggleBookmark,
        ),
        (
            &["menu-search", "menu-bookmark", "next-bookmark"],
            NextBookmark,
        ),
        (
            &["menu-search", "menu-bookmark", "previous-bookmark"],
            PrevBookmark,
        ),
        (
            &["menu-search", "menu-bookmark", "clear-all-bookmarks"],
            ClearBookmarks,
        ),
        (
            &["menu-search", "menu-bookmark", "cut-bookmarked-lines"],
            CutBookmarkedLines,
        ),
        (
            &["menu-search", "menu-bookmark", "copy-bookmarked-lines"],
            CopyBookmarkedLines,
        ),
        (
            &["menu-search", "menu-bookmark", "remove-bookmarked-lines"],
            RemoveBookmarkedLines,
        ),
        (
            &["menu-search", "menu-bookmark", "remove-unbookmarked-lines"],
            RemoveUnbookmarkedLines,
        ),
        (
            &["menu-search", "menu-bookmark", "inverse-bookmarks"],
            InverseBookmarks,
        ),
        (&["menu-view", "word-wrap"], ToggleWordWrap),
        (&["menu-view", "line-numbers"], ToggleLineNumbers),
        (&["menu-view", "highlight-current-line"], ToggleCurrentLine),
        (
            &["menu-view", "menu-show-symbol", "show-whitespace"],
            ToggleWhitespace,
        ),
        (
            &["menu-view", "menu-show-symbol", "show-end-of-line"],
            ToggleEol,
        ),
        (
            &["menu-view", "menu-show-symbol", "show-invisible"],
            ToggleInvisible,
        ),
        (
            &["menu-view", "menu-show-symbol", "show-all-characters"],
            ToggleAllChars,
        ),
        (
            &["menu-view", "menu-show-symbol", "indent-guides"],
            ToggleIndentGuides,
        ),
        (&["menu-view", "long-line-marker"], ToggleEdge),
        (&["menu-view", "function-list"], ToggleFunctionList),
        (&["menu-view", "document-map"], ToggleDocMap),
        (&["menu-view", "character-panel"], ToggleCharPanel),
        (&["menu-view", "hex-view"], ToggleHexView),
        (&["menu-view", "folder-as-workspace"], ToggleFolderPanel),
        (
            &["menu-view", "distraction-free-mode"],
            ToggleDistractionFree,
        ),
        (&["menu-view", "tab-list"], TabList),
        (&["menu-view", "move-to-other-view"], MoveActiveToOtherView),
        (
            &["menu-view", "clone-to-other-view"],
            CloneActiveToOtherView,
        ),
        (&["menu-view", "focus-other-view"], FocusOtherView),
        (&["menu-view", "next-tab"], NextTab),
        (&["menu-view", "previous-tab"], PrevTab),
        (&["menu-view", "menu-folding", "fold-all"], FoldAll),
        (&["menu-view", "menu-folding", "unfold-all"], UnfoldAll),
        (
            &["menu-view", "menu-folding", "fold-current-level"],
            FoldCurrent,
        ),
        (
            &["menu-view", "menu-folding", "unfold-current-level"],
            UnfoldCurrent,
        ),
        (&["menu-view", "zoom-in"], ZoomIn),
        (&["menu-view", "zoom-out"], ZoomOut),
        (&["menu-view", "reset-zoom"], ZoomReset),
        (&["menu-view", "summary-dots"], Summary),
        (&["menu-view", "command-palette"], CommandPalette),
        (&["menu-view", "toolbar-show"], ToggleToolbar),
        (&["menu-view", "toolbar-collapse-menu"], CollapseToolbar),
        (&["menu-view", "menu-bar-show"], ToggleMenuBar),
        (&["menu-view", "customise-toolbar-dots"], CustomiseToolbar),
        (&["menu-theme", "edit-theme-colours"], StyleConfigurator),
        (&["menu-theme", "open-themes-folder"], OpenThemesFolder),
        (&["menu-theme", "reload-themes"], ReloadThemes),
        (&["menu-theme", "restore-built-in-themes"], RestoreThemes),
        (&["menu-language", "new-language"], NewLanguage),
        (
            &["menu-language", "open-languages-folder"],
            OpenLanguagesFolder,
        ),
        (
            &["menu-language", "open-completion-lists-folder"],
            OpenApiFolder,
        ),
        (&["menu-language", "reload-languages"], ReloadLanguages),
        (&["menu-language", "language-guide"], LanguageGuide),
        (&["menu-settings", "preferences-dots"], Preferences),
        (&["menu-settings", "shortcut-mapper-dots"], ShortcutMapper),
        (&["menu-settings", "auto-close-brackets"], ToggleAutoClose),
        (&["menu-settings", "smart-indentation"], ToggleSmartIndent),
        (&["menu-settings", "indent-with-spaces"], ToggleUseSpaces),
        (&["menu-settings", "word-completion"], ToggleCompletion),
        (
            &["menu-settings", "keyword-completion"],
            ToggleLanguageCompletion,
        ),
        (&["menu-settings", "parameter-hints"], ToggleParameterHints),
        (&["menu-settings", "single-window"], ToggleSingleInstance),
        (
            &["menu-settings", "remember-unsaved"],
            ToggleRememberUnsaved,
        ),
        (&["menu-settings", "auto-reload"], ToggleAutoReload),
        (
            &["menu-macro", "start-or-stop-recording"],
            ToggleMacroRecording,
        ),
        (&["menu-macro", "playback"], PlayMacro),
        (&["menu-macro", "run-macro-multiple"], RunMacroMultiple),
        (&["menu-macro", "save-current-macro"], SaveMacro),
        (&["menu-plugins", "manage-plugins"], ManagePlugins),
        (&["menu-plugins", "plugin-log"], ShowPluginLog),
        (&["menu-plugins", "open-plugins-folder"], OpenPluginsFolder),
        (&["menu-plugins", "reload-plugins"], ReloadPlugins),
        (
            &["menu-plugins", "install-sample-plugins"],
            InstallSamplePlugins,
        ),
        (&["menu-plugins", "plugin-guide"], PluginGuide),
        (&["menu-help", "user-manual"], UserManual),
        (&["menu-help", "user-manual-web"], WebManual),
        (&["menu-help", "check-for-updates"], CheckForUpdates),
        (
            &["menu-help", "check-updates-automatically"],
            ToggleUpdateCheck,
        ),
        (&["menu-help", "about-ettin-edit"], About),
    ]
}

impl EttinApp {
    /// Every command, in menu order, with plugin commands and saved macros last.
    pub(super) fn command_catalog(&self, ctx: &egui::Context) -> Vec<Command> {
        let mut out: Vec<Command> = fixed_commands()
            .into_iter()
            .map(|(ids, action)| {
                Command::menu(
                    ids.iter()
                        .map(|id| crate::i18n::tr(id))
                        .collect::<Vec<_>>()
                        .join(" › "),
                    action,
                )
            })
            .collect();
        for level in 1..=5 {
            out.push(Command::menu(
                format!(
                    "{} › {} › {}",
                    t!("menu-view"),
                    t!("menu-folding"),
                    t!("fold-level", level = level)
                ),
                Action::FoldLevel(level),
            ));
        }
        let mut m = MenuModel::new(ctx);
        let edit = self.edit_menu(&mut m, self.active);
        flatten(&t!("menu-edit"), &edit, &m, &mut out);
        let transform = self.transform_menu(&mut m);
        flatten(&t!("menu-transform"), &transform, &m, &mut out);
        let tools = self.tools_menu(&mut m);
        flatten(&t!("menu-tools"), &tools, &m, &mut out);
        let plugins: Vec<PluginCommands> = self
            .plugins
            .plugins
            .iter()
            .filter(|p| p.is_running())
            .map(|p| PluginCommands {
                id: p.id.clone(),
                name: p.name().to_owned(),
                commands: p
                    .commands
                    .iter()
                    .map(|c| (c.id.clone(), c.title.clone(), c.shortcut))
                    .collect(),
            })
            .collect();
        add_recipes_and_plugins(&mut out, &self.recipes, &plugins);
        for (i, mac) in self.settings.macros.iter().enumerate() {
            out.push(Command::menu(
                format!("{} › {}", t!("menu-macro"), mac.name),
                Action::PlaySavedMacro(i),
            ));
        }
        // A command listed twice (Find in two menus) keeps its first place.
        let mut seen = Vec::new();
        out.retain(|c| {
            if seen.contains(&c.action) {
                false
            } else {
                seen.push(c.action.clone());
                true
            }
        });
        out
    }
}

/// How many commands the palette remembers as used recently.
pub const RECENT_COMMANDS: usize = 20;

/// Commands that aren't worth remembering as used recently: moving between
/// tabs, finding the next match, and opening the palette itself.
fn forgettable(action: &Action) -> bool {
    use Action::*;
    matches!(
        action,
        SelectTab(_)
            | NextTab
            | PrevTab
            | CommandPalette
            | TabList
            | FindNext
            | FindPrev
            | OpenPath(_)
            | CompareWith(_)
            | Undo
            | Redo
            | Cut
            | Copy
            | Paste
            | SelectAll
            | FocusOtherView
    )
}

/// `list` with `key` moved to the front, at most [`RECENT_COMMANDS`] long.
fn push_recent(list: &mut Vec<String>, key: String) {
    list.retain(|k| *k != key);
    list.insert(0, key);
    list.truncate(RECENT_COMMANDS);
}

impl EttinApp {
    /// Remembers a command for the palette's "used recently".
    pub(super) fn remember_command(&mut self, action: &Action) {
        if !forgettable(action) {
            push_recent(
                &mut self.settings.recent_commands,
                crate::actions::command_key(action),
            );
        }
    }

    /// The command palette's list: every command with its shortcut, the
    /// recent ones marked, and the ones that can't run now greyed out.
    pub(super) fn palette_items(&self, ctx: &egui::Context) -> Vec<Item<Action>> {
        let all = crate::actions::all_shortcuts();
        let recent = &self.settings.recent_commands;
        self.command_catalog(ctx)
            .into_iter()
            .map(|c| {
                let detail = crate::actions::effective_shortcut(&c.action, c.plugin_shortcut)
                    .or_else(|| all.iter().find(|(_, a)| *a == c.action).map(|(s, _)| *s))
                    .map(|s| ctx.format_shortcut(&s))
                    .unwrap_or_default();
                let key = crate::actions::command_key(&c.action);
                let recent = recent.iter().position(|k| *k == key);
                let enabled = c.enabled && self.command_state(&c.action).0;
                let tag = if !enabled {
                    t!("palette-not-for-tab")
                } else if recent.is_some() {
                    t!("palette-tag-recent")
                } else {
                    match c.kind {
                        Kind::Recipe => t!("palette-tag-recipe"),
                        Kind::Plugin => t!("palette-tag-plugin"),
                        Kind::Menu => String::new(),
                    }
                };
                Item {
                    label: c.palette_name(),
                    context: c.palette_context(),
                    detail,
                    tag,
                    enabled,
                    recent,
                    value: c.action,
                }
            })
            .collect()
    }

    /// The shortcut mapper's rows: every command with its default shortcuts.
    pub(super) fn shortcut_rows(&self, ctx: &egui::Context) -> Vec<crate::shortcut_ui::Row> {
        let defaults = crate::actions::default_shortcuts();
        let fixed = crate::actions::editor_shortcuts();
        self.command_catalog(ctx)
            .into_iter()
            .map(|c| {
                let is_fixed = fixed.iter().any(|(_, a)| *a == c.action);
                let mut list: Vec<egui::KeyboardShortcut> = defaults
                    .iter()
                    .chain(fixed.iter())
                    .filter(|(_, a)| *a == c.action)
                    .map(|(s, _)| *s)
                    .collect();
                list.extend(c.plugin_shortcut);
                crate::shortcut_ui::Row {
                    path: c.path,
                    action: c.action,
                    defaults: list,
                    fixed: is_fixed,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::Recipe;

    #[test]
    fn recent_commands_are_newest_first_and_capped() {
        let mut list = Vec::new();
        for i in 0..25 {
            push_recent(&mut list, format!("c{i}"));
        }
        assert_eq!(list.len(), RECENT_COMMANDS);
        assert_eq!(list[0], "c24");
        push_recent(&mut list, "c10".into());
        assert_eq!(list[0], "c10");
        assert_eq!(list.len(), RECENT_COMMANDS);
        assert!(forgettable(&Action::NextTab));
        assert!(!forgettable(&Action::Json(
            crate::tools::json::JsonOp::Minify
        )));
    }

    /// Every recipe and every plugin command is in the palette, with what
    /// it is after its name.
    #[test]
    fn recipes_and_plugin_commands_are_listed() {
        let mut r = Recipe::new("Clean API dump");
        r.steps = vec![crate::recipes::Step::JoinLines.into(); 3];
        let other = Recipe::new("Log errors only");
        let mut out = vec![Command::menu(
            "Transform › Recipes › Clean API dump".into(),
            Action::RunRecipe(r.id.clone()),
        )];
        let plugins = [PluginCommands {
            id: "jsonl".into(),
            name: "jsonl-tools".into(),
            commands: vec![("explode".into(), "Explode JSON lines".into(), None)],
        }];
        add_recipes_and_plugins(&mut out, &[r.clone(), other.clone()], &plugins);
        assert_eq!(out.len(), 3);
        let recipe = &out[0];
        assert_eq!(recipe.kind, Kind::Recipe);
        assert_eq!(recipe.palette_name(), "Clean API dump");
        assert_eq!(recipe.palette_context(), "Recipe · 3 steps");
        assert_eq!(out[1].action, Action::RunRecipe(other.id.clone()));
        assert_eq!(out[1].palette_context(), "Recipe · 0 steps");
        let plugin = &out[2];
        assert_eq!(plugin.kind, Kind::Plugin);
        assert_eq!(plugin.palette_name(), "Explode JSON lines");
        assert_eq!(plugin.palette_context(), "Plugin · jsonl-tools");
        let menu = Command::menu("Tools › JSON › Format JSON".into(), Action::Save);
        assert_eq!(menu.palette_name(), "Format JSON");
        assert_eq!(menu.palette_context(), "Tools › JSON");
    }
}
