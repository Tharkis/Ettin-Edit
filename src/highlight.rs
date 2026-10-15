//! Language detection, the bundled grammars and the user's own languages.
//! The highlighting itself lives in `editor::highlight`.
//!
//! The grammar set is built in three layers, each winning over the one before
//! when two grammars share a name: syntect's defaults, Ettin's own grammars in
//! `assets/syntaxes`, then the user's files in the `languages` folder
//! (`*.sublime-syntax` as-is, and `*.lang.json` simple definitions, see
//! [`crate::user_lang`]).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use syntect::parsing::syntax_definition::{
    ContextReference, MatchOperation, Pattern, SyntaxDefinition,
};
use syntect::parsing::{SyntaxReference, SyntaxSet, SyntaxSetBuilder};

use crate::user_lang;

pub const PLAIN_TEXT: &str = "Plain Text";

/// Ettin's own grammars: (file name, source).
const BUILTIN_SYNTAXES: &[(&str, &str)] = &[
    (
        "toml.sublime-syntax",
        include_str!("../assets/syntaxes/toml.sublime-syntax"),
    ),
    (
        "ini.sublime-syntax",
        include_str!("../assets/syntaxes/ini.sublime-syntax"),
    ),
    (
        "typescript.sublime-syntax",
        include_str!("../assets/syntaxes/typescript.sublime-syntax"),
    ),
    (
        "powershell.sublime-syntax",
        include_str!("../assets/syntaxes/powershell.sublime-syntax"),
    ),
    (
        "dockerfile.sublime-syntax",
        include_str!("../assets/syntaxes/dockerfile.sublime-syntax"),
    ),
    (
        "hl7.sublime-syntax",
        include_str!("../assets/syntaxes/hl7.sublime-syntax"),
    ),
    (
        "kotlin.sublime-syntax",
        include_str!("../assets/syntaxes/kotlin.sublime-syntax"),
    ),
    (
        "swift.sublime-syntax",
        include_str!("../assets/syntaxes/swift.sublime-syntax"),
    ),
    (
        "dart.sublime-syntax",
        include_str!("../assets/syntaxes/dart.sublime-syntax"),
    ),
    (
        "zig.sublime-syntax",
        include_str!("../assets/syntaxes/zig.sublime-syntax"),
    ),
    (
        "cmake.sublime-syntax",
        include_str!("../assets/syntaxes/cmake.sublime-syntax"),
    ),
    (
        "nginx.sublime-syntax",
        include_str!("../assets/syntaxes/nginx.sublime-syntax"),
    ),
    (
        "dotenv.sublime-syntax",
        include_str!("../assets/syntaxes/dotenv.sublime-syntax"),
    ),
    (
        "protobuf.sublime-syntax",
        include_str!("../assets/syntaxes/protobuf.sublime-syntax"),
    ),
    (
        "graphql.sublime-syntax",
        include_str!("../assets/syntaxes/graphql.sublime-syntax"),
    ),
    (
        "hcl.sublime-syntax",
        include_str!("../assets/syntaxes/hcl.sublime-syntax"),
    ),
];

/// The TypeScript grammar's JSX hook; the TSX grammar replaces it with real tags.
const TS_JSX_STUB: &str = "  jsx: []\n";

/// TSX is TypeScript with JSX tags, made from the TypeScript grammar so the two never drift.
fn tsx_source() -> String {
    let ts = include_str!("../assets/syntaxes/typescript.sublime-syntax");
    let jsx = include_str!("../assets/syntaxes/tsx-jsx.sublime-syntax.part");
    ts.replacen("\nname: TypeScript\n", "\nname: TSX\n", 1)
        .replacen(
            "\nfile_extensions: [ts, mts, cts]\n",
            "\nfile_extensions: [tsx]\n",
            1,
        )
        .replacen("\nscope: source.ts\n", "\nscope: source.tsx\n", 1)
        .replacen(TS_JSX_STUB, jsx, 1)
}

/// Where the user's own grammars live.
pub fn languages_dir() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("languages"))
}

/// Syntect's defaults plus Ettin's grammars, parsed once per run.
fn builtin_definitions() -> &'static Vec<SyntaxDefinition> {
    static DEFS: OnceLock<Vec<SyntaxDefinition>> = OnceLock::new();
    DEFS.get_or_init(|| {
        let mut defs: Vec<SyntaxDefinition> = SyntaxSet::load_defaults_newlines()
            .into_builder()
            .syntaxes()
            .to_vec();
        let tsx = tsx_source();
        let sources = BUILTIN_SYNTAXES
            .iter()
            .copied()
            .chain(std::iter::once(("tsx.sublime-syntax", tsx.as_str())));
        let mut own = Vec::new();
        for (file, src) in sources {
            match SyntaxDefinition::load_from_str(src, true, None) {
                Ok(def) => own.push(def),
                // Covered by tests; never stop the editor over it.
                Err(e) => eprintln!("Ettin Edit: built-in grammar {file} failed to load: {e}"),
            }
        }
        layer(&mut defs, own);
        defs
    })
}

/// Adds `top` over `base`: a grammar in `top` replaces any in `base` with the same name.
fn layer(base: &mut Vec<SyntaxDefinition>, top: Vec<SyntaxDefinition>) {
    let names: HashSet<String> = top.iter().map(|d| d.name.clone()).collect();
    base.retain(|d| !names.contains(&d.name));
    base.extend(top);
}

/// Reads the user's grammars. Problems are reported per file and never stop the others.
pub fn load_user_definitions(dir: &Path) -> (Vec<SyntaxDefinition>, Vec<String>) {
    let mut defs: Vec<SyntaxDefinition> = Vec::new();
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (defs, errors);
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        let Some(file) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        let lower = file.to_lowercase();
        let result = if lower.ends_with(".sublime-syntax") {
            let stem = file[..file.len() - ".sublime-syntax".len()].to_owned();
            std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|src| {
                    SyntaxDefinition::load_from_str(&src, true, Some(&stem))
                        .map_err(|e| e.to_string())
                })
        } else if lower.ends_with(".lang.json") {
            std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|src| user_lang::to_sublime_syntax(&src))
                .and_then(|yaml| {
                    SyntaxDefinition::load_from_str(&yaml, true, None).map_err(|e| e.to_string())
                })
        } else {
            continue;
        };
        match result.and_then(|def| check_definition(&def).map(|()| def)) {
            Ok(def) => {
                // Two user files with one name: the later file (by name) wins.
                defs.retain(|d| d.name != def.name);
                defs.push(def);
            }
            Err(e) => errors.push(format!("Skipped the language file {file}: {e}")),
        }
    }
    (defs, errors)
}

/// Catches mistakes syntect only notices while highlighting: references to
/// contexts that don't exist (unstyled lines) and a broken first-line regex
/// (which would panic when detecting a language).
fn check_definition(def: &SyntaxDefinition) -> Result<(), String> {
    if !def.contexts.contains_key("main") {
        return Err("the grammar has no `main` context".to_owned());
    }
    if let Some(first_line) = &def.first_line_match
        && let Some(e) = syntect::parsing::Regex::try_compile(first_line)
    {
        return Err(format!("first_line_match is not a valid regex: {e}"));
    }
    let check = |r: &ContextReference| -> Result<(), String> {
        match r {
            ContextReference::Named { 0: name, .. } | ContextReference::Inline { 0: name, .. }
                if !def.contexts.contains_key(name) =>
            {
                Err(format!("the context `{name}` is used but never defined"))
            }
            _ => Ok(()),
        }
    };
    for context in def.contexts.values() {
        for pattern in &context.patterns {
            match pattern {
                Pattern::Include(r) => check(r)?,
                Pattern::Match(m) => {
                    if let MatchOperation::Push(refs) | MatchOperation::Set(refs) = &m.operation {
                        for r in refs {
                            check(r)?;
                        }
                    }
                    if let Some(r) = &m.with_prototype {
                        check(r)?;
                    }
                }
            }
        }
    }
    Ok(())
}

pub struct Highlighter {
    pub syntaxes: SyntaxSet,
}

impl Highlighter {
    /// The built-in grammars plus the user's from `user_dir` (normally
    /// [`languages_dir`]). Returns one message for each user file that failed to load.
    pub fn new(user_dir: Option<&Path>) -> (Self, Vec<String>) {
        let mut h = Self {
            syntaxes: SyntaxSet::new(),
        };
        let errors = h.reload(user_dir);
        (h, errors)
    }

    /// Rebuilds the grammar set, reading the user's folder again.
    ///
    /// Editors keep parser state that points into the old set, so callers
    /// should reset their highlighting caches afterwards.
    pub fn reload(&mut self, user_dir: Option<&Path>) -> Vec<String> {
        let mut defs = builtin_definitions().clone();
        let (user, errors) = match user_dir {
            Some(dir) => load_user_definitions(dir),
            None => (Vec::new(), Vec::new()),
        };
        layer(&mut defs, user);
        let mut builder = SyntaxSetBuilder::new();
        for def in defs {
            builder.add(def);
        }
        self.syntaxes = builder.build();
        errors
    }

    /// Language names for the Language menu, sorted case-insensitively.
    pub fn language_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .syntaxes
            .syntaxes()
            .iter()
            .filter(|s| !s.hidden)
            .map(|s| s.name.clone())
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup();
        names
    }

    /// Guesses the language from the file name, then from the first line (shebangs, `<?xml`).
    pub fn detect(&self, path: Option<&Path>, text: &str) -> String {
        let by_path = path.and_then(|p| {
            let name = p.file_name()?.to_str()?;
            self.syntaxes
                .find_syntax_by_extension(name)
                .or_else(|| self.extra_name(name))
                .or_else(|| {
                    let ext = p.extension()?.to_str()?;
                    self.syntaxes
                        .find_syntax_by_extension(ext)
                        .or_else(|| self.syntaxes.find_syntax_by_extension(&ext.to_lowercase()))
                })
                .or_else(|| self.extra_extension(name))
        });
        let syntax = by_path.or_else(|| {
            let first_line = text.lines().next().unwrap_or("");
            self.syntaxes.find_syntax_by_first_line(first_line)
        });
        syntax
            .map(|s| s.name.clone())
            .unwrap_or_else(|| PLAIN_TEXT.to_owned())
    }

    /// File names that carry their language in a prefix rather than the extension.
    fn extra_name(&self, file_name: &str) -> Option<&SyntaxReference> {
        let lower = file_name.to_lowercase();
        let target = if lower == ".env" || lower.starts_with(".env.") {
            "Dotenv"
        } else if lower.starts_with("dockerfile.") || lower.starts_with("containerfile.") {
            "Dockerfile"
        } else {
            return None;
        };
        self.syntaxes.find_syntax_by_name(target)
    }

    /// Common extensions no grammar claims, mapped to a close match.
    fn extra_extension(&self, file_name: &str) -> Option<&SyntaxReference> {
        let ext = file_name.rsplit('.').next()?.to_lowercase();
        let target = match ext.as_str() {
            "mjs" | "cjs" | "jsx" => "JavaScript",
            "xsd" | "xsl" | "xslt" | "svg" | "csproj" | "vcxproj" | "plist" => "XML",
            "jsonc" | "json5" => "JSON",
            "htm" | "xhtml" => "HTML",
            "zsh" | "fish" => "Bourne Again Shell (bash)",
            _ => return None,
        };
        self.syntaxes.find_syntax_by_name(target)
    }

    pub fn syntax(&self, name: &str) -> &SyntaxReference {
        self.syntaxes
            .find_syntax_by_name(name)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use syntect::parsing::{ParseState, ScopeStack};

    pub(crate) fn shared() -> &'static Highlighter {
        static H: OnceLock<Highlighter> = OnceLock::new();
        H.get_or_init(|| Highlighter::new(None).0)
    }

    /// Each token of `text` with its scopes (joined with spaces, innermost last).
    pub(crate) fn tokens(h: &Highlighter, language: &str, text: &str) -> Vec<(String, String)> {
        let syntax = h
            .syntaxes
            .find_syntax_by_name(language)
            .unwrap_or_else(|| panic!("no grammar named {language}"));
        let mut state = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        let mut out = Vec::new();
        for line in text.split_inclusive('\n') {
            let ops = state
                .parse_line(line, &h.syntaxes)
                .unwrap_or_else(|e| panic!("{language}: {e:?} on {line:?}"));
            let mut pos = 0;
            for (i, op) in ops {
                if i > pos {
                    out.push((line[pos..i].to_owned(), scopes(&stack)));
                    pos = i;
                }
                stack.apply(&op).unwrap();
            }
            if pos < line.len() {
                out.push((line[pos..].to_owned(), scopes(&stack)));
            }
        }
        out
    }

    fn scopes(stack: &ScopeStack) -> String {
        stack
            .as_slice()
            .iter()
            .map(|s| s.build_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Asserts that `token` appears in `text` with a scope containing `scope`.
    pub(crate) fn assert_scope(
        h: &Highlighter,
        language: &str,
        text: &str,
        token: &str,
        scope: &str,
    ) {
        let toks = tokens(h, language, text);
        assert!(
            toks.iter().any(|(t, s)| t == token && s.contains(scope)),
            "{language}: `{token}` should have scope {scope}; got {toks:#?}"
        );
    }

    #[test]
    fn detects_languages() {
        let h = shared();
        assert_eq!(h.detect(Some(Path::new("main.rs")), ""), "Rust");
        assert_eq!(h.detect(Some(Path::new("a.py")), ""), "Python");
        assert_eq!(h.detect(Some(Path::new("data.xsd")), ""), "XML");
        assert_eq!(h.detect(None, "#!/usr/bin/env python3\n"), "Python");
        assert_eq!(h.detect(Some(Path::new("notes")), "hello"), PLAIN_TEXT);
    }

    #[test]
    fn detects_new_languages() {
        let h = shared();
        let cases = [
            ("Cargo.toml", "TOML"),
            ("setup.cfg", "INI"),
            ("app.properties", "INI"),
            ("nginx.service", "INI"),
            ("x.desktop", "INI"),
            ("a.ts", "TypeScript"),
            ("a.mts", "TypeScript"),
            ("a.tsx", "TSX"),
            ("p.ps1", "PowerShell"),
            ("m.psd1", "PowerShell"),
            ("Dockerfile", "Dockerfile"),
            ("Containerfile", "Dockerfile"),
            ("web.dockerfile", "Dockerfile"),
            ("Dockerfile.dev", "Dockerfile"),
            ("adt.hl7", "HL7"),
            ("Main.kt", "Kotlin"),
            ("build.gradle.kts", "Kotlin"),
            ("App.swift", "Swift"),
            ("main.dart", "Dart"),
            ("build.zig", "Zig"),
            ("CMakeLists.txt", "CMake"),
            ("rules.cmake", "CMake"),
            ("nginx.conf", "Nginx"),
            (".env", "Dotenv"),
            (".env.local", "Dotenv"),
            ("api.proto", "Protocol Buffers"),
            ("schema.graphql", "GraphQL"),
            ("main.tf", "Terraform (HCL)"),
            ("vars.tfvars", "Terraform (HCL)"),
        ];
        for (file, want) in cases {
            assert_eq!(h.detect(Some(Path::new(file)), ""), want, "{file}");
        }
        assert_eq!(h.detect(None, "MSH|^~\\&|APP\n"), "HL7");
    }

    #[test]
    fn tsx_is_derived_from_typescript() {
        let ts = include_str!("../assets/syntaxes/typescript.sublime-syntax");
        assert!(ts.contains(TS_JSX_STUB));
        assert!(ts.contains("\nname: TypeScript\n"));
        assert!(ts.contains("\nfile_extensions: [ts, mts, cts]\n"));
        assert!(ts.contains("\nscope: source.ts\n"));
        let tsx = tsx_source();
        assert!(!tsx.contains(TS_JSX_STUB));
        assert!(tsx.contains("\nname: TSX\n"));
    }

    #[test]
    fn every_builtin_grammar_loads() {
        let names: Vec<String> = shared().language_names();
        let want = [
            "TOML",
            "INI",
            "TypeScript",
            "TSX",
            "PowerShell",
            "Dockerfile",
            "HL7",
            "Kotlin",
            "Swift",
            "Dart",
            "Zig",
            "CMake",
            "Nginx",
            "Dotenv",
            "Protocol Buffers",
            "GraphQL",
            "Terraform (HCL)",
        ];
        for name in want {
            assert!(names.iter().any(|n| n == name), "missing {name}");
        }
        // Keep the list above in step with the bundled files (+1 for TSX).
        assert_eq!(BUILTIN_SYNTAXES.len() + 1, want.len());
    }

    #[test]
    fn toml() {
        let h = shared();
        let src = "# comment\n[package]\nname = \"ettin\" # trailing\nversion.major = 1_000\n\
                   flags = [true, false, 0x1F, 1e3, inf]\nwhen = 1979-05-27T07:32:00Z\n\
                   [[bin]]\ninline = { a = 'lit', b = \"\"\"multi\nline\"\"\" }\n";
        assert_scope(h, "TOML", src, "name", "entity.name.tag");
        assert_scope(h, "TOML", src, "package", "entity.name.section");
        assert_scope(h, "TOML", src, "bin", "entity.name.section");
        assert_scope(h, "TOML", src, "1_000", "constant.numeric");
        assert_scope(h, "TOML", src, "0x1F", "constant.numeric");
        assert_scope(h, "TOML", src, "true", "constant.language");
        assert_scope(
            h,
            "TOML",
            src,
            "1979-05-27T07:32:00Z",
            "constant.other.datetime",
        );
        assert_scope(h, "TOML", src, "line", "string.quoted.triple");
        assert_scope(h, "TOML", src, "# comment", "comment.line");
        assert_scope(h, "TOML", src, "b", "entity.name.tag");
    }

    #[test]
    fn ini() {
        let h = shared();
        let src = "; comment\n[Unit]\nDescription=Ettin\n# hash comment\nExecStart = /usr/bin/x \"%h\"\n\
                   [HKEY_CURRENT_USER\\Software]\n\"Value\"=dword:00000001\n@=\"default\"\n";
        assert_scope(h, "INI", src, "Unit", "entity.name.section");
        assert_scope(h, "INI", src, "Description", "entity.name.tag");
        assert_scope(h, "INI", src, "; comment", "comment.line");
        assert_scope(h, "INI", src, "# hash comment", "comment.line");
        assert_scope(h, "INI", src, "dword", "storage.type");
        assert_scope(h, "INI", src, "00000001", "constant.numeric");
    }

    #[test]
    fn typescript() {
        let h = shared();
        let src = "import { A } from './a';\nexport interface Shape<T> extends Base {\n  readonly id: number; // id\n}\n\
                   type Id = string | null;\nenum Color { Red = 1 }\nabstract class Box implements Shape<string> {\n\
                   \x20 private async load(x: number): Promise<void> { return await fetch(`/a/${x}`); }\n}\n\
                   const add = (a: number, b = 2) => a + b;\nfunction go(n: number) { if (n > 0x10) throw new Error(\"x\"); }\n\
                   @Component({})\nlet re = /ab+c/gi;\n";
        assert_scope(h, "TypeScript", src, "interface", "storage.type");
        assert_scope(h, "TypeScript", src, "Shape", "entity.name");
        assert_scope(h, "TypeScript", src, "import", "keyword.control");
        assert_scope(h, "TypeScript", src, "readonly", "storage.modifier");
        assert_scope(h, "TypeScript", src, "number", "support.type");
        assert_scope(h, "TypeScript", src, "// id", "comment.line");
        assert_scope(h, "TypeScript", src, "Box", "entity.name.class");
        assert_scope(h, "TypeScript", src, "add", "entity.name.function");
        assert_scope(h, "TypeScript", src, "go", "entity.name.function");
        assert_scope(h, "TypeScript", src, "0x10", "constant.numeric");
        assert_scope(h, "TypeScript", src, "x", "meta.interpolation");
        assert_scope(h, "TypeScript", src, "/ab+c/gi", "string.regexp");
        assert_scope(h, "TypeScript", src, "null", "constant.language");
    }

    #[test]
    fn tsx() {
        let h = shared();
        let src = "export function App({ name }: Props) {\n  return <div className=\"app\" onClick={() => go(1)}>\n\
                   \x20   Hello {name}<br />\n  </div>;\n}\nconst x = f<string>(a);\n";
        assert_scope(h, "TSX", src, "div", "entity.name.tag");
        assert_scope(h, "TSX", src, "className", "entity.other.attribute-name");
        assert_scope(h, "TSX", src, "export", "keyword.control");
        assert_scope(h, "TSX", src, "1", "constant.numeric");
        assert_scope(h, "TSX", src, "string", "support.type");
    }

    #[test]
    fn powershell() {
        let h = shared();
        let src = "<# block\ncomment #>\nfunction Get-Thing {\n  [CmdletBinding()]\n  param([string]$Name = 'x')\n\
                   \x20 if ($Name -eq $null) { Write-Host \"Hi $Name $($env:USER)\" -ForegroundColor Red }\n}\n# done\n";
        assert_scope(h, "PowerShell", src, "function", "storage.type");
        assert_scope(h, "PowerShell", src, "Get-Thing", "entity.name.function");
        assert_scope(h, "PowerShell", src, "Write-Host", "support.function");
        assert_scope(h, "PowerShell", src, "-eq", "keyword.operator");
        assert_scope(h, "PowerShell", src, "$Name", "variable");
        assert_scope(h, "PowerShell", src, "$null", "constant.language");
        assert_scope(
            h,
            "PowerShell",
            src,
            "-ForegroundColor",
            "variable.parameter",
        );
        assert_scope(h, "PowerShell", src, "<#", "comment.block");
        assert_scope(h, "PowerShell", src, "if", "keyword.control");
    }

    #[test]
    fn dockerfile() {
        let h = shared();
        let src = "# syntax=docker/dockerfile:1\nFROM rust:1.80 AS build\nARG MODE=release\n\
                   RUN cargo build --$MODE && \\\n    strip target/x\nCOPY --from=build /a /b\n\
                   CMD [\"./x\", \"--serve\"]\n";
        assert_scope(h, "Dockerfile", src, "FROM", "keyword.control");
        assert_scope(h, "Dockerfile", src, "AS", "keyword.control");
        assert_scope(h, "Dockerfile", src, "$MODE", "variable");
        assert_scope(h, "Dockerfile", src, "--from", "variable.parameter");
        assert_scope(h, "Dockerfile", src, "./x", "string.quoted.double");
    }

    #[test]
    fn hl7_standard_separators() {
        let h = shared();
        let src = "MSH|^~\\&|SEND|FAC|||20240101120000||ADT^A01|MSG1|P|2.5\r\n\
                   PID|1||123^^^HOSP^MR~456||Doe^John\\T\\Jr|A&B|19800101|M\r\n";
        assert_scope(h, "HL7", src, "MSH", "entity.name.tag");
        assert_scope(h, "HL7", src, "PID", "entity.name.tag");
        assert_scope(h, "HL7", src, "^", "constant.character.encoding");
        assert_scope(h, "HL7", src, "|", "keyword.operator.separator.field");
        assert_scope(h, "HL7", src, "^", "punctuation.separator.component");
        assert_scope(h, "HL7", src, "~", "punctuation.separator.repetition");
        assert_scope(h, "HL7", src, "&", "punctuation.separator.subcomponent");
        assert_scope(h, "HL7", src, "\\T\\", "constant.character.escape");
        // A fragment without a header still gets the standard separators.
        assert_scope(h, "HL7", "PID|1^2\n", "^", "separator.component");
    }

    #[test]
    fn hl7_separators_come_from_msh() {
        let h = shared();
        // Field `#`, component `$`, repetition `*`, escape `@`, subcomponent `!`.
        let src = "MSH#$*@!#SEND#FAC\nPID#1##123$$$HOSP*456!X#Doe@T@Jr#a|b^c\n";
        let toks = tokens(h, "HL7", src);
        let scope_of = |t: &str| {
            toks.iter()
                .filter(|(tok, _)| tok == t)
                .map(|(_, s)| s.clone())
                .collect::<Vec<_>>()
        };
        assert!(!scope_of("#").is_empty());
        assert!(scope_of("#").iter().all(|s| s.contains("separator.field")));
        assert!(
            scope_of("$")
                .iter()
                .any(|s| s.contains("separator.component"))
        );
        assert!(
            scope_of("*")
                .iter()
                .any(|s| s.contains("separator.repetition"))
        );
        assert!(
            scope_of("!")
                .iter()
                .any(|s| s.contains("separator.subcomponent"))
        );
        assert!(scope_of("@T@").iter().any(|s| s.contains("escape")));
        // The standard characters are plain text in this message.
        assert!(
            toks.iter()
                .any(|(t, s)| t.contains("a|b^c") && !s.contains("separator")),
            "{toks:#?}"
        );
    }

    #[test]
    fn kotlin() {
        let h = shared();
        let src = "package a.b\n/* doc */\ndata class User(val name: String, var age: Int = 0)\n\
                   fun main(args: Array<String>) { val s = \"hi ${args.size} $x\"; println(s) }\n\
                   object Registry\ninterface Shape\n";
        assert_scope(h, "Kotlin", src, "class", "storage.type");
        assert_scope(h, "Kotlin", src, "User", "entity.name.class");
        assert_scope(h, "Kotlin", src, "main", "entity.name.function");
        assert_scope(h, "Kotlin", src, "val", "storage.type");
        assert_scope(h, "Kotlin", src, "$x", "variable");
        assert_scope(h, "Kotlin", src, " doc ", "comment.block");
        assert_scope(h, "Kotlin", src, "0", "constant.numeric");
    }

    #[test]
    fn swift() {
        let h = shared();
        let src = "import UIKit\nstruct Point { let x: Double }\nprotocol Drawable {}\n\
                   func draw(_ p: Point) -> Bool { guard p.x > 0 else { return false }; print(\"x=\\(p.x)\"); return true }\n\
                   @MainActor final class View: Drawable {}\n";
        assert_scope(h, "Swift", src, "struct", "storage.type");
        assert_scope(h, "Swift", src, "Point", "entity.name");
        assert_scope(h, "Swift", src, "draw", "entity.name.function");
        assert_scope(h, "Swift", src, "guard", "keyword.control");
        assert_scope(h, "Swift", src, "false", "constant.language");
        assert_scope(h, "Swift", src, "@MainActor", "storage.modifier.attribute");
    }

    #[test]
    fn dart() {
        let h = shared();
        let src = "import 'package:flutter/material.dart';\nclass App extends StatelessWidget {\n\
                   \x20 @override\n  Widget build(BuildContext context) => Text('Hi $name ${x + 1}');\n}\n\
                   Future<void> main() async { final n = 0xFF; }\n";
        assert_scope(h, "Dart", src, "class", "storage.type");
        assert_scope(h, "Dart", src, "App", "entity.name.class");
        assert_scope(h, "Dart", src, "build", "entity.name.function");
        assert_scope(h, "Dart", src, "async", "storage.modifier");
        assert_scope(h, "Dart", src, "0xFF", "constant.numeric");
        assert_scope(h, "Dart", src, "$name", "variable");
    }

    #[test]
    fn zig() {
        let h = shared();
        let src = "const std = @import(\"std\");\npub fn main() !void {\n    var x: u32 = 0b1010;\n\
                   \x20   // note\n    try std.debug.print(\"{d}\\n\", .{x});\n}\nconst S = struct { a: i64 };\n";
        assert_scope(h, "Zig", src, "fn", "storage.type");
        assert_scope(h, "Zig", src, "main", "entity.name.function");
        assert_scope(h, "Zig", src, "@import", "support.function");
        assert_scope(h, "Zig", src, "u32", "storage.type");
        assert_scope(h, "Zig", src, "try", "keyword.control");
        assert_scope(h, "Zig", src, "0b1010", "constant.numeric");
    }

    #[test]
    fn cmake() {
        let h = shared();
        let src = "cmake_minimum_required(VERSION 3.20)\n# comment\nproject(ettin LANGUAGES CXX)\n\
                   set(SRC \"${CMAKE_SOURCE_DIR}/main.cpp\")\nif(WIN32)\n  add_executable(app ${SRC})\nendif()\n\
                   function(my_fn arg)\nendfunction()\n";
        assert_scope(h, "CMake", src, "project", "support.function");
        assert_scope(h, "CMake", src, "if", "keyword.control");
        assert_scope(h, "CMake", src, "${CMAKE_SOURCE_DIR}", "variable");
        assert_scope(h, "CMake", src, "VERSION", "constant");
        assert_scope(h, "CMake", src, "my_fn", "entity.name.function");
    }

    #[test]
    fn nginx() {
        let h = shared();
        let src = "user www-data;\nhttp {\n  server {\n    listen 443 ssl; # tls\n    server_name example.org;\n\
                   \x20   location ~* \\.(png|jpg)$ { root /var/www; }\n    set $x \"y\";\n  }\n}\n";
        assert_scope(h, "Nginx", src, "server", "entity.name.section");
        assert_scope(h, "Nginx", src, "listen", "keyword");
        assert_scope(h, "Nginx", src, "443", "constant.numeric");
        assert_scope(h, "Nginx", src, "$x", "variable");
        assert_scope(h, "Nginx", src, "# tls", "comment.line");
    }

    #[test]
    fn dotenv() {
        let h = shared();
        let src =
            "# settings\nexport API_KEY=abc123\nURL=\"http://${HOST}:8080\"\nEMPTY=\nRAW='x$y'\n";
        assert_scope(h, "Dotenv", src, "API_KEY", "variable.other");
        assert_scope(h, "Dotenv", src, "export", "storage.modifier");
        assert_scope(h, "Dotenv", src, "${HOST}", "variable");
        assert_scope(h, "Dotenv", src, "# settings", "comment.line");
    }

    #[test]
    fn protobuf() {
        let h = shared();
        let src = "syntax = \"proto3\";\npackage api.v1;\nmessage User {\n  string name = 1; // n\n\
                   \x20 repeated int64 ids = 2 [packed = true];\n  map<string, int32> tags = 3;\n}\n\
                   service Users { rpc Get(GetRequest) returns (User); }\nenum Kind { KIND_UNKNOWN = 0; }\n";
        let p = "Protocol Buffers";
        assert_scope(h, p, src, "message", "storage.type");
        assert_scope(h, p, src, "User", "entity.name");
        assert_scope(h, p, src, "string", "storage.type");
        assert_scope(h, p, src, "Get", "entity.name.function");
        assert_scope(h, p, src, "repeated", "storage.modifier");
        assert_scope(h, p, src, "1", "constant.numeric");
    }

    #[test]
    fn graphql() {
        let h = shared();
        let src = "# schema\ntype User implements Node @key(fields: \"id\") {\n  id: ID!\n  posts(first: Int = 10): [Post]\n}\n\
                   query GetUser($id: ID!) {\n  user(id: $id) { ...UserFields }\n}\nfragment UserFields on User { name }\n";
        assert_scope(h, "GraphQL", src, "type", "storage.type");
        assert_scope(h, "GraphQL", src, "User", "entity.name");
        assert_scope(h, "GraphQL", src, "query", "keyword");
        assert_scope(h, "GraphQL", src, "GetUser", "entity.name.function");
        assert_scope(h, "GraphQL", src, "$id", "variable");
        assert_scope(h, "GraphQL", src, "@key", "storage.modifier");
        assert_scope(h, "GraphQL", src, "10", "constant.numeric");
    }

    #[test]
    fn hcl() {
        let h = shared();
        let src = "# main\nresource \"aws_s3_bucket\" \"logs\" {\n  bucket = \"logs-${var.env}\"\n  count  = 2\n\
                   \x20 tags = { Name = \"x\" }\n  enabled = true\n}\nvariable \"env\" {\n  default = <<EOT\nprod\nEOT\n}\n\
                   locals { n = length(var.list) }\n";
        let t = "Terraform (HCL)";
        assert_scope(h, t, src, "resource", "storage.type");
        assert_scope(h, t, src, "bucket", "variable.other");
        assert_scope(h, t, src, "var.env", "meta.interpolation");
        assert_scope(h, t, src, "2", "constant.numeric");
        assert_scope(h, t, src, "true", "constant.language");
        assert_scope(h, t, src, "prod\n", "string.unquoted.heredoc");
        assert_scope(h, t, src, "length", "support.function");
    }

    #[test]
    fn user_languages_load_and_win() {
        let dir = std::env::temp_dir().join(format!("ettin-langs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mylang.lang.json"), user_lang::UDL_TEMPLATE).unwrap();
        // Same name as a built-in: the user's copy wins.
        std::fs::write(
            dir.join("toml.sublime-syntax"),
            "%YAML 1.2\n---\nname: TOML\nfile_extensions: [toml]\nscope: source.toml\n\
             contexts:\n  main:\n    - match: '\\bwow\\b'\n      scope: keyword.control.mine\n",
        )
        .unwrap();
        std::fs::write(dir.join("broken.sublime-syntax"), "name: [unclosed").unwrap();
        std::fs::write(dir.join("bad.lang.json"), "{ \"name\": 1 }").unwrap();
        std::fs::write(
            dir.join("missing.sublime-syntax"),
            "%YAML 1.2\n---\nname: Missing\nscope: source.missing\ncontexts:\n  main:\n    - include: nowhere\n",
        )
        .unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();

        let (h, errors) = Highlighter::new(Some(&dir));
        assert_eq!(errors.len(), 3, "{errors:#?}");
        assert!(errors.iter().any(|e| e.contains("broken.sublime-syntax")));
        assert!(errors.iter().any(|e| e.contains("bad.lang.json")));
        assert!(errors.iter().any(|e| e.contains("missing.sublime-syntax")));

        assert_eq!(h.detect(Some(Path::new("x.mylang")), ""), "My Language");
        assert_scope(
            &h,
            "My Language",
            "if x then // note\n",
            "if",
            "keyword.control",
        );
        assert_scope(&h, "TOML", "wow\n", "wow", "keyword.control.mine");
        assert_eq!(
            h.language_names().iter().filter(|n| *n == "TOML").count(),
            1
        );

        // Reloading picks up removals.
        std::fs::remove_dir_all(&dir).unwrap();
        let mut h = h;
        assert!(h.reload(Some(&dir)).is_empty());
        assert!(!h.language_names().iter().any(|n| n == "My Language"));
        assert_scope(&h, "TOML", "[a]\n", "a", "entity.name.section");
    }
}
