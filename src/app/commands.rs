//! Every command with a name, for the shortcut mapper and the command palette.

use super::EttinApp;
use super::menus::MenuModel;
use crate::actions::Action;
use crate::menu::MenuEntry;

/// A command and where it is in the menus, like "Edit › Line Operations › Duplicate".
#[derive(Clone)]
pub struct Command {
    pub path: String,
    pub action: Action,
    /// The shortcut a plugin gave its command.
    pub plugin_shortcut: Option<egui::KeyboardShortcut>,
}

fn flatten(prefix: &str, entries: &[MenuEntry], m: &MenuModel, out: &mut Vec<Command>) {
    for entry in entries {
        match entry {
            MenuEntry::Item { label, id, .. } => {
                if let Some(action) = m.action(*id)
                    && !matches!(action, Action::CompareWith(_))
                {
                    out.push(Command {
                        path: format!("{prefix} › {label}"),
                        action,
                        plugin_shortcut: None,
                    });
                }
            }
            MenuEntry::Submenu { label, entries } => {
                flatten(&format!("{prefix} › {label}"), entries, m, out)
            }
            MenuEntry::Separator => {}
        }
    }
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
            &["menu-view", "menu-show-symbol", "indent-guides"],
            ToggleIndentGuides,
        ),
        (&["menu-view", "long-line-marker"], ToggleEdge),
        (&["menu-view", "function-list"], ToggleFunctionList),
        (&["menu-view", "document-map"], ToggleDocMap),
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
            .map(|(ids, action)| Command {
                path: ids
                    .iter()
                    .map(|id| crate::i18n::tr(id))
                    .collect::<Vec<_>>()
                    .join(" › "),
                action,
                plugin_shortcut: None,
            })
            .collect();
        for level in 1..=5 {
            out.push(Command {
                path: format!(
                    "{} › {} › {}",
                    t!("menu-view"),
                    t!("menu-folding"),
                    t!("fold-level", level = level)
                ),
                action: Action::FoldLevel(level),
                plugin_shortcut: None,
            });
        }
        let mut m = MenuModel::new(ctx);
        let edit = self.edit_menu(&mut m, self.active);
        flatten(&t!("menu-edit"), &edit, &m, &mut out);
        let tools = self.tools_menu(&mut m);
        flatten(&t!("menu-tools"), &tools, &m, &mut out);
        for p in self.plugins.plugins.iter().filter(|p| p.is_running()) {
            for c in &p.commands {
                out.push(Command {
                    path: format!("{} › {} › {}", t!("menu-plugins"), p.name(), c.title),
                    action: Action::PluginCommand {
                        plugin: p.id.clone(),
                        command: c.id.clone(),
                    },
                    plugin_shortcut: c.shortcut,
                });
            }
        }
        for (i, mac) in self.settings.macros.iter().enumerate() {
            out.push(Command {
                path: format!("{} › {}", t!("menu-macro"), mac.name),
                action: Action::PlaySavedMacro(i),
                plugin_shortcut: None,
            });
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

impl EttinApp {
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
