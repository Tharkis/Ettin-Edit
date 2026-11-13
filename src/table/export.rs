//! Writing a table out: delimited text, Excel, JSON, JSON Lines, YAML,
//! XML, HL7 v2 and Markdown. Dotted column names (`clinic.address.city`)
//! become nested objects or elements when asked.

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
        /// Dotted column names as nested objects.
        #[serde(default, skip_serializing_if = "is_false")]
        nested: bool,
    },
    /// One JSON object per line.
    JsonLines {
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        typed: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        empty_as_null: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        nested: bool,
    },
    /// A YAML list of mappings.
    Yaml {
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        typed: bool,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        nested: bool,
    },
    /// An XML document: a `root` element holding one `row` element per row.
    /// Dotted names nest elements; a name part starting with `@` is an
    /// attribute.
    Xml {
        #[serde(default = "rows_name")]
        root: String,
        #[serde(default = "row_name")]
        row: String,
        /// Leave out elements for empty values.
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        skip_empty: bool,
    },
    /// HL7 v2 messages, one per row, from a template with `{Column}`
    /// placeholders. An empty template is made from columns named like
    /// `PID.5.1`.
    Hl7 {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        template: String,
    },
    Markdown,
}

fn rows_name() -> String {
    "rows".into()
}
fn row_name() -> String {
    "row".into()
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
    Yaml,
    Xml,
    Hl7,
    Markdown,
}

impl ExportKind {
    pub const ALL: [ExportKind; 8] = [
        ExportKind::Csv,
        ExportKind::Excel,
        ExportKind::Json,
        ExportKind::JsonLines,
        ExportKind::Yaml,
        ExportKind::Xml,
        ExportKind::Hl7,
        ExportKind::Markdown,
    ];

    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            ExportKind::Csv => "table-export-csv",
            ExportKind::Excel => "table-export-excel",
            ExportKind::Json => "table-export-json",
            ExportKind::JsonLines => "table-export-jsonl",
            ExportKind::Yaml => "table-export-yaml",
            ExportKind::Xml => "table-export-xml",
            ExportKind::Hl7 => "table-export-hl7",
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
                nested: false,
            },
            ExportKind::JsonLines => Export::JsonLines {
                typed: true,
                empty_as_null: false,
                nested: false,
            },
            ExportKind::Yaml => Export::Yaml {
                typed: true,
                nested: true,
            },
            ExportKind::Xml => Export::Xml {
                root: rows_name(),
                row: row_name(),
                skip_empty: true,
            },
            ExportKind::Hl7 => Export::Hl7 {
                template: String::new(),
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
            Export::Yaml { .. } => ExportKind::Yaml,
            Export::Xml { .. } => ExportKind::Xml,
            Export::Hl7 { .. } => ExportKind::Hl7,
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
            Export::Yaml { .. } => "yaml",
            Export::Xml { .. } => "xml",
            Export::Hl7 { .. } => "hl7",
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
            Export::Yaml { .. } => "YAML",
            Export::Xml { .. } => "XML",
            Export::Hl7 { .. } => "HL7",
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
                nested,
            } => {
                let rows: Vec<Value> = (0..t.rows)
                    .map(|r| row_value(t, r, *typed, *empty_as_null, *nested))
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
                nested,
            } => {
                let mut out = String::new();
                for r in 0..t.rows {
                    out.push_str(&row_value(t, r, *typed, *empty_as_null, *nested).to_string());
                    out.push('\n');
                }
                Output::Text(out)
            }
            Export::Yaml { typed, nested } => {
                let rows: Vec<Value> = (0..t.rows)
                    .map(|r| row_value(t, r, *typed, false, *nested))
                    .collect();
                Output::Text(serde_yaml_ng::to_string(&rows).map_err(|e| e.to_string())?)
            }
            Export::Xml {
                root,
                row,
                skip_empty,
            } => Output::Text(write_xml(t, root, row, *skip_empty)),
            Export::Hl7 { template } => {
                let template = if template.trim().is_empty() {
                    hl7_template(t)
                } else {
                    template.clone()
                };
                let mut out = String::new();
                for r in 0..t.rows {
                    let value = |name: &str| -> Option<String> {
                        if name == "#" {
                            return Some((r + 1).to_string());
                        }
                        t.find(name).map(|c| t.get(r, c).to_owned())
                    };
                    let msg = super::hl7::fill(&template, &value)
                        .map_err(|e| t!("table-row-failed", row = r + 1, message = e))?;
                    out.push_str(&msg);
                    out.push('\r');
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

fn row_value(t: &Table, r: usize, typed: bool, empty_as_null: bool, nested: bool) -> Value {
    let mut m = Map::new();
    for c in &t.columns {
        let v = typed_value(c.kind, c.cells.get(r), typed, empty_as_null);
        if nested && c.name.contains('.') {
            insert_nested(&mut m, &c.name, v);
        } else {
            m.insert(c.name.clone(), v);
        }
    }
    Value::Object(m)
}

/// Puts `v` at a dotted path, making objects on the way. When a part on the
/// way already holds a plain value, `v` goes in under its full dotted name
/// instead, so nothing is lost.
fn insert_nested(m: &mut Map<String, Value>, name: &str, v: Value) {
    let parts: Vec<&str> = name.split('.').collect();
    let mut at = &mut *m;
    for (i, p) in parts.iter().enumerate() {
        if i + 1 == parts.len() {
            at.insert((*p).to_owned(), v);
            return;
        }
        let entry = at
            .entry((*p).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            // `a` is a value and `a.b` wants it to be an object.
            m.insert(name.to_owned(), v);
            return;
        }
        at = entry.as_object_mut().expect("checked");
    }
}

/// An element name made from a column name: letters, digits, `-`, `_` and
/// `.`; anything else becomes `_`, and a name can't start with a digit.
fn xml_name(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
        out.insert(0, '_');
    }
    out
}

fn xml_escape(s: &str, attr: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// An element being built: attributes, own text and children in order.
#[derive(Default)]
struct Elem {
    attrs: Vec<(String, String)>,
    text: Option<String>,
    children: Vec<(String, Elem)>,
}

impl Elem {
    fn child(&mut self, name: &str) -> &mut Elem {
        let i = match self.children.iter().position(|(n, _)| n == name) {
            Some(i) => i,
            None => {
                self.children.push((name.to_owned(), Elem::default()));
                self.children.len() - 1
            }
        };
        &mut self.children[i].1
    }

    fn write(&self, name: &str, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        out.push_str(&pad);
        out.push('<');
        out.push_str(name);
        for (k, v) in &self.attrs {
            out.push_str(&format!(" {k}=\"{}\"", xml_escape(v, true)));
        }
        match (&self.text, self.children.is_empty()) {
            (None, true) => out.push_str("/>\n"),
            (Some(t), true) => out.push_str(&format!(">{}</{name}>\n", xml_escape(t, false))),
            (text, false) => {
                out.push_str(">\n");
                if let Some(t) = text {
                    out.push_str(&format!("{pad}  {}\n", xml_escape(t, false)));
                }
                for (n, c) in &self.children {
                    c.write(n, depth + 1, out);
                }
                out.push_str(&format!("{pad}</{name}>\n"));
            }
        }
    }
}

fn write_xml(t: &Table, root: &str, row: &str, skip_empty: bool) -> String {
    let root = xml_name(root.trim());
    let row = xml_name(row.trim());
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!("<{root}>\n"));
    for r in 0..t.rows {
        let mut e = Elem::default();
        for c in &t.columns {
            let v = c.cells.get(r);
            if skip_empty && v.is_empty() {
                continue;
            }
            let parts: Vec<&str> = c.name.split('.').filter(|p| !p.is_empty()).collect();
            let Some((last, path)) = parts.split_last() else {
                continue;
            };
            let mut at = &mut e;
            for p in path {
                at = at.child(&xml_name(p));
            }
            if let Some(attr) = last.strip_prefix('@') {
                at.attrs.push((xml_name(attr), v.to_owned()));
            } else {
                at.child(&xml_name(last)).text = Some(v.to_owned());
            }
        }
        e.write(&row, 1, &mut out);
    }
    out.push_str(&format!("</{root}>\n"));
    out
}

/// A template made from columns named like HL7 fields (`PID.5.1`,
/// `OBX[2].5`, `PID.3[2].1`): each placeholder where its field is. Without
/// such columns, a starting point to fill in.
pub fn hl7_template(t: &Table) -> String {
    // (segment, occurrence) → field → repeat → component → subcomponent → column
    type Subs = std::collections::BTreeMap<usize, String>;
    type Comps = std::collections::BTreeMap<usize, Subs>;
    type Reps = std::collections::BTreeMap<usize, Comps>;
    type Fields = std::collections::BTreeMap<usize, Reps>;
    let re = regex::Regex::new(
        r"^([A-Z][A-Z0-9]{2})(?:\[(\d+)\])?\.(\d+)(?:\[(\d+)\])?(?:\.(\d+))?(?:\.(\d+))?$",
    )
    .expect("valid");
    let mut segments: Vec<((String, usize), Fields)> = Vec::new();
    for c in &t.columns {
        let Some(m) = re.captures(&c.name) else {
            continue;
        };
        let num = |i: usize, d: usize| m.get(i).and_then(|x| x.as_str().parse().ok()).unwrap_or(d);
        let key = (m[1].to_owned(), num(2, 1));
        let field = num(3, 0);
        if key.0 == "MSH" && field <= 2 {
            continue;
        }
        let i = match segments.iter().position(|(k, _)| *k == key) {
            Some(i) => i,
            None => {
                segments.push((key, Fields::new()));
                segments.len() - 1
            }
        };
        segments[i]
            .1
            .entry(field)
            .or_default()
            .entry(num(4, 1))
            .or_default()
            .entry(num(5, 1))
            .or_default()
            .insert(num(6, 1), format!("{{{}}}", c.name));
    }
    if segments.is_empty() {
        let names = t.names();
        let col = |i: usize| names.get(i).map(|n| format!("{{{n}}}")).unwrap_or_default();
        return format!(
            "MSH|^~\\&|ETTIN|||||||ADT^A08^ADT_A01|MSG{{#}}|P|2.5.1\nEVN|A08\nPID|1||{}||{}\n",
            col(0),
            col(1)
        );
    }
    let join = |parts: Vec<String>, sep: char| -> String {
        parts
            .join(&sep.to_string())
            .trim_end_matches(sep)
            .to_owned()
    };
    let numbered =
        |max: usize, get: &dyn Fn(usize) -> String| -> Vec<String> { (1..=max).map(get).collect() };
    let mut out = String::new();
    if !segments.iter().any(|((id, _), _)| id == "MSH") {
        out.push_str("MSH|^~\\&|ETTIN|||||||ADT^A08^ADT_A01|MSG{#}|P|2.5.1\n");
    }
    for ((id, _), fields) in &segments {
        let max_field = *fields.keys().max().unwrap_or(&0);
        let first = if id == "MSH" { 3 } else { 1 };
        let mut line = if id == "MSH" {
            "MSH|^~\\&".to_owned()
        } else {
            id.clone()
        };
        for f in first..=max_field {
            line.push('|');
            let Some(reps) = fields.get(&f) else {
                continue;
            };
            let rep_max = *reps.keys().max().unwrap_or(&1);
            let reps_text = numbered(rep_max, &|r| {
                let Some(comps) = reps.get(&r) else {
                    return String::new();
                };
                let comp_max = *comps.keys().max().unwrap_or(&1);
                join(
                    numbered(comp_max, &|c| {
                        let Some(subs) = comps.get(&c) else {
                            return String::new();
                        };
                        let sub_max = *subs.keys().max().unwrap_or(&1);
                        join(
                            numbered(sub_max, &|s| subs.get(&s).cloned().unwrap_or_default()),
                            '&',
                        )
                    }),
                    '^',
                )
            });
            line.push_str(&join(reps_text, '~'));
        }
        out.push_str(line.trim_end_matches('|'));
        out.push('\n');
    }
    out
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
            nested: false,
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
