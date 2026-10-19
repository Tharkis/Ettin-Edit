//! Converting data between JSON, YAML, TOML, XML, CSV and Markdown tables.
//! Every format is read into a JSON value (keys keep their order) and
//! written out from one.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{ToolError, ToolResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFormat {
    Json,
    Yaml,
    Toml,
    Xml,
    Csv,
    /// A Markdown (GitHub) table; written only.
    Markdown,
}

impl DataFormat {
    /// The formats that can be read.
    pub const SOURCES: [DataFormat; 5] = [
        DataFormat::Json,
        DataFormat::Yaml,
        DataFormat::Toml,
        DataFormat::Xml,
        DataFormat::Csv,
    ];
    pub const TARGETS: [DataFormat; 6] = [
        DataFormat::Json,
        DataFormat::Yaml,
        DataFormat::Toml,
        DataFormat::Xml,
        DataFormat::Csv,
        DataFormat::Markdown,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DataFormat::Json => "JSON",
            DataFormat::Yaml => "YAML",
            DataFormat::Toml => "TOML",
            DataFormat::Xml => "XML",
            DataFormat::Csv => "CSV",
            DataFormat::Markdown => "Markdown",
        }
    }

    /// The language a document in this format is highlighted as.
    pub fn syntax(self) -> &'static str {
        match self {
            DataFormat::Markdown => "Markdown",
            f => f.name(),
        }
    }
}

/// Converts `text` from one format to another, indenting with `indent`.
pub fn convert(text: &str, from: DataFormat, to: DataFormat, indent: &str) -> ToolResult {
    // CSV to a table keeps every value exactly as written.
    let value = if from == DataFormat::Csv && matches!(to, DataFormat::Csv | DataFormat::Markdown) {
        read_csv(text, false)
    } else {
        read(text, from)?
    };
    let mut out = write(&value, to, indent)?;
    if !out.ends_with('\n') && text.ends_with('\n') {
        out.push('\n');
    }
    if out.ends_with('\n') && !text.ends_with('\n') {
        out.pop();
    }
    Ok(out)
}

pub fn read(text: &str, format: DataFormat) -> Result<Value, ToolError> {
    match format {
        DataFormat::Json => serde_json::from_str(text).map_err(|e| {
            let offset = super::line_col_to_offset(text, e.line(), e.column());
            ToolError::at(format!("Not valid JSON: {e}"), offset)
        }),
        DataFormat::Yaml => read_yaml(text),
        DataFormat::Toml => read_toml(text),
        DataFormat::Xml => read_xml(text),
        DataFormat::Csv | DataFormat::Markdown => Ok(read_csv(text, true)),
    }
}

pub fn write(value: &Value, format: DataFormat, indent: &str) -> ToolResult {
    match format {
        DataFormat::Json => Ok(write_json(value, indent) + "\n"),
        DataFormat::Yaml => serde_yaml_ng::to_string(value)
            .map_err(|e| ToolError::new(format!("Can't write YAML: {e}"))),
        DataFormat::Toml => write_toml(value),
        DataFormat::Xml => Ok(write_xml(value, indent)),
        DataFormat::Csv => {
            let (columns, rows) = table(value)?;
            Ok(super::csv::write_csv(&columns, &rows))
        }
        DataFormat::Markdown => {
            let (columns, rows) = table(value)?;
            Ok(markdown_table(&columns, &rows))
        }
    }
}

/// Pretty JSON indented with `indent`, without a final line break.
pub fn write_json(value: &Value, indent: &str) -> String {
    use serde::Serialize;
    let mut out = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
    let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
    value
        .serialize(&mut ser)
        .expect("JSON values always serialise");
    String::from_utf8(out).expect("JSON is UTF-8")
}

// ---------------------------------------------------------------- YAML

fn read_yaml(text: &str) -> Result<Value, ToolError> {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(text).map_err(|e| {
        let offset = e
            .location()
            .map(|l| super::line_col_to_offset(text, l.line(), l.column()));
        ToolError {
            message: format!("Not valid YAML: {e}"),
            offset,
        }
    })?;
    Ok(from_yaml(yaml))
}

fn from_yaml(v: serde_yaml_ng::Value) -> Value {
    use serde_yaml_ng::Value as Y;
    match v {
        Y::Null => Value::Null,
        Y::Bool(b) => Value::Bool(b),
        Y::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into()
            } else if let Some(u) = n.as_u64() {
                u.into()
            } else {
                let f = n.as_f64().unwrap_or(f64::NAN);
                serde_json::Number::from_f64(f)
                    .map_or_else(|| Value::String(n.to_string()), Value::Number)
            }
        }
        Y::String(s) => Value::String(s),
        Y::Sequence(items) => Value::Array(items.into_iter().map(from_yaml).collect()),
        Y::Mapping(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (yaml_key(k), from_yaml(v)))
                .collect(),
        ),
        Y::Tagged(tagged) => from_yaml(tagged.value),
    }
}

/// A YAML key as a JSON object key (`1` becomes `"1"`).
fn yaml_key(k: serde_yaml_ng::Value) -> String {
    use serde_yaml_ng::Value as Y;
    match k {
        Y::String(s) => s,
        Y::Null => "null".into(),
        Y::Bool(b) => b.to_string(),
        Y::Number(n) => n.to_string(),
        other => serde_yaml_ng::to_string(&other)
            .unwrap_or_default()
            .trim()
            .to_owned(),
    }
}

// ---------------------------------------------------------------- TOML

fn read_toml(text: &str) -> Result<Value, ToolError> {
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| ToolError {
        message: format!("Not valid TOML: {}", e.message()),
        offset: e.span().map(|s| s.start),
    })?;
    Ok(from_toml(toml::Value::Table(table)))
}

fn from_toml(v: toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => i.into(),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map_or_else(|| Value::String(f.to_string()), Value::Number),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(items) => Value::Array(items.into_iter().map(from_toml).collect()),
        toml::Value::Table(t) => {
            Value::Object(t.into_iter().map(|(k, v)| (k, from_toml(v))).collect())
        }
    }
}

fn write_toml(value: &Value) -> ToolResult {
    let table = match value {
        Value::Object(map) => to_toml_table(map, "")?,
        // A list of tables becomes `[[item]]` entries.
        Value::Array(_) => {
            let mut map = Map::new();
            map.insert("item".into(), value.clone());
            to_toml_table(&map, "")?
        }
        _ => {
            return Err(ToolError::new(
                "TOML needs a table (an object) at the top level.",
            ));
        }
    };
    toml::to_string_pretty(&table).map_err(|e| ToolError::new(format!("Can't write TOML: {e}")))
}

fn to_toml_table(map: &Map<String, Value>, path: &str) -> Result<toml::Table, ToolError> {
    let mut table = toml::Table::new();
    for (k, v) in map {
        let here = if path.is_empty() {
            k.clone()
        } else {
            format!("{path}.{k}")
        };
        // TOML has no null; a missing key is the closest thing.
        if v.is_null() {
            continue;
        }
        table.insert(k.clone(), to_toml(v, &here)?);
    }
    Ok(table)
}

fn to_toml(v: &Value, path: &str) -> Result<toml::Value, ToolError> {
    Ok(match v {
        Value::Null => {
            return Err(ToolError::new(format!(
                "TOML has no null value, and {path} holds one inside a list."
            )));
        }
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => toml::Value::Integer(i),
            None => toml::Value::Float(n.as_f64().unwrap_or(f64::NAN)),
        },
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Array(items) => toml::Value::Array(
            items
                .iter()
                .map(|i| to_toml(i, path))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(map) => toml::Value::Table(to_toml_table(map, path)?),
    })
}

// ---------------------------------------------------------------- XML

/// The key an XML attribute gets in JSON, and the key for text next to
/// attributes or child elements.
const ATTR: &str = "@";
const TEXT: &str = "#text";

fn read_xml(text: &str) -> Result<Value, ToolError> {
    let doc = super::xml::parse_strict(text)?;
    let root = doc.root_element();
    let mut map = Map::new();
    map.insert(qualified_name(root), xml_element(root));
    Ok(Value::Object(map))
}

fn qualified_name(node: roxmltree::Node) -> String {
    let tag = node.tag_name();
    match tag.namespace().and_then(|ns| node.lookup_prefix(ns)) {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}:{}", tag.name()),
        _ => tag.name().to_owned(),
    }
}

/// An element as JSON: its text when it has nothing else, otherwise an
/// object of `@attributes`, children (repeated ones as a list) and `#text`.
fn xml_element(node: roxmltree::Node) -> Value {
    let mut map = Map::new();
    for a in node.attributes() {
        let name = match a.namespace().and_then(|ns| node.lookup_prefix(ns)) {
            Some(p) if !p.is_empty() => format!("{p}:{}", a.name()),
            _ => a.name().to_owned(),
        };
        map.insert(format!("{ATTR}{name}"), Value::String(a.value().to_owned()));
    }
    let mut text = String::new();
    let mut has_children = false;
    for child in node.children() {
        if child.is_element() {
            has_children = true;
            let name = qualified_name(child);
            let value = xml_element(child);
            match map.get_mut(&name) {
                Some(Value::Array(list)) => list.push(value),
                Some(existing) => {
                    let first = existing.take();
                    *existing = Value::Array(vec![first, value]);
                }
                None => {
                    map.insert(name, value);
                }
            }
        } else if child.is_text() {
            text.push_str(child.text().unwrap_or(""));
        }
    }
    let text = text.trim();
    if map.is_empty() {
        return if text.is_empty() && !has_children {
            Value::Null
        } else {
            Value::String(text.to_owned())
        };
    }
    if !text.is_empty() {
        map.insert(TEXT.into(), Value::String(text.to_owned()));
    }
    Value::Object(map)
}

fn write_xml(value: &Value, indent: &str) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    match value {
        Value::Object(map) if map.len() == 1 && !map.keys().next().unwrap().starts_with(ATTR) => {
            let (name, v) = map.iter().next().unwrap();
            xml_write_element(&mut out, name, v, indent, 0);
        }
        Value::Array(_) => {
            let mut map = Map::new();
            map.insert("item".into(), value.clone());
            xml_write_element(&mut out, "root", &Value::Object(map), indent, 0);
        }
        _ => xml_write_element(&mut out, "root", value, indent, 0),
    }
    out
}

/// A JSON key as an XML name: characters a name can't hold become `_`.
fn xml_name(key: &str) -> String {
    let mut name: String = key
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !name
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
    {
        name.insert(0, '_');
    }
    name
}

fn scalar_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn xml_write_element(out: &mut String, key: &str, value: &Value, indent: &str, depth: usize) {
    if let Value::Array(items) = value {
        for item in items {
            xml_write_element(out, key, item, indent, depth);
        }
        return;
    }
    let name = xml_name(key);
    let pad = indent.repeat(depth);
    out.push_str(&pad);
    out.push('<');
    out.push_str(&name);
    let Value::Object(map) = value else {
        let text = scalar_text(value);
        if text.is_empty() {
            out.push_str("/>\n");
        } else {
            out.push('>');
            out.push_str(&escape_text(&text));
            out.push_str(&format!("</{name}>\n"));
        }
        return;
    };
    for (k, v) in map {
        if let Some(attr) = k.strip_prefix(ATTR) {
            out.push_str(&format!(
                " {}=\"{}\"",
                xml_name(attr),
                escape_attr(&scalar_text(v))
            ));
        }
    }
    let children: Vec<(&String, &Value)> = map
        .iter()
        .filter(|(k, _)| !k.starts_with(ATTR) && *k != TEXT)
        .collect();
    let text = map.get(TEXT).map(scalar_text).unwrap_or_default();
    if children.is_empty() {
        if text.is_empty() {
            out.push_str("/>\n");
        } else {
            out.push('>');
            out.push_str(&escape_text(&text));
            out.push_str(&format!("</{name}>\n"));
        }
        return;
    }
    out.push_str(">\n");
    if !text.is_empty() {
        out.push_str(&indent.repeat(depth + 1));
        out.push_str(&escape_text(&text));
        out.push('\n');
    }
    for (k, v) in children {
        xml_write_element(out, k, v, indent, depth + 1);
    }
    out.push_str(&pad);
    out.push_str(&format!("</{name}>\n"));
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    escape_text(s).replace('"', "&quot;")
}

// ---------------------------------------------------------------- CSV and tables

/// Rows as a list of objects named by the header. With `typed`, columns
/// whose values are all numbers become numbers (numbers with leading zeros
/// stay text); otherwise every value is text.
fn read_csv(text: &str, typed: bool) -> Value {
    use super::csv::{self, ColType};
    let mut format = csv::detect(text, None);
    format.header = true;
    let Ok(table) = csv::parse(text, &format) else {
        return Value::Array(Vec::new());
    };
    let types = if typed {
        csv::infer_types(&table, format.delimiter)
    } else {
        vec![ColType::Text; table.columns.len()]
    };
    let rows = table
        .rows
        .iter()
        .map(|row| {
            let map = table
                .columns
                .iter()
                .zip(row)
                .zip(&types)
                .map(|((name, v), ty)| {
                    let t = v.trim();
                    let value = match ty {
                        _ if t.is_empty() && *ty != ColType::Text => Value::Null,
                        ColType::Integer => t
                            .parse::<i64>()
                            .map_or_else(|_| Value::String(v.clone()), Value::from),
                        ColType::Real => csv::parse_real(t, format.delimiter)
                            .and_then(serde_json::Number::from_f64)
                            .map_or_else(|| Value::String(v.clone()), Value::Number),
                        _ => Value::String(v.clone()),
                    };
                    (name.clone(), value)
                })
                .collect();
            Value::Object(map)
        })
        .collect();
    Value::Array(rows)
}

fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
        other => other.to_string(),
    }
}

/// Columns and rows from a list of objects, a list of lists (the first is
/// the header), a list of values, or an object holding one such list.
fn table(value: &Value) -> Result<(Vec<String>, Vec<Vec<String>>), ToolError> {
    let items = match value {
        Value::Array(items) => items,
        Value::Object(map) => {
            let lists: Vec<&Value> = map.values().filter(|v| v.is_array()).collect();
            if map.len() == 1 && lists.len() == 1 {
                return table(lists[0]);
            }
            return Ok((
                map.keys().cloned().collect(),
                vec![map.values().map(cell).collect()],
            ));
        }
        _ => {
            return Err(ToolError::new(
                "A table needs a list of rows (objects or lists).",
            ));
        }
    };
    if items.iter().all(Value::is_object) {
        let mut columns: Vec<String> = Vec::new();
        for item in items {
            for k in item.as_object().unwrap().keys() {
                if !columns.contains(k) {
                    columns.push(k.clone());
                }
            }
        }
        let rows = items
            .iter()
            .map(|item| {
                let map = item.as_object().unwrap();
                columns
                    .iter()
                    .map(|c| map.get(c).map(cell).unwrap_or_default())
                    .collect()
            })
            .collect();
        return Ok((columns, rows));
    }
    if items.iter().all(Value::is_array) && !items.is_empty() {
        let mut rows: Vec<Vec<String>> = items
            .iter()
            .map(|r| r.as_array().unwrap().iter().map(cell).collect())
            .collect();
        let header = rows.remove(0);
        let width = rows
            .iter()
            .map(Vec::len)
            .chain([header.len()])
            .max()
            .unwrap_or(0);
        let mut columns = header;
        columns.resize(width, String::new());
        for r in &mut rows {
            r.resize(width, String::new());
        }
        return Ok((columns, rows));
    }
    Ok((
        vec!["value".into()],
        items.iter().map(|v| vec![cell(v)]).collect(),
    ))
}

/// A GitHub-style Markdown table with the columns lined up.
pub fn markdown_table(columns: &[String], rows: &[Vec<String>]) -> String {
    let esc = |s: &str| {
        s.replace('|', "\\|")
            .replace("\r\n", "<br>")
            .replace('\n', "<br>")
    };
    let columns: Vec<String> = columns.iter().map(|c| esc(c)).collect();
    let rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| r.iter().map(|c| esc(c)).collect())
        .collect();
    let width = |s: &str| unicode_width::UnicodeWidthStr::width(s);
    let widths: Vec<usize> = (0..columns.len())
        .map(|i| {
            rows.iter()
                .filter_map(|r| r.get(i))
                .chain([&columns[i]])
                .map(|c| width(c))
                .max()
                .unwrap_or(0)
                .max(3)
        })
        .collect();
    let line = |cells: &[String]| {
        let parts: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(c, w)| format!("{c}{}", " ".repeat(w - width(c))))
            .collect();
        format!("| {} |\n", parts.join(" | "))
    };
    let mut out = line(&columns);
    let rule: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    out.push_str(&format!("| {} |\n", rule.join(" | ")));
    for r in &rows {
        out.push_str(&line(r));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use DataFormat::*;

    #[test]
    fn json_yaml_toml_round_trip() {
        let json = "{\n  \"name\": \"Ettin\",\n  \"zeta\": 1,\n  \"alpha\": [true, 2.5]\n}\n";
        let yaml = convert(json, Json, Yaml, "  ").unwrap();
        assert_eq!(yaml, "name: Ettin\nzeta: 1\nalpha:\n- true\n- 2.5\n");
        let back = convert(&yaml, Yaml, Json, "  ").unwrap();
        assert_eq!(
            back,
            "{\n  \"name\": \"Ettin\",\n  \"zeta\": 1,\n  \"alpha\": [\n    true,\n    2.5\n  ]\n}\n"
        );
        let toml = convert(json, Json, Toml, "  ").unwrap();
        assert!(toml.starts_with("name = \"Ettin\"\nzeta = 1\n"), "{toml}");
        let back = convert(&toml, Toml, Json, "    ").unwrap();
        assert!(back.contains("    \"zeta\": 1"));
    }

    #[test]
    fn yaml_keys_and_errors() {
        let out = convert("1: one\ntrue: yes\n", Yaml, Json, "  ").unwrap();
        assert!(out.contains("\"1\": \"one\""));
        let e = convert("a: [1, 2\n", Yaml, Json, "  ").unwrap_err();
        assert!(e.message.starts_with("Not valid YAML"));
        let e = convert("{\"a\": }", Json, Yaml, "  ").unwrap_err();
        assert_eq!(e.offset, Some(6));
    }

    #[test]
    fn toml_limits() {
        let out = convert("[1, 2]", Json, Toml, "  ").unwrap();
        assert!(out.starts_with("item = ["), "{out}");
        assert!(convert("\"x\"", Json, Toml, "  ").is_err());
        // Null keys are left out; a null inside a list can't be written.
        assert_eq!(
            convert("{\"a\": null, \"b\": 1}", Json, Toml, "  ").unwrap(),
            "b = 1"
        );
        assert!(convert("{\"a\": [null]}", Json, Toml, "  ").is_err());
        let out = convert("when = 1979-05-27T07:32:00Z\n", Toml, Json, "  ").unwrap();
        assert!(out.contains("\"1979-05-27T07:32:00Z\""));
    }

    #[test]
    fn xml_to_json_and_back() {
        let xml = r#"<order id="7"><item>a</item><item>b</item><note lang="en">hi</note><empty/></order>"#;
        let json = convert(xml, Xml, Json, "  ").unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["order"]["@id"], "7");
        assert_eq!(v["order"]["item"], serde_json::json!(["a", "b"]));
        assert_eq!(v["order"]["note"]["#text"], "hi");
        assert_eq!(v["order"]["empty"], Value::Null);
        let back = convert(&json, Json, Xml, "  ").unwrap();
        assert_eq!(
            back,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<order id=\"7\">\n  <item>a</item>\n  <item>b</item>\n  <note lang=\"en\">hi</note>\n  <empty/>\n</order>"
        );
        let out = convert(r#"{"a b": 1, "2x": "<&>"}"#, Json, Xml, " ").unwrap();
        assert!(out.contains("<root>\n <a_b>1</a_b>\n <_2x>&lt;&amp;&gt;</_2x>\n</root>"));
    }

    #[test]
    fn csv_and_tables() {
        let csv = "id,zip,name,score\n1,02134,Ann,3.5\n2,90210,\"Lee, Bo\",\n";
        let json = convert(csv, Csv, Json, "  ").unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v[0]["id"], 1);
        assert_eq!(v[0]["zip"], "02134");
        assert_eq!(v[1]["name"], "Lee, Bo");
        assert_eq!(v[0]["score"], 3.5);
        assert_eq!(v[1]["score"], Value::Null);
        let back = convert(&json, Json, Csv, "  ").unwrap();
        assert_eq!(
            back,
            "id,zip,name,score\n1,02134,Ann,3.5\n2,90210,\"Lee, Bo\",\n"
        );
        let md = convert("a,b\nx|y,2.50\n", Csv, Markdown, "  ").unwrap();
        assert_eq!(md, "| a    | b    |\n| ---- | ---- |\n| x\\|y | 2.50 |\n");
        // An object holding one list is that list.
        let out = convert(r#"{"rows": [{"a": 1}, {"b": {"c": 2}}]}"#, Json, Csv, "").unwrap();
        assert_eq!(out, "a,b\n1,\n,\"{\"\"c\"\":2}\"");
        assert!(convert("3", Json, Csv, "").is_err());
    }
}
