//! Writing a table out: delimited text, Excel, JSON, JSON Lines and
//! Markdown.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{Kind, Table};

fn yes() -> bool {
    true
}
fn is_true(b: &bool) -> bool {
    *b
}
fn is_false(b: &bool) -> bool {
    !*b
}
fn comma() -> char {
    ','
}
fn sheet1() -> String {
    "Sheet1".into()
}

/// When delimited text puts a value in quotes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteMode {
    /// Only values with the delimiter, a quote, a line break or spaces at
    /// either end.
    #[default]
    Needed,
    All,
    /// Never: values are written as they are.
    Never,
}

impl QuoteMode {
    pub const ALL: [QuoteMode; 3] = [QuoteMode::Needed, QuoteMode::All, QuoteMode::Never];

    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            QuoteMode::Needed => "table-quote-needed",
            QuoteMode::All => "table-quote-all",
            QuoteMode::Never => "table-quote-never",
        })
    }
}

/// How the table is written out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum Export {
    Csv {
        #[serde(default = "comma")]
        delimiter: char,
        #[serde(default)]
        quote: QuoteMode,
        #[serde(default = "yes")]
        header: bool,
        /// Windows line ends.
        #[serde(default, skip_serializing_if = "is_false")]
        crlf: bool,
    },
    Excel {
        #[serde(default = "sheet1")]
        sheet: String,
    },
    /// A list of objects, one per row.
    Json {
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        pretty: bool,
        /// Numbers and true/false columns as JSON numbers and booleans.
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        typed: bool,
        /// Empty text values as `null` rather than `""`.
        #[serde(default, skip_serializing_if = "is_false")]
        empty_as_null: bool,
    },
    /// One JSON object per line.
    JsonLines {
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        typed: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        empty_as_null: bool,
    },
    Markdown,
}

impl Default for Export {
    fn default() -> Self {
        Export::Csv {
            delimiter: ',',
            quote: QuoteMode::Needed,
            header: true,
            crlf: false,
        }
    }
}

/// The formats, in the order the Export dialog lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Csv,
    Excel,
    Json,
    JsonLines,
    Markdown,
}

impl ExportKind {
    pub const ALL: [ExportKind; 5] = [
        ExportKind::Csv,
        ExportKind::Excel,
        ExportKind::Json,
        ExportKind::JsonLines,
        ExportKind::Markdown,
    ];

    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            ExportKind::Csv => "table-export-csv",
            ExportKind::Excel => "table-export-excel",
            ExportKind::Json => "table-export-json",
            ExportKind::JsonLines => "table-export-jsonl",
            ExportKind::Markdown => "table-export-markdown",
        })
    }

    pub fn default_export(self) -> Export {
        match self {
            ExportKind::Csv => Export::default(),
            ExportKind::Excel => Export::Excel { sheet: sheet1() },
            ExportKind::Json => Export::Json {
                pretty: true,
                typed: true,
                empty_as_null: false,
            },
            ExportKind::JsonLines => Export::JsonLines {
                typed: true,
                empty_as_null: false,
            },
            ExportKind::Markdown => Export::Markdown,
        }
    }
}

/// What an export gives: text for a tab, or a file's bytes.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    Text(String),
    Bytes(Vec<u8>),
}

impl Output {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Output::Text(t) => t.into_bytes(),
            Output::Bytes(b) => b,
        }
    }
}

impl Export {
    pub fn kind(&self) -> ExportKind {
        match self {
            Export::Csv { .. } => ExportKind::Csv,
            Export::Excel { .. } => ExportKind::Excel,
            Export::Json { .. } => ExportKind::Json,
            Export::JsonLines { .. } => ExportKind::JsonLines,
            Export::Markdown => ExportKind::Markdown,
        }
    }

    /// Whether the result is text (it can go to a tab or the clipboard).
    pub fn is_text(&self) -> bool {
        !matches!(self, Export::Excel { .. })
    }

    /// The usual file extension.
    pub fn extension(&self) -> &'static str {
        match self {
            Export::Csv {
                delimiter: '\t', ..
            } => "tsv",
            Export::Csv { .. } => "csv",
            Export::Excel { .. } => "xlsx",
            Export::Json { .. } => "json",
            Export::JsonLines { .. } => "jsonl",
            Export::Markdown => "md",
        }
    }

    /// The language of a tab holding the result.
    pub fn syntax(&self) -> &'static str {
        match self {
            Export::Csv { .. } => "CSV",
            Export::Excel { .. } => "Plain Text",
            Export::Json { .. } => "JSON",
            Export::JsonLines { .. } => "Plain Text",
            Export::Markdown => "Markdown",
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Export::Csv { delimiter, .. } => t!(
                "table-export-summary-csv",
                delimiter = crate::tools::csv::delimiter_name(*delimiter as u8)
            ),
            other => other.kind().label(),
        }
    }

    pub fn write(&self, t: &Table) -> Result<Output, String> {
        Ok(match self {
            Export::Csv {
                delimiter,
                quote,
                header,
                crlf,
            } => Output::Text(write_delimited(t, *delimiter, *quote, *header, *crlf)),
            Export::Excel { sheet } => Output::Bytes(write_xlsx(t, sheet)?),
            Export::Json {
                pretty,
                typed,
                empty_as_null,
            } => {
                let rows: Vec<Value> = (0..t.rows)
                    .map(|r| row_object(t, r, *typed, *empty_as_null))
                    .collect();
                let list = Value::Array(rows);
                let mut text = if *pretty {
                    serde_json::to_string_pretty(&list)
                } else {
                    serde_json::to_string(&list)
                }
                .map_err(|e| e.to_string())?;
                text.push('\n');
                Output::Text(text)
            }
            Export::JsonLines {
                typed,
                empty_as_null,
            } => {
                let mut out = String::new();
                for r in 0..t.rows {
                    out.push_str(&row_object(t, r, *typed, *empty_as_null).to_string());
                    out.push('\n');
                }
                Output::Text(out)
            }
            Export::Markdown => {
                let rows: Vec<Vec<String>> = (0..t.rows).map(|r| t.row(r)).collect();
                Output::Text(crate::tools::convert::markdown_table(&t.names(), &rows))
            }
        })
    }
}

fn write_delimited(
    t: &Table,
    delimiter: char,
    quote: QuoteMode,
    header: bool,
    crlf: bool,
) -> String {
    let eol = if crlf { "\r\n" } else { "\n" };
    let mut out = String::new();
    let field = |out: &mut String, v: &str| {
        let needs = v.contains(delimiter)
            || v.contains(['"', '\n', '\r'])
            || v.starts_with(' ')
            || v.ends_with(' ');
        let quoted = match quote {
            QuoteMode::All => true,
            QuoteMode::Needed => needs,
            QuoteMode::Never => false,
        };
        if quoted {
            out.push('"');
            out.push_str(&v.replace('"', "\"\""));
            out.push('"');
        } else {
            out.push_str(v);
        }
    };
    let line = |out: &mut String, values: &mut dyn Iterator<Item = &str>| {
        for (i, v) in values.enumerate() {
            if i > 0 {
                out.push(delimiter);
            }
            field(out, v);
        }
        out.push_str(eol);
    };
    if header {
        line(&mut out, &mut t.columns.iter().map(|c| c.name.as_str()));
    }
    for r in 0..t.rows {
        line(&mut out, &mut t.columns.iter().map(|c| c.cells.get(r)));
    }
    out
}

fn typed_value(kind: Kind, v: &str, typed: bool, empty_as_null: bool) -> Value {
    if v.is_empty() {
        return if empty_as_null || (typed && kind != Kind::Text) {
            Value::Null
        } else {
            Value::String(String::new())
        };
    }
    if typed {
        match kind {
            Kind::Integer => {
                if let Ok(i) = v.parse::<i64>() {
                    return Value::from(i);
                }
            }
            Kind::Decimal => {
                if let Some(n) = super::parse_decimal(v).and_then(serde_json::Number::from_f64) {
                    return Value::Number(n);
                }
            }
            Kind::Boolean => {
                if let Some(b) = super::parse_bool(v) {
                    return Value::Bool(b);
                }
            }
            _ => {}
        }
    }
    Value::String(v.to_owned())
}

fn row_object(t: &Table, r: usize, typed: bool, empty_as_null: bool) -> Value {
    let mut m = Map::new();
    for c in &t.columns {
        m.insert(
            c.name.clone(),
            typed_value(c.kind, c.cells.get(r), typed, empty_as_null),
        );
    }
    Value::Object(m)
}

/// The most rows a worksheet holds, header included.
const EXCEL_ROWS: usize = 1_048_576;

/// The table as an `.xlsx` workbook with one sheet: a bold header row,
/// numbers, dates and true/false as Excel values, and fitted widths.
pub fn write_xlsx(t: &Table, sheet: &str) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};
    if t.rows + 1 > EXCEL_ROWS {
        return Err(t!("table-too-many-rows-excel", rows = t.rows));
    }
    if t.columns.len() > 16_384 {
        return Err(t!("table-too-many-columns-excel"));
    }
    let err = |e: rust_xlsxwriter::XlsxError| e.to_string();
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let name = sheet.trim();
    if !name.is_empty() {
        ws.set_name(name).map_err(err)?;
    }
    let bold = Format::new().set_bold();
    let date = Format::new().set_num_format("yyyy-mm-dd");
    let datetime = Format::new().set_num_format("yyyy-mm-dd hh:mm:ss");
    for (c, col) in t.columns.iter().enumerate() {
        let c = c as u16;
        ws.write_string_with_format(0, c, &col.name, &bold)
            .map_err(err)?;
        for (r, v) in col.cells.iter().enumerate() {
            let r = r as u32 + 1;
            if v.is_empty() {
                continue;
            }
            let done = match col.kind {
                Kind::Integer | Kind::Decimal => match super::parse_decimal(v) {
                    // Past 15 digits Excel would round it: keep the text.
                    Some(x) if v.trim_start_matches('-').len() <= 15 => {
                        ws.write_number(r, c, x).map_err(err)?;
                        true
                    }
                    _ => false,
                },
                Kind::Boolean => match super::parse_bool(v) {
                    Some(b) => {
                        ws.write_boolean(r, c, b).map_err(err)?;
                        true
                    }
                    None => false,
                },
                Kind::Date | Kind::DateTime => match super::parse_iso_datetime(v) {
                    Some(at) => {
                        use chrono::{Datelike, Timelike};
                        let dt = ExcelDateTime::from_ymd(
                            at.year() as u16,
                            at.month() as u8,
                            at.day() as u8,
                        )
                        .and_then(|d| d.and_hms(at.hour() as u16, at.minute() as u8, at.second()));
                        match dt {
                            Ok(dt) => {
                                let f = if col.kind == Kind::Date {
                                    &date
                                } else {
                                    &datetime
                                };
                                ws.write_datetime_with_format(r, c, &dt, f).map_err(err)?;
                                true
                            }
                            Err(_) => false,
                        }
                    }
                    None => false,
                },
                Kind::Text => false,
            };
            if !done {
                ws.write_string(r, c, v).map_err(err)?;
            }
        }
    }
    ws.autofit();
    wb.save_to_buffer().map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Table {
        let mut t = Table::from_rows(
            vec!["id".into(), "name".into(), "score".into(), "ok".into()],
            &[
                vec!["1".into(), "Ann, B".into(), "2.5".into(), "true".into()],
                vec!["2".into(), "".into(), "".into(), "false".into()],
            ],
        );
        t.detect_types();
        t
    }

    #[test]
    fn delimited() {
        let t = table();
        let Output::Text(csv) = Export::default().write(&t).unwrap() else {
            panic!()
        };
        assert_eq!(csv, "id,name,score,ok\n1,\"Ann, B\",2.5,true\n2,,,false\n");
        let pipe = Export::Csv {
            delimiter: '|',
            quote: QuoteMode::All,
            header: false,
            crlf: true,
        };
        let Output::Text(p) = pipe.write(&t).unwrap() else {
            panic!()
        };
        assert_eq!(
            p,
            "\"1\"|\"Ann, B\"|\"2.5\"|\"true\"\r\n\"2\"|\"\"|\"\"|\"false\"\r\n"
        );
    }

    #[test]
    fn json_is_typed() {
        let t = table();
        let Output::Text(j) = ExportKind::JsonLines.default_export().write(&t).unwrap() else {
            panic!()
        };
        assert_eq!(
            j,
            "{\"id\":1,\"name\":\"Ann, B\",\"score\":2.5,\"ok\":true}\n{\"id\":2,\"name\":\"\",\"score\":null,\"ok\":false}\n"
        );
        let plain = Export::JsonLines {
            typed: false,
            empty_as_null: true,
        };
        let Output::Text(j) = plain.write(&t).unwrap() else {
            panic!()
        };
        assert!(j.starts_with("{\"id\":\"1\""));
        assert!(j.contains("\"name\":null"));
    }

    #[test]
    fn markdown_and_excel() {
        let t = table();
        let Output::Text(md) = Export::Markdown.write(&t).unwrap() else {
            panic!()
        };
        assert!(md.starts_with("| id"));
        let Output::Bytes(x) = (Export::Excel { sheet: "S".into() }).write(&t).unwrap() else {
            panic!()
        };
        assert!(x.starts_with(b"PK"));
    }
}
