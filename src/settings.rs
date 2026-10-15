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
    pub indent_guides: bool,
    pub show_edge: bool,
    pub edge_column: usize,
    pub auto_close: bool,
    pub smart_indent: bool,
    pub use_spaces: bool,
    pub word_completion: bool,
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
    pub doc_map: bool,
    pub folder_panel: bool,
    /// The folder shown in the folder panel.
    pub workspace: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginPrefs {
    pub disabled: bool,
    /// The fingerprint of the plugin's folder when the user allowed it to run.
    pub approved: Option<String>,
    /// Don't ask again when its files change (for plugins the user is writing).
    pub trust_changes: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
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
            indent_guides: true,
            show_edge: false,
            edge_column: 80,
            auto_close: true,
            smart_indent: true,
            use_spaces: false,
            word_completion: true,
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
            editor_font: crate::fonts::EditorFont::default(),
            tab_width: crate::editor::layout::DEFAULT_TAB_WIDTH,
            new_line_ending: None,
            new_encoding: String::new(),
            new_language: crate::highlight::PLAIN_TEXT.to_owned(),
            shortcuts: BTreeMap::new(),
            function_list: false,
            doc_map: false,
            folder_panel: false,
            workspace: None,
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

fn settings_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        settings_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .map(|mut s| {
                if let Some(name) = renamed_theme(&s.theme) {
                    s.theme = name.to_owned();
                }
                s
            })
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = settings_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!(
                        "ettin-edit: could not save settings to {}: {e}",
                        path.display()
                    );
                }
            }
            Err(e) => eprintln!("ettin-edit: could not serialise settings: {e}"),
        }
    }

    pub fn push_recent(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT);
    }
}
