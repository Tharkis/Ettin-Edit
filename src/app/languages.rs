//! The Language menu's commands for user-defined languages.

use super::EttinApp;
use crate::highlight::languages_dir;
use crate::system;

impl EttinApp {
    /// Reads the grammars again, including the languages folder.
    pub(super) fn reload_languages(&mut self) {
        let errors = self.highlighter.reload(languages_dir().as_deref());
        self.errors.extend(errors);
        let (apis, errors) = crate::lang_api::ApiLibrary::load(
            crate::lang_api::api_dir().as_deref(),
            self.highlighter.grammar_words.clone(),
        );
        self.apis = apis;
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

    /// Opens the folder of the user's keyword and function lists, making it
    /// with a commented example the first time.
    pub(super) fn open_api_folder(&mut self) {
        let Some(dir) = crate::lang_api::api_dir() else {
            return;
        };
        let example = dir.join("example.json.txt");
        let result = std::fs::create_dir_all(&dir)
            .and_then(|_| {
                if example.exists() {
                    Ok(())
                } else {
                    std::fs::write(&example, crate::lang_api::API_TEMPLATE)
                }
            })
            .map_err(|e| format!("Couldn't make {}: {e}", dir.display()))
            .and_then(|_| system::open_folder(&dir));
        if let Err(e) = result {
            self.errors.push(e);
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
}
