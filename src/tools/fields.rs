//! Columns and fields (Ettin Edit 2.6): pulling regex matches out of text,
//! lining up columns, picking and sorting delimited fields, splitting and
//! joining lines, escaping strings for a language, and converting Unix
//! times. Everything here works on plain strings.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::{ToolError, ToolResult};

/// A delimiter as typed in a dialog or a recipe: `\t` is a tab, and an
/// empty delimiter means "runs of spaces and tabs".
pub fn delimiter(typed: &str) -> String {
    typed.replace("\\t", "\t")
}

/// Splits `body` into lines and puts the final line break back after `f`.
fn lines_of(text: &str) -> (Vec<&str>, bool) {
    if text.is_empty() {
        return (Vec::new(), false);
    }
    match text.strip_suffix('\n') {
        Some(body) => (body.split('\n').collect(), true),
        None => (text.split('\n').collect(), false),
    }
}

fn join_lines(lines: Vec<String>, trailing: bool) -> String {
    let mut out = lines.join("\n");
    if trailing && !lines.is_empty() {
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------- natural

/// Compares like a person would: runs of digits by their value, so
/// `file2` comes before `file10`; letters ignoring case first.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(&c) = it.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (n, m) = (take(&mut x), take(&mut y));
                let (nt, mt) = (n.trim_start_matches('0'), m.trim_start_matches('0'));
                let ord = nt.len().cmp(&mt.len()).then_with(|| nt.cmp(mt));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(c), Some(d)) => {
                let ord = c
                    .to_lowercase()
                    .cmp(d.to_lowercase())
                    .then_with(|| Ordering::Equal);
                if ord != Ordering::Equal {
                    return ord;
                }
                x.next();
                y.next();
            }
        }
    }
}

// ---------------------------------------------------------------- extract

/// Every match of `pattern` (or of its capture group `group`), one per
/// line. With `unique`, each different match once, in the order found.
pub fn extract_matches(
    text: &str,
    pattern: &str,
    match_case: bool,
    group: usize,
    unique: bool,
) -> ToolResult {
    if pattern.is_empty() {
        return Err(ToolError::new("Type a pattern to look for."));
    }
    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(!match_case)
        .multi_line(true)
        .build()
        .map_err(|e| ToolError::new(e.to_string()))?;
    if group >= re.captures_len() {
        return Err(ToolError::new(format!(
            "The pattern has {} capture group(s); there is no group {group}.",
            re.captures_len() - 1
        )));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = String::new();
    for caps in re.captures_iter(text) {
        let Some(m) = caps.get(group) else { continue };
        if unique && !seen.insert(m.as_str().to_owned()) {
            continue;
        }
        out.push_str(m.as_str());
        out.push('\n');
    }
    Ok(out)
}

// ---------------------------------------------------------------- fields

/// The fields of `line`. With a one-character delimiter, a field that
/// starts with `"` runs to its closing quote (`""` inside is a quote), so
/// CSV with commas in quotes splits right. Quotes are kept.
pub fn split_fields<'a>(line: &'a str, delim: &str) -> Vec<&'a str> {
    if delim.is_empty() {
        return line.split_whitespace().collect();
    }
    let mut chars = delim.chars();
    let single = match (chars.next(), chars.next()) {
        (Some(c), None) if c != '"' => Some(c),
        _ => None,
    };
    let Some(d) = single else {
        return line.split(delim).collect();
    };
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut at_field_start = true;
    for (i, c) in line.char_indices() {
        if at_field_start {
            at_field_start = false;
            if c == '"' {
                in_quotes = true;
                continue;
            }
        }
        if in_quotes {
            if c == '"' {
                // `""` stays in quotes: the next quote turns it back on.
                in_quotes = false;
            }
            continue;
        }
        if c == '"' && line[start..i].starts_with('"') {
            in_quotes = true;
            continue;
        }
        if c == d {
            out.push(&line[start..i]);
            start = i + c.len_utf8();
            at_field_start = true;
        }
    }
    out.push(&line[start..]);
    out
}

/// Which fields: `1,3-5,7-` (counted from 1). `-2` means the last two.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldSpec(Vec<(usize, Option<usize>)>);

impl FieldSpec {
    pub fn parse(spec: &str) -> Result<Self, ToolError> {
        let mut out = Vec::new();
        for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let bad = || {
                ToolError::new(format!(
                    "\"{part}\" isn't a field number or range, like 2 or 3-5."
                ))
            };
            let num = |s: &str| -> Result<usize, ToolError> {
                match s.trim().parse::<usize>() {
                    Ok(n) if n > 0 => Ok(n),
                    _ => Err(bad()),
                }
            };
            match part.split_once('-') {
                None => {
                    let n = num(part)?;
                    out.push((n, Some(n)));
                }
                Some(("", to)) => out.push((1, Some(num(to)?))),
                Some((from, "")) => out.push((num(from)?, None)),
                Some((from, to)) => {
                    let (a, b) = (num(from)?, num(to)?);
                    if b < a {
                        return Err(bad());
                    }
                    out.push((a, Some(b)));
                }
            }
        }
        if out.is_empty() {
            return Err(ToolError::new("Say which fields, like 1,3 or 2-4."));
        }
        Ok(Self(out))
    }

    /// The 0-based field indexes for a line with `count` fields, in the
    /// order the spec names them.
    fn indexes(&self, count: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for &(from, to) in &self.0 {
            let to = to.unwrap_or(count).min(count);
            out.extend((from..=to).map(|n| n - 1));
        }
        out
    }
}

/// Keeps the fields `spec` names, in that order (or with `remove`, all
/// but them), joined by `out_delim` or, when that is empty, the delimiter.
/// Lines without the delimiter are left as they are.
pub fn select_fields(
    text: &str,
    delim: &str,
    spec: &str,
    remove: bool,
    out_delim: &str,
) -> ToolResult {
    let spec = FieldSpec::parse(spec)?;
    let delim = delimiter(delim);
    let out_delim = match (out_delim.is_empty(), delim.is_empty()) {
        (false, _) => delimiter(out_delim),
        (true, true) => " ".to_owned(),
        (true, false) => delim.clone(),
    };
    let (lines, trailing) = lines_of(text);
    let out = lines
        .into_iter()
        .map(|line| {
            let fields = split_fields(line, &delim);
            if fields.len() < 2 && !delim.is_empty() && !line.contains(&delim) {
                return line.to_owned();
            }
            let named = spec.indexes(fields.len());
            let keep: Vec<&str> = if remove {
                (0..fields.len())
                    .filter(|i| !named.contains(i))
                    .map(|i| fields[i])
                    .collect()
            } else {
                named.iter().map(|&i| fields[i]).collect()
            };
            keep.join(&out_delim)
        })
        .collect();
    Ok(join_lines(out, trailing))
}

/// How Sort Lines by Field compares the field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldOrder {
    /// `file2` before `file10`, ignoring case.
    #[default]
    Natural,
    /// In character order.
    Text,
    /// As a number; fields that aren't numbers go first.
    Number,
}

/// Sorts the lines by field `field` (from 1). Lines without that field go
/// first, in the order they were.
pub fn sort_by_field(
    text: &str,
    delim: &str,
    field: usize,
    order: FieldOrder,
    descending: bool,
) -> ToolResult {
    if field == 0 {
        return Err(ToolError::new("Fields are counted from 1."));
    }
    let delim = delimiter(delim);
    let (lines, trailing) = lines_of(text);
    let mut keyed: Vec<(Option<String>, &str)> = lines
        .into_iter()
        .map(|line| {
            let key = split_fields(line, &delim)
                .get(field - 1)
                .map(|f| f.trim().trim_matches('"').to_owned());
            (key, line)
        })
        .collect();
    let cmp = |a: &str, b: &str| match order {
        FieldOrder::Natural => natural_cmp(a, b),
        FieldOrder::Text => a.cmp(b),
        FieldOrder::Number => {
            let n = |s: &str| s.replace(',', "").parse::<f64>().ok();
            match (n(a), n(b)) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
                (None, Some(_)) => Ordering::Less,
                (Some(_), None) => Ordering::Greater,
                (None, None) => a.cmp(b),
            }
        }
    };
    keyed.sort_by(|(a, _), (b, _)| match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) if descending => cmp(b, a),
        (Some(a), Some(b)) => cmp(a, b),
    });
    Ok(join_lines(
        keyed.into_iter().map(|(_, l)| l.to_owned()).collect(),
        trailing,
    ))
}

// ---------------------------------------------------------------- align

/// Lines up the columns of the lines that contain the delimiter. Cells are
/// trimmed and padded to the widest in their column; with `spaced`, the
/// delimiter gets a space on each side. An empty delimiter lines up words
/// split on spaces and tabs, like `column -t`. Indentation is kept.
pub fn align_columns(text: &str, delim: &str, spaced: bool) -> String {
    use unicode_width::UnicodeWidthStr;
    let delim = delimiter(delim);
    let (lines, trailing) = lines_of(text);
    let rows: Vec<Option<(&str, Vec<&str>)>> = lines
        .iter()
        .map(|line| {
            let body = line.trim_start();
            let indent = &line[..line.len() - body.len()];
            if !delim.is_empty() && !body.contains(delim.as_str()) {
                return None;
            }
            let cells: Vec<&str> = split_fields(body, &delim)
                .into_iter()
                .map(str::trim)
                .collect();
            (cells.len() > 1).then_some((indent, cells))
        })
        .collect();
    let mut widths: Vec<usize> = Vec::new();
    for (_, cells) in rows.iter().flatten() {
        for (i, c) in cells.iter().enumerate() {
            if widths.len() <= i {
                widths.push(0);
            }
            widths[i] = widths[i].max(c.width());
        }
    }
    let joiner = if delim.is_empty() {
        " ".to_owned()
    } else if spaced {
        format!(" {delim} ")
    } else {
        delim.clone()
    };
    let out = lines
        .iter()
        .zip(rows)
        .map(|(line, row)| {
            let Some((indent, cells)) = row else {
                return (*line).to_owned();
            };
            let last = cells.len() - 1;
            let mut s = indent.to_owned();
            for (i, c) in cells.iter().enumerate() {
                s.push_str(c);
                if i < last {
                    s.push_str(&" ".repeat(widths[i] - c.width()));
                    s.push_str(&joiner);
                }
            }
            // An empty first or last cell (`| a | b |`) leaves no stray spaces.
            let s = s.trim_end().to_owned();
            if cells[0].is_empty() && spaced && !delim.is_empty() {
                let rest = s[indent.len()..].trim_start();
                format!("{indent}{rest}")
            } else {
                s
            }
        })
        .collect();
    join_lines(out, trailing)
}

// ---------------------------------------------------------------- split / join

/// Breaks each line at every `separator`, so each piece is on a line of
/// its own. With `trim`, spaces around the pieces go and empty pieces are
/// dropped.
pub fn split_at(text: &str, separator: &str, trim: bool) -> ToolResult {
    let sep = delimiter(separator);
    if sep.is_empty() {
        return Err(ToolError::new("Type the text to split at."));
    }
    let (lines, trailing) = lines_of(text);
    let mut out = Vec::new();
    for line in lines {
        for piece in line.split(sep.as_str()) {
            if trim {
                let p = piece.trim();
                if !p.is_empty() {
                    out.push(p.to_owned());
                }
            } else {
                out.push(piece.to_owned());
            }
        }
    }
    Ok(join_lines(out, trailing))
}

/// Joins the lines into one, each wrapped in `quote` and separated by
/// `separator`: `'a', 'b', 'c'`. A quote inside a line is doubled, as SQL
/// and CSV expect. Empty lines are skipped when `skip_empty` is set.
pub fn join_with(text: &str, separator: &str, quote: &str, skip_empty: bool) -> String {
    let sep = delimiter(separator);
    let (lines, trailing) = lines_of(text);
    let parts: Vec<String> = lines
        .into_iter()
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .filter(|l| !(skip_empty && l.trim().is_empty()))
        .map(|l| {
            if quote.is_empty() {
                l.to_owned()
            } else {
                let doubled = format!("{quote}{quote}");
                format!("{quote}{}{quote}", l.replace(quote, &doubled))
            }
        })
        .collect();
    let mut out = parts.join(&sep);
    if trailing {
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------- escape

/// The languages Escape String and Unescape String know.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StringKind {
    /// JSON and JavaScript strings: `\"`, `\\`, `\n`, `\u00e9`…
    Json,
    /// C, C++, Java and C# strings: as JSON, plus `\'`, `\0`, `\xHH`.
    C,
    /// SQL: `'` doubled.
    Sql,
    /// Regular expressions: every special character gets a backslash.
    Regex,
    /// POSIX shells: the whole text in single quotes.
    Shell,
    /// CSV: in double quotes, `"` doubled, when it needs quoting.
    Csv,
}

impl StringKind {
    pub const ALL: [StringKind; 6] = [
        StringKind::Json,
        StringKind::C,
        StringKind::Sql,
        StringKind::Regex,
        StringKind::Shell,
        StringKind::Csv,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StringKind::Json => "JSON / JavaScript",
            StringKind::C => "C / Java / C#",
            StringKind::Sql => "SQL",
            StringKind::Regex => "Regex",
            StringKind::Shell => "Shell",
            StringKind::Csv => "CSV",
        }
    }
}

/// Escapes `text` so it can go inside a string of that language. JSON, C
/// and SQL leave the outer quotes to you; Shell and CSV add them.
pub fn escape(text: &str, kind: StringKind) -> String {
    match kind {
        StringKind::Json | StringKind::C => {
            let c = kind == StringKind::C;
            let mut out = String::with_capacity(text.len() + 8);
            for ch in text.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    '\'' if c => out.push_str("\\'"),
                    '\0' if c => out.push_str("\\0"),
                    '\u{8}' => out.push_str("\\b"),
                    '\u{c}' => out.push_str("\\f"),
                    ch if (ch as u32) < 0x20 || ch == '\u{7f}' => {
                        if c {
                            out.push_str(&format!("\\x{:02x}", ch as u32));
                        } else {
                            out.push_str(&format!("\\u{:04x}", ch as u32));
                        }
                    }
                    ch => out.push(ch),
                }
            }
            out
        }
        StringKind::Sql => text.replace('\'', "''"),
        StringKind::Regex => regex::escape(text),
        StringKind::Shell => format!("'{}'", text.replace('\'', r"'\''")),
        StringKind::Csv => {
            if text.contains(['"', ',', '\n', '\r']) || text.trim() != text {
                format!("\"{}\"", text.replace('"', "\"\""))
            } else {
                text.to_owned()
            }
        }
    }
}

/// Turns escaped text back into what it stands for. Quotes around the
/// whole text are taken off when the language uses them.
pub fn unescape(text: &str, kind: StringKind) -> ToolResult {
    match kind {
        StringKind::Json | StringKind::C => unescape_c(strip_quotes(text, &['"', '\'']), kind),
        StringKind::Sql => Ok(strip_quotes(text, &['\'']).replace("''", "'")),
        StringKind::Regex => {
            let mut out = String::new();
            let mut chars = text.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some(n) if !n.is_alphanumeric() => out.push(n),
                        Some(n) => {
                            out.push('\\');
                            out.push(n);
                        }
                        None => out.push('\\'),
                    }
                } else {
                    out.push(c);
                }
            }
            Ok(out)
        }
        StringKind::Shell => unescape_shell(text),
        StringKind::Csv => {
            let t = text.trim();
            match t.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
                Some(inner) => Ok(inner.replace("\"\"", "\"")),
                None => Ok(text.to_owned()),
            }
        }
    }
}

fn strip_quotes<'a>(text: &'a str, quotes: &[char]) -> &'a str {
    for &q in quotes {
        if text.len() >= 2 && text.starts_with(q) && text.ends_with(q) {
            return &text[1..text.len() - 1];
        }
    }
    text
}

fn unescape_c(text: &str, kind: StringKind) -> ToolResult {
    let mut out = String::with_capacity(text.len());
    let mut it = text.char_indices().peekable();
    let hex = |s: &str, at: usize| {
        u32::from_str_radix(s, 16)
            .ok()
            .ok_or_else(|| ToolError::at(format!("\"{s}\" isn't hex."), at))
    };
    while let Some((i, c)) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some((_, e)) = it.next() else {
            return Err(ToolError::at("A backslash at the end has nothing to escape.", i));
        };
        match e {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            '0' if kind == StringKind::C => out.push('\0'),
            '"' | '\\' | '/' | '\'' => out.push(e),
            'x' if kind == StringKind::C => {
                let digits: String = std::iter::from_fn(|| {
                    it.next_if(|(_, c)| c.is_ascii_hexdigit()).map(|(_, c)| c)
                })
                .take(2)
                .collect();
                let n = hex(&digits, i)?;
                out.push(char::from_u32(n).unwrap_or('\u{fffd}'));
            }
            'u' => {
                let digits: String = (0..4).filter_map(|_| it.next().map(|(_, c)| c)).collect();
                let mut n = hex(&digits, i)?;
                // A surrogate pair: \ud83d\ude00.
                if (0xd800..0xdc00).contains(&n) {
                    let rest: String = text[i + 6..].chars().take(6).collect();
                    if let Some(low) = rest.strip_prefix("\\u")
                        && let Ok(lo) = u32::from_str_radix(low, 16)
                        && (0xdc00..0xe000).contains(&lo)
                    {
                        n = 0x10000 + ((n - 0xd800) << 10) + (lo - 0xdc00);
                        for _ in 0..6 {
                            it.next();
                        }
                    }
                }
                out.push(char::from_u32(n).unwrap_or('\u{fffd}'));
            }
            other => {
                return Err(ToolError::at(
                    format!("\"\\{other}\" isn't an escape this language knows."),
                    i,
                ));
            }
        }
    }
    Ok(out)
}

/// Reads shell quoting: `'…'`, `"…"` with `\` before `"`, `\`, `$` and
/// `` ` ``, and `\` outside quotes.
fn unescape_shell(text: &str) -> ToolResult {
    let mut out = String::new();
    let mut it = text.trim().char_indices();
    while let Some((i, c)) = it.next() {
        match c {
            '\'' => loop {
                match it.next() {
                    Some((_, '\'')) => break,
                    Some((_, c)) => out.push(c),
                    None => return Err(ToolError::at("This single quote isn't closed.", i)),
                }
            },
            '"' => loop {
                match it.next() {
                    Some((_, '"')) => break,
                    Some((_, '\\')) => match it.next() {
                        Some((_, c @ ('"' | '\\' | '$' | '`'))) => out.push(c),
                        Some((_, '\n')) => {}
                        Some((_, c)) => {
                            out.push('\\');
                            out.push(c);
                        }
                        None => return Err(ToolError::at("This double quote isn't closed.", i)),
                    },
                    Some((_, c)) => out.push(c),
                    None => return Err(ToolError::at("This double quote isn't closed.", i)),
                }
            },
            '\\' => {
                if let Some((_, c)) = it.next() {
                    out.push(c);
                }
            }
            c => out.push(c),
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- times

/// Replaces Unix times in the text (10-digit seconds or 13-digit
/// milliseconds, on their own) with ISO 8601 dates, in UTC or local time.
pub fn epoch_to_iso(text: &str, utc: bool) -> String {
    let re = regex::Regex::new(r"\b(\d{13}|\d{10})\b").expect("valid");
    re.replace_all(text, |caps: &regex::Captures| {
        let s = &caps[1];
        let n: i64 = s.parse().unwrap_or(0);
        let (secs, millis) = if s.len() == 13 {
            (n.div_euclid(1000), Some(n.rem_euclid(1000)))
        } else {
            (n, None)
        };
        let Some(at) = chrono::DateTime::from_timestamp(secs, 0) else {
            return s.to_owned();
        };
        let at = match millis {
            Some(ms) => at + chrono::Duration::milliseconds(ms),
            None => at,
        };
        let fmt = if millis.is_some() {
            "%Y-%m-%dT%H:%M:%S%.3f"
        } else {
            "%Y-%m-%dT%H:%M:%S"
        };
        if utc {
            format!("{}Z", at.format(fmt))
        } else {
            let local = at.with_timezone(&chrono::Local);
            format!("{}{}", local.format(fmt), local.format("%:z"))
        }
    })
    .into_owned()
}

/// Replaces ISO 8601 date-times in the text (`2026-11-12T22:10:02Z`,
/// with or without fractions, `Z` or an offset; a space for the `T` is
/// fine; no zone means UTC) with Unix times, in seconds or milliseconds.
pub fn iso_to_epoch(text: &str, millis: bool) -> String {
    let re = regex::Regex::new(
        r"\b(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}(?::\d{2}(?:[.,]\d+)?)?)(Z|[+-]\d{2}:?\d{2})?",
    )
    .expect("valid");
    re.replace_all(text, |caps: &regex::Captures| {
        let whole = caps[0].to_owned();
        let mut time = caps[2].replace(',', ".");
        if time.len() == 5 {
            time.push_str(":00");
        }
        let zone = match caps.get(3).map(|m| m.as_str()) {
            None | Some("Z") => "+00:00".to_owned(),
            Some(z) if z.len() == 5 => format!("{}:{}", &z[..3], &z[3..]),
            Some(z) => z.to_owned(),
        };
        let stamp = format!("{}T{time}{zone}", &caps[1]);
        match chrono::DateTime::parse_from_rfc3339(&stamp) {
            Ok(at) if millis => at.timestamp_millis().to_string(),
            Ok(at) => at.timestamp().to_string(),
            Err(_) => whole,
        }
    })
    .into_owned()
}

// ---------------------------------------------------------------- ops

fn yes() -> bool {
    true
}

fn one() -> usize {
    1
}

fn comma_space() -> String {
    ", ".to_owned()
}

/// A Columns and Fields command with its settings. It is a Transform
/// menu command, a macro step and (with the same JSON) a recipe step:
/// `{"op": "extract_matches", "pattern": "…"}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FieldOp {
    ExtractMatches {
        pattern: String,
        #[serde(default)]
        match_case: bool,
        /// 0 is the whole match, 1 the first capture group…
        #[serde(default)]
        group: usize,
        #[serde(default)]
        unique: bool,
    },
    AlignColumns {
        #[serde(default)]
        delimiter: String,
        #[serde(default = "yes")]
        spaced: bool,
    },
    SelectFields {
        #[serde(default)]
        delimiter: String,
        fields: String,
        #[serde(default)]
        remove: bool,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        output_delimiter: String,
    },
    SortByField {
        #[serde(default)]
        delimiter: String,
        #[serde(default = "one")]
        field: usize,
        #[serde(default)]
        order: FieldOrder,
        #[serde(default)]
        descending: bool,
    },
    SplitLines {
        separator: String,
        #[serde(default = "yes")]
        trim: bool,
    },
    JoinWith {
        #[serde(default = "comma_space")]
        separator: String,
        #[serde(default)]
        quote: String,
        #[serde(default = "yes")]
        skip_empty: bool,
    },
    EscapeString {
        language: StringKind,
    },
    UnescapeString {
        language: StringKind,
    },
    UnixTimeToDate {
        #[serde(default = "yes")]
        utc: bool,
    },
    DateToUnixTime {
        #[serde(default)]
        millis: bool,
    },
}

/// Which Columns and Fields dialog: the commands with settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldKind {
    Extract,
    Align,
    Select,
    SortBy,
    Split,
    Join,
}

impl FieldKind {
    pub const ALL: [FieldKind; 6] = [
        FieldKind::Extract,
        FieldKind::Align,
        FieldKind::Select,
        FieldKind::SortBy,
        FieldKind::Split,
        FieldKind::Join,
    ];

    /// The command with its usual settings.
    pub fn default_op(self) -> FieldOp {
        match self {
            FieldKind::Extract => FieldOp::ExtractMatches {
                pattern: String::new(),
                match_case: false,
                group: 0,
                unique: false,
            },
            FieldKind::Align => FieldOp::AlignColumns {
                delimiter: "=".into(),
                spaced: true,
            },
            FieldKind::Select => FieldOp::SelectFields {
                delimiter: ",".into(),
                fields: "1".into(),
                remove: false,
                output_delimiter: String::new(),
            },
            FieldKind::SortBy => FieldOp::SortByField {
                delimiter: ",".into(),
                field: 1,
                order: FieldOrder::Natural,
                descending: false,
            },
            FieldKind::Split => FieldOp::SplitLines {
                separator: ",".into(),
                trim: true,
            },
            FieldKind::Join => FieldOp::JoinWith {
                separator: ", ".into(),
                quote: "'".into(),
                skip_empty: true,
            },
        }
    }

    /// The translation key of its menu item (with `…`) and its name.
    pub fn keys(self) -> (&'static str, &'static str) {
        match self {
            FieldKind::Extract => ("extract-matches-dots", "extract-matches"),
            FieldKind::Align => ("align-columns-dots", "align-columns"),
            FieldKind::Select => ("select-fields-dots", "select-fields"),
            FieldKind::SortBy => ("sort-by-field-dots", "sort-by-field"),
            FieldKind::Split => ("split-lines-at-dots", "split-lines-at"),
            FieldKind::Join => ("join-lines-with-dots", "join-lines-with"),
        }
    }
}

impl FieldOp {
    /// The dialog that edits it, when it has settings worth a dialog.
    pub fn kind(&self) -> Option<FieldKind> {
        Some(match self {
            FieldOp::ExtractMatches { .. } => FieldKind::Extract,
            FieldOp::AlignColumns { .. } => FieldKind::Align,
            FieldOp::SelectFields { .. } => FieldKind::Select,
            FieldOp::SortByField { .. } => FieldKind::SortBy,
            FieldOp::SplitLines { .. } => FieldKind::Split,
            FieldOp::JoinWith { .. } => FieldKind::Join,
            _ => return None,
        })
    }

    /// Whether it works on whole lines (a selection is grown to whole
    /// lines first).
    pub fn whole_lines(&self) -> bool {
        !matches!(
            self,
            FieldOp::ExtractMatches { .. }
                | FieldOp::EscapeString { .. }
                | FieldOp::UnescapeString { .. }
                | FieldOp::UnixTimeToDate { .. }
                | FieldOp::DateToUnixTime { .. }
        )
    }

    /// Checks the settings without running anything.
    pub fn check(&self) -> Result<(), ToolError> {
        match self {
            FieldOp::ExtractMatches {
                pattern,
                match_case,
                group,
                ..
            } => extract_matches("", pattern, *match_case, *group, false).map(|_| ()),
            FieldOp::SelectFields { fields, .. } => FieldSpec::parse(fields).map(|_| ()),
            FieldOp::SortByField { field: 0, .. } => {
                Err(ToolError::new("Fields are counted from 1."))
            }
            FieldOp::SplitLines { separator, .. } if separator.is_empty() => {
                Err(ToolError::new("Type the text to split at."))
            }
            _ => Ok(()),
        }
    }

    /// Runs it on `text`.
    pub fn apply(&self, text: &str) -> ToolResult {
        match self {
            FieldOp::ExtractMatches {
                pattern,
                match_case,
                group,
                unique,
            } => extract_matches(text, pattern, *match_case, *group, *unique),
            FieldOp::AlignColumns { delimiter, spaced } => {
                Ok(align_columns(text, delimiter, *spaced))
            }
            FieldOp::SelectFields {
                delimiter,
                fields,
                remove,
                output_delimiter,
            } => select_fields(text, delimiter, fields, *remove, output_delimiter),
            FieldOp::SortByField {
                delimiter,
                field,
                order,
                descending,
            } => sort_by_field(text, delimiter, *field, *order, *descending),
            FieldOp::SplitLines { separator, trim } => split_at(text, separator, *trim),
            FieldOp::JoinWith {
                separator,
                quote,
                skip_empty,
            } => Ok(join_with(text, separator, quote, *skip_empty)),
            FieldOp::EscapeString { language } => Ok(escape(text, *language)),
            FieldOp::UnescapeString { language } => unescape(text, *language),
            FieldOp::UnixTimeToDate { utc } => Ok(epoch_to_iso(text, *utc)),
            FieldOp::DateToUnixTime { millis } => Ok(iso_to_epoch(text, *millis)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_read_and_write_recipe_json() {
        let op: FieldOp =
            serde_json::from_str(r#"{"op": "join_with", "quote": "'"}"#).unwrap();
        assert_eq!(op.apply("a\nb\n").unwrap(), "'a', 'b'\n");
        for kind in FieldKind::ALL {
            let op = kind.default_op();
            assert_eq!(op.kind(), Some(kind));
            let json = serde_json::to_string(&op).unwrap();
            assert_eq!(serde_json::from_str::<FieldOp>(&json).unwrap(), op);
        }
    }

    #[test]
    fn natural_order_reads_numbers() {
        let mut v = vec!["file10", "File2", "file1", "file02b", "a"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["a", "file1", "File2", "file02b", "file10"]);
    }

    #[test]
    fn extracts_matches_and_groups() {
        let log = "ERR 42 disk\nok\nERR 7 net\nERR 42 again\n";
        assert_eq!(
            extract_matches(log, r"ERR (\d+)", true, 1, false).unwrap(),
            "42\n7\n42\n"
        );
        assert_eq!(
            extract_matches(log, r"err (\d+)", false, 1, true).unwrap(),
            "42\n7\n"
        );
        assert!(extract_matches(log, r"ERR", true, 1, false).is_err());
        assert!(extract_matches(log, r"(", true, 0, false).is_err());
    }

    #[test]
    fn splits_csv_fields_with_quotes() {
        assert_eq!(
            split_fields(r#"a,"b, c",d"#, ","),
            vec!["a", "\"b, c\"", "d"]
        );
        assert_eq!(
            split_fields(r#""say ""hi""",x"#, ","),
            vec![r#""say ""hi""""#, "x"]
        );
        assert_eq!(split_fields("a  b\tc", ""), vec!["a", "b", "c"]);
        assert_eq!(split_fields("a::b::c", "::"), vec!["a", "b", "c"]);
    }

    #[test]
    fn selects_reorders_and_removes_fields() {
        let csv = "id,name,age\n1,Ann,30\nplain line\n";
        assert_eq!(
            select_fields(csv, ",", "2,1", false, "").unwrap(),
            "name,id\nAnn,1\nplain line\n"
        );
        assert_eq!(
            select_fields(csv, ",", "2", true, "\\t").unwrap(),
            "id\tage\n1\t30\nplain line\n"
        );
        assert_eq!(
            select_fields("a b c d", "", "2-", false, "").unwrap(),
            "b c d"
        );
        assert!(select_fields(csv, ",", "x", false, "").is_err());
        assert!(select_fields(csv, ",", "3-1", false, "").is_err());
    }

    #[test]
    fn sorts_by_field() {
        let t = "b,file10\na,file2\nnone\nc,file1\n";
        assert_eq!(
            sort_by_field(t, ",", 2, FieldOrder::Natural, false).unwrap(),
            "none\nc,file1\na,file2\nb,file10\n"
        );
        assert_eq!(
            sort_by_field("x 3\ny 20\nz 1.5", "", 2, FieldOrder::Number, true).unwrap(),
            "y 20\nx 3\nz 1.5"
        );
    }

    #[test]
    fn aligns_columns() {
        let t = "  a = 1\n  long_name = 22\nno delimiter\n";
        assert_eq!(
            align_columns(t, "=", true),
            "  a         = 1\n  long_name = 22\nno delimiter\n"
        );
        assert_eq!(
            align_columns("| a | bb |\n| ccc | d |", "|", true),
            "| a   | bb |\n| ccc | d  |"
        );
        assert_eq!(align_columns("x 1\nyy  22", "", false), "x  1\nyy 22");
    }

    #[test]
    fn splits_and_joins() {
        assert_eq!(split_at("a, b,,c\nd", ",", true).unwrap(), "a\nb\nc\nd");
        assert_eq!(split_at("a,,b", ",", false).unwrap(), "a\n\nb");
        assert_eq!(
            join_with("a\nO'Neil\n\nc\n", ", ", "'", true),
            "'a', 'O''Neil', 'c'\n"
        );
        assert_eq!(join_with("1\n2", "\\t", "", false), "1\t2");
    }

    #[test]
    fn escapes_round_trip() {
        let s = "He said \"hi\"\n\tit's C:\\temp é";
        for kind in StringKind::ALL {
            let e = escape(s, kind);
            assert_eq!(unescape(&e, kind).unwrap(), s, "{kind:?}: {e}");
        }
        assert_eq!(escape("a\"b\n", StringKind::Json), "a\\\"b\\n");
        assert_eq!(escape("it's", StringKind::Shell), r"'it'\''s'");
        assert_eq!(escape("a.b*", StringKind::Regex), r"a\.b\*");
        assert_eq!(escape("O'Neil", StringKind::Sql), "O''Neil");
        assert_eq!(unescape(r#""\ud83d\ude00""#, StringKind::Json).unwrap(), "😀");
        assert!(unescape(r"\q", StringKind::Json).is_err());
        assert_eq!(unescape(r#"a\ b"c\$d""#, StringKind::Shell).unwrap(), "a bc$d");
    }

    #[test]
    fn converts_unix_times() {
        assert_eq!(
            epoch_to_iso("at 1763000000 and 1763000000123 not 123", true),
            "at 2025-11-13T02:13:20Z and 2025-11-13T02:13:20.123Z not 123"
        );
        assert_eq!(
            iso_to_epoch("at 2025-11-13T02:13:20Z, 2025-11-13 03:13:20+01:00", false),
            "at 1763000000, 1763000000"
        );
        assert_eq!(
            iso_to_epoch("2025-11-13T02:13:20.123Z", true),
            "1763000000123"
        );
    }
}
