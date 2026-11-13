//! Running a recipe from the command line, without opening a window:
//!
//! ```text
//! ettin-edit --recipe NAME-OR-ID [--allow-commands] [--in-place] [--out FILE] [FILE...]
//! ettin-edit --list-recipes
//! ```
//!
//! With no files the text comes from standard input. Results go to standard
//! output, back into the files with `--in-place`, or to one file with
//! `--out`. A recipe whose table step reads a workbook gets the file's
//! bytes, and one that writes a workbook needs `--out`.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::recipes::{self, Env, Recipe};
use crate::table::export::Output;

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
        out: Option<PathBuf>,
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
    let mut out = None;
    let mut skip = None;
    for (i, arg) in args.iter().enumerate() {
        if i == at || i == at + 1 || Some(i) == skip {
            continue;
        }
        if arg == "--out" {
            let Some(path) = args.get(i + 1) else {
                return Some(Err("--out needs a file name".into()));
            };
            out = Some(PathBuf::from(path));
            skip = Some(i + 1);
        } else if arg == "--in-place" {
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
    if out.is_some() && (in_place || files.len() > 1) {
        return Some(Err("--out takes the result of one file".into()));
    }
    Some(Ok(Request::Run {
        recipe: recipe.to_owned(),
        files,
        in_place,
        allow_commands,
        out,
    }))
}

/// A recipe by name or id from the recipes folder, or from a recipe file.
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
                "usage: ettin-edit --recipe NAME-OR-ID [--allow-commands] [--in-place] [--out FILE] [FILE...]\n       ettin-edit --list-recipes"
            );
            return Some(USAGE);
        }
    };
    let (recipe, files, in_place, allow_commands, out_path) = match request {
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
            out,
        } => (recipe, files, in_place, allow_commands, out),
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
    // Plugin steps start the plugins they use, once, as Ettin Edit would.
    let plugins = crate::plugins::headless::Headless::from_settings(&settings);
    let wants_bytes = recipe.wants_bytes();
    let apply = |input: Output, file: Option<&Path>| -> Result<Output, String> {
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
            plugins: Some(&plugins),
        };
        recipe.run_output(input, &env).map_err(|f| {
            t!(
                "recipe-failed",
                name = recipe.name.clone(),
                step = f.step + 1,
                message = f.message
            )
        })
    };
    // The bytes to write for one input: text goes back with the input's
    // line ends and byte-order mark.
    let run_one =
        |bytes: Vec<u8>, what: &str, file: Option<&Path>| -> Result<(bool, Vec<u8>), String> {
            if wants_bytes {
                let out = apply(Output::Bytes(bytes.clone()), file)
                    .map_err(|e| format!("{what}: {e}"))?;
                let out = out.into_bytes();
                return Ok((out != bytes, out));
            }
            let (text, shape) = to_editor(bytes, what)?;
            match apply(Output::Text(text.clone()), file).map_err(|e| format!("{what}: {e}"))? {
                Output::Text(t) => Ok((t != text, from_editor(&t, &shape).into_bytes())),
                Output::Bytes(b) => Ok((true, b)),
            }
        };
    let mut stdout = std::io::stdout().lock();
    let deliver = |stdout: &mut std::io::StdoutLock, out: Vec<u8>| -> Result<(), String> {
        match &out_path {
            Some(path) => std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display())),
            None => stdout.write_all(&out).map_err(|e| e.to_string()),
        }
    };
    if files.is_empty() {
        let mut bytes = Vec::new();
        if let Err(e) = std::io::stdin().read_to_end(&mut bytes) {
            eprintln!("ettin-edit: {e}");
            return Some(FAILED);
        }
        let result =
            run_one(bytes, "standard input", None).and_then(|(_, out)| deliver(&mut stdout, out));
        return Some(match result {
            Ok(()) => 0,
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
            .and_then(|b| run_one(b, &what, Some(&file)));
        let result = match result {
            Ok((changed, out)) if in_place => {
                if changed {
                    std::fs::write(&file, out).map_err(|e| format!("{what}: {e}"))
                } else {
                    Ok(())
                }
            }
            Ok((_, out)) => deliver(&mut stdout, out),
            Err(e) => Err(e),
        };
        if let Err(e) = result {
            eprintln!("ettin-edit: {e}");
            code = FAILED;
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
                out: None,
            }))
        );
        assert_eq!(
            parse(&args(&["--recipe", "T", "--out", "x.xlsx", "a.csv"])),
            Some(Ok(Request::Run {
                recipe: "T".into(),
                files: vec![PathBuf::from("a.csv")],
                in_place: false,
                allow_commands: false,
                out: Some(PathBuf::from("x.xlsx")),
            }))
        );
        assert!(matches!(
            parse(&args(&["--recipe", "T", "--out", "x", "a", "b"])),
            Some(Err(_))
        ));
        assert!(matches!(
            parse(&args(&["--recipe", "T", "--out"])),
            Some(Err(_))
        ));
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
