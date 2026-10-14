//! JavaScript formatting and minifying, for code that isn't strict JSON
//! (JSON with comments counts as JavaScript here). Both work on tokens, so
//! strings, template literals, regular expressions and comments come through
//! untouched; neither needs the code to be complete or correct.

use super::{ToolError, ToolResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Word,
    Num,
    Str,
    Template,
    Regex,
    LineComment,
    BlockComment,
    Punct,
}

#[derive(Clone, Copy, Debug)]
struct Token<'a> {
    kind: Kind,
    text: &'a str,
    /// Line breaks between this token and the previous one in the source.
    newlines: usize,
}

impl Token<'_> {
    fn is(&self, p: &str) -> bool {
        self.kind == Kind::Punct && self.text == p
    }

    fn is_word(&self, w: &str) -> bool {
        self.kind == Kind::Word && self.text == w
    }

    fn is_comment(&self) -> bool {
        matches!(self.kind, Kind::LineComment | Kind::BlockComment)
    }
}

/// Words after which a `/` starts a regular expression and `-` is unary.
const EXPRESSION_KEYWORDS: &[&str] = &[
    "return",
    "typeof",
    "instanceof",
    "in",
    "of",
    "new",
    "delete",
    "void",
    "throw",
    "case",
    "do",
    "else",
    "yield",
    "await",
];

/// Words that take a space before a following `(`.
const SPACED_KEYWORDS: &[&str] = &[
    "if",
    "for",
    "while",
    "switch",
    "catch",
    "with",
    "return",
    "typeof",
    "await",
    "yield",
    "void",
    "delete",
    "in",
    "of",
    "instanceof",
    "case",
    "throw",
    "new",
    "else",
    "do",
];

const PUNCTUATORS: &[&str] = &[
    ">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=", "=>", "==", "!=",
    "<=", ">=", "&&", "||", "??", "?.", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "**", "<<", ">>",
];

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$' || c == '#' || c == '\\'
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$' || c == '\\' || c == '\u{200c}' || c == '\u{200d}'
}

/// True if a `/` after `prev` starts a regular expression rather than dividing.
fn regex_allowed(prev: Option<&Token>) -> bool {
    match prev {
        None => true,
        Some(t) => match t.kind {
            Kind::Word => EXPRESSION_KEYWORDS.contains(&t.text),
            Kind::Num | Kind::Str | Kind::Template | Kind::Regex => false,
            Kind::Punct => !matches!(t.text, ")" | "]" | "}"),
            Kind::LineComment | Kind::BlockComment => true,
        },
    }
}

fn tokenize(src: &str) -> Result<Vec<Token<'_>>, ToolError> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0;
    let mut newlines = 0;
    let at = |i: usize| src[i..].chars().next();
    while let Some(c) = at(i) {
        let start = i;
        if c == '\n' {
            newlines += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() || c == '\u{feff}' {
            i += c.len_utf8();
            continue;
        }
        let prev = tokens.iter().rev().find(|t| !t.is_comment());
        let rest = &src[i..];
        let kind = if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
            Kind::LineComment
        } else if let Some(body) = rest.strip_prefix("/*") {
            match body.find("*/") {
                Some(end) => i += end + 4,
                None => return Err(ToolError::at("This comment is never closed.", start)),
            }
            Kind::BlockComment
        } else if c == '"' || c == '\'' {
            i += 1;
            let mut closed = false;
            while let Some(d) = at(i) {
                if d == '\n' {
                    break;
                }
                i += d.len_utf8();
                if d == '\\' {
                    i += at(i).map_or(0, char::len_utf8);
                } else if d == c {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err(ToolError::at("This string is never closed.", start));
            }
            Kind::Str
        } else if c == '`' {
            i = template_end(src, i)
                .ok_or_else(|| ToolError::at("This template literal is never closed.", start))?;
            Kind::Template
        } else if c.is_ascii_digit()
            || (c == '.' && rest[1..].starts_with(|d: char| d.is_ascii_digit()))
        {
            i += 1;
            while let Some(d) = at(i) {
                // An exponent's sign belongs to the number (but `0xE-1` is a subtraction).
                let exponent_sign = (d == '+' || d == '-')
                    && matches!(src[..i].chars().last(), Some('e' | 'E'))
                    && !src[start..].starts_with("0x")
                    && !src[start..].starts_with("0X");
                if d.is_ascii_alphanumeric() || d == '.' || d == '_' || exponent_sign {
                    i += 1;
                } else {
                    break;
                }
            }
            Kind::Num
        } else if is_ident_start(c) {
            i += c.len_utf8();
            while let Some(d) = at(i).filter(|&d| is_ident_char(d)) {
                i += d.len_utf8();
            }
            Kind::Word
        } else if c == '/'
            && regex_allowed(prev)
            && let Some(end) = regex_end(src, i)
        {
            i = end;
            Kind::Regex
        } else {
            let p = PUNCTUATORS
                .iter()
                .find(|p| rest.starts_with(**p))
                // `a?.5:b` is a conditional, not optional chaining.
                .filter(|p| **p != "?." || !rest[2..].starts_with(|d: char| d.is_ascii_digit()));
            i += p.map_or(c.len_utf8(), |p| p.len());
            Kind::Punct
        };
        tokens.push(Token {
            kind,
            text: &src[start..i],
            newlines,
        });
        newlines = 0;
    }
    Ok(tokens)
}

/// The end of the template literal starting at `start`, skipping `${...}`
/// expressions (which can hold strings and further templates).
fn template_end(src: &str, start: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'`' => return Some(i + 1),
            b'$' if b.get(i + 1) == Some(&b'{') => {
                i += 2;
                let mut depth = 1;
                while i < b.len() && depth > 0 {
                    match b[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'`' => {
                            i = template_end(src, i)? - 1;
                        }
                        q @ (b'"' | b'\'') => {
                            i += 1;
                            while i < b.len() && b[i] != q {
                                if b[i] == b'\\' {
                                    i += 1;
                                }
                                i += 1;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// The end of the regular expression literal starting at `start`, or `None`
/// if the line ends first (then the `/` is a division after all).
fn regex_end(src: &str, start: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut i = start + 1;
    let mut in_class = false;
    if b.get(i) == Some(&b'*') || b.get(i) == Some(&b'/') {
        return None;
    }
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'\n' => return None,
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => {
                i += 1;
                while i < b.len() && (b[i].is_ascii_alphabetic()) {
                    i += 1;
                }
                return Some(i);
            }
            _ => {}
        }
        i += 1;
    }
    None
}

// ----------------------------------------------------------------------
// Minify

fn ends_statement(t: &Token) -> bool {
    match t.kind {
        Kind::Word | Kind::Num | Kind::Str | Kind::Template | Kind::Regex => true,
        Kind::Punct => matches!(t.text, ")" | "]" | "}" | "++" | "--"),
        _ => false,
    }
}

fn starts_statement(t: &Token) -> bool {
    match t.kind {
        Kind::Word | Kind::Num | Kind::Str | Kind::Template | Kind::Regex => true,
        Kind::Punct => matches!(
            t.text,
            "{" | "(" | "[" | "++" | "--" | "!" | "~" | "+" | "-"
        ),
        _ => false,
    }
}

/// Removes comments and every space and line break the code doesn't need.
/// A line break stays where a missing semicolon might depend on it.
pub fn minify(src: &str) -> ToolResult {
    let tokens = tokenize(src)?;
    let mut out = String::with_capacity(src.len() / 2);
    let mut prev: Option<Token> = None;
    let mut pending_newline = false;
    for t in tokens {
        if t.is_comment() {
            if t.kind == Kind::LineComment || t.text.contains('\n') {
                pending_newline = true;
            }
            continue;
        }
        if let Some(p) = &prev {
            if (t.newlines > 0 || pending_newline) && ends_statement(p) && starts_statement(&t) {
                out.push('\n');
            } else if needs_space(p, &t) {
                out.push(' ');
            }
        }
        out.push_str(t.text);
        pending_newline = false;
        prev = Some(t);
    }
    Ok(out)
}

/// True if two tokens would run together without a space.
fn needs_space(a: &Token, b: &Token) -> bool {
    let (Some(x), Some(y)) = (a.text.chars().last(), b.text.chars().next()) else {
        return false;
    };
    (is_ident_char(x) && (is_ident_char(y) || y == '#'))
        || (a.kind == Kind::Num && y == '.')
        || (x == '+' && y == '+')
        || (x == '-' && y == '-')
        || (x == '/' && (y == '/' || y == '*'))
        || (a.kind == Kind::Regex && is_ident_char(y))
}

// ----------------------------------------------------------------------
// Format

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Frame {
    /// A block of statements, or the top level.
    Block {
        /// Inside a `switch` after a `case` label: one extra level of indent.
        in_case: bool,
        after_do: bool,
    },
    Object,
    /// Braces kept on one line: `import { a, b }`, `const { x } = o`.
    Inline,
    Paren,
    Bracket,
}

struct Writer<'a> {
    out: String,
    unit: &'a str,
    depth: usize,
    line_start: bool,
}

impl Writer<'_> {
    fn newline(&mut self, blank: bool) {
        while self.out.ends_with(' ') {
            self.out.pop();
        }
        if self.out.is_empty() {
            return;
        }
        if !self.line_start {
            self.out.push('\n');
        }
        if blank && !self.out.ends_with("\n\n") {
            self.out.push('\n');
        }
        self.line_start = true;
    }

    fn write(&mut self, text: &str, space: bool) {
        if self.line_start {
            for _ in 0..self.depth {
                self.out.push_str(self.unit);
            }
            self.line_start = false;
        } else if space && !self.out.is_empty() && !self.out.ends_with(' ') {
            self.out.push(' ');
        }
        self.out.push_str(text);
    }
}

fn is_unary_position(prev: Option<&Token>) -> bool {
    match prev {
        None => true,
        Some(t) => match t.kind {
            Kind::Word => EXPRESSION_KEYWORDS.contains(&t.text),
            Kind::Punct => !matches!(t.text, ")" | "]" | "}"),
            Kind::Num | Kind::Str | Kind::Template | Kind::Regex => false,
            _ => true,
        },
    }
}

/// Re-indents and re-spaces JavaScript (or JSON with comments).
pub fn format(src: &str, unit: &str) -> ToolResult {
    let tokens = tokenize(src)?;
    let mut w = Writer {
        out: String::with_capacity(src.len() * 5 / 4),
        unit,
        depth: 0,
        line_start: true,
    };
    let mut stack = vec![Frame::Block {
        in_case: false,
        after_do: false,
    }];
    // Inside a `case` label, waiting for its colon; counts open `?`s.
    let mut case_label: Option<usize> = None;
    let mut ternaries: Vec<usize> = Vec::new();
    let mut prev: Option<Token> = None;
    let mut prev_was_unary = false;
    let mut pending_do = false;
    // A line break held back so a comment can stay at the end of the line.
    let mut deferred_newline = false;

    for (n, &t) in tokens.iter().enumerate() {
        let next = tokens[n + 1..].iter().find(|t| !t.is_comment());
        let trailing_comment = tokens
            .get(n + 1)
            .is_some_and(|c| c.is_comment() && c.newlines == 0);
        let mut break_line = |w: &mut Writer| {
            if trailing_comment {
                deferred_newline = true;
            } else {
                w.newline(false);
            }
        };
        let frame = *stack.last().unwrap_or(&Frame::Paren);
        let in_statements = matches!(frame, Frame::Block { .. });

        // Line breaks from the source between statements (code without semicolons).
        if let Some(p) = &prev
            && t.newlines > 0
            && !w.line_start
            && !t.is_comment()
            && in_statements
            && ends_statement(p)
            && starts_statement(&t)
            && !matches!(t.text, "(" | "[" | "+" | "-")
            && !(p.is("}") && (t.is_word("else") || t.is_word("catch") || t.is_word("finally")))
        {
            w.newline(t.newlines > 1);
        } else if t.newlines > 1 && w.line_start && in_statements && prev.is_some() {
            w.newline(true);
        }

        match t.kind {
            Kind::LineComment => {
                if t.newlines > 0 {
                    w.newline(t.newlines > 1);
                }
                w.write(t.text, true);
                w.newline(false);
                deferred_newline = false;
                prev_was_unary = false;
                continue;
            }
            Kind::BlockComment => {
                if t.newlines > 0 {
                    w.newline(t.newlines > 1);
                }
                w.write(t.text, true);
                if std::mem::take(&mut deferred_newline)
                    || tokens.get(n + 1).is_some_and(|n| n.newlines > 0)
                {
                    w.newline(false);
                }
                prev_was_unary = false;
                continue;
            }
            _ => {}
        }

        let p = prev.as_ref();
        if t.kind == Kind::Punct {
            match t.text {
                "{" => {
                    let inline = p.is_some_and(|p| {
                        matches!(p.text, "import" | "export" | "const" | "let" | "var")
                            && p.kind == Kind::Word
                    });
                    let object = p.is_some_and(|p| {
                        p.kind == Kind::Punct
                            && matches!(
                                p.text,
                                "(" | ","
                                    | ":"
                                    | "="
                                    | "["
                                    | "?"
                                    | "||"
                                    | "&&"
                                    | "??"
                                    | "!"
                                    | "=="
                                    | "==="
                                    | "+="
                                    | "..."
                                    | "-"
                                    | "+"
                                    | "<"
                                    | ">"
                            )
                            || p.is_word("return")
                            || p.is_word("yield")
                            || p.is_word("await")
                    }) || (p.is_none() && next.is_some_and(|n| n.kind == Kind::Str));
                    let space = !p.is_some_and(|p| p.is("(") || p.is("[") || p.is("..."));
                    w.write("{", space);
                    if next.is_some_and(|n| n.is("}")) {
                        stack.push(Frame::Paren);
                    } else if inline {
                        stack.push(Frame::Inline);
                    } else {
                        stack.push(if object {
                            Frame::Object
                        } else {
                            Frame::Block {
                                in_case: false,
                                after_do: std::mem::take(&mut pending_do),
                            }
                        });
                        w.depth += 1;
                        break_line(&mut w);
                    }
                }
                "}" => {
                    let closed = stack.pop().unwrap_or(Frame::Paren);
                    if matches!(closed, Frame::Block { .. } | Frame::Object) {
                        if let Frame::Block { in_case: true, .. } = closed {
                            w.depth = w.depth.saturating_sub(1);
                        }
                        w.depth = w.depth.saturating_sub(1);
                        w.newline(false);
                    }
                    w.write("}", closed == Frame::Inline);
                    let after_do = matches!(closed, Frame::Block { after_do: true, .. });
                    // At statement level a block ends the line, unless the
                    // statement goes on (`} else`, `};`, `}).then`, `} = x`).
                    let stays = closed == Frame::Inline
                        || next.is_none_or(|n| {
                            n.kind == Kind::Punct
                                && !matches!(
                                    n.text,
                                    "(" | "[" | "{" | "+" | "-" | "!" | "~" | "++" | "--"
                                )
                                || n.is_word("else")
                                || n.is_word("catch")
                                || n.is_word("finally")
                                || (after_do && n.is_word("while"))
                        });
                    if !stays && matches!(stack.last(), Some(Frame::Block { .. })) {
                        break_line(&mut w);
                    }
                }
                "(" | "[" => {
                    let call = p.is_some_and(|p| {
                        (p.kind == Kind::Word && !SPACED_KEYWORDS.contains(&p.text))
                            || p.is(")")
                            || p.is("]")
                            || p.is("?.")
                            || p.kind == Kind::Template
                            || p.kind == Kind::Str && t.text == "["
                    }) || p
                        .is_some_and(|p| p.is("(") || p.is("[") || p.is("!") || p.is("..."))
                        || prev_was_unary;
                    w.write(t.text, !call);
                    stack.push(if t.text == "(" {
                        Frame::Paren
                    } else {
                        Frame::Bracket
                    });
                }
                ")" | "]" => {
                    stack.pop();
                    w.write(t.text, false);
                }
                ";" => {
                    w.write(";", false);
                    if in_statements {
                        break_line(&mut w);
                    }
                }
                "," => {
                    w.write(",", false);
                    if matches!(frame, Frame::Object) {
                        break_line(&mut w);
                    }
                }
                "." | "?." => w.write(t.text, false),
                "?" => {
                    ternaries.push(stack.len());
                    w.write("?", true);
                }
                ":" => {
                    if case_label.is_some_and(|d| d == stack.len()) && ternaries.is_empty() {
                        case_label = None;
                        w.write(":", false);
                        if let Some(Frame::Block { in_case, .. }) = stack.last_mut()
                            && !*in_case
                        {
                            *in_case = true;
                            w.depth += 1;
                        }
                        if !next.is_some_and(|n| n.is("{")) {
                            break_line(&mut w);
                        }
                    } else if ternaries.last() == Some(&stack.len()) {
                        ternaries.pop();
                        w.write(":", true);
                    } else {
                        w.write(":", false);
                    }
                }
                "!" | "~" => w.write(t.text, !p.is_some_and(|p| p.is("(") || p.is("["))),
                "++" | "--" => {
                    let postfix = p.is_some_and(|p| {
                        matches!(p.kind, Kind::Word | Kind::Num)
                            && !EXPRESSION_KEYWORDS.contains(&p.text)
                            || p.is(")")
                            || p.is("]")
                    });
                    w.write(
                        t.text,
                        !postfix && !p.is_some_and(|p| p.is("(") || p.is("[")),
                    );
                }
                "-" | "+" if is_unary_position(p) => {
                    w.write(t.text, !p.is_some_and(|p| p.is("(") || p.is("[")));
                }
                "..." => w.write(
                    "...",
                    !p.is_some_and(|p| p.is("(") || p.is("[") || p.is("{")),
                ),
                _ => w.write(t.text, true),
            }
        } else {
            let tight = p.is_some_and(|p| {
                p.is("(")
                    || p.is("[")
                    || p.is(".")
                    || p.is("?.")
                    || p.is("...")
                    || p.is("!")
                    || p.is("~")
                    || (p.is("++") || p.is("--") || p.is("-") || p.is("+")) && prev_was_unary
            });
            if t.kind == Kind::Word {
                match t.text {
                    "case" | "default"
                        if in_statements
                            && !p.is_some_and(|p| p.is("."))
                            && (t.text == "case" || next.is_some_and(|n| n.is(":"))) =>
                    {
                        if let Some(Frame::Block { in_case, .. }) = stack.last_mut()
                            && *in_case
                        {
                            *in_case = false;
                            w.depth = w.depth.saturating_sub(1);
                        }
                        w.newline(false);
                        case_label = Some(stack.len());
                    }
                    "do" => pending_do = true,
                    _ => {}
                }
            }
            w.write(t.text, !tight);
        }
        prev_was_unary = t.kind == Kind::Punct
            && matches!(t.text, "-" | "+" | "++" | "--" | "!" | "~")
            && is_unary_position(p);
        prev = Some(t);
    }
    w.newline(false);
    let mut out = w.out;
    while out.ends_with('\n') {
        out.pop();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_statements_and_blocks() {
        let src = "function f(a,b){if(a>b){return a-b}else{return -b}}var x=f(1,2);for(let i=0;i<3;i++){x+=i}";
        assert_eq!(
            format(src, "  ").unwrap(),
            "function f(a, b) {\n  if (a > b) {\n    return a - b\n  } else {\n    return -b\n  }\n}\nvar x = f(1, 2);\nfor (let i = 0; i < 3; i++) {\n  x += i\n}"
        );
    }

    #[test]
    fn formats_objects_arrays_and_switch() {
        let src = "const o={a:1,b:[1,2],c:{d:x?1:2}};switch(k){case 1:go();break;default:stop()}";
        assert_eq!(
            format(src, "  ").unwrap(),
            "const o = {\n  a: 1,\n  b: [1, 2],\n  c: {\n    d: x ? 1 : 2\n  }\n};\nswitch (k) {\n  case 1:\n    go();\n    break;\n  default:\n    stop()\n}"
        );
    }

    #[test]
    fn keeps_strings_regexes_comments_and_templates() {
        let src = "// head\nlet s='a  b',r=/[/]+/g, t=`x ${ {a:1}.a } y`;/* c */\nf(s/2)";
        let out = format(src, "\t").unwrap();
        assert!(
            out.starts_with("// head\nlet s = 'a  b', r = /[/]+/g, t = `x ${ {a:1}.a } y`;"),
            "{out}"
        );
        assert!(out.contains("/* c */"));
        assert!(out.ends_with("f(s / 2)"), "{out}");
    }

    #[test]
    fn statements_without_semicolons_stay_apart() {
        let src = "let a = 1\nlet b = a\n\n\nfoo(b)\n.then(x => x)";
        assert_eq!(
            format(src, "  ").unwrap(),
            "let a = 1\nlet b = a\n\nfoo(b).then(x => x)"
        );
    }

    #[test]
    fn minifies_safely() {
        let src = "// c\nfunction f ( a ) {\n  return a + +b - -c / 2 // x\n}\nlet r = /a b/g\nlet s = 'a  b'\nx = 1 .toString()";
        let min = minify(src).unwrap();
        assert_eq!(
            min,
            "function f(a){return a+ +b- -c/2}\nlet r=/a b/g\nlet s='a  b'\nx=1 .toString()"
        );
        assert!(minify("var a = 'x").is_err());
        assert!(minify("a /* b").is_err());
    }

    #[test]
    fn modern_syntax() {
        let src = "import {a,b} from './x.js';export default class A{#p=1;get v(){return this.#p?.v??0}}\nconst {x,y}=o;label:for(const k in o){continue label}\n(function(){})()";
        assert_eq!(
            format(src, "  ").unwrap(),
            "import { a, b } from './x.js';\nexport default class A {\n  #p = 1;\n  get v() {\n    return this.#p?.v ?? 0\n  }\n}\nconst { x, y } = o;\nlabel: for (const k in o) {\n  continue label\n}\n(function() {})()"
        );
    }

    #[test]
    fn json_with_comments_formats() {
        let src = "{\n// settings\n\"a\": 1, /* two */ \"b\": [true, null]}";
        assert_eq!(
            format(src, "  ").unwrap(),
            "{\n  // settings\n  \"a\": 1, /* two */\n  \"b\": [true, null]\n}"
        );
    }
}
