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
}

const CMD: Modifiers = Modifiers::COMMAND;
const CMD_SHIFT: Modifiers = Modifiers { shift: true, ..Modifiers::COMMAND };

/// Keyboard shortcuts for actions that have one.
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
    ]
}

/// The shortcut text shown next to a menu item.
pub fn shortcut_label(ctx: &egui::Context, action: &Action) -> Option<String> {
    shortcuts()
        .into_iter()
        .find(|(_, a)| a == action)
        .map(|(s, _)| ctx.format_shortcut(&s))
}
