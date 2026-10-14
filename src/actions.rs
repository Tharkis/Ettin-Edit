//! Every user-facing command goes through [`Action`], whether it comes from a
//! menu, a keyboard shortcut, or (later) a plugin. Keeping one list of commands
//! is what will let plugins call into the editor and add their own.

use egui::{Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};

use crate::editor::lines::{CaseKind, SortKind, TrimKind};

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
    ClearRecent,
    About,
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
    ToggleSingleInstance,
    ToggleRememberUnsaved,
    ToggleAutoReload,
    // Macros
    ToggleMacroRecording,
    PlayMacro,
    RunMacroMultiple,
    SaveMacro,
    PlaySavedMacro(usize),
    DeleteSavedMacro(usize),
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

/// Keyboard shortcuts for actions that have one. A shortcut without Shift or
/// Alt also matches with them held, so those variants must come first.
pub fn shortcuts() -> Vec<(KeyboardShortcut, Action)> {
    let s = |m, k| KeyboardShortcut::new(m, k);
    vec![
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
    ]
}

/// Shortcuts shown in menus but handled by the editor itself, so text boxes
/// keep their own undo, copy and paste.
fn editor_shortcuts() -> Vec<(KeyboardShortcut, Action)> {
    let s = |m, k| KeyboardShortcut::new(m, k);
    vec![
        (s(CMD, Key::Z), Action::Undo),
        (s(CMD, Key::Y), Action::Redo),
        (s(CMD, Key::X), Action::Cut),
        (s(CMD, Key::C), Action::Copy),
        (s(CMD, Key::V), Action::Paste),
        (s(CMD, Key::A), Action::SelectAll),
    ]
}

/// The shortcut text shown next to a menu item.
pub fn shortcut_label(ctx: &egui::Context, action: &Action) -> Option<String> {
    shortcuts()
        .into_iter()
        .chain(editor_shortcuts())
        .find(|(_, a)| a == action)
        .map(|(s, _)| ctx.format_shortcut(&s))
}
