//! Every user-facing command goes through [`Action`], whether it comes from a
//! menu, a keyboard shortcut, or (later) a plugin. Keeping one list of commands
//! is what will let plugins call into the editor and add their own.

use std::collections::BTreeMap;
use std::sync::RwLock;

use egui::{Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};

use crate::editor::lines::{CaseKind, SortKind, TrimKind};
use crate::plugins::shortcut::Chord;
use crate::tools::json::JsonOp;
use crate::tools::mime::MimeOp;
use crate::tools::transform::TransformOp;
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
    UserManual,
    /// Help > Welcome: the Welcome tab.
    Welcome,
    WebManual,
    CheckForUpdates,
    /// Language > Define Language…: the form for a simple language.
    DefineLanguage,
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
    /// Column mode: a plain drag or Shift+arrows select a rectangle.
    ToggleColumnMode,
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
    /// Labels for control characters, unusual spaces and zero-width characters.
    ToggleInvisible,
    /// Whitespace, line ends and invisible characters together.
    ToggleAllChars,
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
    // Transform
    Transform(TransformOp),
    /// Opens the dialog that asks for a Transform command's settings.
    TransformDialog(crate::transform_ui::DialogKind),
    /// Transform > Convert to > Read as: the format the active tab is read
    /// as, or `None` to tell from the text.
    SetReadAs(Option<crate::tools::convert::DataFormat>),
    // Recipes
    /// Shows or hides the Recipes window.
    Recipes,
    /// Runs a recipe, by id (or, from before 2.0.2, by name), on the
    /// selections or the whole document.
    RunRecipe(String),
    /// Runs a recipe, by id (or name), on every open tab.
    RunRecipeOnAllTabs(String),
    ToggleRecipeRecording,
    OpenRecipesFolder,
    /// Runs the recipe that ran last again, on the active tab.
    RunLastRecipe,
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
    /// Turns the recorded macro into a recipe, when every step is a transform.
    SaveMacroAsRecipe,
    PlaySavedMacro(usize),
    DeleteSavedMacro(usize),
    // Plugins
    /// Run a command a plugin added.
    PluginCommand {
        plugin: String,
        command: String,
    },
    /// Run a transform a plugin added: asks for its options first, when it
    /// has any.
    PluginTransform {
        plugin: String,
        transform: String,
    },
    /// Plugins > New Plugin: a dialog that writes a working plugin.
    NewPlugin,
    ManagePlugins,
    ShowPluginLog,
    OpenPluginsFolder,
    ReloadPlugins,
    InstallSamplePlugins,
    PluginGuide,
    // Customisation and panels
    Preferences,
    ShortcutMapper,
    /// Settings > Export Settings…: settings, themes, recipes… in one file.
    ExportSettings,
    ImportSettings,
    OpenSettingsFolder,
    StyleConfigurator,
    CommandPalette,
    ToggleFunctionList,
    ToggleDocMap,
    ToggleFolderPanel,
    OpenFolderAsWorkspace,
    CloseWorkspace,
    ToggleDistractionFree,
    TabList,
    /// Show the active tab's bytes in hex, or its text again.
    ToggleHexView,
    ToggleCharPanel,
    /// View > Markdown Preview: the active Markdown tab, rendered, beside it.
    ToggleMarkdownPreview,
    // Toolbar
    CustomiseToolbar,
    /// Show or hide the whole toolbar.
    ToggleToolbar,
    /// Show only the toolbar's tab names, or the buttons too.
    CollapseToolbar,
    /// Switch between the full ribbon and the simplified one-row toolbar.
    ToggleSimplifiedToolbar,
    /// Show or hide the menu bar.
    ToggleMenuBar,
    /// A toolbar button that opens a menu; it runs nothing itself.
    ToolbarMenu(ToolbarMenu),
}

/// The menus a toolbar drop-down button can open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolbarMenu {
    /// Format, minify and sort JSON; format and linearize XML.
    Format,
    /// Transform > Convert to.
    ConvertTo,
    /// Transform > Recipes.
    Recipes,
    /// Transform > Convert Case To.
    Case,
}

impl ToolbarMenu {
    pub const ALL: [ToolbarMenu; 4] = [
        ToolbarMenu::Format,
        ToolbarMenu::ConvertTo,
        ToolbarMenu::Recipes,
        ToolbarMenu::Case,
    ];

    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            ToolbarMenu::Format => "toolbar-menu-format",
            ToolbarMenu::ConvertTo => "menu-convert-to",
            ToolbarMenu::Recipes => "menu-recipes",
            ToolbarMenu::Case => "toolbar-menu-case",
        })
    }
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
                | Action::NewPlugin
                | Action::PluginTransform { .. }
                | Action::ShowPluginLog
                | Action::Preferences
                | Action::ExportSettings
                | Action::ImportSettings
                | Action::OpenSettingsFolder
                | Action::ShortcutMapper
                | Action::StyleConfigurator
                | Action::DefineLanguage
                | Action::CommandPalette
                | Action::TabList
                | Action::UserManual
                | Action::LanguageGuide
                | Action::PluginGuide
                | Action::CustomiseToolbar
                | Action::TransformDialog(_)
                | Action::Recipes
                | Action::Transform(TransformOp::Pipe { .. })
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
                | Action::RunRecipe(_)
                | Action::Transform(
                    TransformOp::FilterLines { .. }
                        | TransformOp::AffixLines { .. }
                        | TransformOp::NumberLines { .. }
                        | TransformOp::Reflow { .. }
                        | TransformOp::Convert { .. }
                        | TransformOp::ConvertTo { .. }
                        | TransformOp::Hash(_)
                        | TransformOp::HexEncode
                        | TransformOp::HexDecode
                        | TransformOp::JwtDecode
                        | TransformOp::Field(_)
                )
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
    let (list, chords, errors) = parse_overrides(saved);
    *OVERRIDES.write().unwrap_or_else(|e| e.into_inner()) = list;
    *CHORD_OVERRIDES.write().unwrap_or_else(|e| e.into_inner()) = chords;
    errors
}

type Overrides = (
    Vec<(Action, Vec<KeyboardShortcut>)>,
    Vec<(Action, Vec<Chord>)>,
    Vec<String>,
);

/// The saved shortcuts and chords by command, and messages for the ones
/// that can't be read.
fn parse_overrides(saved: &BTreeMap<String, Vec<String>>) -> Overrides {
    let mut errors = Vec::new();
    let mut list = Vec::new();
    let mut chords = Vec::new();
    for (key, texts) in saved {
        let Ok(action) = serde_json::from_str::<Action>(key) else {
            // A command from an older or newer version; keep it in the settings.
            continue;
        };
        let mut shortcuts = Vec::new();
        let mut action_chords = Vec::new();
        for text in texts {
            let parsed = if crate::plugins::shortcut::is_chord(text) {
                crate::plugins::shortcut::parse_chord(text).map(|c| action_chords.push(c))
            } else {
                crate::plugins::shortcut::parse(text).map(|s| shortcuts.push(s))
            };
            if let Err(e) = parsed {
                errors.push(t!("shortcut-settings-error", error = e));
            }
        }
        chords.push((action.clone(), action_chords));
        list.push((action, shortcuts));
    }
    (list, chords, errors)
}

/// Like [`OVERRIDES`], for two-key chords.
static CHORD_OVERRIDES: RwLock<Vec<(Action, Vec<Chord>)>> = RwLock::new(Vec::new());

/// The default chords of a keymap preset. Ctrl+K is free in both keymaps,
/// so both get VS Code's chords: with the Notepad++ style keys you can use
/// these too.
pub fn preset_chords(_keymap: crate::settings::Keymap) -> Vec<(Chord, Action)> {
    let k = KeyboardShortcut::new(CMD, Key::K);
    let c = |m, key| (k, KeyboardShortcut::new(m, key));
    vec![
        (c(NONE, Key::S), Action::SaveAll),
        (c(NONE, Key::W), Action::CloseAll),
        (c(NONE, Key::R), Action::OpenContainingFolder),
        (c(NONE, Key::Z), Action::ToggleDistractionFree),
        (c(NONE, Key::V), Action::ToggleMarkdownPreview),
        (c(CMD, Key::S), Action::ShortcutMapper),
        (c(CMD, Key::O), Action::OpenFolderAsWorkspace),
        (c(CMD, Key::Num0), Action::FoldAll),
        (c(CMD, Key::J), Action::UnfoldAll),
        (c(CMD, Key::OpenBracket), Action::FoldCurrent),
        (c(CMD, Key::CloseBracket), Action::UnfoldCurrent),
    ]
}

/// The chords in use: the keymap's, after the user's changes.
pub fn chords() -> Vec<(Chord, Action)> {
    let overrides = CHORD_OVERRIDES.read().unwrap_or_else(|e| e.into_inner());
    chords_with(preset_chords(keymap()), &overrides)
}

fn chords_with(
    mut list: Vec<(Chord, Action)>,
    overrides: &[(Action, Vec<Chord>)],
) -> Vec<(Chord, Action)> {
    list.retain(|(_, a)| !overrides.iter().any(|(o, _)| o == a));
    for (action, chords) in overrides.iter() {
        for c in chords {
            list.push((*c, action.clone()));
        }
    }
    list
}

/// The first chord of a command, as menus show it.
pub fn chord_label(ctx: &egui::Context, action: &Action) -> Option<String> {
    chords()
        .iter()
        .find(|(_, a)| a == action)
        .map(|(c, _)| crate::plugins::shortcut::format_chord(ctx, c))
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

/// The keymap preset in use (Settings > Shortcut Mapper > Keymap).
static KEYMAP: RwLock<crate::settings::Keymap> = RwLock::new(crate::settings::Keymap::Ettin);

pub fn set_keymap(keymap: crate::settings::Keymap) {
    *KEYMAP.write().unwrap_or_else(|e| e.into_inner()) = keymap;
}

pub fn keymap() -> crate::settings::Keymap {
    *KEYMAP.read().unwrap_or_else(|e| e.into_inner())
}

/// The default shortcuts of the keymap in use, before the user's changes.
/// A shortcut without Shift or Alt also matches with them held, so those
/// variants come first.
pub fn default_shortcuts() -> Vec<(KeyboardShortcut, Action)> {
    preset_shortcuts(keymap())
}

/// The default shortcuts of a keymap preset.
pub fn preset_shortcuts(keymap: crate::settings::Keymap) -> Vec<(KeyboardShortcut, Action)> {
    let mut list = ettin_shortcuts();
    if keymap == crate::settings::Keymap::VsCode {
        let changes = vs_code_changes();
        list.retain(|(_, a)| !changes.iter().any(|(c, _)| c == a));
        for (action, keys) in changes {
            for k in keys {
                list.push((k, action.clone()));
            }
        }
        list.sort_by_key(|(s, _)| std::cmp::Reverse(crate::plugins::shortcut::modifier_count(s)));
    }
    list
}

/// Where VS Code style differs from Ettin's keys: each command's shortcuts
/// in VS Code style (an empty list has none).
pub fn vs_code_changes() -> Vec<(Action, Vec<KeyboardShortcut>)> {
    let s = |m, k| KeyboardShortcut::new(m, k);
    vec![
        (Action::SaveAs, vec![s(CMD_SHIFT, Key::S)]),
        // And VS Code's chord Ctrl+K S (see `preset_chords`).
        (Action::SaveAll, vec![s(CMD_ALT, Key::S)]),
        (
            Action::CommandPalette,
            vec![s(CMD_SHIFT, Key::P), s(NONE, Key::F1)],
        ),
        (Action::PlayMacro, vec![]),
        (Action::TabList, vec![s(CMD, Key::P)]),
        (Action::Print, vec![]),
        (Action::DeleteLines, vec![s(CMD_SHIFT, Key::K)]),
        (Action::SelectAllOccurrences, vec![s(CMD_SHIFT, Key::L)]),
        (Action::SelectNextOccurrence, vec![s(CMD, Key::D)]),
        (Action::Duplicate, vec![s(ALT_SHIFT, Key::ArrowDown)]),
        (Action::MoveLinesUp, vec![s(ALT, Key::ArrowUp)]),
        (Action::MoveLinesDown, vec![s(ALT, Key::ArrowDown)]),
        (Action::ToggleLineComment, vec![s(CMD, Key::Slash)]),
        (Action::ToggleBlockComment, vec![s(ALT_SHIFT, Key::A)]),
        (Action::InsertBlankBelow, vec![s(CMD, Key::Enter)]),
        (Action::InsertBlankAbove, vec![s(CMD_SHIFT, Key::Enter)]),
        (Action::FoldCurrent, vec![s(CMD_SHIFT, Key::OpenBracket)]),
        (Action::UnfoldCurrent, vec![s(CMD_SHIFT, Key::CloseBracket)]),
        (Action::ToggleMarkdownPreview, vec![s(CMD_SHIFT, Key::V)]),
        (Action::ToggleFolderPanel, vec![s(CMD, Key::B)]),
    ]
}

/// Ettin Edit's own shortcuts (the Notepad++ style keymap).
fn ettin_shortcuts() -> Vec<(KeyboardShortcut, Action)> {
    let s = |m, k| KeyboardShortcut::new(m, k);
    vec![
        // Text tools (before the shorter combinations they contain)
        (s(CMD_ALT_SHIFT, Key::B), Action::Xml(XmlOp::PrettyPrint)),
        (s(CMD_ALT_SHIFT, Key::L), Action::Xml(XmlOp::Linearize)),
        (s(CMD_ALT_SHIFT, Key::M), Action::Xml(XmlOp::CheckSyntax)),
        (s(CMD_ALT_SHIFT, Key::P), Action::CopyXPath),
        (s(CMD_ALT_SHIFT, Key::E), Action::ToggleXPathPanel),
        (s(CMD_ALT_SHIFT, Key::H), Action::ToggleHexView),
        (s(CMD_ALT, Key::M), Action::Json(JsonOp::Format)),
        (s(CMD_ALT, Key::N), Action::Json(JsonOp::Minify)),
        (s(CMD_ALT, Key::J), Action::ToggleJsonTree),
        (s(CMD_ALT, Key::Q), Action::ToggleCsvQuery),
        (
            s(CMD_ALT, Key::P),
            Action::TransformDialog(crate::transform_ui::DialogKind::Pipe),
        ),
        (
            s(CMD_ALT, Key::K),
            Action::TransformDialog(crate::transform_ui::DialogKind::FilterLines { keep: true }),
        ),
        (s(CMD_ALT, Key::R), Action::Recipes),
        (s(CMD_ALT, Key::D), Action::CompareTabs),
        (s(SHIFT, Key::F7), Action::PrevDifference),
        (s(NONE, Key::F7), Action::NextDifference),
        (s(CMD, Key::Comma), Action::Preferences),
        (s(CMD, Key::F1), Action::CollapseToolbar),
        (s(CMD_SHIFT, Key::M), Action::ToggleMarkdownPreview),
        (s(CMD, Key::M), Action::ToggleMenuBar),
        (s(SHIFT, Key::F1), Action::UserManual),
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
        (s(ALT_SHIFT, Key::C), Action::ToggleColumnMode),
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
        .or_else(|| chord_label(ctx, action))
}

/// The id plugins pass to `run_action` / `runAction` to run a command:
/// its name, like `Duplicate`, or JSON for one with settings, like
/// `{"ConvertCase":"Upper"}`. Settings > Shortcut Mapper and the command
/// palette copy it (Copy Command ID).
pub fn command_id(action: &Action) -> String {
    match serde_json::to_value(action) {
        Ok(serde_json::Value::String(name)) => name,
        Ok(v) => v.to_string(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Keymap;
    use crate::tools::xml::XmlOp;

    /// No two commands share a shortcut in either keymap, counting the
    /// editor's own keys.
    #[test]
    fn keymaps_have_no_clashes() {
        for keymap in [Keymap::Ettin, Keymap::VsCode] {
            let mut all = preset_shortcuts(keymap);
            all.extend(editor_shortcuts());
            for (i, (a, x)) in all.iter().enumerate() {
                for (b, y) in &all[i + 1..] {
                    assert!(
                        !crate::plugins::shortcut::same(a, b) || x == y,
                        "{keymap:?}: {x:?} and {y:?} share a shortcut"
                    );
                }
            }
        }
    }

    /// A chord's first key isn't also a shortcut, or it could never wait
    /// for the second key; and chords are unique.
    #[test]
    fn chords_start_with_free_keys() {
        for keymap in [Keymap::Ettin, Keymap::VsCode] {
            let mut singles = preset_shortcuts(keymap);
            singles.extend(editor_shortcuts());
            let chords = preset_chords(keymap);
            for (i, (c, a)) in chords.iter().enumerate() {
                assert!(
                    !singles
                        .iter()
                        .any(|(s, _)| crate::plugins::shortcut::same(s, &c.0)),
                    "{keymap:?}: {a:?} starts with a shortcut"
                );
                for (d, b) in &chords[i + 1..] {
                    assert!(
                        !crate::plugins::shortcut::same_chord(c, d),
                        "{a:?} and {b:?} share a chord"
                    );
                }
            }
        }
    }

    #[test]
    fn saved_chords_replace_a_commands_defaults() {
        let saved = BTreeMap::from([
            (
                command_key(&Action::SaveAll),
                vec!["Ctrl+Shift+S".to_owned()],
            ),
            (
                command_key(&Action::FindInFiles),
                vec!["Ctrl+K F".to_owned(), "Ctrl+Shift+F".to_owned()],
            ),
        ]);
        let (singles, overrides, errors) = parse_overrides(&saved);
        assert!(errors.is_empty());
        let chords = chords_with(preset_chords(Keymap::Ettin), &overrides);
        assert!(!chords.iter().any(|(_, a)| *a == Action::SaveAll));
        let find = chords
            .iter()
            .find(|(_, a)| *a == Action::FindInFiles)
            .unwrap();
        assert_eq!(crate::plugins::shortcut::chord_to_text(&find.0), "Ctrl+K F");
        let (_, find_keys) = singles
            .iter()
            .find(|(a, _)| *a == Action::FindInFiles)
            .unwrap();
        assert_eq!(find_keys.len(), 1);
    }

    #[test]
    fn vs_code_style_changes_save_as() {
        let find = |keymap, action: &Action| -> Vec<KeyboardShortcut> {
            preset_shortcuts(keymap)
                .into_iter()
                .filter(|(_, a)| a == action)
                .map(|(s, _)| s)
                .collect()
        };
        let ctrl_shift_s = KeyboardShortcut::new(CMD_SHIFT, Key::S);
        assert!(crate::plugins::shortcut::same(
            &find(Keymap::VsCode, &Action::SaveAs)[0],
            &ctrl_shift_s
        ));
        assert!(crate::plugins::shortcut::same(
            &find(Keymap::Ettin, &Action::SaveAll)[0],
            &ctrl_shift_s
        ));
        assert!(find(Keymap::VsCode, &Action::Print).is_empty());
        // Every Ettin command keeps a place in the list or is listed as changed.
        assert_eq!(
            find(Keymap::VsCode, &Action::Find),
            find(Keymap::Ettin, &Action::Find)
        );
    }

    /// 2.0.1 renamed Pretty Print to Format XML; the command's id stays the
    /// same so saved shortcuts, toolbars and recipes keep working.
    #[test]
    fn format_xml_keeps_its_old_id() {
        let key = command_key(&Action::Xml(XmlOp::PrettyPrint));
        assert_eq!(key, r#"{"Xml":"PrettyPrint"}"#);
        let back: Action = serde_json::from_str(&key).unwrap();
        assert_eq!(back, Action::Xml(XmlOp::PrettyPrint));
    }
}
