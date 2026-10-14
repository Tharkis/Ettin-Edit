//! Every user-facing command goes through [`Action`], whether it comes from a
//! menu, a keyboard shortcut, or (later) a plugin. Keeping one list of commands
//! is what will let plugins call into the editor and add their own.

use egui::{Key, KeyboardShortcut, Modifiers};

#[derive(Clone, Debug, PartialEq)]
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
        )
    }
}

const CMD: Modifiers = Modifiers::COMMAND;
const CMD_SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::COMMAND
};
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;

/// Keyboard shortcuts for actions that have one. A shortcut without Shift also
/// matches with Shift held, so Shift variants must come first.
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
        (s(CMD_SHIFT, Key::F), Action::FindInFiles),
        (s(CMD, Key::F), Action::Find),
        (s(CMD, Key::H), Action::Replace),
        (s(CMD_SHIFT, Key::F3), Action::FindSelectedPrev),
        (s(CMD, Key::F3), Action::FindSelectedNext),
        (s(SHIFT, Key::F3), Action::FindPrev),
        (s(NONE, Key::F3), Action::FindNext),
        (s(CMD, Key::G), Action::GoToLine),
        (s(CMD, Key::F2), Action::ToggleBookmark),
        (s(SHIFT, Key::F2), Action::PrevBookmark),
        (s(NONE, Key::F2), Action::NextBookmark),
    ]
}

/// The shortcut text shown next to a menu item.
pub fn shortcut_label(ctx: &egui::Context, action: &Action) -> Option<String> {
    shortcuts()
        .into_iter()
        .find(|(_, a)| a == action)
        .map(|(s, _)| ctx.format_shortcut(&s))
}
