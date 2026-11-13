//! The steps that change a table. Each takes the table as it is and gives a
//! new one; columns are named, not numbered, so a step still finds its
//! column when the next file has its columns in another order.

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::pattern::Pattern;
use super::{Cells, Column, Kind, Table};

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextOp {
    #[default]
    Trim,
    /// Runs of spaces become one, and the ends are trimmed.
    CollapseSpaces,
    Upper,
    Lower,
    /// The first letter of each word upper case, the rest lower case.
    Title,
}

impl TextOp {
    pub const ALL: [TextOp; 5] = [
        TextOp::Trim,
        TextOp::CollapseSpaces,
        TextOp::Upper,
        TextOp::Lower,
        TextOp::Title,
    ];

    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            TextOp::Trim => "table-text-trim",
            TextOp::CollapseSpaces => "table-text-collapse",
            TextOp::Upper => "table-text-upper",
            TextOp::Lower => "table-text-lower",
            TextOp::Title => "table-text-title",
        })
    }

    pub fn apply(self, v: &str) -> String {
        match self {
            TextOp::Trim => v.trim().to_owned(),
            TextOp::CollapseSpaces => v.split_whitespace().collect::<Vec<_>>().join(" "),
            TextOp::Upper => v.to_uppercase(),
            TextOp::Lower => v.to_lowercase(),
            TextOp::Title => {
                let mut out = String::with_capacity(v.len());
                let mut start = true;
                for c in v.chars() {
                    if c.is_alphanumeric() || c == '\'' {
                        if start {
                            out.extend(c.to_uppercase());
                        } else {
                            out.extend(c.to_lowercase());
                        }
                        start = false;
                    } else {
                        out.push(c);
                        start = true;
                    }
                }
                out
            }
        }
    }
}

/// How a filter compares a row's value with the filter's value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    #[default]
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    IsEmpty,
    NotEmpty,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    /// Matches a regular expression.
    Matches,
    /// Is one of `values`.
    OneOf,
}

impl Condition {
    pub const ALL: [Condition; 14] = [
        Condition::Equals,
        Condition::NotEquals,
        Condition::Contains,
        Condition::NotContains,
        Condition::StartsWith,
        Condition::EndsWith,
        Condition::IsEmpty,
        Condition::NotEmpty,
        Condition::Greater,
        Condition::GreaterOrEqual,
        Condition::Less,
        Condition::LessOrEqual,
        Condition::Matches,
        Condition::OneOf,
    ];

    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            Condition::Equals => "table-cond-equals",
            Condition::NotEquals => "table-cond-not-equals",
            Condition::Contains => "table-cond-contains",
            Condition::NotContains => "table-cond-not-contains",
            Condition::StartsWith => "table-cond-starts-with",
            Condition::EndsWith => "table-cond-ends-with",
            Condition::IsEmpty => "table-cond-is-empty",
            Condition::NotEmpty => "table-cond-not-empty",
            Condition::Greater => "table-cond-greater",
            Condition::GreaterOrEqual => "table-cond-greater-or-equal",
            Condition::Less => "table-cond-less",
            Condition::LessOrEqual => "table-cond-less-or-equal",
            Condition::Matches => "table-cond-matches",
            Condition::OneOf => "table-cond-one-of",
        })
    }

    /// Whether it compares with a typed value.
    pub fn needs_value(self) -> bool {
        !matches!(
            self,
            Condition::IsEmpty | Condition::NotEmpty | Condition::OneOf
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortKey {
    pub column: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub descending: bool,
}

/// One change to a table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum TableStep {
    RenameColumn {
        column: String,
        to: String,
    },
    /// Moves a column to position `to` (0 is first).
    MoveColumn {
        column: String,
        to: usize,
    },
    RemoveColumns {
        columns: Vec<String>,
    },
    /// Keeps only these columns, in this order.
    ChooseColumns {
        columns: Vec<String>,
    },
    DuplicateColumn {
        column: String,
        to: String,
    },
    /// Sets the column's type. Values are read with `format` (a date
    /// pattern like `DD/MM/YYYY`, or for numbers `,` when they use a
    /// decimal comma) and rewritten in the type's own form; ones that can't
    /// be read are left as they are.
    ChangeType {
        column: String,
        kind: Kind,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        format: String,
    },
    /// Replaces text in one column, or in every column when `column` is
    /// empty.
    ReplaceValues {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        column: String,
        find: String,
        #[serde(default)]
        replace: String,
        /// Only values that are exactly `find` (all of the cell).
        #[serde(default, skip_serializing_if = "is_false")]
        whole_cell: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        match_case: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        regex: bool,
    },
    Text {
        column: String,
        text: TextOp,
    },
    /// Keeps the rows where the condition holds.
    FilterRows {
        column: String,
        #[serde(default)]
        condition: Condition,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        value: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        values: Vec<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        match_case: bool,
    },
    SortRows {
        keys: Vec<SortKey>,
    },
    /// Keeps the first row of each set of rows with the same values in
    /// `columns` (all columns when empty).
    RemoveDuplicates {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        columns: Vec<String>,
    },
    /// Removes rows where every value is empty (or only spaces).
    RemoveEmptyRows,
    /// Removes rows by their number (from 0) at this step.
    RemoveRows {
        rows: Vec<usize>,
    },
    /// A value typed into a cell. Rows are numbered from 0 at this step.
    EditCell {
        row: usize,
        column: String,
        value: String,
    },
}

impl TableStep {
    /// A short description for the Steps list and recipes.
    pub fn summary(&self) -> String {
        match self {
            TableStep::RenameColumn { column, to } => t!(
                "table-step-rename",
                column = column.clone(),
                to = to.clone()
            ),
            TableStep::MoveColumn { column, to } => {
                t!("table-step-move", column = column.clone(), to = to + 1)
            }
            TableStep::RemoveColumns { columns } => {
                t!("table-step-remove-columns", columns = list(columns))
            }
            TableStep::ChooseColumns { columns } => {
                t!("table-step-choose-columns", columns = list(columns))
            }
            TableStep::DuplicateColumn { column, to } => t!(
                "table-step-duplicate",
                column = column.clone(),
                to = to.clone()
            ),
            TableStep::ChangeType {
                column,
                kind,
                format,
            } => {
                if format.is_empty() {
                    t!(
                        "table-step-type",
                        column = column.clone(),
                        kind = kind.label()
                    )
                } else {
                    t!(
                        "table-step-type-format",
                        column = column.clone(),
                        kind = kind.label(),
                        format = format.clone()
                    )
                }
            }
            TableStep::ReplaceValues {
                column,
                find,
                replace,
                ..
            } => {
                let column = if column.is_empty() {
                    t!("table-all-columns")
                } else {
                    column.clone()
                };
                t!(
                    "table-step-replace",
                    column = column,
                    find = find.clone(),
                    replace = replace.clone()
                )
            }
            TableStep::Text { column, text } => {
                t!("table-step-text", column = column.clone(), op = text.label())
            }
            TableStep::FilterRows {
                column,
                condition,
                value,
                values,
                ..
            } => {
                let what = match condition {
                    Condition::OneOf => list(values),
                    c if c.needs_value() => format!("\"{value}\""),
                    _ => String::new(),
                };
                t!(
                    "table-step-filter",
                    column = column.clone(),
                    condition = condition.label(),
                    value = what
                )
                .trim_end()
                .to_owned()
            }
            TableStep::SortRows { keys } => {
                let keys: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        if k.descending {
                            format!("{} ↓", k.column)
                        } else {
                            format!("{} ↑", k.column)
                        }
                    })
                    .collect();
                t!("table-step-sort", keys = keys.join(", "))
            }
            TableStep::RemoveDuplicates { columns } if columns.is_empty() => {
                t!("table-step-dedupe-all")
            }
            TableStep::RemoveDuplicates { columns } => {
                t!("table-step-dedupe", columns = list(columns))
            }
            TableStep::RemoveEmptyRows => t!("table-step-remove-empty"),
            TableStep::RemoveRows { rows } => t!("table-step-remove-rows", count = rows.len()),
            TableStep::EditCell { row, column, value } => t!(
                "table-step-edit-cell",
                row = row + 1,
                column = column.clone(),
                value = value.clone()
            ),
        }
    }

    /// Whether replaying the step on another file may not do what it did
    /// here: it picks rows by number.
    pub fn by_row_number(&self) -> bool {
        matches!(
            self,
            TableStep::EditCell { .. } | TableStep::RemoveRows { .. }
        )
    }

    /// The step after column `old` was renamed to `new` by an earlier step.
    pub fn rename_column(&mut self, old: &str, new: &str) {
        let fix = |c: &mut String| {
            if c == old {
                *c = new.to_owned();
            }
        };
        match self {
            TableStep::RenameColumn { column, .. }
            | TableStep::MoveColumn { column, .. }
            | TableStep::DuplicateColumn { column, .. }
            | TableStep::ChangeType { column, .. }
            | TableStep::ReplaceValues { column, .. }
            | TableStep::Text { column, .. }
            | TableStep::FilterRows { column, .. }
            | TableStep::EditCell { column, .. } => fix(column),
            TableStep::RemoveColumns { columns }
            | TableStep::ChooseColumns { columns }
            | TableStep::RemoveDuplicates { columns } => columns.iter_mut().for_each(fix),
            TableStep::SortRows { keys } => keys.iter_mut().for_each(|k| fix(&mut k.column)),
            TableStep::RemoveEmptyRows | TableStep::RemoveRows { .. } => {}
        }
    }

    /// Checks the settings without a table: patterns compile, names aren't empty.
    pub fn check(&self) -> Result<(), String> {
        match self {
            TableStep::RenameColumn { to, .. } | TableStep::DuplicateColumn { to, .. }
                if to.trim().is_empty() =>
            {
                Err(t!("table-name-empty"))
            }
            TableStep::ChangeType {
                kind: Kind::Date | Kind::DateTime,
                format,
                ..
            } if !format.is_empty() => Pattern::new(format).map(|_| ()),
            TableStep::ReplaceValues { find, .. } if find.is_empty() => Err(t!("recipe-no-find")),
            TableStep::ReplaceValues {
                find,
                regex: true,
                match_case,
                ..
            } => regex::RegexBuilder::new(find)
                .case_insensitive(!match_case)
                .build()
                .map(|_| ())
                .map_err(|e| e.to_string()),
            TableStep::FilterRows {
                condition: Condition::Matches,
                value,
                match_case,
                ..
            } => regex::RegexBuilder::new(value)
                .case_insensitive(!match_case)
                .build()
                .map(|_| ())
                .map_err(|e| e.to_string()),
            TableStep::SortRows { keys } if keys.is_empty() => Err(t!("table-sort-no-keys")),
            TableStep::ChooseColumns { columns } if columns.is_empty() => {
                Err(t!("table-choose-none"))
            }
            _ => Ok(()),
        }
    }

    /// Runs the step.
    pub fn apply(&self, t: &Table) -> Result<Table, String> {
        self.check()?;
        let mut out = t.clone();
        match self {
            TableStep::RenameColumn { column, to } => {
                let c = t.need(column)?;
                let to = to.trim();
                if let Some(other) = t.find(to)
                    && other != c
                    && t.columns[other].name == to
                {
                    return Err(t!("table-name-taken", name = to.to_owned()));
                }
                out.columns[c].name = to.to_owned();
            }
            TableStep::MoveColumn { column, to } => {
                let c = t.need(column)?;
                let col = out.columns.remove(c);
                let to = (*to).min(out.columns.len());
                out.columns.insert(to, col);
            }
            TableStep::RemoveColumns { columns } => {
                let mut drop = Vec::new();
                for name in columns {
                    drop.push(t.need(name)?);
                }
                out.columns = t
                    .columns
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !drop.contains(i))
                    .map(|(_, c)| c.clone())
                    .collect();
            }
            TableStep::ChooseColumns { columns } => {
                let mut keep = Vec::new();
                for name in columns {
                    let c = t.need(name)?;
                    if !keep.contains(&c) {
                        keep.push(c);
                    }
                }
                out.columns = keep.into_iter().map(|c| t.columns[c].clone()).collect();
            }
            TableStep::DuplicateColumn { column, to } => {
                let c = t.need(column)?;
                let to = to.trim();
                if t.find(to).is_some_and(|o| t.columns[o].name == to) {
                    return Err(t!("table-name-taken", name = to.to_owned()));
                }
                let mut copy = t.columns[c].clone();
                copy.name = to.to_owned();
                out.columns.insert(c + 1, copy);
            }
            TableStep::ChangeType {
                column,
                kind,
                format,
            } => {
                let c = t.need(column)?;
                let col = &t.columns[c];
                let cells = convert_cells(&col.cells, *kind, format)?;
                out.columns[c] = Column {
                    name: col.name.clone(),
                    kind: *kind,
                    cells: Arc::new(cells),
                };
            }
            TableStep::ReplaceValues {
                column,
                find,
                replace,
                whole_cell,
                match_case,
                regex,
            } => {
                let targets: Vec<usize> = if column.is_empty() {
                    (0..t.columns.len()).collect()
                } else {
                    vec![t.need(column)?]
                };
                let replacer = Replacer::new(find, replace, *whole_cell, *match_case, *regex)?;
                for c in targets {
                    let cells = t.columns[c].cells.map(|v| replacer.apply(v));
                    out.columns[c].cells = Arc::new(cells);
                }
            }
            TableStep::Text { column, text } => {
                let c = t.need(column)?;
                out.columns[c].cells = Arc::new(t.columns[c].cells.map(|v| text.apply(v)));
            }
            TableStep::FilterRows {
                column,
                condition,
                value,
                values,
                match_case,
            } => {
                let c = t.need(column)?;
                let test = Filter::new(t.columns[c].kind, *condition, value, values, *match_case)?;
                let cells = &t.columns[c].cells;
                let keep: Vec<usize> = (0..t.rows).filter(|&r| test.keep(cells.get(r))).collect();
                out = t.pick_rows(&keep);
            }
            TableStep::SortRows { keys } => {
                let mut cols = Vec::new();
                for k in keys {
                    cols.push((t.need(&k.column)?, k.descending));
                }
                let mut order: Vec<usize> = (0..t.rows).collect();
                order.sort_by(|&a, &b| {
                    for &(c, desc) in &cols {
                        let col = &t.columns[c];
                        let (x, y) = (col.cells.get(a), col.cells.get(b));
                        let mut o = col.kind.compare(x, y);
                        // Empty values stay last either way.
                        if desc && !x.is_empty() && !y.is_empty() {
                            o = o.reverse();
                        }
                        if o != std::cmp::Ordering::Equal {
                            return o;
                        }
                    }
                    std::cmp::Ordering::Equal
                });
                out = t.pick_rows(&order);
            }
            TableStep::RemoveDuplicates { columns } => {
                let cols: Vec<usize> = if columns.is_empty() {
                    (0..t.columns.len()).collect()
                } else {
                    let mut v = Vec::new();
                    for name in columns {
                        v.push(t.need(name)?);
                    }
                    v
                };
                let mut seen = HashSet::new();
                let keep: Vec<usize> = (0..t.rows)
                    .filter(|&r| {
                        let key: Vec<&str> = cols.iter().map(|&c| t.get(r, c)).collect();
                        seen.insert(key.join("\u{1f}"))
                    })
                    .collect();
                out = t.pick_rows(&keep);
            }
            TableStep::RemoveEmptyRows => {
                let keep: Vec<usize> = (0..t.rows)
                    .filter(|&r| t.columns.iter().any(|c| !c.cells.get(r).trim().is_empty()))
                    .collect();
                out = t.pick_rows(&keep);
            }
            TableStep::RemoveRows { rows } => {
                let drop: HashSet<usize> = rows.iter().copied().collect();
                let keep: Vec<usize> = (0..t.rows).filter(|r| !drop.contains(r)).collect();
                out = t.pick_rows(&keep);
            }
            TableStep::EditCell { row, column, value } => {
                let c = t.need(column)?;
                if *row >= t.rows {
                    return Err(t!("table-no-row", row = row + 1));
                }
                let cells = &t.columns[c].cells;
                let mut new = Cells::with_capacity(t.rows);
                for (r, v) in cells.iter().enumerate() {
                    new.push(if r == *row { value } else { v });
                }
                out.columns[c].cells = Arc::new(new);
            }
        }
        Ok(out)
    }
}

fn list(names: &[String]) -> String {
    names.join(", ")
}

/// Rewrites values in `kind`'s canonical form, reading them with `format`.
/// Values that can't be read stay as they are.
pub fn convert_cells(cells: &Cells, kind: Kind, format: &str) -> Result<Cells, String> {
    let pattern = match kind {
        Kind::Date | Kind::DateTime if !format.is_empty() => Some(Pattern::new(format)?),
        _ => None,
    };
    let decimal_comma = format == ",";
    Ok(cells.map(|v| convert_value(v, kind, pattern.as_ref(), decimal_comma).unwrap_or_else(|| v.to_owned())))
}

/// `v` in `kind`'s form, or `None` when it can't be read as one.
pub fn convert_value(
    v: &str,
    kind: Kind,
    pattern: Option<&Pattern>,
    decimal_comma: bool,
) -> Option<String> {
    let s = v.trim();
    if s.is_empty() {
        return Some(String::new());
    }
    match kind {
        Kind::Text => Some(v.to_owned()),
        Kind::Integer | Kind::Decimal => {
            let mut n: String = s.chars().filter(|c| !c.is_whitespace()).collect();
            if decimal_comma {
                n = n.replace('.', "").replace(',', ".");
            } else if n.matches(',').count() >= 1 && n.contains('.') || n.matches(',').count() > 1 {
                // Thousands separators: 1,234,567.5
                n = n.replace(',', "");
            }
            let n = n.strip_prefix('+').unwrap_or(&n).to_owned();
            if kind == Kind::Integer {
                if super::is_integer(&n) {
                    return Some(n.parse::<i64>().ok()?.to_string());
                }
                let x = super::parse_decimal(&n)?;
                (x.fract() == 0.0 && x.abs() < 9.0e15).then(|| format!("{}", x as i64))
            } else {
                super::parse_decimal(&n)?;
                Some(n)
            }
        }
        Kind::Date | Kind::DateTime => {
            let at = match pattern {
                Some(p) => p.parse(s)?,
                None => super::parse_iso_datetime(s)?,
            };
            Some(if kind == Kind::Date {
                super::format_date(at.date())
            } else {
                super::format_datetime(at)
            })
        }
        Kind::Boolean => match s.to_lowercase().as_str() {
            "true" | "yes" | "y" | "1" | "t" | "on" => Some("true".into()),
            "false" | "no" | "n" | "0" | "f" | "off" => Some("false".into()),
            _ => None,
        },
    }
}

struct Replacer {
    find: String,
    replace: String,
    whole_cell: bool,
    match_case: bool,
    regex: Option<regex::Regex>,
}

impl Replacer {
    fn new(
        find: &str,
        replace: &str,
        whole_cell: bool,
        match_case: bool,
        regex: bool,
    ) -> Result<Self, String> {
        let re = if regex || !match_case {
            let pattern = if regex {
                find.to_owned()
            } else {
                regex::escape(find)
            };
            let pattern = if whole_cell {
                format!("^(?:{pattern})$")
            } else {
                pattern
            };
            Some(
                regex::RegexBuilder::new(&pattern)
                    .case_insensitive(!match_case)
                    .build()
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        Ok(Self {
            find: find.to_owned(),
            replace: if regex {
                replace.to_owned()
            } else {
                replace.replace('$', "$$")
            },
            whole_cell,
            match_case,
            regex: re,
        })
    }

    fn apply(&self, v: &str) -> String {
        match &self.regex {
            Some(re) => re.replace_all(v, self.replace.as_str()).into_owned(),
            None if self.whole_cell => {
                if v == self.find {
                    self.replace.replace("$$", "$")
                } else {
                    v.to_owned()
                }
            }
            None => {
                debug_assert!(self.match_case);
                v.replace(&self.find, &self.replace.replace("$$", "$"))
            }
        }
    }
}

struct Filter {
    kind: Kind,
    condition: Condition,
    value: String,
    values: HashSet<String>,
    match_case: bool,
    regex: Option<regex::Regex>,
}

impl Filter {
    fn new(
        kind: Kind,
        condition: Condition,
        value: &str,
        values: &[String],
        match_case: bool,
    ) -> Result<Self, String> {
        let regex = match condition {
            Condition::Matches => Some(
                regex::RegexBuilder::new(value)
                    .case_insensitive(!match_case)
                    .build()
                    .map_err(|e| e.to_string())?,
            ),
            _ => None,
        };
        let fold = |s: &str| {
            if match_case {
                s.to_owned()
            } else {
                s.to_lowercase()
            }
        };
        Ok(Self {
            kind,
            condition,
            value: fold(value),
            values: values.iter().map(|v| fold(v)).collect(),
            match_case,
            regex,
        })
    }

    fn keep(&self, v: &str) -> bool {
        use std::cmp::Ordering::*;
        let folded = if self.match_case {
            std::borrow::Cow::Borrowed(v)
        } else {
            std::borrow::Cow::Owned(v.to_lowercase())
        };
        let f = folded.as_ref();
        let order = || {
            if v.is_empty() {
                None
            } else if self.kind == Kind::Text {
                Some(crate::tools::fields::natural_cmp(v, &self.value))
            } else if self.kind.fits(v) && self.kind.fits(&self.value) {
                Some(self.kind.compare(v, &self.value))
            } else {
                None
            }
        };
        match self.condition {
            Condition::Equals => f == self.value,
            Condition::NotEquals => f != self.value,
            Condition::Contains => f.contains(&self.value),
            Condition::NotContains => !f.contains(&self.value),
            Condition::StartsWith => f.starts_with(&self.value),
            Condition::EndsWith => f.ends_with(&self.value),
            Condition::IsEmpty => v.trim().is_empty(),
            Condition::NotEmpty => !v.trim().is_empty(),
            Condition::Greater => order() == Some(Greater),
            Condition::GreaterOrEqual => matches!(order(), Some(Greater | Equal)),
            Condition::Less => order() == Some(Less),
            Condition::LessOrEqual => matches!(order(), Some(Less | Equal)),
            Condition::Matches => self.regex.as_ref().is_some_and(|r| r.is_match(v)),
            Condition::OneOf => self.values.contains(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Table {
        let rows: Vec<Vec<String>> = [
            ["100231", "Okafor, Ada", "02/03/1984", "F", "5.4"],
            ["100877", "Lindqvist, Per", "19/11/1951", "M", "4.1"],
            ["101402", "moreau, claire", "25/07/1990", "F", "139"],
            ["100231", "Okafor, Ada", "02/03/1984", "F", "5.4"],
            ["102118", "Tanaka, Hiro", "31/02/1979", "", ""],
        ]
        .iter()
        .map(|r| r.iter().map(|s| s.to_string()).collect())
        .collect();
        let mut t = Table::from_rows(
            ["MRN", "Patient", "DOB", "Sex", "Result"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            &rows,
        );
        t.detect_types();
        t
    }

    fn col(t: &Table, name: &str) -> Vec<String> {
        let c = t.find(name).unwrap();
        t.columns[c].cells.iter().map(str::to_owned).collect()
    }

    #[test]
    fn change_type_reads_a_date_pattern_and_keeps_misfits() {
        let t = table();
        let step = TableStep::ChangeType {
            column: "DOB".into(),
            kind: Kind::Date,
            format: "DD/MM/YYYY".into(),
        };
        let out = step.apply(&t).unwrap();
        assert_eq!(
            col(&out, "DOB"),
            ["1984-03-02", "1951-11-19", "1990-07-25", "1984-03-02", "31/02/1979"]
        );
        assert_eq!(out.problems(), 1);
    }

    #[test]
    fn numbers_with_separators() {
        let c = Cells::from_values(["1,234.5", "3,5", " 7 ", "x"]);
        let d = convert_cells(&c, Kind::Decimal, "").unwrap();
        assert_eq!(d.iter().collect::<Vec<_>>(), ["1234.5", "3,5", "7", "x"]);
        let d = convert_cells(&c, Kind::Decimal, ",").unwrap();
        assert_eq!(d.get(1), "3.5");
        let i = convert_cells(&Cells::from_values(["+12", "12.0", "1.5"]), Kind::Integer, "").unwrap();
        assert_eq!(i.iter().collect::<Vec<_>>(), ["12", "12", "1.5"]);
    }

    #[test]
    fn filter_sort_dedupe() {
        let t = table();
        let f = TableStep::FilterRows {
            column: "Sex".into(),
            condition: Condition::Equals,
            value: "f".into(),
            values: vec![],
            match_case: false,
        };
        assert_eq!(f.apply(&t).unwrap().rows, 3);
        let gt = TableStep::FilterRows {
            column: "Result".into(),
            condition: Condition::Greater,
            value: "5".into(),
            values: vec![],
            match_case: false,
        };
        assert_eq!(col(&gt.apply(&t).unwrap(), "Result"), ["5.4", "139", "5.4"]);
        let sort = TableStep::SortRows {
            keys: vec![SortKey {
                column: "Result".into(),
                descending: true,
            }],
        };
        assert_eq!(
            col(&sort.apply(&t).unwrap(), "Result"),
            ["139", "5.4", "5.4", "4.1", ""]
        );
        let d = TableStep::RemoveDuplicates { columns: vec![] };
        assert_eq!(d.apply(&t).unwrap().rows, 4);
        let d = TableStep::RemoveDuplicates {
            columns: vec!["Sex".into()],
        };
        assert_eq!(d.apply(&t).unwrap().rows, 3);
        let one = TableStep::FilterRows {
            column: "MRN".into(),
            condition: Condition::OneOf,
            value: String::new(),
            values: vec!["100877".into(), "102118".into()],
            match_case: true,
        };
        assert_eq!(one.apply(&t).unwrap().rows, 2);
    }

    #[test]
    fn columns_by_name() {
        let t = table();
        let s = TableStep::ChooseColumns {
            columns: vec!["Result".into(), "mrn".into()],
        };
        let out = s.apply(&t).unwrap();
        assert_eq!(out.names(), ["Result", "MRN"]);
        let r = TableStep::RenameColumn {
            column: "Sex".into(),
            to: "Gender".into(),
        };
        assert_eq!(r.apply(&t).unwrap().names()[3], "Gender");
        let bad = TableStep::RenameColumn {
            column: "Sex".into(),
            to: "DOB".into(),
        };
        assert!(bad.apply(&t).is_err());
        let missing = TableStep::RemoveColumns {
            columns: vec!["Nope".into()],
        };
        assert!(missing.apply(&t).unwrap_err().contains("Nope"));
        let m = TableStep::MoveColumn {
            column: "Result".into(),
            to: 0,
        };
        assert_eq!(m.apply(&t).unwrap().names()[0], "Result");
    }

    #[test]
    fn text_and_replace() {
        let t = table();
        let s = TableStep::Text {
            column: "Patient".into(),
            text: TextOp::Title,
        };
        assert_eq!(col(&s.apply(&t).unwrap(), "Patient")[2], "Moreau, Claire");
        let r = TableStep::ReplaceValues {
            column: "Sex".into(),
            find: "f".into(),
            replace: "Female".into(),
            whole_cell: true,
            match_case: false,
            regex: false,
        };
        assert_eq!(col(&r.apply(&t).unwrap(), "Sex")[0], "Female");
        let r = TableStep::ReplaceValues {
            column: String::new(),
            find: r"(\w+), (\w+)".into(),
            replace: "$2 $1".into(),
            whole_cell: false,
            match_case: true,
            regex: true,
        };
        assert_eq!(col(&r.apply(&t).unwrap(), "Patient")[0], "Ada Okafor");
        let lit = TableStep::ReplaceValues {
            column: "Patient".into(),
            find: ", ".into(),
            replace: "$".into(),
            whole_cell: false,
            match_case: false,
            regex: false,
        };
        assert_eq!(col(&lit.apply(&t).unwrap(), "Patient")[0], "Okafor$Ada");
    }

    #[test]
    fn edit_and_remove_rows() {
        let t = table();
        let e = TableStep::EditCell {
            row: 4,
            column: "Sex".into(),
            value: "M".into(),
        };
        assert_eq!(col(&e.apply(&t).unwrap(), "Sex")[4], "M");
        let r = TableStep::RemoveRows { rows: vec![0, 3] };
        assert_eq!(r.apply(&t).unwrap().rows, 3);
        assert!(e.by_row_number());
    }

    #[test]
    fn steps_round_trip_as_json() {
        let s = TableStep::FilterRows {
            column: "Sex".into(),
            condition: Condition::NotEmpty,
            value: String::new(),
            values: vec![],
            match_case: false,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(
            json,
            r#"{"op":"filter_rows","column":"Sex","condition":"not_empty"}"#
        );
        assert_eq!(serde_json::from_str::<TableStep>(&json).unwrap(), s);
    }
}
