//! CsvQuery's database: delimited tabs loaded as SQLite tables in memory, and
//! queries run against them. The tabs themselves are never changed.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, params_from_iter};

use super::csv::{ColType, Table, parse_real};

/// A value in a query result.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

impl Value {
    /// The value as shown in the grid and written out (NULL is empty).
    pub fn display(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Integer(i) => i.to_string(),
            Value::Real(x) => format_real(*x),
            Value::Text(t) => t.clone(),
        }
    }

    /// Orders values for sorting: empty first, then numbers, then text
    /// (case-insensitive).
    pub fn sort_cmp(&self, other: &Value) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        fn rank(v: &Value) -> u8 {
            match v {
                Value::Null => 0,
                Value::Integer(_) | Value::Real(_) => 1,
                Value::Text(_) => 2,
            }
        }
        fn num(v: &Value) -> f64 {
            match v {
                Value::Integer(i) => *i as f64,
                Value::Real(x) => *x,
                _ => 0.0,
            }
        }
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Text(a), Value::Text(b)) => a
                .to_lowercase()
                .cmp(&b.to_lowercase())
                .then_with(|| a.cmp(b)),
            _ if rank(self) == 1 && rank(other) == 1 => num(self)
                .partial_cmp(&num(other))
                .unwrap_or(Ordering::Equal),
            _ => rank(self).cmp(&rank(other)),
        }
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Integer(_) | Value::Real(_))
    }
}

/// A decimal without a trailing `.0` for whole numbers that fit, and without
/// float noise like `0.30000000000000004`.
fn format_real(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        return format!("{x:.0}");
    }
    let short = format!("{x:.12}");
    let short = short.trim_end_matches('0').trim_end_matches('.');
    if short.parse::<f64>().ok() == Some(x) || x.abs() >= 1e-6 {
        short.to_owned()
    } else {
        x.to_string()
    }
}

/// What a query returned.
#[derive(Clone, Debug, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    /// More rows matched than were kept.
    pub truncated: bool,
    /// Rows changed by the last statement, when it changed data.
    pub changed: Option<usize>,
    pub elapsed: Duration,
}

/// Quotes a name for SQL: `"my ""odd"" name"`.
pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// A table name from a file name: letters, digits and `_`, not starting with
/// a digit, so it can be typed without quotes. `orders 2024.csv` → `orders_2024`.
pub fn table_name(file_title: &str) -> String {
    let stem = match file_title.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.len() <= 4 => stem,
        _ => file_title,
    };
    let mut name: String = stem
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    name = name.trim_matches('_').to_owned();
    while name.contains("__") {
        name = name.replace("__", "_");
    }
    if name.is_empty() {
        name = "data".into();
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, '_');
    }
    if name.eq_ignore_ascii_case("this") {
        name.push_str("_1");
    }
    name
}

/// The in-memory database with one table per loaded tab.
pub struct Db {
    conn: Connection,
    /// Tables this database created from tabs, by name.
    tables: HashMap<String, Vec<ColType>>,
}

impl Db {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Ok(Self {
            conn,
            tables: HashMap::new(),
        })
    }

    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    /// (Re)creates table `name` from `table`, storing each column as `types`
    /// says. Empty fields are stored as NULL; a value that doesn't fit its
    /// column's type is kept as text.
    pub fn load(
        &mut self,
        name: &str,
        table: &Table,
        types: &[ColType],
        delimiter: u8,
    ) -> Result<(), String> {
        let q = quote_ident(name);
        let cols: Vec<String> = table
            .columns
            .iter()
            .zip(types)
            .map(|(c, t)| format!("{} {}", quote_ident(c), t.sql()))
            .collect();
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(&format!(
            "DROP TABLE IF EXISTS {q}; CREATE TABLE {q} ({});",
            cols.join(", ")
        ))
        .map_err(|e| e.to_string())?;
        if !table.columns.is_empty() {
            let marks = vec!["?"; table.columns.len()].join(", ");
            let mut insert = tx
                .prepare(&format!("INSERT INTO {q} VALUES ({marks})"))
                .map_err(|e| e.to_string())?;
            for row in &table.rows {
                let values = row.iter().zip(types).map(|(v, t)| to_sql(v, *t, delimiter));
                insert
                    .execute(params_from_iter(values))
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.tables.insert(name.to_owned(), types.to_vec());
        Ok(())
    }

    /// Drops the tables made from tabs that aren't in `keep`.
    pub fn retain(&mut self, keep: &[&str]) {
        let gone: Vec<String> = self
            .tables
            .keys()
            .filter(|t| !keep.contains(&t.as_str()))
            .cloned()
            .collect();
        for t in gone {
            let _ = self
                .conn
                .execute_batch(&format!("DROP TABLE IF EXISTS {}", quote_ident(&t)));
            self.tables.remove(&t);
        }
    }

    /// Points the `this` view at `table`.
    pub fn set_this(&self, table: &str) -> Result<(), String> {
        self.conn
            .execute_batch(&format!(
                "DROP VIEW IF EXISTS temp.this; CREATE TEMP VIEW this AS SELECT * FROM {};",
                quote_ident(table)
            ))
            .map_err(|e| e.to_string())
    }

    /// Runs `sql` (one or more statements; the last one's rows are returned),
    /// keeping at most `limit` rows.
    pub fn query(&self, sql: &str, limit: usize) -> Result<QueryResult, String> {
        let start = Instant::now();
        let statements = split_statements(sql);
        let Some((last, before)) = statements.split_last() else {
            return Err("Type a query, like SELECT * FROM this".into());
        };
        for s in before {
            self.conn.execute_batch(s).map_err(|e| e.to_string())?;
        }
        let mut stmt = self.conn.prepare(last).map_err(|e| e.to_string())?;
        let columns: Vec<String> = stmt.column_names().iter().map(|c| c.to_string()).collect();
        let n = columns.len();
        let mut result = QueryResult {
            columns,
            ..QueryResult::default()
        };
        let readonly = stmt.readonly();
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            if result.rows.len() >= limit {
                result.truncated = true;
                break;
            }
            let values = (0..n)
                .map(|i| match row.get_ref(i) {
                    Ok(ValueRef::Integer(v)) => Value::Integer(v),
                    Ok(ValueRef::Real(v)) => Value::Real(v),
                    Ok(ValueRef::Text(t)) => Value::Text(String::from_utf8_lossy(t).into_owned()),
                    Ok(ValueRef::Blob(b)) => Value::Text(format!("<{} bytes>", b.len())),
                    _ => Value::Null,
                })
                .collect();
            result.rows.push(values);
        }
        drop(rows);
        if !readonly && n == 0 {
            result.changed = Some(self.conn.changes() as usize);
        }
        result.elapsed = start.elapsed();
        Ok(result)
    }
}

fn to_sql(v: &str, ty: ColType, delimiter: u8) -> rusqlite::types::Value {
    use rusqlite::types::Value as V;
    let t = v.trim();
    if t.is_empty() {
        return V::Null;
    }
    match ty {
        ColType::Integer => t
            .parse::<i64>()
            .map(V::Integer)
            .unwrap_or_else(|_| V::Text(v.to_owned())),
        ColType::Real => parse_real(t, delimiter)
            .map(V::Real)
            .unwrap_or_else(|| V::Text(v.to_owned())),
        ColType::Text => V::Text(v.to_owned()),
    }
}

/// Splits SQL at `;` outside quotes and comments, dropping empty statements.
pub fn split_statements(sql: &str) -> Vec<&str> {
    let bytes = sql.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            q @ (b'\'' | b'"' | b'`') => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == q {
                        if bytes.get(i + 1) == Some(&q) {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    i += 1;
                }
            }
            b'[' => {
                while i < bytes.len() && bytes[i] != b']' {
                    i += 1;
                }
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 1;
            }
            b';' => {
                out.push(&sql[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&sql[start.min(sql.len())..]);
    out.into_iter()
        .filter(|s| !is_blank_sql(s))
        .map(str::trim)
        .collect()
}

/// True for whitespace and comments only.
fn is_blank_sql(s: &str) -> bool {
    let mut rest = s.trim_start();
    loop {
        if rest.is_empty() {
            return true;
        }
        if let Some(r) = rest.strip_prefix("--") {
            rest = r.split_once('\n').map_or("", |(_, r)| r).trim_start();
        } else if let Some(r) = rest.strip_prefix("/*") {
            rest = r.split_once("*/").map_or("", |(_, r)| r).trim_start();
        } else {
            return false;
        }
    }
}

/// Statistics of one column.
#[derive(Clone, Debug, Default)]
pub struct ColumnStats {
    pub filled: usize,
    pub empty: usize,
    pub distinct: usize,
    pub min: String,
    pub max: String,
    /// Average, for number columns.
    pub mean: Option<f64>,
}

impl Db {
    /// Count, empties, distinct values, min, max and (for numbers) the
    /// average of every column of `table`.
    pub fn stats(&self, table: &str) -> Result<Vec<ColumnStats>, String> {
        let Some(types) = self.tables.get(table) else {
            return Ok(Vec::new());
        };
        let q = quote_ident(table);
        let stmt = self
            .conn
            .prepare(&format!("SELECT * FROM {q} LIMIT 0"))
            .map_err(|e| e.to_string())?;
        let names: Vec<String> = stmt.column_names().iter().map(|c| c.to_string()).collect();
        drop(stmt);
        let mut out = Vec::new();
        for (name, ty) in names.iter().zip(types) {
            let c = quote_ident(name);
            let avg = if *ty == ColType::Text {
                "NULL".to_owned()
            } else {
                format!("AVG({c})")
            };
            let sql = format!(
                "SELECT COUNT({c}), COUNT(*) - COUNT({c}), COUNT(DISTINCT {c}), MIN({c}), MAX({c}), {avg} FROM {q}"
            );
            let stats = self
                .conn
                .query_row(&sql, [], |r| {
                    let text = |i| -> rusqlite::Result<String> {
                        Ok(match r.get_ref(i)? {
                            ValueRef::Integer(v) => v.to_string(),
                            ValueRef::Real(v) => format_real(v),
                            ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                            _ => String::new(),
                        })
                    };
                    Ok(ColumnStats {
                        filled: r.get::<_, i64>(0)? as usize,
                        empty: r.get::<_, i64>(1)? as usize,
                        distinct: r.get::<_, i64>(2)? as usize,
                        min: text(3)?,
                        max: text(4)?,
                        mean: r.get::<_, Option<f64>>(5)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            out.push(stats);
        }
        Ok(out)
    }
}

/// Formats a number for the statistics view.
pub fn format_number(x: f64) -> String {
    format_real((x * 1e6).round() / 1e6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::csv::{CsvFormat, infer_types, parse};

    fn db_with(name: &str, text: &str) -> Db {
        let mut db = Db::new().unwrap();
        let f = CsvFormat::default();
        let t = parse(text, &f).unwrap();
        let types = infer_types(&t, b',');
        db.load(name, &t, &types, b',').unwrap();
        db
    }

    #[test]
    fn query_with_types_and_this() {
        let db = db_with("people", "name,age,zip\nann,31,0123\nbob,,0456\ncy,7,\n");
        db.set_this("people").unwrap();
        let r = db
            .query(
                "SELECT name, age FROM this WHERE age > 10 ORDER BY age",
                100,
            )
            .unwrap();
        assert_eq!(r.columns, vec!["name", "age"]);
        assert_eq!(
            r.rows,
            vec![vec![Value::Text("ann".into()), Value::Integer(31)]]
        );
        // Empty fields are NULL; zips keep their zeros.
        let r = db
            .query("SELECT zip FROM people WHERE age IS NULL", 100)
            .unwrap();
        assert_eq!(r.rows, vec![vec![Value::Text("0456".into())]]);
    }

    #[test]
    fn joins_limits_and_several_statements() {
        let mut db = db_with("a", "id,x\n1,p\n2,q\n3,r\n");
        let f = CsvFormat::default();
        let t = parse("id,y\n1,P\n3,R\n", &f).unwrap();
        db.load("b", &t, &infer_types(&t, b','), b',').unwrap();
        let r = db
            .query("SELECT a.x, b.y FROM a JOIN b USING (id) ORDER BY id", 1)
            .unwrap();
        assert_eq!(r.rows.len(), 1);
        assert!(r.truncated);
        let r = db
            .query(
                "UPDATE a SET x = 'z' WHERE id = 1; SELECT x FROM a WHERE id = 1",
                10,
            )
            .unwrap();
        assert_eq!(r.rows, vec![vec![Value::Text("z".into())]]);
        let r = db.query("DELETE FROM a WHERE id > 1", 10).unwrap();
        assert_eq!(r.changed, Some(2));
        assert!(db.query("SELEC 1", 10).is_err());
        assert!(db.query("  -- nothing\n", 10).is_err());
        db.retain(&["a"]);
        assert!(db.query("SELECT * FROM b", 10).is_err());
    }

    #[test]
    fn splits_statements_outside_quotes() {
        assert_eq!(
            split_statements("select ';'; -- a; b\nselect \"x;y\" /* ; */ ;"),
            vec!["select ';'", "-- a; b\nselect \"x;y\" /* ; */"]
        );
    }

    #[test]
    fn names_and_numbers() {
        assert_eq!(table_name("orders 2024.csv"), "orders_2024");
        assert_eq!(table_name("2024-sales.tsv"), "_2024_sales");
        assert_eq!(table_name("new 1"), "new_1");
        assert_eq!(table_name("this.csv"), "this_1");
        assert_eq!(table_name("...csv"), "data");
        assert_eq!(format_real(0.1 + 0.2), "0.3");
        assert_eq!(format_real(3.0), "3");
        assert_eq!(format_real(2.5), "2.5");
    }

    #[test]
    fn stats_per_column() {
        let db = db_with("t", "n,s\n1,a\n3,a\n,b\n");
        let s = db.stats("t").unwrap();
        assert_eq!((s[0].filled, s[0].empty, s[0].distinct), (2, 1, 2));
        assert_eq!((s[0].min.as_str(), s[0].max.as_str()), ("1", "3"));
        assert_eq!(s[0].mean, Some(2.0));
        assert_eq!((s[1].distinct, s[1].mean), (2, None));
    }

    #[test]
    fn sorting_values() {
        let mut v = vec![
            Value::Text("b".into()),
            Value::Integer(10),
            Value::Null,
            Value::Real(2.5),
            Value::Text("A".into()),
        ];
        v.sort_by(|a, b| a.sort_cmp(b));
        assert_eq!(
            v,
            vec![
                Value::Null,
                Value::Real(2.5),
                Value::Integer(10),
                Value::Text("A".into()),
                Value::Text("b".into())
            ]
        );
    }
}
