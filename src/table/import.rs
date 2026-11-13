//! Reading a table: delimited text (CSV, TSV…), Excel and OpenDocument
//! workbooks, JSON and XML (rows found by a path, nested values flattened
//! to dotted columns) and HL7 v2 messages.

use std::io::Cursor;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Table, unique_names};
use crate::tools::csv::{self, CsvFormat};

fn yes() -> bool {
    true
}
fn is_true(b: &bool) -> bool {
    *b
}
fn one() -> usize {
    1
}
fn comma() -> char {
    ','
}
fn dquote() -> Option<char> {
    Some('"')
}
fn default_keep() -> Vec<String> {
    ["MSH.10", "PID.3.1", "PID.5.1", "PID.5.2"]
        .map(String::from)
        .to_vec()
}

/// How the table's data is read. Saved as the first step of a table job.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum Source {
    /// Delimited text.
    Csv {
        #[serde(default = "comma")]
        delimiter: char,
        /// `None` when fields are never quoted.
        #[serde(default = "dquote")]
        quote: Option<char>,
        #[serde(default = "yes")]
        header: bool,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        detect_types: bool,
    },
    /// An Excel (`.xlsx`, `.xlsm`, `.xls`, `.xlsb`) or OpenDocument (`.ods`)
    /// workbook.
    Excel {
        /// The sheet's name; the first sheet when empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        sheet: String,
        /// The row with the column names, counted from 1 as the
        /// spreadsheet shows it. 0 when there is none.
        #[serde(default = "one")]
        header_row: usize,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        detect_types: bool,
    },
    /// JSON: a list of objects (or of lists, the first naming the columns).
    Json {
        /// Where the list is, like `data.items`; found by itself when empty.
        /// A path through a list, like `orders.lines`, takes the rows from
        /// every item.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rows: String,
        /// On a path through a list, copy each item's other fields
        /// (`orders.id`) into its rows.
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        parents: bool,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        detect_types: bool,
    },
    /// XML: one row per element at a path like `schedule.appointment`.
    /// Attributes and child elements become columns.
    Xml {
        /// Element names from the root down; the root may be left out.
        /// Found by itself when empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rows: String,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        parents: bool,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        detect_types: bool,
    },
    /// HL7 v2 messages: a row per message, or per segment of one kind.
    Hl7 {
        /// The segment that makes a row, like `OBX`; one row per message
        /// when empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        segment: String,
        /// Split fields into components (`PID.5.1`), or keep them whole.
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        components: bool,
        /// With a segment: fields copied from the rest of the message.
        #[serde(default = "default_keep")]
        keep: Vec<String>,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        detect_types: bool,
    },
}

impl Default for Source {
    fn default() -> Self {
        Source::Csv {
            delimiter: ',',
            quote: Some('"'),
            header: true,
            detect_types: true,
        }
    }
}

/// What a table is read from.
pub enum Input<'a> {
    Text(&'a str),
    Bytes(&'a [u8]),
}

/// File extensions read as workbooks.
pub const WORKBOOK_EXTENSIONS: [&str; 5] = ["xlsx", "xlsm", "xls", "xlsb", "ods"];

pub fn is_workbook(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| WORKBOOK_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

impl Source {
    pub fn is_excel(&self) -> bool {
        matches!(self, Source::Excel { .. })
    }

    fn detect_types(&self) -> bool {
        match self {
            Source::Csv { detect_types, .. }
            | Source::Excel { detect_types, .. }
            | Source::Json { detect_types, .. }
            | Source::Xml { detect_types, .. }
            | Source::Hl7 { detect_types, .. } => *detect_types,
        }
    }

    pub fn set_detect_types(&mut self, on: bool) {
        match self {
            Source::Csv { detect_types, .. }
            | Source::Excel { detect_types, .. }
            | Source::Json { detect_types, .. }
            | Source::Xml { detect_types, .. }
            | Source::Hl7 { detect_types, .. } => *detect_types = on,
        }
    }

    /// A guess at how to read `text`: JSON, XML or HL7 when it starts like
    /// them, otherwise delimited text with the delimiter worked out.
    pub fn guess(text: &str, extension: Option<&str>) -> Source {
        let start = text.trim_start_matches('\u{feff}').trim_start();
        if extension == Some("json")
            || (start.starts_with(['[', '{']) && serde_json::from_str::<Value>(text).is_ok())
        {
            return Source::Json {
                rows: String::new(),
                parents: true,
                detect_types: true,
            };
        }
        if extension == Some("hl7") || super::hl7::looks_like(text) {
            return Source::Hl7 {
                segment: String::new(),
                components: true,
                keep: default_keep(),
                detect_types: true,
            };
        }
        if extension == Some("xml")
            || (start.starts_with('<') && roxmltree::Document::parse(text).is_ok())
        {
            return Source::Xml {
                rows: String::new(),
                parents: true,
                detect_types: true,
            };
        }
        let f = csv::detect(text, extension);
        Source::Csv {
            delimiter: f.delimiter as char,
            quote: f.quote.map(|q| q as char),
            header: f.header,
            detect_types: true,
        }
    }

    /// A short description for the Steps list.
    pub fn summary(&self) -> String {
        match self {
            Source::Csv {
                delimiter, header, ..
            } => {
                let d = csv::delimiter_name(*delimiter as u8);
                if *header {
                    t!("table-source-csv", delimiter = d)
                } else {
                    t!("table-source-csv-no-header", delimiter = d)
                }
            }
            Source::Excel {
                sheet, header_row, ..
            } => {
                let sheet = if sheet.is_empty() {
                    t!("table-first-sheet")
                } else {
                    t!("table-sheet-named", sheet = sheet.clone())
                };
                if *header_row == 0 {
                    t!("table-source-excel-no-header", sheet = sheet)
                } else {
                    t!("table-source-excel", sheet = sheet, row = *header_row)
                }
            }
            Source::Json { rows, .. } if rows.is_empty() => t!("table-source-json"),
            Source::Json { rows, .. } => t!("table-source-json-at", path = rows.clone()),
            Source::Xml { rows, .. } if rows.is_empty() => t!("table-source-xml"),
            Source::Xml { rows, .. } => t!("table-source-xml-at", path = rows.clone()),
            Source::Hl7 { segment, .. } if segment.is_empty() => t!("table-source-hl7"),
            Source::Hl7 { segment, .. } => {
                t!("table-source-hl7-segment", segment = segment.clone())
            }
        }
    }

    /// Reads the table.
    pub fn read(&self, input: Input) -> Result<Table, String> {
        let mut table = match (self, input) {
            (
                Source::Excel {
                    sheet, header_row, ..
                },
                Input::Bytes(b),
            ) => read_workbook(b, sheet, *header_row)?,
            (Source::Excel { .. }, Input::Text(_)) => return Err(t!("table-excel-needs-file")),
            (_, Input::Bytes(b)) => {
                let text = std::str::from_utf8(b).map_err(|_| t!("table-not-utf8"))?;
                let text = text.strip_prefix('\u{feff}').unwrap_or(text);
                return self.read(Input::Text(text));
            }
            (
                Source::Csv {
                    delimiter,
                    quote,
                    header,
                    ..
                },
                Input::Text(text),
            ) => {
                let format = CsvFormat {
                    delimiter: ascii(*delimiter)?,
                    quote: quote.map(ascii).transpose()?,
                    header: *header,
                };
                let t = csv::parse(text, &format).map_err(|e| e.message)?;
                let names = if *header {
                    t.columns
                } else {
                    vec![String::new(); t.columns.len()]
                };
                Table::from_rows(names, &t.rows)
            }
            (Source::Json { rows, parents, .. }, Input::Text(text)) => {
                let value: Value = serde_json::from_str(text)
                    .map_err(|e| t!("table-json-unreadable", error = e.to_string()))?;
                read_value(&value, rows, *parents)?
            }
            (Source::Xml { rows, parents, .. }, Input::Text(text)) => {
                let (root, value) = xml_value(text)?;
                let path = rows.trim();
                let path = if path.is_empty() || path.split('.').next() == Some(root.as_str()) {
                    path.to_owned()
                } else {
                    format!("{root}.{path}")
                };
                let mut t = read_value(&value, &path, *parents)?;
                // Parent columns are named from below the root element.
                let prefix = format!("{root}.");
                for c in &mut t.columns {
                    if let Some(rest) = c.name.strip_prefix(&prefix) {
                        c.name = rest.to_owned();
                    }
                }
                t
            }
            (
                Source::Hl7 {
                    segment,
                    components,
                    keep,
                    ..
                },
                Input::Text(text),
            ) => {
                let messages = super::hl7::parse(text)?;
                let (names, rows) =
                    super::hl7::tabulate(super::hl7::rows(&messages, segment, *components, keep));
                Table::from_rows(names, &rows)
            }
        };
        if self.detect_types() {
            table.detect_types();
        }
        Ok(table)
    }
}

fn ascii(c: char) -> Result<u8, String> {
    if c.is_ascii() {
        Ok(c as u8)
    } else {
        Err(t!("table-delimiter-ascii"))
    }
}

/// What the Import Options dialog can offer for `text`: paths to lists in
/// JSON, element paths in XML, and segment ids in HL7. Each is empty when
/// the text isn't in that format.
#[derive(Clone, Debug, Default)]
pub struct Choices {
    pub json: Vec<String>,
    pub xml: Vec<String>,
    pub segments: Vec<String>,
}

pub fn choices(text: &str) -> Choices {
    let start = text.trim_start_matches('\u{feff}').trim_start();
    let mut c = Choices::default();
    if start.starts_with(['[', '{'])
        && let Ok(v) = serde_json::from_str::<Value>(text)
    {
        list_paths(&v, "", &mut c.json);
    }
    if start.starts_with('<')
        && let Ok((root, v)) = xml_value(text)
    {
        let mut paths = Vec::new();
        list_paths(&v, "", &mut paths);
        // Elements that appear once but hold others can be rows too.
        object_paths(&v, "", &mut paths);
        let prefix = format!("{root}.");
        c.xml = paths
            .into_iter()
            .map(|p| p.strip_prefix(&prefix).map(str::to_owned).unwrap_or(p))
            .filter(|p| p != &root)
            .collect();
    }
    if super::hl7::looks_like(text)
        && let Ok(m) = super::hl7::parse(text)
    {
        c.segments = super::hl7::segment_ids(&m);
    }
    c
}

/// Paths to the lists of objects in `v`, through lists on the way.
fn list_paths(v: &Value, path: &str, out: &mut Vec<String>) {
    let join = |k: &str| {
        if path.is_empty() {
            k.to_owned()
        } else {
            format!("{path}.{k}")
        }
    };
    match v {
        Value::Array(a) => {
            if !path.is_empty() && a.iter().any(Value::is_object) && !out.iter().any(|p| p == path)
            {
                out.push(path.to_owned());
            }
            for item in a.iter().take(50) {
                if let Value::Object(m) = item {
                    for (k, v) in m {
                        list_paths(v, &join(k), out);
                    }
                }
            }
        }
        Value::Object(m) => {
            for (k, v) in m {
                list_paths(v, &join(k), out);
            }
        }
        _ => {}
    }
}

fn object_paths(v: &Value, path: &str, out: &mut Vec<String>) {
    if let Value::Object(m) = v {
        for (k, v) in m {
            let p = if path.is_empty() {
                k.clone()
            } else {
                format!("{path}.{k}")
            };
            if let Value::Object(inner) = v
                && inner.values().any(|x| x.is_object() || x.is_array())
            {
                if !out.contains(&p) {
                    out.push(p.clone());
                }
                object_paths(v, &p, out);
            }
        }
    }
}

/// The sheet names of a workbook.
pub fn sheet_names(bytes: &[u8]) -> Result<Vec<String>, String> {
    let wb = calamine::open_workbook_auto_from_rs(Cursor::new(bytes.to_vec()))
        .map_err(|e| t!("table-workbook-unreadable", error = e.to_string()))?;
    Ok(calamine::Reader::sheet_names(&wb))
}

/// The first row of a sheet with anything in it, counted from 1: where the
/// headers usually are. The first sheet when `sheet` is empty.
pub fn first_row(bytes: &[u8], sheet: &str) -> Option<usize> {
    use calamine::Reader;
    let mut wb = calamine::open_workbook_auto_from_rs(Cursor::new(bytes.to_vec())).ok()?;
    let name = if sheet.is_empty() {
        wb.sheet_names().first()?.clone()
    } else {
        sheet.to_owned()
    };
    let range = wb.worksheet_range(&name).ok()?;
    Some(range.start()?.0 as usize + 1)
}

fn read_workbook(bytes: &[u8], sheet: &str, header_row: usize) -> Result<Table, String> {
    use calamine::{Data, Reader};
    let mut wb = calamine::open_workbook_auto_from_rs(Cursor::new(bytes.to_vec()))
        .map_err(|e| t!("table-workbook-unreadable", error = e.to_string()))?;
    let names = wb.sheet_names();
    let name = if sheet.is_empty() {
        names
            .first()
            .cloned()
            .ok_or_else(|| t!("table-workbook-empty"))?
    } else {
        names
            .iter()
            .find(|n| *n == sheet)
            .or_else(|| names.iter().find(|n| n.eq_ignore_ascii_case(sheet)))
            .cloned()
            .ok_or_else(|| t!("table-no-sheet", sheet = sheet.to_owned()))?
    };
    let range = wb
        .worksheet_range(&name)
        .map_err(|e| t!("table-workbook-unreadable", error = e.to_string()))?;
    let Some((top, left)) = range.start() else {
        return Ok(Table::default());
    };
    let width = range.width();
    // Rows as the spreadsheet numbers them, from 1.
    let mut rows: Vec<(usize, Vec<String>)> = range
        .rows()
        .enumerate()
        .map(|(i, r)| {
            let mut row: Vec<String> = vec![String::new(); left as usize];
            row.extend(r.iter().map(cell_text));
            (top as usize + i + 1, row)
        })
        .collect();
    let width = width + left as usize;
    for (_, r) in &mut rows {
        r.resize(width, String::new());
    }
    let header: Vec<String> = if header_row == 0 {
        vec![String::new(); width]
    } else {
        let h = rows
            .iter()
            .find(|(n, _)| *n == header_row)
            .map(|(_, r)| r.clone())
            .unwrap_or_else(|| vec![String::new(); width]);
        rows.retain(|(n, _)| *n > header_row);
        h
    };
    // Empty rows at the end aren't data.
    while rows
        .last()
        .is_some_and(|(_, r)| r.iter().all(|c| c.is_empty()))
    {
        rows.pop();
    }
    // Columns that are empty all the way down, header included, go.
    let used: Vec<usize> = (0..width)
        .filter(|&c| !header[c].is_empty() || rows.iter().any(|(_, r)| !r[c].is_empty()))
        .collect();
    let header: Vec<String> = used.iter().map(|&c| header[c].clone()).collect();
    let rows: Vec<Vec<String>> = rows
        .into_iter()
        .map(|(_, r)| used.iter().map(|&c| r[c].clone()).collect())
        .collect();
    return Ok(Table::from_rows(header, &rows));

    fn cell_text(d: &Data) -> String {
        match d {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => number_text(*f),
            Data::Bool(b) => b.to_string(),
            Data::DateTime(dt) => match dt.as_datetime() {
                Some(at) if dt.is_datetime() => {
                    if at.time() == chrono::NaiveTime::MIN {
                        super::format_date(at.date())
                    } else {
                        super::format_datetime(at)
                    }
                }
                _ => number_text(dt.as_f64()),
            },
            Data::DateTimeIso(s) | Data::DurationIso(s) => s.replace('T', " "),
            Data::Error(e) => e.to_string(),
        }
    }
}

/// A spreadsheet number as text: whole numbers without `.0`.
fn number_text(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        let s = format!("{f}");
        // Floating point noise from the file: 0.30000000000000004.
        let short = format!("{:.12}", f)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned();
        if short.len() < s.len() && short.parse::<f64>().is_ok() {
            short
        } else {
            s
        }
    }
}

/// Reads a list of rows from JSON (or XML turned into JSON). Objects inside
/// a row become columns named with dots (`address.city`); lists inside a row
/// stay as JSON text. A path through a list (`orders.lines`) takes the rows
/// from every item, and with `parents` copies the item's other fields.
fn read_value(value: &Value, path: &str, parents: bool) -> Result<Table, String> {
    let path = path.trim().trim_start_matches("$.");
    let mut found: Vec<(&Value, Vec<(String, String)>)> = Vec::new();
    if path.is_empty() {
        let list = find_list(value).ok_or_else(|| t!("table-json-no-list"))?;
        found.extend(list.iter().map(|v| (v, Vec::new())));
    } else {
        let keys: Vec<&str> = path.split('.').collect();
        if !collect(value, &keys, "", &[], parents, &mut found) {
            return Err(t!("table-json-no-path", path = path.to_owned()));
        }
    }
    if !found.is_empty() && found.iter().all(|(v, p)| v.is_array() && p.is_empty()) {
        let mut rows: Vec<Vec<String>> = found
            .iter()
            .map(|(r, _)| r.as_array().unwrap().iter().map(scalar).collect())
            .collect();
        let header = rows.remove(0);
        return Ok(Table::from_rows(unique_names(header), &rows));
    }
    let mut names: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut flat: Vec<Vec<(String, String)>> = Vec::with_capacity(found.len());
    // Rows that are XML elements with text: the text is named after them.
    let own = path
        .rsplit('.')
        .next()
        .filter(|p| !p.is_empty())
        .unwrap_or("value");
    for (item, inherited) in found {
        let mut row = inherited;
        match item {
            Value::Object(m) => {
                for (k, v) in m {
                    let k = if k == "#text" { own } else { k.as_str() };
                    flatten(k, v, &mut row);
                }
            }
            other => row.push((t!("table-json-value-column"), scalar(other))),
        }
        for (k, _) in &row {
            if seen.insert(k.clone()) {
                names.push(k.clone());
            }
        }
        flat.push(row);
    }
    let index: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let rows: Vec<Vec<String>> = flat
        .into_iter()
        .map(|row| {
            let mut r = vec![String::new(); names.len()];
            for (k, v) in row {
                r[index[k.as_str()]] = v;
            }
            r
        })
        .collect();
    Ok(Table::from_rows(names.clone(), &rows))
}

/// Follows `keys` from `at`, collecting the rows at the end with the
/// fields inherited from the list items passed on the way. False when the
/// path leads nowhere.
fn collect<'a>(
    at: &'a Value,
    keys: &[&str],
    prefix: &str,
    inherited: &[(String, String)],
    parents: bool,
    out: &mut Vec<(&'a Value, Vec<(String, String)>)>,
) -> bool {
    let Some((key, rest)) = keys.split_first() else {
        match at {
            Value::Array(a) => out.extend(a.iter().map(|v| (v, inherited.to_vec()))),
            other => out.push((other, inherited.to_vec())),
        }
        return true;
    };
    match at {
        Value::Array(a) => {
            if let Ok(i) = key.parse::<usize>() {
                return a
                    .get(i)
                    .is_some_and(|v| collect(v, rest, prefix, inherited, parents, out));
            }
            let mut any = false;
            for item in a {
                let mut inh = inherited.to_vec();
                if parents && let Value::Object(m) = item {
                    for (k, v) in m {
                        if k != key && !v.is_array() {
                            let name = if prefix.is_empty() {
                                k.clone()
                            } else {
                                format!("{prefix}.{k}")
                            };
                            flatten(&name, v, &mut inh);
                        }
                    }
                }
                any |= collect(item, keys, prefix, &inh, parents, out);
            }
            any
        }
        Value::Object(m) => match m.get(*key) {
            Some(v) => {
                let p = if prefix.is_empty() {
                    (*key).to_owned()
                } else {
                    format!("{prefix}.{key}")
                };
                collect(v, rest, &p, inherited, parents, out)
            }
            None => false,
        },
        _ => false,
    }
}

/// XML as JSON: an element with only text is a string; otherwise an object
/// of its attributes and child elements (repeated children make a list)
/// with any text under `#text`. Returns the root's name and `{root: …}`.
fn xml_value(text: &str) -> Result<(String, Value), String> {
    let doc = roxmltree::Document::parse(text)
        .map_err(|e| t!("table-xml-unreadable", error = e.to_string()))?;
    let root = doc.root_element();
    let name = root.tag_name().name().to_owned();
    let mut top = serde_json::Map::new();
    top.insert(name.clone(), element_value(root));
    Ok((name, Value::Object(top)))
}

fn element_value(e: roxmltree::Node) -> Value {
    let children: Vec<roxmltree::Node> = e.children().filter(|c| c.is_element()).collect();
    let text: String = e
        .children()
        .filter(|c| c.is_text())
        .filter_map(|c| c.text())
        .collect::<String>();
    let text = text.trim();
    if children.is_empty() && e.attributes().len() == 0 {
        return Value::String(text.to_owned());
    }
    let mut m = serde_json::Map::new();
    for a in e.attributes() {
        m.insert(a.name().to_owned(), Value::String(a.value().to_owned()));
    }
    for c in children {
        let name = c.tag_name().name().to_owned();
        let v = element_value(c);
        // A child named like an attribute keeps both: the attribute gets `@`.
        if let Some(existing) = m.get_mut(&name) {
            if e.attribute(name.as_str()).is_some() && !existing.is_array() {
                let attr = m.remove(&name).unwrap();
                m.insert(format!("@{name}"), attr);
                m.insert(name, v);
                continue;
            }
            match existing {
                Value::Array(a) => a.push(v),
                other => {
                    let first = other.take();
                    *other = Value::Array(vec![first, v]);
                }
            }
        } else {
            m.insert(name, v);
        }
    }
    if !text.is_empty() {
        m.insert("#text".into(), Value::String(text.to_owned()));
    }
    Value::Object(m)
}

/// The list of rows: the value itself, or the only list inside an object
/// (looking one level at a time, like `{"data": {"items": [...]}}`).
fn find_list(v: &Value) -> Option<&[Value]> {
    match v {
        Value::Array(a) => Some(a),
        Value::Object(m) => {
            let lists: Vec<&Value> = m.values().filter(|v| v.is_array()).collect();
            if lists.len() == 1 {
                return find_list(lists[0]);
            }
            let objects: Vec<&Value> = m.values().filter(|v| v.is_object()).collect();
            if lists.is_empty() && objects.len() == 1 {
                return find_list(objects[0]);
            }
            None
        }
        _ => None,
    }
}

fn flatten(prefix: &str, v: &Value, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) if !m.is_empty() || prefix.is_empty() => {
            for (k, v) in m {
                // An XML element's own text is the element's column.
                let key = if k == "#text" && !prefix.is_empty() {
                    prefix.to_owned()
                } else if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        other => out.push((prefix.to_owned(), scalar(other))),
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Kind;

    #[test]
    fn reads_csv_with_types() {
        let src = Source::guess("id;name;score\n1;Ann;2.5\n2;Bob;3\n", None);
        let t = src
            .read(Input::Text("id;name;score\n1;Ann;2.5\n2;Bob;3\n"))
            .unwrap();
        assert_eq!(t.names(), ["id", "name", "score"]);
        assert_eq!(t.rows, 2);
        assert_eq!(t.columns[0].kind, Kind::Integer);
        assert_eq!(t.columns[2].kind, Kind::Decimal);
        let plain = Source::Csv {
            delimiter: ';',
            quote: Some('"'),
            header: false,
            detect_types: false,
        };
        let t = plain.read(Input::Text("1;2\n3;4\n")).unwrap();
        assert_eq!(t.names(), ["Column 1", "Column 2"]);
        assert_eq!(t.columns[0].kind, Kind::Text);
    }

    #[test]
    fn reads_json_lists() {
        let text = r#"{"meta":{"n":2},"data":[{"id":1,"who":{"name":"Ann","tags":["a"]},"ok":true},{"id":2,"extra":null}]}"#;
        let src = Source::guess(text, None);
        let t = src.read(Input::Text(text)).unwrap();
        assert_eq!(t.names(), ["id", "who.name", "who.tags", "ok", "extra"]);
        assert_eq!(t.row(0), ["1", "Ann", "[\"a\"]", "true", ""]);
        assert_eq!(t.columns[3].kind, Kind::Boolean);
        let at = Source::Json {
            parents: true,
            rows: "data".into(),
            detect_types: true,
        };
        assert_eq!(at.read(Input::Text(text)).unwrap().rows, 2);
        let lists = r#"[["a","b"],[1,2],[3,4]]"#;
        let t = Source::guess(lists, None).read(Input::Text(lists)).unwrap();
        assert_eq!(t.names(), ["a", "b"]);
        assert_eq!(t.rows, 2);
        let bad = Source::Json {
            parents: true,
            rows: "nope".into(),
            detect_types: true,
        };
        assert!(bad.read(Input::Text(text)).is_err());
    }

    #[test]
    fn spreadsheet_numbers() {
        assert_eq!(number_text(5.0), "5");
        assert_eq!(number_text(0.1 + 0.2), "0.3");
        assert_eq!(number_text(2.31), "2.31");
    }

    #[test]
    fn reads_a_workbook_we_wrote() {
        let mut t = Table::from_rows(
            vec!["Name".into(), "Count".into(), "When".into()],
            &[
                vec!["a".into(), "1".into(), "2026-01-02".into()],
                vec!["b".into(), "2.5".into(), "2026-01-03 10:30:00".into()],
            ],
        );
        t.detect_types();
        let bytes = crate::table::export::write_xlsx(&t, "Data").unwrap();
        assert_eq!(sheet_names(&bytes).unwrap(), ["Data"]);
        let src = Source::Excel {
            sheet: String::new(),
            header_row: 1,
            detect_types: true,
        };
        let back = src.read(Input::Bytes(&bytes)).unwrap();
        assert_eq!(back.names(), ["Name", "Count", "When"]);
        assert_eq!(back.row(0), ["a", "1", "2026-01-02"]);
        assert_eq!(back.row(1), ["b", "2.5", "2026-01-03 10:30:00"]);
        assert_eq!(back.columns[1].kind, Kind::Decimal);
        assert_eq!(back.columns[2].kind, Kind::DateTime);
        let none = Source::Excel {
            sheet: "Missing".into(),
            header_row: 1,
            detect_types: true,
        };
        assert!(none.read(Input::Bytes(&bytes)).is_err());
    }
}
