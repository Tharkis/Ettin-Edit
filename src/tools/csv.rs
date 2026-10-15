//! Reading delimited text (CSV, TSV and friends) for CsvQuery: guessing the
//! delimiter, quote character and header row, splitting the text into rows,
//! guessing each column's type, and writing rows back out as CSV.

use serde::{Deserialize, Serialize};

use super::ToolError;

/// Delimiters tried when guessing, most common first.
const DELIMITERS: [u8; 4] = *b",;\t|";
/// How much of the text is looked at when guessing the format.
const SAMPLE_BYTES: usize = 64 * 1024;
const SAMPLE_ROWS: usize = 200;

/// How a delimited file is laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CsvFormat {
    pub delimiter: u8,
    /// `None` when fields are never quoted.
    pub quote: Option<u8>,
    /// The first row names the columns.
    pub header: bool,
}

impl Default for CsvFormat {
    fn default() -> Self {
        Self {
            delimiter: b',',
            quote: Some(b'"'),
            header: true,
        }
    }
}

/// What a column holds, which decides how it is stored and compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColType {
    Text,
    Integer,
    Real,
}

impl ColType {
    pub fn sql(self) -> &'static str {
        match self {
            ColType::Text => "TEXT",
            ColType::Integer => "INTEGER",
            ColType::Real => "REAL",
        }
    }

    pub fn label(self) -> String {
        match self {
            ColType::Text => t!("column-type-text"),
            ColType::Integer => t!("column-type-integer"),
            ColType::Real => t!("column-type-decimal"),
        }
    }
}

/// A delimited file split into rows. Every row has one field per column.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Name of a delimiter for menus and messages.
pub fn delimiter_name(d: u8) -> String {
    match d {
        b',' => t!("delimiter-comma"),
        b';' => t!("delimiter-semicolon"),
        b'\t' => t!("delimiter-tab"),
        b'|' => t!("delimiter-pipe"),
        b' ' => t!("delimiter-space"),
        other => format!("'{}'", other as char),
    }
}

/// Guesses the layout of `text`. `extension` (lowercase, without the dot)
/// settles ties: a `.tsv` file is tab-separated.
pub fn detect(text: &str, extension: Option<&str>) -> CsvFormat {
    let sample = sample(text);
    let quote = detect_quote(sample);
    let preferred = match extension {
        Some("tsv" | "tab") => Some(b'\t'),
        Some("psv") => Some(b'|'),
        _ => None,
    };
    let mut best: Option<(f64, u8)> = None;
    for &d in &DELIMITERS {
        let mut score = delimiter_score(sample, d, quote);
        if Some(d) == preferred && score > 0.0 {
            score += 0.5;
        }
        if score > best.map_or(0.0, |b| b.0) {
            best = Some((score, d));
        }
    }
    let delimiter = best.map_or(preferred.unwrap_or(b','), |b| b.1);
    let mut format = CsvFormat {
        delimiter,
        quote,
        header: true,
    };
    if let Ok(rows) = read_rows(sample, &format, SAMPLE_ROWS) {
        format.header = detect_header(&rows, delimiter);
    }
    format
}

/// The start of `text`, cut at a line end.
fn sample(text: &str) -> &str {
    if text.len() <= SAMPLE_BYTES {
        return text;
    }
    let cut = super::floor_char_boundary(text, SAMPLE_BYTES);
    match text[..cut].rfind('\n') {
        Some(i) => &text[..i],
        None => &text[..cut],
    }
}

/// `"` unless single quotes clearly wrap fields instead.
fn detect_quote(sample: &str) -> Option<u8> {
    let starts = |q: char| {
        sample
            .lines()
            .take(SAMPLE_ROWS)
            .map(|line| {
                let mut n = usize::from(line.starts_with(q));
                for &d in &DELIMITERS {
                    n += line.matches(&format!("{}{q}", d as char)).count();
                }
                n
            })
            .sum::<usize>()
    };
    let (double, single) = (starts('"'), starts('\''));
    if single > double * 2 && single >= 2 {
        Some(b'\'')
    } else {
        Some(b'"')
    }
}

/// How well `d` splits the sample into rows of the same length: the share of
/// rows with the most common field count, weighted towards more fields.
fn delimiter_score(sample: &str, d: u8, quote: Option<u8>) -> f64 {
    let format = CsvFormat {
        delimiter: d,
        quote,
        header: false,
    };
    let Ok(rows) = read_rows(sample, &format, SAMPLE_ROWS) else {
        return 0.0;
    };
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|r| !(r.len() == 1 && r[0].is_empty()))
        .collect();
    if rows.is_empty() {
        return 0.0;
    }
    let mut counts = std::collections::HashMap::new();
    for r in &rows {
        *counts.entry(r.len()).or_insert(0usize) += 1;
    }
    let (&fields, &n) = counts
        .iter()
        .max_by_key(|&(&fields, &n)| (n, fields))
        .expect("rows is not empty");
    if fields < 2 {
        return 0.0;
    }
    let consistency = n as f64 / rows.len() as f64;
    consistency * (1.0 + (fields as f64).ln() / 10.0)
}

/// Splits up to `limit` rows, keeping ragged rows as they are.
fn read_rows(text: &str, format: &CsvFormat, limit: usize) -> Result<Vec<Vec<String>>, ToolError> {
    let mut builder = csv::ReaderBuilder::new();
    builder
        .delimiter(format.delimiter)
        .has_headers(false)
        .flexible(true);
    match format.quote {
        Some(q) => builder.quote(q),
        None => builder.quoting(false),
    };
    let mut reader = builder.from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for record in reader.records().take(limit) {
        let record = record.map_err(|e| {
            let offset = e.position().map(|p| p.byte() as usize);
            let message = format!("Couldn't read the file as delimited text: {e}");
            match offset {
                Some(o) => ToolError::at(message, o),
                None => ToolError::new(message),
            }
        })?;
        rows.push(record.iter().map(str::to_owned).collect());
    }
    Ok(rows)
}

/// The first row is a header when its fields are distinct names and at
/// least one column below it holds numbers while its first field doesn't.
/// With text everywhere, a first row of distinct, non-empty values that
/// don't come up again in their columns counts as a header too.
fn detect_header(rows: &[Vec<String>], delimiter: u8) -> bool {
    let Some((first, rest)) = rows.split_first() else {
        return true;
    };
    if rest.is_empty() {
        return true;
    }
    let mut seen = std::collections::HashSet::new();
    if first
        .iter()
        .any(|f| f.trim().is_empty() || !seen.insert(f.trim()))
    {
        return false;
    }
    let mut numeric_below = false;
    for (c, name) in first.iter().enumerate() {
        let values: Vec<&str> = rest
            .iter()
            .filter_map(|r| r.get(c))
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        let numeric = !values.is_empty()
            && values
                .iter()
                .all(|v| value_type(v, delimiter) != ColType::Text);
        if numeric {
            if value_type(name.trim(), delimiter) != ColType::Text {
                return false;
            }
            numeric_below = true;
        }
    }
    numeric_below
        || first.iter().enumerate().all(|(c, name)| {
            !rest
                .iter()
                .any(|r| r.get(c).is_some_and(|v| v.trim() == name.trim()))
        })
}

/// Splits `text` into a table, padding short rows so every row has a field
/// for every column.
pub fn parse(text: &str, format: &CsvFormat) -> Result<Table, ToolError> {
    let mut rows = read_rows(text, format, usize::MAX)?;
    // A trailing empty line isn't a row.
    while rows.last().is_some_and(|r| r.len() == 1 && r[0].is_empty()) {
        rows.pop();
    }
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    let names: Vec<String> = if format.header && !rows.is_empty() {
        rows.remove(0)
    } else {
        Vec::new()
    };
    let width = width.max(names.len());
    let columns = column_names(&names, width);
    for row in &mut rows {
        row.resize(width, String::new());
    }
    Ok(Table { columns, rows })
}

/// Unique, non-empty column names: `col1`, `col2`... where the header has none.
fn column_names(header: &[String], width: usize) -> Vec<String> {
    let mut used = std::collections::HashSet::new();
    (0..width)
        .map(|i| {
            let base = header
                .get(i)
                .map(|h| h.trim())
                .filter(|h| !h.is_empty())
                .map_or_else(|| format!("col{}", i + 1), str::to_owned);
            let mut name = base.clone();
            let mut n = 2;
            while !used.insert(name.to_lowercase()) {
                name = format!("{base}_{n}");
                n += 1;
            }
            name
        })
        .collect()
}

/// What a single value looks like. Numbers with leading zeros (`007`, zip
/// codes, ids) stay text so they keep their zeros. With a delimiter other than
/// a comma, a decimal comma (`3,5`) counts as a number.
pub fn value_type(v: &str, delimiter: u8) -> ColType {
    let digits = v.strip_prefix(['-', '+']).unwrap_or(v);
    if digits.is_empty() {
        return ColType::Text;
    }
    let leading_zero = digits.len() > 1 && digits.starts_with('0') && !digits.starts_with("0.");
    if leading_zero && !(delimiter != b',' && digits.starts_with("0,")) {
        return ColType::Text;
    }
    if digits.bytes().all(|b| b.is_ascii_digit()) {
        return if v.parse::<i64>().is_ok() {
            ColType::Integer
        } else {
            ColType::Real
        };
    }
    if parse_real(v, delimiter).is_some() {
        ColType::Real
    } else {
        ColType::Text
    }
}

/// A decimal number, allowing a decimal comma when the delimiter isn't one.
/// Words like `inf` and `NaN` are not numbers here.
pub fn parse_real(v: &str, delimiter: u8) -> Option<f64> {
    if !v
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b',' | b'-' | b'+' | b'e' | b'E'))
        || !v.bytes().any(|b| b.is_ascii_digit())
    {
        return None;
    }
    if let Ok(x) = v.parse::<f64>() {
        return Some(x);
    }
    if delimiter != b',' && v.matches(',').count() == 1 && !v.contains('.') {
        return v.replace(',', ".").parse().ok();
    }
    None
}

/// The narrowest type that fits every non-empty value of each column.
pub fn infer_types(table: &Table, delimiter: u8) -> Vec<ColType> {
    (0..table.columns.len())
        .map(|c| {
            let mut ty = None;
            for row in &table.rows {
                let v = row[c].trim();
                if v.is_empty() {
                    continue;
                }
                let t = value_type(v, delimiter);
                ty = Some(match (ty, t) {
                    (_, ColType::Text) => return ColType::Text,
                    (None, t) => t,
                    (Some(ColType::Integer), ColType::Integer) => ColType::Integer,
                    _ => ColType::Real,
                });
            }
            ty.unwrap_or(ColType::Text)
        })
        .collect()
}

/// Writes rows as CSV (comma-separated, quoted where needed, `\n` line ends).
pub fn write_csv(columns: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    let mut line = |fields: &mut dyn Iterator<Item = &str>| {
        let mut first = true;
        for f in fields {
            if !first {
                out.push(',');
            }
            first = false;
            if f.contains([',', '"', '\n', '\r']) || f.starts_with(' ') || f.ends_with(' ') {
                out.push('"');
                out.push_str(&f.replace('"', "\"\""));
                out.push('"');
            } else {
                out.push_str(f);
            }
        }
        out.push('\n');
    };
    line(&mut columns.iter().map(String::as_str));
    for row in rows {
        line(&mut row.iter().map(String::as_str));
    }
    out
}

/// Writes rows tab-separated, for pasting into a spreadsheet. Tabs and line
/// breaks inside values become spaces.
pub fn write_tsv(columns: &[String], rows: &[Vec<String>]) -> String {
    let clean = |f: &str| f.replace(['\t', '\n', '\r'], " ");
    let mut out = String::new();
    for fields in std::iter::once(columns).chain(rows.iter().map(Vec::as_slice)) {
        let line: Vec<String> = fields.iter().map(|f| clean(f)).collect();
        out.push_str(&line.join("\t"));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_layouts() {
        let f = detect("name,age\nann,31\nbob,42\n", None);
        assert_eq!((f.delimiter, f.header), (b',', true));
        let f = detect("name;price\nwidget;3,50\ngadget;12,00\n", None);
        assert_eq!((f.delimiter, f.header), (b';', true));
        let f = detect("a\tb\tc\n1\t2\t3\n", Some("tsv"));
        assert_eq!(f.delimiter, b'\t');
        let f = detect("x|y\n1|2\n3|4\n", None);
        assert_eq!(f.delimiter, b'|');
    }

    #[test]
    fn quoted_delimiters_do_not_split() {
        let f = detect("id,note\n1,\"a, b; c\"\n2,\"d; e\"\n", None);
        assert_eq!(f.delimiter, b',');
        let t = parse("id,note\n1,\"a, b\"\n", &f).unwrap();
        assert_eq!(t.rows, vec![vec!["1".to_string(), "a, b".to_string()]]);
    }

    #[test]
    fn header_detection() {
        // Numbers under names: a header.
        assert!(detect("city,pop\nOslo,700000\nBergen,290000\n", None).header);
        // Numbers all the way down: no header.
        assert!(!detect("1,2\n3,4\n5,6\n", None).header);
        // A repeated first value: no header.
        assert!(!detect("red,big\nred,small\nblue,big\n", None).header);
        // Distinct names over text that never repeats them: a header.
        assert!(detect("first,last\nAda,Lovelace\nAlan,Turing\n", None).header);
    }

    #[test]
    fn parse_pads_ragged_rows_and_names_columns() {
        let f = CsvFormat {
            header: true,
            ..CsvFormat::default()
        };
        let t = parse("a,,a\n1,2,3,4\n5\n\n", &f).unwrap();
        assert_eq!(t.columns, vec!["a", "col2", "a_2", "col4"]);
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[1], vec!["5", "", "", ""]);
    }

    #[test]
    fn types() {
        let f = CsvFormat::default();
        let t = parse("id,zip,price,name,empty\n1,0123,1.5,x,\n2,0456,2,y,\n", &f).unwrap();
        assert_eq!(
            infer_types(&t, b','),
            vec![
                ColType::Integer,
                ColType::Text,
                ColType::Real,
                ColType::Text,
                ColType::Text
            ]
        );
        assert_eq!(value_type("3,5", b';'), ColType::Real);
        assert_eq!(value_type("3,5", b','), ColType::Text);
        assert_eq!(value_type("0,5", b';'), ColType::Real);
        assert_eq!(value_type("inf", b','), ColType::Text);
        assert_eq!(value_type("1e3", b','), ColType::Real);
        assert_eq!(value_type("99999999999999999999", b','), ColType::Real);
    }

    #[test]
    fn writes_csv_and_tsv() {
        let cols = vec!["a".to_string(), "b".to_string()];
        let rows = vec![vec!["x,y".to_string(), "say \"hi\"".to_string()]];
        assert_eq!(write_csv(&cols, &rows), "a,b\n\"x,y\",\"say \"\"hi\"\"\"\n");
        assert_eq!(write_tsv(&cols, &rows), "a\tb\nx,y\tsay \"hi\"\n");
    }
}
