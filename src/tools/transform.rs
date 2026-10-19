//! The Transform menu's text tools: keeping or removing lines that match,
//! counting distinct lines, adding text around lines, numbering lines,
//! reflowing paragraphs, hashes, hex and JWT decoding. Everything here works
//! on plain strings; the app applies the results to the editor.

use serde::{Deserialize, Serialize};

use super::convert::DataFormat;
use super::{ToolError, ToolResult};

/// A Transform menu command. The ones with settings carry them, so macros
/// and plugins can run them again without the dialog.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TransformOp {
    /// Keep (or remove) the lines that contain `pattern`.
    FilterLines {
        pattern: String,
        regex: bool,
        match_case: bool,
        keep: bool,
    },
    /// Each distinct line once, with how often it appears, in a new tab.
    CountLines,
    /// Adds text before and after each line.
    AffixLines {
        prefix: String,
        suffix: String,
        skip_empty: bool,
    },
    NumberLines {
        start: i64,
        step: i64,
        /// Text between the number and the line.
        separator: String,
        pad: NumberPad,
        skip_empty: bool,
    },
    /// Rewraps paragraphs so no line is longer than `width` columns.
    Reflow {
        width: usize,
    },
    Convert {
        from: DataFormat,
        to: DataFormat,
    },
    Hash(HashKind),
    HexEncode,
    HexDecode,
    JwtDecode,
    /// Sends the text through a shell command and puts back its output.
    Pipe {
        command: String,
    },
}

/// How line numbers are padded to the width of the largest one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumberPad {
    #[default]
    None,
    Spaces,
    Zeros,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashKind {
    Md5,
    Sha1,
    Sha256,
    Sha512,
    Crc32,
}

impl HashKind {
    pub fn name(self) -> &'static str {
        match self {
            HashKind::Md5 => "MD5",
            HashKind::Sha1 => "SHA-1",
            HashKind::Sha256 => "SHA-256",
            HashKind::Sha512 => "SHA-512",
            HashKind::Crc32 => "CRC32",
        }
    }
}

/// Calls `f` on each line of `text` (without its line break); `None` drops
/// the line. A final line break stays a final line break.
fn map_lines(text: &str, mut f: impl FnMut(&str) -> Option<String>) -> String {
    if text.is_empty() {
        return String::new();
    }
    let (body, trailing) = match text.strip_suffix('\n') {
        Some(body) => (body, true),
        None => (text, false),
    };
    let lines: Vec<String> = body.split('\n').filter_map(&mut f).collect();
    let mut out = lines.join("\n");
    if trailing && !lines.is_empty() {
        out.push('\n');
    }
    out
}

/// What [`filter_lines`] looks for in each line.
pub struct LineMatcher {
    regex: Option<regex::Regex>,
    plain: String,
    match_case: bool,
}

impl LineMatcher {
    pub fn new(pattern: &str, regex: bool, match_case: bool) -> Result<Self, ToolError> {
        let regex = if regex {
            Some(
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(!match_case)
                    .build()
                    .map_err(|e| ToolError::new(e.to_string()))?,
            )
        } else {
            None
        };
        let plain = if match_case {
            pattern.to_owned()
        } else {
            pattern.to_lowercase()
        };
        Ok(Self {
            regex,
            plain,
            match_case,
        })
    }

    pub fn is_match(&self, line: &str) -> bool {
        match &self.regex {
            Some(re) => re.is_match(line),
            None if self.match_case => line.contains(&self.plain),
            None => line.to_lowercase().contains(&self.plain),
        }
    }
}

/// Keeps the lines that match (or, with `keep` false, the ones that don't).
pub fn filter_lines(text: &str, matcher: &LineMatcher, keep: bool) -> String {
    map_lines(text, |line| {
        (matcher.is_match(line) == keep).then(|| line.to_owned())
    })
}

/// How many lines of `text` match, out of how many.
pub fn count_matching(text: &str, matcher: &LineMatcher) -> (usize, usize) {
    let body = text.strip_suffix('\n').unwrap_or(text);
    if body.is_empty() && text.is_empty() {
        return (0, 0);
    }
    let mut total = 0;
    let mut hits = 0;
    for line in body.split('\n') {
        total += 1;
        hits += matcher.is_match(line) as usize;
    }
    (hits, total)
}

/// Every distinct line once, the most frequent first (ties in the order
/// they first appear), as `count<Tab>line`.
pub fn count_lines(text: &str) -> String {
    let body = text.strip_suffix('\n').unwrap_or(text);
    let mut order: Vec<(&str, usize)> = Vec::new();
    let mut index: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for line in body.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        match index.get(line) {
            Some(&i) => order[i].1 += 1,
            None => {
                index.insert(line, order.len());
                order.push((line, 1));
            }
        }
    }
    order.sort_by_key(|o| std::cmp::Reverse(o.1));
    let width = order.first().map_or(1, |(_, n)| n.to_string().len());
    let mut out = String::new();
    for (line, n) in order {
        out.push_str(&format!("{n:>width$}\t{line}\n"));
    }
    out
}

pub fn affix_lines(text: &str, prefix: &str, suffix: &str, skip_empty: bool) -> String {
    map_lines(text, |line| {
        Some(if skip_empty && line.trim().is_empty() {
            line.to_owned()
        } else {
            format!("{prefix}{line}{suffix}")
        })
    })
}

pub fn number_lines(
    text: &str,
    start: i64,
    step: i64,
    separator: &str,
    pad: NumberPad,
    skip_empty: bool,
) -> String {
    let body = text.strip_suffix('\n').unwrap_or(text);
    let numbered = body
        .split('\n')
        .filter(|l| !(skip_empty && l.trim().is_empty()))
        .count() as i64;
    let last = start.saturating_add(step.saturating_mul((numbered - 1).max(0)));
    let width = start.to_string().len().max(last.to_string().len());
    let mut n = start;
    map_lines(text, |line| {
        if skip_empty && line.trim().is_empty() {
            return Some(line.to_owned());
        }
        let number = match pad {
            NumberPad::None => n.to_string(),
            NumberPad::Spaces => format!("{n:>width$}"),
            NumberPad::Zeros if n < 0 => format!("-{:0>w$}", n.unsigned_abs(), w = width - 1),
            NumberPad::Zeros => format!("{n:0>width$}"),
        };
        n = n.saturating_add(step);
        Some(format!("{number}{separator}{line}"))
    })
}

/// The start of a line that every line of its paragraph repeats: indentation
/// and a comment or quote marker, like `    // ` or `> `.
fn line_prefix(line: &str) -> &str {
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    for marker in ["///", "//!", "//", "#", ">", "--", ";", "*"] {
        if let Some(after) = rest.strip_prefix(marker) {
            // `*` only as a block-comment margin, not as a list bullet.
            if marker == "*" && !(after.is_empty() || after.starts_with(' ')) {
                break;
            }
            if marker == "*" && after.starts_with(' ') && indent == 0 {
                break;
            }
            let spaces = after.len() - after.trim_start().len();
            return &line[..indent + marker.len() + spaces];
        }
    }
    &line[..indent]
}

/// The bullet at the start of `text` (`- `, `* `, `1. `), with its spaces.
fn bullet(text: &str) -> Option<&str> {
    let b = text.as_bytes();
    let marker = if matches!(b.first(), Some(b'-' | b'*' | b'+')) {
        1
    } else {
        let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 || digits > 9 || !matches!(b.get(digits), Some(b'.' | b')')) {
            return None;
        }
        digits + 1
    };
    let spaces = text[marker..].len() - text[marker..].trim_start_matches(' ').len();
    (spaces > 0).then(|| &text[..marker + spaces])
}

/// Rewraps paragraphs (runs of lines with the same prefix, ended by a blank
/// line or a list item) so lines are at most `width` chars where words allow.
/// Indentation, comment markers and list bullets are kept.
pub fn reflow(text: &str, width: usize) -> String {
    let width = width.max(1);
    let (body, trailing) = match text.strip_suffix('\n') {
        Some(body) => (body, "\n"),
        None => (text, ""),
    };
    let mut out: Vec<String> = Vec::new();
    // The paragraph being collected: first-line prefix, later-line prefix, words.
    let mut para: Option<(String, String, Vec<String>)> = None;
    let flush = |para: &mut Option<(String, String, Vec<String>)>, out: &mut Vec<String>| {
        if let Some((first, rest, words)) = para.take() {
            let mut line = first;
            let mut has_word = false;
            for w in words {
                let len = line.chars().count();
                if has_word && len + 1 + w.chars().count() > width {
                    out.push(std::mem::replace(&mut line, rest.clone()));
                    has_word = false;
                }
                if has_word {
                    line.push(' ');
                }
                line.push_str(&w);
                has_word = true;
            }
            out.push(line);
        }
    };
    for line in body.split('\n') {
        let prefix = line_prefix(line);
        let content = &line[prefix.len()..];
        if content.trim().is_empty() {
            flush(&mut para, &mut out);
            out.push(line.trim_end().to_owned());
            continue;
        }
        let item = bullet(content);
        let continues = item.is_none()
            && para
                .as_ref()
                .is_some_and(|(_, rest, _)| rest.trim_end() == prefix.trim_end());
        if !continues {
            flush(&mut para, &mut out);
            let (first, hang) = match item {
                Some(b) => (
                    format!("{prefix}{b}"),
                    format!("{prefix}{}", " ".repeat(b.chars().count())),
                ),
                None => (prefix.to_owned(), prefix.to_owned()),
            };
            para = Some((first, hang, Vec::new()));
        }
        let words = &mut para.as_mut().expect("paragraph").2;
        let content = item.map_or(content, |b| &content[b.len()..]);
        words.extend(content.split_whitespace().map(str::to_owned));
    }
    flush(&mut para, &mut out);
    out.join("\n") + trailing
}

/// The hash of `bytes` in lower-case hex, as `sha256sum` and friends print it.
pub fn hash(kind: HashKind, bytes: &[u8]) -> String {
    use md5::Digest;
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
    match kind {
        HashKind::Md5 => hex(&md5::Md5::digest(bytes)),
        HashKind::Sha1 => hex(&sha1::Sha1::digest(bytes)),
        HashKind::Sha256 => hex(&sha2::Sha256::digest(bytes)),
        HashKind::Sha512 => hex(&sha2::Sha512::digest(bytes)),
        HashKind::Crc32 => format!("{:08x}", crc32fast::hash(bytes)),
    }
}

/// The UTF-8 bytes of `text` as hex pairs: `48 65 6C 6C 6F`.
pub fn hex_encode(text: &str) -> String {
    let pairs: Vec<String> = text.bytes().map(|b| format!("{b:02X}")).collect();
    pairs.join(" ")
}

/// Hex digits back to text. Spaces, line breaks, commas, colons, `0x` and
/// `\x` between bytes are ignored.
pub fn hex_decode(text: &str) -> ToolResult {
    let mut bytes = Vec::new();
    let mut high: Option<(u8, usize)> = None;
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        // `0x` or `\x` in front of a byte.
        if high.is_none()
            && (c == b'0' || c == b'\\')
            && matches!(b.get(i + 1), Some(b'x' | b'X'))
            && b.get(i + 2).is_some_and(|d| d.is_ascii_hexdigit())
        {
            i += 2;
            continue;
        }
        if let Some(d) = (c as char).to_digit(16) {
            match high.take() {
                Some((h, _)) => bytes.push(h << 4 | d as u8),
                None => high = Some((d as u8, i)),
            }
        } else if c.is_ascii_whitespace() || matches!(c, b',' | b':' | b'-' | b';') {
            if let Some((_, at)) = high {
                return Err(ToolError::at(
                    "A byte needs two hex digits; this one has one.",
                    at,
                ));
            }
        } else {
            return Err(ToolError::at("This isn't a hex digit.", i));
        }
        i += 1;
    }
    if let Some((_, at)) = high {
        return Err(ToolError::at(
            "A byte needs two hex digits; this one has one.",
            at,
        ));
    }
    String::from_utf8(bytes).map_err(|e| {
        ToolError::new(format!(
            "Byte {} isn't UTF-8 text; use View > Hex View to see the bytes.",
            e.utf8_error().valid_up_to() + 1
        ))
    })
}

/// A decoded JSON Web Token.
pub struct Jwt {
    /// `{"header": ..., "payload": ...}`, formatted.
    pub json: String,
    /// When it was issued and expires (`iat`, `exp`), in local time.
    pub issued: Option<String>,
    pub expires: Option<String>,
    pub expired: bool,
}

/// Decodes the header and payload of a JWT. The signature is not checked.
pub fn jwt_decode(token: &str, indent: &str) -> Result<Jwt, ToolError> {
    let token = token.trim();
    let token = token
        .strip_prefix("Bearer ")
        .or_else(|| token.strip_prefix("bearer "))
        .unwrap_or(token)
        .trim();
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 && parts.len() != 5 {
        return Err(ToolError::new(
            "This isn't a JSON Web Token (three Base64 parts joined by dots).",
        ));
    }
    if parts.len() == 5 {
        return Err(ToolError::new(
            "This is an encrypted token (JWE); only signed tokens can be read without the key.",
        ));
    }
    let part = |i: usize| -> Result<serde_json::Value, ToolError> {
        let bytes = super::mime::MimeOp::Base64Decode
            .run(parts[i])
            .map_err(|_| {
                ToolError::new("This isn't a JSON Web Token (three Base64 parts joined by dots).")
            })?;
        serde_json::from_str(&bytes).map_err(|_| {
            ToolError::new("This isn't a JSON Web Token (three Base64 parts joined by dots).")
        })
    };
    let header = part(0)?;
    let payload = part(1)?;
    let time = |key: &str| {
        let secs = payload.get(key)?.as_i64()?;
        let at = chrono::DateTime::from_timestamp(secs, 0)?;
        Some((
            at,
            at.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        ))
    };
    let issued = time("iat").map(|t| t.1);
    let exp = time("exp");
    let expired = exp.as_ref().is_some_and(|(at, _)| *at < chrono::Utc::now());
    let value = serde_json::json!({ "header": header, "payload": payload });
    Ok(Jwt {
        json: super::convert::write_json(&value, indent),
        issued,
        expires: exp.map(|t| t.1),
        expired,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_lines() {
        let m = LineMatcher::new("err", false, false).unwrap();
        assert_eq!(
            filter_lines("ok\nERROR 1\nfine\nerr 2\n", &m, true),
            "ERROR 1\nerr 2\n"
        );
        assert_eq!(filter_lines("ok\nERROR 1\nfine\n", &m, false), "ok\nfine\n");
        assert_eq!(filter_lines("ERROR\n", &m, false), "");
        let m = LineMatcher::new(r"^\d+$", true, true).unwrap();
        assert_eq!(filter_lines("12\na1\n7", &m, true), "12\n7");
        assert_eq!(count_matching("12\na1\n7\n", &m), (2, 3));
        assert!(LineMatcher::new("(", true, true).is_err());
        let m = LineMatcher::new("Err", false, true).unwrap();
        assert_eq!(filter_lines("err\nErr", &m, true), "Err");
    }

    #[test]
    fn counts_lines() {
        assert_eq!(count_lines("b\na\nb\nc\nb\na\n"), "3\tb\n2\ta\n1\tc\n");
        let many = "x\n".repeat(12) + "y\n";
        assert_eq!(count_lines(&many), "12\tx\n 1\ty\n");
    }

    #[test]
    fn affixes_and_numbers_lines() {
        assert_eq!(affix_lines("a\n\nb\n", "'", "',", true), "'a',\n\n'b',\n");
        assert_eq!(affix_lines("a\n\nb", "- ", "", false), "- a\n- \n- b");
        let ten = "x\n".repeat(10);
        let out = number_lines(&ten, 1, 1, ". ", NumberPad::Spaces, false);
        assert!(out.starts_with(" 1. x\n 2. x\n"));
        assert!(out.ends_with("10. x\n"));
        assert_eq!(
            number_lines("a\n\nb", 5, 5, " ", NumberPad::Zeros, true),
            "05 a\n\n10 b"
        );
        assert_eq!(
            number_lines("a\nb", 0, 1, ":", NumberPad::None, false),
            "0:a\n1:b"
        );
    }

    #[test]
    fn reflows_paragraphs() {
        assert_eq!(
            reflow("one two three four five six\n\nseven eight\n", 10),
            "one two\nthree four\nfive six\n\nseven\neight\n"
        );
        // Short lines are joined.
        assert_eq!(reflow("a\nb\nc", 20), "a b c");
        // Comment markers and indentation stay.
        assert_eq!(
            reflow("    // alpha beta gamma\n    // delta", 18),
            "    // alpha beta\n    // gamma delta"
        );
        // List items hang under their bullet and don't merge.
        assert_eq!(
            reflow("- one two three\n- four\n1. five six", 9),
            "- one two\n  three\n- four\n1. five\n   six"
        );
        assert_eq!(reflow("> a b c d", 5), "> a b\n> c d");
    }

    #[test]
    fn hashes() {
        assert_eq!(
            hash(HashKind::Md5, b"abc"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hash(HashKind::Sha1, b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hash(HashKind::Sha256, b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(hash(HashKind::Sha512, b"abc").starts_with("ddaf35a193617aba"));
        assert_eq!(hash(HashKind::Crc32, b"abc"), "352441c2");
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(hex_encode("Hé"), "48 C3 A9");
        assert_eq!(hex_decode("48 C3 A9").unwrap(), "Hé");
        assert_eq!(hex_decode("0x48,0x69").unwrap(), "Hi");
        assert_eq!(hex_decode("\\x48\\x69").unwrap(), "Hi");
        assert_eq!(hex_decode("4869\n").unwrap(), "Hi");
        assert_eq!(hex_decode("48:69").unwrap(), "Hi");
        assert!(hex_decode("486").is_err());
        assert!(hex_decode("4g").is_err());
        assert!(hex_decode("ff").is_err());
    }

    #[test]
    fn decodes_jwt() {
        // {"alg":"HS256","typ":"JWT"} . {"sub":"1234567890","name":"John Doe","iat":1516239022}
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let jwt = jwt_decode(&format!("Bearer {token}\n"), "  ").unwrap();
        assert!(jwt.json.contains("\"alg\": \"HS256\""));
        assert!(jwt.json.contains("\"name\": \"John Doe\""));
        assert!(jwt.json.find("header").unwrap() < jwt.json.find("payload").unwrap());
        assert!(jwt.issued.is_some());
        assert!(jwt.expires.is_none());
        assert!(jwt_decode("not a token", "  ").is_err());
    }
}
