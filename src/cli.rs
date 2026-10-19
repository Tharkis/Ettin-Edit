//! Running a recipe from the command line, without opening a window:
//!
//! ```text
//! ettin-edit --recipe NAME [--allow-commands] [--in-place] [FILE...]
//! ettin-edit --list-recipes
//! ```
//!
//! With no files the text comes from standard input. Results go to standard
//! output, or back into the files with `--in-place`.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::recipes::{self, Env, Recipe};

/// Exit codes.
const FAILED: i32 = 1;
const USAGE: i32 = 2;

/// What the arguments ask for.
#[derive(Debug, PartialEq)]
enum Request {
    List,
    Run {
        recipe: String,
        files: Vec<PathBuf>,
        in_place: bool,
        allow_commands: bool,
    },
}

/// `None` when the arguments are for the editor, not the command line.
fn parse(args: &[OsString]) -> Option<Result<Request, String>> {
    if args.iter().any(|a| a == "--list-recipes") {
        return Some(Ok(Request::List));
    }
    let at = args.iter().position(|a| a == "--recipe")?;
    let Some(recipe) = args.get(at + 1).and_then(|r| r.to_str()) else {
        return Some(Err("--recipe needs the name of a recipe".into()));
    };
    let mut files = Vec::new();
    let mut in_place = false;
    let mut allow_commands = false;
    for (i, arg) in args.iter().enumerate() {
        if i == at || i == at + 1 {
            continue;
        }
        if arg == "--in-place" {
            in_place = true;
        } else if arg == "--allow-commands" {
            allow_commands = true;
        } else if arg.to_string_lossy().starts_with("--") {
            return Some(Err(format!("unknown option {}", arg.to_string_lossy())));
        } else {
            files.push(PathBuf::from(arg));
        }
    }
    if in_place && files.is_empty() {
        return Some(Err("--in-place needs files to change".into()));
    }
    Some(Ok(Request::Run {
        recipe: recipe.to_owned(),
        files,
        in_place,
        allow_commands,
    }))
}

/// A recipe by name from the recipes folder, or from a recipe file.
fn find_recipe(name: &str) -> Result<Recipe, String> {
    let path = Path::new(name);
    if path.extension().is_some_and(|e| e == "json") && path.is_file() {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        return Recipe::from_json(&text).map_err(|e| format!("{}: {e}", path.display()));
    }
    let dir = recipes::recipes_dir().ok_or("no settings folder")?;
    let (list, _) = recipes::load_all(&dir);
    recipes::find(&list, name)
        .cloned()
        .ok_or_else(|| t!("recipe-not-found", name = name.to_owned()))
}

/// Text as the editor keeps it (`\n` line breaks, no byte-order mark), and
/// how to put it back.
struct Shape {
    bom: bool,
    crlf: bool,
}

fn to_editor(bytes: Vec<u8>, what: &str) -> Result<(String, Shape), String> {
    let text = String::from_utf8(bytes).map_err(|_| format!("{what}: not UTF-8 text"))?;
    let (bom, text) = match text.strip_prefix('\u{feff}') {
        Some(rest) => (true, rest.to_owned()),
        None => (false, text),
    };
    let crlf = text.contains("\r\n");
    let text = text.replace("\r\n", "\n");
    Ok((text, Shape { bom, crlf }))
}

fn from_editor(text: &str, shape: &Shape) -> String {
    let mut out = if shape.bom {
        "\u{feff}".to_owned()
    } else {
        String::new()
    };
    if shape.crlf {
        out.push_str(&text.replace('\n', "\r\n"));
    } else {
        out.push_str(text);
    }
    out
}

/// Runs the command line when the arguments ask for it; returns the exit
/// code, or `None` to start the editor.
pub fn run(args: &[OsString]) -> Option<i32> {
    let request = parse(args)?;
    attach_console();
    let settings = crate::settings::Settings::load();
    let _ = crate::i18n::set_language(&settings.ui_language);
    crate::editor::layout::set_tab_width(settings.tab_width);
    let request = match request {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ettin-edit: {e}");
            eprintln!(
                "usage: ettin-edit --recipe NAME [--allow-commands] [--in-place] [FILE...]\n       ettin-edit --list-recipes"
            );
            return Some(USAGE);
        }
    };
    let (recipe, files, in_place, allow_commands) = match request {
        Request::List => {
            let Some(dir) = recipes::recipes_dir() else {
                return Some(FAILED);
            };
            let (list, errors) = recipes::load_all(&dir);
            for e in errors {
                eprintln!("ettin-edit: {e}");
            }
            for r in list {
                if r.description.is_empty() {
                    println!("{}", r.name);
                } else {
                    println!("{}\t{}", r.name, r.description);
                }
            }
            return Some(0);
        }
        Request::Run {
            recipe,
            files,
            in_place,
            allow_commands,
        } => (recipe, files, in_place, allow_commands),
    };
    let recipe = match find_recipe(&recipe) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ettin-edit: {e}");
            return Some(FAILED);
        }
    };
    let commands = recipe.commands();
    if !commands.is_empty() && !allow_commands {
        eprintln!(
            "ettin-edit: \"{}\" runs shell commands; add --allow-commands to run them:",
            recipe.name
        );
        for c in commands {
            eprintln!("  {c}");
        }
        return Some(USAGE);
    }
    let indent = if settings.use_spaces {
        " ".repeat(settings.tab_width.max(1))
    } else {
        "\t".to_owned()
    };
    let cwd = std::env::current_dir().ok();
    let apply = |text: &str, file: Option<&Path>| -> Result<String, String> {
        let dir = file
            .and_then(|f| f.parent())
            .filter(|d| !d.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .or_else(|| cwd.clone());
        let env = Env {
            indent: &indent,
            dir: dir.as_deref(),
            file,
            cancel: None,
        };
        recipe.run(text, &env).map_err(|f| {
            t!(
                "recipe-failed",
                name = recipe.name.clone(),
                step = f.step + 1,
                message = f.message
            )
        })
    };
    let mut stdout = std::io::stdout().lock();
    if files.is_empty() {
        let mut bytes = Vec::new();
        if let Err(e) = std::io::stdin().read_to_end(&mut bytes) {
            eprintln!("ettin-edit: {e}");
            return Some(FAILED);
        }
        let result = to_editor(bytes, "standard input")
            .and_then(|(text, shape)| Ok(from_editor(&apply(&text, None)?, &shape)));
        return Some(match result {
            Ok(out) => {
                let _ = stdout.write_all(out.as_bytes());
                0
            }
            Err(e) => {
                eprintln!("ettin-edit: {e}");
                FAILED
            }
        });
    }
    let mut code = 0;
    for file in files {
        let what = file.display().to_string();
        let result = std::fs::read(&file)
            .map_err(|e| format!("{what}: {e}"))
            .and_then(|b| to_editor(b, &what))
            .and_then(|(text, shape)| {
                let out = apply(&text, Some(&file)).map_err(|e| format!("{what}: {e}"))?;
                Ok((text != out, from_editor(&out, &shape)))
            });
        match result {
            Ok((changed, out)) if in_place => {
                if changed && let Err(e) = std::fs::write(&file, out) {
                    eprintln!("ettin-edit: {what}: {e}");
                    code = FAILED;
                }
            }
            Ok((_, out)) => {
                let _ = stdout.write_all(out.as_bytes());
            }
            Err(e) => {
                eprintln!("ettin-edit: {e}");
                code = FAILED;
            }
        }
    }
    Some(code)
}

/// A Windows build has no console of its own; use the one it was started
/// from, so output shows in the terminal.
#[cfg(windows)]
fn attach_console() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // SAFETY: AttachConsole has no preconditions; failing (already attached,
    // or no parent console) leaves things as they were.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_console() {}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn reads_the_arguments() {
        assert_eq!(parse(&args(&["file.txt"])), None);
        assert_eq!(parse(&args(&["--list-recipes"])), Some(Ok(Request::List)));
        assert_eq!(
            parse(&args(&["--recipe", "Tidy", "a.txt", "--in-place"])),
            Some(Ok(Request::Run {
                recipe: "Tidy".into(),
                files: vec![PathBuf::from("a.txt")],
                in_place: true,
                allow_commands: false,
            }))
        );
        assert!(matches!(parse(&args(&["--recipe"])), Some(Err(_))));
        assert!(matches!(
            parse(&args(&["--recipe", "x", "--in-place"])),
            Some(Err(_))
        ));
        assert!(matches!(
            parse(&args(&["--recipe", "x", "--bogus"])),
            Some(Err(_))
        ));
    }

    #[test]
    fn keeps_line_endings_and_bom() {
        let (text, shape) = to_editor(b"\xef\xbb\xbfa\r\nb\r\n".to_vec(), "x").unwrap();
        assert_eq!(text, "a\nb\n");
        assert_eq!(from_editor("b\na\n", &shape), "\u{feff}b\r\na\r\n");
        assert!(to_editor(vec![0xff], "x").is_err());
    }
}
