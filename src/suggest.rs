//! Suggestions for the file you just opened or pasted: "This JSON is on one
//! line. Format?" The detectors here are pure; the bar that shows them is in
//! `app/suggest.rs`.

use serde::{Deserialize, Serialize};

use crate::tools::mime::base64_decode;

/// Detectors look at no more than this many bytes of the text.
pub const SCAN_BYTES: usize = 64 * 1024;
/// A paste smaller than this doesn't start the detectors.
pub const PASTE_BYTES: usize = 1024;
/// A minified file has a line at least this long.
const LONG_LINE: usize = 500;

/// What a suggestion is about. The names are stored in settings when a kind
/// is dismissed for good, so they must not change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    MinifiedJson,
    MinifiedXml,
    Jwt,
    Base64,
    Csv,
    MixedLineEndings,
    MixedIndentation,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::MinifiedJson,
        Kind::MinifiedXml,
        Kind::Jwt,
        Kind::Base64,
        Kind::Csv,
        Kind::MixedLineEndings,
        Kind::MixedIndentation,
    ];

    /// The name stored in settings.
    pub fn key(self) -> &'static str {
        match self {
            Kind::MinifiedJson => "minified-json",
            Kind::MinifiedXml => "minified-xml",
            Kind::Jwt => "jwt",
            Kind::Base64 => "base64",
            Kind::Csv => "csv",
            Kind::MixedLineEndings => "mixed-line-endings",
            Kind::MixedIndentation => "mixed-indentation",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// What the detectors are told about the text besides the text itself.
#[derive(Default)]
pub struct Facts<'a> {
    /// The tab's language, such as "JSON" or "Plain Text".
    pub language: &'a str,
    /// The file had more than one kind of line break when it was read.
    pub mixed_line_endings: bool,
    /// The text's full length in bytes, when only part of it is passed.
    pub full_len: usize,
}

/// The first `SCAN_BYTES` of `text`, cut at a character boundary.
pub fn scan_part(text: &str) -> &str {
    if text.len() <= SCAN_BYTES {
        return text;
    }
    let mut end = SCAN_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// The most useful suggestion for `text` (already cut with [`scan_part`]),
/// skipping the kinds in `dismissed`.
pub fn detect(text: &str, facts: &Facts, dismissed: &[Kind]) -> Option<Kind> {
    let full = facts.full_len.max(text.len());
    let checks: [(Kind, &dyn Fn() -> bool); 7] = [
        (Kind::Jwt, &|| is_jwt(text)),
        (Kind::Base64, &|| is_base64_blob(text, full)),
        (Kind::MinifiedJson, &|| is_minified_json(text, full)),
        (Kind::MinifiedXml, &|| is_minified_xml(text)),
        (Kind::Csv, &|| is_csv(text, facts.language)),
        (Kind::MixedLineEndings, &|| facts.mixed_line_endings),
        (Kind::MixedIndentation, &|| has_mixed_indentation(text)),
    ];
    checks
        .into_iter()
        .filter(|(k, _)| !dismissed.contains(k))
        .find(|(_, check)| check())
        .map(|(k, _)| k)
}

/// The longest line's length, and how much of the text it is (0 to 1).
fn longest_line(text: &str) -> (usize, f32) {
    let longest = text.split('\n').map(str::len).max().unwrap_or(0);
    let share = if text.is_empty() {
        0.0
    } else {
        longest as f32 / text.trim_end().len().max(1) as f32
    };
    (longest, share)
}

/// JSON with almost everything on one long line. When only the start of a
/// big file is passed, it must be valid as far as it goes.
pub fn is_minified_json(text: &str, full_len: usize) -> bool {
    let trimmed = text.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return false;
    }
    let (longest, share) = longest_line(text);
    if longest < LONG_LINE || share < 0.9 {
        return false;
    }
    match serde_json::from_str::<serde::de::IgnoredAny>(trimmed) {
        Ok(_) => true,
        // Cut off part way through a bigger file: fine if valid so far.
        Err(e) => e.is_eof() && full_len > text.len(),
    }
}

/// XML with almost everything on one long line.
pub fn is_minified_xml(text: &str) -> bool {
    let trimmed = text.trim_start_matches('\u{feff}').trim();
    if !trimmed.starts_with('<') {
        return false;
    }
    let (longest, share) = longest_line(text);
    longest >= LONG_LINE
        && share >= 0.9
        && trimmed.matches("><").count() >= 10
        && trimmed.contains("</")
}

/// The decoded bytes of a Base64 URL part, if it is one.
fn b64url(part: &str) -> Option<Vec<u8>> {
    if part.is_empty()
        || !part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    base64_decode(part).ok()
}

/// A JSON Web Token: three Base64 URL parts, the first a JSON header with
/// an `alg`.
pub fn is_jwt(text: &str) -> bool {
    let token = text.trim();
    let token = token
        .strip_prefix("Bearer ")
        .or_else(|| token.strip_prefix("bearer "))
        .unwrap_or(token);
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 || b64url(parts[1]).is_none() {
        return false;
    }
    // The signature is empty for unsigned tokens.
    if !parts[2].is_empty() && b64url(parts[2]).is_none() {
        return false;
    }
    let Some(header) = b64url(parts[0]) else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&header)
        .ok()
        .and_then(|v| v.as_object().map(|o| o.contains_key("alg")))
        .unwrap_or(false)
}

/// A block of Base64 that decodes to mostly readable text: the whole text,
/// with line breaks allowed.
pub fn is_base64_blob(text: &str, full_len: usize) -> bool {
    let compact: String = text.split_whitespace().collect();
    // Short words are often valid Base64 by chance.
    if compact.len() < 24 || full_len > text.len() {
        return false;
    }
    if !compact.len().is_multiple_of(4)
        || !compact
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
    {
        return false;
    }
    // Plain words and numbers are valid Base64 too; real Base64 mixes cases
    // and digits.
    let has = |f: fn(&u8) -> bool| compact.bytes().any(|b| f(&b));
    if !(has(u8::is_ascii_lowercase) && has(u8::is_ascii_uppercase) && has(u8::is_ascii_digit)) {
        return false;
    }
    let Ok(bytes) = base64_decode(&compact) else {
        return false;
    };
    let Ok(decoded) = String::from_utf8(bytes) else {
        return false;
    };
    let total = decoded.chars().count();
    let printable = decoded
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .count();
    total > 0 && printable * 10 >= total * 8
}

/// Comma, tab or semicolon separated values that aren't already a CSV tab.
/// Markdown tables use `|` and are left alone.
pub fn is_csv(text: &str, language: &str) -> bool {
    if language == "CSV" || language == "TSV" {
        // A .csv file: CsvQuery is the thing to suggest.
        return text.lines().filter(|l| !l.trim().is_empty()).count() >= 2;
    }
    if language != "Plain Text" && !language.is_empty() {
        return false;
    }
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 3 || lines.iter().any(|l| l.trim_start().starts_with('|')) {
        return false;
    }
    // looks_like_csv also accepts `|`; ask it about the other separators only.
    let without_pipes = !text.contains('|');
    without_pipes && crate::tools::convert::looks_like_csv(text)
}

/// Some lines are indented with tabs and others with spaces.
pub fn has_mixed_indentation(text: &str) -> bool {
    let mut tabs = 0;
    let mut spaces = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('\t') {
            tabs += 1;
        } else if line.starts_with("  ") {
            spaces += 1;
        }
    }
    tabs >= 2 && spaces >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts<'static> {
        Facts {
            language: "Plain Text",
            ..Default::default()
        }
    }

    fn minified_json(size: usize) -> String {
        let mut s = String::from("{\"data\":[");
        let mut i = 0;
        while s.len() < size {
            if i > 0 {
                s.push(',');
            }
            s += &format!(
                "{{\"id\":{i},\"attributes\":{{\"name\":\"Ada\",\"roles\":[\"admin\",\"dev\"]}}}}"
            );
            i += 1;
        }
        s + "]}"
    }

    #[test]
    fn minified_json_is_found() {
        let json = minified_json(48 * 1024);
        assert_eq!(detect(&json, &facts(), &[]), Some(Kind::MinifiedJson));
        // Only the start of a big file is looked at.
        let big = minified_json(200 * 1024);
        let part = scan_part(&big);
        let f = Facts {
            full_len: big.len(),
            ..facts()
        };
        assert_eq!(detect(part, &f, &[]), Some(Kind::MinifiedJson));
        // Formatted JSON isn't minified.
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let pretty = serde_json::to_string_pretty(&value).unwrap();
        assert_eq!(detect(&pretty, &facts(), &[]), None);
        // Dismissed kinds are skipped.
        assert_eq!(detect(&json, &facts(), &[Kind::MinifiedJson]), None);
    }

    #[test]
    fn broken_json_is_not_minified_json() {
        let json = minified_json(4000);
        let broken = json.replacen(':', "", 3);
        assert!(!is_minified_json(&broken, broken.len()));
    }

    #[test]
    fn minified_xml_is_found() {
        let mut xml = String::from("<?xml version=\"1.0\"?><root>");
        for i in 0..40 {
            xml += &format!("<item id=\"{i}\"><name>Thing {i}</name></item>");
        }
        xml += "</root>";
        assert_eq!(detect(&xml, &facts(), &[]), Some(Kind::MinifiedXml));
        let pretty = xml.replace("><", ">\n<");
        assert_eq!(detect(&pretty, &facts(), &[]), None);
    }

    #[test]
    fn jwt_is_found() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
                     eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.\
                     SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert_eq!(detect(token, &facts(), &[]), Some(Kind::Jwt));
        assert!(is_jwt(&format!("Bearer {token}\n")));
        // Three dotted words aren't a token.
        assert!(!is_jwt("www.example.com"));
        // A header without "alg" isn't one either ({"typ":"JWT"}).
        assert!(!is_jwt("eyJ0eXAiOiJKV1QifQ.e30.abc"));
    }

    #[test]
    fn base64_blob_is_found() {
        let text = "Ettin Edit turns text into other text, quickly and safely.\n";
        let b64 = crate::tools::mime::base64_encode(
            text.as_bytes(),
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
            true,
            Some(76),
        );
        assert_eq!(detect(&b64, &facts(), &[]), Some(Kind::Base64));
        // Base64 of binary data isn't suggested: decoding gives no text.
        let binary: Vec<u8> = (0u8..=255).collect();
        let b64 = crate::tools::mime::base64_encode(
            &binary,
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
            true,
            None,
        );
        assert!(!is_base64_blob(&b64, b64.len()));
    }

    #[test]
    fn prose_suggests_nothing() {
        let prose = "Ettin Edit is a text editor for Linux and Windows. It is \
                     built for turning text into other text: filtering log files, \
                     converting JSON to YAML, decoding tokens and the like.\n\n\
                     It is not an IDE, and it doesn't try to be one.\n";
        assert_eq!(detect(prose, &facts(), &[]), None);
        // A short word that is valid Base64.
        assert_eq!(detect("test", &facts(), &[]), None);
        assert_eq!(detect("Supercalifragilisticexpi", &facts(), &[]), None);
        assert_eq!(detect("abcdefghijklmnopqrstuvwx", &facts(), &[]), None);
    }

    #[test]
    fn markdown_table_is_not_csv() {
        let table = "| Name | Role |\n|------|------|\n| Ada | admin |\n| Bob | dev |\n";
        assert_eq!(detect(table, &facts(), &[]), None);
        let md = Facts {
            language: "Markdown",
            ..facts()
        };
        assert_eq!(detect("a,b\n1,2\n3,4\n", &md, &[]), None);
    }

    #[test]
    fn csv_is_found() {
        let csv = "name,role,id\nAda,admin,1\nBob,dev,2\nCy,ops,3\n";
        assert_eq!(detect(csv, &facts(), &[]), Some(Kind::Csv));
        let f = Facts {
            language: "CSV",
            ..facts()
        };
        assert_eq!(detect(csv, &f, &[]), Some(Kind::Csv));
    }

    #[test]
    fn mixed_line_endings_and_indentation() {
        let f = Facts {
            mixed_line_endings: true,
            ..facts()
        };
        assert_eq!(detect("one\ntwo\n", &f, &[]), Some(Kind::MixedLineEndings));
        let code = "fn a() {\n\tone();\n\ttwo();\n}\nfn b() {\n    three();\n    four();\n}\n";
        assert_eq!(detect(code, &facts(), &[]), Some(Kind::MixedIndentation));
        let tabs = "fn a() {\n\tone();\n\ttwo();\n}\n";
        assert_eq!(detect(tabs, &facts(), &[]), None);
    }

    #[test]
    fn kinds_round_trip_through_their_keys() {
        for k in Kind::ALL {
            assert_eq!(Kind::from_key(k.key()), Some(k));
            let json = serde_json::to_string(&k).unwrap();
            assert_eq!(json, format!("\"{}\"", k.key()));
        }
    }
}
