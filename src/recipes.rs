//! Recipes: named lists of Transform steps that run one after another on the
//! same text. Each recipe is a JSON file in the recipes folder, so recipes can
//! be replayed, edited by hand and shared.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::editor::lines::{self, CaseKind, SortKind, TrimKind};
use crate::search::{Query, SearchOptions};
use crate::tools::convert::{self, DataFormat};
use crate::tools::json::{self, JsonOp};
use crate::tools::mime::MimeOp;
use crate::tools::pipe::{self, RunError};
use crate::tools::transform::{self, HashKind, LineMatcher, NumberPad};
use crate::tools::xml::{self, XmlOp};
use crate::tools::{ToolError, ToolResult, js, keep_final_newline};

/// The file format version this build writes.
pub const FILE_VERSION: u32 = 1;

fn version() -> u32 {
    FILE_VERSION
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    #[serde(default = "version")]
    pub version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default)]
    pub steps: Vec<RecipeStep>,
    /// The file it was read from or saved to.
    #[serde(skip)]
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeStep {
    #[serde(flatten)]
    pub op: Step,
    /// A switched-off step is kept but skipped.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

impl From<Step> for RecipeStep {
    fn from(op: Step) -> Self {
        Self { op, enabled: true }
    }
}

/// One step of a recipe. Each turns the whole text it is given into new text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Step {
    /// Replace every match, with the same options as the find bar.
    Replace {
        find: String,
        #[serde(default)]
        replace: String,
        #[serde(default)]
        regex: bool,
        #[serde(default)]
        match_case: bool,
        #[serde(default)]
        whole_word: bool,
        #[serde(default)]
        extended: bool,
    },
    /// Keep (or with `keep` false, remove) the lines that contain `pattern`.
    FilterLines {
        pattern: String,
        #[serde(default)]
        regex: bool,
        #[serde(default)]
        match_case: bool,
        #[serde(default = "yes")]
        keep: bool,
    },
    /// Each distinct line once, the most frequent first, with its count.
    CountDistinct,
    AffixLines {
        #[serde(default)]
        prefix: String,
        #[serde(default)]
        suffix: String,
        #[serde(default)]
        skip_empty: bool,
    },
    NumberLines {
        #[serde(default = "one")]
        start: i64,
        #[serde(default = "one")]
        step: i64,
        #[serde(default = "dot")]
        separator: String,
        #[serde(default)]
        pad: NumberPad,
        #[serde(default)]
        skip_empty: bool,
    },
    Reflow {
        #[serde(default = "eighty")]
        width: usize,
    },
    Convert {
        from: DataFormat,
        to: DataFormat,
    },
    /// The text's hash in place of the text.
    Hash {
        kind: HashKind,
    },
    HexEncode,
    HexDecode,
    JwtDecode,
    SortLines {
        #[serde(default = "lexical")]
        kind: SortKind,
        #[serde(default)]
        descending: bool,
    },
    ReverseLines,
    RemoveDuplicates {
        #[serde(default)]
        consecutive: bool,
    },
    RemoveEmptyLines {
        #[serde(default = "yes")]
        blank: bool,
    },
    Trim {
        kind: TrimKind,
    },
    /// All lines on one, joined by spaces.
    JoinLines,
    TabsToSpaces,
    SpacesToTabs {
        #[serde(default)]
        all: bool,
    },
    Case {
        kind: CaseKind,
    },
    /// A MIME Tools encoding or decoding.
    Mime {
        kind: MimeOp,
    },
    Json {
        kind: JsonOp,
    },
    Xml {
        kind: XmlOp,
    },
    /// Sends the text through a shell command. Recipes with one ask before
    /// they run.
    Command {
        command: String,
    },
}

fn one() -> i64 {
    1
}
fn dot() -> String {
    ". ".into()
}
fn eighty() -> usize {
    80
}
fn lexical() -> SortKind {
    SortKind::Lexical
}

/// What the steps may need from where they run.
#[derive(Default)]
pub struct Env<'a> {
    /// One level of indentation, for formatters.
    pub indent: &'a str,
    /// Where commands run.
    pub dir: Option<&'a Path>,
    /// The file being changed, for `ETTIN_FILE`.
    pub file: Option<&'a Path>,
    /// Set to stop a running command.
    pub cancel: Option<&'a AtomicBool>,
}

/// A step that failed: its index and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub step: usize,
    pub message: String,
}

/// Calls `f` with the lines of `text` (without line breaks). A final line
/// break stays a final line break.
fn with_lines(text: &str, f: impl FnOnce(Vec<String>) -> Vec<String>) -> String {
    let (body, trailing) = match text.strip_suffix('\n') {
        Some(body) => (body, true),
        None => (text, false),
    };
    let out = f(body.split('\n').map(str::to_owned).collect()).join("\n");
    if trailing && !out.is_empty() {
        out + "\n"
    } else {
        out
    }
}

fn replace_all(text: &str, query: &Query, template: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in query.find_iter(text) {
        out.push_str(&text[last..m.start]);
        out.push_str(&query.expand(text, &m, template));
        last = m.end;
    }
    out.push_str(&text[last..]);
    out
}

/// JSON formatted or minified; text that isn't JSON as JavaScript.
fn json_format(text: &str, indent: Option<&str>) -> ToolResult {
    let out = match json::parse(text) {
        Ok(doc) => doc.write(text, indent, false),
        Err(_) => match indent {
            Some(unit) => js::format(text, unit)?,
            None => js::minify(text)?,
        },
    };
    Ok(keep_final_newline(text, out))
}

impl Step {
    /// True for the steps that run a shell command.
    pub fn is_command(&self) -> bool {
        matches!(self, Step::Command { .. })
    }

    /// Checks the settings without running anything: the patterns compile
    /// and the command is one a recipe can hold.
    pub fn check(&self) -> Result<(), String> {
        match self {
            Step::Replace {
                find,
                regex,
                match_case,
                whole_word,
                extended,
                ..
            } => {
                if find.is_empty() {
                    return Err(t!("recipe-no-find"));
                }
                let opts = SearchOptions {
                    match_case: *match_case,
                    whole_word: *whole_word,
                    regex: *regex,
                    extended: *extended,
                    wrap: false,
                };
                Query::new(find, &opts).map(|_| ())
            }
            Step::FilterLines {
                pattern,
                regex,
                match_case,
                ..
            } => {
                if pattern.is_empty() {
                    return Err(t!("filter-no-pattern"));
                }
                LineMatcher::new(pattern, *regex, *match_case)
                    .map(|_| ())
                    .map_err(|e| e.message)
            }
            Step::Command { command } if command.trim().is_empty() => Err(t!("pipe-no-command")),
            Step::Convert { to, from } if !DataFormat::SOURCES.contains(from) || from == to => {
                Err(t!("recipe-step-not-allowed"))
            }
            Step::Json {
                kind: JsonOp::Validate,
            }
            | Step::Xml {
                kind:
                    XmlOp::CheckSyntax
                    | XmlOp::ValidateSchema
                    | XmlOp::ChooseSchema
                    | XmlOp::XslTransform,
            } => Err(t!("recipe-step-not-allowed")),
            _ => Ok(()),
        }
    }

    /// Runs the step on `text`.
    pub fn apply(&self, text: &str, env: &Env) -> ToolResult {
        self.check().map_err(ToolError::new)?;
        let indent = if env.indent.is_empty() {
            "    "
        } else {
            env.indent
        };
        Ok(match self {
            Step::Replace {
                find,
                replace,
                regex,
                match_case,
                whole_word,
                extended,
            } => {
                let opts = SearchOptions {
                    match_case: *match_case,
                    whole_word: *whole_word,
                    regex: *regex,
                    extended: *extended,
                    wrap: false,
                };
                match Query::new(find, &opts).map_err(ToolError::new)? {
                    Some(q) => replace_all(text, &q, replace),
                    None => text.to_owned(),
                }
            }
            Step::FilterLines {
                pattern,
                regex,
                match_case,
                keep,
            } => {
                let matcher = LineMatcher::new(pattern, *regex, *match_case)?;
                transform::filter_lines(text, &matcher, *keep)
            }
            Step::CountDistinct if text.is_empty() => String::new(),
            Step::CountDistinct => transform::count_lines(text),
            Step::AffixLines {
                prefix,
                suffix,
                skip_empty,
            } => transform::affix_lines(text, prefix, suffix, *skip_empty),
            Step::NumberLines {
                start,
                step,
                separator,
                pad,
                skip_empty,
            } => transform::number_lines(text, *start, *step, separator, *pad, *skip_empty),
            Step::Reflow { width } => transform::reflow(text, (*width).clamp(1, 10_000)),
            Step::Convert { from, to } => convert::convert(text, *from, *to, indent)?,
            Step::Hash { kind } => transform::hash(*kind, text.as_bytes()),
            Step::HexEncode => transform::hex_encode(text),
            Step::HexDecode => transform::hex_decode(text)?,
            Step::JwtDecode => transform::jwt_decode(text, indent)?.json,
            Step::SortLines { kind, descending } => with_lines(text, |mut l| {
                lines::sort_lines(&mut l, *kind, *descending);
                l
            }),
            Step::ReverseLines => with_lines(text, |mut l| {
                l.reverse();
                l
            }),
            Step::RemoveDuplicates { consecutive } => {
                with_lines(text, |l| lines::remove_duplicates(l, *consecutive))
            }
            Step::RemoveEmptyLines { blank } => {
                with_lines(text, |l| lines::remove_empty(l, *blank))
            }
            Step::Trim { kind } => with_lines(text, |l| {
                l.iter().map(|l| lines::trim_line(l, *kind)).collect()
            }),
            Step::JoinLines => with_lines(text, |l| vec![l.join(" ")]),
            Step::TabsToSpaces => {
                with_lines(text, |l| l.iter().map(|l| lines::expand_tabs(l)).collect())
            }
            Step::SpacesToTabs { all } => with_lines(text, |l| {
                l.iter().map(|l| lines::spaces_to_tabs(l, *all)).collect()
            }),
            Step::Case { kind } => lines::convert_case(text, *kind),
            Step::Mime { kind } => {
                let out = kind.run(text)?;
                if *kind == MimeOp::SamlDecode {
                    keep_final_newline(text, out)
                } else {
                    out
                }
            }
            Step::Json { kind } => match kind {
                JsonOp::Format => json_format(text, Some(indent))?,
                JsonOp::Minify => json_format(text, None)?,
                JsonOp::SortKeys => keep_final_newline(text, json::sort_keys(text, indent)?),
                JsonOp::Validate => unreachable!("checked"),
            },
            Step::Xml { kind } => match kind {
                XmlOp::PrettyPrint | XmlOp::PrettyPrintAttributes => keep_final_newline(
                    text,
                    xml::pretty_print(text, indent, *kind == XmlOp::PrettyPrintAttributes)?,
                ),
                XmlOp::Linearize => keep_final_newline(text, xml::linearize(text)?),
                XmlOp::Escape => xml::escape(text),
                XmlOp::Unescape => xml::unescape(text),
                _ => unreachable!("checked"),
            },
            Step::Command { command } => {
                match pipe::run(command, env.dir, env.file, text, env.cancel) {
                    Ok(out) => out,
                    Err(RunError::Start(e)) => {
                        return Err(ToolError::new(t!("pipe-start-failed", error = e)));
                    }
                    Err(RunError::Failed(code, stderr)) => {
                        let code = code.map_or_else(|| "?".to_owned(), |c| c.to_string());
                        let message = t!("pipe-failed", code = code);
                        return Err(ToolError::new(if stderr.is_empty() {
                            message
                        } else {
                            format!("{message}\n{stderr}")
                        }));
                    }
                    Err(RunError::Stopped) => return Err(ToolError::new(t!("recipe-stopped"))),
                }
            }
        })
    }
}

/// What [`Recipe::preview`] got to.
pub struct Preview {
    pub text: Result<String, Failure>,
    /// The command step the preview stopped before, if it met one.
    pub stopped_at: Option<usize>,
}

impl Recipe {
    pub fn new(name: &str) -> Self {
        Self {
            version: FILE_VERSION,
            name: name.to_owned(),
            description: String::new(),
            steps: Vec::new(),
            path: None,
        }
    }

    fn enabled(&self) -> impl Iterator<Item = (usize, &Step)> {
        self.steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.enabled)
            .map(|(i, s)| (i, &s.op))
    }

    /// The shell commands the switched-on steps run, in order.
    pub fn commands(&self) -> Vec<String> {
        self.enabled()
            .filter_map(|(_, s)| match s {
                Step::Command { command } => Some(command.clone()),
                _ => None,
            })
            .collect()
    }

    /// Identifies the recipe's name and commands, for remembering that the
    /// user allowed them. Changing a command asks again.
    pub fn fingerprint(&self) -> String {
        let mut hash = sha2::Sha256::new();
        hash.update(self.name.as_bytes());
        for c in self.commands() {
            hash.update([0]);
            hash.update(c.as_bytes());
        }
        hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Runs every switched-on step on `text`, in order.
    pub fn run(&self, text: &str, env: &Env) -> Result<String, Failure> {
        let mut text = text.to_owned();
        for (i, step) in self.enabled() {
            text = step.apply(&text, env).map_err(|e| Failure {
                step: i,
                message: e.message,
            })?;
        }
        Ok(text)
    }

    /// Like [`Self::run`], but stops before the first command step instead
    /// of running it.
    pub fn preview(&self, text: &str, env: &Env) -> Preview {
        let mut text = text.to_owned();
        for (i, step) in self.enabled() {
            if step.is_command() {
                return Preview {
                    text: Ok(text),
                    stopped_at: Some(i),
                };
            }
            match step.apply(&text, env) {
                Ok(t) => text = t,
                Err(e) => {
                    return Preview {
                        text: Err(Failure {
                            step: i,
                            message: e.message,
                        }),
                        stopped_at: None,
                    };
                }
            }
        }
        Preview {
            text: Ok(text),
            stopped_at: None,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default() + "\n"
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let recipe: Recipe = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if recipe.name.trim().is_empty() {
            return Err(t!("recipe-no-name"));
        }
        Ok(recipe)
    }
}

// -------------------------------------------------------------------------
// The recipes folder

pub fn recipes_dir() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("recipes"))
}

/// The recipe called `name`; exact first, then ignoring case.
pub fn find<'a>(recipes: &'a [Recipe], name: &str) -> Option<&'a Recipe> {
    recipes
        .iter()
        .find(|r| r.name == name)
        .or_else(|| recipes.iter().find(|r| r.name.eq_ignore_ascii_case(name)))
}

/// Reads every `.json` file in `dir`, sorted by name. The first time (when
/// the folder doesn't exist yet) it is made and the sample recipes are put in
/// it. Returns the recipes and a message for each file that can't be read.
pub fn load_all(dir: &Path) -> (Vec<Recipe>, Vec<String>) {
    if !dir.exists() {
        let _ = install_samples(dir);
    }
    let mut recipes = Vec::new();
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (recipes, errors);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| Recipe::from_json(&t))
        {
            Ok(mut r) => {
                r.path = Some(path);
                recipes.push(r);
            }
            Err(e) => errors.push(t!(
                "recipe-read-failed",
                path = path.display().to_string(),
                error = e
            )),
        }
    }
    recipes.sort_by_key(|r| r.name.to_lowercase());
    (recipes, errors)
}

/// A file name for a new recipe: its name made safe, and not taken yet.
fn file_name_for(dir: &Path, name: &str) -> PathBuf {
    let mut stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while stem.contains("--") {
        stem = stem.replace("--", "-");
    }
    let stem = stem.trim_matches('-');
    let stem = if stem.is_empty() { "recipe" } else { stem };
    let stem: String = stem.chars().take(60).collect();
    let mut path = dir.join(format!("{stem}.json"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}.json"));
        n += 1;
    }
    path
}

/// Writes the recipe to its file, picking a file in `dir` for a new one.
pub fn save(recipe: &mut Recipe, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = match &recipe.path {
        Some(p) => p.clone(),
        None => file_name_for(dir, &recipe.name),
    };
    std::fs::write(&path, recipe.to_json()).map_err(|e| {
        t!(
            "recipe-save-failed",
            path = path.display().to_string(),
            error = e.to_string()
        )
    })?;
    recipe.path = Some(path);
    Ok(())
}

/// The recipes Ettin Edit starts with, to show what recipes can do.
pub fn samples() -> Vec<Recipe> {
    let recipe = |name: &str, description: &str, steps: Vec<Step>| Recipe {
        description: description.to_owned(),
        steps: steps.into_iter().map(RecipeStep::from).collect(),
        ..Recipe::new(name)
    };
    let replace = |find: &str, with: &str| Step::Replace {
        find: find.into(),
        replace: with.into(),
        regex: true,
        match_case: true,
        whole_word: false,
        extended: false,
    };
    vec![
        recipe(
            "Tidy Whitespace",
            "Removes spaces at line ends and squeezes runs of blank lines into one.",
            vec![
                Step::Trim {
                    kind: TrimKind::Trailing,
                },
                replace(r"\n{3,}", r"\n\n"),
            ],
        ),
        recipe(
            "Unique Sorted Lines",
            "Each line once, sorted, without blank lines.",
            vec![
                Step::RemoveEmptyLines { blank: true },
                Step::SortLines {
                    kind: SortKind::IgnoreCase,
                    descending: false,
                },
                Step::RemoveDuplicates { consecutive: false },
            ],
        ),
        recipe(
            "Count Errors and Warnings",
            "Keeps log lines with ERROR or WARN and counts how often each message appears.",
            vec![
                Step::FilterLines {
                    pattern: r"\b(ERROR|WARN(ING)?)\b".into(),
                    regex: true,
                    match_case: true,
                    keep: true,
                },
                // Drop the time and anything else before the level, so the
                // same message counts once.
                replace(r"^.*?\b(ERROR|WARN(ING)?)\b", "$1"),
                Step::CountDistinct,
            ],
        ),
        recipe(
            "Lines to SQL IN List",
            "Turns one value per line into IN ('a', 'b', 'c').",
            vec![
                Step::Trim {
                    kind: TrimKind::Both,
                },
                Step::RemoveEmptyLines { blank: true },
                replace("'", "''"),
                Step::AffixLines {
                    prefix: "'".into(),
                    suffix: "',".into(),
                    skip_empty: false,
                },
                Step::JoinLines,
                replace(r",[ \t]*$", ""),
                Step::AffixLines {
                    prefix: "IN (".into(),
                    suffix: ")".into(),
                    skip_empty: false,
                },
            ],
        ),
        recipe(
            "CSV to Markdown Table",
            "Converts CSV with a header row to a Markdown table.",
            vec![Step::Convert {
                from: DataFormat::Csv,
                to: DataFormat::Markdown,
            }],
        ),
    ]
}

/// Writes the sample recipes into `dir`.
pub fn install_samples(dir: &Path) -> Result<(), String> {
    for mut r in samples() {
        save(&mut r, dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(steps: Vec<Step>, text: &str) -> String {
        let r = Recipe {
            steps: steps.into_iter().map(RecipeStep::from).collect(),
            ..Recipe::new("t")
        };
        r.run(text, &Env::default()).unwrap()
    }

    #[test]
    fn file_format_reads_back() {
        let mut r = samples().remove(3);
        r.steps[1].enabled = false;
        let json = r.to_json();
        assert!(json.contains(r#""op": "affix_lines""#), "{json}");
        assert!(json.contains(r#""enabled": false"#));
        assert_eq!(Recipe::from_json(&json).unwrap(), r);
    }

    #[test]
    fn hand_written_files_use_defaults() {
        let text = r#"{"name": "x", "steps": [
            {"op": "replace", "find": "a"},
            {"op": "sort_lines"},
            {"op": "number_lines"},
            {"op": "filter_lines", "pattern": "b"}
        ]}"#;
        let r = Recipe::from_json(text).unwrap();
        assert_eq!(r.version, FILE_VERSION);
        assert!(r.steps.iter().all(|s| s.enabled));
        assert_eq!(r.run("b a\nc\n", &Env::default()).unwrap(), "1. b \n");
        assert!(Recipe::from_json(r#"{"name": " "}"#).is_err());
        assert!(Recipe::from_json(r#"{"name": "x", "steps": [{"op": "nope"}]}"#).is_err());
    }

    #[test]
    fn samples_do_what_they_say() {
        let s = samples();
        let env = Env::default();
        assert_eq!(s[0].run("a  \n\n\n\nb\t\n", &env).unwrap(), "a\n\nb\n");
        assert_eq!(s[1].run("b\n\na\nB\nb\n", &env).unwrap(), "a\nB\nb\n");
        assert_eq!(
            s[2].run("10:00 ERROR one\nok\n10:01 ERROR one\nWARN two\n", &env)
                .unwrap(),
            "2\tERROR one\n1\tWARN two\n"
        );
        assert_eq!(
            s[3].run(" o'brien \n\nsmith\n", &env).unwrap(),
            "IN ('o''brien', 'smith')\n"
        );
        assert!(s[4].run("a,b\n1,2\n", &env).unwrap().starts_with("| a"));
    }

    #[test]
    fn replace_uses_find_options() {
        let step = |find: &str, replace: &str, regex, whole_word| Step::Replace {
            find: find.into(),
            replace: replace.into(),
            regex,
            match_case: false,
            whole_word,
            extended: false,
        };
        assert_eq!(
            run(vec![step("cat", "dog", false, true)], "Cat cats"),
            "dog cats"
        );
        assert_eq!(
            run(
                vec![step(r"(\w+)@(\w+)", "$2 at $1", true, false)],
                "me@home"
            ),
            "home at me"
        );
        // Line breaks match `\r\n` as well, as in the find bar.
        assert_eq!(run(vec![step(r"\r\n", ",", true, false)], "a\nb"), "a,b");
    }

    #[test]
    fn line_steps_keep_the_final_line_break() {
        let sort = Step::SortLines {
            kind: SortKind::Lexical,
            descending: true,
        };
        assert_eq!(run(vec![sort.clone()], "a\nc\nb\n"), "c\nb\na\n");
        assert_eq!(run(vec![sort], "a\nb"), "b\na");
        assert_eq!(run(vec![Step::JoinLines], "a\nb\n"), "a b\n");
        assert_eq!(run(vec![Step::ReverseLines], ""), "");
        assert_eq!(run(vec![Step::CountDistinct], ""), "");
    }

    #[test]
    fn switched_off_steps_are_skipped_and_errors_name_the_step() {
        let mut r = Recipe::new("t");
        r.steps.push(
            Step::Case {
                kind: CaseKind::Upper,
            }
            .into(),
        );
        r.steps.push(RecipeStep {
            op: Step::ReverseLines,
            enabled: false,
        });
        r.steps.push(
            Step::Xml {
                kind: XmlOp::Linearize,
            }
            .into(),
        );
        assert_eq!(
            r.run("<a>\n <b/>\n</a>", &Env::default()).unwrap(),
            "<A><B/></A>"
        );
        let err = r.run("<a>", &Env::default()).unwrap_err();
        assert_eq!(err.step, 2);
        r.steps.push(
            Step::Xml {
                kind: XmlOp::XslTransform,
            }
            .into(),
        );
        assert_eq!(r.run("<a/>", &Env::default()).unwrap_err().step, 3);
    }

    #[test]
    fn preview_stops_before_commands() {
        let mut r = Recipe::new("t");
        r.steps.push(
            Step::Case {
                kind: CaseKind::Upper,
            }
            .into(),
        );
        r.steps.push(
            Step::Command {
                command: "rm -rf /nothing".into(),
            }
            .into(),
        );
        r.steps.push(Step::ReverseLines.into());
        let p = r.preview("a\nb", &Env::default());
        assert_eq!(p.text.unwrap(), "A\nB");
        assert_eq!(p.stopped_at, Some(1));
        assert_eq!(r.commands(), vec!["rm -rf /nothing".to_owned()]);
        let before = r.fingerprint();
        r.steps[1].op = Step::Command {
            command: "true".into(),
        };
        assert_ne!(r.fingerprint(), before);
    }

    #[cfg(unix)]
    #[test]
    fn command_steps_run() {
        let r = Recipe {
            steps: vec![
                Step::Command {
                    command: "sort".into(),
                }
                .into(),
                Step::Case {
                    kind: CaseKind::Upper,
                }
                .into(),
            ],
            ..Recipe::new("t")
        };
        assert_eq!(r.run("b\na\n", &Env::default()).unwrap(), "A\nB\n");
    }

    #[test]
    fn folder_round_trip() {
        let dir = std::env::temp_dir().join(format!("ettin-recipes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (list, errors) = load_all(&dir);
        assert!(errors.is_empty());
        assert_eq!(list.len(), samples().len());
        std::fs::write(dir.join("broken.json"), "{").unwrap();
        let mut mine = Recipe::new("Tidy Whitespace");
        save(&mut mine, &dir).unwrap();
        assert!(
            mine.path
                .as_ref()
                .unwrap()
                .ends_with("tidy-whitespace-2.json")
        );
        let (list, errors) = load_all(&dir);
        assert_eq!(errors.len(), 1);
        assert_eq!(list.len(), samples().len() + 1);
        assert!(find(&list, "unique sorted lines").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
