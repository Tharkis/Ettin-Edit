//! Translations of the interface. Every piece of text the app shows is looked
//! up by id in a Fluent file (`assets/i18n/<code>.ftl`) with [`t!`]. English is
//! the source and the fallback: a message missing from a translation is shown
//! in English.
//!
//! Users can add or change translations by putting `<code>.ftl` files in the
//! `translations` folder of the config folder. A file there for a language
//! that is built in only needs the messages it changes.

use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

/// The English messages: the source every translation is made from.
pub const SOURCE: &str = include_str!("../assets/i18n/en.ftl");

/// The translations that come with Ettin Edit: (language code, messages).
pub const BUILT_IN: &[(&str, &str)] = &[
    ("en", SOURCE),
    ("de", include_str!("../assets/i18n/de.ftl")),
    ("es", include_str!("../assets/i18n/es.ftl")),
    ("fr", include_str!("../assets/i18n/fr.ftl")),
];

/// Text to fill into a message's `{ $name }` placeables.
pub type Args<'a> = FluentArgs<'a>;

/// Looks up a message by id: `t!("save-as")`, or with values for its
/// placeables: `t!("open-failed", path = name, error = e.to_string())`.
#[macro_export]
macro_rules! t {
    ($id:literal) => {
        $crate::i18n::tr($id)
    };
    ($id:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = $crate::i18n::Args::new();
        $( args.set(stringify!($name), $value); )+
        $crate::i18n::tr_args($id, &args)
    }};
}

/// A language the interface can be shown in.
#[derive(Clone, Debug, PartialEq)]
pub struct Language {
    /// Like `de` or `pt-BR`.
    pub code: String,
    /// The language's name in that language, from its `language-name` message.
    pub name: String,
    /// The user's file for it, if there is one.
    pub user_file: Option<PathBuf>,
    pub built_in: bool,
}

/// The `translations` folder in the config folder.
pub fn translations_dir() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("translations"))
}

/// The copy of the English messages kept in the translations folder, to
/// start a translation from. Its name doesn't end in `.ftl`, so it's not
/// read as a translation itself.
pub const SOURCE_COPY: &str = "english-source.ftl.txt";

/// Opens the translations folder, making it first if needed, with an
/// up-to-date copy of the English messages to translate.
pub fn open_translations_folder() -> Result<(), String> {
    let Some(dir) = translations_dir() else {
        return Err(t!("no-config-folder"));
    };
    std::fs::create_dir_all(&dir)
        .and_then(|_| std::fs::write(dir.join(SOURCE_COPY), SOURCE))
        .map_err(|e| {
            t!(
                "make-folder-failed",
                path = dir.display().to_string(),
                error = e.to_string()
            )
        })?;
    crate::system::open_folder(&dir)
}

/// The language code a `.ftl` file is for, from its name.
fn file_code(path: &Path) -> Option<String> {
    if path.extension()? != "ftl" {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    let id: LanguageIdentifier = stem.parse().ok()?;
    Some(id.to_string())
}

/// The user's translation files: (language code, path), sorted by code.
fn user_files(dir: Option<&Path>) -> Vec<(String, PathBuf)> {
    let Some(entries) = dir.and_then(|d| std::fs::read_dir(d).ok()) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .flatten()
        .map(|e| e.path())
        .filter_map(|p| file_code(&p).map(|c| (c, p)))
        .collect();
    files.sort();
    files
}

/// The `language-name` message of a file's text.
fn language_name(text: &str) -> Option<String> {
    let resource = FluentResource::try_new(text.to_owned()).unwrap_or_else(|(r, _)| r);
    let mut bundle = new_bundle("en");
    let _ = bundle.add_resource(resource);
    let pattern = bundle.get_message("language-name")?.value()?;
    let name = bundle.format_pattern(pattern, None, &mut Vec::new());
    Some(name.trim().to_owned()).filter(|n| !n.is_empty())
}

/// Every language the interface can use: the built-in ones and those in the
/// user's translations folder, sorted by name.
pub fn available() -> Vec<Language> {
    available_in(translations_dir().as_deref())
}

fn available_in(dir: Option<&Path>) -> Vec<Language> {
    let mut list: Vec<Language> = BUILT_IN
        .iter()
        .map(|(code, text)| Language {
            code: (*code).to_owned(),
            name: language_name(text).unwrap_or_else(|| (*code).to_owned()),
            user_file: None,
            built_in: true,
        })
        .collect();
    for (code, path) in user_files(dir) {
        let name = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| language_name(&t));
        match list.iter_mut().find(|l| l.code == code) {
            Some(l) => {
                l.user_file = Some(path);
                if let Some(name) = name {
                    l.name = name;
                }
            }
            None => list.push(Language {
                name: name.unwrap_or_else(|| code.clone()),
                code,
                user_file: Some(path),
                built_in: false,
            }),
        }
    }
    list.sort_by_key(|l| l.name.to_lowercase());
    list
}

/// The system's language, like `de-DE`, if it can be read.
pub fn system_locale() -> Option<String> {
    sys_locale::get_locale()
}

/// The available language that best matches `wanted` (`de-AT` matches `de`).
pub fn best_match(wanted: &str, available: &[Language]) -> Option<String> {
    let id: LanguageIdentifier = wanted.replace('_', "-").split('.').next()?.parse().ok()?;
    let full = id.to_string();
    if let Some(l) = available
        .iter()
        .find(|l| l.code.eq_ignore_ascii_case(&full))
    {
        return Some(l.code.clone());
    }
    let lang = id.language.as_str();
    available
        .iter()
        .find(|l| l.code.eq_ignore_ascii_case(lang))
        .or_else(|| {
            available.iter().find(|l| {
                l.code
                    .split('-')
                    .next()
                    .is_some_and(|c| c.eq_ignore_ascii_case(lang))
            })
        })
        .map(|l| l.code.clone())
}

/// The messages in use: the chosen language first, English last.
struct Localizer {
    code: String,
    bundles: Vec<FluentBundle<FluentResource>>,
}

fn new_bundle(code: &str) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = code.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // egui would draw the Unicode isolation marks around values as boxes.
    bundle.set_use_isolating(false);
    bundle
}

/// Parses `text` into a bundle for `code`; messages with mistakes are left
/// out and described in `errors`.
fn bundle_for(
    code: &str,
    text: &str,
    source: &str,
    errors: &mut Vec<String>,
) -> FluentBundle<FluentResource> {
    let mut bundle = new_bundle(code);
    let resource = match FluentResource::try_new(text.to_owned()) {
        Ok(r) => r,
        Err((r, list)) => {
            for e in list {
                errors.push(format!("{source}: {e}"));
            }
            r
        }
    };
    if let Err(list) = bundle.add_resource(resource) {
        for e in list {
            errors.push(format!("{source}: {e:?}"));
        }
    }
    bundle
}

impl Localizer {
    fn english() -> Self {
        Self {
            code: "en".to_owned(),
            bundles: vec![bundle_for("en", SOURCE, "en.ftl", &mut Vec::new())],
        }
    }

    /// The bundles for `code`: the user's file over the built-in one, for the
    /// full code and then the language alone, then English.
    fn load(code: &str, available: &[Language], errors: &mut Vec<String>) -> Self {
        let mut codes = vec![code.to_owned()];
        if let Some((lang, _)) = code.split_once('-') {
            codes.push(lang.to_owned());
        }
        if !codes.iter().any(|c| c == "en") {
            codes.push("en".to_owned());
        }
        let mut bundles = Vec::new();
        for c in &codes {
            let Some(lang) = available.iter().find(|l| &l.code == c) else {
                continue;
            };
            if let Some(path) = &lang.user_file {
                match std::fs::read_to_string(path) {
                    Ok(text) => {
                        bundles.push(bundle_for(c, &text, &path.display().to_string(), errors))
                    }
                    Err(e) => errors.push(format!("{}: {e}", path.display())),
                }
            }
            if let Some((_, text)) = BUILT_IN.iter().find(|(b, _)| b == c) {
                bundles.push(bundle_for(c, text, &format!("{c}.ftl"), errors));
            }
        }
        Self {
            code: code.to_owned(),
            bundles,
        }
    }

    fn format(&self, id: &str, args: Option<&FluentArgs>) -> String {
        for bundle in &self.bundles {
            if let Some(pattern) = bundle.get_message(id).and_then(|m| m.value()) {
                let mut errors = Vec::new();
                return bundle
                    .format_pattern(pattern, args, &mut errors)
                    .into_owned();
            }
        }
        // A message missing everywhere is a mistake in the app; show its id.
        id.to_owned()
    }
}

static CURRENT: LazyLock<RwLock<Localizer>> = LazyLock::new(|| RwLock::new(Localizer::english()));

/// Switches the interface to the language chosen in Preferences; an empty
/// choice follows the system. Returns messages about translation files with
/// mistakes (the rest of such a file is still used).
pub fn set_language(choice: &str) -> Vec<String> {
    let available = available();
    let code = resolve(choice, &available);
    let mut errors = Vec::new();
    let localizer = Localizer::load(&code, &available, &mut errors);
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = localizer;
    errors
}

/// The language a choice from Preferences means.
pub fn resolve(choice: &str, available: &[Language]) -> String {
    let wanted = if choice.is_empty() {
        system_locale().unwrap_or_default()
    } else {
        choice.to_owned()
    };
    best_match(&wanted, available).unwrap_or_else(|| "en".to_owned())
}

/// The code of the language in use, like `de`.
pub fn current() -> String {
    CURRENT
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .code
        .clone()
}

/// The message `id` in the current language.
pub fn tr(id: &str) -> String {
    CURRENT
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .format(id, None)
}

/// The message `id` with its placeables filled in.
pub fn tr_args(id: &str, args: &FluentArgs) -> String {
    CURRENT
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .format(id, Some(args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// Every message and term id in a file, with the variables its value uses.
    fn messages(text: &str) -> BTreeMap<String, BTreeSet<String>> {
        let resource = FluentResource::try_new(text.to_owned())
            .unwrap_or_else(|(_, e)| panic!("doesn't parse: {e:?}"));
        let mut out = BTreeMap::new();
        for entry in resource.entries() {
            use fluent_syntax::ast::Entry;
            if let Entry::Message(m) = entry {
                let mut vars = BTreeSet::new();
                if let Some(value) = &m.value {
                    collect_vars(value, &mut vars);
                }
                out.insert(m.id.name.to_owned(), vars);
            }
        }
        out
    }

    fn collect_vars(pattern: &fluent_syntax::ast::Pattern<&str>, out: &mut BTreeSet<String>) {
        use fluent_syntax::ast::{Expression, InlineExpression, PatternElement};
        fn inline(e: &InlineExpression<&str>, out: &mut BTreeSet<String>) {
            if let InlineExpression::VariableReference { id } = e {
                out.insert(id.name.to_owned());
            }
        }
        for element in &pattern.elements {
            if let PatternElement::Placeable { expression } = element {
                match expression {
                    Expression::Inline(e) => inline(e, out),
                    Expression::Select { selector, variants } => {
                        inline(selector, out);
                        for v in variants {
                            collect_vars(&v.value, out);
                        }
                    }
                }
            }
        }
    }

    /// The ids the source code looks up with `t!("…")` or `tr("…")`, and every
    /// quoted string in it (ids kept in tables are looked up later).
    fn ids_in_source() -> (BTreeSet<String>, String) {
        fn walk(dir: &Path, all: &mut String) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk(&p, all);
                } else if p.extension().is_some_and(|e| e == "rs") {
                    all.push_str(&std::fs::read_to_string(&p).unwrap());
                }
            }
        }
        let mut all = String::new();
        walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut all);
        let re = regex::Regex::new(r#"\b(?:t!|tr)\(\s*"([a-z0-9-]+)""#).unwrap();
        let used = re.captures_iter(&all).map(|c| c[1].to_owned()).collect();
        (used, all)
    }

    #[test]
    fn every_message_the_code_uses_is_in_english() {
        let english = messages(SOURCE);
        let (used, _) = ids_in_source();
        let missing: Vec<&String> = used
            .iter()
            .filter(|id| !english.contains_key(*id))
            .collect();
        assert!(missing.is_empty(), "not in en.ftl: {missing:?}");
    }

    #[test]
    fn every_english_message_is_used() {
        let (_, source) = ids_in_source();
        let unused: Vec<String> = messages(SOURCE)
            .into_keys()
            .filter(|id| id != "language-name" && !source.contains(&format!("\"{id}\"")))
            .collect();
        assert!(unused.is_empty(), "in en.ftl but not used: {unused:?}");
    }

    #[test]
    fn translations_match_english() {
        let english = messages(SOURCE);
        for (code, text) in BUILT_IN {
            let translated = messages(text);
            assert!(
                translated.contains_key("language-name"),
                "{code}: no language-name"
            );
            for (id, vars) in &translated {
                let Some(source_vars) = english.get(id) else {
                    panic!("{code}.ftl has {id}, which en.ftl doesn't");
                };
                assert!(
                    vars.is_subset(source_vars),
                    "{code}.ftl {id} uses {vars:?}, English has {source_vars:?}"
                );
            }
            let missing: Vec<&String> = english
                .keys()
                .filter(|id| !translated.contains_key(*id))
                .collect();
            assert!(missing.is_empty(), "{code}.ftl is missing {missing:?}");
        }
    }

    #[test]
    fn falls_back_to_english() {
        let available = available_in(None);
        let mut errors = Vec::new();
        let l = Localizer::load("de", &available, &mut errors);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(l.format("language-name", None), "Deutsch");
        assert_eq!(l.format("no-such-message", None), "no-such-message");
        let en = Localizer::load("en", &available, &mut errors);
        assert_eq!(en.bundles.len(), 1);
    }

    #[test]
    fn matches_system_locales() {
        let available = available_in(None);
        assert_eq!(best_match("de-AT", &available).as_deref(), Some("de"));
        assert_eq!(best_match("fr_CA.UTF-8", &available).as_deref(), Some("fr"));
        assert_eq!(best_match("en-US", &available).as_deref(), Some("en"));
        assert_eq!(best_match("ja-JP", &available), None);
        assert_eq!(resolve("es", &available), "es");
        assert_eq!(resolve("xx", &available), "en");
    }

    #[test]
    fn user_files_add_and_override_languages() {
        let dir = std::env::temp_dir().join(format!("ettin-i18n-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("pt-BR.ftl"),
            "language-name = Português (Brasil)\nsave = Salvar\n",
        )
        .unwrap();
        std::fs::write(dir.join("de.ftl"), "save = Sichern\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a translation").unwrap();
        let available = available_in(Some(&dir));
        let pt = available.iter().find(|l| l.code == "pt-BR").unwrap();
        assert_eq!(pt.name, "Português (Brasil)");
        assert!(!pt.built_in);
        let de = available.iter().find(|l| l.code == "de").unwrap();
        assert!(de.built_in && de.user_file.is_some());
        let mut errors = Vec::new();
        let l = Localizer::load("de", &available, &mut errors);
        assert_eq!(l.format("save", None), "Sichern");
        assert_eq!(l.format("language-name", None), "Deutsch");
        let l = Localizer::load("pt-BR", &available, &mut errors);
        assert_eq!(l.format("save", None), "Salvar");
        assert_eq!(l.format("save-as", None), messages_value("save-as"));
        assert!(errors.is_empty(), "{errors:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn messages_value(id: &str) -> String {
        Localizer::english().format(id, None)
    }

    #[test]
    fn fills_in_values() {
        let mut args = Args::new();
        args.set("count", 3);
        let text = Localizer::english().format("replaced-count", Some(&args));
        assert_eq!(text, "3 replaced");
    }
}
