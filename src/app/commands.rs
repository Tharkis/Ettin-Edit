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
fn fixed_commands() -> Vec<(&'static str, Action)> {
    use Action::*;
    vec![
        ("File › New", NewTab),
        ("File › Open…", Open),
        ("File › Save", Save),
        ("File › Save As…", SaveAs),
        ("File › Save All", SaveAll),
        ("File › Reload from Disk", ReloadFromDisk),
        ("File › Rename…", RenameFile),
        ("File › Move to Trash…", MoveToTrash),
        ("File › Open Containing Folder", OpenContainingFolder),
        ("File › Open Terminal Here", OpenTerminalHere),
        ("File › Open Folder as Workspace…", OpenFolderAsWorkspace),
        ("File › Close Workspace", CloseWorkspace),
        ("File › Close", CloseTab),
        ("File › Close All", CloseAll),
        ("File › Close Others", CloseOthers),
        ("File › Clear Recent Files", ClearRecent),
        ("File › Exit", Quit),
        ("Search › Find…", Find),
        ("Search › Replace…", Replace),
        ("Search › Find Next", FindNext),
        ("Search › Find Previous", FindPrev),
        ("Search › Find Selected Next", FindSelectedNext),
        ("Search › Find Selected Previous", FindSelectedPrev),
        ("Search › Find in Files…", FindInFiles),
        ("Search › Find All in Current Document", FindAllInDoc),
        ("Search › Find All in Open Tabs", FindAllInTabs),
        ("Search › Select All Matches", SelectAllMatches),
        ("Search › Mark All", MarkAll { bookmark: false }),
        ("Search › Clear All Marks", ClearMarks),
        ("Search › Go to Line…", GoToLine),
        ("Search › Bookmark › Toggle Bookmark", ToggleBookmark),
        ("Search › Bookmark › Next Bookmark", NextBookmark),
        ("Search › Bookmark › Previous Bookmark", PrevBookmark),
        ("Search › Bookmark › Clear All Bookmarks", ClearBookmarks),
        (
            "Search › Bookmark › Cut Bookmarked Lines",
            CutBookmarkedLines,
        ),
        (
            "Search › Bookmark › Copy Bookmarked Lines",
            CopyBookmarkedLines,
        ),
        (
            "Search › Bookmark › Remove Bookmarked Lines",
            RemoveBookmarkedLines,
        ),
        (
            "Search › Bookmark › Remove Unbookmarked Lines",
            RemoveUnbookmarkedLines,
        ),
        ("Search › Bookmark › Inverse Bookmarks", InverseBookmarks),
        ("View › Word Wrap", ToggleWordWrap),
        ("View › Line Numbers", ToggleLineNumbers),
        ("View › Highlight Current Line", ToggleCurrentLine),
        ("View › Show Symbol › Show Whitespace", ToggleWhitespace),
        ("View › Show Symbol › Show End of Line", ToggleEol),
        ("View › Show Symbol › Indent Guides", ToggleIndentGuides),
        ("View › Long-Line Marker", ToggleEdge),
        ("View › Function List", ToggleFunctionList),
        ("View › Document Map", ToggleDocMap),
        ("View › Folder as Workspace", ToggleFolderPanel),
        ("View › Distraction-Free Mode", ToggleDistractionFree),
        ("View › Tab List…", TabList),
        ("View › Move to Other View", MoveActiveToOtherView),
        ("View › Clone to Other View", CloneActiveToOtherView),
        ("View › Focus Other View", FocusOtherView),
        ("View › Next Tab", NextTab),
        ("View › Previous Tab", PrevTab),
        ("View › Folding › Fold All", FoldAll),
        ("View › Folding › Unfold All", UnfoldAll),
        ("View › Folding › Fold Current Level", FoldCurrent),
        ("View › Folding › Unfold Current Level", UnfoldCurrent),
        ("View › Folding › Fold Level 1", FoldLevel(1)),
        ("View › Folding › Fold Level 2", FoldLevel(2)),
        ("View › Folding › Fold Level 3", FoldLevel(3)),
        ("View › Folding › Fold Level 4", FoldLevel(4)),
        ("View › Folding › Fold Level 5", FoldLevel(5)),
        ("View › Zoom In", ZoomIn),
        ("View › Zoom Out", ZoomOut),
        ("View › Reset Zoom", ZoomReset),
        ("View › Summary…", Summary),
        ("View › Command Palette…", CommandPalette),
        ("Theme › Edit Theme Colours…", StyleConfigurator),
        ("Theme › Open Themes Folder", OpenThemesFolder),
        ("Theme › Reload Themes", ReloadThemes),
        ("Theme › Restore Built-in Themes", RestoreThemes),
        ("Settings › Preferences…", Preferences),
        ("Settings › Shortcut Mapper…", ShortcutMapper),
        ("Settings › Auto-Close Brackets and Quotes", ToggleAutoClose),
        ("Settings › Smart Indentation", ToggleSmartIndent),
        ("Settings › Indent With Spaces", ToggleUseSpaces),
        ("Settings › Word Completion", ToggleCompletion),
        ("Settings › Single Window", ToggleSingleInstance),
        (
            "Settings › Remember Unsaved Changes on Exit",
            ToggleRememberUnsaved,
        ),
        (
            "Settings › Reload Files Changed on Disk Automatically",
            ToggleAutoReload,
        ),
        ("Macro › Start or Stop Recording", ToggleMacroRecording),
        ("Macro › Playback", PlayMacro),
        ("Macro › Run a Macro Multiple Times…", RunMacroMultiple),
        ("Macro › Save Current Macro…", SaveMacro),
        ("Plugins › Manage Plugins…", ManagePlugins),
        ("Plugins › Plugin Log", ShowPluginLog),
        ("Plugins › Open Plugins Folder", OpenPluginsFolder),
        ("Plugins › Reload Plugins", ReloadPlugins),
        ("Plugins › Install Sample Plugins", InstallSamplePlugins),
        ("Plugins › Plugin Guide", PluginGuide),
        ("Help › About Ettin Edit", About),
    ]
}

impl EttinApp {
    /// Every command, in menu order, with plugin commands and saved macros last.
    pub(super) fn command_catalog(&self, ctx: &egui::Context) -> Vec<Command> {
        let mut out: Vec<Command> = fixed_commands()
            .into_iter()
            .map(|(path, action)| Command {
                path: path.to_owned(),
                action,
                plugin_shortcut: None,
            })
            .collect();
        let mut m = MenuModel::new(ctx);
        let edit = self.edit_menu(&mut m, self.active);
        flatten("Edit", &edit, &m, &mut out);
        let tools = self.tools_menu(&mut m);
        flatten("Tools", &tools, &m, &mut out);
        for p in self.plugins.plugins.iter().filter(|p| p.is_running()) {
            for c in &p.commands {
                out.push(Command {
                    path: format!("Plugins › {} › {}", p.name(), c.title),
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
                path: format!("Macro › {}", mac.name),
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
