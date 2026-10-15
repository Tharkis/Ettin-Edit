//! The Language menu's commands for user-defined languages.

use super::EttinApp;
use crate::highlight::languages_dir;
use crate::system;

/// The language guide, opened from Language > Language Guide.
const GUIDE: &str = include_str!("../../docs/languages.md");

impl EttinApp {
    /// Reads the grammars again, including the languages folder.
    pub(super) fn reload_languages(&mut self) {
        let errors = self.highlighter.reload(languages_dir().as_deref());
        self.errors.extend(errors);
        self.languages = self.highlighter.language_names();
        // Highlighting caches belong to the old grammars; reloading the themes drops them.
        self.reload_themes();
        for doc in &mut self.docs {
            if !self.languages.contains(&doc.syntax) {
                doc.syntax = crate::highlight::PLAIN_TEXT.to_owned();
            }
        }
    }

    pub(super) fn open_languages_folder(&mut self) {
        let Some(dir) = languages_dir() else {
            return;
        };
        let result = std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Couldn't make {}: {e}", dir.display()))
            .and_then(|_| system::open_folder(&dir));
        if let Err(e) = result {
            self.errors.push(e);
        }
    }

    /// Starts a new language from the template, in the languages folder.
    pub(super) fn new_language(&mut self) {
        let Some(dir) = languages_dir() else {
            return;
        };
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.errors
                .push(format!("Couldn't make {}: {e}", dir.display()));
            return;
        }
        let path = (1..)
            .map(|n| {
                let name = if n == 1 {
                    "my-language.lang.json".to_owned()
                } else {
                    format!("my-language-{n}.lang.json")
                };
                dir.join(name)
            })
            .find(|p| !p.exists())
            .expect("an unused name");
        if let Err(e) = std::fs::write(&path, crate::user_lang::UDL_TEMPLATE) {
            self.errors
                .push(format!("Couldn't write {}: {e}", path.display()));
            return;
        }
        self.open_path(&path, true);
    }

    /// Opens the language guide as a read-only tab.
    pub(super) fn language_guide(&mut self) {
        let Some(dir) = languages_dir() else {
            return;
        };
        let path = dir.join("Language Guide.md");
        let written = std::fs::create_dir_all(&dir).and_then(|_| {
            if std::fs::read_to_string(&path).ok().as_deref() == Some(GUIDE) {
                Ok(())
            } else {
                std::fs::write(&path, GUIDE)
            }
        });
        if let Err(e) = written {
            self.errors
                .push(format!("Couldn't write {}: {e}", path.display()));
            return;
        }
        self.open_path(&path, false);
        if let Some(doc) = self.docs.get_mut(self.active)
            && doc.path.as_deref() == Some(path.as_path())
        {
            doc.editor.buffer.read_only = true;
        }
    }
}
