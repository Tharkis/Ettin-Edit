//! Function list: the outline of a document (functions, classes, sections,
//! headings, keys), found by small line-based parsers, one per language family.
//!
//! These are fast heuristics rather than compilers. They skip comments and
//! strings and are meant to be right for ordinary, conventionally formatted
//! code; a 5 MB file takes a fraction of a second.

use std::sync::LazyLock;

use regex::Regex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    /// A function inside a class, struct, impl, trait or interface.
    Method,
    /// Classes and objects; also Rust `impl` blocks and Swift/Dart extensions.
    Class,
    Struct,
    Enum,
    /// Traits, interfaces, protocols and type aliases.
    Trait,
    /// Modules, namespaces and packages.
    Module,
    /// INI/TOML sections, HL7 segments, CSS at-rules.
    Section,
    /// Markdown headings.
    Heading,
    /// JSON/YAML keys, CSS selectors, XML ids, SQL indexes.
    Key,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// 0-based line, counted the way the editor counts lines.
    pub line: usize,
    /// Nesting level: 0 for top level, 1 inside one enclosing symbol, …
    pub depth: usize,
}

/// The symbols of `text`, in document order, for the language named as in the
/// Language menu. Unknown languages give an empty list.
pub fn symbols(language: &str, text: &str) -> Vec<Symbol> {
    match language {
        "Rust" => braces(text, &RUST),
        "C" | "C++" | "Objective-C" | "Objective-C++" => braces(text, &C_FAMILY),
        "C#" => braces(text, &CSHARP),
        "Java" | "Groovy" => braces(text, &JAVA),
        "Kotlin" => braces(text, &KOTLIN),
        "Scala" => braces(text, &SCALA),
        "Swift" => braces(text, &SWIFT),
        "Dart" => braces(text, &DART),
        "Zig" => braces(text, &ZIG),
        "Go" => braces(text, &GO),
        "JavaScript" | "JavaScript (Rails)" | "TypeScript" | "TSX" => braces(text, &JS),
        "PHP" => braces(text, &PHP),
        "PowerShell" => braces(text, &POWERSHELL),
        "Bourne Again Shell (bash)" | "Shell-Unix-Generic" => braces(text, &SHELL),
        "Python" => indented(text, &PYTHON),
        "Ruby" | "Ruby on Rails" => indented(text, &RUBY),
        "Lua" => indented(text, &LUA),
        "Batch File" => batch(text),
        "SQL" | "SQL (Rails)" => sql(text),
        "Markdown" | "MultiMarkdown" => markdown(text),
        "INI" | "TOML" => ini_sections(text),
        "YAML" => yaml(text),
        "JSON" => json(text),
        "XML" | "HTML" | "SVG" => xml_ids(text),
        "HL7" => hl7(text),
        "CSS" => css(text),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Lines

/// Splits text into lines the way the editor's rope does: LF, CRLF, CR,
/// VT, FF, NEL, LS and PS all end a line.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut i = 0;
    let mut done = false;
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        while i < bytes.len() {
            let brk = match bytes[i] {
                b'\n' | 0x0B | 0x0C => Some(1),
                b'\r' => Some(if bytes.get(i + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                }),
                0xC2 if bytes.get(i + 1) == Some(&0x85) => Some(2),
                0xE2 if bytes.get(i + 1) == Some(&0x80)
                    && matches!(bytes.get(i + 2), Some(0xA8 | 0xA9)) =>
                {
                    Some(3)
                }
                _ => None,
            };
            if let Some(len) = brk {
                let line = &text[start..i];
                i += len;
                start = i;
                return Some(line);
            }
            i += 1;
        }
        done = true;
        Some(&text[start..])
    })
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Comment and string stripping

#[derive(Clone, Copy)]
struct Lexis {
    line_comments: &'static [&'static str],
    block_comment: Option<(&'static str, &'static str)>,
    /// Block comments nest (Rust, Kotlin, Swift, Dart, Scala).
    nested_comments: bool,
    /// `=begin` / `=end` style: the block comment markers only count at the start of a line.
    block_at_line_start: bool,
    /// Single-byte string delimiters whose strings end with the line.
    quotes: &'static [u8],
    /// Delimiters whose strings may span lines (JS/Go backticks).
    multiline_quotes: &'static [u8],
    /// `"""` / `'''` strings spanning lines.
    triple_quotes: bool,
    /// `'` starts a char literal only when it looks like one (Rust lifetimes),
    /// and `r"…"` / `r#"…"#` are raw strings.
    rust_quotes: bool,
    escape: Option<u8>,
    /// Shell: `#` starts a comment only at the start of a word.
    hash_at_word_start: bool,
}

const fn lexis() -> Lexis {
    Lexis {
        line_comments: &[],
        block_comment: None,
        nested_comments: false,
        block_at_line_start: false,
        quotes: &[],
        multiline_quotes: &[],
        triple_quotes: false,
        rust_quotes: false,
        escape: Some(b'\\'),
        hash_at_word_start: false,
    }
}

const C_LEXIS: Lexis = Lexis {
    line_comments: &["//"],
    block_comment: Some(("/*", "*/")),
    quotes: b"\"'",
    ..lexis()
};

#[derive(Clone, Copy, PartialEq)]
enum LexState {
    Code,
    Block(usize),
    Str(u8),
    Triple(u8),
    Raw(usize),
}

struct Scanner {
    lex: Lexis,
    state: LexState,
    out: Vec<u8>,
}

impl Scanner {
    fn new(lex: Lexis) -> Self {
        Self {
            lex,
            state: LexState::Code,
            out: Vec::new(),
        }
    }

    /// The code of `line`: comments dropped, string contents removed (the
    /// quotes stay, so `"x"` becomes `""`).
    fn clean(&mut self, line: &str) -> &str {
        self.out.clear();
        let b = line.as_bytes();
        let lex = self.lex;
        if lex.block_at_line_start
            && let Some((open, close)) = lex.block_comment
        {
            if let LexState::Block(_) = self.state {
                if line.starts_with(close) {
                    self.state = LexState::Code;
                }
                return "";
            }
            if line.starts_with(open) {
                self.state = LexState::Block(1);
                return "";
            }
        }
        let mut i = 0;
        while i < b.len() {
            match self.state {
                LexState::Block(depth) => {
                    let (open, close) = lex.block_comment.unwrap_or(("\0", "\0"));
                    if b[i..].starts_with(close.as_bytes()) {
                        i += close.len();
                        self.state = if depth > 1 {
                            LexState::Block(depth - 1)
                        } else {
                            self.out.push(b' ');
                            LexState::Code
                        };
                    } else if lex.nested_comments && b[i..].starts_with(open.as_bytes()) {
                        i += open.len();
                        self.state = LexState::Block(depth + 1);
                    } else {
                        i += 1;
                    }
                }
                LexState::Str(q) => {
                    if Some(b[i]) == lex.escape && i + 1 < b.len() {
                        i += 2;
                    } else if b[i] == q {
                        self.out.push(q);
                        i += 1;
                        self.state = LexState::Code;
                    } else {
                        i += 1;
                    }
                }
                LexState::Triple(q) => {
                    if Some(b[i]) == lex.escape && i + 1 < b.len() {
                        i += 2;
                    } else if b[i..].starts_with(&[q, q, q]) {
                        self.out.extend_from_slice(&[q, q, q]);
                        i += 3;
                        self.state = LexState::Code;
                    } else {
                        i += 1;
                    }
                }
                LexState::Raw(hashes) => {
                    if b[i] == b'"'
                        && b[i + 1..].iter().take_while(|&&c| c == b'#').count() >= hashes
                    {
                        self.out.push(b'"');
                        i += 1 + hashes;
                        self.state = LexState::Code;
                    } else {
                        i += 1;
                    }
                }
                LexState::Code => {
                    let c = b[i];
                    let rest = &b[i..];
                    if let Some((open, _)) = lex.block_comment
                        && !lex.block_at_line_start
                        && rest.starts_with(open.as_bytes())
                    {
                        self.state = LexState::Block(1);
                        i += open.len();
                        continue;
                    }
                    if let Some(lc) = lex
                        .line_comments
                        .iter()
                        .find(|lc| rest.starts_with(lc.as_bytes()))
                    {
                        let word_start = i == 0 || b[i - 1].is_ascii_whitespace();
                        if !(lex.hash_at_word_start && lc.starts_with('#') && !word_start) {
                            break;
                        }
                    }
                    if lex.triple_quotes
                        && (rest.starts_with(b"\"\"\"") || rest.starts_with(b"'''"))
                    {
                        self.out.extend_from_slice(&rest[..3]);
                        self.state = LexState::Triple(c);
                        i += 3;
                        continue;
                    }
                    if lex.rust_quotes {
                        if c == b'r'
                            && (i == 0 || !is_word(b[i - 1]))
                            && let Some(h) = raw_string_start(&rest[1..])
                        {
                            self.out.push(b'"');
                            self.state = LexState::Raw(h);
                            i += 2 + h;
                            continue;
                        }
                        if c == b'\'' {
                            // 'x', '\n', '\u{1F600}' are chars; 'a is a lifetime.
                            if let Some(len) = char_literal_len(rest) {
                                self.out.extend_from_slice(b"''");
                                i += len;
                            } else {
                                self.out.push(c);
                                i += 1;
                            }
                            continue;
                        }
                    }
                    if lex.quotes.contains(&c) || lex.multiline_quotes.contains(&c) {
                        self.out.push(c);
                        self.state = LexState::Str(c);
                        i += 1;
                        continue;
                    }
                    self.out.push(c);
                    i += 1;
                }
            }
        }
        // Ordinary strings end with the line.
        if let LexState::Str(q) = self.state
            && !lex.multiline_quotes.contains(&q)
        {
            self.state = LexState::Code;
        }
        // Only ASCII delimiters are cut, so the bytes are still UTF-8.
        std::str::from_utf8(&self.out).unwrap_or("")
    }
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// After an `r`: `"`, `#"`, `##"` … starts a raw string. Returns the number of `#`.
fn raw_string_start(rest: &[u8]) -> Option<usize> {
    let hashes = rest.iter().take_while(|&&c| c == b'#').count();
    (rest.get(hashes) == Some(&b'"')).then_some(hashes)
}

/// Length of a Rust char literal at the start of `s` (which starts with `'`).
fn char_literal_len(s: &[u8]) -> Option<usize> {
    if *s.get(1)? == b'\\' {
        let end = s[2..].iter().position(|&c| c == b'\'')?;
        return Some(end + 3);
    }
    // One character, which may take up to four UTF-8 bytes.
    let len = match s[1] {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    };
    (s.get(1 + len) == Some(&b'\'')).then_some(len + 2)
}

// ---------------------------------------------------------------------------
// Brace languages

struct Candidate {
    name: String,
    kind: SymbolKind,
    /// Byte offset in the cleaned line where the declaration starts.
    start: usize,
    /// Only a declaration if a body `{` (or `=>`) follows before a `;`.
    needs_body: bool,
}

#[derive(Clone, Copy)]
struct Ctx {
    /// Inside a function body.
    in_code: bool,
    /// The nearest enclosing symbol.
    parent: Option<SymbolKind>,
}

impl Ctx {
    /// Directly inside a type's body, where functions are methods.
    fn in_type(&self) -> bool {
        !self.in_code
            && matches!(
                self.parent,
                Some(SymbolKind::Class | SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Trait)
            )
    }
}

struct BraceLang {
    lexis: Lexis,
    detect: fn(&str, Ctx) -> Option<Candidate>,
    /// Statements end with `;`, so a body may follow a few lines later.
    semicolons: bool,
    /// `=> expr` counts as a body (C#, Dart).
    arrow_body: bool,
    /// Lines starting with `#` are preprocessor lines (C family).
    skip_hash_lines: bool,
}

struct Brace {
    code: bool,
    symbol: Option<SymbolKind>,
}

struct Pending {
    symbol: Symbol,
    emitted: bool,
    /// Where the declaration starts in its line.
    start: usize,
    parens: i32,
    lines_left: u8,
}

fn braces(text: &str, lang: &BraceLang) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut scanner = Scanner::new(lang.lexis);
    let mut stack: Vec<Brace> = Vec::new();
    let mut pending: Option<Pending> = None;
    for (line_no, line) in lines(text).enumerate() {
        let code = scanner.clean(line);
        let trimmed = code.trim_start();
        if trimmed.is_empty() || (lang.skip_hash_lines && trimmed.starts_with('#')) {
            continue;
        }

        // Without semicolons, a body must follow straight after the signature.
        if let Some(p) = &pending
            && p.symbol.line < line_no
            && p.parens <= 0
            && !lang.semicolons
            && !trimmed.starts_with('{')
            && !(lang.arrow_body && trimmed.starts_with("=>"))
        {
            pending = None;
        }

        // A line is scanned in segments: after a `{` that opens a symbol's
        // body, the rest of the line may declare members (`trait T { fn f(); }`).
        let mut from = 0;
        loop {
            let ctx = Ctx {
                in_code: stack.last().is_some_and(|b| b.code),
                parent: stack.iter().rev().find_map(|b| b.symbol),
            };
            if pending.is_none()
                && let Some(c) = (lang.detect)(&code[from..], ctx)
            {
                let kind = if c.kind == SymbolKind::Function && ctx.in_type() {
                    SymbolKind::Method
                } else {
                    c.kind
                };
                let symbol = Symbol {
                    name: c.name,
                    kind,
                    line: line_no,
                    depth: stack.iter().filter(|b| b.symbol.is_some()).count(),
                };
                if !c.needs_body {
                    out.push(symbol.clone());
                }
                pending = Some(Pending {
                    symbol,
                    emitted: !c.needs_body,
                    start: from + c.start,
                    parens: 0,
                    lines_left: 6,
                });
            }

            let bytes = code.as_bytes();
            let mut next = None;
            for i in from..bytes.len() {
                let b = bytes[i];
                let active = pending
                    .as_ref()
                    .is_some_and(|p| p.symbol.line < line_no || i >= p.start);
                let settled = active && pending.as_ref().is_some_and(|p| p.parens <= 0);
                match b {
                    b'(' | b'[' if active => {
                        if let Some(p) = &mut pending {
                            p.parens += 1;
                        }
                    }
                    b')' | b']' if active => {
                        if let Some(p) = &mut pending {
                            p.parens -= 1;
                        }
                    }
                    b'{' => {
                        let opened = pending.take_if(|_| settled);
                        let entry = match &opened {
                            Some(p) => {
                                if !p.emitted {
                                    out.push(p.symbol.clone());
                                }
                                Brace {
                                    code: matches!(
                                        p.symbol.kind,
                                        SymbolKind::Function | SymbolKind::Method
                                    ),
                                    symbol: Some(p.symbol.kind),
                                }
                            }
                            None => Brace {
                                code: stack.last().is_some_and(|b| b.code),
                                symbol: None,
                            },
                        };
                        let container = entry.symbol.is_some() && !entry.code;
                        stack.push(entry);
                        if container && !code[i + 1..].trim().is_empty() {
                            next = Some(i + 1);
                            break;
                        }
                    }
                    b'}' => {
                        stack.pop();
                        // `trait T { def f: Int }`: an unfinished member ends here.
                        if pending
                            .as_ref()
                            .is_some_and(|p| p.symbol.line == line_no && p.parens <= 0)
                        {
                            pending = None;
                        }
                    }
                    b';' if settled => {
                        pending = None;
                        if stack.last().is_some_and(|b| b.symbol.is_some() && !b.code)
                            && !code[i + 1..].trim().is_empty()
                        {
                            next = Some(i + 1);
                            break;
                        }
                    }
                    b'=' if settled && lang.arrow_body && bytes.get(i + 1) == Some(&b'>') => {
                        if let Some(p) = pending.take()
                            && !p.emitted
                        {
                            out.push(p.symbol);
                        }
                    }
                    _ => {}
                }
            }
            match next {
                Some(n) => from = n,
                None => break,
            }
        }

        if let Some(p) = &mut pending
            && p.symbol.line < line_no
        {
            p.lines_left = p.lines_left.saturating_sub(1);
            if p.lines_left == 0 {
                pending = None;
            }
        }
    }
    out
}

/// A candidate named by capture group `group`.
fn cand(
    caps: &regex::Captures,
    group: usize,
    kind: SymbolKind,
    needs_body: bool,
) -> Option<Candidate> {
    let m = caps.get(group)?;
    Some(Candidate {
        name: collapse(m.as_str()),
        kind,
        start: caps.get(0).map_or(0, |m| m.start()),
        needs_body,
    })
}

macro_rules! re {
    ($name:ident, $pattern:expr) => {
        static $name: LazyLock<Regex> =
            LazyLock::new(|| Regex::new($pattern).expect("valid regex"));
    };
}

/// A cheap test run before the regexes, which are slow in debug builds.
fn mentions(code: &str, words: &[&str]) -> bool {
    words.iter().any(|w| code.contains(w))
}

/// Keywords that look like calls (`if (`) in C-like languages.
fn is_control_word(name: &str) -> bool {
    matches!(
        name,
        "if" | "for"
            | "foreach"
            | "while"
            | "switch"
            | "catch"
            | "return"
            | "sizeof"
            | "alignof"
            | "decltype"
            | "typeof"
            | "else"
            | "do"
            | "case"
            | "new"
            | "delete"
            | "throw"
            | "using"
            | "lock"
            | "fixed"
            | "when"
            | "synchronized"
            | "static_assert"
            | "defined"
            | "await"
            | "super"
            | "this"
            | "base"
            | "assert"
            | "function"
            | "elif"
            | "with"
            | "try"
            | "yield"
            | "guard"
    )
}

// Rust ---------------------------------------------------------------------

re!(
    RUST_FN,
    r#"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:default\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+(?:""\s+)?)?fn\s+([A-Za-z_]\w*)"#
);
re!(
    RUST_TYPE,
    r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:unsafe\s+)?(?:auto\s+)?(struct|enum|union|trait|mod)\s+([A-Za-z_]\w*)"
);
re!(RUST_IMPL, r"^\s*(?:unsafe\s+)?impl\b");
re!(RUST_MACRO, r"^\s*macro_rules!\s*([A-Za-z_]\w*)");

/// `impl<T: X<Y>> Display for Foo<T> where …` → `impl Display for Foo<T>`.
fn impl_name(code: &str, after: usize) -> String {
    let mut rest = code[after..].trim_start();
    if rest.starts_with('<') {
        let mut depth = 0;
        for (i, c) in rest.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        rest = &rest[i + 1..];
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let mut end = rest.find('{').unwrap_or(rest.len());
    if let Some(w) = rest.find(" where") {
        end = end.min(w);
    }
    format!("impl {}", collapse(&rest[..end]))
}

fn detect_rust(code: &str, _ctx: Ctx) -> Option<Candidate> {
    if !mentions(
        code,
        &[
            "fn",
            "struct",
            "enum",
            "union",
            "trait",
            "mod",
            "impl",
            "macro_rules",
        ],
    ) {
        return None;
    }
    if let Some(c) = RUST_FN.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    if let Some(c) = RUST_TYPE.captures(code) {
        let kind = match &c[1] {
            "mod" => SymbolKind::Module,
            "trait" => SymbolKind::Trait,
            "enum" => SymbolKind::Enum,
            _ => SymbolKind::Struct,
        };
        return cand(&c, 2, kind, false);
    }
    if let Some(m) = RUST_IMPL.find(code) {
        return Some(Candidate {
            name: impl_name(code, m.end()),
            kind: SymbolKind::Class,
            start: m.start(),
            needs_body: false,
        });
    }
    if let Some(c) = RUST_MACRO.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    None
}

static RUST: BraceLang = BraceLang {
    lexis: Lexis {
        nested_comments: true,
        quotes: b"\"",
        rust_quotes: true,
        ..C_LEXIS
    },
    detect: detect_rust,
    semicolons: true,
    arrow_body: false,
    skip_hash_lines: false,
};

// C and C++ ----------------------------------------------------------------

re!(
    C_TYPE,
    r"^\s*(?:template\s*<.*>\s*)?(?:typedef\s+)?(?:export\s+)?(class|struct|union|enum(?:\s+class|\s+struct)?|namespace)\s+(?:(?:alignas|__declspec|__attribute__)\s*\(\(?[^)]*\)?\)\s*|\[\[[^\]]*\]\]\s*|[A-Z][A-Z0-9_]*_(?:API|EXPORT)\s+)*([A-Za-z_][\w:]*)\s*(?:final\s*)?(?::[^;{]*)?(?:\{.*)?$"
);
re!(
    C_FUNC,
    r"^\s*(?:[\w:<>,*&\[\]~]+[\s*&]+)*?((?:[A-Za-z_]\w*(?:<[^<>()]*>)?::)*(?:~?[A-Za-z_]\w*|operator\s*(?:\(\)|[^\s(]+)))\s*\("
);

fn detect_c(code: &str, ctx: Ctx) -> Option<Candidate> {
    if mentions(code, &["class", "struct", "union", "enum", "namespace"])
        && let Some(c) = C_TYPE.captures(code)
    {
        let kind = match c[1].split_whitespace().next().unwrap_or("") {
            "namespace" => SymbolKind::Module,
            "class" => SymbolKind::Class,
            "enum" => SymbolKind::Enum,
            _ => SymbolKind::Struct,
        };
        return cand(&c, 2, kind, true);
    }
    if ctx.in_code || !code.contains('(') {
        return None;
    }
    let c = C_FUNC.captures(code)?;
    let name = c.get(1)?;
    let last = name.as_str().rsplit("::").next().unwrap_or("");
    if is_control_word(last) || code[..name.start()].contains('=') {
        return None;
    }
    let kind = if name.as_str().contains("::") {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };
    cand(&c, 1, kind, true)
}

static C_FAMILY: BraceLang = BraceLang {
    lexis: C_LEXIS,
    detect: detect_c,
    semicolons: true,
    arrow_body: false,
    skip_hash_lines: true,
};

// C# and Java --------------------------------------------------------------

re!(
    CS_TYPE,
    r"^\s*(?:\[[^\]]*\]\s*)*(?:(?:public|private|protected|internal|static|abstract|sealed|partial|readonly|unsafe|new|file|ref)\s+)*(class|struct|interface|enum|record(?:\s+class|\s+struct)?|namespace)\s+([A-Za-z_][\w.]*)"
);
re!(
    CS_METHOD,
    r"^\s*(?:\[[^\]]*\]\s*)*(?:(?:public|private|protected|internal|static|virtual|override|abstract|sealed|async|extern|unsafe|new|partial|readonly|implicit|explicit)\s+)*(?:[\w.<>\[\],?()]+[?\[\]]*\s+)??([A-Za-z_]\w*)\s*(?:<[^<>()]*>)?\s*\("
);

fn detect_csharp(code: &str, ctx: Ctx) -> Option<Candidate> {
    if mentions(
        code,
        &[
            "class",
            "struct",
            "interface",
            "enum",
            "record",
            "namespace",
        ],
    ) && let Some(c) = CS_TYPE.captures(code)
    {
        let kind = match &c[1] {
            "namespace" => SymbolKind::Module,
            "interface" => SymbolKind::Trait,
            "enum" => SymbolKind::Enum,
            "struct" | "record struct" | "record" => SymbolKind::Struct,
            _ => SymbolKind::Class,
        };
        return cand(&c, 2, kind, false);
    }
    if !ctx.in_type() || !code.contains('(') {
        return None;
    }
    let c = CS_METHOD.captures(code)?;
    if is_control_word(c.get(1)?.as_str()) {
        return None;
    }
    cand(&c, 1, SymbolKind::Method, true)
}

static CSHARP: BraceLang = BraceLang {
    lexis: C_LEXIS,
    detect: detect_csharp,
    semicolons: true,
    arrow_body: true,
    skip_hash_lines: true,
};

re!(
    JAVA_TYPE,
    r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:(?:public|private|protected|static|abstract|final|sealed|non-sealed|strictfp)\s+)*(class|interface|enum|record|@interface)\s+([A-Za-z_]\w*)"
);
re!(
    JAVA_METHOD,
    r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:(?:public|private|protected|static|final|abstract|synchronized|native|default|strictfp|def)\s+)*(?:<[^()]*>\s+)?(?:[\w.<>\[\],?]+\s+)??([A-Za-z_]\w*)\s*\("
);

fn detect_java(code: &str, ctx: Ctx) -> Option<Candidate> {
    if mentions(code, &["class", "interface", "enum", "record"])
        && let Some(c) = JAVA_TYPE.captures(code)
    {
        let kind = match &c[1] {
            "interface" | "@interface" => SymbolKind::Trait,
            "enum" => SymbolKind::Enum,
            "record" => SymbolKind::Struct,
            _ => SymbolKind::Class,
        };
        return cand(&c, 2, kind, false);
    }
    if !ctx.in_type() || !code.contains('(') {
        return None;
    }
    let c = JAVA_METHOD.captures(code)?;
    if is_control_word(c.get(1)?.as_str()) {
        return None;
    }
    cand(&c, 1, SymbolKind::Method, true)
}

static JAVA: BraceLang = BraceLang {
    lexis: C_LEXIS,
    detect: detect_java,
    semicolons: true,
    arrow_body: false,
    skip_hash_lines: false,
};

// Kotlin, Scala, Swift, Dart, Zig ------------------------------------------

re!(
    KT_FUN,
    r"^\s*(?:@[\w.:]+(?:\([^)]*\))?\s+)*(?:(?:public|private|protected|internal|open|final|abstract|override|suspend|inline|tailrec|operator|infix|external|actual|expect)\s+)*fun\s+(?:<[^>]*>\s*)?([^\s(=]+)\s*\("
);
re!(
    KT_TYPE,
    r"^\s*(?:@[\w.:]+(?:\([^)]*\))?\s+)*(?:(?:public|private|protected|internal|open|final|abstract|sealed|data|enum|annotation|inner|value|companion|fun|actual|expect)\s+)*(class|interface|object)\s+([A-Za-z_]\w*)"
);

fn detect_kotlin(code: &str, _ctx: Ctx) -> Option<Candidate> {
    if !mentions(code, &["fun", "class", "interface", "object"]) {
        return None;
    }
    if let Some(c) = KT_FUN.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    let c = KT_TYPE.captures(code)?;
    let kind = if &c[1] == "interface" {
        SymbolKind::Trait
    } else if code[..c.get(1)?.start()].contains("enum") {
        SymbolKind::Enum
    } else {
        SymbolKind::Class
    };
    cand(&c, 2, kind, false)
}

static KOTLIN: BraceLang = BraceLang {
    lexis: Lexis {
        nested_comments: true,
        triple_quotes: true,
        ..C_LEXIS
    },
    detect: detect_kotlin,
    semicolons: false,
    arrow_body: false,
    skip_hash_lines: false,
};

re!(
    SCALA_DEF,
    r"^\s*(?:(?:private|protected|override|final|implicit|inline|transparent|abstract|lazy)(?:\[[^\]]*\])?\s+)*def\s+([^\s(\[:=]+)"
);
re!(
    SCALA_TYPE,
    r"^\s*(?:(?:private|protected|final|sealed|abstract|implicit|case|open|lazy)(?:\[[^\]]*\])?\s+)*(class|trait|object|enum)\s+([A-Za-z_]\w*)"
);

fn detect_scala(code: &str, _ctx: Ctx) -> Option<Candidate> {
    if !mentions(code, &["def", "class", "trait", "object", "enum"]) {
        return None;
    }
    if let Some(c) = SCALA_DEF.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    let c = SCALA_TYPE.captures(code)?;
    let kind = match &c[1] {
        "trait" => SymbolKind::Trait,
        "enum" => SymbolKind::Enum,
        _ => SymbolKind::Class,
    };
    cand(&c, 2, kind, false)
}

static SCALA: BraceLang = BraceLang {
    lexis: Lexis {
        nested_comments: true,
        triple_quotes: true,
        ..C_LEXIS
    },
    detect: detect_scala,
    semicolons: false,
    arrow_body: false,
    skip_hash_lines: false,
};

re!(
    SWIFT_FUNC,
    r"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?:(?:public|private|fileprivate|internal|open|package|static|class|final|override|mutating|nonmutating|dynamic|optional|required|convenience|nonisolated|isolated|prefix|postfix|infix)\s+)*(?:func\s+([^\s(<]+)|(init|deinit|subscript)\b)"
);
re!(
    SWIFT_TYPE,
    r"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?:(?:public|private|fileprivate|internal|open|package|final|indirect)\s+)*(class|struct|enum|protocol|extension|actor)\s+([A-Za-z_][\w.]*)"
);

fn detect_swift(code: &str, _ctx: Ctx) -> Option<Candidate> {
    if !mentions(
        code,
        &[
            "func",
            "init",
            "subscript",
            "class",
            "struct",
            "enum",
            "protocol",
            "extension",
            "actor",
        ],
    ) {
        return None;
    }
    if let Some(c) = SWIFT_FUNC.captures(code) {
        let group = if c.get(1).is_some() { 1 } else { 2 };
        return cand(&c, group, SymbolKind::Function, false);
    }
    let c = SWIFT_TYPE.captures(code)?;
    let kind = match &c[1] {
        "struct" => SymbolKind::Struct,
        "enum" => SymbolKind::Enum,
        "protocol" => SymbolKind::Trait,
        _ => SymbolKind::Class,
    };
    let mut candidate = cand(&c, 2, kind, false)?;
    if &c[1] == "extension" {
        candidate.name = format!("extension {}", candidate.name);
    }
    Some(candidate)
}

static SWIFT: BraceLang = BraceLang {
    lexis: Lexis {
        nested_comments: true,
        quotes: b"\"",
        triple_quotes: true,
        ..C_LEXIS
    },
    detect: detect_swift,
    semicolons: false,
    arrow_body: false,
    skip_hash_lines: false,
};

re!(
    DART_TYPE,
    r"^\s*(?:(?:abstract|base|interface|final|sealed|mixin)\s+)*(class|mixin|enum|extension)\s+([A-Za-z_]\w*)"
);
re!(
    DART_FUNC,
    r"^\s*(?:(?:static|external|factory|abstract|const|late|final|covariant)\s+)*(?:[\w.<>?,\[\] ]+?\s+)??(?:(get|set|operator)\s+)?([A-Za-z_$][\w$.]*)\s*(?:<[^<>()]*>)?\s*(\(|\{|=>)"
);

fn detect_dart(code: &str, ctx: Ctx) -> Option<Candidate> {
    if mentions(code, &["class", "mixin", "enum", "extension"])
        && let Some(c) = DART_TYPE.captures(code)
    {
        let kind = if &c[1] == "enum" {
            SymbolKind::Enum
        } else {
            SymbolKind::Class
        };
        return cand(&c, 2, kind, false);
    }
    if ctx.in_code || !mentions(code, &["(", "{", "=>"]) {
        return None;
    }
    let c = DART_FUNC.captures(code)?;
    let name = c.get(2)?;
    if is_control_word(name.as_str()) || code[..name.start()].contains('=') {
        return None;
    }
    // Without parentheses only a getter (`int get size => …`) is a declaration.
    if &c[3] != "(" && c.get(1).is_none_or(|m| m.as_str() != "get") {
        return None;
    }
    cand(&c, 2, SymbolKind::Function, true)
}

static DART: BraceLang = BraceLang {
    lexis: Lexis {
        nested_comments: true,
        triple_quotes: true,
        ..C_LEXIS
    },
    detect: detect_dart,
    semicolons: true,
    arrow_body: true,
    skip_hash_lines: false,
};

re!(ZIG_FN, r"\bfn\s+([A-Za-z_]\w*)\s*\(");
re!(
    ZIG_TYPE,
    r"^\s*(?:pub\s+)?const\s+([A-Za-z_]\w*)\s*(?::[^=]*)?=\s*(?:(?:extern|packed)\s+)?(struct|enum|union|opaque)\b"
);

fn detect_zig(code: &str, _ctx: Ctx) -> Option<Candidate> {
    if !mentions(code, &["fn", "const"]) {
        return None;
    }
    if let Some(c) = ZIG_FN.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    let c = ZIG_TYPE.captures(code)?;
    let kind = if &c[2] == "enum" {
        SymbolKind::Enum
    } else {
        SymbolKind::Struct
    };
    cand(&c, 1, kind, false)
}

static ZIG: BraceLang = BraceLang {
    lexis: C_LEXIS,
    detect: detect_zig,
    semicolons: true,
    arrow_body: false,
    skip_hash_lines: false,
};

// Go -----------------------------------------------------------------------

re!(
    GO_FUNC,
    r"^func\s+(?:\(\s*(?:[A-Za-z_]\w*\s+)?\*?\s*([A-Za-z_]\w*)(?:\[[^\]]*\])?\s*\)\s*)?([A-Za-z_]\w*)"
);
re!(
    GO_TYPE,
    r"^(?:type\s+|\s+)([A-Za-z_]\w*)(?:\[[^\]]*\])?\s+(struct|interface)\b"
);

fn detect_go(code: &str, ctx: Ctx) -> Option<Candidate> {
    if !mentions(code, &["func", "struct", "interface"]) {
        return None;
    }
    if let Some(c) = GO_FUNC.captures(code) {
        let (name, kind) = match c.get(1) {
            Some(recv) => (format!("{}.{}", recv.as_str(), &c[2]), SymbolKind::Method),
            None => (c[2].to_owned(), SymbolKind::Function),
        };
        return Some(Candidate {
            name,
            kind,
            start: 0,
            needs_body: false,
        });
    }
    // Indented types only count inside a `type ( … )` group, which has no braces.
    if code.starts_with(char::is_whitespace) && (ctx.parent.is_some() || ctx.in_code) {
        return None;
    }
    let c = GO_TYPE.captures(code)?;
    let kind = if &c[2] == "interface" {
        SymbolKind::Trait
    } else {
        SymbolKind::Struct
    };
    cand(&c, 1, kind, false)
}

static GO: BraceLang = BraceLang {
    lexis: Lexis {
        multiline_quotes: b"`",
        ..C_LEXIS
    },
    detect: detect_go,
    semicolons: false,
    arrow_body: false,
    skip_hash_lines: false,
};

// JavaScript and TypeScript ------------------------------------------------

re!(
    JS_FUNCTION,
    r"^\s*(?:export\s+)?(?:default\s+)?(?:declare\s+)?(?:async\s+)?function\s*\*?\s*([A-Za-z_$][\w$]*)"
);
re!(
    JS_CLASS,
    r"^\s*(?:export\s+)?(?:default\s+)?(?:declare\s+)?(?:abstract\s+)?(class|interface|enum|const\s+enum|namespace|module|type)\s+([A-Za-z_$][\w$.]*)"
);
re!(
    JS_ARROW,
    r"^\s*(?:export\s+)?(?:declare\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::[^=]+)?=\s*(?:async\s+)?(?:function\b|(?:<[^>]*>\s*)?(?:\([^)]*\)|[A-Za-z_$][\w$]*)\s*(?::[^=]+)?=>|\([^)]*$)"
);
re!(
    JS_METHOD,
    r"^\s*(?:(?:public|private|protected|static|async|readonly|override|abstract|declare|get|set|accessor)\s+)*\*?\s*(#?[A-Za-z_$][\w$]*)\s*\??\s*(?:<[^<>()]*>)?\s*\("
);
re!(
    JS_FIELD_ARROW,
    r"^\s*(?:(?:public|private|protected|static|readonly|override)\s+)*(#?[A-Za-z_$][\w$]*)\s*(?::[^=]+)?=\s*(?:async\s+)?(?:\([^)]*\)|[A-Za-z_$][\w$]*)\s*(?::[^=]+)?=>"
);

fn detect_js(code: &str, ctx: Ctx) -> Option<Candidate> {
    let keywords = [
        "function",
        "class",
        "interface",
        "enum",
        "namespace",
        "module",
        "type",
        "const",
        "let",
        "var",
    ];
    if !mentions(code, &keywords) && !(ctx.in_type() && mentions(code, &["(", "=>"])) {
        return None;
    }
    if let Some(c) = JS_FUNCTION.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    if let Some(c) = JS_CLASS.captures(code) {
        let kind = match c[1].split_whitespace().last().unwrap_or("") {
            "interface" | "type" => SymbolKind::Trait,
            "enum" => SymbolKind::Enum,
            "namespace" | "module" => SymbolKind::Module,
            _ => SymbolKind::Class,
        };
        // `type X = …` is an alias; `type` alone is often just a property name.
        if &c[1] == "type" && !code[c.get(2)?.end()..].trim_start().starts_with(['=', '<']) {
            return None;
        }
        return cand(&c, 2, kind, false);
    }
    if let Some(c) = JS_ARROW.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    if ctx.in_type() {
        if let Some(c) = JS_FIELD_ARROW.captures(code) {
            return cand(&c, 1, SymbolKind::Method, false);
        }
        if let Some(c) = JS_METHOD.captures(code)
            && !is_control_word(c.get(1)?.as_str())
        {
            return cand(&c, 1, SymbolKind::Method, true);
        }
    }
    None
}

static JS: BraceLang = BraceLang {
    lexis: Lexis {
        multiline_quotes: b"`",
        ..C_LEXIS
    },
    detect: detect_js,
    semicolons: true,
    arrow_body: false,
    skip_hash_lines: false,
};

// PHP ----------------------------------------------------------------------

re!(
    PHP_FUNCTION,
    r"(?i)^\s*(?:(?:public|private|protected|static|abstract|final|readonly)\s+)*function\s+&?\s*([A-Za-z_]\w*)"
);
re!(
    PHP_TYPE,
    r"(?i)^\s*(?:(?:abstract|final|readonly)\s+)*(class|interface|trait|enum|namespace)\s+([A-Za-z_\\][\w\\]*)"
);

fn detect_php(code: &str, _ctx: Ctx) -> Option<Candidate> {
    if let Some(c) = PHP_FUNCTION.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    let c = PHP_TYPE.captures(code)?;
    let kind = match c[1].to_ascii_lowercase().as_str() {
        "interface" | "trait" => SymbolKind::Trait,
        "enum" => SymbolKind::Enum,
        "namespace" => SymbolKind::Module,
        _ => SymbolKind::Class,
    };
    cand(&c, 2, kind, false)
}

static PHP: BraceLang = BraceLang {
    lexis: Lexis {
        line_comments: &["//", "#"],
        ..C_LEXIS
    },
    detect: detect_php,
    semicolons: true,
    arrow_body: false,
    skip_hash_lines: false,
};

// PowerShell and shell -----------------------------------------------------

re!(
    PS_FUNCTION,
    r"(?i)^\s*(?:function|filter|workflow|configuration)\s+([\w-]+(?::[\w-]+)?)"
);
re!(PS_TYPE, r"(?i)^\s*(class|enum)\s+([A-Za-z_]\w*)");
re!(
    PS_METHOD,
    r"^\s*(?:(?i:static|hidden)\s+)*(?:\[[^\]]+\]\s*)*([A-Za-z_]\w*)\s*\("
);

fn detect_powershell(code: &str, ctx: Ctx) -> Option<Candidate> {
    if let Some(c) = PS_FUNCTION.captures(code) {
        return cand(&c, 1, SymbolKind::Function, false);
    }
    if let Some(c) = PS_TYPE.captures(code) {
        let kind = if c[1].eq_ignore_ascii_case("enum") {
            SymbolKind::Enum
        } else {
            SymbolKind::Class
        };
        return cand(&c, 2, kind, false);
    }
    if ctx.in_type() {
        let c = PS_METHOD.captures(code)?;
        if is_control_word(&c[1].to_ascii_lowercase()) {
            return None;
        }
        return cand(&c, 1, SymbolKind::Method, true);
    }
    None
}

static POWERSHELL: BraceLang = BraceLang {
    lexis: Lexis {
        line_comments: &["#"],
        block_comment: Some(("<#", "#>")),
        quotes: b"\"'",
        escape: Some(b'`'),
        ..lexis()
    },
    detect: detect_powershell,
    semicolons: false,
    arrow_body: false,
    skip_hash_lines: false,
};

re!(
    SH_FUNCTION,
    r"^\s*(?:function\s+([\w.:@+-]+)(?:\s*\(\s*\))?|([\w.:@+-]+)\s*\(\s*\))"
);

fn detect_shell(code: &str, _ctx: Ctx) -> Option<Candidate> {
    if !mentions(code, &["function", "("]) {
        return None;
    }
    let c = SH_FUNCTION.captures(code)?;
    let group = if c.get(1).is_some() { 1 } else { 2 };
    cand(&c, group, SymbolKind::Function, false)
}

static SHELL: BraceLang = BraceLang {
    lexis: Lexis {
        line_comments: &["#"],
        quotes: b"\"'",
        hash_at_word_start: true,
        ..lexis()
    },
    detect: detect_shell,
    semicolons: false,
    arrow_body: false,
    skip_hash_lines: false,
};

// ---------------------------------------------------------------------------
// Indentation languages: Python, Ruby, Lua

struct IndentLang {
    lexis: Lexis,
    detect: fn(&str) -> Option<(String, SymbolKind)>,
}

fn indent_width(line: &str) -> usize {
    let mut width = 0;
    for c in line.chars() {
        match c {
            ' ' => width += 1,
            '\t' => width = (width / 8 + 1) * 8,
            _ => break,
        }
    }
    width
}

fn indented(text: &str, lang: &IndentLang) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut scanner = Scanner::new(lang.lexis);
    let mut stack: Vec<(usize, SymbolKind)> = Vec::new();
    for (line_no, line) in lines(text).enumerate() {
        // A line that starts inside a string or comment says nothing about
        // the indentation.
        let continued = scanner.state != LexState::Code;
        let code = scanner.clean(line);
        if continued || code.trim().is_empty() {
            continue;
        }
        let indent = indent_width(code);
        while stack.last().is_some_and(|&(i, _)| i >= indent) {
            stack.pop();
        }
        if let Some((name, mut kind)) = (lang.detect)(code) {
            if kind == SymbolKind::Function
                && matches!(
                    stack.last(),
                    Some((_, SymbolKind::Class | SymbolKind::Module))
                )
            {
                kind = SymbolKind::Method;
            }
            out.push(Symbol {
                name,
                kind,
                line: line_no,
                depth: stack.len(),
            });
            stack.push((indent, kind));
        }
    }
    out
}

re!(PY_DEF, r"^\s*(?:async\s+)?def\s+([A-Za-z_]\w*)");
re!(PY_CLASS, r"^\s*class\s+([A-Za-z_]\w*)");

fn detect_python(code: &str) -> Option<(String, SymbolKind)> {
    if !mentions(code, &["def", "class"]) {
        return None;
    }
    if let Some(c) = PY_DEF.captures(code) {
        return Some((c[1].to_owned(), SymbolKind::Function));
    }
    PY_CLASS
        .captures(code)
        .map(|c| (c[1].to_owned(), SymbolKind::Class))
}

static PYTHON: IndentLang = IndentLang {
    lexis: Lexis {
        line_comments: &["#"],
        quotes: b"\"'",
        triple_quotes: true,
        ..lexis()
    },
    detect: detect_python,
};

re!(RB_DEF, r"^\s*def\s+((?:self\.)?[A-Za-z_][\w.]*[?!=]?)");
re!(RB_TYPE, r"^\s*(class|module)\s+([A-Z][\w:]*)");

fn detect_ruby(code: &str) -> Option<(String, SymbolKind)> {
    if !mentions(code, &["def", "class", "module"]) {
        return None;
    }
    if let Some(c) = RB_DEF.captures(code) {
        return Some((c[1].to_owned(), SymbolKind::Function));
    }
    RB_TYPE.captures(code).map(|c| {
        let kind = if &c[1] == "module" {
            SymbolKind::Module
        } else {
            SymbolKind::Class
        };
        (c[2].to_owned(), kind)
    })
}

static RUBY: IndentLang = IndentLang {
    lexis: Lexis {
        line_comments: &["#"],
        block_comment: Some(("=begin", "=end")),
        block_at_line_start: true,
        quotes: b"\"'",
        ..lexis()
    },
    detect: detect_ruby,
};

re!(
    LUA_FUNCTION,
    r"^\s*(?:local\s+)?function\s+([A-Za-z_][\w.:]*)"
);
re!(
    LUA_ASSIGNED,
    r"^\s*(?:local\s+)?([A-Za-z_][\w.:\[\]]*)\s*=\s*function\b"
);

fn detect_lua(code: &str) -> Option<(String, SymbolKind)> {
    if !code.contains("function") {
        return None;
    }
    LUA_FUNCTION
        .captures(code)
        .or_else(|| LUA_ASSIGNED.captures(code))
        .map(|c| (c[1].to_owned(), SymbolKind::Function))
}

static LUA: IndentLang = IndentLang {
    lexis: Lexis {
        line_comments: &["--"],
        block_comment: Some(("--[[", "]]")),
        quotes: b"\"'",
        ..lexis()
    },
    detect: detect_lua,
};

// ---------------------------------------------------------------------------
// Other formats

re!(BATCH_LABEL, r"^\s*:([A-Za-z_][\w.-]*)");

fn batch(text: &str) -> Vec<Symbol> {
    lines(text)
        .enumerate()
        .filter_map(|(line, l)| {
            let c = BATCH_LABEL.captures(l)?;
            Some(Symbol {
                name: c[1].to_owned(),
                kind: SymbolKind::Function,
                line,
                depth: 0,
            })
        })
        .collect()
}

re!(
    SQL_CREATE,
    r#"(?i)\bcreate\s+(?:or\s+(?:replace|alter)\s+)?(?:(?:global|local|temporary|temp|unlogged|virtual|unique|clustered|nonclustered|materialized|recursive|secure|external|transient)\s+|definer\s*=\s*\S+\s+)*(table|view|function|procedure|proc|trigger|index|type|schema|database|sequence|package\s+body|package|domain|extension|event|role|user|macro)\s+(?:if\s+not\s+exists\s+)?((?:[\w$@#]+|"[^"]+"|`[^`]+`|\[[^\]]+\])(?:\s*\.\s*(?:[\w$@#]+|"[^"]+"|`[^`]+`|\[[^\]]+\]))*)"#
);

fn sql(text: &str) -> Vec<Symbol> {
    let mut scanner = Scanner::new(Lexis {
        line_comments: &["--", "#"],
        block_comment: Some(("/*", "*/")),
        quotes: b"'",
        escape: None,
        hash_at_word_start: true,
        ..lexis()
    });
    let mut out = Vec::new();
    for (line_no, line) in lines(text).enumerate() {
        let code = scanner.clean(line);
        if !code
            .as_bytes()
            .windows(6)
            .any(|w| w.eq_ignore_ascii_case(b"create"))
        {
            continue;
        }
        for c in SQL_CREATE.captures_iter(code) {
            let what = c[1].to_ascii_lowercase();
            let kind = match what.split_whitespace().next().unwrap_or("") {
                "table" | "view" | "type" | "domain" => SymbolKind::Struct,
                "function" | "procedure" | "proc" | "trigger" | "event" | "macro" => {
                    SymbolKind::Function
                }
                "schema" | "database" | "package" | "extension" => SymbolKind::Module,
                _ => SymbolKind::Key,
            };
            out.push(Symbol {
                name: collapse(&c[2]),
                kind,
                line: line_no,
                depth: 0,
            });
        }
    }
    out
}

re!(
    MD_ATX,
    r"^ {0,3}(#{1,6})(?:[ \t]+(.*?))?(?:[ \t]+#+)?[ \t]*$"
);
re!(MD_FENCE, r"^ {0,3}(`{3,}|~{3,})");

fn markdown(text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut fence: Option<String> = None;
    let mut front_matter = false;
    let mut prev: Option<(usize, &str)> = None;
    for (line_no, line) in lines(text).enumerate() {
        if line_no == 0 && line.trim_end() == "---" {
            front_matter = true;
            continue;
        }
        if front_matter {
            if matches!(line.trim_end(), "---" | "...") {
                front_matter = false;
            }
            continue;
        }
        let t = line.trim();
        if t.starts_with(['`', '~'])
            && let Some(c) = MD_FENCE.captures(line)
        {
            let marker = &c[1];
            match &fence {
                None => fence = Some(marker.to_owned()),
                Some(open)
                    if marker.as_bytes()[0] == open.as_bytes()[0]
                        && marker.len() >= open.len()
                        && line.trim().len() == marker.len() =>
                {
                    fence = None
                }
                _ => {}
            }
            prev = None;
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if t.starts_with('#')
            && let Some(c) = MD_ATX.captures(line)
        {
            out.push(Symbol {
                name: c.get(2).map_or(String::new(), |m| collapse(m.as_str())),
                kind: SymbolKind::Heading,
                line: line_no,
                depth: c[1].len() - 1,
            });
            prev = None;
            continue;
        }
        // Setext headings: a paragraph line underlined with === or ---.
        let underline = !t.is_empty()
            && line.len() - line.trim_start().len() < 4
            && (t.bytes().all(|b| b == b'=') || t.bytes().all(|b| b == b'-'));
        if underline && let Some((pline, ptext)) = prev {
            out.push(Symbol {
                name: collapse(ptext),
                kind: SymbolKind::Heading,
                line: pline,
                depth: if t.starts_with('=') { 0 } else { 1 },
            });
            prev = None;
            continue;
        }
        let not_paragraph = t.is_empty()
            || line.starts_with("    ")
            || line.starts_with('\t')
            || t.starts_with(['>', '-', '*', '+', '|', '<'])
            || t.as_bytes()[0].is_ascii_digit();
        prev = (!not_paragraph).then_some((line_no, t));
    }
    out
}

re!(INI_SECTION, r"^\s*\[\[?\s*([^\]]*?)\s*\]\]?\s*(?:[;#].*)?$");

fn ini_sections(text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut in_multiline: Option<&str> = None;
    for (line_no, line) in lines(text).enumerate() {
        // Skip TOML multi-line strings, which may hold `[x]` lines.
        if let Some(q) = in_multiline {
            if line.contains(q) {
                in_multiline = None;
            }
            continue;
        }
        for q in ["\"\"\"", "'''"] {
            if line.matches(q).count() % 2 == 1 {
                in_multiline = Some(q);
            }
        }
        if in_multiline.is_some() {
            continue;
        }
        if let Some(c) = INI_SECTION.captures(line) {
            out.push(Symbol {
                name: c[1].to_owned(),
                kind: SymbolKind::Section,
                line: line_no,
                depth: 0,
            });
        }
    }
    out
}

re!(
    YAML_KEY,
    r#"^("(?:[^"\\]|\\.)*"|'(?:[^']|'')*'|[^\s#'"\-?:,\[\]{}&*!|>%@`][^#]*?)\s*:(?:\s|$)"#
);

fn yaml(text: &str) -> Vec<Symbol> {
    lines(text)
        .enumerate()
        .filter_map(|(line, l)| {
            let c = YAML_KEY.captures(l)?;
            let key = c[1].trim();
            let key = key
                .strip_prefix('"')
                .and_then(|k| k.strip_suffix('"'))
                .or_else(|| key.strip_prefix('\'').and_then(|k| k.strip_suffix('\'')))
                .unwrap_or(key);
            Some(Symbol {
                name: key.to_owned(),
                kind: SymbolKind::Key,
                line,
                depth: 0,
            })
        })
        .collect()
}

/// Keys of the top-level object.
fn json(text: &str) -> Vec<Symbol> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut root_is_object = false;
    let mut line = 0usize;
    // The last string seen at depth 1 (start, end, line), waiting for a `:`.
    let mut last_string: Option<(usize, usize, usize)> = None;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\n' => line += 1,
            b'\r' if b.get(i + 1) != Some(&b'\n') => line += 1,
            b'"' => {
                let start = i + 1;
                let start_line = line;
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    } else if b[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
                if depth == 1 && root_is_object {
                    last_string = Some((start, i.min(b.len()), start_line));
                }
            }
            b'{' | b'[' => {
                if depth == 0 && b[i] == b'{' {
                    root_is_object = true;
                }
                depth += 1;
                last_string = None;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                last_string = None;
            }
            b':' => {
                if let Some((s, e, l)) = last_string.take() {
                    out.push(Symbol {
                        name: String::from_utf8_lossy(&b[s..e]).into_owned(),
                        kind: SymbolKind::Key,
                        line: l,
                        depth: 0,
                    });
                }
            }
            b',' => last_string = None,
            _ => {}
        }
        i += 1;
    }
    out
}

re!(
    XML_ID,
    r#"<([A-Za-z][\w:.-]*)\b[^>]*?\s(?:id|xml:id)\s*=\s*(?:"([^"]*)"|'([^']*)')"#
);

fn xml_ids(text: &str) -> Vec<Symbol> {
    let mut scanner = Scanner::new(Lexis {
        block_comment: Some(("<!--", "-->")),
        escape: None,
        ..lexis()
    });
    let mut out = Vec::new();
    for (line_no, line) in lines(text).enumerate() {
        let code = scanner.clean(line);
        if !code.contains("id") {
            continue;
        }
        for c in XML_ID.captures_iter(code) {
            let id = c.get(2).or(c.get(3)).map_or("", |m| m.as_str());
            out.push(Symbol {
                name: format!("{}#{id}", &c[1]),
                kind: SymbolKind::Key,
                line: line_no,
                depth: 0,
            });
        }
    }
    out
}

/// Segments; headers (MSH, FHS, BHS) at the top level, the rest under them.
fn hl7(text: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    for (line_no, line) in lines(text).enumerate() {
        let b = line.as_bytes();
        let is_segment = b.len() >= 3
            && b[0].is_ascii_uppercase()
            && b[1..3]
                .iter()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            && b.get(3).is_none_or(|c| !c.is_ascii_alphanumeric());
        if !is_segment {
            continue;
        }
        let id = &line[..3];
        let header = matches!(id, "MSH" | "FHS" | "BHS");
        let mut name = id.to_owned();
        if id == "MSH" && b.len() > 4 {
            // MSH-9, the message type. MSH-1 is the separator itself, so it
            // is the ninth piece when splitting on it.
            let (field, component) = (b[3] as char, b[4] as char);
            if let Some(t) = line.split(field).nth(8).filter(|t| !t.is_empty()) {
                name = format!("MSH {}", t.replace(component, "^"));
            }
        }
        out.push(Symbol {
            name,
            kind: SymbolKind::Section,
            line: line_no,
            depth: usize::from(!(header || matches!(id, "BTS" | "FTS"))),
        });
    }
    out
}

/// Rule selectors and at-rules, nested by braces.
fn css(text: &str) -> Vec<Symbol> {
    let mut scanner = Scanner::new(Lexis {
        block_comment: Some(("/*", "*/")),
        quotes: b"\"'",
        ..lexis()
    });
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut selector = String::new();
    let mut selector_line = 0usize;
    let mut started = false;
    for (line_no, line) in lines(text).enumerate() {
        let code = scanner.clean(line);
        let mut start = 0;
        for (i, c) in code.char_indices() {
            match c {
                '{' => {
                    selector.push_str(&code[start..i]);
                    let name = collapse(&selector);
                    if !name.is_empty() {
                        out.push(Symbol {
                            kind: if name.starts_with('@') {
                                SymbolKind::Section
                            } else {
                                SymbolKind::Key
                            },
                            name,
                            line: selector_line,
                            depth,
                        });
                    }
                    depth += 1;
                    selector.clear();
                    started = false;
                    start = i + 1;
                }
                '}' | ';' => {
                    if c == '}' {
                        depth = depth.saturating_sub(1);
                    }
                    selector.clear();
                    started = false;
                    start = i + 1;
                }
                _ if !started && !c.is_whitespace() => {
                    selector_line = line_no;
                    started = true;
                }
                _ => {}
            }
        }
        selector.push_str(&code[start..]);
        selector.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::SymbolKind::*;
    use super::*;

    type Row = (String, SymbolKind, usize, usize);

    fn names(language: &str, text: &str) -> Vec<Row> {
        symbols(language, text)
            .into_iter()
            .map(|s| (s.name, s.kind, s.line, s.depth))
            .collect()
    }

    fn s(name: &str, kind: SymbolKind, line: usize, depth: usize) -> Row {
        (name.to_owned(), kind, line, depth)
    }

    #[test]
    fn symbols_are_send() {
        fn send<T: Send + Clone>() {}
        send::<Symbol>();
    }

    #[test]
    fn line_numbers_follow_the_editor() {
        let all: Vec<&str> = lines("a\nb\r\nc\rd\u{2028}e").collect();
        assert_eq!(all, ["a", "b", "c", "d", "e"]);
        assert_eq!(lines("").count(), 1);
        assert_eq!(lines("x\n").count(), 2);
    }

    #[test]
    fn rust() {
        let src = r##"
// fn commented_out() {}
pub struct Point<T> { x: T }
/* fn in_block() {}
   /* nested */ fn still_comment() {} */
impl<T: Into<String>> fmt::Display for Point<T>
where
    T: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = "fn not_a_fn() {";
        let c = '{';
        write!(f, "{}", s)
    }
}
pub(crate) async unsafe fn run<'a>(x: &'a str) {}
pub trait Shape { fn area(&self) -> f64; }
mod inner {
    pub enum Kind { A }
}
macro_rules! make { () => {}; }
fn last() { let r = r#"fn raw() {"#; }
"##;
        assert_eq!(
            names("Rust", src),
            vec![
                s("Point", Struct, 2, 0),
                s("impl fmt::Display for Point<T>", Class, 5, 0),
                s("fmt", Method, 9, 1),
                s("run", Function, 15, 0),
                s("Shape", Trait, 16, 0),
                s("area", Method, 16, 1),
                s("inner", Module, 17, 0),
                s("Kind", Enum, 18, 1),
                s("make", Function, 20, 0),
                s("last", Function, 21, 0),
            ]
        );
    }

    #[test]
    fn c_and_cpp() {
        let src = r#"#include <stdio.h>
#define MAX(a, b) ((a) > (b) ? (a) : (b))
static int helper(int x);
struct node { int v; };
static int
compute(int a,
        int b)
{
    if (a) { return helper(b); }
    return 0;
}
namespace app {
class Widget : public Base {
public:
    Widget(int x) : x_(x) {}
    void draw() const override;
    int size() const { return n; }
};
}
void app::Widget::draw() const {
    printf("void fake() {");
}
extern "C" {
int c_api(void) { return 1; }
}
"#;
        assert_eq!(
            names("C++", src),
            vec![
                s("node", Struct, 3, 0),
                s("compute", Function, 5, 0),
                s("app", Module, 11, 0),
                s("Widget", Class, 12, 1),
                s("Widget", Method, 14, 2),
                s("size", Method, 16, 2),
                s("app::Widget::draw", Method, 19, 0),
                s("c_api", Function, 23, 0),
            ]
        );
    }

    #[test]
    fn csharp_and_java() {
        let cs = r#"namespace Demo.App
{
    [Serializable]
    public sealed class Person : IPerson
    {
        public string Name { get; set; }
        public Person(string name) { Name = name; }
        public override string ToString() => Name;
        private static async Task<int> LoadAsync(
            int id)
        {
            if (id > 0) { return await Get(id); }
            return 0;
        }
    }
    public interface IPerson { string Name { get; } }
    public record Point(int X, int Y);
}
"#;
        assert_eq!(
            names("C#", cs),
            vec![
                s("Demo.App", Module, 0, 0),
                s("Person", Class, 3, 1),
                s("Person", Method, 6, 2),
                s("ToString", Method, 7, 2),
                s("LoadAsync", Method, 8, 2),
                s("IPerson", Trait, 15, 1),
                s("Point", Struct, 16, 1),
            ]
        );
        let java = r#"package demo;
@Entity
public class Order implements Serializable {
    private final List<Item> items = new ArrayList<>();
    @Override
    public String toString() {
        return "class Fake {";
    }
    static <T> List<T> copy(List<T> in) throws IOException {
        for (T t : in) { }
        return in;
    }
    enum Status { OPEN, CLOSED }
    abstract void hook();
}
interface Repo { void save(Order o); }
"#;
        assert_eq!(
            names("Java", java),
            vec![
                s("Order", Class, 2, 0),
                s("toString", Method, 5, 1),
                s("copy", Method, 8, 1),
                s("Status", Enum, 12, 1),
                s("Repo", Trait, 15, 0),
            ]
        );
    }

    #[test]
    fn kotlin_swift_dart_zig_scala() {
        let kt = "package a\ndata class User(val name: String) {\n    fun greet(): String = \"hi\"\n    companion object Factory {\n        fun create() = User(\"x\")\n    }\n}\nfun <T> List<T>.second(): T = this[1]\nsealed interface Shape\nenum class Color { RED }\nfun main(\n    args: Array<String>\n) {\n    println(\"fun fake()\")\n}\n";
        assert_eq!(
            names("Kotlin", kt),
            vec![
                s("User", Class, 1, 0),
                s("greet", Method, 2, 1),
                s("Factory", Class, 3, 1),
                s("create", Method, 4, 2),
                s("List<T>.second", Function, 7, 0),
                s("Shape", Trait, 8, 0),
                s("Color", Enum, 9, 0),
                s("main", Function, 10, 0),
            ]
        );
        let swift = "import Foundation\n@MainActor final class View {\n    init(frame: Int) {}\n    func draw(_ r: Rect) {\n        let s = \"func fake() {\"\n    }\n}\nprotocol Drawable {}\nextension View: Drawable {\n    static func make() -> View { View(frame: 0) }\n}\n";
        assert_eq!(
            names("Swift", swift),
            vec![
                s("View", Class, 1, 0),
                s("init", Method, 2, 1),
                s("draw", Method, 3, 1),
                s("Drawable", Trait, 7, 0),
                s("extension View", Class, 8, 0),
                s("make", Method, 9, 1),
            ]
        );
        let dart = "import 'a.dart';\nclass Counter extends ChangeNotifier {\n  int _n = 0;\n  int get value => _n;\n  void increment() {\n    if (_n > 0) { print('void fake() {'); }\n  }\n  @override\n  Widget build(BuildContext context) => Text('$_n');\n}\nFuture<void> main() async {\n  runApp(App());\n}\nenum Mode { a, b }\n";
        assert_eq!(
            names("Dart", dart),
            vec![
                s("Counter", Class, 1, 0),
                s("value", Method, 3, 1),
                s("increment", Method, 4, 1),
                s("build", Method, 8, 1),
                s("main", Function, 10, 0),
                s("Mode", Enum, 13, 0),
            ]
        );
        let zig = "const std = @import(\"std\");\npub const Point = struct {\n    x: i32,\n    pub fn init(x: i32) Point { return .{ .x = x }; }\n};\npub fn main() !void {}\n";
        assert_eq!(
            names("Zig", zig),
            vec![
                s("Point", Struct, 1, 0),
                s("init", Method, 3, 1),
                s("main", Function, 5, 0),
            ]
        );
        let scala = "object Main {\n  def main(args: Array[String]): Unit = {\n    println(\"def x\")\n  }\n}\ncase class P(x: Int)\ntrait T { def f: Int }\n";
        assert_eq!(
            names("Scala", scala),
            vec![
                s("Main", Class, 0, 0),
                s("main", Method, 1, 1),
                s("P", Class, 5, 0),
                s("T", Trait, 6, 0),
                s("f", Method, 6, 1),
            ]
        );
    }

    #[test]
    fn go() {
        let src = "package main\n\nimport \"fmt\"\n\ntype Server struct {\n\tAddr string\n\tOpts struct {\n\t\tDebug bool\n\t}\n}\n\ntype (\n\tReader interface {\n\t\tRead() error\n\t}\n)\n\nfunc (s *Server) Start() error {\n\tmsg := `func fake() {\n`\n\treturn nil\n}\n\nfunc main() {\n\tfmt.Println(\"x\")\n}\n";
        assert_eq!(
            names("Go", src),
            vec![
                s("Server", Struct, 4, 0),
                s("Reader", Trait, 12, 0),
                s("Server.Start", Method, 17, 0),
                s("main", Function, 23, 0),
            ]
        );
    }

    #[test]
    fn javascript_and_typescript() {
        let src = r#"import x from "y";
// function commented() {}
export default async function load(url) {
  const inner = (a) => a + 1;
  if (url) { return fetch(url); }
}
export const add = (a: number, b: number): number => a + b;
const handler = async function () {};
let notFn = compute(1);
export abstract class Store<T> extends Base {
  #items: T[] = [];
  private count = 0;
  constructor(private readonly name: string) { super(); }
  static create(): Store<any> {
    return null;
  }
  get size() { return this.#items.length; }
  onClick = (e: Event) => {
    console.log(`function fake() {`);
  };
  abstract save(item: T): void;
}
export interface Item { id: number; }
export type Id = string | number;
enum Color { Red }
const multi = (
  a,
  b,
) => a;
"#;
        assert_eq!(
            names("TypeScript", src),
            vec![
                s("load", Function, 2, 0),
                s("inner", Function, 3, 1),
                s("add", Function, 6, 0),
                s("handler", Function, 7, 0),
                s("Store", Class, 9, 0),
                s("constructor", Method, 12, 1),
                s("create", Method, 13, 1),
                s("size", Method, 16, 1),
                s("onClick", Method, 17, 1),
                s("Item", Trait, 22, 0),
                s("Id", Trait, 23, 0),
                s("Color", Enum, 24, 0),
                s("multi", Function, 25, 0),
            ]
        );
    }

    #[test]
    fn php_powershell_shell_batch() {
        let php = "<?php\nnamespace App\\Models;\nfinal class User extends Model {\n    public static function find(int $id): ?self {\n        return null; // function fake() {}\n    }\n}\nfunction helper() {}\ninterface Jsonable {}\n";
        assert_eq!(
            names("PHP", php),
            vec![
                s("App\\Models", Module, 1, 0),
                s("User", Class, 2, 0),
                s("find", Method, 3, 1),
                s("helper", Function, 7, 0),
                s("Jsonable", Trait, 8, 0),
            ]
        );
        let ps = "<#\nfunction Commented {}\n#>\nfunction Get-Thing {\n    param([string]$Name)\n    \"function fake {\"\n}\nclass Car {\n    [string] Drive([int]$speed) {\n        if ($speed) { return 'x' }\n    }\n}\nenum Color { Red }\n";
        assert_eq!(
            names("PowerShell", ps),
            vec![
                s("Get-Thing", Function, 3, 0),
                s("Car", Class, 7, 0),
                s("Drive", Method, 8, 1),
                s("Color", Enum, 12, 0),
            ]
        );
        let sh = "#!/bin/bash\n# build() { }\nbuild() {\n  echo \"${#args[@]} deploy() {\"\n}\nfunction deploy {\n  :\n}\nfunction clean() { rm -rf x; }\n";
        assert_eq!(
            names("Bourne Again Shell (bash)", sh),
            vec![
                s("build", Function, 2, 0),
                s("deploy", Function, 5, 0),
                s("clean", Function, 8, 0),
            ]
        );
        let bat = "@echo off\n:: comment\ncall :sub\ngoto :eof\n:sub\necho hi\n:end_label\n";
        assert_eq!(
            names("Batch File", bat),
            vec![s("sub", Function, 4, 0), s("end_label", Function, 6, 0)]
        );
    }

    #[test]
    fn python_ruby_lua() {
        let py = "import os\n\nclass Shape:\n    \"\"\"Docs.\n    def not_a_method(self):\n    \"\"\"\n    def area(self):\n        def helper():\n            pass\n        return 0\n\n    async def load(self): ...\n\ndef main():\n    s = 'def fake():'\n";
        assert_eq!(
            names("Python", py),
            vec![
                s("Shape", Class, 2, 0),
                s("area", Method, 6, 1),
                s("helper", Function, 7, 2),
                s("load", Method, 11, 1),
                s("main", Function, 13, 0),
            ]
        );
        let rb = "module Shop\n  class Cart\n    def self.build\n    end\n    def add(item)\n      # def fake\n    end\n  end\nend\n=begin\ndef hidden\n=end\ndef top?; end\n";
        assert_eq!(
            names("Ruby", rb),
            vec![
                s("Shop", Module, 0, 0),
                s("Cart", Class, 1, 1),
                s("self.build", Method, 2, 2),
                s("add", Method, 4, 2),
                s("top?", Function, 12, 0),
            ]
        );
        let lua = "local M = {}\n--[[ function hidden() end ]]\nfunction M.new(x)\n  local function helper() end\nend\nM.run = function() end\n-- function commented()\nlocal function main() end\n";
        assert_eq!(
            names("Lua", lua),
            vec![
                s("M.new", Function, 2, 0),
                s("helper", Function, 3, 1),
                s("M.run", Function, 5, 0),
                s("main", Function, 7, 0),
            ]
        );
    }

    #[test]
    fn sql() {
        let src = "-- CREATE TABLE commented (id int);\nCREATE TABLE IF NOT EXISTS public.users (\n  id int\n);\ncreate or replace function add(a int) returns int as $$ select 1 $$;\nCREATE UNIQUE INDEX idx_users ON users(id); CREATE VIEW \"Active Users\" AS SELECT 1;\nINSERT INTO t VALUES ('CREATE TABLE fake');\nCREATE PROCEDURE [dbo].[Load] AS BEGIN SELECT 1 END\n";
        assert_eq!(
            names("SQL", src),
            vec![
                s("public.users", Struct, 1, 0),
                s("add", Function, 4, 0),
                s("idx_users", Key, 5, 0),
                s("\"Active Users\"", Struct, 5, 0),
                s("[dbo].[Load]", Function, 7, 0),
            ]
        );
    }

    #[test]
    fn markdown() {
        let src = "---\ntitle: x\n# not a heading\n---\n# Title\n\nIntro\n\n## Part *one* ##\n```\n# code comment\n```\nSetext\n======\nSub\n---\n- item\n---\n####### too deep\n";
        assert_eq!(
            names("Markdown", src),
            vec![
                s("Title", Heading, 4, 0),
                s("Part *one*", Heading, 8, 1),
                s("Setext", Heading, 12, 0),
                s("Sub", Heading, 14, 1),
            ]
        );
    }

    #[test]
    fn config_formats() {
        let toml = "; c\n[general]\nkey=1\n  [ section two ] ; note\n[[bin]]\nx = \"\"\"\n[not a section]\n\"\"\"\n";
        assert_eq!(
            names("TOML", toml),
            vec![
                s("general", Section, 1, 0),
                s("section two", Section, 3, 0),
                s("bin", Section, 4, 0),
            ]
        );
        let yaml = "# comment\nname: app\n\"quoted key\": 1\nnested:\n  child: 2\n- item: 3\n---\nother: |\n  text: no\nurl: http://x\n";
        assert_eq!(
            names("YAML", yaml),
            vec![
                s("name", Key, 1, 0),
                s("quoted key", Key, 2, 0),
                s("nested", Key, 3, 0),
                s("other", Key, 7, 0),
                s("url", Key, 9, 0),
            ]
        );
        let json = "{\n  \"name\": \"x:y\",\n  \"nested\": {\"inner\": 1},\n  \"list\": [{\"a\": 1}],\n  \"esc\\\"aped\": true\n}\n";
        assert_eq!(
            names("JSON", json),
            vec![
                s("name", Key, 1, 0),
                s("nested", Key, 2, 0),
                s("list", Key, 3, 0),
                s("esc\\\"aped", Key, 4, 0),
            ]
        );
        assert!(names("JSON", "[{\"a\": 1}]").is_empty());
    }

    #[test]
    fn markup_hl7_css() {
        let html = "<html>\n<!-- <div id=\"hidden\"> -->\n<div class=\"a\" id=\"main\">\n<span id='x'></span><p id=\"y\">\n";
        assert_eq!(
            names("HTML", html),
            vec![
                s("div#main", Key, 2, 0),
                s("span#x", Key, 3, 0),
                s("p#y", Key, 3, 0),
            ]
        );
        let hl7 = "MSH|^~\\&|APP|FAC|||20240101||ADT^A01|1|P|2.5\rEVN|A01\rPID|1||123\rMSH#$*@!#A#B#####ORU$R01\rOBX#1\r";
        assert_eq!(
            names("HL7", hl7),
            vec![
                s("MSH ADT^A01", Section, 0, 0),
                s("EVN", Section, 1, 1),
                s("PID", Section, 2, 1),
                s("MSH ORU^R01", Section, 3, 0),
                s("OBX", Section, 4, 1),
            ]
        );
        let css = "/* a { } */\nbody, html {\n  margin: 0;\n}\n@media (max-width: 600px) {\n  .nav a:hover,\n  .nav a:focus {\n    color: red;\n  }\n}\n";
        assert_eq!(
            names("CSS", css),
            vec![
                s("body, html", Key, 1, 0),
                s("@media (max-width: 600px)", Section, 4, 0),
                s(".nav a:hover, .nav a:focus", Key, 5, 1),
            ]
        );
    }

    #[test]
    fn unknown_language_is_empty() {
        assert!(symbols("Plain Text", "fn main() {}").is_empty());
    }

    #[test]
    fn large_files_are_fast() {
        let samples = [
            (
                "Rust",
                "pub struct S { a: u8 }\nimpl S {\n    /// docs\n    pub fn get(&self) -> u8 { let s = \"{\"; self.a }\n}\n",
            ),
            (
                "C++",
                "class A {\npublic:\n    int f(int x) const { return x; }\n};\nint g(int y)\n{\n    return y * 2;\n}\n",
            ),
            (
                "TypeScript",
                "export class A {\n  m(x: number) { return `${x}`; }\n}\nconst f = (a: number) => a;\n",
            ),
            (
                "Python",
                "class A:\n    def f(self):\n        return \"x\"\n\ndef g():\n    pass\n",
            ),
            (
                "JSON",
                "{\"key\": [1, 2, {\"a\": \"b\"}], \"other\": \"value\",\n\"more\": 1,\n",
            ),
            (
                "Markdown",
                "# Heading\n\nSome text with `code`.\n\n## More\n",
            ),
            (
                "SQL",
                "CREATE TABLE t (id int);\nSELECT * FROM t WHERE x = 'y';\n",
            ),
            ("CSS", ".a .b > c:hover { color: red; margin: 0 auto; }\n"),
        ];
        for (language, sample) in samples {
            let mut text = String::with_capacity(5_300_000);
            while text.len() < 5_000_000 {
                text.push_str(sample);
            }
            // And one huge minified line.
            text.push_str(&sample.replace('\n', " ").repeat(5_000));
            let start = std::time::Instant::now();
            let found = symbols(language, &text);
            let took = start.elapsed();
            assert!(!found.is_empty(), "{language}");
            assert!(
                took < std::time::Duration::from_secs(3),
                "{language} took {took:?}"
            );
        }
    }
}
