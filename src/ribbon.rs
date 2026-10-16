//! The toolbar: tabs of grouped command buttons above the document tabs,
//! like an Office ribbon. Users choose which commands it shows (any menu
//! command, plugin commands and saved macros); the layout is kept in
//! `toolbar.json` in the config folder, and the defaults are used while that
//! file doesn't exist.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use egui_phosphor::regular as ph;
use serde::{Deserialize, Serialize};

use crate::actions::{self, Action};
use crate::editor::lines::{CaseKind, SortKind, TrimKind};
use crate::tools::json::JsonOp;
use crate::tools::mime::MimeOp;
use crate::tools::xml::XmlOp;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    /// A tall button with the icon above the label.
    Large,
    /// A short button with the icon beside the label; three fit in a column.
    #[default]
    Small,
}

/// A button on the toolbar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// The command, as [`actions::command_key`] writes it.
    pub command: String,
    /// The label to show instead of the command's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// A Phosphor icon name (like `star`) to show instead of the command's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub size: Size,
}

impl Item {
    pub fn new(action: &Action, size: Size) -> Self {
        Self {
            command: actions::command_key(action),
            label: None,
            icon: None,
            size,
        }
    }

    #[cfg(test)]
    pub fn action(&self) -> Option<Action> {
        serde_json::from_str(&self.command).ok()
    }
}

/// A tab or group name: a built-in one follows the interface language until
/// the user renames it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Name {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// The translation id of a built-in name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Name {
    pub fn user(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            id: None,
        }
    }

    fn builtin(id: &str) -> Self {
        Self {
            text: String::new(),
            id: Some(id.to_owned()),
        }
    }

    pub fn show(&self) -> String {
        match &self.id {
            Some(id) => crate::i18n::tr(id),
            None => self.text.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub name: Name,
    #[serde(default)]
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub name: Name,
    /// Hidden tabs stay in the layout but aren't shown.
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub groups: Vec<Group>,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    #[serde(default = "version")]
    pub version: u32,
    pub tabs: Vec<Tab>,
}

fn version() -> u32 {
    1
}

pub const FILE_NAME: &str = "toolbar.json";

pub fn layout_path() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join(FILE_NAME))
}

impl Layout {
    /// The saved layout, or the default one. An unreadable file gives an
    /// error message and the defaults (the file is left alone).
    pub fn load() -> (Self, Option<String>) {
        let Some(path) = layout_path() else {
            return (Self::default(), None);
        };
        match std::fs::read_to_string(&path) {
            Err(_) => (Self::default(), None),
            Ok(text) => match serde_json::from_str(&text) {
                Ok(layout) => (layout, None),
                Err(e) => (
                    Self::default(),
                    Some(t!(
                        "toolbar-read-failed",
                        path = path.display().to_string(),
                        error = e.to_string()
                    )),
                ),
            },
        }
    }

    /// Writes the layout, or removes the file when it is the default one so
    /// later versions' defaults apply.
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = layout_path() else {
            return Ok(());
        };
        if *self == Self::default() {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            };
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| {
            t!(
                "toolbar-save-failed",
                path = path.display().to_string(),
                error = e.to_string()
            )
        })
    }

    /// Removes the button at `tab`, `group`, `item`.
    pub fn remove(&mut self, tab: usize, group: usize, item: usize) {
        if let Some(items) = self
            .tabs
            .get_mut(tab)
            .and_then(|t| t.groups.get_mut(group))
            .map(|g| &mut g.items)
            && item < items.len()
        {
            items.remove(item);
        }
    }
}

fn group(id: &str, items: Vec<(Action, Size)>) -> Group {
    Group {
        name: Name::builtin(id),
        items: items.iter().map(|(a, s)| Item::new(a, *s)).collect(),
    }
}

fn tab(id: &str, groups: Vec<Group>) -> Tab {
    Tab {
        name: Name::builtin(id),
        visible: true,
        groups,
    }
}

impl Default for Layout {
    fn default() -> Self {
        use Action::*;
        use Size::{Large as L, Small as S};
        Self {
            version: 1,
            tabs: vec![
                tab(
                    "toolbar-tab-home",
                    vec![
                        group(
                            "toolbar-group-file",
                            vec![
                                (NewTab, L),
                                (Open, L),
                                (Save, L),
                                (SaveAs, S),
                                (SaveAll, S),
                                (Print, S),
                            ],
                        ),
                        group(
                            "toolbar-group-clipboard",
                            vec![(Paste, L), (Cut, S), (Copy, S), (SelectAll, S)],
                        ),
                        group("toolbar-group-history", vec![(Undo, S), (Redo, S)]),
                        group(
                            "toolbar-group-find",
                            vec![
                                (Find, L),
                                (Replace, L),
                                (FindInFiles, S),
                                (GoToLine, S),
                                (ToggleBookmark, S),
                            ],
                        ),
                        group(
                            "toolbar-group-tabs",
                            vec![(CloseTab, S), (CloseAll, S), (TabList, S)],
                        ),
                    ],
                ),
                tab(
                    "toolbar-tab-edit",
                    vec![
                        group(
                            "toolbar-group-lines",
                            vec![
                                (Duplicate, S),
                                (DeleteLines, S),
                                (JoinLines, S),
                                (MoveLinesUp, S),
                                (MoveLinesDown, S),
                                (InsertBlankBelow, S),
                            ],
                        ),
                        group(
                            "toolbar-group-sort",
                            vec![
                                (
                                    SortLines {
                                        kind: SortKind::Lexical,
                                        desc: false,
                                    },
                                    S,
                                ),
                                (
                                    SortLines {
                                        kind: SortKind::Lexical,
                                        desc: true,
                                    },
                                    S,
                                ),
                                (RemoveDuplicateLines, S),
                            ],
                        ),
                        group(
                            "toolbar-group-case",
                            vec![
                                (ConvertCase(CaseKind::Upper), S),
                                (ConvertCase(CaseKind::Lower), S),
                                (ConvertCase(CaseKind::Title), S),
                            ],
                        ),
                        group(
                            "toolbar-group-comment",
                            vec![(ToggleLineComment, L), (ToggleBlockComment, L)],
                        ),
                        group(
                            "toolbar-group-blanks",
                            vec![
                                (Trim(TrimKind::Trailing), S),
                                (TabsToSpaces, S),
                                (SpacesToTabs { all: false }, S),
                            ],
                        ),
                        group(
                            "toolbar-group-selection",
                            vec![
                                (SelectNextOccurrence, S),
                                (SelectAllOccurrences, S),
                                (ColumnEditor, S),
                            ],
                        ),
                    ],
                ),
                tab(
                    "toolbar-tab-view",
                    vec![
                        group(
                            "toolbar-group-show",
                            vec![
                                (ToggleWordWrap, S),
                                (ToggleLineNumbers, S),
                                (ToggleCurrentLine, S),
                                (ToggleWhitespace, S),
                                (ToggleEol, S),
                                (ToggleIndentGuides, S),
                            ],
                        ),
                        group(
                            "toolbar-group-zoom",
                            vec![(ZoomIn, S), (ZoomOut, S), (ZoomReset, S)],
                        ),
                        group(
                            "toolbar-group-folding",
                            vec![
                                (FoldAll, S),
                                (UnfoldAll, S),
                                (FoldCurrent, S),
                                (UnfoldCurrent, S),
                            ],
                        ),
                        group(
                            "toolbar-group-panels",
                            vec![
                                (ToggleFunctionList, S),
                                (ToggleDocMap, S),
                                (ToggleFolderPanel, S),
                            ],
                        ),
                        group(
                            "toolbar-group-window",
                            vec![
                                (ToggleDistractionFree, L),
                                (MoveActiveToOtherView, S),
                                (CloneActiveToOtherView, S),
                                (CommandPalette, S),
                            ],
                        ),
                        group(
                            "toolbar-group-customise",
                            vec![(Preferences, S), (ShortcutMapper, S), (CustomiseToolbar, S)],
                        ),
                    ],
                ),
                tab(
                    "toolbar-tab-tools",
                    vec![
                        group(
                            "toolbar-group-compare",
                            vec![
                                (CompareTabs, L),
                                (CompareToSaved, S),
                                (CompareToClipboard, S),
                                (NextDifference, S),
                                (PrevDifference, S),
                                (CloseComparison, S),
                            ],
                        ),
                        group(
                            "toolbar-group-json",
                            vec![
                                (Json(JsonOp::Format), L),
                                (Json(JsonOp::Minify), S),
                                (Json(JsonOp::Validate), S),
                                (ToggleJsonTree, S),
                            ],
                        ),
                        group(
                            "toolbar-group-xml",
                            vec![
                                (Xml(XmlOp::PrettyPrint), L),
                                (Xml(XmlOp::CheckSyntax), S),
                                (Xml(XmlOp::ValidateSchema), S),
                                (ToggleXPathPanel, S),
                            ],
                        ),
                        group(
                            "toolbar-group-data",
                            vec![
                                (ToggleCsvQuery, L),
                                (Mime(MimeOp::Base64Encode), S),
                                (Mime(MimeOp::Base64Decode), S),
                                (Mime(MimeOp::UrlDecode), S),
                            ],
                        ),
                        group(
                            "toolbar-group-macros",
                            vec![
                                (ToggleMacroRecording, L),
                                (PlayMacro, S),
                                (RunMacroMultiple, S),
                                (SaveMacro, S),
                            ],
                        ),
                    ],
                ),
            ],
        }
    }
}

// -------------------------------------------------------------------------
// Icons

/// Phosphor icons by name, like `floppy-disk`.
fn icon_table() -> &'static HashMap<String, &'static str> {
    static TABLE: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ph::ICONS
            .iter()
            .map(|(name, glyph)| (icon_name(name), *glyph))
            .collect()
    })
}

/// `FLOPPY_DISK` as users write it: `floppy-disk`.
fn icon_name(constant: &str) -> String {
    constant.to_ascii_lowercase().replace('_', "-")
}

/// The glyph of a Phosphor icon name (`floppy-disk`, `FLOPPY_DISK` and
/// `floppy_disk` all work).
pub fn glyph(name: &str) -> Option<&'static str> {
    icon_table()
        .get(&name.trim().to_ascii_lowercase().replace('_', "-"))
        .copied()
}

/// Every icon name, sorted, for the icon picker.
pub fn icon_names() -> Vec<(String, &'static str)> {
    let mut list: Vec<(String, &'static str)> =
        icon_table().iter().map(|(n, g)| (n.clone(), *g)).collect();
    list.sort();
    list
}

/// The icon for commands that have none of their own.
pub const GENERIC_ICON: &str = ph::LIGHTNING;
/// The icon for plugin commands whose plugin gave none.
pub const PLUGIN_ICON: &str = ph::PUZZLE_PIECE;

/// The icon a command shows unless the user or a plugin picked another.
pub fn default_icon(action: &Action) -> &'static str {
    use Action::*;
    match action {
        NewTab => ph::FILE_PLUS,
        Open | OpenPath(_) => ph::FOLDER_OPEN,
        Save => ph::FLOPPY_DISK,
        SaveAs => ph::FLOPPY_DISK_BACK,
        SaveAll => ph::FILES,
        CloseTab | CloseTabAt(_) => ph::X,
        CloseAll => ph::X_SQUARE,
        CloseOthers => ph::X_CIRCLE,
        Quit => ph::SIGN_OUT,
        NextTab => ph::CARET_RIGHT,
        PrevTab => ph::CARET_LEFT,
        TabList => ph::TABS,
        MoveActiveToOtherView | MoveToOtherView(..) => ph::SQUARE_SPLIT_HORIZONTAL,
        CloneActiveToOtherView | CloneToOtherView(..) => ph::COLUMNS,
        FocusOtherView => ph::ARROWS_LEFT_RIGHT,
        ZoomIn => ph::MAGNIFYING_GLASS_PLUS,
        ZoomOut => ph::MAGNIFYING_GLASS_MINUS,
        ZoomReset => ph::ARROWS_IN_SIMPLE,
        ToggleWordWrap => ph::ARROW_BEND_DOWN_LEFT,
        ToggleLineNumbers => ph::LIST_NUMBERS,
        ToggleCurrentLine => ph::HIGHLIGHTER,
        SetTheme(_) | OpenThemesFolder | ReloadThemes | RestoreThemes | StyleConfigurator => {
            ph::PALETTE
        }
        SetLanguage(_) | NewLanguage | OpenLanguagesFolder | ReloadLanguages => ph::CODE,
        OpenApiFolder => ph::LIST_MAGNIFYING_GLASS,
        SetLineEnding(_) => ph::KEY_RETURN,
        ClearRecent => ph::BROOM,
        About => ph::INFO,
        UserManual | LanguageGuide | PluginGuide => ph::BOOK_OPEN,
        WebManual => ph::GLOBE,
        CheckForUpdates | ToggleUpdateCheck => ph::DOWNLOAD,
        Print => ph::PRINTER,
        ExportHtml => ph::FILE_HTML,
        ReloadFromDisk | IgnoreDiskChange => ph::ARROWS_CLOCKWISE,
        RenameFile => ph::PENCIL_SIMPLE,
        MoveToTrash => ph::TRASH,
        OpenContainingFolder => ph::FOLDER_NOTCH_OPEN,
        OpenTerminalHere => ph::TERMINAL_WINDOW,
        Summary => ph::INFO,
        TogglePin(_) => ph::PUSH_PIN,
        ConvertEncoding(_) | ReopenWithEncoding(_) => ph::TRANSLATE,
        Find | FindAllInDoc => ph::MAGNIFYING_GLASS,
        Replace | ReplaceOne | ReplaceAll => ph::SWAP,
        FindNext | FindSelectedNext => ph::ARROW_DOWN,
        FindPrev | FindSelectedPrev => ph::ARROW_UP,
        FindAllInTabs => ph::BINOCULARS,
        FindInFiles => ph::FOLDERS,
        SelectAllMatches | SelectAllOccurrences => ph::SELECTION_ALL,
        MarkAll { .. } => ph::HIGHLIGHTER_CIRCLE,
        ClearMarks => ph::ERASER,
        CloseFindBar => ph::X,
        GoToLine => ph::ARROW_LINE_DOWN,
        ToggleBookmark => ph::BOOKMARK_SIMPLE,
        NextBookmark => ph::CARET_DOUBLE_DOWN,
        PrevBookmark => ph::CARET_DOUBLE_UP,
        ClearBookmarks | InverseBookmarks => ph::BOOKMARKS,
        CutBookmarkedLines | CopyBookmarkedLines => ph::BOOKMARKS,
        RemoveBookmarkedLines | RemoveUnbookmarkedLines => ph::BOOKMARKS,
        Undo => ph::ARROW_U_UP_LEFT,
        Redo => ph::ARROW_U_UP_RIGHT,
        Cut => ph::SCISSORS,
        Copy => ph::COPY,
        Paste => ph::CLIPBOARD_TEXT,
        SelectAll => ph::SELECTION,
        Duplicate => ph::COPY_SIMPLE,
        DeleteLines => ph::BACKSPACE,
        MoveLinesUp => ph::ARROW_FAT_UP,
        MoveLinesDown => ph::ARROW_FAT_DOWN,
        JoinLines => ph::ARROWS_IN_LINE_VERTICAL,
        SplitLines => ph::ARROWS_OUT_LINE_VERTICAL,
        InsertBlankAbove => ph::ROWS_PLUS_TOP,
        InsertBlankBelow => ph::ROWS_PLUS_BOTTOM,
        SortLines { desc: false, .. } => ph::SORT_ASCENDING,
        SortLines { desc: true, .. } => ph::SORT_DESCENDING,
        ReverseLines => ph::ARROWS_DOWN_UP,
        RemoveDuplicateLines | RemoveConsecutiveDuplicates => ph::LIST_DASHES,
        RemoveEmptyLines { .. } => ph::ROWS,
        Trim(_) => ph::TEXT_OUTDENT,
        EolToSpace => ph::TEXT_ALIGN_JUSTIFY,
        TabsToSpaces | SpacesToTabs { .. } => ph::TEXT_INDENT,
        ConvertCase(CaseKind::Upper) => ph::TEXT_AA,
        ConvertCase(CaseKind::Lower) => ph::TEXT_T,
        ConvertCase(_) => ph::TEXT_A_UNDERLINE,
        ToggleLineComment => ph::CHAT_CENTERED_TEXT,
        ToggleBlockComment => ph::CODE_BLOCK,
        InsertDateTime { .. } => ph::CALENDAR_BLANK,
        SelectNextOccurrence => ph::SELECTION_PLUS,
        CompleteWord => ph::TEXTBOX,
        ShowParameterHint => ph::BRACKETS_ROUND,
        ColumnEditor => ph::TEXT_COLUMNS,
        ToggleReadOnly => ph::LOCK_SIMPLE,
        FoldAll | FoldCurrent | FoldLevel(_) => ph::MINUS_SQUARE,
        UnfoldAll | UnfoldCurrent => ph::PLUS_SQUARE,
        ToggleWhitespace => ph::DOTS_THREE,
        ToggleEol => ph::PARAGRAPH,
        ToggleIndentGuides => ph::LINE_VERTICAL,
        ToggleEdge => ph::RULER,
        ToggleAutoClose => ph::BRACKETS_CURLY,
        ToggleSmartIndent => ph::TEXT_INDENT,
        ToggleUseSpaces => ph::ARROWS_HORIZONTAL,
        ToggleCompletion | ToggleLanguageCompletion => ph::TEXTBOX,
        ToggleParameterHints => ph::BRACKETS_ROUND,
        ToggleSingleInstance => ph::APP_WINDOW,
        ToggleRememberUnsaved => ph::CLOCK_COUNTER_CLOCKWISE,
        ToggleAutoReload => ph::ARROWS_CLOCKWISE,
        Mime(_) => ph::ENVELOPE_SIMPLE,
        Json(JsonOp::Validate) => ph::CHECK_CIRCLE,
        Json(JsonOp::Minify) => ph::ARROWS_IN_SIMPLE,
        Json(_) => ph::BRACKETS_CURLY,
        Xml(XmlOp::CheckSyntax | XmlOp::ValidateSchema | XmlOp::ChooseSchema) => ph::SEAL_CHECK,
        Xml(XmlOp::XslTransform) => ph::FLOW_ARROW,
        Xml(_) => ph::BRACKETS_ANGLE,
        ToggleJsonTree => ph::TREE_VIEW,
        ToggleCsvQuery | RunCsvQuery => ph::DATABASE,
        CopyCsvResults | CsvResultsToNewTab => ph::TABLE,
        ToggleXPathPanel | CopyXPath => ph::PATH,
        ToggleCloseTags => ph::BRACKETS_ANGLE,
        ToggleXmlCheckOnSave => ph::SEAL_CHECK,
        CompareTabs | CompareWith(_) | CompareSelections | ShowComparison => ph::GIT_DIFF,
        CompareToSaved => ph::FLOPPY_DISK,
        CompareToClipboard => ph::CLIPBOARD,
        CompareToGitHead => ph::GIT_BRANCH,
        NextDifference => ph::ARROW_SQUARE_DOWN,
        PrevDifference => ph::ARROW_SQUARE_UP,
        CloseComparison => ph::X_SQUARE,
        ExportComparePatch => ph::EXPORT,
        ToggleCompareOption(_) => ph::FUNNEL,
        ToggleMacroRecording => ph::RECORD,
        PlayMacro | PlaySavedMacro(_) => ph::PLAY,
        RunMacroMultiple => ph::REPEAT,
        SaveMacro => ph::FLOPPY_DISK,
        DeleteSavedMacro(_) => ph::TRASH,
        PluginCommand { .. } => PLUGIN_ICON,
        ManagePlugins | ReloadPlugins | InstallSamplePlugins => ph::PUZZLE_PIECE,
        ShowPluginLog => ph::SCROLL,
        OpenPluginsFolder => ph::FOLDER_OPEN,
        Preferences => ph::GEAR_SIX,
        ShortcutMapper => ph::KEYBOARD,
        CommandPalette => ph::COMMAND,
        ToggleFunctionList => ph::LIST_BULLETS,
        ToggleDocMap => ph::MAP_TRIFOLD,
        ToggleFolderPanel | OpenFolderAsWorkspace => ph::TREE_STRUCTURE,
        CloseWorkspace => ph::FOLDER_MINUS,
        ToggleDistractionFree => ph::CORNERS_OUT,
        CustomiseToolbar => ph::TOOLBOX,
        ToggleToolbar => ph::SQUARES_FOUR,
        CollapseToolbar => ph::CARET_UP,
        SelectTab(_) | SelectTabInView(..) | CloseTabInView(..) => GENERIC_ICON,
    }
}

/// A menu label as a button label: without the "…" that says a dialog opens.
pub fn button_label(label: &str) -> String {
    label
        .trim_end_matches('…')
        .trim_end_matches("...")
        .trim_end()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_reads_back() {
        let layout = Layout::default();
        let json = serde_json::to_string(&layout).unwrap();
        let back: Layout = serde_json::from_str(&json).unwrap();
        assert_eq!(back, layout);
    }

    #[test]
    fn default_commands_have_their_own_icons() {
        for tab in Layout::default().tabs {
            for group in tab.groups {
                assert!(!group.items.is_empty());
                for item in group.items {
                    let action = item.action().expect(&item.command);
                    assert_ne!(default_icon(&action), GENERIC_ICON, "{}", item.command);
                }
            }
        }
    }

    #[test]
    fn finds_icons_by_name() {
        assert_eq!(glyph("floppy-disk"), Some(ph::FLOPPY_DISK));
        assert_eq!(glyph("FLOPPY_DISK"), Some(ph::FLOPPY_DISK));
        assert_eq!(glyph(" Star "), Some(ph::STAR));
        assert_eq!(glyph("no-such-icon"), None);
        assert!(icon_names().len() > 1000);
    }

    #[test]
    fn button_labels_drop_the_ellipsis() {
        assert_eq!(button_label("Open…"), "Open");
        assert_eq!(button_label("Find..."), "Find");
        assert_eq!(button_label("Save"), "Save");
    }

    #[test]
    fn older_files_without_optional_fields_load() {
        let text = r#"{"tabs":[{"name":{"text":"Mine"},"groups":[{"name":{"text":"G"},
            "items":[{"command":"\"Save\""}]}]}]}"#;
        let layout: Layout = serde_json::from_str(text).unwrap();
        let tab = &layout.tabs[0];
        assert!(tab.visible);
        assert_eq!(tab.name.show(), "Mine");
        assert_eq!(tab.groups[0].items[0].size, Size::Small);
        assert_eq!(tab.groups[0].items[0].action(), Some(Action::Save));
    }
}
