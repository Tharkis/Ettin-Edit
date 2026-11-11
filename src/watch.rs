//! Notices changes to the settings files while Ettin Edit runs: the themes,
//! languages, translations and recipes folders, and `settings.json`.
//!
//! Changes are collected for [`DEBOUNCE`] seconds (editors often save a
//! file in several steps) before they are handed to the app. Files the app
//! wrote itself are recognised by their content and left out.

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

/// Seconds a file must stay quiet before it is reloaded.
pub const DEBOUNCE: f64 = 0.3;

/// What a changed file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Settings,
    Theme,
    Language,
    Translation,
    Recipe,
}

/// The folders watched, each with what its files are.
pub fn watched_dirs(config: &Path) -> Vec<(PathBuf, Kind)> {
    vec![
        (config.join("themes"), Kind::Theme),
        (config.join("languages"), Kind::Language),
        (config.join("translations"), Kind::Translation),
        (config.join("recipes"), Kind::Recipe),
    ]
}

/// What `path` is, if it's a file the app reloads.
pub fn classify(config: &Path, path: &Path) -> Option<Kind> {
    let name = path.file_name()?.to_str()?.to_lowercase();
    if path.parent() == Some(config) {
        return (name == crate::settings::FILE_NAME).then_some(Kind::Settings);
    }
    let kind = watched_dirs(config)
        .into_iter()
        .find(|(dir, _)| path.parent() == Some(dir.as_path()))
        .map(|(_, kind)| kind)?;
    let wanted = match kind {
        Kind::Theme => name.ends_with(".tmtheme"),
        Kind::Language => name.ends_with(".lang.json") || name.ends_with(".sublime-syntax"),
        Kind::Translation => name.ends_with(".ftl"),
        Kind::Recipe => name.ends_with(".json"),
        Kind::Settings => false,
    };
    wanted.then_some(kind)
}

fn hash(bytes: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

/// The content of the files the app wrote last, by path.
static OWN_WRITES: Mutex<Option<HashMap<PathBuf, u64>>> = Mutex::new(None);

/// Remembers that the app wrote `contents` to `path`, so the change isn't
/// read back as the user's.
pub fn note_write(path: &Path, contents: &[u8]) {
    let mut own = OWN_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    own.get_or_insert_with(HashMap::new)
        .insert(path.to_path_buf(), hash(contents));
}

/// Writes a file and remembers it as the app's own write.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let contents = contents.as_ref();
    note_write(path, contents);
    std::fs::write(path, contents)
}

/// True when `contents` is what the app last wrote to `path`.
pub fn is_own(path: &Path, contents: &[u8]) -> bool {
    let own = OWN_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    own.as_ref()
        .and_then(|m| m.get(path))
        .is_some_and(|&h| h == hash(contents))
}

/// True when the app wrote `path` before and the file now holds something
/// else (or is gone).
pub fn changed_since_own_write(path: &Path) -> bool {
    let known = {
        let own = OWN_WRITES.lock().unwrap_or_else(|e| e.into_inner());
        own.as_ref().and_then(|m| m.get(path)).copied()
    };
    match known {
        None => false,
        Some(h) => std::fs::read(path).map(|b| hash(&b) != h).unwrap_or(false),
    }
}

/// The running watcher.
pub struct Watcher {
    config: PathBuf,
    // Dropping it stops watching.
    _inner: Option<notify::RecommendedWatcher>,
    rx: Receiver<PathBuf>,
    /// Changed files and when they last changed (egui time).
    pending: BTreeMap<PathBuf, f64>,
}

impl Watcher {
    /// Starts watching `config` (only `settings.json` in it) and the folders
    /// in [`watched_dirs`], making those that don't exist yet. `wake` is
    /// called on each change. Problems are returned, not fatal: the app
    /// still works, only without live reload.
    pub fn start(config: &Path, wake: Arc<dyn Fn() + Send + Sync>) -> (Self, Option<String>) {
        use notify::{RecursiveMode, Watcher as _};
        let (tx, rx) = channel();
        let handler = move |event: notify::Result<notify::Event>| {
            let Ok(event) = event else { return };
            if matches!(event.kind, notify::EventKind::Access(_)) {
                return;
            }
            for path in event.paths {
                let _ = tx.send(path);
            }
            wake();
        };
        let mut error = None;
        let inner = match notify::recommended_watcher(handler) {
            Ok(mut w) => {
                let _ = std::fs::create_dir_all(config);
                let mut watch = |dir: &Path| {
                    if let Err(e) = w.watch(dir, RecursiveMode::NonRecursive) {
                        error.get_or_insert(format!("{}: {e}", dir.display()));
                    }
                };
                watch(config);
                for (dir, _) in watched_dirs(config) {
                    let _ = std::fs::create_dir_all(&dir);
                    watch(&dir);
                }
                Some(w)
            }
            Err(e) => {
                error = Some(e.to_string());
                None
            }
        };
        (
            Self {
                config: config.to_path_buf(),
                _inner: inner,
                rx,
                pending: BTreeMap::new(),
            },
            error,
        )
    }

    /// The files that changed and have been quiet for [`DEBOUNCE`] seconds,
    /// leaving out the app's own writes. Also says whether more are waiting
    /// (call again shortly).
    pub fn take_ready(&mut self, now: f64) -> (Vec<(Kind, PathBuf)>, bool) {
        while let Ok(path) = self.rx.try_recv() {
            if classify(&self.config, &path).is_some() {
                self.pending.insert(path, now);
            }
        }
        let ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, t)| now - **t >= DEBOUNCE)
            .map(|(p, _)| p.clone())
            .collect();
        let mut out = Vec::new();
        for path in ready {
            self.pending.remove(&path);
            if let Ok(bytes) = std::fs::read(&path)
                && is_own(&path, &bytes)
            {
                continue;
            }
            if let Some(kind) = classify(&self.config, &path) {
                out.push((kind, path));
            }
        }
        out.sort();
        (out, !self.pending.is_empty())
    }
}

/// The line and column in an error message like serde's "… at line 12
/// column 5", if it has them.
pub fn error_place(message: &str) -> Option<(usize, usize)> {
    let re = regex::Regex::new(r"(?i)line:? (\d+)(?:,? col(?:umn)?:? (\d+))?").ok()?;
    let caps = re.captures(message)?;
    let line = caps.get(1)?.as_str().parse().ok()?;
    let col = caps
        .get(2)
        .and_then(|c| c.as_str().parse().ok())
        .unwrap_or(1);
    Some((line, col))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_settings_files() {
        let config = Path::new("/cfg");
        assert_eq!(
            classify(config, Path::new("/cfg/settings.json")),
            Some(Kind::Settings)
        );
        assert_eq!(classify(config, Path::new("/cfg/session.json")), None);
        assert_eq!(
            classify(config, Path::new("/cfg/themes/Mine.tmTheme")),
            Some(Kind::Theme)
        );
        assert_eq!(
            classify(config, Path::new("/cfg/recipes/clean.recipe.json")),
            Some(Kind::Recipe)
        );
        assert_eq!(
            classify(config, Path::new("/cfg/languages/gcode.lang.json")),
            Some(Kind::Language)
        );
        assert_eq!(
            classify(config, Path::new("/cfg/translations/nl.ftl")),
            Some(Kind::Translation)
        );
        assert_eq!(classify(config, Path::new("/cfg/themes/notes.txt")), None);
        assert_eq!(
            classify(config, Path::new("/cfg/backups/settings.json")),
            None
        );
    }

    #[test]
    fn finds_the_place_in_error_messages() {
        assert_eq!(
            error_place("expected `,` at line 12 column 5"),
            Some((12, 5))
        );
        assert_eq!(error_place("bad thing on line 3"), Some((3, 1)));
        assert_eq!(error_place("no place"), None);
    }

    #[test]
    fn own_writes_are_recognised() {
        let dir = std::env::temp_dir().join(format!("ettin-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        write(&path, "{}").unwrap();
        assert!(is_own(&path, b"{}"));
        assert!(!changed_since_own_write(&path));
        std::fs::write(&path, "{\"theme\": \"x\"}").unwrap();
        assert!(!is_own(&path, b"{\"theme\": \"x\"}"));
        assert!(changed_since_own_write(&path));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file written in the watched folder arrives once, after the pause.
    #[test]
    fn reports_changes_after_the_pause() {
        let config = std::env::temp_dir().join(format!("ettin-watch-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&config);
        let (mut watcher, error) = Watcher::start(&config, Arc::new(|| {}));
        assert!(error.is_none(), "{error:?}");
        let theme = config.join("themes").join("Mine.tmTheme");
        std::fs::write(&theme, "x").unwrap();
        let mut found = Vec::new();
        for i in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let now = i as f64 * 0.05;
            let (ready, _) = watcher.take_ready(now);
            found.extend(ready);
            if !found.is_empty() {
                break;
            }
        }
        assert_eq!(found, vec![(Kind::Theme, theme.clone())]);
        // The app's own write is not reported.
        write(&theme, "y").unwrap();
        let mut again = Vec::new();
        for i in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let (ready, _) = watcher.take_ready(10.0 + i as f64 * 0.05);
            again.extend(ready);
        }
        assert!(again.is_empty(), "{again:?}");
        let _ = std::fs::remove_dir_all(&config);
    }
}
