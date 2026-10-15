//! XML Schema (XSD 1.0) validation, written from the W3C XML Schema
//! recommendation (parts 1 and 2).
//!
//! It covers the practical subset of XSD 1.0: global and local elements and
//! attributes, named and anonymous types, sequence/choice/all with any
//! occurrence counts, model and attribute groups, simple and complex content
//! derivation, simple types with facets, lists and unions, the built-in
//! types, wildcards, substitution groups, xsi:type and xsi:nil, ID/IDREF
//! checks, and xs:include / xs:import of local files.
//!
//! Not supported: identity constraints (xs:key, xs:keyref, xs:unique are
//! ignored), xs:redefine, block/final derivation control, and fetching
//! schemas over the network (imports of remote namespaces are validated
//! laxly).
#![allow(dead_code)]

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;
use roxmltree::{Document, Node};

pub use super::Problem;
use super::{ToolError, line_col, xml};

const XS: &str = "http://www.w3.org/2001/XMLSchema";
const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// Validates `doc_text` against the XSD `schema_text`. Includes and imports
/// are read relative to `schema_dir`. Returns the problems found (empty when
/// the document is valid); `Err` means the document or schema couldn't be
/// read.
pub fn validate_xsd(
    doc_text: &str,
    schema_text: &str,
    schema_dir: Option<&Path>,
) -> Result<Vec<Problem>, ToolError> {
    // Deeply nested documents recurse deeply; use a roomy stack.
    std::thread::scope(|s| {
        match std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn_scoped(s, || run(doc_text, schema_text, schema_dir))
        {
            Ok(h) => h
                .join()
                .unwrap_or_else(|_| Err(ToolError::new("Validation failed unexpectedly."))),
            Err(_) => run(doc_text, schema_text, schema_dir),
        }
    })
}

/// The schema a document names with `xsi:noNamespaceSchemaLocation` or
/// `xsi:schemaLocation` (the entry for the root's namespace, else the first).
pub fn schema_location(doc_text: &str) -> Option<String> {
    let doc = Document::parse_with_options(doc_text, xml::parse_options()).ok()?;
    let root = doc.root_element();
    if let Some(v) = root.attribute((XSI, "noNamespaceSchemaLocation")) {
        return Some(v.trim().to_owned());
    }
    let pairs = root.attribute((XSI, "schemaLocation"))?;
    let tokens: Vec<&str> = pairs.split_whitespace().collect();
    let ns = root.tag_name().namespace().unwrap_or("");
    tokens
        .chunks(2)
        .find(|c| c.len() == 2 && c[0] == ns)
        .or_else(|| tokens.chunks(2).find(|c| c.len() == 2))
        .map(|c| c[1].to_owned())
}

fn run(
    doc_text: &str,
    schema_text: &str,
    schema_dir: Option<&Path>,
) -> Result<Vec<Problem>, ToolError> {
    let schema = Loader::load(schema_text, schema_dir)?;
    let doc = xml::parse_strict(doc_text).map_err(|e| ToolError {
        message: format!("The XML document isn't well-formed: {}", e.message),
        offset: e.offset,
    })?;
    let mut v = Validator {
        s: &schema,
        text: doc_text,
        problems: Vec::new(),
        ids: HashSet::new(),
        idrefs: Vec::new(),
    };
    v.root(doc.root_element());
    Ok(v.finish())
}

// ----------------------------------------------------------------------
// Built-in types and values

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ws {
    Preserve,
    Replace,
    Collapse,
}

fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

fn normalize(s: &str, ws: Ws) -> String {
    match ws {
        Ws::Preserve => s.to_owned(),
        Ws::Replace => s.replace(['\t', '\n', '\r'], " "),
        Ws::Collapse => s
            .split(is_xml_space)
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateKind {
    DateTime,
    Date,
    Time,
    GYearMonth,
    GYear,
    GMonthDay,
    GDay,
    GMonth,
}

/// The primitive a type derives from, which decides how facets compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Prim {
    Str,
    Bool,
    Decimal,
    Float,
    Duration,
    Date(DateKind),
    Hex,
    Base64,
    QName,
    List,
    Union,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IdKind {
    Other,
    Id,
    IdRef,
    IdRefs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum B {
    AnySimple,
    String,
    NormalizedString,
    Token,
    Language,
    Name,
    NCName,
    Id,
    IdRef,
    IdRefs,
    Entity,
    Entities,
    NmToken,
    NmTokens,
    QName,
    Notation,
    AnyUri,
    Boolean,
    Decimal,
    Integer,
    NonPositiveInteger,
    NegativeInteger,
    Long,
    Int,
    Short,
    Byte,
    NonNegativeInteger,
    UnsignedLong,
    UnsignedInt,
    UnsignedShort,
    UnsignedByte,
    PositiveInteger,
    Float,
    Double,
    Duration,
    DateTime,
    Date,
    Time,
    GYearMonth,
    GYear,
    GMonthDay,
    GDay,
    GMonth,
    HexBinary,
    Base64Binary,
}

const BUILTINS: &[(&str, B)] = &[
    ("anySimpleType", B::AnySimple),
    ("string", B::String),
    ("normalizedString", B::NormalizedString),
    ("token", B::Token),
    ("language", B::Language),
    ("Name", B::Name),
    ("NCName", B::NCName),
    ("ID", B::Id),
    ("IDREF", B::IdRef),
    ("IDREFS", B::IdRefs),
    ("ENTITY", B::Entity),
    ("ENTITIES", B::Entities),
    ("NMTOKEN", B::NmToken),
    ("NMTOKENS", B::NmTokens),
    ("QName", B::QName),
    ("NOTATION", B::Notation),
    ("anyURI", B::AnyUri),
    ("boolean", B::Boolean),
    ("decimal", B::Decimal),
    ("integer", B::Integer),
    ("nonPositiveInteger", B::NonPositiveInteger),
    ("negativeInteger", B::NegativeInteger),
    ("long", B::Long),
    ("int", B::Int),
    ("short", B::Short),
    ("byte", B::Byte),
    ("nonNegativeInteger", B::NonNegativeInteger),
    ("unsignedLong", B::UnsignedLong),
    ("unsignedInt", B::UnsignedInt),
    ("unsignedShort", B::UnsignedShort),
    ("unsignedByte", B::UnsignedByte),
    ("positiveInteger", B::PositiveInteger),
    ("float", B::Float),
    ("double", B::Double),
    ("duration", B::Duration),
    ("dateTime", B::DateTime),
    ("date", B::Date),
    ("time", B::Time),
    ("gYearMonth", B::GYearMonth),
    ("gYear", B::GYear),
    ("gMonthDay", B::GMonthDay),
    ("gDay", B::GDay),
    ("gMonth", B::GMonth),
    ("hexBinary", B::HexBinary),
    ("base64Binary", B::Base64Binary),
];

fn builtin(local: &str) -> Option<B> {
    BUILTINS.iter().find(|(n, _)| *n == local).map(|&(_, b)| b)
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric()
        || matches!(c, '_' | '-' | '.' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}')
}

fn is_ncname(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(is_name_start) && cs.all(is_name_char)
}

fn is_name(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(|c| is_name_start(c) || c == ':')
        && cs.all(|c| is_name_char(c) || c == ':')
}

fn is_nmtoken(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| is_name_char(c) || c == ':')
}

fn is_qname(s: &str) -> bool {
    match s.split_once(':') {
        Some((p, l)) => is_ncname(p) && is_ncname(l),
        None => is_ncname(s),
    }
}

/// A decimal number held exactly: sign, integer digits (no leading zeros)
/// and fraction digits (no trailing zeros).
#[derive(Clone, Debug, PartialEq)]
struct Dec {
    neg: bool,
    int: String,
    frac: String,
}

fn parse_decimal(s: &str) -> Option<Dec> {
    let (neg, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let (i, f) = rest.split_once('.').unwrap_or((rest, ""));
    if (i.is_empty() && f.is_empty())
        || !i.bytes().all(|b| b.is_ascii_digit())
        || !f.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let int = i.trim_start_matches('0').to_owned();
    let frac = f.trim_end_matches('0').to_owned();
    let zero = int.is_empty() && frac.is_empty();
    Some(Dec {
        neg: neg && !zero,
        int,
        frac,
    })
}

fn cmp_magnitude(a: &Dec, b: &Dec) -> Ordering {
    a.int
        .len()
        .cmp(&b.int.len())
        .then_with(|| a.int.cmp(&b.int))
        .then_with(|| a.frac.cmp(&b.frac))
}

fn cmp_decimal(a: &Dec, b: &Dec) -> Ordering {
    match (a.neg, b.neg) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (false, false) => cmp_magnitude(a, b),
        (true, true) => cmp_magnitude(b, a),
    }
}

fn parse_float(s: &str) -> Option<f64> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$").unwrap()
    });
    match s {
        "INF" | "+INF" => Some(f64::INFINITY),
        "-INF" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        _ if RE.is_match(s) => s.parse().ok(),
        _ => None,
    }
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 in the proleptic Gregorian calendar.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Parses a date/time value into seconds (timezone-adjusted) for comparing.
fn parse_date(kind: DateKind, s: &str) -> Option<f64> {
    const TZ: &str = r"(Z|[+-][0-9]{2}:[0-9]{2})?";
    static RES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        let year = r"(-?[0-9]{4,})";
        let time = r"([0-9]{2}):([0-9]{2}):([0-9]{2}(?:\.[0-9]+)?)";
        [
            format!("^{year}-([0-9]{{2}})-([0-9]{{2}})T{time}{TZ}$"),
            format!("^{year}-([0-9]{{2}})-([0-9]{{2}}){TZ}$"),
            format!("^{time}{TZ}$"),
            format!("^{year}-([0-9]{{2}}){TZ}$"),
            format!("^{year}{TZ}$"),
            format!("^--([0-9]{{2}})-([0-9]{{2}}){TZ}$"),
            format!("^---([0-9]{{2}}){TZ}$"),
            format!("^--([0-9]{{2}}){TZ}$"),
        ]
        .iter()
        .map(|p| Regex::new(p).unwrap())
        .collect()
    });
    let caps = RES[kind as usize].captures(s)?;
    let g: Vec<Option<&str>> = caps.iter().skip(1).map(|m| m.map(|m| m.as_str())).collect();
    let num = |i: usize| -> Option<u32> { g.get(i).copied().flatten()?.parse().ok() };
    let (mut year, mut month, mut day) = (2000i64, 1u32, 1u32);
    let (mut hour, mut minute, mut second) = (0u32, 0u32, 0f64);
    let mut year_text = None;
    let tz_index;
    match kind {
        DateKind::DateTime => {
            year_text = g[0];
            month = num(1)?;
            day = num(2)?;
            hour = num(3)?;
            minute = num(4)?;
            second = g[5]?.parse().ok()?;
            tz_index = 6;
        }
        DateKind::Date => {
            year_text = g[0];
            month = num(1)?;
            day = num(2)?;
            tz_index = 3;
        }
        DateKind::Time => {
            hour = num(0)?;
            minute = num(1)?;
            second = g[2]?.parse().ok()?;
            tz_index = 3;
        }
        DateKind::GYearMonth => {
            year_text = g[0];
            month = num(1)?;
            tz_index = 2;
        }
        DateKind::GYear => {
            year_text = g[0];
            tz_index = 1;
        }
        DateKind::GMonthDay => {
            month = num(0)?;
            day = num(1)?;
            tz_index = 2;
        }
        DateKind::GDay => {
            day = num(0)?;
            tz_index = 1;
        }
        DateKind::GMonth => {
            month = num(0)?;
            tz_index = 1;
        }
    }
    if let Some(y) = year_text {
        let digits = y.trim_start_matches('-');
        if digits.len() > 4 && digits.starts_with('0') {
            return None;
        }
        year = y.parse().ok()?;
        if year == 0 {
            return None;
        }
    }
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    if minute > 59 || second >= 60.0 || hour > 24 || (hour == 24 && (minute > 0 || second > 0.0)) {
        return None;
    }
    let mut offset = 0i64;
    if let Some(tz) = g.get(tz_index).copied().flatten()
        && tz != "Z"
    {
        let h: i64 = tz[1..3].parse().ok()?;
        let m: i64 = tz[4..6].parse().ok()?;
        if h > 14 || m > 59 || (h == 14 && m > 0) {
            return None;
        }
        offset = (h * 60 + m) * if tz.starts_with('-') { -1 } else { 1 };
    }
    let days = days_from_civil(year, month, day);
    Some(
        days as f64 * 86400.0 + f64::from(hour) * 3600.0 + f64::from(minute) * 60.0 + second
            - offset as f64 * 60.0,
    )
}

/// Parses a duration into approximate seconds (months count as 30.44 days).
fn parse_duration(s: &str) -> Option<f64> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(-)?P(?:([0-9]+)Y)?(?:([0-9]+)M)?(?:([0-9]+)D)?(T(?:([0-9]+)H)?(?:([0-9]+)M)?(?:([0-9]+(?:\.[0-9]+)?)S)?)?$",
        )
        .unwrap()
    });
    let c = RE.captures(s)?;
    let f = |i: usize| c.get(i).map(|m| m.as_str().parse::<f64>().unwrap_or(0.0));
    let parts = [f(2), f(3), f(4), f(6), f(7), f(8)];
    if parts.iter().all(Option::is_none) {
        return None;
    }
    if c.get(5).is_some() && parts[3..].iter().all(Option::is_none) {
        return None;
    }
    let v = |o: Option<f64>| o.unwrap_or(0.0);
    let secs = (v(parts[0]) * 12.0 + v(parts[1])) * 30.436875 * 86400.0
        + v(parts[2]) * 86400.0
        + v(parts[3]) * 3600.0
        + v(parts[4]) * 60.0
        + v(parts[5]);
    Some(if c.get(1).is_some() { -secs } else { secs })
}

fn base64_octets(s: &str) -> Option<usize> {
    let b: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    if !b.len().is_multiple_of(4) {
        return None;
    }
    let pad = b.iter().rev().take_while(|&&c| c == b'=').count();
    if pad > 2 {
        return None;
    }
    let body = &b[..b.len() - pad];
    if !body
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == b'+' || *c == b'/')
    {
        return None;
    }
    Some(b.len() / 4 * 3 - pad)
}

impl B {
    fn whitespace(self) -> Ws {
        match self {
            B::String | B::AnySimple => Ws::Preserve,
            B::NormalizedString => Ws::Replace,
            _ => Ws::Collapse,
        }
    }

    fn prim(self) -> Prim {
        match self {
            B::Boolean => Prim::Bool,
            B::Decimal
            | B::Integer
            | B::NonPositiveInteger
            | B::NegativeInteger
            | B::Long
            | B::Int
            | B::Short
            | B::Byte
            | B::NonNegativeInteger
            | B::UnsignedLong
            | B::UnsignedInt
            | B::UnsignedShort
            | B::UnsignedByte
            | B::PositiveInteger => Prim::Decimal,
            B::Float | B::Double => Prim::Float,
            B::Duration => Prim::Duration,
            B::DateTime => Prim::Date(DateKind::DateTime),
            B::Date => Prim::Date(DateKind::Date),
            B::Time => Prim::Date(DateKind::Time),
            B::GYearMonth => Prim::Date(DateKind::GYearMonth),
            B::GYear => Prim::Date(DateKind::GYear),
            B::GMonthDay => Prim::Date(DateKind::GMonthDay),
            B::GDay => Prim::Date(DateKind::GDay),
            B::GMonth => Prim::Date(DateKind::GMonth),
            B::HexBinary => Prim::Hex,
            B::Base64Binary => Prim::Base64,
            B::QName | B::Notation => Prim::QName,
            B::IdRefs | B::Entities | B::NmTokens => Prim::List,
            _ => Prim::Str,
        }
    }

    fn id_kind(self) -> IdKind {
        match self {
            B::Id => IdKind::Id,
            B::IdRef => IdKind::IdRef,
            B::IdRefs => IdKind::IdRefs,
            _ => IdKind::Other,
        }
    }

    /// How the type is named in messages.
    fn label(self) -> &'static str {
        match self {
            B::Language => "language code (like en or en-US)",
            B::Name => "XML name",
            B::NCName => "name without a colon",
            B::Id => "ID (a name without a colon)",
            B::IdRef => "IDREF (a name without a colon)",
            B::IdRefs => "list of IDREFs",
            B::Entity => "entity name",
            B::Entities => "list of entity names",
            B::NmToken => "name token",
            B::NmTokens => "list of name tokens",
            B::QName => "qualified name",
            B::Notation => "notation name",
            B::Boolean => "boolean (true, false, 1 or 0)",
            B::Decimal => "decimal number",
            B::Integer => "integer",
            B::NonPositiveInteger => "integer that is 0 or less",
            B::NegativeInteger => "negative integer",
            B::Long => "long integer",
            B::Int => "int (a 32-bit integer)",
            B::Short => "short (a 16-bit integer)",
            B::Byte => "byte (-128 to 127)",
            B::NonNegativeInteger => "integer that is 0 or more",
            B::UnsignedLong => "unsigned long integer",
            B::UnsignedInt => "unsigned int",
            B::UnsignedShort => "unsigned short (0 to 65535)",
            B::UnsignedByte => "unsigned byte (0 to 255)",
            B::PositiveInteger => "positive integer",
            B::Float => "float",
            B::Double => "double",
            B::Duration => "duration (like P1Y2M3DT4H5M6S)",
            B::DateTime => "date and time (YYYY-MM-DDThh:mm:ss)",
            B::Date => "date (YYYY-MM-DD)",
            B::Time => "time (hh:mm:ss)",
            B::GYearMonth => "year and month (YYYY-MM)",
            B::GYear => "year (YYYY)",
            B::GMonthDay => "month and day (--MM-DD)",
            B::GDay => "day (---DD)",
            B::GMonth => "month (--MM)",
            B::HexBinary => "hexadecimal binary value",
            B::Base64Binary => "base64 value",
            B::AnyUri => "URI",
            _ => "string",
        }
    }

    fn integer_range(self) -> (Option<&'static str>, Option<&'static str>) {
        match self {
            B::NonPositiveInteger => (None, Some("0")),
            B::NegativeInteger => (None, Some("-1")),
            B::Long => (Some("-9223372036854775808"), Some("9223372036854775807")),
            B::Int => (Some("-2147483648"), Some("2147483647")),
            B::Short => (Some("-32768"), Some("32767")),
            B::Byte => (Some("-128"), Some("127")),
            B::NonNegativeInteger => (Some("0"), None),
            B::UnsignedLong => (Some("0"), Some("18446744073709551615")),
            B::UnsignedInt => (Some("0"), Some("4294967295")),
            B::UnsignedShort => (Some("0"), Some("65535")),
            B::UnsignedByte => (Some("0"), Some("255")),
            B::PositiveInteger => (Some("1"), None),
            _ => (None, None),
        }
    }

    /// Checks a whitespace-normalized value.
    fn check(self, v: &str) -> bool {
        static LANGUAGE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^[a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*$").unwrap());
        match self {
            B::AnySimple | B::String | B::NormalizedString | B::Token | B::AnyUri => true,
            B::Language => LANGUAGE.is_match(v),
            B::Name => is_name(v),
            B::NCName | B::Id | B::IdRef | B::Entity => is_ncname(v),
            B::NmToken => is_nmtoken(v),
            B::IdRefs | B::Entities => !v.is_empty() && v.split(' ').all(is_ncname),
            B::NmTokens => !v.is_empty() && v.split(' ').all(is_nmtoken),
            B::QName | B::Notation => is_qname(v),
            B::Boolean => matches!(v, "true" | "false" | "1" | "0"),
            B::Decimal => parse_decimal(v).is_some(),
            B::Float | B::Double => parse_float(v).is_some(),
            B::Duration => parse_duration(v).is_some(),
            B::HexBinary => v.len().is_multiple_of(2) && v.bytes().all(|b| b.is_ascii_hexdigit()),
            B::Base64Binary => base64_octets(v).is_some(),
            B::DateTime
            | B::Date
            | B::Time
            | B::GYearMonth
            | B::GYear
            | B::GMonthDay
            | B::GDay
            | B::GMonth => match self.prim() {
                Prim::Date(k) => parse_date(k, v).is_some(),
                _ => false,
            },
            _ => {
                // The integer types.
                if v.contains('.') {
                    return false;
                }
                let Some(d) = parse_decimal(v) else {
                    return false;
                };
                let (min, max) = self.integer_range();
                let below = min.is_some_and(|m| {
                    cmp_decimal(&d, &parse_decimal(m).expect("bound")) == Ordering::Less
                });
                let above = max.is_some_and(|m| {
                    cmp_decimal(&d, &parse_decimal(m).expect("bound")) == Ordering::Greater
                });
                !below && !above
            }
        }
    }
}

/// Shows a value in a message, shortened if long.
fn show(v: &str) -> String {
    let mut s: String = v.chars().take(40).collect();
    if s.len() < v.len() {
        s.push('…');
    }
    format!("\"{s}\"")
}

// ----------------------------------------------------------------------
// XSD regular expressions

/// Unicode blocks XSD patterns can name with `\p{IsBlock}`.
const BLOCKS: &[(&str, &str)] = &[
    ("BasicLatin", r"\x{0}-\x{7F}"),
    ("Latin-1Supplement", r"\x{80}-\x{FF}"),
    ("LatinExtended-A", r"\x{100}-\x{17F}"),
    ("LatinExtended-B", r"\x{180}-\x{24F}"),
    ("IPAExtensions", r"\x{250}-\x{2AF}"),
    ("SpacingModifierLetters", r"\x{2B0}-\x{2FF}"),
    ("CombiningDiacriticalMarks", r"\x{300}-\x{36F}"),
    ("Greek", r"\x{370}-\x{3FF}"),
    ("GreekandCoptic", r"\x{370}-\x{3FF}"),
    ("Cyrillic", r"\x{400}-\x{4FF}"),
    ("Armenian", r"\x{530}-\x{58F}"),
    ("Hebrew", r"\x{590}-\x{5FF}"),
    ("Arabic", r"\x{600}-\x{6FF}"),
    ("Devanagari", r"\x{900}-\x{97F}"),
    ("Thai", r"\x{E00}-\x{E7F}"),
    ("LatinExtendedAdditional", r"\x{1E00}-\x{1EFF}"),
    ("GreekExtended", r"\x{1F00}-\x{1FFF}"),
    ("GeneralPunctuation", r"\x{2000}-\x{206F}"),
    ("CurrencySymbols", r"\x{20A0}-\x{20CF}"),
    ("LetterlikeSymbols", r"\x{2100}-\x{214F}"),
    ("NumberForms", r"\x{2150}-\x{218F}"),
    ("Arrows", r"\x{2190}-\x{21FF}"),
    ("MathematicalOperators", r"\x{2200}-\x{22FF}"),
    ("BoxDrawing", r"\x{2500}-\x{257F}"),
    ("GeometricShapes", r"\x{25A0}-\x{25FF}"),
    ("MiscellaneousSymbols", r"\x{2600}-\x{26FF}"),
    ("CJKSymbolsandPunctuation", r"\x{3000}-\x{303F}"),
    ("Hiragana", r"\x{3040}-\x{309F}"),
    ("Katakana", r"\x{30A0}-\x{30FF}"),
    ("CJKUnifiedIdeographs", r"\x{4E00}-\x{9FFF}"),
    ("HangulSyllables", r"\x{AC00}-\x{D7AF}"),
    ("PrivateUse", r"\x{E000}-\x{F8FF}"),
    ("AlphabeticPresentationForms", r"\x{FB00}-\x{FB4F}"),
    ("HalfwidthandFullwidthForms", r"\x{FF00}-\x{FFEF}"),
];

/// A multi-character escape (`\d`, `\s`, `\i`...) or category as a Rust class.
fn class_escape(c: char, chars: &[char], i: &mut usize) -> Result<Option<String>, String> {
    Ok(Some(match c {
        'd' => r"\p{Nd}".into(),
        'D' => r"\P{Nd}".into(),
        's' => r"[ \t\n\r]".into(),
        'S' => r"[^ \t\n\r]".into(),
        'w' => r"[^\p{P}\p{Z}\p{C}]".into(),
        'W' => r"[\p{P}\p{Z}\p{C}]".into(),
        'i' => r"[_:\p{L}]".into(),
        'I' => r"[^_:\p{L}]".into(),
        'c' => r"[-._:\p{L}\p{Nd}\p{Mn}\p{Mc}\x{B7}]".into(),
        'C' => r"[^-._:\p{L}\p{Nd}\p{Mn}\p{Mc}\x{B7}]".into(),
        'p' | 'P' => {
            if chars.get(*i) != Some(&'{') {
                return Err(format!("\\{c} must be followed by {{name}}."));
            }
            let close = chars[*i..]
                .iter()
                .position(|&x| x == '}')
                .ok_or("A \\p{ is never closed.")?;
            let name: String = chars[*i + 1..*i + close].iter().collect();
            *i += close + 1;
            if let Some(block) = name.strip_prefix("Is") {
                let range = BLOCKS
                    .iter()
                    .find(|(n, _)| *n == block)
                    .map(|(_, r)| *r)
                    .ok_or_else(|| format!("The Unicode block {block} isn't supported."))?;
                if c == 'p' {
                    format!("[{range}]")
                } else {
                    format!("[^{range}]")
                }
            } else {
                format!("\\{c}{{{name}}}")
            }
        }
        _ => return Ok(None),
    }))
}

fn single_escape(c: char) -> Option<String> {
    Some(match c {
        'n' => r"\n".into(),
        'r' => r"\r".into(),
        't' => r"\t".into(),
        '\\' | '|' | '.' | '?' | '*' | '+' | '(' | ')' | '{' | '}' | '-' | '[' | ']' | '^'
        | '$' => format!("\\{c}"),
        _ => return None,
    })
}

/// Translates a character class (after its `[`) into Rust regex syntax.
fn translate_class(chars: &[char], i: &mut usize) -> Result<String, String> {
    let mut negated = false;
    if chars.get(*i) == Some(&'^') {
        negated = true;
        *i += 1;
    }
    let mut items = String::new();
    let mut subtract = None;
    let mut first = true;
    loop {
        let Some(&c) = chars.get(*i) else {
            return Err("A '[' is never closed.".into());
        };
        *i += 1;
        match c {
            ']' if !first => break,
            '-' if chars.get(*i) == Some(&'[') => {
                *i += 1;
                subtract = Some(translate_class(chars, i)?);
                if chars.get(*i) != Some(&']') {
                    return Err("A class subtraction must end the class.".into());
                }
                *i += 1;
                break;
            }
            '\\' => {
                let Some(&e) = chars.get(*i) else {
                    return Err("The pattern ends with a '\\'.".into());
                };
                *i += 1;
                if let Some(s) = class_escape(e, chars, i)? {
                    items.push_str(&s);
                } else if let Some(s) = single_escape(e) {
                    items.push_str(&s);
                } else {
                    return Err(format!("'\\{e}' isn't a valid escape."));
                }
            }
            '[' => return Err("A '[' inside a class must be escaped.".into()),
            '&' | '~' | '^' | ' ' => {
                items.push('\\');
                items.push(c);
            }
            c => items.push(c),
        }
        first = false;
    }
    let base = format!("[{}{items}]", if negated { "^" } else { "" });
    Ok(match subtract {
        Some(sub) => format!("[{base}--{sub}]"),
        None => base,
    })
}

/// Translates an XSD pattern into an anchored Rust regex.
fn xsd_regex(src: &str) -> Result<String, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::from("^(?:");
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        match c {
            '\\' => {
                let Some(&e) = chars.get(i) else {
                    return Err("The pattern ends with a '\\'.".into());
                };
                i += 1;
                if let Some(s) = class_escape(e, &chars, &mut i)? {
                    out.push_str(&s);
                } else if let Some(s) = single_escape(e) {
                    out.push_str(&s);
                } else {
                    return Err(format!("'\\{e}' isn't a valid escape."));
                }
            }
            '[' => out.push_str(&translate_class(&chars, &mut i)?),
            '.' => out.push_str(r"[^\n\r]"),
            '^' | '$' | '#' | '&' | '~' => {
                out.push('\\');
                out.push(c);
            }
            // XSD has no group modifiers; keep groups non-capturing.
            '(' => out.push_str("(?:"),
            c => out.push(c),
        }
    }
    out.push_str(")$");
    Ok(out)
}

// ----------------------------------------------------------------------
// The schema model

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct QName {
    ns: String,
    local: String,
}

impl QName {
    fn new(ns: &str, local: &str) -> Self {
        Self {
            ns: ns.to_owned(),
            local: local.to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
enum TypeRef {
    Builtin(B),
    AnyType,
    Simple(usize),
    Complex(usize),
    /// A named type, resolved when used.
    Named(QName),
}

#[derive(Clone, Debug)]
struct ElemDecl {
    name: QName,
    ty: Option<TypeRef>,
    nillable: bool,
    abstract_: bool,
    default: Option<String>,
    fixed: Option<String>,
    subst: Option<QName>,
}

#[derive(Clone, Debug, Default)]
struct Facets {
    enums: Vec<String>,
    patterns: Vec<(String, Regex)>,
    length: Option<usize>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    min_inclusive: Option<String>,
    max_inclusive: Option<String>,
    min_exclusive: Option<String>,
    max_exclusive: Option<String>,
    total_digits: Option<usize>,
    fraction_digits: Option<usize>,
    whitespace: Option<Ws>,
}

#[derive(Clone, Debug)]
enum SimpleDef {
    Restrict { base: TypeRef, facets: Box<Facets> },
    List(TypeRef),
    Union(Vec<TypeRef>),
}

#[derive(Clone, Debug)]
struct Simple {
    name: Option<QName>,
    def: SimpleDef,
}

#[derive(Clone, Debug)]
enum Content {
    Empty,
    Particle(usize),
    Simple(TypeRef),
}

#[derive(Clone, Debug)]
enum Deriv {
    None,
    ExtComplex(TypeRef),
    RestrComplex(TypeRef),
    ExtSimple(TypeRef),
    RestrSimple(TypeRef, Box<Facets>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Use {
    Optional,
    Required,
    Prohibited,
}

#[derive(Clone, Debug)]
struct AttrUse {
    name: QName,
    ty: TypeRef,
    use_: Use,
    default: Option<String>,
    fixed: Option<String>,
}

#[derive(Clone, Debug)]
enum AttrItem {
    Use(AttrUse),
    Ref {
        name: QName,
        use_: Use,
        default: Option<String>,
        fixed: Option<String>,
    },
    Group(QName),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Process {
    Strict,
    Lax,
    Skip,
}

#[derive(Clone, Debug)]
enum NsSet {
    Any,
    /// `##other`: any namespace but this one (and not unqualified).
    Not(String),
    List(Vec<String>),
}

#[derive(Clone, Debug)]
struct Wildcard {
    ns: NsSet,
    process: Process,
}

impl Wildcard {
    fn allows(&self, ns: &str) -> bool {
        match &self.ns {
            NsSet::Any => true,
            NsSet::Not(t) => ns != t && !ns.is_empty(),
            NsSet::List(l) => l.iter().any(|x| x == ns),
        }
    }
}

#[derive(Clone, Debug)]
struct Complex {
    name: Option<QName>,
    mixed: bool,
    abstract_: bool,
    deriv: Deriv,
    content: Content,
    attrs: Vec<AttrItem>,
    any_attr: Option<Wildcard>,
}

/// A complex type with its derivation applied.
#[derive(Clone, Debug)]
struct Effective {
    content: Content,
    mixed: bool,
    attrs: Vec<AttrUse>,
    any_attr: Option<Wildcard>,
}

#[derive(Clone, Debug)]
enum ElemRef {
    Local(usize),
    Global(QName),
}

#[derive(Clone, Debug)]
enum Term {
    Elem(ElemRef),
    Seq(Vec<usize>),
    Choice(Vec<usize>),
    All(Vec<usize>),
    Any(Wildcard),
    Group(QName),
}

#[derive(Clone, Debug)]
struct Particle {
    min: u32,
    /// `None` is unbounded.
    max: Option<u32>,
    term: Term,
}

#[derive(Clone, Debug)]
struct AttrGroup {
    items: Vec<AttrItem>,
    any_attr: Option<Wildcard>,
}

#[derive(Debug, Default)]
struct Schema {
    elements: Vec<ElemDecl>,
    global_elements: HashMap<QName, usize>,
    simple: Vec<Simple>,
    complex: Vec<Complex>,
    effective: Vec<Effective>,
    types: HashMap<QName, TypeRef>,
    groups: HashMap<QName, usize>,
    attr_groups: HashMap<QName, AttrGroup>,
    global_attrs: HashMap<QName, AttrUse>,
    particles: Vec<Particle>,
    /// Namespaces imported from places we can't read; validated laxly.
    lenient: HashSet<String>,
}

impl Schema {
    fn resolve(&self, t: &TypeRef) -> TypeRef {
        match t {
            TypeRef::Named(q) => self.types.get(q).cloned().unwrap_or(TypeRef::AnyType),
            t => t.clone(),
        }
    }

    fn global_attr(&self, q: &QName) -> Option<AttrUse> {
        if let Some(a) = self.global_attrs.get(q) {
            return Some(a.clone());
        }
        if q.ns != XML_NS {
            return None;
        }
        let ty = match q.local.as_str() {
            "lang" | "space" => B::Token,
            "base" => B::AnyUri,
            "id" => B::Id,
            _ => return None,
        };
        Some(AttrUse {
            name: q.clone(),
            ty: TypeRef::Builtin(ty),
            use_: Use::Optional,
            default: None,
            fixed: None,
        })
    }

    /// An element's declared type (inherited from its substitution group head if unset).
    fn elem_type(&self, d: usize) -> TypeRef {
        let mut cur = d;
        for _ in 0..32 {
            let e = &self.elements[cur];
            if let Some(t) = &e.ty {
                return self.resolve(t);
            }
            match e.subst.as_ref().and_then(|s| self.global_elements.get(s)) {
                Some(&h) => cur = h,
                None => break,
            }
        }
        TypeRef::AnyType
    }

    fn substitutes_for(&self, e: usize, head: &QName) -> bool {
        let mut cur = self.elements[e].subst.clone();
        for _ in 0..32 {
            match cur {
                Some(q) if &q == head => return true,
                Some(q) => {
                    cur = self
                        .global_elements
                        .get(&q)
                        .and_then(|&g| self.elements[g].subst.clone())
                }
                None => return false,
            }
        }
        false
    }

    fn whitespace(&self, t: &TypeRef) -> Ws {
        match self.resolve(t) {
            TypeRef::Builtin(b) => b.whitespace(),
            TypeRef::Simple(i) => match &self.simple[i].def {
                SimpleDef::Restrict { base, facets } => {
                    facets.whitespace.unwrap_or_else(|| self.whitespace(base))
                }
                _ => Ws::Collapse,
            },
            TypeRef::Complex(i) => match &self.effective[i].content {
                Content::Simple(t) => self.whitespace(t),
                _ => Ws::Preserve,
            },
            _ => Ws::Preserve,
        }
    }

    fn prim(&self, t: &TypeRef) -> Prim {
        match self.resolve(t) {
            TypeRef::Builtin(b) => b.prim(),
            TypeRef::Simple(i) => match &self.simple[i].def {
                SimpleDef::Restrict { base, .. } => self.prim(base),
                SimpleDef::List(_) => Prim::List,
                SimpleDef::Union(_) => Prim::Union,
            },
            TypeRef::Complex(i) => match &self.effective[i].content {
                Content::Simple(t) => self.prim(t),
                _ => Prim::Str,
            },
            _ => Prim::Str,
        }
    }

    fn type_label(&self, t: &TypeRef) -> String {
        match self.resolve(t) {
            TypeRef::Builtin(b) => b.label().to_owned(),
            TypeRef::Simple(i) => match &self.simple[i].name {
                Some(n) => n.local.clone(),
                None => match &self.simple[i].def {
                    SimpleDef::Restrict { base, .. } => self.type_label(base),
                    SimpleDef::List(_) => "list".into(),
                    SimpleDef::Union(_) => "union".into(),
                },
            },
            _ => "value".into(),
        }
    }

    /// Checks a value against a simple type; returns what kind of ID it is.
    fn check_value(&self, t: &TypeRef, raw: &str) -> Result<IdKind, String> {
        match self.resolve(t) {
            TypeRef::Builtin(b) => {
                let v = normalize(raw, b.whitespace());
                if b.check(&v) {
                    Ok(b.id_kind())
                } else {
                    Err(format!("{} isn't a valid {}.", show(&v), b.label()))
                }
            }
            TypeRef::Simple(i) => match &self.simple[i].def {
                SimpleDef::Restrict { base, facets } => {
                    let kind = self.check_value(base, raw)?;
                    let v = normalize(raw, self.whitespace(t));
                    self.check_facets(facets, &v, self.prim(base))?;
                    Ok(kind)
                }
                SimpleDef::List(item) => {
                    let mut kind = IdKind::Other;
                    for token in raw.split(is_xml_space).filter(|s| !s.is_empty()) {
                        if self.check_value(item, token)? == IdKind::IdRef {
                            kind = IdKind::IdRefs;
                        }
                    }
                    Ok(kind)
                }
                SimpleDef::Union(members) => {
                    for m in members {
                        if let Ok(k) = self.check_value(m, raw) {
                            return Ok(k);
                        }
                    }
                    let names: Vec<String> = members.iter().map(|m| self.type_label(m)).collect();
                    Err(format!(
                        "{} isn't valid for any of the allowed types ({}).",
                        show(&normalize(raw, Ws::Collapse)),
                        names.join(", ")
                    ))
                }
            },
            TypeRef::Complex(i) => match &self.effective[i].content {
                Content::Simple(st) => self.check_value(st, raw),
                _ => Ok(IdKind::Other),
            },
            _ => Ok(IdKind::Other),
        }
    }

    fn compare(&self, prim: Prim, a: &str, b: &str) -> Option<Ordering> {
        match prim {
            Prim::Decimal => Some(cmp_decimal(&parse_decimal(a)?, &parse_decimal(b)?)),
            Prim::Float => parse_float(a)?.partial_cmp(&parse_float(b)?),
            Prim::Date(k) => parse_date(k, a)?.partial_cmp(&parse_date(k, b)?),
            Prim::Duration => parse_duration(a)?.partial_cmp(&parse_duration(b)?),
            _ => None,
        }
    }

    fn equal(&self, prim: Prim, a: &str, b: &str) -> bool {
        match prim {
            Prim::Decimal | Prim::Float | Prim::Date(_) | Prim::Duration => {
                a == b || self.compare(prim, a, b) == Some(Ordering::Equal)
            }
            Prim::Bool => {
                let t = |s: &str| s == "true" || s == "1";
                t(a) == t(b)
            }
            _ => a == b,
        }
    }

    fn check_facets(&self, f: &Facets, v: &str, prim: Prim) -> Result<(), String> {
        if !f.enums.is_empty() && !f.enums.iter().any(|e| self.equal(prim, v, e)) {
            let mut list: Vec<String> = f.enums.iter().take(10).map(|e| show(e)).collect();
            if f.enums.len() > 10 {
                list.push("…".into());
            }
            return Err(format!(
                "{} isn't one of the allowed values: {}.",
                show(v),
                list.join(", ")
            ));
        }
        if !f.patterns.is_empty() && !f.patterns.iter().any(|(_, re)| re.is_match(v)) {
            let shown: Vec<String> = f.patterns.iter().map(|(p, _)| show(p)).collect();
            return Err(format!(
                "{} doesn't match the pattern {}.",
                show(v),
                shown.join(" or ")
            ));
        }
        let (len, unit) = match prim {
            Prim::Hex => (v.len() / 2, "bytes"),
            Prim::Base64 => (base64_octets(v).unwrap_or(0), "bytes"),
            Prim::List => (v.split(' ').filter(|s| !s.is_empty()).count(), "items"),
            _ => (v.chars().count(), "characters"),
        };
        if prim != Prim::QName {
            if let Some(n) = f.length
                && len != n
            {
                return Err(format!(
                    "{} should be exactly {n} {unit} long (it has {len}).",
                    show(v)
                ));
            }
            if let Some(n) = f.min_length
                && len < n
            {
                return Err(format!(
                    "{} is shorter than the minimum of {n} {unit}.",
                    show(v)
                ));
            }
            if let Some(n) = f.max_length
                && len > n
            {
                return Err(format!(
                    "{} is longer than the maximum of {n} {unit}.",
                    show(v)
                ));
            }
        }
        let cmp = |bound: &str| self.compare(prim, v, bound);
        if let Some(b) = &f.min_inclusive
            && cmp(b) == Some(Ordering::Less)
        {
            return Err(format!("{} is less than the minimum of {b}.", show(v)));
        }
        if let Some(b) = &f.max_inclusive
            && cmp(b) == Some(Ordering::Greater)
        {
            return Err(format!("{} is more than the maximum of {b}.", show(v)));
        }
        if let Some(b) = &f.min_exclusive
            && matches!(cmp(b), Some(Ordering::Less | Ordering::Equal))
        {
            return Err(format!("{} must be greater than {b}.", show(v)));
        }
        if let Some(b) = &f.max_exclusive
            && matches!(cmp(b), Some(Ordering::Greater | Ordering::Equal))
        {
            return Err(format!("{} must be less than {b}.", show(v)));
        }
        if prim == Prim::Decimal
            && let Some(d) = parse_decimal(v)
        {
            if let Some(n) = f.total_digits
                && d.int.len() + d.frac.len() > n
            {
                return Err(format!("{} has more than {n} digits.", show(v)));
            }
            if let Some(n) = f.fraction_digits
                && d.frac.len() > n
            {
                return Err(format!(
                    "{} has more than {n} digits after the decimal point.",
                    show(v)
                ));
            }
        }
        Ok(())
    }
}

// ----------------------------------------------------------------------
// Loading schema documents

fn is_xs(n: Node, local: &str) -> bool {
    n.is_element() && n.tag_name().namespace() == Some(XS) && n.tag_name().name() == local
}

/// The element children in the XSD namespace, without annotations.
fn xs_children<'a, 'i>(n: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(|c| {
        c.is_element()
            && c.tag_name().namespace() == Some(XS)
            && c.tag_name().name() != "annotation"
    })
}

fn is_remote(loc: &str) -> bool {
    let l = loc.to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://") || l.starts_with("ftp://")
}

/// Per-document settings while loading one schema file.
struct DocCx<'t> {
    text: &'t str,
    /// The file's name, for files other than the main schema.
    file: Option<String>,
    dir: Option<PathBuf>,
    tns: String,
    chameleon: bool,
    elem_qualified: bool,
    attr_qualified: bool,
}

impl DocCx<'_> {
    fn err(&self, n: Node, message: impl Into<String>) -> ToolError {
        let message = message.into();
        let offset = n.range().start;
        let (line, col) = line_col(self.text, offset);
        match &self.file {
            None => ToolError::at(
                format!("{message} (schema line {line}, column {col})"),
                offset,
            ),
            Some(f) => ToolError::new(format!("{message} (in {f}, line {line}, column {col})")),
        }
    }
}

type Refs = Vec<(QName, ToolError)>;

struct Loader {
    s: Schema,
    loaded: HashSet<PathBuf>,
    type_refs: Refs,
    elem_refs: Refs,
    group_refs: Refs,
    attr_group_refs: Refs,
    attr_refs: Refs,
}

impl Loader {
    fn load(text: &str, dir: Option<&Path>) -> Result<Schema, ToolError> {
        let mut l = Loader {
            s: Schema::default(),
            loaded: HashSet::new(),
            type_refs: Vec::new(),
            elem_refs: Vec::new(),
            group_refs: Vec::new(),
            attr_group_refs: Vec::new(),
            attr_refs: Vec::new(),
        };
        l.load_doc(text, None, dir.map(Path::to_path_buf), None)?;
        l.finish()
    }

    fn load_doc(
        &mut self,
        text: &str,
        file: Option<String>,
        dir: Option<PathBuf>,
        chameleon_tns: Option<String>,
    ) -> Result<(), ToolError> {
        let doc = xml::parse_strict(text).map_err(|e| match &file {
            None => ToolError {
                message: format!("The schema isn't well-formed XML: {}", e.message),
                offset: e.offset,
            },
            Some(f) => ToolError::new(format!("{f} isn't well-formed XML: {}", e.message)),
        })?;
        let root = doc.root_element();
        let declared = root.attribute("targetNamespace").unwrap_or("").to_owned();
        let (tns, chameleon) = match chameleon_tns {
            Some(t) if declared.is_empty() && !t.is_empty() => (t, true),
            _ => (declared, false),
        };
        let cx = DocCx {
            text,
            file,
            dir,
            tns,
            chameleon,
            elem_qualified: root.attribute("elementFormDefault") == Some("qualified"),
            attr_qualified: root.attribute("attributeFormDefault") == Some("qualified"),
        };
        if !is_xs(root, "schema") {
            return Err(cx.err(
                root,
                "This isn't an XML Schema: the root element should be xs:schema.",
            ));
        }
        for child in xs_children(root) {
            let local = child.tag_name().name();
            match local {
                "include" => {
                    let loc = self.required(&cx, child, "schemaLocation")?;
                    if is_remote(loc) {
                        return Err(cx.err(
                            child,
                            format!("The included schema \"{loc}\" is online; download it next to this schema and include it by file name."),
                        ));
                    }
                    let tns = cx.tns.clone();
                    self.load_file(&cx, child, loc, Some(tns))?;
                }
                "import" => {
                    let ns = child.attribute("namespace").unwrap_or("");
                    match child.attribute("schemaLocation") {
                        Some(loc) if !is_remote(loc) => self.load_file(&cx, child, loc, None)?,
                        _ => {
                            if ns != XML_NS {
                                self.s.lenient.insert(ns.to_owned());
                            }
                        }
                    }
                }
                "redefine" | "override" => {
                    return Err(cx.err(child, format!("xs:{local} isn't supported.")));
                }
                "element" => {
                    let id = self.element_decl(&cx, child, true)?;
                    let name = self.s.elements[id].name.clone();
                    self.s.global_elements.insert(name, id);
                }
                "complexType" => {
                    let name = QName::new(&cx.tns, self.required(&cx, child, "name")?);
                    let id = self.complex_type(&cx, child, Some(name.clone()))?;
                    self.s.types.insert(name, TypeRef::Complex(id));
                }
                "simpleType" => {
                    let name = QName::new(&cx.tns, self.required(&cx, child, "name")?);
                    let id = self.simple_type(&cx, child, Some(name.clone()))?;
                    self.s.types.insert(name, TypeRef::Simple(id));
                }
                "group" => {
                    let name = QName::new(&cx.tns, self.required(&cx, child, "name")?);
                    let Some(inner) = xs_children(child).next() else {
                        return Err(cx.err(child, "xs:group needs a sequence, choice or all."));
                    };
                    let p = match self.particle(&cx, inner)? {
                        Some(p) => p,
                        None => self.push_particle(0, Some(0), Term::Seq(Vec::new())),
                    };
                    self.s.groups.insert(name, p);
                }
                "attributeGroup" => {
                    let name = QName::new(&cx.tns, self.required(&cx, child, "name")?);
                    let mut items = Vec::new();
                    let mut any_attr = None;
                    for c in xs_children(child) {
                        self.attr_child(&cx, c, &mut items, &mut any_attr)?;
                    }
                    self.s
                        .attr_groups
                        .insert(name, AttrGroup { items, any_attr });
                }
                "attribute" => {
                    if let AttrItem::Use(u) = self.attribute(&cx, child, true)? {
                        self.s.global_attrs.insert(u.name.clone(), u);
                    }
                }
                "notation" => {}
                other => {
                    return Err(cx.err(
                        child,
                        format!("xs:{other} isn't allowed at the top level of a schema."),
                    ));
                }
            }
        }
        Ok(())
    }

    fn load_file(
        &mut self,
        cx: &DocCx,
        n: Node,
        loc: &str,
        chameleon_tns: Option<String>,
    ) -> Result<(), ToolError> {
        let Some(dir) = &cx.dir else {
            return Err(cx.err(
                n,
                format!("The schema refers to \"{loc}\", but its folder is unknown; save the schema to a file first."),
            ));
        };
        let rel = loc.strip_prefix("file://").unwrap_or(loc);
        let path = dir.join(rel);
        let key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !self.loaded.insert(key) {
            return Ok(());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| cx.err(n, format!("Couldn't read the schema \"{loc}\": {e}.")))?;
        let name = path
            .file_name()
            .map_or_else(|| loc.to_owned(), |f| f.to_string_lossy().into_owned());
        self.load_doc(
            &text,
            Some(name),
            path.parent().map(Path::to_path_buf),
            chameleon_tns,
        )
    }

    fn required<'a>(&self, cx: &DocCx, n: Node<'a, '_>, attr: &str) -> Result<&'a str, ToolError> {
        n.attribute(attr).ok_or_else(|| {
            cx.err(
                n,
                format!("xs:{} needs a {attr} attribute.", n.tag_name().name()),
            )
        })
    }

    fn qname(&self, cx: &DocCx, n: Node, value: &str) -> Result<QName, ToolError> {
        let value = value.trim();
        let (prefix, local) = match value.split_once(':') {
            Some((p, l)) => (Some(p), l),
            None => (None, value),
        };
        let ns = match prefix {
            Some("xml") => XML_NS.to_owned(),
            Some(p) => n
                .lookup_namespace_uri(Some(p))
                .ok_or_else(|| {
                    cx.err(
                        n,
                        format!("The namespace prefix '{p}' in \"{value}\" isn't declared."),
                    )
                })?
                .to_owned(),
            None => n.lookup_namespace_uri(None).unwrap_or("").to_owned(),
        };
        let ns = if cx.chameleon && ns.is_empty() {
            cx.tns.clone()
        } else {
            ns
        };
        Ok(QName {
            ns,
            local: local.to_owned(),
        })
    }

    fn type_ref(&mut self, cx: &DocCx, n: Node, value: &str) -> Result<TypeRef, ToolError> {
        let q = self.qname(cx, n, value)?;
        if q.ns == XS {
            if q.local == "anyType" {
                return Ok(TypeRef::AnyType);
            }
            return builtin(&q.local).map(TypeRef::Builtin).ok_or_else(|| {
                cx.err(n, format!("\"{value}\" isn't a built-in XML Schema type."))
            });
        }
        self.type_refs.push((
            q.clone(),
            cx.err(
                n,
                format!("The type \"{value}\" isn't defined in the schema."),
            ),
        ));
        Ok(TypeRef::Named(q))
    }

    fn occurs(&self, cx: &DocCx, n: Node) -> Result<(u32, Option<u32>), ToolError> {
        let num = |attr: &str, v: &str| -> Result<u32, ToolError> {
            v.trim()
                .parse()
                .map_err(|_| cx.err(n, format!("{attr}=\"{v}\" should be a whole number.")))
        };
        let min = match n.attribute("minOccurs") {
            Some(v) => num("minOccurs", v)?,
            None => 1,
        };
        let max = match n.attribute("maxOccurs") {
            Some(v) if v.trim() == "unbounded" => None,
            Some(v) => Some(num("maxOccurs", v)?),
            None => Some(1),
        };
        if max.is_some_and(|m| m < min) {
            return Err(cx.err(n, "maxOccurs is less than minOccurs."));
        }
        Ok((min, max))
    }

    fn push_particle(&mut self, min: u32, max: Option<u32>, term: Term) -> usize {
        self.s.particles.push(Particle { min, max, term });
        self.s.particles.len() - 1
    }

    fn element_decl(&mut self, cx: &DocCx, n: Node, global: bool) -> Result<usize, ToolError> {
        let name = self.required(cx, n, "name")?;
        let qualified = global
            || n.attribute("form")
                .map_or(cx.elem_qualified, |f| f == "qualified");
        let ns = if qualified { cx.tns.as_str() } else { "" };
        let ty = if let Some(t) = n.attribute("type") {
            Some(self.type_ref(cx, n, t)?)
        } else if let Some(c) = xs_children(n).find(|c| is_xs(*c, "complexType")) {
            Some(TypeRef::Complex(self.complex_type(cx, c, None)?))
        } else if let Some(c) = xs_children(n).find(|c| is_xs(*c, "simpleType")) {
            Some(TypeRef::Simple(self.simple_type(cx, c, None)?))
        } else {
            None
        };
        let subst = match n.attribute("substitutionGroup") {
            Some(s) => {
                let q = self.qname(cx, n, s)?;
                self.elem_refs.push((
                    q.clone(),
                    cx.err(
                        n,
                        format!("The element \"{s}\" isn't declared in the schema."),
                    ),
                ));
                Some(q)
            }
            None => None,
        };
        let yes = |a: &str| matches!(n.attribute(a).map(str::trim), Some("true" | "1"));
        self.s.elements.push(ElemDecl {
            name: QName::new(ns, name),
            ty,
            nillable: yes("nillable"),
            abstract_: yes("abstract"),
            default: n.attribute("default").map(str::to_owned),
            fixed: n.attribute("fixed").map(str::to_owned),
            subst,
        });
        Ok(self.s.elements.len() - 1)
    }

    fn wildcard(&self, cx: &DocCx, n: Node) -> Wildcard {
        let ns = match n.attribute("namespace").map(str::trim).unwrap_or("##any") {
            "##any" => NsSet::Any,
            "##other" => NsSet::Not(cx.tns.clone()),
            list => NsSet::List(
                list.split_whitespace()
                    .map(|t| match t {
                        "##targetNamespace" => cx.tns.clone(),
                        "##local" => String::new(),
                        t => t.to_owned(),
                    })
                    .collect(),
            ),
        };
        let process = match n.attribute("processContents") {
            Some("lax") => Process::Lax,
            Some("skip") => Process::Skip,
            _ => Process::Strict,
        };
        Wildcard { ns, process }
    }

    fn particle(&mut self, cx: &DocCx, n: Node) -> Result<Option<usize>, ToolError> {
        let (min, max) = self.occurs(cx, n)?;
        if max == Some(0) {
            return Ok(None);
        }
        let local = n.tag_name().name();
        let term = match local {
            "element" => match n.attribute("ref") {
                Some(r) => {
                    let q = self.qname(cx, n, r)?;
                    self.elem_refs.push((
                        q.clone(),
                        cx.err(
                            n,
                            format!("The element \"{r}\" isn't declared in the schema."),
                        ),
                    ));
                    Term::Elem(ElemRef::Global(q))
                }
                None => Term::Elem(ElemRef::Local(self.element_decl(cx, n, false)?)),
            },
            "sequence" | "choice" | "all" => {
                let mut kids = Vec::new();
                for c in xs_children(n) {
                    if let Some(p) = self.particle(cx, c)? {
                        kids.push(p);
                    }
                }
                match local {
                    "sequence" => Term::Seq(kids),
                    "choice" => Term::Choice(kids),
                    _ => Term::All(kids),
                }
            }
            "any" => Term::Any(self.wildcard(cx, n)),
            "group" => {
                let r = self.required(cx, n, "ref")?;
                let q = self.qname(cx, n, r)?;
                self.group_refs.push((
                    q.clone(),
                    cx.err(n, format!("The group \"{r}\" isn't defined in the schema.")),
                ));
                Term::Group(q)
            }
            other => {
                return Err(cx.err(n, format!("xs:{other} isn't allowed in a content model.")));
            }
        };
        Ok(Some(self.push_particle(min, max, term)))
    }

    fn attribute(&mut self, cx: &DocCx, n: Node, global: bool) -> Result<AttrItem, ToolError> {
        let use_ = match n.attribute("use").map(str::trim) {
            Some("required") => Use::Required,
            Some("prohibited") => Use::Prohibited,
            _ => Use::Optional,
        };
        let default = n.attribute("default").map(str::to_owned);
        let fixed = n.attribute("fixed").map(str::to_owned);
        if let Some(r) = n.attribute("ref") {
            let q = self.qname(cx, n, r)?;
            self.attr_refs.push((
                q.clone(),
                cx.err(
                    n,
                    format!("The attribute \"{r}\" isn't declared in the schema."),
                ),
            ));
            return Ok(AttrItem::Ref {
                name: q,
                use_,
                default,
                fixed,
            });
        }
        let name = self.required(cx, n, "name")?;
        let qualified = global
            || n.attribute("form")
                .map_or(cx.attr_qualified, |f| f == "qualified");
        let ty = if let Some(t) = n.attribute("type") {
            self.type_ref(cx, n, t)?
        } else if let Some(c) = xs_children(n).find(|c| is_xs(*c, "simpleType")) {
            TypeRef::Simple(self.simple_type(cx, c, None)?)
        } else {
            TypeRef::Builtin(B::AnySimple)
        };
        Ok(AttrItem::Use(AttrUse {
            name: QName::new(if qualified { &cx.tns } else { "" }, name),
            ty,
            use_,
            default,
            fixed,
        }))
    }

    /// Handles an attribute-related child; returns false if it isn't one.
    fn attr_child(
        &mut self,
        cx: &DocCx,
        c: Node,
        items: &mut Vec<AttrItem>,
        any_attr: &mut Option<Wildcard>,
    ) -> Result<bool, ToolError> {
        match c.tag_name().name() {
            "attribute" => items.push(self.attribute(cx, c, false)?),
            "attributeGroup" => {
                let r = self.required(cx, c, "ref")?;
                let q = self.qname(cx, c, r)?;
                self.attr_group_refs.push((
                    q.clone(),
                    cx.err(
                        c,
                        format!("The attribute group \"{r}\" isn't defined in the schema."),
                    ),
                ));
                items.push(AttrItem::Group(q));
            }
            "anyAttribute" => *any_attr = Some(self.wildcard(cx, c)),
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn complex_type(
        &mut self,
        cx: &DocCx,
        n: Node,
        name: Option<QName>,
    ) -> Result<usize, ToolError> {
        let yes = |n: Node, a: &str| matches!(n.attribute(a).map(str::trim), Some("true" | "1"));
        let mut c = Complex {
            name,
            mixed: yes(n, "mixed"),
            abstract_: yes(n, "abstract"),
            deriv: Deriv::None,
            content: Content::Empty,
            attrs: Vec::new(),
            any_attr: None,
        };
        for child in xs_children(n) {
            match child.tag_name().name() {
                "simpleContent" | "complexContent" => {
                    let simple = child.tag_name().name() == "simpleContent";
                    if yes(child, "mixed") {
                        c.mixed = true;
                    }
                    let Some(d) = xs_children(child).next() else {
                        return Err(cx.err(child, "Expected xs:extension or xs:restriction here."));
                    };
                    let base = self.required(cx, d, "base")?;
                    let base = self.type_ref(cx, d, base)?;
                    let extension = match d.tag_name().name() {
                        "extension" => true,
                        "restriction" => false,
                        other => {
                            return Err(cx.err(
                                d,
                                format!("xs:{other} isn't allowed here; use xs:extension or xs:restriction."),
                            ));
                        }
                    };
                    for g in xs_children(d) {
                        if self.attr_child(cx, g, &mut c.attrs, &mut c.any_attr)? {
                            continue;
                        }
                        if !simple
                            && matches!(
                                g.tag_name().name(),
                                "sequence" | "choice" | "all" | "group"
                            )
                        {
                            c.content = match self.particle(cx, g)? {
                                Some(p) => Content::Particle(p),
                                None => Content::Empty,
                            };
                        }
                    }
                    c.deriv = match (simple, extension) {
                        (true, true) => Deriv::ExtSimple(base),
                        (true, false) => Deriv::RestrSimple(base, Box::new(self.facets(cx, d)?)),
                        (false, true) => Deriv::ExtComplex(base),
                        (false, false) => Deriv::RestrComplex(base),
                    };
                }
                "sequence" | "choice" | "all" | "group" => {
                    c.content = match self.particle(cx, child)? {
                        Some(p) => Content::Particle(p),
                        None => Content::Empty,
                    };
                }
                _ => {
                    if !self.attr_child(cx, child, &mut c.attrs, &mut c.any_attr)? {
                        return Err(cx.err(
                            child,
                            format!(
                                "xs:{} isn't allowed in xs:complexType.",
                                child.tag_name().name()
                            ),
                        ));
                    }
                }
            }
        }
        self.s.complex.push(c);
        Ok(self.s.complex.len() - 1)
    }

    fn simple_type(
        &mut self,
        cx: &DocCx,
        n: Node,
        name: Option<QName>,
    ) -> Result<usize, ToolError> {
        let Some(d) = xs_children(n).next() else {
            return Err(cx.err(n, "xs:simpleType needs a restriction, list or union."));
        };
        let inline = |l: &mut Self| -> Result<Option<TypeRef>, ToolError> {
            match xs_children(d).find(|c| is_xs(*c, "simpleType")) {
                Some(c) => Ok(Some(TypeRef::Simple(l.simple_type(cx, c, None)?))),
                None => Ok(None),
            }
        };
        let def = match d.tag_name().name() {
            "restriction" => {
                let base = match d.attribute("base") {
                    Some(b) => self.type_ref(cx, d, b)?,
                    None => inline(self)?.ok_or_else(|| {
                        cx.err(d, "xs:restriction needs a base attribute or a simpleType.")
                    })?,
                };
                SimpleDef::Restrict {
                    base,
                    facets: Box::new(self.facets(cx, d)?),
                }
            }
            "list" => {
                let item = match d.attribute("itemType") {
                    Some(t) => self.type_ref(cx, d, t)?,
                    None => inline(self)?.ok_or_else(|| {
                        cx.err(d, "xs:list needs an itemType attribute or a simpleType.")
                    })?,
                };
                SimpleDef::List(item)
            }
            "union" => {
                let mut members = Vec::new();
                if let Some(m) = d.attribute("memberTypes") {
                    for t in m.split_whitespace() {
                        members.push(self.type_ref(cx, d, t)?);
                    }
                }
                for c in xs_children(d).filter(|c| is_xs(*c, "simpleType")) {
                    members.push(TypeRef::Simple(self.simple_type(cx, c, None)?));
                }
                if members.is_empty() {
                    return Err(cx.err(d, "xs:union has no member types."));
                }
                SimpleDef::Union(members)
            }
            other => {
                return Err(cx.err(d, format!("xs:{other} isn't allowed in xs:simpleType.")));
            }
        };
        self.s.simple.push(Simple { name, def });
        Ok(self.s.simple.len() - 1)
    }

    fn facets(&mut self, cx: &DocCx, d: Node) -> Result<Facets, ToolError> {
        let mut f = Facets::default();
        for c in xs_children(d) {
            let local = c.tag_name().name();
            let facet = matches!(
                local,
                "enumeration"
                    | "pattern"
                    | "length"
                    | "minLength"
                    | "maxLength"
                    | "minInclusive"
                    | "maxInclusive"
                    | "minExclusive"
                    | "maxExclusive"
                    | "totalDigits"
                    | "fractionDigits"
                    | "whiteSpace"
            );
            if !facet {
                continue;
            }
            let value = self.required(cx, c, "value")?;
            let count = |v: &str| -> Result<usize, ToolError> {
                v.trim()
                    .parse()
                    .map_err(|_| cx.err(c, format!("xs:{local} needs a whole number.")))
            };
            match local {
                "enumeration" => f.enums.push(normalize(value, Ws::Collapse)),
                "pattern" => {
                    let re = xsd_regex(value)
                        .and_then(|r| Regex::new(&r).map_err(|e| e.to_string()))
                        .map_err(|e| {
                            cx.err(
                                c,
                                format!(
                                    "The pattern {} isn't a valid regular expression: {e}",
                                    show(value)
                                ),
                            )
                        })?;
                    f.patterns.push((value.to_owned(), re));
                }
                "length" => f.length = Some(count(value)?),
                "minLength" => f.min_length = Some(count(value)?),
                "maxLength" => f.max_length = Some(count(value)?),
                "totalDigits" => f.total_digits = Some(count(value)?),
                "fractionDigits" => f.fraction_digits = Some(count(value)?),
                "minInclusive" => f.min_inclusive = Some(value.trim().to_owned()),
                "maxInclusive" => f.max_inclusive = Some(value.trim().to_owned()),
                "minExclusive" => f.min_exclusive = Some(value.trim().to_owned()),
                "maxExclusive" => f.max_exclusive = Some(value.trim().to_owned()),
                _ => {
                    f.whitespace = Some(match value.trim() {
                        "preserve" => Ws::Preserve,
                        "replace" => Ws::Replace,
                        "collapse" => Ws::Collapse,
                        v => {
                            return Err(cx.err(
                                c,
                                format!(
                                    "whiteSpace \"{v}\" should be preserve, replace or collapse."
                                ),
                            ));
                        }
                    });
                }
            }
        }
        Ok(f)
    }

    fn finish(mut self) -> Result<Schema, ToolError> {
        let s = &self.s;
        let check = |refs: Refs, exists: &dyn Fn(&QName) -> bool| {
            for (q, e) in refs {
                if !exists(&q) && !s.lenient.contains(&q.ns) {
                    return Err(e);
                }
            }
            Ok(())
        };
        check(std::mem::take(&mut self.type_refs), &|q| {
            s.types.contains_key(q)
        })?;
        check(std::mem::take(&mut self.elem_refs), &|q| {
            s.global_elements.contains_key(q)
        })?;
        check(std::mem::take(&mut self.group_refs), &|q| {
            s.groups.contains_key(q)
        })?;
        check(std::mem::take(&mut self.attr_group_refs), &|q| {
            s.attr_groups.contains_key(q)
        })?;
        check(std::mem::take(&mut self.attr_refs), &|q| {
            s.global_attr(q).is_some()
        })?;
        let n = self.s.complex.len();
        let mut state = vec![0u8; n];
        let mut done: Vec<Option<Effective>> = vec![None; n];
        for i in 0..n {
            self.effective(i, &mut state, &mut done)?;
        }
        self.s.effective = done.into_iter().map(|e| e.expect("computed")).collect();
        Ok(self.s)
    }

    fn flatten_attrs(
        &self,
        items: &[AttrItem],
        out: &mut Vec<AttrUse>,
        any: &mut Option<Wildcard>,
        depth: usize,
    ) {
        for it in items {
            let u = match it {
                AttrItem::Use(u) => u.clone(),
                AttrItem::Ref {
                    name,
                    use_,
                    default,
                    fixed,
                } => {
                    let g = self.s.global_attr(name);
                    AttrUse {
                        name: name.clone(),
                        ty: g
                            .as_ref()
                            .map_or(TypeRef::Builtin(B::AnySimple), |g| g.ty.clone()),
                        use_: *use_,
                        default: default
                            .clone()
                            .or_else(|| g.as_ref().and_then(|g| g.default.clone())),
                        fixed: fixed
                            .clone()
                            .or_else(|| g.as_ref().and_then(|g| g.fixed.clone())),
                    }
                }
                AttrItem::Group(q) => {
                    if depth < 32
                        && let Some(g) = self.s.attr_groups.get(q)
                    {
                        self.flatten_attrs(&g.items, out, any, depth + 1);
                        if any.is_none() {
                            any.clone_from(&g.any_attr);
                        }
                    }
                    continue;
                }
            };
            merge_attr(out, u);
        }
    }

    fn effective(
        &mut self,
        i: usize,
        state: &mut [u8],
        done: &mut [Option<Effective>],
    ) -> Result<Effective, ToolError> {
        if let Some(e) = &done[i] {
            return Ok(e.clone());
        }
        if state[i] == 1 {
            let name = self.s.complex[i]
                .name
                .as_ref()
                .map_or("(anonymous)".to_owned(), |q| q.local.clone());
            return Err(ToolError::new(format!(
                "The schema type \"{name}\" derives from itself."
            )));
        }
        state[i] = 1;
        let c = self.s.complex[i].clone();
        let mut own = Vec::new();
        let mut own_any = c.any_attr.clone();
        self.flatten_attrs(&c.attrs, &mut own, &mut own_any, 0);
        let base = match &c.deriv {
            Deriv::None => None,
            Deriv::ExtComplex(b)
            | Deriv::RestrComplex(b)
            | Deriv::ExtSimple(b)
            | Deriv::RestrSimple(b, _) => Some(b.clone()),
        };
        let b = match base.as_ref().map(|b| self.s.resolve(b)) {
            Some(TypeRef::Complex(j)) => Some(self.effective(j, state, done)?),
            _ => None,
        };
        let mut attrs = b.as_ref().map(|b| b.attrs.clone()).unwrap_or_default();
        for u in own {
            merge_attr(&mut attrs, u);
        }
        let eff = match &c.deriv {
            Deriv::None => Effective {
                content: c.content.clone(),
                mixed: c.mixed,
                attrs,
                any_attr: own_any,
            },
            Deriv::ExtComplex(_) => {
                let content = match (b.as_ref().map(|b| &b.content), &c.content) {
                    (Some(Content::Particle(p)), Content::Particle(q)) => {
                        let seq = Term::Seq(vec![*p, *q]);
                        Content::Particle(self.push_particle(1, Some(1), seq))
                    }
                    (Some(Content::Particle(p)), Content::Empty) => Content::Particle(*p),
                    (Some(Content::Simple(t)), _) => Content::Simple(t.clone()),
                    (_, own) => own.clone(),
                };
                Effective {
                    content,
                    mixed: c.mixed || b.as_ref().is_some_and(|b| b.mixed),
                    attrs,
                    any_attr: own_any.or_else(|| b.and_then(|b| b.any_attr)),
                }
            }
            Deriv::RestrComplex(_) => Effective {
                content: c.content.clone(),
                mixed: c.mixed,
                attrs,
                any_attr: own_any,
            },
            Deriv::ExtSimple(base) | Deriv::RestrSimple(base, _) => {
                let simple_base = match &b {
                    Some(b) => match &b.content {
                        Content::Simple(t) => t.clone(),
                        _ => TypeRef::Builtin(B::AnySimple),
                    },
                    None => base.clone(),
                };
                let content = match &c.deriv {
                    Deriv::RestrSimple(_, facets) => {
                        self.s.simple.push(Simple {
                            name: None,
                            def: SimpleDef::Restrict {
                                base: simple_base,
                                facets: facets.clone(),
                            },
                        });
                        Content::Simple(TypeRef::Simple(self.s.simple.len() - 1))
                    }
                    _ => Content::Simple(simple_base),
                };
                Effective {
                    content,
                    mixed: false,
                    attrs,
                    any_attr: own_any.or_else(|| b.and_then(|b| b.any_attr)),
                }
            }
        };
        state[i] = 2;
        done[i] = Some(eff.clone());
        Ok(eff)
    }
}

/// Adds an attribute use, replacing one with the same name.
fn merge_attr(list: &mut Vec<AttrUse>, u: AttrUse) {
    match list.iter_mut().find(|a| a.name == u.name) {
        Some(a) => *a = u,
        None => list.push(u),
    }
}

// ----------------------------------------------------------------------
// Matching element children against a content model

#[derive(Clone, Copy, Debug)]
enum Assigned {
    Decl(usize),
    /// Matched a wildcard (the particle's index).
    Wild(usize),
    /// Matched by name an element from a namespace we couldn't load.
    Unknown,
}

struct Matcher<'s> {
    s: &'s Schema,
    kids: Vec<QName>,
    memo: HashMap<(usize, usize), Rc<BTreeSet<usize>>>,
    /// The furthest child position reached, and what could come there.
    furthest: usize,
    expected: Vec<String>,
}

impl Matcher<'_> {
    fn note(&mut self, pos: usize, label: Option<String>) {
        if pos > self.furthest {
            self.furthest = pos;
            self.expected.clear();
        }
        if pos == self.furthest
            && let Some(l) = label
            && !self.expected.contains(&l)
        {
            self.expected.push(l);
        }
    }

    fn label(&self, term: &Term) -> String {
        match term {
            Term::Elem(ElemRef::Local(d)) => format!("<{}>", self.s.elements[*d].name.local),
            Term::Elem(ElemRef::Global(q)) => format!("<{}>", q.local),
            Term::Any(_) => "any element".into(),
            _ => String::new(),
        }
    }

    fn leaf(&self, pid: usize, i: usize) -> Option<Assigned> {
        let kid = self.kids.get(i)?;
        match &self.s.particles[pid].term {
            Term::Elem(ElemRef::Local(d)) => {
                (self.s.elements[*d].name == *kid).then_some(Assigned::Decl(*d))
            }
            Term::Elem(ElemRef::Global(q)) => match self.s.global_elements.get(kid) {
                Some(&g) => (kid == q
                    || (self.s.substitutes_for(g, q) && !self.s.elements[g].abstract_))
                    .then_some(Assigned::Decl(g)),
                // A reference into a namespace we couldn't load matches by name.
                None => (kid == q).then_some(Assigned::Unknown),
            },
            Term::Any(w) => w.allows(&kid.ns).then_some(Assigned::Wild(pid)),
            _ => None,
        }
    }

    /// Positions one occurrence of the particle's term can end at.
    fn term_ends(&mut self, pid: usize, i: usize) -> BTreeSet<usize> {
        let s = self.s;
        let mut out = BTreeSet::new();
        match &s.particles[pid].term {
            Term::Elem(_) | Term::Any(_) => {
                if self.leaf(pid, i).is_some() {
                    out.insert(i + 1);
                    self.note(i + 1, None);
                } else {
                    let l = self.label(&s.particles[pid].term);
                    self.note(i, Some(l));
                }
            }
            Term::Seq(ps) => {
                let mut cur = BTreeSet::from([i]);
                for &p in ps {
                    let mut next = BTreeSet::new();
                    for &pos in &cur {
                        next.extend(self.ends(p, pos).iter().copied());
                    }
                    cur = next;
                    if cur.is_empty() {
                        break;
                    }
                }
                out = cur;
            }
            Term::Choice(ps) => {
                for &p in ps {
                    out.extend(self.ends(p, i).iter().copied());
                }
            }
            Term::All(ps) => {
                let (j, _, missing) = self.all_greedy(ps, i);
                if !missing {
                    out.insert(j);
                }
            }
            Term::Group(q) => {
                if let Some(&g) = s.groups.get(q) {
                    out.extend(self.ends(g, i).iter().copied());
                }
            }
        }
        out
    }

    /// Matches an `xs:all` greedily: returns the end, the assignments, and
    /// whether a required member is missing.
    #[allow(clippy::type_complexity)]
    fn all_greedy(&mut self, ps: &[usize], i: usize) -> (usize, Vec<(usize, Assigned)>, bool) {
        let s = self.s;
        let mut used = vec![false; ps.len()];
        let mut assigned = Vec::new();
        let mut j = i;
        'outer: while j < self.kids.len() {
            for (k, &p) in ps.iter().enumerate() {
                if !used[k]
                    && let Some(a) = self.leaf(p, j)
                {
                    used[k] = true;
                    assigned.push((j, a));
                    j += 1;
                    continue 'outer;
                }
            }
            break;
        }
        self.note(j, None);
        let mut missing = false;
        for (k, &p) in ps.iter().enumerate() {
            if !used[k] {
                let l = self.label(&s.particles[p].term);
                self.note(j, Some(l));
                if s.particles[p].min > 0 {
                    missing = true;
                }
            }
        }
        (j, assigned, missing)
    }

    /// Positions the whole particle (with its occurrence counts) can end at.
    fn ends(&mut self, pid: usize, i: usize) -> Rc<BTreeSet<usize>> {
        if let Some(r) = self.memo.get(&(pid, i)) {
            return r.clone();
        }
        // Guard against re-entry (recursive groups) while computing.
        self.memo.insert((pid, i), Rc::new(BTreeSet::new()));
        let p = &self.s.particles[pid];
        let (min, max) = (p.min, p.max);
        let mut result = BTreeSet::new();
        if min == 0 {
            result.insert(i);
        }
        let mut frontier = BTreeSet::from([i]);
        let mut k = 0u32;
        loop {
            if max.is_some_and(|m| k >= m) {
                break;
            }
            k += 1;
            let mut next = BTreeSet::new();
            for &pos in &frontier {
                next.extend(self.term_ends(pid, pos));
            }
            if next.is_empty() {
                break;
            }
            if k >= min {
                let new: BTreeSet<usize> = next.difference(&result).copied().collect();
                result.extend(next.iter().copied());
                if new.is_empty() {
                    break;
                }
                frontier = new;
            } else {
                frontier = next;
            }
        }
        let r = Rc::new(result);
        self.memo.insert((pid, i), r.clone());
        r
    }

    /// Records which declaration matched each child between `i` and `end`.
    fn assign(&mut self, pid: usize, i: usize, end: usize, out: &mut [Option<Assigned>]) {
        let p = &self.s.particles[pid];
        let min = p.min as usize;
        let max = p.max.map(|m| m as usize);
        if i == end && min == 0 {
            return;
        }
        // Breadth-first over (position, occurrences so far capped at min),
        // which finds the fewest occurrences.
        let start = (i, 0usize);
        let target = (end, min);
        let mut prev: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut dist: HashMap<(usize, usize), usize> = HashMap::from([(start, 0)]);
        let mut queue = std::collections::VecDeque::from([start]);
        while let Some(state) = queue.pop_front() {
            if state == target && state != start {
                break;
            }
            let d = dist[&state];
            if max.is_some_and(|m| d >= m) {
                continue;
            }
            for t in self.term_ends(pid, state.0) {
                if t > end {
                    continue;
                }
                let next = (t, (state.1 + 1).min(min));
                if let std::collections::hash_map::Entry::Vacant(e) = dist.entry(next) {
                    e.insert(d + 1);
                    prev.insert(next, state);
                    queue.push_back(next);
                }
            }
        }
        if !prev.contains_key(&target) {
            return;
        }
        let mut path = vec![target];
        let mut cur = target;
        while let Some(&p) = prev.get(&cur) {
            path.push(p);
            cur = p;
            if cur == start {
                break;
            }
        }
        path.reverse();
        for w in path.windows(2) {
            self.assign_term(pid, w[0].0, w[1].0, out);
        }
    }

    fn assign_term(&mut self, pid: usize, i: usize, end: usize, out: &mut [Option<Assigned>]) {
        let s = self.s;
        match &s.particles[pid].term {
            Term::Elem(_) | Term::Any(_) => {
                if i < out.len() {
                    out[i] = self.leaf(pid, i);
                }
            }
            Term::Seq(ps) => {
                // Positions after each member, then walk back from the end.
                let mut sets = vec![BTreeSet::from([i])];
                for &p in ps {
                    let mut next = BTreeSet::new();
                    for &pos in sets.last().expect("set") {
                        next.extend(self.ends(p, pos).iter().copied().filter(|&e| e <= end));
                    }
                    sets.push(next);
                }
                // Walking back, give each earlier member as much as it can
                // take, so the first matching particle wins.
                let mut target = end;
                let mut steps = Vec::new();
                for (k, &p) in ps.iter().enumerate().rev() {
                    let from = sets[k]
                        .iter()
                        .rev()
                        .copied()
                        .find(|&st| self.ends(p, st).contains(&target));
                    let Some(from) = from else {
                        return;
                    };
                    steps.push((p, from, target));
                    target = from;
                }
                for (p, from, to) in steps.into_iter().rev() {
                    self.assign(p, from, to, out);
                }
            }
            Term::Choice(ps) => {
                for &p in ps {
                    if self.ends(p, i).contains(&end) {
                        self.assign(p, i, end, out);
                        return;
                    }
                }
            }
            Term::All(ps) => {
                let (_, assigned, _) = self.all_greedy(ps, i);
                for (j, a) in assigned {
                    out[j] = Some(a);
                }
            }
            Term::Group(q) => {
                if let Some(&g) = s.groups.get(q) {
                    self.assign(g, i, end, out);
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Validating a document

struct Validator<'s, 't> {
    s: &'s Schema,
    text: &'t str,
    problems: Vec<Problem>,
    ids: HashSet<String>,
    idrefs: Vec<(String, usize)>,
}

fn qname_of(n: Node) -> QName {
    QName::new(n.tag_name().namespace().unwrap_or(""), n.tag_name().name())
}

impl Validator<'_, '_> {
    fn problem(&mut self, offset: usize, message: impl Into<String>) {
        self.problems.push(Problem {
            message: message.into(),
            offset,
        });
    }

    /// The element's name as written, like `<p:item>`.
    fn label(&self, n: Node) -> String {
        let rest = &self.text[n.range().start + 1..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .unwrap_or(rest.len());
        format!("<{}>", &rest[..end])
    }

    fn attr_label(&self, a: &roxmltree::Attribute) -> String {
        self.text[a.range_qname()].to_owned()
    }

    fn finish(mut self) -> Vec<Problem> {
        for (id, offset) in std::mem::take(&mut self.idrefs) {
            if !self.ids.contains(&id) {
                self.problem(
                    offset,
                    format!("No element has the ID \"{id}\" that this IDREF points to."),
                );
            }
        }
        self.problems.sort_by_key(|p| p.offset);
        self.problems
    }

    fn record_ids(&mut self, kind: IdKind, value: &str, offset: usize) {
        match kind {
            IdKind::Other => {}
            IdKind::Id => {
                let v = normalize(value, Ws::Collapse);
                if !self.ids.insert(v.clone()) {
                    self.problem(offset, format!("The ID \"{v}\" is used more than once."));
                }
            }
            IdKind::IdRef => self.idrefs.push((normalize(value, Ws::Collapse), offset)),
            IdKind::IdRefs => {
                for t in value.split(is_xml_space).filter(|t| !t.is_empty()) {
                    self.idrefs.push((t.to_owned(), offset));
                }
            }
        }
    }

    fn root(&mut self, root: Node) {
        let q = qname_of(root);
        if let Some(&d) = self.s.global_elements.get(&q) {
            let ty = self.s.elem_type(d);
            self.element(root, Some(d), ty);
        } else if self.s.lenient.contains(&q.ns) {
            self.lax(root);
        } else {
            let mut msg = format!(
                "The schema doesn't declare a root element {}",
                self.label(root)
            );
            if !q.ns.is_empty() {
                msg.push_str(&format!(" in the namespace \"{}\"", q.ns));
            }
            let mut globals: Vec<&QName> = self.s.global_elements.keys().collect();
            globals.sort();
            if !globals.is_empty() {
                let names: Vec<String> = globals
                    .iter()
                    .take(8)
                    .map(|g| format!("<{}>", g.local))
                    .collect();
                msg.push_str(&format!("; it declares {}", names.join(", ")));
            }
            msg.push('.');
            self.problem(root.range().start, msg);
        }
    }

    fn instance_type(&self, n: Node, value: &str) -> Option<TypeRef> {
        let value = value.trim();
        let (p, l) = match value.split_once(':') {
            Some((p, l)) => (Some(p), l),
            None => (None, value),
        };
        let ns = n.lookup_namespace_uri(p).unwrap_or("");
        if ns == XS {
            return if l == "anyType" {
                Some(TypeRef::AnyType)
            } else {
                builtin(l).map(TypeRef::Builtin)
            };
        }
        self.s.types.get(&QName::new(ns, l)).cloned()
    }

    fn element(&mut self, n: Node, decl: Option<usize>, ty: TypeRef) {
        let name = self.label(n);
        let at = n.range().start;
        let mut ty = self.s.resolve(&ty);
        let d = decl.map(|d| &self.s.elements[d]);
        if d.is_some_and(|d| d.abstract_) {
            self.problem(
                at,
                format!("Element {name} is abstract; use an element that substitutes for it."),
            );
        }
        let mut bad_type = false;
        if let Some(xt) = n.attribute((XSI, "type")) {
            match self.instance_type(n, xt) {
                Some(t) => ty = t,
                None => {
                    bad_type = true;
                    self.problem(
                        at,
                        format!("The xsi:type \"{xt}\" on {name} isn't a type in the schema."),
                    );
                }
            }
        }
        if let TypeRef::Complex(i) = ty
            && self.s.complex[i].abstract_
            && !bad_type
        {
            self.problem(
                at,
                format!(
                    "Element {name} has an abstract type; use xsi:type to choose a concrete one."
                ),
            );
        }
        let nil = n
            .attribute((XSI, "nil"))
            .is_some_and(|v| matches!(v.trim(), "true" | "1"));
        if nil && !d.is_some_and(|d| d.nillable) {
            self.problem(
                at,
                format!("Element {name} can't be nil because it isn't nillable."),
            );
        }
        match ty {
            TypeRef::AnyType | TypeRef::Named(_) => self.lax(n),
            TypeRef::Builtin(_) | TypeRef::Simple(_) => {
                self.attributes(n, &[], None, &name);
                self.simple_content(n, &ty, decl, nil, &name);
            }
            TypeRef::Complex(i) => {
                let s = self.s;
                let eff = &s.effective[i];
                self.attributes(n, &eff.attrs, eff.any_attr.as_ref(), &name);
                if nil {
                    if n.children().any(|c| {
                        c.is_element() || (c.is_text() && !c.text().unwrap_or("").trim().is_empty())
                    }) {
                        self.problem(at, format!("Element {name} is nil, so it must be empty."));
                    }
                    return;
                }
                match &eff.content {
                    Content::Empty => {
                        if let Some(c) = n.children().find(Node::is_element) {
                            let l = self.label(c);
                            self.problem(
                                c.range().start,
                                format!("Element {name} must be empty, but it contains {l}."),
                            );
                        } else if !eff.mixed {
                            self.no_text(n, &name);
                        }
                    }
                    Content::Simple(t) => self.simple_content(n, t, decl, false, &name),
                    Content::Particle(p) => {
                        if !eff.mixed {
                            self.no_text(n, &name);
                        }
                        self.children(n, *p, &name);
                    }
                }
            }
        }
    }

    fn no_text(&mut self, n: Node, name: &str) {
        if let Some(t) = n
            .children()
            .find(|c| c.is_text() && !c.text().unwrap_or("").chars().all(is_xml_space))
        {
            let text = t.text().unwrap_or("").trim();
            self.problem(
                t.range().start,
                format!("Element {name} can't contain text (found {}).", show(text)),
            );
        }
    }

    fn simple_content(
        &mut self,
        n: Node,
        ty: &TypeRef,
        decl: Option<usize>,
        nil: bool,
        name: &str,
    ) {
        if let Some(c) = n.children().find(Node::is_element) {
            let l = self.label(c);
            self.problem(
                c.range().start,
                format!("Element {name} can't contain child elements (found {l})."),
            );
            return;
        }
        let text: String = n
            .children()
            .filter(Node::is_text)
            .filter_map(|t| t.text())
            .collect();
        if nil {
            if !text.is_empty() {
                self.problem(
                    n.range().start,
                    format!("Element {name} is nil, so it must be empty."),
                );
            }
            return;
        }
        let d = decl.map(|d| &self.s.elements[d]);
        let value = if text.is_empty() {
            d.and_then(|d| d.default.clone().or_else(|| d.fixed.clone()))
                .unwrap_or(text)
        } else {
            text
        };
        if let Some(fixed) = d.and_then(|d| d.fixed.clone()) {
            let ws = self.s.whitespace(ty);
            if normalize(&value, ws) != normalize(&fixed, ws) {
                self.problem(
                    n.range().start,
                    format!("Element {name} must have the fixed value {}.", show(&fixed)),
                );
                return;
            }
        }
        match self.s.check_value(ty, &value) {
            Ok(kind) => self.record_ids(kind, &value, n.range().start),
            Err(m) => self.problem(n.range().start, format!("Element {name}: {m}")),
        }
    }

    fn attributes(&mut self, n: Node, uses: &[AttrUse], any: Option<&Wildcard>, name: &str) {
        for a in n.attributes() {
            let ns = a.namespace().unwrap_or("");
            if ns == XSI {
                continue;
            }
            let q = QName::new(ns, a.name());
            let label = self.attr_label(&a);
            let at = a.range().start;
            let (ty, fixed) = match uses.iter().find(|u| u.name == q) {
                Some(u) if u.use_ == Use::Prohibited => {
                    self.problem(
                        at,
                        format!("Element {name} doesn't allow attribute \"{label}\"."),
                    );
                    continue;
                }
                Some(u) => (u.ty.clone(), u.fixed.clone()),
                None => match any.filter(|w| w.allows(ns)) {
                    Some(w) if w.process == Process::Skip => continue,
                    Some(w) => match self.s.global_attr(&q) {
                        Some(g) => (g.ty, g.fixed),
                        None if w.process == Process::Lax || self.s.lenient.contains(ns) => {
                            continue;
                        }
                        None => {
                            self.problem(
                                at,
                                format!(
                                    "Attribute \"{label}\" on {name} isn't declared in the schema."
                                ),
                            );
                            continue;
                        }
                    },
                    None => {
                        self.problem(
                            at,
                            format!("Element {name} doesn't allow attribute \"{label}\"."),
                        );
                        continue;
                    }
                },
            };
            match self.s.check_value(&ty, a.value()) {
                Ok(kind) => self.record_ids(kind, a.value(), at),
                Err(m) => {
                    self.problem(at, format!("Attribute \"{label}\" of {name}: {m}"));
                    continue;
                }
            }
            if let Some(f) = fixed {
                let ws = self.s.whitespace(&ty);
                if normalize(a.value(), ws) != normalize(&f, ws) {
                    self.problem(
                        at,
                        format!("Attribute \"{label}\" of {name} must be {}.", show(&f)),
                    );
                }
            }
        }
        for u in uses.iter().filter(|u| u.use_ == Use::Required) {
            let present = n
                .attributes()
                .any(|a| a.namespace().unwrap_or("") == u.name.ns && a.name() == u.name.local);
            if !present {
                let shown = if u.name.ns == XML_NS {
                    format!("xml:{}", u.name.local)
                } else {
                    u.name.local.clone()
                };
                self.problem(
                    n.range().start,
                    format!("Element {name} is missing required attribute \"{shown}\"."),
                );
            }
        }
    }

    /// Validates whatever has a declaration and accepts the rest.
    fn lax(&mut self, n: Node) {
        for a in n.attributes() {
            let q = QName::new(a.namespace().unwrap_or(""), a.name());
            if let Some(g) = self.s.global_attr(&q) {
                match self.s.check_value(&g.ty, a.value()) {
                    Ok(kind) => self.record_ids(kind, a.value(), a.range().start),
                    Err(m) => {
                        let l = self.attr_label(&a);
                        let name = self.label(n);
                        self.problem(a.range().start, format!("Attribute \"{l}\" of {name}: {m}"));
                    }
                }
            }
        }
        for c in n.children().filter(Node::is_element) {
            match self.s.global_elements.get(&qname_of(c)) {
                Some(&d) => {
                    let ty = self.s.elem_type(d);
                    self.element(c, Some(d), ty);
                }
                None => self.lax(c),
            }
        }
    }

    fn children(&mut self, n: Node, pid: usize, name: &str) {
        let kids: Vec<Node> = n.children().filter(Node::is_element).collect();
        let mut m = Matcher {
            s: self.s,
            kids: kids.iter().map(|k| qname_of(*k)).collect(),
            memo: HashMap::new(),
            furthest: 0,
            expected: Vec::new(),
        };
        let ends = m.ends(pid, 0);
        let mut assigned = vec![None; kids.len()];
        if ends.contains(&kids.len()) {
            m.assign(pid, 0, kids.len(), &mut assigned);
        } else {
            let pos = m.furthest.min(kids.len());
            let list = |e: &[String]| {
                if e.len() == 1 {
                    e[0].clone()
                } else {
                    format!("one of {}", e.join(", "))
                }
            };
            if pos < kids.len() {
                let found = self.label(kids[pos]);
                let msg = if m.expected.is_empty() {
                    format!("Element {found} isn't allowed here in {name}.")
                } else {
                    format!("Expected {} but found {found}.", list(&m.expected))
                };
                self.problem(kids[pos].range().start, msg);
            } else {
                let msg = if m.expected.len() == 1 {
                    format!(
                        "Element {name} is missing required child {}.",
                        m.expected[0]
                    )
                } else {
                    format!(
                        "Element {name} is missing a required child: {}.",
                        list(&m.expected)
                    )
                };
                self.problem(n.range().start, msg);
            }
            // Still check the children we can identify by name.
            let decls = local_decls(self.s, pid);
            for (k, a) in kids.iter().zip(assigned.iter_mut()) {
                let q = qname_of(*k);
                *a = decls
                    .iter()
                    .find(|&&d| self.s.elements[d].name == q)
                    .map(|&d| Assigned::Decl(d));
            }
        }
        for (k, a) in kids.into_iter().zip(assigned) {
            match a {
                Some(Assigned::Decl(d)) => {
                    let ty = self.s.elem_type(d);
                    self.element(k, Some(d), ty);
                }
                Some(Assigned::Unknown) => self.lax(k),
                Some(Assigned::Wild(p)) => {
                    let Term::Any(w) = &self.s.particles[p].term else {
                        continue;
                    };
                    match (w.process, self.s.global_elements.get(&qname_of(k))) {
                        (Process::Skip, _) => {}
                        (_, Some(&d)) => {
                            let ty = self.s.elem_type(d);
                            self.element(k, Some(d), ty);
                        }
                        (Process::Lax, None) => self.lax(k),
                        (Process::Strict, None) => {
                            if let Some(t) = k
                                .attribute((XSI, "type"))
                                .and_then(|t| self.instance_type(k, t))
                            {
                                self.element(k, None, t);
                            } else if self.s.lenient.contains(&qname_of(k).ns) {
                                self.lax(k);
                            } else {
                                let l = self.label(k);
                                self.problem(
                                    k.range().start,
                                    format!(
                                        "Element {l} isn't declared in the schema (required by xs:any)."
                                    ),
                                );
                            }
                        }
                    }
                }
                None => {}
            }
        }
    }
}

/// Element declarations reachable in a content model (for reporting after a mismatch).
fn local_decls(s: &Schema, pid: usize) -> Vec<usize> {
    fn walk(s: &Schema, pid: usize, out: &mut Vec<usize>, seen: &mut HashSet<usize>) {
        if !seen.insert(pid) {
            return;
        }
        match &s.particles[pid].term {
            Term::Elem(ElemRef::Local(d)) => out.push(*d),
            Term::Elem(ElemRef::Global(q)) => {
                if let Some(&g) = s.global_elements.get(q) {
                    out.push(g);
                }
            }
            Term::Seq(ps) | Term::Choice(ps) | Term::All(ps) => {
                for &p in ps {
                    walk(s, p, out, seen);
                }
            }
            Term::Group(q) => {
                if let Some(&g) = s.groups.get(q) {
                    walk(s, g, out, seen);
                }
            }
            Term::Any(_) => {}
        }
    }
    let mut out = Vec::new();
    walk(s, pid, &mut out, &mut HashSet::new());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(body: &str) -> String {
        format!(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{body}</xs:schema>"#)
    }

    fn problems(doc: &str, body: &str) -> Vec<String> {
        validate_xsd(doc, &schema(body), None)
            .unwrap_or_else(|e| panic!("{e:?}"))
            .into_iter()
            .map(|p| p.message)
            .collect()
    }

    fn valid(doc: &str, body: &str) {
        let p = problems(doc, body);
        assert!(p.is_empty(), "{doc}: {p:?}");
    }

    fn one(doc: &str, body: &str, needle: &str) {
        let p = problems(doc, body);
        assert!(
            p.len() == 1 && p[0].contains(needle),
            "{doc}: expected one problem with {needle:?}, got {p:?}"
        );
    }

    const ORDER: &str = r#"
<xs:element name="order">
  <xs:complexType>
    <xs:sequence>
      <xs:element name="customer" type="xs:string"/>
      <xs:element name="item" maxOccurs="unbounded">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="name" type="xs:string"/>
            <xs:element name="qty" type="xs:positiveInteger"/>
            <xs:element name="note" type="xs:string" minOccurs="0"/>
          </xs:sequence>
          <xs:attribute name="id" type="xs:ID" use="required"/>
          <xs:attribute name="gift" type="xs:boolean" default="false"/>
        </xs:complexType>
      </xs:element>
    </xs:sequence>
    <xs:attribute name="date" type="xs:date"/>
  </xs:complexType>
</xs:element>"#;

    #[test]
    fn valid_document() {
        valid(
            r#"<order date="2024-02-29"><customer>Ann</customer>
<item id="a1"><name>Pen</name><qty>2</qty></item>
<item id="a2" gift="true"><name>Ink</name><qty>1</qty><note>blue</note></item></order>"#,
            ORDER,
        );
    }

    #[test]
    fn content_model_errors() {
        one(
            "<order><item id='a'><name>x</name><qty>1</qty></item></order>",
            ORDER,
            "Expected <customer> but found <item>.",
        );
        one(
            "<order><customer>A</customer></order>",
            ORDER,
            "Element <order> is missing required child <item>.",
        );
        one(
            "<order><customer>A</customer><item id='a'><name>x</name><qty>1</qty><bogus/></item></order>",
            ORDER,
            "Expected <note> but found <bogus>.",
        );
        one(
            "<order><customer>A</customer><item id='a'><name>x</name><qty>1</qty></item><customer>B</customer></order>",
            ORDER,
            "Expected <item> but found <customer>.",
        );
        one(
            "<order><customer>A</customer><item id='a'><qty>1</qty></item></order>",
            ORDER,
            "Expected <name> but found <qty>.",
        );
        one("<other/>", ORDER, "doesn't declare a root element <other>");
    }

    #[test]
    fn attribute_errors() {
        one(
            "<order><customer>A</customer><item><name>x</name><qty>1</qty></item></order>",
            ORDER,
            "Element <item> is missing required attribute \"id\".",
        );
        one(
            "<order><customer>A</customer><item id='a' color='red'><name>x</name><qty>1</qty></item></order>",
            ORDER,
            "Element <item> doesn't allow attribute \"color\".",
        );
        one(
            "<order date='2023-02-29'><customer>A</customer><item id='a'><name>x</name><qty>1</qty></item></order>",
            ORDER,
            "isn't a valid date",
        );
        one(
            "<order><customer>A</customer><item id='a' gift='yes'><name>x</name><qty>1</qty></item></order>",
            ORDER,
            "boolean",
        );
    }

    #[test]
    fn value_and_id_errors() {
        one(
            "<order><customer>A</customer><item id='a'><name>x</name><qty>0</qty></item></order>",
            ORDER,
            "isn't a valid positive integer",
        );
        one(
            "<order><customer>A</customer><item id='a'><name>x</name><qty>1</qty></item><item id='a'><name>y</name><qty>1</qty></item></order>",
            ORDER,
            "The ID \"a\" is used more than once.",
        );
        one(
            "<order><customer>A</customer><item id='1a'><name>x</name><qty>1</qty></item></order>",
            ORDER,
            "isn't a valid ID",
        );
        let body = r#"<xs:element name="r"><xs:complexType><xs:sequence>
<xs:element name="p" maxOccurs="unbounded"><xs:complexType><xs:attribute name="id" type="xs:ID"/><xs:attribute name="ref" type="xs:IDREF"/><xs:attribute name="refs" type="xs:IDREFS"/></xs:complexType></xs:element>
</xs:sequence></xs:complexType></xs:element>"#;
        valid("<r><p id='a'/><p ref='a' refs='a b'/><p id='b'/></r>", body);
        one(
            "<r><p id='a'/><p ref='zz'/></r>",
            body,
            "No element has the ID \"zz\"",
        );
        one("<r><p refs='a q'/><p id='a'/></r>", body, "\"q\"");
    }

    #[test]
    fn text_and_mixed_content() {
        one(
            "<order>stray<customer>A</customer><item id='a'><name>x</name><qty>1</qty></item></order>",
            ORDER,
            "Element <order> can't contain text (found \"stray\").",
        );
        let body = r#"<xs:element name="p"><xs:complexType mixed="true"><xs:choice minOccurs="0" maxOccurs="unbounded"><xs:element name="b" type="xs:string"/><xs:element name="i" type="xs:string"/></xs:choice></xs:complexType></xs:element>"#;
        valid("<p>Some <b>bold</b> and <i>it</i> text</p>", body);
        one(
            "<p>x<u>no</u></p>",
            body,
            "Expected one of <b>, <i> but found <u>.",
        );
        let empty = r#"<xs:element name="br"><xs:complexType/></xs:element>"#;
        valid("<br/>", empty);
        one("<br>x</br>", empty, "can't contain text");
        one("<br><a/></br>", empty, "must be empty");
        one(
            "<qty><x/></qty>",
            r#"<xs:element name="qty" type="xs:int"/>"#,
            "can't contain child elements",
        );
    }

    #[test]
    fn choice_all_occurs_and_groups() {
        let body = r#"
<xs:group name="names"><xs:sequence><xs:element name="first" type="xs:string"/><xs:element name="last" type="xs:string"/></xs:sequence></xs:group>
<xs:element name="r"><xs:complexType><xs:sequence>
  <xs:choice><xs:element name="a" type="xs:string"/><xs:group ref="names"/></xs:choice>
  <xs:element name="c" type="xs:string" minOccurs="2" maxOccurs="3"/>
  <xs:sequence minOccurs="0" maxOccurs="2"><xs:element name="d" type="xs:string"/><xs:element name="e" type="xs:string" minOccurs="0"/></xs:sequence>
</xs:sequence></xs:complexType></xs:element>"#;
        valid("<r><a/><c/><c/></r>", body);
        valid("<r><first/><last/><c/><c/><c/><d/><d/><e/></r>", body);
        one(
            "<r><a/><c/></r>",
            body,
            "Element <r> is missing required child <c>.",
        );
        one(
            "<r><a/><c/><c/><c/><c/></r>",
            body,
            "Expected <d> but found <c>.",
        );
        one(
            "<r><first/><c/><c/></r>",
            body,
            "Expected <last> but found <c>.",
        );
        one("<r><a/><c/><c/><d/><d/><d/></r>", body, "found <d>");
        let all = r#"<xs:element name="r"><xs:complexType><xs:all><xs:element name="x" type="xs:string"/><xs:element name="y" type="xs:string" minOccurs="0"/></xs:all></xs:complexType></xs:element>"#;
        valid("<r><y/><x/></r>", all);
        valid("<r><x/></r>", all);
        one("<r><y/></r>", all, "missing required child <x>");
        one("<r><x/><x/></r>", all, "found <x>");
    }

    #[test]
    fn simple_type_facets() {
        let body = r#"
<xs:simpleType name="code"><xs:restriction base="xs:string"><xs:pattern value="[A-Z]{2}-\d{3}"/></xs:restriction></xs:simpleType>
<xs:simpleType name="size"><xs:restriction base="xs:token"><xs:enumeration value="S"/><xs:enumeration value="M"/><xs:enumeration value="L"/></xs:restriction></xs:simpleType>
<xs:simpleType name="pct"><xs:restriction base="xs:decimal"><xs:minInclusive value="0"/><xs:maxExclusive value="100"/><xs:totalDigits value="4"/><xs:fractionDigits value="2"/></xs:restriction></xs:simpleType>
<xs:simpleType name="short"><xs:restriction base="xs:string"><xs:minLength value="2"/><xs:maxLength value="4"/></xs:restriction></xs:simpleType>
<xs:simpleType name="exact"><xs:restriction base="xs:hexBinary"><xs:length value="2"/></xs:restriction></xs:simpleType>
<xs:simpleType name="later"><xs:restriction base="xs:date"><xs:minExclusive value="2020-01-01"/></xs:restriction></xs:simpleType>
<xs:simpleType name="ws"><xs:restriction base="xs:string"><xs:whiteSpace value="collapse"/><xs:length value="3"/></xs:restriction></xs:simpleType>
<xs:element name="r"><xs:complexType>
  <xs:attribute name="code" type="code"/><xs:attribute name="size" type="size"/><xs:attribute name="pct" type="pct"/>
  <xs:attribute name="short" type="short"/><xs:attribute name="hex" type="exact"/><xs:attribute name="day" type="later"/><xs:attribute name="ws" type="ws"/>
</xs:complexType></xs:element>"#;
        valid(
            r#"<r code="AB-123" size=" M " pct="99.5" short="abc" hex="0aFF" day="2021-05-01" ws="  a b "/>"#,
            body,
        );
        one(r#"<r code="ab-123"/>"#, body, "doesn't match the pattern");
        one(
            r#"<r size="XL"/>"#,
            body,
            "isn't one of the allowed values: \"S\", \"M\", \"L\"",
        );
        one(r#"<r pct="100"/>"#, body, "must be less than 100");
        one(r#"<r pct="-1"/>"#, body, "less than the minimum of 0");
        one(r#"<r pct="1.234"/>"#, body, "digits");
        one(r#"<r pct="abc"/>"#, body, "isn't a valid decimal number");
        one(
            r#"<r short="a"/>"#,
            body,
            "shorter than the minimum of 2 characters",
        );
        one(
            r#"<r short="abcde"/>"#,
            body,
            "longer than the maximum of 4 characters",
        );
        one(r#"<r hex="0a"/>"#, body, "exactly 2 bytes");
        one(
            r#"<r day="2020-01-01"/>"#,
            body,
            "must be greater than 2020-01-01",
        );
        one(r#"<r ws="a  bc"/>"#, body, "exactly 3 characters");
    }

    #[test]
    fn lists_and_unions() {
        let body = r#"
<xs:simpleType name="ints"><xs:list itemType="xs:int"/></xs:simpleType>
<xs:simpleType name="few"><xs:restriction base="ints"><xs:maxLength value="3"/></xs:restriction></xs:simpleType>
<xs:simpleType name="numOrAuto"><xs:union memberTypes="xs:integer"><xs:simpleType><xs:restriction base="xs:string"><xs:enumeration value="auto"/></xs:restriction></xs:simpleType></xs:union></xs:simpleType>
<xs:element name="r"><xs:complexType><xs:attribute name="l" type="few"/><xs:attribute name="u" type="numOrAuto"/></xs:complexType></xs:element>"#;
        valid(r#"<r l=" 1 2  3" u="auto"/>"#, body);
        valid(r#"<r u="42"/>"#, body);
        one(r#"<r l="1 x"/>"#, body, "\"x\" isn't a valid int");
        one(r#"<r l="1 2 3 4"/>"#, body, "maximum of 3 items");
        one(
            r#"<r u="big"/>"#,
            body,
            "isn't valid for any of the allowed types",
        );
    }

    #[test]
    fn builtin_types() {
        let check = |b: B, v: &str| b.check(&normalize(v, b.whitespace()));
        for (b, good, bad) in [
            (B::Integer, "-0012", "1.0"),
            (B::Byte, "-128", "128"),
            (
                B::UnsignedLong,
                "18446744073709551615",
                "18446744073709551616",
            ),
            (B::Decimal, "+.5", "1e3"),
            (B::Float, "-1.5E-3", "1,5"),
            (B::Double, "INF", "inf"),
            (B::Boolean, "1", "True"),
            (B::Date, "2000-02-29Z", "1900-02-29"),
            (
                B::DateTime,
                "2024-01-31T23:59:59.5+05:30",
                "2024-01-31 23:59:59",
            ),
            (B::Time, "24:00:00", "24:00:01"),
            (B::GYearMonth, "2024-12", "2024-13"),
            (B::GYear, "-0044", "0000"),
            (B::GMonthDay, "--02-29", "--02-30"),
            (B::GDay, "---31", "---32"),
            (B::GMonth, "--12", "--13"),
            (B::Duration, "P1Y2M3DT4H5M6.5S", "P1Y2MT"),
            (B::Duration, "-PT0S", "P"),
            (B::HexBinary, "00ff", "0f0"),
            (B::Base64Binary, "aGVsbG8=", "aGVsbG8"),
            (B::Language, "en-US", "toolongname-x"),
            (B::NCName, "a.b-c", "a:b"),
            (B::QName, "p:local", "p:"),
            (B::NmTokens, "a b 1", ""),
            (B::Name, ":x", "1x"),
            (B::NormalizedString, "a\tb", ""),
        ] {
            assert!(check(b, good), "{b:?} should accept {good:?}");
            if !(bad.is_empty() && matches!(b, B::NormalizedString)) {
                assert!(!check(b, bad), "{b:?} should reject {bad:?}");
            }
        }
        assert_eq!(base64_octets("aGk="), Some(2));
        assert!(
            parse_date(DateKind::DateTime, "2024-01-01T00:00:00Z")
                == parse_date(DateKind::DateTime, "2024-01-01T01:00:00+01:00")
        );
    }

    #[test]
    fn complex_derivation() {
        let body = r#"
<xs:complexType name="base"><xs:sequence><xs:element name="a" type="xs:string"/></xs:sequence><xs:attribute name="x" type="xs:int"/></xs:complexType>
<xs:complexType name="ext"><xs:complexContent><xs:extension base="base"><xs:sequence><xs:element name="b" type="xs:string"/></xs:sequence><xs:attribute name="y" type="xs:int" use="required"/></xs:extension></xs:complexContent></xs:complexType>
<xs:complexType name="restr"><xs:complexContent><xs:restriction base="base"><xs:sequence><xs:element name="a" type="xs:string"/></xs:sequence><xs:attribute name="x" use="prohibited"/></xs:restriction></xs:complexContent></xs:complexType>
<xs:complexType name="price"><xs:simpleContent><xs:extension base="xs:decimal"><xs:attribute name="cur" type="xs:string" use="required"/></xs:extension></xs:simpleContent></xs:complexType>
<xs:complexType name="smallPrice"><xs:simpleContent><xs:restriction base="price"><xs:maxInclusive value="10"/></xs:restriction></xs:simpleContent></xs:complexType>
<xs:element name="r"><xs:complexType><xs:sequence>
  <xs:element name="e" type="ext"/><xs:element name="s" type="restr"/><xs:element name="p" type="price"/><xs:element name="q" type="smallPrice" minOccurs="0"/>
</xs:sequence></xs:complexType></xs:element>"#;
        valid(
            r#"<r><e x="1" y="2"><a/><b/></e><s><a/></s><p cur="EUR">9.99</p><q cur="USD">10</q></r>"#,
            body,
        );
        one(
            r#"<r><e y="2"><a/></e><s><a/></s><p cur="E">1</p></r>"#,
            body,
            "missing required child <b>",
        );
        one(
            r#"<r><e x="1"><a/><b/></e><s><a/></s><p cur="E">1</p></r>"#,
            body,
            "missing required attribute \"y\"",
        );
        one(
            r#"<r><e y="1"><a/><b/></e><s x="1"><a/></s><p cur="E">1</p></r>"#,
            body,
            "doesn't allow attribute \"x\"",
        );
        one(
            r#"<r><e y="1"><a/><b/></e><s><a/></s><p cur="E">cheap</p></r>"#,
            body,
            "isn't a valid decimal",
        );
        one(
            r#"<r><e y="1"><a/><b/></e><s><a/></s><p>1</p></r>"#,
            body,
            "missing required attribute \"cur\"",
        );
        one(
            r#"<r><e y="1"><a/><b/></e><s><a/></s><p cur="E">1</p><q cur="E">11</q></r>"#,
            body,
            "more than the maximum of 10",
        );
    }

    #[test]
    fn attribute_groups_defaults_and_fixed() {
        let body = r###"
<xs:attributeGroup name="common"><xs:attribute name="lang" type="xs:language"/><xs:attribute name="v" type="xs:string" fixed="1"/></xs:attributeGroup>
<xs:attribute name="global" type="xs:int"/>
<xs:element name="r"><xs:complexType><xs:simpleContent><xs:extension base="xs:string"><xs:attributeGroup ref="common"/><xs:attribute ref="global"/><xs:anyAttribute namespace="##other" processContents="lax"/></xs:extension></xs:simpleContent></xs:complexType></xs:element>
<xs:element name="f" type="xs:string" fixed="yes"/>"###;
        valid(
            r#"<r lang="en" v="1" global="3" xmlns:o="urn:o" o:extra="anything">t</r>"#,
            body,
        );
        one(r#"<r v="2">t</r>"#, body, "must be \"1\"");
        one(r#"<r lang="not a language">t</r>"#, body, "language code");
        one(
            r#"<r unknown="1">t</r>"#,
            body,
            "doesn't allow attribute \"unknown\"",
        );
        valid("<f>yes</f>", body);
        valid("<f/>", body);
        one("<f>no</f>", body, "must have the fixed value \"yes\"");
    }

    #[test]
    fn namespaces_and_forms() {
        let body = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" targetNamespace="urn:t" elementFormDefault="qualified">
<xs:element name="r"><xs:complexType><xs:sequence><xs:element name="a" type="t:code"/><xs:element name="b" form="unqualified" type="xs:string"/></xs:sequence><xs:attribute name="n" type="xs:int"/></xs:complexType></xs:element>
<xs:simpleType name="code"><xs:restriction base="xs:string"><xs:length value="1"/></xs:restriction></xs:simpleType>
</xs:schema>"#;
        let p = |doc: &str| -> Vec<String> {
            validate_xsd(doc, body, None)
                .unwrap()
                .into_iter()
                .map(|p| p.message)
                .collect()
        };
        assert!(p(r#"<r xmlns="urn:t" n="1"><a>x</a><b xmlns="">y</b></r>"#).is_empty());
        assert!(p(r#"<t:r xmlns:t="urn:t"><t:a>x</t:a><b>y</b></t:r>"#).is_empty());
        let e = p(r#"<t:r xmlns:t="urn:t"><a>x</a><b>y</b></t:r>"#);
        assert!(e[0].contains("Expected <a> but found <a>"), "{e:?}");
        let e = p(r#"<r><a/></r>"#);
        assert!(e[0].contains("doesn't declare a root element <r>"), "{e:?}");
        let e = p(r#"<r xmlns="urn:t"><a>xy</a><b xmlns="">y</b></r>"#);
        assert!(e[0].contains("exactly 1 characters"), "{e:?}");
    }

    #[test]
    fn wildcards_nil_xsi_type_and_substitution() {
        let body = r###"
<xs:complexType name="shape" abstract="true"><xs:attribute name="id" type="xs:string"/></xs:complexType>
<xs:complexType name="circle"><xs:complexContent><xs:extension base="shape"><xs:attribute name="r" type="xs:double" use="required"/></xs:extension></xs:complexContent></xs:complexType>
<xs:element name="note" type="xs:string"/>
<xs:element name="msg" type="xs:string" substitutionGroup="note"/>
<xs:element name="r"><xs:complexType><xs:sequence>
  <xs:element name="s" type="shape" minOccurs="0"/>
  <xs:element ref="note" maxOccurs="unbounded"/>
  <xs:element name="n" type="xs:int" nillable="true" minOccurs="0"/>
  <xs:any namespace="##other" processContents="lax" minOccurs="0"/>
  <xs:any namespace="##local" processContents="strict" minOccurs="0"/>
</xs:sequence></xs:complexType></xs:element>"###;
        let xsi = r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#;
        valid(
            &format!(
                r#"<r {xsi}><s xsi:type="circle" r="2"/><note/><msg>hi</msg><n xsi:nil="true"/><o:x xmlns:o="urn:o"/></r>"#
            ),
            body,
        );
        valid(&format!(r#"<r {xsi}><note/><note/></r>"#), body);
        one(
            &format!(r#"<r {xsi}><s/><note/></r>"#),
            body,
            "abstract type",
        );
        one(
            &format!(r#"<r {xsi}><s xsi:type="nope"/><note/></r>"#),
            body,
            "xsi:type \"nope\"",
        );
        one(
            &format!(r#"<r {xsi}><s xsi:type="circle"/><note/></r>"#),
            body,
            "missing required attribute \"r\"",
        );
        one(
            &format!(r#"<r {xsi}><note/><n xsi:nil="true">5</n></r>"#),
            body,
            "is nil, so it must be empty",
        );
        one(
            &format!(r#"<r {xsi}><note/><mystery/></r>"#),
            body,
            "isn't declared in the schema (required by xs:any)",
        );
        one(
            &format!(r#"<r {xsi}><note/><n>x</n></r>"#),
            body,
            "isn't a valid int",
        );
    }

    #[test]
    fn regex_translation() {
        let m = |p: &str, s: &str| Regex::new(&xsd_regex(p).unwrap()).unwrap().is_match(s);
        assert!(m(r"\d{3}-\d{4}", "555-1234"));
        assert!(!m(r"\d{3}", "1234"));
        assert!(m("[a-z-[aeiou]]+", "bcd"));
        assert!(!m("[a-z-[aeiou]]+", "bad"));
        assert!(m(r"\i\c*", "_x.1"));
        assert!(!m(r"\i\c*", "1x"));
        assert!(m(r"a$b^", "a$b^"));
        assert!(m(r"\p{Lu}\P{Lu}*", "Ab1"));
        assert!(m(r"\p{IsBasicLatin}+", "abc"));
        assert!(!m(r"\p{IsBasicLatin}+", "é"));
        assert!(m(".", "x"));
        assert!(!m(".", "\n"));
        assert!(m(r"[\s\d]+", " 1 2"));
        assert!(m("(ab|cd)+", "abcd"));
        assert!(m("[^0-9]", "x"));
        assert!(xsd_regex("[a").is_err());
        assert!(xsd_regex(r"\q").is_err());
    }

    #[test]
    fn schema_errors() {
        let err = |body: &str| validate_xsd("<r/>", &schema(body), None).unwrap_err();
        assert!(
            err(r#"<xs:element name="r" type="missing"/>"#)
                .message
                .contains("The type \"missing\" isn't defined")
        );
        assert!(
            err(r#"<xs:element name="r" type="xs:strung"/>"#)
                .message
                .contains("isn't a built-in")
        );
        assert!(err(r#"<xs:element name="r"><xs:complexType><xs:sequence><xs:element ref="nope"/></xs:sequence></xs:complexType></xs:element>"#).message.contains("isn't declared"));
        assert!(err(r#"<xs:simpleType name="t"><xs:restriction base="xs:string"><xs:pattern value="[a"/></xs:restriction></xs:simpleType>"#).message.contains("isn't a valid regular expression"));
        assert!(
            err(r#"<xs:include schemaLocation="other.xsd"/>"#)
                .message
                .contains("folder is unknown")
        );
        assert!(err(r#"<xs:element/>"#).message.contains("needs a name"));
        let e = validate_xsd("<r/>", "<schema/>", None).unwrap_err();
        assert!(e.message.contains("isn't an XML Schema"));
        let e = validate_xsd("<r/>", "<xs:schema", None).unwrap_err();
        assert!(e.message.contains("well-formed"));
        let e = validate_xsd("<r>", &schema(""), None).unwrap_err();
        assert!(e.message.contains("document isn't well-formed"));
        let e = err(
            r#"<xs:complexType name="a"><xs:complexContent><xs:extension base="b"/></xs:complexContent></xs:complexType><xs:complexType name="b"><xs:complexContent><xs:extension base="a"/></xs:complexContent></xs:complexType>"#,
        );
        assert!(e.message.contains("derives from itself"), "{e:?}");
        let e = err("\n\n<xs:element name='r' type='zz'/>");
        assert!(e.message.contains("schema line 3"), "{e:?}");
        assert!(e.offset.is_some());
    }

    #[test]
    fn include_and_import_files() {
        let dir = std::env::temp_dir().join(format!("ettin-xsd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("types.xsd"),
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:simpleType name="small"><xs:restriction base="xs:int"><xs:maxInclusive value="5"/></xs:restriction></xs:simpleType></xs:schema>"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("other.xsd"),
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:o"><xs:element name="extra" type="xs:boolean"/></xs:schema>"#,
        )
        .unwrap();
        let main = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" xmlns:o="urn:o" targetNamespace="urn:t">
<xs:include schemaLocation="types.xsd"/>
<xs:import namespace="urn:o" schemaLocation="other.xsd"/>
<xs:import namespace="urn:remote" schemaLocation="http://example.com/remote.xsd"/>
<xs:element name="r"><xs:complexType><xs:sequence><xs:element name="n" type="t:small" form="unqualified"/><xs:element ref="o:extra"/><xs:any namespace="urn:remote" minOccurs="0"/></xs:sequence></xs:complexType></xs:element>
</xs:schema>"#;
        let run = |doc: &str| -> Vec<String> {
            validate_xsd(doc, main, Some(&dir))
                .unwrap()
                .into_iter()
                .map(|p| p.message)
                .collect()
        };
        assert!(run(r#"<t:r xmlns:t="urn:t" xmlns:o="urn:o"><n>3</n><o:extra>true</o:extra><x:y xmlns:x="urn:remote"/></t:r>"#).is_empty());
        let e =
            run(r#"<t:r xmlns:t="urn:t" xmlns:o="urn:o"><n>9</n><o:extra>maybe</o:extra></t:r>"#);
        assert_eq!(e.len(), 2, "{e:?}");
        assert!(e[0].contains("maximum of 5"));
        assert!(e[1].contains("boolean"));
        let missing = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:include schemaLocation="nothere.xsd"/></xs:schema>"#;
        let e = validate_xsd("<r/>", missing, Some(&dir)).unwrap_err();
        assert!(
            e.message
                .contains("Couldn't read the schema \"nothere.xsd\""),
            "{e:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn problem_offsets_and_order() {
        let doc = "<order><customer>A</customer><item id='a' bad='1'><name>x</name><qty>-1</qty></item></order>";
        let p = validate_xsd(doc, &schema(ORDER), None).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(&doc[p[0].offset..p[0].offset + 7], "bad='1'");
        assert!(doc[p[1].offset..].starts_with("<qty>"));
    }

    #[test]
    fn finds_schema_location() {
        assert_eq!(
            schema_location(
                r#"<r xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation=" a.xsd "/>"#
            ),
            Some("a.xsd".into())
        );
        assert_eq!(
            schema_location(
                r#"<r xmlns="urn:b" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="urn:a a.xsd  urn:b b.xsd"/>"#
            ),
            Some("b.xsd".into())
        );
        assert_eq!(schema_location("<r/>"), None);
        assert_eq!(schema_location("<r"), None);
    }
}
