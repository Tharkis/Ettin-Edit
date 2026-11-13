//! Reading a table: delimited text (CSV, TSV…), Excel and OpenDocument
//! workbooks, and JSON lists of objects.

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
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rows: String,
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
            | Source::Json { detect_types, .. } => *detect_types,
        }
    }

    pub fn set_detect_types(&mut self, on: bool) {
        match self {
            Source::Csv { detect_types, .. }
            | Source::Excel { detect_types, .. }
            | Source::Json { detect_types, .. } => *detect_types = on,
        }
    }

    /// A guess at how to read `text`: JSON when it starts like JSON,
    /// otherwise delimited text with the delimiter worked out.
    pub fn guess(text: &str, extension: Option<&str>) -> Source {
        let start = text.trim_start();
        if extension == Some("json")
            || (start.starts_with(['[', '{']) && serde_json::from_str::<Value>(text).is_ok())
        {
            return Source::Json {
                rows: String::new(),
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
                    format!("\"{sheet}\"")
                };
                if *header_row == 0 {
                    t!("table-source-excel-no-header", sheet = sheet)
                } else {
                    t!("table-source-excel", sheet = sheet, row = *header_row)
                }
            }
            Source::Json { rows, .. } if rows.is_empty() => t!("table-source-json"),
            Source::Json { rows, .. } => t!("table-source-json-at", path = rows.clone()),
        }
    }

    /// Reads the table.
    pub fn read(&self, input: Input) -> Result<Table, String> {
        let mut table = match (self, input) {
            (Source::Excel { sheet, header_row, .. }, Input::Bytes(b)) => {
                read_workbook(b, sheet, *header_row)?
            }
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
            (Source::Json { rows, .. }, Input::Text(text)) => read_json(text, rows)?,
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

/// The sheet names of a workbook.
pub fn sheet_names(bytes: &[u8]) -> Result<Vec<String>, String> {
    let wb = calamine::open_workbook_auto_from_rs(Cursor::new(bytes.to_vec()))
        .map_err(|e| t!("table-workbook-unreadable", error = e.to_string()))?;
    Ok(calamine::Reader::sheet_names(&wb))
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

/// Reads a JSON list of rows. Objects inside a row become columns named
/// with dots (`address.city`); lists inside a row stay as JSON text.
fn read_json(text: &str, path: &str) -> Result<Table, String> {
    let value: Value =
        serde_json::from_str(text).map_err(|e| t!("table-json-unreadable", error = e.to_string()))?;
    let list = if path.trim().is_empty() {
        find_list(&value).ok_or_else(|| t!("table-json-no-list"))?
    } else {
        let mut at = &value;
        for key in path.trim().trim_start_matches("$.").split('.') {
            at = match at {
                Value::Object(m) => m.get(key),
                Value::Array(a) => key.parse::<usize>().ok().and_then(|i| a.get(i)),
                _ => None,
            }
            .ok_or_else(|| t!("table-json-no-path", path = path.to_owned()))?;
        }
        match at {
            Value::Array(a) => a.as_slice(),
            _ => return Err(t!("table-json-not-list", path = path.to_owned())),
        }
    };
    if !list.is_empty() && list.iter().all(Value::is_array) {
        let mut rows: Vec<Vec<String>> = list
            .iter()
            .map(|r| r.as_array().unwrap().iter().map(scalar).collect())
            .collect();
        let header = rows.remove(0);
        return Ok(Table::from_rows(unique_names(header), &rows));
    }
    let mut names: Vec<String> = Vec::new();
    let mut flat: Vec<Vec<(String, String)>> = Vec::with_capacity(list.len());
    for item in list {
        let mut row = Vec::new();
        match item {
            Value::Object(_) => flatten("", item, &mut row),
            other => row.push((t!("table-json-value-column"), scalar(other))),
        }
        for (k, _) in &row {
            if !names.contains(k) {
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
                let key = if prefix.is_empty() {
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
        let t = src.read(Input::Text("id;name;score\n1;Ann;2.5\n2;Bob;3\n")).unwrap();
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
            rows: "data".into(),
            detect_types: true,
        };
        assert_eq!(at.read(Input::Text(text)).unwrap().rows, 2);
        let lists = r#"[["a","b"],[1,2],[3,4]]"#;
        let t = Source::guess(lists, None).read(Input::Text(lists)).unwrap();
        assert_eq!(t.names(), ["a", "b"]);
        assert_eq!(t.rows, 2);
        let bad = Source::Json {
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
