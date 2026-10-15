//! Every user-facing command goes through [`Action`], whether it comes from a
//! menu, a keyboard shortcut, or (later) a plugin. Keeping one list of commands
//! is what will let plugins call into the editor and add their own.

use std::collections::BTreeMap;
use std::sync::RwLock;

use egui::{Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};

use crate::editor::lines::{CaseKind, SortKind, TrimKind};
use crate::tools::json::JsonOp;
use crate::tools::mime::MimeOp;
use crate::tools::xml::XmlOp;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    NewTab,
    Open,
    OpenPath(std::path::PathBuf),
    Save,
    SaveAs,
    SaveAll,
    CloseTab,
    CloseTabAt(usize),
    CloseAll,
    CloseOthers,
    Quit,
    NextTab,
    PrevTab,
    SelectTab(usize),
    // Split view (tab index, view: 0 main, 1 other)
    SelectTabInView(usize, usize),
    CloseTabInView(usize, usize),
    MoveToOtherView(usize, usize),
    CloneToOtherView(usize, usize),
    MoveActiveToOtherView,
    CloneActiveToOtherView,
    FocusOtherView,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleWordWrap,
    ToggleLineNumbers,
    ToggleCurrentLine,
    SetTheme(String),
    SetLanguage(String),
    SetLineEnding(crate::document::LineEnding),
    OpenThemesFolder,
    /// Reads the themes folder again, so edited themes take effect.
    ReloadThemes,
    /// Writes the original built-in themes back to the themes folder.
    RestoreThemes,
    ClearRecent,
    About,
    CheckForUpdates,
    NewLanguage,
    OpenLanguagesFolder,
    OpenApiFolder,
    ReloadLanguages,
    LanguageGuide,
    Print,
    ExportHtml,
    ToggleUpdateCheck,
    // Files
    ReloadFromDisk,
    /// Keep the text as it is after the file changed or vanished on disk.
    IgnoreDiskChange,
    RenameFile,
    MoveToTrash,
    OpenContainingFolder,
    OpenTerminalHere,
    Summary,
    TogglePin(usize),
    /// Mark the text to be saved in another encoding (by name).
    ConvertEncoding(String),
    /// Read the file again in another encoding (by name).
    ReopenWithEncoding(String),
    // Search
    Find,
    Replace,
    FindNext,
    FindPrev,
    /// Search for the selection (or the word at the caret).
    FindSelectedNext,
    FindSelectedPrev,
    ReplaceOne,
    ReplaceAll,
    FindAllInDoc,
    FindAllInTabs,
    FindInFiles,
    SelectAllMatches,
    MarkAll {
        bookmark: bool,
    },
    ClearMarks,
    CloseFindBar,
    GoToLine,
    // Bookmarks
    ToggleBookmark,
    NextBookmark,
    PrevBookmark,
    ClearBookmarks,
    CutBookmarkedLines,
    CopyBookmarkedLines,
    RemoveBookmarkedLines,
    RemoveUnbookmarkedLines,
    InverseBookmarks,
    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Duplicate,
    DeleteLines,
    MoveLinesUp,
    MoveLinesDown,
    JoinLines,
    SplitLines,
    InsertBlankAbove,
    InsertBlankBelow,
    SortLines {
        kind: SortKind,
        desc: bool,
    },
    ReverseLines,
    RemoveDuplicateLines,
    RemoveConsecutiveDuplicates,
    RemoveEmptyLines {
        blank: bool,
    },
    Trim(TrimKind),
    EolToSpace,
    TabsToSpaces,
    SpacesToTabs {
        all: bool,
    },
    ConvertCase(CaseKind),
    ToggleLineComment,
    ToggleBlockComment,
    InsertDateTime {
        long: bool,
    },
    SelectNextOccurrence,
    SelectAllOccurrences,
    CompleteWord,
    ShowParameterHint,
    ColumnEditor,
    ToggleReadOnly,
    // Folding
    FoldAll,
    UnfoldAll,
    FoldCurrent,
    UnfoldCurrent,
    FoldLevel(usize),
    // View
    ToggleWhitespace,
    ToggleEol,
    ToggleIndentGuides,
    ToggleEdge,
    // Settings
    ToggleAutoClose,
    ToggleSmartIndent,
    ToggleUseSpaces,
    ToggleCompletion,
    ToggleLanguageCompletion,
    ToggleParameterHints,
    ToggleSingleInstance,
    ToggleRememberUnsaved,
    ToggleAutoReload,
    // Text tools
    Mime(MimeOp),
    Json(JsonOp),
    Xml(XmlOp),
    ToggleJsonTree,
    // CsvQuery
    ToggleCsvQuery,
    RunCsvQuery,
    CopyCsvResults,
    CsvResultsToNewTab,
    ToggleXPathPanel,
    /// Copy the XPath of the element at the caret.
    CopyXPath,
    ToggleCloseTags,
    ToggleXmlCheckOnSave,
    // Compare
    /// Compare the tabs of the two views, or the tab with the one used before it.
    CompareTabs,
    /// Compare the active tab with this one.
    CompareWith(usize),
    CompareToSaved,
    CompareToClipboard,
    CompareToGitHead,
    /// Compare two selections: two in one tab, or one in each view.
    CompareSelections,
    NextDifference,
    PrevDifference,
    ShowComparison,
    CloseComparison,
    ExportComparePatch,
    ToggleCompareOption(crate::compare::CompareOpt),
    // Macros
    ToggleMacroRecording,
    PlayMacro,
    RunMacroMultiple,
    SaveMacro,
    PlaySavedMacro(usize),
    DeleteSavedMacro(usize),
    // Plugins
    /// Run a command a plugin added.
    PluginCommand {
        plugin: String,
        command: String,
    },
    ManagePlugins,
    ShowPluginLog,
    OpenPluginsFolder,
    ReloadPlugins,
    InstallSamplePlugins,
    PluginGuide,
    // Customisation and panels
    Preferences,
    ShortcutMapper,
    StyleConfigurator,
    CommandPalette,
    ToggleFunctionList,
    ToggleDocMap,
    ToggleFolderPanel,
    OpenFolderAsWorkspace,
    CloseWorkspace,
    ToggleDistractionFree,
    TabList,
}

impl Action {
    /// False for commands that leave keyboard focus where it is (in the find bar,
    /// or in a dialog they open).
    pub fn focuses_editor(&self) -> bool {
        !matches!(
            self,
            Action::Find
                | Action::Replace
                | Action::FindNext
                | Action::FindPrev
                | Action::ReplaceOne
                | Action::ReplaceAll
                | Action::FindAllInDoc
                | Action::FindAllInTabs
                | Action::FindInFiles
                | Action::MarkAll { .. }
                | Action::ClearMarks
                | Action::GoToLine
                | Action::ColumnEditor
                | Action::RunMacroMultiple
                | Action::SaveMacro
                | Action::RenameFile
                | Action::MoveToTrash
                | Action::Summary
                | Action::ToggleXPathPanel
                | Action::ManagePlugins
                | Action::ShowPluginLog
                | Action::Preferences
                | Action::ShortcutMapper
                | Action::StyleConfigurator
                | Action::CommandPalette
                | Action::TabList
        )
    }

    /// Commands a macro records: the ones that change or move through the text.
    pub fn recordable(&self) -> bool {
        matches!(
            self,
            Action::FindNext
                | Action::FindPrev
                | Action::FindSelectedNext
                | Action::FindSelectedPrev
                | Action::ReplaceOne
                | Action::ReplaceAll
                | Action::GoToLine
                | Action::ToggleBookmark
                | Action::NextBookmark
                | Action::PrevBookmark
                | Action::Undo
                | Action::Redo
                | Action::Cut
                | Action::Copy
                | Action::Paste
                | Action::SelectAll
                | Action::Duplicate
                | Action::DeleteLines
                | Action::MoveLinesUp
                | Action::MoveLinesDown
                | Action::JoinLines
                | Action::SplitLines
                | Action::InsertBlankAbove
                | Action::InsertBlankBelow
                | Action::SortLines { .. }
                | Action::ReverseLines
                | Action::RemoveDuplicateLines
                | Action::RemoveConsecutiveDuplicates
                | Action::RemoveEmptyLines { .. }
                | Action::Trim(_)
                | Action::EolToSpace
                | Action::TabsToSpaces
                | Action::SpacesToTabs { .. }
                | Action::ConvertCase(_)
                | Action::ToggleLineComment
                | Action::ToggleBlockComment
                | Action::InsertDateTime { .. }
                | Action::SelectNextOccurrence
                | Action::SelectAllOccurrences
                | Action::Mime(_)
                | Action::Json(JsonOp::Format | JsonOp::Minify | JsonOp::SortKeys)
                | Action::Xml(
                    XmlOp::PrettyPrint
                        | XmlOp::PrettyPrintAttributes
                        | XmlOp::Linearize
                        | XmlOp::Escape
                        | XmlOp::Unescape
                )
        )
    }
}

const CMD: Modifiers = Modifiers::COMMAND;
const CMD_SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::COMMAND
};
const CMD_ALT: Modifiers = Modifiers {
    alt: true,
    ..Modifiers::COMMAND
};
const CMD_ALT_SHIFT: Modifiers = Modifiers {
    alt: true,
    shift: true,
    ..Modifiers::COMMAND
};
const ALT: Modifiers = Modifiers::ALT;
const ALT_SHIFT: Modifiers = Modifiers {
    alt: true,
    shift: true,
    ..Modifiers::NONE
};
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;

/// Shortcuts changed in the shortcut mapper: each listed command's shortcuts
/// replace its defaults (an empty list leaves it without one).
static OVERRIDES: RwLock<Vec<(Action, Vec<KeyboardShortcut>)>> = RwLock::new(Vec::new());

/// The key a command is saved under in the settings.
pub fn command_key(action: &Action) -> String {
    serde_json::to_string(action).unwrap_or_default()
}

/// Uses the shortcuts from the settings. Returns a message for each one that
/// can't be read; those are left out.
pub fn set_overrides(saved: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut errors = Vec::new();
    let mut list = Vec::new();
    for (key, texts) in saved {
        let Ok(action) = serde_json::from_str::<Action>(key) else {
            // A command from an older or newer version; keep it in the settings.
            continue;
        };
        let mut shortcuts = Vec::new();
        for text in texts {
            match crate::plugins::shortcut::parse(text) {
                Ok(s) => shortcuts.push(s),
                Err(e) => errors.push(format!("Shortcut settings: {e}")),
            }
        }
        list.push((action, shortcuts));
    }
    *OVERRIDES.write().unwrap_or_else(|e| e.into_inner()) = list;
    errors
}

/// Replaces the shortcuts of commands changed in the shortcut mapper, then
/// puts the ones with more modifiers first (see [`shortcuts`]).
pub fn apply_overrides(list: &mut Vec<(KeyboardShortcut, Action)>) {
    let overrides = OVERRIDES.read().unwrap_or_else(|e| e.into_inner());
    if overrides.is_empty() {
        return;
    }
    list.retain(|(_, a)| !overrides.iter().any(|(o, _)| o == a));
    for (action, shortcuts) in overrides.iter() {
        for s in shortcuts {
            list.push((*s, action.clone()));
        }
    }
    list.sort_by_key(|(s, _)| std::cmp::Reverse(crate::plugins::shortcut::modifier_count(s)));
}

/// The shortcut shown for a command whose default is `default` (a plugin's),
/// after the user's changes.
pub fn effective_shortcut(
    action: &Action,
    default: Option<KeyboardShortcut>,
) -> Option<KeyboardShortcut> {
    let overrides = OVERRIDES.read().unwrap_or_else(|e| e.into_inner());
    match overrides.iter().find(|(a, _)| a == action) {
        Some((_, list)) => list.first().copied(),
        None => default,
    }
}

/// Keyboard shortcuts for actions that have one, after the user's changes.
pub fn shortcuts() -> Vec<(KeyboardShortcut, Action)> {
    let mut list = default_shortcuts();
    apply_overrides(&mut list);
    list
}

/// Ettin Edit's own shortcuts. A shortcut without Shift or Alt also matches
/// with them held, so those variants must come first.
pub fn default_shortcuts() -> Vec<(KeyboardShortcut, Action)> {
    let s = |m, k| KeyboardShortcut::new(m, k);
    vec![
        // Text tools (before the shorter combinations they contain)
        (s(CMD_ALT_SHIFT, Key::B), Action::Xml(XmlOp::PrettyPrint)),
        (s(CMD_ALT_SHIFT, Key::L), Action::Xml(XmlOp::Linearize)),
        (s(CMD_ALT_SHIFT, Key::M), Action::Xml(XmlOp::CheckSyntax)),
        (s(CMD_ALT_SHIFT, Key::P), Action::CopyXPath),
        (s(CMD_ALT_SHIFT, Key::E), Action::ToggleXPathPanel),
        (s(CMD_ALT, Key::M), Action::Json(JsonOp::Format)),
        (s(CMD_ALT, Key::N), Action::Json(JsonOp::Minify)),
        (s(CMD_ALT, Key::J), Action::ToggleJsonTree),
        (s(CMD_ALT, Key::Q), Action::ToggleCsvQuery),
        (s(CMD_ALT, Key::D), Action::CompareTabs),
        (s(SHIFT, Key::F7), Action::PrevDifference),
        (s(NONE, Key::F7), Action::NextDifference),
        (s(CMD, Key::Comma), Action::Preferences),
        (s(NONE, Key::F1), Action::CommandPalette),
        (s(NONE, Key::F11), Action::ToggleDistractionFree),
        (s(CMD, Key::N), Action::NewTab),
        (s(CMD, Key::O), Action::Open),
        (s(CMD_SHIFT, Key::S), Action::SaveAll),
        (s(CMD, Key::S), Action::Save),
        (s(Modifiers { alt: true, ..CMD }, Key::S), Action::SaveAs),
        (s(CMD, Key::W), Action::CloseTab),
        (s(CMD_SHIFT, Key::W), Action::CloseAll),
        (s(CMD_SHIFT, Key::Tab), Action::PrevTab),
        (s(CMD, Key::Tab), Action::NextTab),
        (s(CMD, Key::PageDown), Action::NextTab),
        (s(CMD, Key::PageUp), Action::PrevTab),
        (s(CMD, Key::Plus), Action::ZoomIn),
        (s(CMD, Key::Equals), Action::ZoomIn),
        (s(CMD, Key::Minus), Action::ZoomOut),
        (s(CMD, Key::Num0), Action::ZoomReset),
        (s(Modifiers::ALT, Key::Z), Action::ToggleWordWrap),
        (s(CMD_ALT_SHIFT, Key::F), Action::UnfoldCurrent),
        (s(CMD_ALT, Key::F), Action::FoldCurrent),
        (s(CMD_SHIFT, Key::F), Action::FindInFiles),
        (s(CMD, Key::F), Action::Find),
        (s(CMD, Key::H), Action::Replace),
        (s(ALT, Key::F3), Action::SelectAllOccurrences),
        (s(CMD_SHIFT, Key::F3), Action::FindSelectedPrev),
        (s(CMD, Key::F3), Action::FindSelectedNext),
        (s(SHIFT, Key::F3), Action::FindPrev),
        (s(NONE, Key::F3), Action::FindNext),
        (s(CMD, Key::G), Action::GoToLine),
        (s(CMD, Key::F2), Action::ToggleBookmark),
        (s(SHIFT, Key::F2), Action::PrevBookmark),
        (s(NONE, Key::F2), Action::NextBookmark),
        (s(NONE, Key::F8), Action::FocusOtherView),
        // Edit
        (s(CMD_SHIFT, Key::D), Action::SelectNextOccurrence),
        (s(CMD, Key::D), Action::Duplicate),
        (s(CMD_SHIFT, Key::L), Action::DeleteLines),
        (s(CMD_SHIFT, Key::ArrowUp), Action::MoveLinesUp),
        (s(CMD_SHIFT, Key::ArrowDown), Action::MoveLinesDown),
        (s(CMD, Key::J), Action::JoinLines),
        (s(CMD, Key::I), Action::SplitLines),
        (s(CMD_ALT_SHIFT, Key::Enter), Action::InsertBlankBelow),
        (s(CMD_ALT, Key::Enter), Action::InsertBlankAbove),
        (
            s(CMD_ALT_SHIFT, Key::U),
            Action::ConvertCase(CaseKind::Invert),
        ),
        (s(CMD_ALT, Key::U), Action::ConvertCase(CaseKind::Sentence)),
        (s(CMD_SHIFT, Key::U), Action::ConvertCase(CaseKind::Upper)),
        (s(CMD, Key::U), Action::ConvertCase(CaseKind::Lower)),
        (s(ALT, Key::U), Action::ConvertCase(CaseKind::Title)),
        (s(CMD_SHIFT, Key::Q), Action::ToggleBlockComment),
        (s(CMD, Key::Q), Action::ToggleLineComment),
        (s(SHIFT, Key::F5), Action::InsertDateTime { long: true }),
        (s(NONE, Key::F5), Action::InsertDateTime { long: false }),
        (s(ALT, Key::C), Action::ColumnEditor),
        // Folding
        (s(ALT_SHIFT, Key::Num0), Action::UnfoldAll),
        (s(ALT, Key::Num0), Action::FoldAll),
        (s(ALT, Key::Num1), Action::FoldLevel(1)),
        (s(ALT, Key::Num2), Action::FoldLevel(2)),
        (s(ALT, Key::Num3), Action::FoldLevel(3)),
        (s(ALT, Key::Num4), Action::FoldLevel(4)),
        (s(ALT, Key::Num5), Action::FoldLevel(5)),
        // Macros
        (s(CMD_SHIFT, Key::R), Action::ToggleMacroRecording),
        (s(CMD_SHIFT, Key::P), Action::PlayMacro),
        (s(CMD, Key::P), Action::Print),
    ]
}

/// Shortcuts shown in menus but handled by the editor itself, so text boxes
/// keep their own undo, copy and paste. They can't be changed.
pub fn editor_shortcuts() -> Vec<(KeyboardShortcut, Action)> {
    let s = |m, k| KeyboardShortcut::new(m, k);
    vec![
        (s(CMD, Key::Z), Action::Undo),
        (s(CMD, Key::Y), Action::Redo),
        (s(CMD, Key::X), Action::Cut),
        (s(CMD, Key::C), Action::Copy),
        (s(CMD, Key::V), Action::Paste),
        (s(CMD, Key::A), Action::SelectAll),
        (s(CMD_SHIFT, Key::Space), Action::ShowParameterHint),
        (s(CMD, Key::Space), Action::CompleteWord),
    ]
}

/// Every shortcut, including the editor's own; for looking up menu labels.
pub fn all_shortcuts() -> Vec<(KeyboardShortcut, Action)> {
    let mut all = shortcuts();
    all.extend(editor_shortcuts());
    all
}

/// The shortcut text shown next to a menu item.
pub fn shortcut_label(ctx: &egui::Context, action: &Action) -> Option<String> {
    shortcut_label_in(&all_shortcuts(), ctx, action)
}

/// [`shortcut_label`] with the list from [`all_shortcuts`] built once.
pub fn shortcut_label_in(
    all: &[(KeyboardShortcut, Action)],
    ctx: &egui::Context,
    action: &Action,
) -> Option<String> {
    all.iter()
        .find(|(_, a)| a == action)
        .map(|(s, _)| ctx.format_shortcut(s))
}
