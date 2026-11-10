//! What Ettin Edit keeps for each plugin outside its folder: its settings
//! (`ettin.settings`) and a folder for its own files (`ettin.data_dir()`).
//! They live in `<config>/plugin-data/<id>/`, so updating or reinstalling
//! the plugin keeps them.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::manifest::PluginOption;

pub const SETTINGS_FILE: &str = "settings.json";

/// `~/.config/EttinEdit/plugin-data` on Linux.
pub fn data_root() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("plugin-data"))
}

/// The settings saved in `dir` (a plugin's data folder), with the defaults
/// of the manifest's options filled in and values of the wrong kind put
/// back to the default.
pub fn load_settings(dir: &Path, options: &[PluginOption]) -> Map<String, Value> {
    let mut saved: Map<String, Value> = fs::read_to_string(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    for o in options {
        let value = saved
            .get(&o.key)
            .map_or_else(|| o.default.clone(), |v| o.fit(v));
        saved.insert(o.key.clone(), value);
    }
    saved
}

/// Writes a plugin's settings to its data folder.
pub fn save_settings(dir: &Path, values: &Map<String, Value>) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(values).map_err(|e| e.to_string())? + "\n";
    fs::write(dir.join(SETTINGS_FILE), text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::OptionKind;
    use serde_json::json;

    #[test]
    fn settings_round_trip_with_defaults() {
        let dir = std::env::temp_dir().join(format!("ettin-data-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let options = vec![PluginOption {
            key: "sep".into(),
            title: "Separator".into(),
            kind: OptionKind::String,
            default: json!("|"),
            choices: Vec::new(),
            description: String::new(),
        }];
        let values = load_settings(&dir, &options);
        assert_eq!(Value::Object(values.clone()), json!({"sep": "|"}));
        let mut values = values;
        values.insert("sep".into(), json!(5));
        values.insert("seen".into(), json!(true));
        save_settings(&dir, &values).unwrap();
        // A plugin's own keys stay; a wrong kind goes back to the default.
        let back = load_settings(&dir, &options);
        assert_eq!(Value::Object(back), json!({"sep": "|", "seen": true}));
        let _ = fs::remove_dir_all(&dir);
    }
}
