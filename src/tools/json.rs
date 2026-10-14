//! JSON tools: a strict parser that remembers where every value is (for the
//! tree panel and error positions), and format, minify and sort-keys writers
//! that keep strings and numbers exactly as written.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::{ToolError, ToolResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonOp {
    /// JSON is formatted as JSON; anything else as JavaScript.
    Format,
    Minify,
    Validate,
    SortKeys,
}

/// How deep objects and arrays may nest before parsing gives up.
const MAX_DEPTH: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Object,
    Array,
    String,
    Number,
    Bool,
    Null,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: Kind,
    /// The member name, for values inside an object.
    pub key: Option<String>,
    /// Where the member name is, quotes included.
    pub key_span: Option<Range<usize>>,
    /// Where the value is, as byte offsets into the parsed text.
    pub span: Range<usize>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// A parsed JSON text: `nodes[0]` is the top-level value.
#[derive(Clone, Debug)]
pub struct JsonDoc {
    pub nodes: Vec<Node>,
}

pub fn parse(text: &str) -> Result<JsonDoc, ToolError> {
    let mut p = Parser {
        s: text.as_bytes(),
        text,
        i: 0,
        nodes: Vec::new(),
    };
    if text.starts_with('\u{feff}') {
        p.i = 3;
    }
    p.ws();
    if p.i >= p.s.len() {
        return Err(ToolError::at(
            "The text is empty; expected a JSON value.",
            p.i,
        ));
    }
    p.value()?;
    p.ws();
    if p.i < p.s.len() {
        return Err(ToolError::at(
            "Unexpected text after the end of the JSON value.",
            p.i,
        ));
    }
    Ok(JsonDoc { nodes: p.nodes })
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    i: usize,
    nodes: Vec<Node>,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn unexpected(&self, expected: &str) -> ToolError {
        match self.text[self.i.min(self.text.len())..].chars().next() {
            Some(c) => ToolError::at(format!("Expected {expected}, found '{c}'."), self.i),
            None => ToolError::at(format!("Expected {expected}, but the text ended."), self.i),
        }
    }

    /// Parses one value and everything inside it, without recursion so deep
    /// nesting can't overflow the stack.
    fn value(&mut self) -> Result<(), ToolError> {
        // Open objects and arrays, innermost last.
        let mut open: Vec<usize> = Vec::new();
        let mut key: Option<(String, Range<usize>)> = None;
        loop {
            if open.len() > MAX_DEPTH {
                return Err(ToolError::at("The JSON is nested too deeply.", self.i));
            }
            let parent = open.last().copied();
            let start = self.i;
            let id = self.nodes.len();
            let (k, key_span) = key.take().unzip();
            self.nodes.push(Node {
                kind: Kind::Null,
                key: k,
                key_span,
                span: start..start,
                parent,
                children: Vec::new(),
            });
            if let Some(p) = parent {
                self.nodes[p].children.push(id);
            }
            let scalar = match self.peek() {
                Some(c @ (b'{' | b'[')) => {
                    self.i += 1;
                    self.ws();
                    let (kind, close) = if c == b'{' {
                        (Kind::Object, b'}')
                    } else {
                        (Kind::Array, b']')
                    };
                    self.nodes[id].kind = kind;
                    if self.peek() == Some(close) {
                        self.i += 1;
                        self.nodes[id].span.end = self.i;
                    } else {
                        open.push(id);
                        if kind == Kind::Object {
                            key = Some(self.member_name()?);
                        }
                        continue;
                    }
                    None
                }
                Some(b'"') => {
                    self.string()?;
                    Some(Kind::String)
                }
                Some(b'-' | b'0'..=b'9') => {
                    self.number()?;
                    Some(Kind::Number)
                }
                Some(b't') => Some(self.literal("true", Kind::Bool)?),
                Some(b'f') => Some(self.literal("false", Kind::Bool)?),
                Some(b'n') => Some(self.literal("null", Kind::Null)?),
                _ => return Err(self.unexpected("a JSON value")),
            };
            if let Some(kind) = scalar {
                self.nodes[id].kind = kind;
                self.nodes[id].span.end = self.i;
            }
            // Close finished containers until one wants another member.
            loop {
                let Some(&top) = open.last() else {
                    return Ok(());
                };
                self.ws();
                let object = self.nodes[top].kind == Kind::Object;
                match self.peek() {
                    Some(b',') => {
                        self.i += 1;
                        self.ws();
                        if object {
                            key = Some(self.member_name()?);
                        }
                        break;
                    }
                    Some(b'}') if object => {}
                    Some(b']') if !object => {}
                    _ if object => return Err(self.unexpected("',' or '}'")),
                    _ => return Err(self.unexpected("',' or ']'")),
                }
                self.i += 1;
                self.nodes[top].span.end = self.i;
                open.pop();
            }
        }
    }

    /// Reads `"name":` and the space after it.
    fn member_name(&mut self) -> Result<(String, Range<usize>), ToolError> {
        if self.peek() != Some(b'"') {
            return Err(self.unexpected("a member name in double quotes"));
        }
        let start = self.i;
        let name = self.string()?;
        let span = start..self.i;
        self.ws();
        if self.peek() != Some(b':') {
            return Err(self.unexpected("':'"));
        }
        self.i += 1;
        self.ws();
        Ok((name, span))
    }

    fn literal(&mut self, word: &str, kind: Kind) -> Result<Kind, ToolError> {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(kind)
        } else {
            Err(self.unexpected("a JSON value"))
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.i;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        self.i - start
    }

    fn number(&mut self) -> Result<(), ToolError> {
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.i += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(ToolError::at(
                        "Numbers can't have leading zeros.",
                        self.i - 1,
                    ));
                }
            }
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.unexpected("a digit")),
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if self.digits() == 0 {
                return Err(self.unexpected("a digit after the decimal point"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if self.digits() == 0 {
                return Err(self.unexpected("a digit in the exponent"));
            }
        }
        Ok(())
    }

    /// Reads a string starting at the opening quote and returns its value.
    fn string(&mut self) -> Result<String, ToolError> {
        let start = self.i;
        self.i += 1;
        let mut out = String::new();
        loop {
            let run = self.i;
            while let Some(c) = self.peek() {
                if c == b'"' || c == b'\\' || c < 0x20 {
                    break;
                }
                self.i += 1;
            }
            out.push_str(&self.text[run..self.i]);
            match self.peek() {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let esc = self.peek();
                    self.i += 1;
                    match esc {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\u{8}'),
                        Some(b'f') => out.push('\u{c}'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            let hi = self.hex4()?;
                            let c = if (0xD800..0xDC00).contains(&hi)
                                && self.s[self.i..].starts_with(b"\\u")
                            {
                                self.i += 2;
                                let lo = self.hex4()?;
                                char::from_u32(
                                    0x10000
                                        + ((hi - 0xD800) << 10)
                                        + (lo.wrapping_sub(0xDC00) & 0x3FF),
                                )
                            } else {
                                char::from_u32(hi)
                            };
                            out.push(c.unwrap_or('\u{FFFD}'));
                        }
                        _ => {
                            return Err(ToolError::at(
                                "Unknown escape sequence in a string.",
                                self.i - 2,
                            ));
                        }
                    }
                }
                Some(_) => {
                    return Err(ToolError::at(
                        "Strings can't contain line breaks or control characters; use an escape like \\n.",
                        self.i,
                    ));
                }
                None => return Err(ToolError::at("This string is never closed.", start)),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ToolError> {
        let digits = self.s.get(self.i..self.i + 4).unwrap_or_default();
        let ok = digits.len() == 4 && digits.iter().all(u8::is_ascii_hexdigit);
        if !ok {
            return Err(ToolError::at("Expected four hex digits after \\u.", self.i));
        }
        self.i += 4;
        Ok(u32::from_str_radix(std::str::from_utf8(digits).unwrap_or("0"), 16).unwrap_or(0))
    }
}

impl JsonDoc {
    /// Writes the document back out. `indent` of `None` minifies.
    pub fn write(&self, text: &str, indent: Option<&str>, sort_keys: bool) -> String {
        let mut out = String::with_capacity(text.len() + text.len() / 4);
        self.write_node(0, text, indent, sort_keys, 0, &mut out);
        out
    }

    fn write_node(
        &self,
        id: usize,
        text: &str,
        indent: Option<&str>,
        sort: bool,
        depth: usize,
        out: &mut String,
    ) {
        let node = &self.nodes[id];
        let (open, close) = match node.kind {
            Kind::Object => ('{', '}'),
            Kind::Array => ('[', ']'),
            _ => {
                out.push_str(&text[node.span.clone()]);
                return;
            }
        };
        out.push(open);
        if node.children.is_empty() {
            out.push(close);
            return;
        }
        let mut children = node.children.clone();
        if sort && node.kind == Kind::Object {
            children.sort_by(|&a, &b| self.nodes[a].key.cmp(&self.nodes[b].key));
        }
        let newline = |out: &mut String, depth: usize| {
            if let Some(unit) = indent {
                out.push('\n');
                for _ in 0..depth {
                    out.push_str(unit);
                }
            }
        };
        for (n, &child) in children.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            newline(out, depth + 1);
            if let Some(span) = &self.nodes[child].key_span {
                out.push_str(&text[span.clone()]);
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
            }
            self.write_node(child, text, indent, sort, depth + 1, out);
        }
        newline(out, depth);
        out.push(close);
    }

    /// A short description of a value for the tree: scalars as written
    /// (strings decoded and shortened), containers by their size.
    pub fn preview(&self, id: usize, text: &str) -> String {
        let node = &self.nodes[id];
        match node.kind {
            Kind::Object => format!("{{{}}}", node.children.len()),
            Kind::Array => format!("[{}]", node.children.len()),
            Kind::String => {
                let s = self.string_value(id, text);
                let mut short: String = s.chars().take(120).collect();
                if short.len() < s.len() {
                    short.push('…');
                }
                format!("\"{}\"", short.replace('\n', "⏎"))
            }
            _ => text[node.span.clone()].to_owned(),
        }
    }

    /// A string node's decoded value.
    pub fn string_value(&self, id: usize, text: &str) -> String {
        let raw = &text[self.nodes[id].span.clone()];
        let mut p = Parser {
            s: raw.as_bytes(),
            text: raw,
            i: 0,
            nodes: Vec::new(),
        };
        p.string().unwrap_or_default()
    }

    /// The value as it would be copied: strings decoded, anything else as written.
    pub fn copy_value(&self, id: usize, text: &str) -> String {
        match self.nodes[id].kind {
            Kind::String => self.string_value(id, text),
            _ => text[self.nodes[id].span.clone()].to_owned(),
        }
    }

    /// The node's index within its parent array.
    pub fn index_in_parent(&self, id: usize) -> Option<usize> {
        let parent = self.nodes[id].parent?;
        if self.nodes[parent].kind != Kind::Array {
            return None;
        }
        self.nodes[parent].children.iter().position(|&c| c == id)
    }

    /// A JavaScript-style path to the node, like `$.items[2].name`.
    pub fn path(&self, id: usize) -> String {
        let mut parts = Vec::new();
        let mut cur = id;
        while let Some(parent) = self.nodes[cur].parent {
            let node = &self.nodes[cur];
            parts.push(match (&node.key, self.index_in_parent(cur)) {
                (Some(k), _) if is_identifier(k) => format!(".{k}"),
                (Some(k), _) => format!("[{}]", serde_json::to_string(k).unwrap_or_default()),
                (None, Some(i)) => format!("[{i}]"),
                (None, None) => String::new(),
            });
            cur = parent;
        }
        parts.push("$".to_owned());
        parts.reverse();
        parts.concat()
    }

    /// The innermost node whose value contains `offset`.
    pub fn node_at(&self, offset: usize) -> Option<usize> {
        let mut cur = 0;
        if !self.nodes.first()?.span.contains(&offset) {
            return None;
        }
        'descend: loop {
            for &child in &self.nodes[cur].children {
                let n = &self.nodes[child];
                let start = n.key_span.as_ref().map_or(n.span.start, |k| k.start);
                if (start..n.span.end).contains(&offset) {
                    cur = child;
                    continue 'descend;
                }
            }
            return Some(cur);
        }
    }
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

#[cfg(test)]
pub fn format(text: &str, indent: &str) -> ToolResult {
    Ok(parse(text)?.write(text, Some(indent), false))
}

#[cfg(test)]
pub fn minify(text: &str) -> ToolResult {
    Ok(parse(text)?.write(text, None, false))
}

pub fn sort_keys(text: &str, indent: &str) -> ToolResult {
    Ok(parse(text)?.write(text, Some(indent), true))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        r#"{"b": [1, 2.5e3, {"x": null}], "a": "h\u00e9\n", "c": {}, "d": [], "e": true}"#;

    #[test]
    fn formats_minifies_and_sorts() {
        let pretty = format(SAMPLE, "  ").unwrap();
        assert_eq!(
            pretty,
            "{\n  \"b\": [\n    1,\n    2.5e3,\n    {\n      \"x\": null\n    }\n  ],\n  \"a\": \"h\\u00e9\\n\",\n  \"c\": {},\n  \"d\": [],\n  \"e\": true\n}"
        );
        assert_eq!(
            minify(&pretty).unwrap(),
            r#"{"b":[1,2.5e3,{"x":null}],"a":"h\u00e9\n","c":{},"d":[],"e":true}"#
        );
        let sorted = sort_keys(SAMPLE, "\t").unwrap();
        let keys: Vec<&str> = sorted
            .lines()
            .filter(|l| l.starts_with("\t\""))
            .map(|l| &l[2..3])
            .collect();
        assert_eq!(keys, ["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn errors_have_positions() {
        let cases = [
            ("{\"a\": 1,}", 8, "member name"),
            ("[1 2]", 3, "',' or ']'"),
            ("{\"a\" 1}", 5, "':'"),
            ("\"abc", 0, "never closed"),
            ("[01]", 1, "leading zeros"),
            ("[1.]", 3, "decimal point"),
            ("{} x", 3, "after the end"),
            ("", 0, "empty"),
            ("[\"a\nb\"]", 3, "line breaks"),
            ("[tru]", 1, "JSON value"),
        ];
        for (text, offset, words) in cases {
            let e = parse(text).unwrap_err();
            assert_eq!(e.offset, Some(offset), "{text}: {}", e.message);
            assert!(e.message.contains(words), "{text}: {}", e.message);
        }
        let deep = "[".repeat(2000);
        assert!(parse(&deep).unwrap_err().message.contains("deeply"));
    }

    #[test]
    fn tree_paths_and_values() {
        let doc = parse(SAMPLE).unwrap();
        let root = &doc.nodes[0];
        assert_eq!(root.kind, Kind::Object);
        assert_eq!(root.children.len(), 5);
        let b = root.children[0];
        let x_obj = doc.nodes[b].children[2];
        let x = doc.nodes[x_obj].children[0];
        assert_eq!(doc.path(x), "$.b[2].x");
        assert_eq!(doc.preview(b, SAMPLE), "[3]");
        let a = root.children[1];
        assert_eq!(doc.string_value(a, SAMPLE), "hé\n");
        assert_eq!(doc.preview(a, SAMPLE), "\"hé⏎\"");
        let offset = SAMPLE.find("null").unwrap() + 1;
        assert_eq!(doc.node_at(offset), Some(x));
        let doc = parse(r#"{"a b": {"ü": 1}}"#).unwrap();
        assert_eq!(doc.path(2), "$[\"a b\"].ü");
        let doc = parse("\"\\ud83d\\ude00\"").unwrap();
        assert_eq!(doc.string_value(0, "\"\\ud83d\\ude00\""), "😀");
    }
}
