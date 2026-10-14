//! User settings, stored as JSON in the platform config folder
//! (`~/.config/NixPad` on Linux, `%APPDATA%\NixPad` on Windows).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::macros::SavedMacro;
use crate::search::SearchOptions;

pub const MAX_RECENT: usize = 12;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub font_size: f32,
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
    /// Opening a file while NixPad runs sends it to the open window.
    pub single_instance: bool,
    /// Quit without asking about unsaved changes; they come back next time.
    pub remember_unsaved: bool,
    /// Reload files changed on disk without asking, if they have no unsaved changes.
    pub auto_reload: bool,
    /// Typing `>` after an opening tag in HTML or XML adds the closing tag.
    pub close_tags: bool,
    /// Check that XML is well-formed each time it's saved.
    pub xml_check_on_save: bool,
    /// How Compare matches lines.
    pub compare: crate::compare::CompareOptions,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "NixPad Dark".to_owned(),
            font_size: 14.0,
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
            compare: crate::compare::CompareOptions::default(),
        }
    }
}

/// The NixPad folder inside the platform config directory.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("NixPad"))
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
            .and_then(|s| serde_json::from_str(&s).ok())
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
                    eprintln!("nixpad: could not save settings to {}: {e}", path.display());
                }
            }
            Err(e) => eprintln!("nixpad: could not serialise settings: {e}"),
        }
    }

    pub fn push_recent(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT);
    }
}
