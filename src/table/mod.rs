//! Tables (2.7): structured data held as typed columns, changed by a list of
//! steps that can be replayed on the next file. The table tab shows one; a
//! recipe's `table` step runs one without a window.
//!
//! Values are kept as text in their column's canonical form (dates as
//! `YYYY-MM-DD`, decimals with a point), so a value that doesn't fit its
//! column's type is never lost: it stays as it was and counts as a problem.

pub mod export;
pub mod hl7;
pub mod import;
pub mod job;
pub mod pattern;
pub mod steps;

use std::cmp::Ordering;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub use job::TableJob;
pub use steps::TableStep;

/// What a column holds. It decides how values sort, compare and export.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Text,
    Integer,
    Decimal,
    Date,
    DateTime,
    Boolean,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Text,
        Kind::Integer,
        Kind::Decimal,
        Kind::Date,
        Kind::DateTime,
        Kind::Boolean,
    ];

    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            Kind::Text => "table-type-text",
            Kind::Integer => "table-type-integer",
            Kind::Decimal => "table-type-decimal",
            Kind::Date => "table-type-date",
            Kind::DateTime => "table-type-datetime",
            Kind::Boolean => "table-type-boolean",
        })
    }

    /// The short tag on a column header.
    pub fn badge(self) -> &'static str {
        match self {
            Kind::Text => "abc",
            Kind::Integer => "123",
            Kind::Decimal => "1.2",
            Kind::Date => "date",
            Kind::DateTime => "time",
            Kind::Boolean => "y/n",
        }
    }

    pub fn is_number(self) -> bool {
        matches!(self, Kind::Integer | Kind::Decimal)
    }

    /// Whether `v` (not empty) is in this kind's canonical form.
    pub fn fits(self, v: &str) -> bool {
        match self {
            Kind::Text => true,
            Kind::Integer => is_integer(v),
            Kind::Decimal => parse_decimal(v).is_some(),
            Kind::Date => parse_iso_date(v).is_some(),
            Kind::DateTime => parse_iso_datetime(v).is_some(),
            Kind::Boolean => parse_bool(v).is_some(),
        }
    }

    /// Orders two values of a column of this kind. Empty values go last,
    /// then values that don't fit the kind (in text order).
    pub fn compare(self, a: &str, b: &str) -> Ordering {
        match (a.is_empty(), b.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            _ => {}
        }
        let typed = match self {
            Kind::Text => return crate::tools::fields::natural_cmp(a, b),
            Kind::Integer | Kind::Decimal => match (parse_decimal(a), parse_decimal(b)) {
                (Some(x), Some(y)) => Some(x.total_cmp(&y)),
                _ => None,
            },
            Kind::Date | Kind::DateTime => match (parse_iso_datetime(a), parse_iso_datetime(b)) {
                (Some(x), Some(y)) => Some(x.cmp(&y)),
                _ => None,
            },
            Kind::Boolean => match (parse_bool(a), parse_bool(b)) {
                (Some(x), Some(y)) => Some(x.cmp(&y)),
                _ => None,
            },
        };
        match typed {
            Some(o) => o,
            None => {
                let (fa, fb) = (self.fits(a), self.fits(b));
                fb.cmp(&fa)
                    .then_with(|| crate::tools::fields::natural_cmp(a, b))
            }
        }
    }
}

pub fn is_integer(v: &str) -> bool {
    let digits = v.strip_prefix(['-', '+']).unwrap_or(v);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && v.parse::<i64>().is_ok()
}

/// A number written with a decimal point (and maybe an exponent).
pub fn parse_decimal(v: &str) -> Option<f64> {
    if !v
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+' | b'e' | b'E'))
        || !v.bytes().any(|b| b.is_ascii_digit())
    {
        return None;
    }
    v.parse::<f64>().ok().filter(|x| x.is_finite())
}

pub fn parse_bool(v: &str) -> Option<bool> {
    match v {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// `YYYY-MM-DD`.
pub fn parse_iso_date(v: &str) -> Option<chrono::NaiveDate> {
    let b = v.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i32 = v[..4].parse().ok()?;
    let m: u32 = v[5..7].parse().ok()?;
    let d: u32 = v[8..10].parse().ok()?;
    chrono::NaiveDate::from_ymd_opt(y, m, d)
}

/// `YYYY-MM-DD HH:MM[:SS]` (or with a `T`); a plain date counts as midnight.
pub fn parse_iso_datetime(v: &str) -> Option<chrono::NaiveDateTime> {
    if v.len() == 10 {
        return parse_iso_date(v)?.and_hms_opt(0, 0, 0);
    }
    if v.len() < 16 || !v.is_char_boundary(10) {
        return None;
    }
    let date = parse_iso_date(&v[..10])?;
    let sep = v.as_bytes()[10];
    if sep != b' ' && sep != b'T' {
        return None;
    }
    let time = &v[11..];
    let b = time.as_bytes();
    if b.len() < 5 || b[2] != b':' {
        return None;
    }
    let h: u32 = time[..2].parse().ok()?;
    let m: u32 = time[3..5].parse().ok()?;
    let s: u32 = match b.len() {
        5 => 0,
        8 if b[5] == b':' => time[6..8].parse().ok()?,
        n if n > 9
            && b[5] == b':'
            && b[8] == b'.'
            && time[9..].bytes().all(|c| c.is_ascii_digit()) =>
        {
            time[6..8].parse().ok()?
        }
        _ => return None,
    };
    date.and_hms_opt(h, m, s)
}

pub fn format_date(d: chrono::NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub fn format_datetime(d: chrono::NaiveDateTime) -> String {
    d.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// The values of one column, packed into one string.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cells {
    buf: String,
    ends: Vec<u32>,
}

impl Cells {
    pub fn with_capacity(rows: usize) -> Self {
        Self {
            buf: String::new(),
            ends: Vec::with_capacity(rows),
        }
    }

    pub fn push(&mut self, v: &str) {
        self.buf.push_str(v);
        self.ends.push(self.buf.len() as u32);
    }

    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn get(&self, i: usize) -> &str {
        let end = self.ends[i] as usize;
        let start = if i == 0 { 0 } else { self.ends[i - 1] as usize };
        &self.buf[start..end]
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        (0..self.len()).map(|i| self.get(i))
    }

    /// The values at `rows`, in that order.
    pub fn pick(&self, rows: &[usize]) -> Self {
        let mut out = Cells::with_capacity(rows.len());
        for &r in rows {
            out.push(self.get(r));
        }
        out
    }

    pub fn map(&self, mut f: impl FnMut(&str) -> String) -> Self {
        let mut out = Cells::with_capacity(self.len());
        for v in self.iter() {
            out.push(&f(v));
        }
        out
    }

    #[cfg(test)]
    pub fn from_values<S: AsRef<str>>(values: impl IntoIterator<Item = S>) -> Self {
        let mut out = Cells::default();
        for v in values {
            out.push(v.as_ref());
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub kind: Kind,
    pub cells: Arc<Cells>,
}

impl Column {
    pub fn new(name: impl Into<String>, kind: Kind, cells: Cells) -> Self {
        Self {
            name: name.into(),
            kind,
            cells: Arc::new(cells),
        }
    }

    /// The rows whose value doesn't fit the column's type.
    pub fn problems(&self) -> usize {
        if self.kind == Kind::Text {
            return 0;
        }
        self.cells
            .iter()
            .filter(|v| !v.is_empty() && !self.kind.fits(v))
            .count()
    }

    pub fn is_problem(&self, row: usize) -> bool {
        let v = self.cells.get(row);
        self.kind != Kind::Text && !v.is_empty() && !self.kind.fits(v)
    }
}

/// Rows and typed columns. Columns that a step doesn't touch are shared
/// with the table before it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: usize,
}

impl Table {
    /// A table from names and rows of text, every column typed as `Text`.
    pub fn from_rows(names: Vec<String>, rows: &[Vec<String>]) -> Self {
        let names = unique_names(names);
        let columns = names
            .into_iter()
            .enumerate()
            .map(|(c, name)| {
                let mut cells = Cells::with_capacity(rows.len());
                for r in rows {
                    cells.push(r.get(c).map_or("", String::as_str));
                }
                Column::new(name, Kind::Text, cells)
            })
            .collect();
        Self {
            columns,
            rows: rows.len(),
        }
    }

    pub fn get(&self, row: usize, col: usize) -> &str {
        self.columns[col].cells.get(row)
    }

    pub fn names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.clone()).collect()
    }

    /// The row as text, one value per column.
    pub fn row(&self, row: usize) -> Vec<String> {
        self.columns
            .iter()
            .map(|c| c.cells.get(row).to_owned())
            .collect()
    }

    /// The column called `name`: an exact match, or else one that differs
    /// only in case.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.name == name)
            .or_else(|| {
                self.columns
                    .iter()
                    .position(|c| c.name.to_lowercase() == name.to_lowercase())
            })
    }

    /// Like [`Self::find`], with an error naming the missing column.
    pub fn need(&self, name: &str) -> Result<usize, String> {
        self.find(name)
            .ok_or_else(|| t!("table-no-column", column = name.to_owned()))
    }

    /// The rows at `rows`, in that order.
    pub fn pick_rows(&self, rows: &[usize]) -> Table {
        Table {
            columns: self
                .columns
                .iter()
                .map(|c| Column {
                    name: c.name.clone(),
                    kind: c.kind,
                    cells: Arc::new(c.cells.pick(rows)),
                })
                .collect(),
            rows: rows.len(),
        }
    }

    pub fn problems(&self) -> usize {
        self.columns.iter().map(Column::problems).sum()
    }

    /// Guesses each text column's type from its values: the narrowest one
    /// every non-empty value fits. Numbers with a leading zero stay text so
    /// codes like `007` keep their zeros.
    pub fn detect_types(&mut self) {
        for col in &mut self.columns {
            if col.kind == Kind::Text {
                col.kind = detect_kind(&col.cells);
            }
        }
    }

    /// A name for a new column that no column has yet.
    pub fn free_name(&self, base: &str) -> String {
        let mut name = base.to_owned();
        let mut n = 2;
        while self.find(&name).is_some() {
            name = format!("{base} {n}");
            n += 1;
        }
        name
    }
}

/// The kind of a column of text values (see [`Table::detect_types`]).
pub fn detect_kind(cells: &Cells) -> Kind {
    let mut candidates = [
        Kind::Integer,
        Kind::Decimal,
        Kind::Date,
        Kind::DateTime,
        Kind::Boolean,
    ]
    .to_vec();
    let mut any = false;
    for v in cells.iter() {
        if v.is_empty() {
            continue;
        }
        any = true;
        let leading_zero = {
            let d = v.strip_prefix('-').unwrap_or(v);
            d.len() > 1 && d.starts_with('0') && !d.starts_with("0.")
        };
        candidates.retain(|k| match k {
            Kind::Integer | Kind::Decimal => !leading_zero && k.fits(v),
            _ => k.fits(v),
        });
        if candidates.is_empty() {
            return Kind::Text;
        }
    }
    if !any {
        return Kind::Text;
    }
    // Integer before Decimal, Date before DateTime: the first that fits.
    candidates[0]
}

/// Non-empty, unique column names: `Column 1`… where there is none, and
/// `Name 2` for the second `Name`.
pub fn unique_names(names: Vec<String>) -> Vec<String> {
    let mut used = std::collections::HashSet::new();
    names
        .into_iter()
        .enumerate()
        .map(|(i, n)| {
            let base = match n.trim() {
                "" => t!("table-column-n", n = i + 1),
                t => t.to_owned(),
            };
            let mut name = base.clone();
            let mut k = 2;
            while !used.insert(name.to_lowercase()) {
                name = format!("{base} {k}");
                k += 1;
            }
            name
        })
        .collect()
}

#[cfg(test)]
mod tree_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn table(names: &[&str], rows: &[&[&str]]) -> Table {
        Table::from_rows(
            names.iter().map(|s| s.to_string()).collect(),
            &rows
                .iter()
                .map(|r| r.iter().map(|s| s.to_string()).collect())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn cells_pack_values() {
        let c = Cells::from_values(["a", "", "ccc"]);
        assert_eq!(c.len(), 3);
        assert_eq!(c.get(0), "a");
        assert_eq!(c.get(1), "");
        assert_eq!(c.get(2), "ccc");
        assert_eq!(c.pick(&[2, 0]).iter().collect::<Vec<_>>(), ["ccc", "a"]);
    }

    #[test]
    fn detects_types() {
        let mut t = table(
            &["id", "zip", "price", "day", "at", "ok", "name"],
            &[
                &[
                    "1",
                    "007",
                    "1.5",
                    "2026-01-02",
                    "2026-01-02 10:00",
                    "true",
                    "x",
                ],
                &["22", "123", "2", "", "2026-01-02T10:00:30", "false", "y"],
            ],
        );
        t.detect_types();
        let kinds: Vec<Kind> = t.columns.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            [
                Kind::Integer,
                Kind::Text,
                Kind::Decimal,
                Kind::Date,
                Kind::DateTime,
                Kind::Boolean,
                Kind::Text
            ]
        );
    }

    #[test]
    fn names_are_unique_and_never_empty() {
        let names = unique_names(vec!["a".into(), "".into(), "A".into(), "a".into()]);
        assert_eq!(names, ["a", "Column 2", "A 2", "a 3"]);
    }

    #[test]
    fn typed_comparison_puts_empty_and_misfits_last() {
        let mut v = vec!["10", "", "9", "x", "-1"];
        v.sort_by(|a, b| Kind::Integer.compare(a, b));
        assert_eq!(v, ["-1", "9", "10", "x", ""]);
        let mut d = vec!["2026-02-01", "2025-12-31", "31/02/1979"];
        d.sort_by(|a, b| Kind::Date.compare(a, b));
        assert_eq!(d, ["2025-12-31", "2026-02-01", "31/02/1979"]);
    }

    #[test]
    fn problems_are_values_that_dont_fit() {
        let mut t = table(&["n"], &[&["1"], &["two"], &[""]]);
        t.columns[0].kind = Kind::Integer;
        assert_eq!(t.problems(), 1);
        assert!(t.columns[0].is_problem(1));
        assert!(!t.columns[0].is_problem(2));
    }
}
