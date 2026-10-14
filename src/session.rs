//! The session: which tabs were open, and backups of their unsaved changes,
//! so work survives a restart or a crash. Kept in the config folder as
//! `session.json` plus one file per backed-up tab in `backup/`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::document::LineEnding;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionTab {
    /// The file, or `None` for a new, unsaved tab.
    pub path: Option<PathBuf>,
    /// File name inside the backup folder holding unsaved text.
    #[serde(default)]
    pub backup: Option<String>,
    #[serde(default)]
    pub untitled_number: usize,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub line_ending: Option<LineEnding>,
    #[serde(default)]
    pub syntax: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub read_only: bool,
    /// Caret position in characters.
    #[serde(default)]
    pub caret: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub tabs: Vec<SessionTab>,
    pub active: usize,
}

pub struct SessionStore {
    dir: PathBuf,
    /// What was last written, to skip writing the same thing again.
    last_written: Option<String>,
}

impl SessionStore {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            dir: config_dir.to_path_buf(),
            last_written: None,
        }
    }

    fn session_path(&self) -> PathBuf {
        self.dir.join("session.json")
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.dir.join("backup")
    }

    pub fn load(&self) -> Option<Session> {
        let text = fs::read_to_string(self.session_path()).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&mut self, session: &Session) -> io::Result<()> {
        let json = serde_json::to_string_pretty(session).map_err(io::Error::other)?;
        if self.last_written.as_deref() == Some(json.as_str()) {
            return Ok(());
        }
        fs::create_dir_all(&self.dir)?;
        write_atomic(&self.session_path(), json.as_bytes())?;
        self.last_written = Some(json);
        Ok(())
    }

    /// A new, unused backup file name for a tab titled `title`.
    pub fn new_backup_path(&self, title: &str) -> PathBuf {
        let safe: String = title
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || "._- ".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .take(60)
            .collect();
        let stamp = chrono::Local::now().format("%Y-%m-%d_%H%M%S");
        let dir = self.backup_dir();
        let mut n = 0;
        loop {
            let name = if n == 0 {
                format!("{safe}@{stamp}.txt")
            } else {
                format!("{safe}@{stamp}-{n}.txt")
            };
            let path = dir.join(name);
            if !path.exists() {
                return path;
            }
            n += 1;
        }
    }

    /// Writes a backup of `text` (always UTF-8 with `\n` line endings).
    pub fn write_backup<'a>(
        &self,
        path: &Path,
        chunks: impl Iterator<Item = &'a str>,
    ) -> io::Result<()> {
        fs::create_dir_all(self.backup_dir())?;
        let mut data = Vec::new();
        for c in chunks {
            data.extend_from_slice(c.as_bytes());
        }
        write_atomic(path, &data)
    }

    pub fn read_backup(&self, name: &str) -> io::Result<String> {
        let bytes = fs::read(self.backup_dir().join(name))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Deletes backups that no tab in `keep` refers to.
    pub fn remove_stale_backups(&self, keep: &[PathBuf]) {
        let Ok(entries) = fs::read_dir(self.backup_dir()) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && !keep.contains(&path) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

/// Writes to a temporary file and renames it over `path`, so a crash
/// mid-write never leaves a half-written file behind.
fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_sessions_and_backups() {
        let dir = std::env::temp_dir().join(format!("nixpad-session-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut store = SessionStore::new(&dir);
        assert_eq!(store.load(), None);

        let backup = store.new_backup_path("new 1");
        store
            .write_backup(&backup, ["héllo\n", "world"].into_iter())
            .unwrap();
        let name = backup.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("new 1@"));
        let session = Session {
            tabs: vec![SessionTab {
                path: None,
                backup: Some(name.clone()),
                untitled_number: 1,
                encoding: Some("UTF-8".into()),
                line_ending: Some(LineEnding::CrLf),
                syntax: None,
                pinned: true,
                read_only: false,
                caret: 3,
            }],
            active: 0,
        };
        store.save(&session).unwrap();
        assert_eq!(store.load(), Some(session));
        assert_eq!(store.read_backup(&name).unwrap(), "héllo\nworld");

        // A different name is picked while the first one exists.
        assert_ne!(store.new_backup_path("new 1"), backup);
        store.remove_stale_backups(&[]);
        assert!(!backup.exists());
        fs::remove_dir_all(&dir).ok();
    }
}
