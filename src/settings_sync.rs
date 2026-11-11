//! Settings > Export Settings… and Import Settings…: one `.ettin-settings.zip`
//! with the settings (shortcuts included), the toolbar, themes, recipes and
//! languages. Plugins are listed by id and version, not copied.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The end of an export's file name.
pub const EXTENSION: &str = "ettin-settings.zip";

/// Single files in the config folder that are copied.
const FILES: [&str; 2] = [crate::settings::FILE_NAME, crate::ribbon::FILE_NAME];
/// Config folders whose files are copied (not their subfolders).
pub const FOLDERS: [&str; 3] = ["themes", "recipes", "languages"];
/// What the export says about itself.
const MANIFEST: &str = "ettin-settings.json";

/// A plugin that was installed where the settings were exported.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginRef {
    pub id: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Manifest {
    /// The Ettin Edit version that wrote it.
    version: String,
    plugins: Vec<PluginRef>,
}

/// An export read back: the files it holds, by their path in the config
/// folder (like `themes/Mine.tmTheme`).
pub struct Archive {
    pub files: Vec<(String, Vec<u8>)>,
    pub plugins: Vec<PluginRef>,
    /// The version that exported it.
    pub version: String,
}

/// True for a path an export may hold: one of [`FILES`], or a plain file
/// name inside one of [`FOLDERS`].
fn allowed(name: &str) -> bool {
    if FILES.contains(&name) {
        return true;
    }
    let Some((folder, file)) = name.split_once('/') else {
        return false;
    };
    FOLDERS.contains(&folder)
        && !file.is_empty()
        && !file.contains(['/', '\\', ':'])
        && file != "."
        && file != ".."
}

/// Writes the export to `out`. `settings_json` is the settings as they are
/// now (they may not be saved yet). Returns how many files went in.
pub fn export(
    config: &Path,
    settings_json: &str,
    plugins: &[PluginRef],
    out: &Path,
) -> Result<usize, String> {
    let file = std::fs::File::create(out).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut put = |name: &str, bytes: &[u8]| -> Result<(), String> {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(bytes).map_err(|e| e.to_string())
    };
    let manifest = Manifest {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        plugins: plugins.to_vec(),
    };
    put(
        MANIFEST,
        serde_json::to_string_pretty(&manifest)
            .unwrap_or_default()
            .as_bytes(),
    )?;
    let mut count = 0;
    put(crate::settings::FILE_NAME, settings_json.as_bytes())?;
    count += 1;
    if let Ok(bytes) = std::fs::read(config.join(crate::ribbon::FILE_NAME)) {
        put(crate::ribbon::FILE_NAME, &bytes)?;
        count += 1;
    }
    for folder in FOLDERS {
        let Ok(entries) = std::fs::read_dir(config.join(folder)) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        paths.sort();
        for path in paths {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            put(&format!("{folder}/{name}"), &bytes)?;
            count += 1;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(count)
}

/// Reads an export. Anything in it that isn't a settings file is skipped.
pub fn read(path: &Path) -> Result<Archive, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    let mut manifest = None;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().map_err(|e| e.to_string())?.into_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if name == MANIFEST {
            manifest = serde_json::from_slice::<Manifest>(&bytes).ok();
        } else if allowed(&name) {
            files.push((name, bytes));
        }
    }
    let Some(manifest) = manifest else {
        return Err(t!("import-not-settings"));
    };
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(Archive {
        files,
        plugins: manifest.plugins,
        version: manifest.version,
    })
}

impl Archive {
    /// The files that already exist in `config` and would be replaced.
    pub fn replaces(&self, config: &Path) -> Vec<String> {
        self.files
            .iter()
            .filter(|(name, _)| config.join(name).exists())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Copies the files into `config`, first putting the files they replace
    /// in a new folder under `config/backups`. Returns that folder (`None`
    /// when nothing was replaced).
    pub fn apply(&self, config: &Path) -> Result<Option<PathBuf>, String> {
        let replaced = self.replaces(config);
        let backup = if replaced.is_empty() {
            None
        } else {
            let stamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S").to_string();
            let mut dir = config.join("backups").join(&stamp);
            let mut n = 2;
            while dir.exists() {
                dir = config.join("backups").join(format!("{stamp}-{n}"));
                n += 1;
            }
            for name in &replaced {
                let to = dir.join(name);
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::copy(config.join(name), &to)
                    .map_err(|e| format!("{}: {e}", to.display()))?;
            }
            Some(dir)
        };
        for (name, bytes) in &self.files {
            let to = config.join(name);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            crate::watch::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
        }
        Ok(backup)
    }

    /// How many files of each kind it holds: settings files, themes,
    /// recipes, languages.
    pub fn counts(&self) -> [usize; 4] {
        let mut c = [0; 4];
        for (name, _) in &self.files {
            let i = match name.split_once('/') {
                None => 0,
                Some(("themes", _)) => 1,
                Some(("recipes", _)) => 2,
                Some(_) => 3,
            };
            c[i] += 1;
        }
        c
    }
}

/// A file name for a new export, like `Ettin Edit settings 2026-11-10.ettin-settings.zip`.
pub fn default_file_name() -> String {
    format!(
        "Ettin Edit settings {}.{EXTENSION}",
        chrono::Local::now().format("%Y-%m-%d")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ettin-sync-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn export_then_import_on_a_fresh_profile() {
        let from = temp("from");
        std::fs::write(from.join("toolbar.json"), "{\"version\":7}").unwrap();
        for (folder, file, text) in [
            ("themes", "Mine.tmTheme", "<plist/>"),
            ("recipes", "clean.json", "{\"name\":\"Clean\"}"),
            ("languages", "gcode.lang.json", "{\"name\":\"Gcode\"}"),
        ] {
            std::fs::create_dir_all(from.join(folder)).unwrap();
            std::fs::write(from.join(folder).join(file), text).unwrap();
        }
        // Subfolders and other files stay behind.
        std::fs::create_dir_all(from.join("recipes").join("old")).unwrap();
        std::fs::write(from.join("session.json"), "{}").unwrap();
        let settings = crate::settings::Settings {
            theme: "Ocean Dark".to_owned(),
            shortcuts: [("\"Save\"".to_owned(), vec!["Ctrl+Alt+S".to_owned()])].into(),
            ..Default::default()
        };
        let zip = from.join(default_file_name());
        let plugins = vec![PluginRef {
            id: "hl7".into(),
            version: "1.0".into(),
        }];
        let count = export(&from, &settings.to_json(), &plugins, &zip).unwrap();
        assert_eq!(count, 5);

        let to = temp("to");
        let archive = read(&zip).unwrap();
        assert_eq!(archive.counts(), [2, 1, 1, 1]);
        assert_eq!(archive.plugins, plugins);
        assert!(archive.replaces(&to).is_empty());
        assert_eq!(archive.apply(&to).unwrap(), None);
        let back = crate::settings::Settings::from_json(
            &std::fs::read_to_string(to.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(back.theme, "Ocean Dark");
        assert_eq!(back.shortcuts, settings.shortcuts);
        for name in [
            "toolbar.json",
            "themes/Mine.tmTheme",
            "recipes/clean.json",
            "languages/gcode.lang.json",
        ] {
            assert_eq!(
                std::fs::read(to.join(name)).unwrap(),
                std::fs::read(from.join(name)).unwrap(),
                "{name}"
            );
        }
        assert!(!to.join("session.json").exists());

        // Importing again replaces those files and backs them up first.
        std::fs::write(to.join("themes/Mine.tmTheme"), "changed").unwrap();
        assert_eq!(archive.replaces(&to).len(), 5);
        let backup = archive.apply(&to).unwrap().unwrap();
        assert_eq!(
            std::fs::read_to_string(backup.join("themes/Mine.tmTheme")).unwrap(),
            "changed"
        );
        assert_eq!(
            std::fs::read_to_string(to.join("themes/Mine.tmTheme")).unwrap(),
            "<plist/>"
        );
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_dir_all(&to);
    }

    #[test]
    fn only_settings_paths_are_taken() {
        assert!(allowed("settings.json"));
        assert!(allowed("themes/Mine.tmTheme"));
        assert!(!allowed("session.json"));
        assert!(!allowed("themes/../settings.json"));
        assert!(!allowed("plugins/x/main.py"));
        assert!(!allowed("recipes/a/b.json"));
        assert!(!allowed("../evil"));
    }

    #[test]
    fn other_zips_are_refused() {
        let dir = temp("other");
        let path = dir.join("x.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file("settings.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"{}").unwrap();
        zip.finish().unwrap();
        assert!(read(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
