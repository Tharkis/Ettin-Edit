//! HL7 version 2 messages as tables: a parser for the pipe-and-hat format
//! (segments, fields, repeats, components, subcomponents and escapes, with
//! the separators each message declares in MSH), names for fields and
//! components, and writing messages from a template.
//!
//! Columns are named the usual way: `PID.5.1` is PID field 5, component 1.
//! A second segment of the same kind in a row is `OBX[2]`, and a field's
//! second repeat is `PID.3[2]`.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The separators a message uses (MSH-1 and MSH-2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Encoding {
    pub field: char,
    pub component: char,
    pub repeat: char,
    pub escape: char,
    pub sub: char,
}

impl Default for Encoding {
    fn default() -> Self {
        Self {
            field: '|',
            component: '^',
            repeat: '~',
            escape: '\\',
            sub: '&',
        }
    }
}

/// One segment: its id and its fields as written (escapes left in).
/// `fields[n]` is field `n`; `fields[0]` is the id. For MSH, `fields[1]` is
/// the field separator and `fields[2]` the encoding characters, as HL7
/// numbers them.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub id: String,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub encoding: Encoding,
    pub segments: Vec<Segment>,
}

/// Whether `text` looks like HL7 v2: it starts with an MSH (or batch
/// header) segment.
pub fn looks_like(text: &str) -> bool {
    let t = text.trim_start_matches(['\u{feff}', '\r', '\n', ' ']);
    ["MSH", "FHS", "BHS"]
        .iter()
        .any(|h| t.starts_with(h) && t[3..].chars().next().is_some_and(|c| !c.is_alphanumeric()))
}

/// Splits text into messages. Segments end with CR, LF or CRLF; a message
/// starts at each MSH. Batch and file header and trailer segments (FHS, BHS,
/// BTS, FTS) and anything before the first MSH are left out.
pub fn parse(text: &str) -> Result<Vec<Message>, String> {
    let mut messages: Vec<Message> = Vec::new();
    for line in text.split(['\r', '\n']) {
        let line = line.trim_start_matches('\u{feff}').trim_end();
        if line.len() < 3 {
            continue;
        }
        if line.starts_with("MSH") {
            let encoding = encoding_of(line)?;
            let mut fields: Vec<String> = Vec::new();
            fields.push("MSH".into());
            fields.push(encoding.field.to_string());
            let rest = &line[3 + encoding.field.len_utf8()..];
            fields.extend(rest.split(encoding.field).map(str::to_owned));
            messages.push(Message {
                encoding,
                segments: vec![Segment {
                    id: "MSH".into(),
                    fields,
                }],
            });
            continue;
        }
        let Some(message) = messages.last_mut() else {
            continue;
        };
        let id: String = line.chars().take(3).collect();
        if matches!(id.as_str(), "FHS" | "BHS" | "BTS" | "FTS") {
            continue;
        }
        let fields: Vec<String> = line
            .split(message.encoding.field)
            .map(str::to_owned)
            .collect();
        message.segments.push(Segment {
            id: fields[0].clone(),
            fields,
        });
    }
    if messages.is_empty() {
        return Err(t!("table-hl7-no-messages"));
    }
    Ok(messages)
}

fn encoding_of(msh: &str) -> Result<Encoding, String> {
    let mut chars = msh[3..].chars();
    let field = chars.next().ok_or_else(|| t!("table-hl7-bad-msh"))?;
    let enc: Vec<char> = chars.take_while(|c| *c != field).collect();
    if enc.len() < 2 {
        return Err(t!("table-hl7-bad-msh"));
    }
    let d = Encoding::default();
    Ok(Encoding {
        field,
        component: enc[0],
        repeat: enc[1],
        escape: enc.get(2).copied().unwrap_or(d.escape),
        sub: enc.get(3).copied().unwrap_or(d.sub),
    })
}

/// Replaces escape sequences (`\F\`, `\S\`, `\T\`, `\R\`, `\E\`, `\.br\`,
/// `\Xhh…\`) with what they stand for. Others are kept as written.
pub fn unescape(text: &str, e: &Encoding) -> String {
    if !text.contains(e.escape) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(e.escape) {
        out.push_str(&rest[..start]);
        let after = &rest[start + e.escape.len_utf8()..];
        let Some(end) = after.find(e.escape) else {
            out.push_str(&rest[start..]);
            return out;
        };
        let code = &after[..end];
        match code {
            "F" => out.push(e.field),
            "S" => out.push(e.component),
            "T" => out.push(e.sub),
            "R" => out.push(e.repeat),
            "E" => out.push(e.escape),
            ".br" => out.push('\n'),
            c if c.starts_with('X') && c.len() > 1 && c.len() % 2 == 1 => {
                let bytes: Option<Vec<u8>> = (1..c.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&c[i..i + 2], 16).ok())
                    .collect();
                match bytes {
                    Some(b) => out.push_str(&String::from_utf8_lossy(&b)),
                    None => out.push_str(&rest[start..start + 2 * e.escape.len_utf8() + end]),
                }
            }
            _ => out.push_str(&rest[start..start + 2 * e.escape.len_utf8() + end]),
        }
        rest = &after[end + e.escape.len_utf8()..];
    }
    out.push_str(rest);
    out
}

/// Escapes the separators in `text` so it can go into a field. Line breaks
/// become `\.br\`.
pub fn escape(text: &str, e: &Encoding) -> String {
    let mut out = String::with_capacity(text.len());
    let esc = |out: &mut String, code: &str| {
        out.push(e.escape);
        out.push_str(code);
        out.push(e.escape);
    };
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c == e.escape => esc(&mut out, "E"),
            c if c == e.field => esc(&mut out, "F"),
            c if c == e.component => esc(&mut out, "S"),
            c if c == e.sub => esc(&mut out, "T"),
            c if c == e.repeat => esc(&mut out, "R"),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                esc(&mut out, ".br");
            }
            '\n' => esc(&mut out, ".br"),
            c => out.push(c),
        }
    }
    out
}

/// A segment's fields as `(name, value)` pairs, named after `prefix`
/// (`PID`, `OBX[2]`). With `components`, a field with components is split
/// into `PID.5.1`, `PID.5.2`…, and subcomponents into `PID.3.4.1`. Empty
/// values are left out.
pub fn segment_pairs(
    seg: &Segment,
    prefix: &str,
    e: &Encoding,
    components: bool,
    out: &mut Vec<(String, String)>,
) {
    for (n, raw) in seg.fields.iter().enumerate().skip(1) {
        if raw.is_empty() {
            continue;
        }
        let field = format!("{prefix}.{n}");
        // MSH-1 and MSH-2 are the separators themselves, not data.
        if seg.id == "MSH" && n <= 2 {
            continue;
        }
        for (r, rep) in raw.split(e.repeat).enumerate() {
            if rep.is_empty() {
                continue;
            }
            let name = if r == 0 {
                field.clone()
            } else {
                format!("{field}[{}]", r + 1)
            };
            if !components || !(rep.contains(e.component) || rep.contains(e.sub)) {
                let value = if components {
                    unescape(rep, e)
                } else {
                    rep.to_owned()
                };
                out.push((name, value));
                continue;
            }
            for (c, comp) in rep.split(e.component).enumerate() {
                if comp.is_empty() {
                    continue;
                }
                let cname = format!("{name}.{}", c + 1);
                if comp.contains(e.sub) {
                    for (s, sub) in comp.split(e.sub).enumerate() {
                        if !sub.is_empty() {
                            out.push((format!("{cname}.{}", s + 1), unescape(sub, e)));
                        }
                    }
                } else {
                    out.push((cname, unescape(comp, e)));
                }
            }
        }
    }
}

/// Names segments in order: `PID`, then `OBX`, `OBX[2]`… for repeats.
fn numbered<'a>(segments: impl Iterator<Item = &'a Segment>) -> Vec<(&'a Segment, String)> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    segments
        .map(|s| {
            let n = seen.entry(s.id.as_str()).or_insert(0);
            *n += 1;
            let prefix = if *n == 1 {
                s.id.clone()
            } else {
                format!("{}[{n}]", s.id)
            };
            (s, prefix)
        })
        .collect()
}

/// A whole message as pairs, one column per field (or component).
pub fn message_pairs(m: &Message, components: bool) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (seg, prefix) in numbered(m.segments.iter()) {
        segment_pairs(seg, &prefix, &m.encoding, components, &mut out);
    }
    out
}

/// The rows of a set of messages: one per message when `segment` is empty,
/// otherwise one per segment with that id, with the segments that follow
/// it (up to the next one) and the fields named in `keep` copied from the
/// rest of its message.
pub fn rows(
    messages: &[Message],
    segment: &str,
    components: bool,
    keep: &[String],
) -> Vec<Vec<(String, String)>> {
    let segment = segment.trim().to_ascii_uppercase();
    if segment.is_empty() {
        return messages
            .iter()
            .map(|m| message_pairs(m, components))
            .collect();
    }
    let mut rows = Vec::new();
    for m in messages {
        let whole = if keep.is_empty() {
            Vec::new()
        } else {
            message_pairs(m, components)
        };
        let kept: Vec<(String, String)> = keep
            .iter()
            .flat_map(|k| {
                let k = k.trim();
                let prefix = format!("{k}.");
                whole
                    .iter()
                    .filter(move |(n, _)| n == k || n.starts_with(&prefix))
                    .cloned()
            })
            .collect();
        let starts: Vec<usize> = m
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == segment)
            .map(|(i, _)| i)
            .collect();
        for (k, &start) in starts.iter().enumerate() {
            let end = starts.get(k + 1).copied().unwrap_or(m.segments.len());
            let mut row = kept.clone();
            for (seg, prefix) in numbered(m.segments[start..end].iter()) {
                segment_pairs(seg, &prefix, &m.encoding, components, &mut row);
            }
            rows.push(row);
        }
    }
    rows
}

/// Turns rows of pairs into column names and rows. Columns come in the
/// order they first appear; when one message has `PID.5` and another
/// `PID.5.1`, the plain one joins `PID.5.1`.
pub fn tabulate(rows: Vec<Vec<(String, String)>>) -> (Vec<String>, Vec<Vec<String>>) {
    let mut names: Vec<String> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for row in &rows {
        for (n, _) in row {
            if !seen.contains_key(n) {
                seen.insert(n.clone(), names.len());
                names.push(n.clone());
            }
        }
    }
    let rename: HashMap<String, String> = names
        .iter()
        .filter(|n| seen.contains_key(&format!("{n}.1")))
        .map(|n| (n.clone(), format!("{n}.1")))
        .collect();
    let names: Vec<String> = names
        .into_iter()
        .filter(|n| !rename.contains_key(n))
        .collect();
    let index: HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let rows = rows
        .into_iter()
        .map(|row| {
            let mut r = vec![String::new(); names.len()];
            for (n, v) in row {
                let n = rename.get(&n).cloned().unwrap_or(n);
                if let Some(&i) = index.get(n.as_str())
                    && r[i].is_empty()
                {
                    r[i] = v;
                }
            }
            r
        })
        .collect();
    (names, rows)
}

/// Every segment id in the messages, in the order they first appear.
pub fn segment_ids(messages: &[Message]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for s in messages.iter().flat_map(|m| &m.segments) {
        if !ids.contains(&s.id) {
            ids.push(s.id.clone());
        }
    }
    ids
}

// ----------------------------------------------------------------------
// Names

struct Dict {
    segments: HashMap<String, String>,
    fields: HashMap<(String, usize), (String, String)>,
    types: HashMap<String, String>,
    components: HashMap<(String, usize), (String, String)>,
}

fn dict() -> &'static Dict {
    static DICT: OnceLock<Dict> = OnceLock::new();
    DICT.get_or_init(|| {
        let mut d = Dict {
            segments: HashMap::new(),
            fields: HashMap::new(),
            types: HashMap::new(),
            components: HashMap::new(),
        };
        for line in include_str!("hl7_dict.txt").lines() {
            let p: Vec<&str> = line.split('|').collect();
            match p.as_slice() {
                ["S", seg, desc] => {
                    d.segments.insert((*seg).into(), (*desc).into());
                }
                ["F", seg, n, name, ty] => {
                    if let Ok(n) = n.parse() {
                        d.fields
                            .insert(((*seg).into(), n), ((*name).into(), (*ty).into()));
                    }
                }
                ["T", ty, name] => {
                    d.types.insert((*ty).into(), (*name).into());
                }
                ["C", ty, n, name, cty] => {
                    if let Ok(n) = n.parse() {
                        d.components
                            .insert(((*ty).into(), n), ((*name).into(), (*cty).into()));
                    }
                }
                _ => {}
            }
        }
        d
    })
}

/// What a column named like `PID.5.1` holds, from the HL7 v2.5.1
/// definitions: `Patient Identification › Patient Name › Family Name`.
/// `None` for names that aren't HL7 designations.
pub fn describe(column: &str) -> Option<String> {
    let mut parts = column.split('.');
    let seg = strip_index(parts.next()?);
    if seg.len() != 3 || !seg.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let field: usize = strip_index(parts.next()?).parse().ok()?;
    let d = dict();
    let mut out = vec![
        d.segments
            .get(seg)
            .cloned()
            .unwrap_or_else(|| seg.to_owned()),
    ];
    let (fname, mut ty) = d.fields.get(&(seg.to_owned(), field))?.clone();
    out.push(fname);
    for part in parts {
        let n: usize = part.parse().ok()?;
        match d.components.get(&(ty.clone(), n)) {
            Some((name, cty)) => {
                out.push(name.clone());
                ty = cty.clone();
            }
            None => {
                out.push(format!("{n}"));
                break;
            }
        }
    }
    let type_name = d
        .types
        .get(&ty)
        .map(|n| format!(" ({ty}, {n})"))
        .unwrap_or_default();
    Some(format!("{}{type_name}", out.join(" › ")))
}

/// `OBX[2]` → `OBX`; `3[2]` → `3`.
fn strip_index(s: &str) -> &str {
    s.split('[').next().unwrap_or(s)
}

// ----------------------------------------------------------------------
// Writing

/// Fills a message template for one row. `{Column}` is replaced with the
/// row's value of that column, escaped for HL7; `{{` and `}}` are literal
/// braces. Segments may be written one per line; they are joined with CR.
/// An unknown column is an error, so typos don't go unnoticed.
pub fn fill(template: &str, value: &dyn Fn(&str) -> Option<String>) -> Result<String, String> {
    let encoding = template
        .lines()
        .find(|l| l.starts_with("MSH"))
        .and_then(|l| encoding_of(l).ok())
        .unwrap_or_default();
    let mut segments = Vec::new();
    for line in template.split(['\r', '\n']) {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let mut out = String::with_capacity(line.len());
        let mut rest = line;
        while let Some(i) = rest.find(['{', '}']) {
            out.push_str(&rest[..i]);
            let after = &rest[i + 1..];
            if rest[i..].starts_with("{{") {
                out.push('{');
                rest = &rest[i + 2..];
            } else if rest[i..].starts_with("}}") {
                out.push('}');
                rest = &rest[i + 2..];
            } else if rest[i..].starts_with('{')
                && let Some(end) = after.find('}')
            {
                let name = &after[..end];
                let v = value(name.trim())
                    .ok_or_else(|| t!("table-no-column", column = name.trim().to_owned()))?;
                out.push_str(&escape(&v, &encoding));
                rest = &after[end + 1..];
            } else {
                out.push_str(&rest[i..i + 1]);
                rest = after;
            }
        }
        out.push_str(rest);
        segments.push(out);
    }
    Ok(segments.join("\r"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAB: &str = include_str!("../../samples/tables/lab-results.hl7");

    #[test]
    fn parses_messages_and_separators() {
        let m = parse(LAB).unwrap();
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].segments[0].fields[1], "|");
        assert_eq!(m[0].segments[0].fields[2], "^~\\&");
        assert_eq!(m[0].segments[0].fields[10], "LAB20261103-0001");
        assert_eq!(m[0].segments.iter().filter(|s| s.id == "OBX").count(), 4);
        // Other separators, declared in MSH.
        let odd = parse("MSH#*!/%#A#B\rPID#1##12*X!13*Y#").unwrap();
        let pairs = message_pairs(&odd[0], true);
        assert!(pairs.contains(&("PID.3.1".into(), "12".into())));
        assert!(pairs.contains(&("PID.3[2].2".into(), "Y".into())));
    }

    #[test]
    fn escapes_round_trip() {
        let e = Encoding::default();
        assert_eq!(
            unescape(r"A \T\ B\.br\C \F\ \S\ \E\ \X41\", &e),
            "A & B\nC | ^ \\ A"
        );
        let text = "a|b^c&d~e\\f\ng";
        assert_eq!(unescape(&escape(text, &e), &e), text);
    }

    #[test]
    fn one_row_per_message() {
        let m = parse(LAB).unwrap();
        let (names, rows) = tabulate(rows(&m, "", true, &[]));
        let col = |n: &str| names.iter().position(|x| x == n).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0][col("PID.5.1")], "Quillfeather");
        assert_eq!(rows[1][col("PID.5.1")], "Okafor-Lindqvist");
        assert_eq!(rows[0][col("PID.3[2].1")], "X44812");
        assert_eq!(rows[0][col("OBX[3].5")], "5.4");
        assert_eq!(
            rows[0][col("NTE.3")],
            "Slight hemolysis & repeat draw advised.\nCalled to Dr. Halvorsen 08:10."
        );
        // PID.8 never has components, so it stays whole.
        assert_eq!(rows[2][col("PID.8")], "U");
    }

    #[test]
    fn one_row_per_segment_with_parent_fields() {
        let m = parse(LAB).unwrap();
        let keep = vec!["MSH.10".to_owned(), "PID.3".to_owned()];
        let (names, rows) = tabulate(rows(&m, "obx", true, &keep));
        let col = |n: &str| names.iter().position(|x| x == n).unwrap();
        assert_eq!(rows.len(), 9);
        assert_eq!(names[0], "MSH.10");
        assert_eq!(rows[3][col("OBX.3.2")], "Sodium");
        assert_eq!(rows[2][col("NTE.3")].lines().count(), 2);
        assert_eq!(rows[2][col("PID.3.1")], "800123456");
        assert_eq!(
            rows[8][col("OBX.5")],
            "Result pending confirmation | see note"
        );
        assert_eq!(rows[8][col("MSH.10")], "LAB20261103-0003");
    }

    #[test]
    fn names_fields_and_components() {
        assert_eq!(
            describe("PID.5.1").unwrap(),
            "Patient Identification › Patient Name › Family Name (FN, Family Name)"
        );
        assert!(describe("OBX[2].5").unwrap().contains("Observation Value"));
        assert_eq!(describe("Name"), None);
        assert_eq!(describe("ZZZ.1"), None);
    }

    #[test]
    fn fills_a_template() {
        let row = |n: &str| match n {
            "id" => Some("42".to_owned()),
            "name" => Some("O'Brien^Pat".to_owned()),
            _ => None,
        };
        let out = fill(
            "MSH|^~\\&|ETTIN||||||ADT^A08|{id}|P|2.5.1\nPID|1||{id}||{name}",
            &row,
        )
        .unwrap();
        assert_eq!(
            out,
            "MSH|^~\\&|ETTIN||||||ADT^A08|42|P|2.5.1\rPID|1||42||O'Brien\\S\\Pat"
        );
        assert!(fill("PID|{nope}", &row).is_err());
        assert!(looks_like(&out));
        assert!(!looks_like("Name,MSH"));
    }
}
