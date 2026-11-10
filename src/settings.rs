//! User settings, stored as JSON in the platform config folder
//! (`~/.config/EttinEdit` on Linux, `%APPDATA%\EttinEdit` on Windows).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::macros::SavedMacro;
use crate::search::SearchOptions;

pub const MAX_RECENT: usize = 12;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The interface language, like `de`; empty follows the system.
    pub ui_language: String,
    pub theme: String,
    /// The editor font size, changed by zooming.
    pub font_size: f32,
    /// The size zooming resets to.
    pub default_font_size: f32,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
    pub highlight_current_line: bool,
    pub restore_session: bool,
    /// Tabs from before 0.5, read once when `session.json` doesn't exist yet.
    #[serde(skip_serializing)]
    pub session_files: Vec<PathBuf>,
    #[serde(skip_serializing)]
    pub session_active: usize,
    pub recent_files: Vec<PathBuf>,
    pub search: SearchOptions,
    pub fif_dir: String,
    pub fif_filters: String,
    pub fif_subfolders: bool,
    pub fif_hidden: bool,
    pub show_whitespace: bool,
    pub show_eol: bool,
    /// Label control characters, unusual spaces and zero-width characters.
    pub show_invisible: bool,
    pub indent_guides: bool,
    pub show_edge: bool,
    pub edge_column: usize,
    pub auto_close: bool,
    pub smart_indent: bool,
    pub use_spaces: bool,
    pub word_completion: bool,
    /// Suggest the language's keywords and functions too.
    pub language_completion: bool,
    /// Show a function's parameters while typing a call.
    pub parameter_hints: bool,
    pub macros: Vec<SavedMacro>,
    /// Opening a file while Ettin Edit runs sends it to the open window.
    pub single_instance: bool,
    /// Quit without asking about unsaved changes; they come back next time.
    pub remember_unsaved: bool,
    /// Reload files changed on disk without asking, if they have no unsaved changes.
    pub auto_reload: bool,
    /// Typing `>` after an opening tag in HTML or XML adds the closing tag.
    pub close_tags: bool,
    /// Check that XML is well-formed each time it's saved.
    pub xml_check_on_save: bool,
    /// The schema (XSD or DTD) each XML file was last validated against.
    pub xml_schemas: BTreeMap<String, PathBuf>,
    /// The last XSL stylesheet run, where the next one is picked from.
    pub last_xsl: Option<PathBuf>,
    /// Look for a newer release once a week.
    pub check_updates: bool,
    pub print_line_numbers: bool,
    /// Print with syntax colours (darkened for paper) rather than all black.
    pub print_colours: bool,
    /// When the last automatic update check ran (Unix seconds).
    pub last_update_check: i64,
    /// How Compare matches lines.
    pub compare: crate::compare::CompareOptions,
    /// Per plugin (by folder name): turned off, and which version was allowed to run.
    pub plugins: BTreeMap<String, PluginPrefs>,
    /// The Python and Node programs plugins run with; empty finds them on the PATH.
    pub python_path: String,
    pub node_path: String,
    /// Where Manage Plugins reads the list of plugins to install from.
    pub plugin_index_url: String,
    /// The editor font; the built-in one when no file is set.
    pub editor_font: crate::fonts::EditorFont,
    /// Columns between tab stops.
    pub tab_width: usize,
    /// Line ending for new documents; `None` uses the platform's.
    pub new_line_ending: Option<crate::document::LineEnding>,
    /// Encoding for new documents, by name; empty is UTF-8.
    pub new_encoding: String,
    /// Language for new documents.
    pub new_language: String,
    /// Shortcuts changed in the shortcut mapper: command key to shortcuts
    /// (an empty list removes the default ones).
    pub shortcuts: BTreeMap<String, Vec<String>>,
    pub function_list: bool,
    /// The character panel is open.
    pub char_panel: bool,
    pub doc_map: bool,
    pub folder_panel: bool,
    /// The folder shown in the folder panel.
    pub workspace: Option<PathBuf>,
    /// The toolbar under the menu bar (its layout is in `toolbar.json`).
    pub show_toolbar: bool,
    /// Only the toolbar's tab names show.
    pub toolbar_collapsed: bool,
    /// The full ribbon, or the simplified one-row toolbar.
    pub toolbar_mode: ToolbarMode,
    /// The File, Edit, View… menu bar. When it is hidden, tapping Alt shows it
    /// until a command runs.
    pub show_menu_bar: bool,
    /// Commands run with Transform > Filter Through Command, newest first.
    pub pipe_commands: Vec<String>,
    /// Recipes whose shell commands may run without asking, by
    /// [`crate::recipes::Recipe::fingerprint`].
    pub allowed_recipes: Vec<String>,
    /// The commands run last, newest first, by command key; the command
    /// palette lists them first.
    pub recent_commands: Vec<String>,
    /// The id of the recipe that ran last, for Run Last Recipe.
    pub last_recipe: Option<String>,
    /// Where each Transform dialog sent its result last time (This tab,
    /// New tab or Clipboard), by [`crate::transform_ui::DialogKind::key`].
    pub transform_results: BTreeMap<String, crate::transform_ui::Dest>,
    /// Show the suggestion bar ("This JSON is on one line. Format?").
    pub suggestions: bool,
    /// Suggestion kinds turned off with "Don't suggest this again", by
    /// [`crate::suggest::Kind::key`].
    pub dismissed_suggestions: Vec<String>,
    /// The version whose Welcome or What's new tab was shown last.
    pub welcome_shown_version: String,
    /// Open the Welcome tab every time Ettin Edit starts.
    pub show_welcome: bool,
    /// Switch between `light_theme` and `dark_theme` with the system's
    /// light/dark setting.
    pub follow_system_theme: bool,
    pub light_theme: String,
    pub dark_theme: String,
    /// The size of menus, buttons and dialogs, in percent (80–200).
    pub ui_scale: u32,
    /// Which set of default shortcuts to start from.
    pub keymap: Keymap,
    /// What a hash does with no selection.
    pub hash_whole_document: HashResult,
    /// Recipe command steps ask before they run (until the recipe changes).
    pub recipe_commands_ask: bool,
    /// The shell Filter Through Command and recipe command steps use, like
    /// `bash -c`; empty is `sh -c` (`cmd /C` on Windows).
    pub shell: String,
    /// The indentation Format JSON, Format XML and Convert to write.
    pub format_indent: FormatIndent,
    /// Tab width and tabs/spaces per language (by lower-case name), over
    /// `tab_width` and `use_spaces`.
    pub language_overrides: BTreeMap<String, LanguageIndent>,
    /// Look at how a file is indented when it opens and use that.
    pub detect_indent: bool,
    /// True when these settings were read from the settings file (false on
    /// a fresh profile).
    #[serde(skip)]
    pub loaded_from_file: bool,
}

/// How the toolbar is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolbarMode {
    /// Groups of large and small buttons with their names below.
    #[default]
    Full,
    /// One row of small buttons.
    Simplified,
}

/// The default shortcuts to start from; the user's own changes go on top.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Keymap {
    /// Ettin Edit's own keys, close to Notepad++.
    #[default]
    Ettin,
    /// Keys from Visual Studio Code where they differ.
    VsCode,
}

/// Where a hash of the whole document goes (with no selection).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashResult {
    #[default]
    Clipboard,
    NewTab,
}

/// The indentation formatters write.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormatIndent {
    /// What the document itself uses.
    #[default]
    Document,
    Two,
    Four,
    Tab,
}

impl FormatIndent {
    /// The indent text, or `None` to use the document's own.
    pub fn unit(self) -> Option<&'static str> {
        match self {
            FormatIndent::Document => None,
            FormatIndent::Two => Some("  "),
            FormatIndent::Four => Some("    "),
            FormatIndent::Tab => Some("\t"),
        }
    }
}

/// A language's own indentation; unset parts come from the general settings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageIndent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_tabs: Option<bool>,
}

/// Languages that indent their own way out of the box.
pub fn default_language_overrides() -> BTreeMap<String, LanguageIndent> {
    let spaces = |w| LanguageIndent {
        tab_width: Some(w),
        use_tabs: Some(false),
    };
    let tabs = LanguageIndent {
        tab_width: None,
        use_tabs: Some(true),
    };
    BTreeMap::from([
        ("json".to_owned(), spaces(2)),
        ("yaml".to_owned(), spaces(2)),
        ("makefile".to_owned(), tabs.clone()),
        ("go".to_owned(), tabs),
    ])
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginPrefs {
    pub disabled: bool,
    /// The fingerprint of the plugin's folder when the user allowed it to run.
    pub approved: Option<String>,
    /// Don't ask again when its files change (for plugins the user is writing).
    pub trust_changes: bool,
    /// Restart it when its files change (for plugins the user is writing).
    pub reload_on_change: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_language: String::new(),
            theme: "Ettin Dark".to_owned(),
            font_size: 14.0,
            default_font_size: 14.0,
            word_wrap: false,
            show_line_numbers: true,
            highlight_current_line: true,
            restore_session: true,
            session_files: Vec::new(),
            session_active: 0,
            recent_files: Vec::new(),
            search: SearchOptions {
                wrap: true,
                ..Default::default()
            },
            fif_dir: String::new(),
            fif_filters: String::new(),
            fif_subfolders: true,
            fif_hidden: false,
            show_whitespace: false,
            show_eol: false,
            show_invisible: false,
            indent_guides: true,
            show_edge: false,
            edge_column: 80,
            auto_close: true,
            smart_indent: true,
            use_spaces: false,
            word_completion: true,
            language_completion: true,
            parameter_hints: true,
            macros: Vec::new(),
            single_instance: true,
            remember_unsaved: true,
            auto_reload: false,
            close_tags: true,
            xml_check_on_save: false,
            xml_schemas: BTreeMap::new(),
            last_xsl: None,
            check_updates: true,
            print_line_numbers: true,
            print_colours: true,
            last_update_check: 0,
            compare: crate::compare::CompareOptions::default(),
            plugins: BTreeMap::new(),
            python_path: String::new(),
            node_path: String::new(),
            plugin_index_url: crate::plugins::store::DEFAULT_INDEX_URL.to_owned(),
            editor_font: crate::fonts::EditorFont::default(),
            tab_width: crate::editor::layout::DEFAULT_TAB_WIDTH,
            new_line_ending: None,
            new_encoding: String::new(),
            new_language: crate::highlight::PLAIN_TEXT.to_owned(),
            shortcuts: BTreeMap::new(),
            function_list: false,
            char_panel: false,
            doc_map: false,
            folder_panel: false,
            workspace: None,
            show_toolbar: true,
            toolbar_collapsed: false,
            toolbar_mode: ToolbarMode::Full,
            show_menu_bar: true,
            pipe_commands: Vec::new(),
            allowed_recipes: Vec::new(),
            recent_commands: Vec::new(),
            last_recipe: None,
            transform_results: BTreeMap::new(),
            suggestions: true,
            dismissed_suggestions: Vec::new(),
            welcome_shown_version: String::new(),
            show_welcome: false,
            follow_system_theme: false,
            light_theme: "Ettin Light".to_owned(),
            dark_theme: "Ettin Dark".to_owned(),
            ui_scale: 100,
            keymap: Keymap::Ettin,
            hash_whole_document: HashResult::Clipboard,
            recipe_commands_ask: true,
            shell: String::new(),
            format_indent: FormatIndent::Document,
            language_overrides: default_language_overrides(),
            detect_indent: true,
            loaded_from_file: false,
        }
    }
}

/// The Ettin Edit folder inside the platform config directory.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("EttinEdit"))
}

/// Before the rename to Ettin Edit, settings lived in a `NixPad` folder.
const OLD_DIR_NAME: &str = "NixPad";

/// Copies settings, session, themes and plugins from the old `NixPad` folder
/// the first time Ettin Edit starts. The old folder is left as it was.
pub fn migrate_old_config() {
    let (Some(new), Some(old)) = (
        config_dir(),
        dirs::config_dir().map(|d| d.join(OLD_DIR_NAME)),
    ) else {
        return;
    };
    if new.exists() || !old.is_dir() {
        return;
    }
    if let Err(e) = copy_dir(&old, &new) {
        eprintln!(
            "ettin-edit: could not copy settings from {}: {e}",
            old.display()
        );
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        // The running-instance file and the plugin SDK belong to the old app.
        if name == "instance.json" || name == ".sdk" {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir(&entry.path(), &to.join(&name))?;
        } else if kind.is_file() {
            fs::copy(entry.path(), to.join(&name))?;
        }
    }
    Ok(())
}

/// Theme names from before the rename.
fn renamed_theme(name: &str) -> Option<&'static str> {
    match name {
        "NixPad Dark" | "Eighties" => Some("Ettin Dark"),
        "NixPad Light" => Some("Ettin Light"),
        _ => None,
    }
}

pub fn themes_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("themes"))
}

/// The settings file's name, in [`config_dir`].
pub const FILE_NAME: &str = "settings.json";

pub fn settings_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(FILE_NAME))
}

impl Settings {
    pub fn load() -> Self {
        settings_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| Self::from_json(&s).ok())
            .unwrap_or_default()
    }

    /// Reads settings from the text of a settings file.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut s = serde_json::from_str::<Self>(text).map_err(|e| e.to_string())?;
        if let Some(name) = renamed_theme(&s.theme) {
            s.theme = name.to_owned();
        }
        s.loaded_from_file = true;
        Ok(s)
    }

    /// The settings as the settings file holds them.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Writes the settings file, unless it already holds these settings.
    pub fn save(&self) {
        let Some(path) = settings_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let json = self.to_json();
        if json.is_empty() {
            return;
        }
        if fs::read(&path).is_ok_and(|old| old == json.as_bytes()) {
            crate::watch::note_write(&path, json.as_bytes());
            return;
        }
        if let Err(e) = crate::watch::write(&path, &json) {
            eprintln!(
                "ettin-edit: could not save settings to {}: {e}",
                path.display()
            );
        }
    }

    pub fn push_recent(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT);
    }
}
