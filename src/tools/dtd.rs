//! DTD validation, written from the XML 1.0 recommendation.
//!
//! Reads the document's internal subset and, optionally, an external DTD's
//! text, then checks element content models (EMPTY, ANY, mixed, and
//! sequences/choices with `?`, `*` and `+`) and attribute lists (CDATA, ID,
//! IDREF(S), ENTITY(IES), NMTOKEN(S), NOTATION and enumerations, with
//! #REQUIRED, #IMPLIED, #FIXED and defaults). Parameter entities and
//! conditional sections are expanded; external parameter entities can't be
//! loaded and count as empty.
#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap, HashSet};

use roxmltree::Node;

pub use super::Problem;
use super::{ToolError, line_col, xml};

/// Validates `doc_text` against its internal DTD subset plus `external_dtd`
/// (the text of the file its DOCTYPE names, if the caller loaded it).
pub fn validate_dtd(doc_text: &str, external_dtd: Option<&str>) -> Result<Vec<Problem>, ToolError> {
    std::thread::scope(|s| {
        match std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn_scoped(s, || run(doc_text, external_dtd))
        {
            Ok(h) => h
                .join()
                .unwrap_or_else(|_| Err(ToolError::new("Validation failed unexpectedly."))),
            Err(_) => run(doc_text, external_dtd),
        }
    })
}

/// The system identifier (usually a file name) of the document's DOCTYPE.
pub fn doctype_system_id(doc_text: &str) -> Option<String> {
    find_doctype(doc_text).ok().flatten()?.system_id
}

fn run(doc_text: &str, external_dtd: Option<&str>) -> Result<Vec<Problem>, ToolError> {
    let doctype = find_doctype(doc_text)?;
    let mut dtd = Dtd::default();
    // The internal subset is read first, so its declarations take precedence.
    if let Some(dt) = &doctype
        && let Some((start, subset)) = &dt.internal
    {
        DtdParser {
            dtd: &mut dtd,
            text: doc_text,
            external: false,
            depth: 0,
        }
        .subset(subset, *start)?;
    }
    if let Some(ext) = external_dtd {
        DtdParser {
            dtd: &mut dtd,
            text: ext,
            external: true,
            depth: 0,
        }
        .subset(ext, 0)?;
    }
    if doctype.is_none() && external_dtd.is_none() {
        return Err(ToolError::new(
            "The document has no DOCTYPE, and no DTD was given to validate against.",
        ));
    }
    if dtd.elements.is_empty() && dtd.attlists.is_empty() {
        let hint = match doctype.as_ref().and_then(|d| d.system_id.as_deref()) {
            Some(sys) if external_dtd.is_none() => {
                format!(" Load its external DTD (\"{sys}\") to validate.")
            }
            _ => String::new(),
        };
        return Err(ToolError::new(format!(
            "The DTD doesn't declare any elements.{hint}"
        )));
    }
    let doc = xml::parse_strict(doc_text).map_err(|e| ToolError {
        message: format!("The XML document isn't well-formed: {}", e.message),
        offset: e.offset,
    })?;
    let mut v = Validator {
        dtd: &dtd,
        text: doc_text,
        problems: Vec::new(),
        ids: HashSet::new(),
        idrefs: Vec::new(),
    };
    let root = doc.root_element();
    if let Some(dt) = &doctype {
        let name = v.raw_name(root);
        if name != dt.name {
            v.problem(
                root.range().start,
                format!(
                    "The root element <{name}> doesn't match the DOCTYPE name <{}>.",
                    dt.name
                ),
            );
        }
    }
    for n in root.descendants().filter(Node::is_element) {
        v.element(n);
    }
    Ok(v.finish())
}

// ----------------------------------------------------------------------
// Finding the DOCTYPE

struct Doctype {
    name: String,
    public_id: Option<String>,
    system_id: Option<String>,
    /// The internal subset and where it starts in the document.
    internal: Option<(usize, String)>,
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == ':'
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric()
        || matches!(c, '_' | ':' | '-' | '.' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}')
}

fn is_name(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(is_name_start) && cs.all(is_name_char)
}

fn is_nmtoken(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_name_char)
}

fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// A cursor over DTD or prolog text.
struct Cur<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Cur<'a> {
    fn rest(&self) -> &'a str {
        &self.s[self.i..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn ws(&mut self) -> bool {
        let start = self.i;
        while let Some(c) = self.peek().filter(|&c| is_xml_space(c)) {
            self.i += c.len_utf8();
        }
        self.i > start
    }

    fn eat(&mut self, t: &str) -> bool {
        if self.rest().starts_with(t) {
            self.i += t.len();
            true
        } else {
            false
        }
    }

    fn name(&mut self) -> Option<&'a str> {
        let start = self.i;
        if !self.peek().is_some_and(is_name_start) {
            return None;
        }
        while let Some(c) = self.peek().filter(|&c| is_name_char(c)) {
            self.i += c.len_utf8();
        }
        Some(&self.s[start..self.i])
    }

    fn nmtoken(&mut self) -> Option<&'a str> {
        let start = self.i;
        while let Some(c) = self.peek().filter(|&c| is_name_char(c)) {
            self.i += c.len_utf8();
        }
        (self.i > start).then(|| &self.s[start..self.i])
    }

    fn literal(&mut self) -> Option<&'a str> {
        let q = self.peek().filter(|&c| c == '"' || c == '\'')?;
        let end = self.s[self.i + 1..].find(q)?;
        let lit = &self.s[self.i + 1..self.i + 1 + end];
        self.i += end + 2;
        Some(lit)
    }

    /// Skips to just after `t`; false if it never comes.
    fn skip_past(&mut self, t: &str) -> bool {
        match self.rest().find(t) {
            Some(p) => {
                self.i += p + t.len();
                true
            }
            None => false,
        }
    }
}

fn find_doctype(text: &str) -> Result<Option<Doctype>, ToolError> {
    let mut c = Cur { s: text, i: 0 };
    c.eat("\u{feff}");
    loop {
        c.ws();
        if c.rest().starts_with("<?") {
            if !c.skip_past("?>") {
                return Ok(None);
            }
        } else if c.rest().starts_with("<!--") {
            if !c.skip_past("-->") {
                return Ok(None);
            }
        } else {
            break;
        }
    }
    let start = c.i;
    if !c.eat("<!DOCTYPE") {
        return Ok(None);
    }
    let err = |c: &Cur, m: &str| ToolError::at(format!("{m} in the DOCTYPE."), c.i);
    c.ws();
    let name = c
        .name()
        .ok_or_else(|| err(&c, "Expected the root element's name"))?;
    let mut dt = Doctype {
        name: name.to_owned(),
        public_id: None,
        system_id: None,
        internal: None,
    };
    c.ws();
    if c.eat("SYSTEM") {
        c.ws();
        dt.system_id = Some(
            c.literal()
                .ok_or_else(|| err(&c, "Expected a quoted system ID"))?
                .to_owned(),
        );
    } else if c.eat("PUBLIC") {
        c.ws();
        dt.public_id = Some(
            c.literal()
                .ok_or_else(|| err(&c, "Expected a quoted public ID"))?
                .to_owned(),
        );
        c.ws();
        dt.system_id = c.literal().map(str::to_owned);
    }
    c.ws();
    if c.eat("[") {
        let subset_start = c.i;
        // Find the closing ']', skipping comments, PIs and quoted strings.
        loop {
            let r = c.rest();
            if r.is_empty() {
                return Err(ToolError::at(
                    "The DOCTYPE's internal subset is never closed with ']'.",
                    start,
                ));
            }
            if r.starts_with("<!--") {
                if !c.skip_past("-->") {
                    return Err(ToolError::at("A comment in the DTD is never closed.", c.i));
                }
            } else if r.starts_with("<?") {
                if !c.skip_past("?>") {
                    return Err(ToolError::at(
                        "A processing instruction in the DTD is never closed.",
                        c.i,
                    ));
                }
            } else if r.starts_with('"') || r.starts_with('\'') {
                if c.literal().is_none() {
                    return Err(ToolError::at(
                        "A quoted string in the DTD is never closed.",
                        c.i,
                    ));
                }
            } else if r.starts_with(']') && !r.starts_with("]]>") {
                break;
            } else {
                c.i += r.chars().next().map_or(1, char::len_utf8);
            }
        }
        dt.internal = Some((subset_start, text[subset_start..c.i].to_owned()));
        c.i += 1;
        c.ws();
    }
    if !c.eat(">") {
        return Err(err(&c, "Expected '>'"));
    }
    Ok(Some(dt))
}

// ----------------------------------------------------------------------
// Parsing the DTD

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Occ {
    One,
    Opt,
    Star,
    Plus,
}

#[derive(Clone, Debug, PartialEq)]
enum Cm {
    Name(String, Occ),
    Seq(Vec<Cm>, Occ),
    Choice(Vec<Cm>, Occ),
}

#[derive(Clone, Debug, PartialEq)]
enum ContentSpec {
    Empty,
    Any,
    Mixed(Vec<String>),
    Children(Cm),
}

#[derive(Clone, Debug, PartialEq)]
enum AttType {
    Cdata,
    Id,
    IdRef,
    IdRefs,
    Entity,
    Entities,
    NmToken,
    NmTokens,
    Notation(Vec<String>),
    Enum(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
enum AttDefault {
    Required,
    Implied,
    Fixed(String),
    Default(String),
}

#[derive(Clone, Debug)]
struct AttDef {
    name: String,
    ty: AttType,
    default: AttDefault,
}

#[derive(Debug, Default)]
struct Dtd {
    elements: HashMap<String, ContentSpec>,
    attlists: HashMap<String, Vec<AttDef>>,
    params: HashMap<String, String>,
    unparsed_entities: HashSet<String>,
}

struct DtdParser<'d, 't> {
    dtd: &'d mut Dtd,
    /// The text offsets refer to: the document, or the external DTD.
    text: &'t str,
    external: bool,
    depth: usize,
}

impl DtdParser<'_, '_> {
    fn err(&self, offset: usize, message: impl Into<String>) -> ToolError {
        let message = message.into();
        let offset = offset.min(self.text.len());
        let (line, col) = line_col(self.text, offset);
        if self.external {
            ToolError::new(format!("{message} (DTD line {line}, column {col})"))
        } else {
            ToolError::at(format!("{message} (line {line}, column {col})"), offset)
        }
    }

    /// Replaces `%name;` references with their text.
    fn expand(&self, s: &str, at: usize, depth: usize) -> Result<String, ToolError> {
        if !s.contains('%') {
            return Ok(s.to_owned());
        }
        if depth > 16 {
            return Err(self.err(at, "Parameter entities refer to each other endlessly"));
        }
        let mut out = String::new();
        let mut rest = s;
        while let Some(p) = rest.find('%') {
            out.push_str(&rest[..p]);
            let after = &rest[p + 1..];
            let mut c = Cur { s: after, i: 0 };
            match c.name() {
                Some(name) if c.eat(";") => {
                    let value = self.dtd.params.get(name).ok_or_else(|| {
                        self.err(at, format!("The parameter entity %{name}; isn't declared"))
                    })?;
                    out.push(' ');
                    out.push_str(&self.expand(value, at, depth + 1)?);
                    out.push(' ');
                    rest = &after[c.i..];
                }
                _ => {
                    out.push('%');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        Ok(out)
    }

    /// Parses a sequence of markup declarations. `base` is where `s` starts in `self.text`.
    fn subset(&mut self, s: &str, base: usize) -> Result<(), ToolError> {
        self.depth += 1;
        if self.depth > 32 {
            return Err(self.err(base, "The DTD nests too deeply"));
        }
        let mut c = Cur { s, i: 0 };
        loop {
            c.ws();
            let at = base + c.i;
            let r = c.rest();
            if r.is_empty() {
                break;
            }
            if r.starts_with("<!--") {
                if !c.skip_past("-->") {
                    return Err(self.err(at, "A comment in the DTD is never closed"));
                }
            } else if r.starts_with("<?") {
                if !c.skip_past("?>") {
                    return Err(self.err(at, "A processing instruction in the DTD is never closed"));
                }
            } else if r.starts_with("<![") {
                c.i += 3;
                let Some(open) = c.rest().find('[') else {
                    return Err(self.err(at, "A conditional section is missing its '['"));
                };
                let keyword = self.expand(&c.rest()[..open], at, 0)?;
                c.i += open + 1;
                // Find the matching ]]>, allowing nested sections.
                let body_start = c.i;
                let mut nest = 1;
                while nest > 0 {
                    let r = c.rest();
                    if r.is_empty() {
                        return Err(
                            self.err(at, "A conditional section is never closed with ']]>'")
                        );
                    }
                    if r.starts_with("<![") {
                        nest += 1;
                        c.i += 3;
                    } else if r.starts_with("]]>") {
                        nest -= 1;
                        c.i += 3;
                    } else {
                        c.i += r.chars().next().map_or(1, char::len_utf8);
                    }
                }
                let body = &s[body_start..c.i - 3];
                match keyword.trim() {
                    "INCLUDE" => self.subset(body, base + body_start)?,
                    "IGNORE" => {}
                    k => {
                        return Err(self.err(
                            at,
                            format!(
                                "A conditional section should say INCLUDE or IGNORE, not '{k}'"
                            ),
                        ));
                    }
                }
            } else if r.starts_with("<!") {
                let end = decl_end(r)
                    .ok_or_else(|| self.err(at, "This declaration is never closed with '>'"))?;
                let decl = &r[..end];
                c.i += end + 1;
                self.declaration(decl, at)?;
            } else if r.starts_with('%') {
                c.i += 1;
                let name = c.name();
                let Some(name) = name.filter(|_| c.eat(";")) else {
                    return Err(self.err(at, "Expected a parameter entity reference like %name;"));
                };
                let value = self.dtd.params.get(name).cloned().ok_or_else(|| {
                    self.err(at, format!("The parameter entity %{name}; isn't declared"))
                })?;
                self.subset(&value, at)?;
            } else {
                let shown: String = r.chars().take(20).collect();
                return Err(self.err(at, format!("Unexpected text in the DTD: \"{shown}\"")));
            }
        }
        self.depth -= 1;
        Ok(())
    }

    /// Parses one `<!...` declaration (without its closing `>`).
    fn declaration(&mut self, decl: &str, at: usize) -> Result<(), ToolError> {
        if let Some(rest) = decl.strip_prefix("<!ENTITY") {
            return self.entity(rest, at);
        }
        if decl.starts_with("<!NOTATION") {
            return Ok(());
        }
        let (kind, rest) = if let Some(r) = decl.strip_prefix("<!ELEMENT") {
            ("ELEMENT", r)
        } else if let Some(r) = decl.strip_prefix("<!ATTLIST") {
            ("ATTLIST", r)
        } else {
            let shown: String = decl.chars().take(20).collect();
            return Err(self.err(at, format!("Unknown declaration \"{shown}\"")));
        };
        let text = self.expand(rest, at, 0)?;
        let mut c = Cur { s: &text, i: 0 };
        if !c.ws() {
            return Err(self.err(at, format!("Expected a space after <!{kind}")));
        }
        let name = c
            .name()
            .ok_or_else(|| self.err(at, format!("Expected an element name in <!{kind}>")))?
            .to_owned();
        if kind == "ELEMENT" {
            c.ws();
            let spec = self
                .content_spec(&mut c)
                .map_err(|m| self.err(at, format!("In <!ELEMENT {name}>: {m}")))?;
            c.ws();
            if !c.rest().is_empty() {
                return Err(self.err(
                    at,
                    format!("Unexpected text at the end of <!ELEMENT {name}>"),
                ));
            }
            // The first declaration wins (XML forbids duplicates).
            self.dtd.elements.entry(name).or_insert(spec);
        } else {
            let defs = self
                .attdefs(&mut c)
                .map_err(|m| self.err(at, format!("In <!ATTLIST {name}>: {m}")))?;
            let list = self.dtd.attlists.entry(name).or_default();
            for d in defs {
                // The first definition of an attribute wins.
                if !list.iter().any(|e| e.name == d.name) {
                    list.push(d);
                }
            }
        }
        Ok(())
    }

    fn entity(&mut self, rest: &str, at: usize) -> Result<(), ToolError> {
        let mut c = Cur { s: rest, i: 0 };
        c.ws();
        let param = c.eat("%");
        c.ws();
        let name = c
            .name()
            .ok_or_else(|| self.err(at, "Expected a name in <!ENTITY>"))?
            .to_owned();
        c.ws();
        if let Some(value) = c.literal() {
            let value = if param {
                self.expand(value, at, 0)?
            } else {
                value.to_owned()
            };
            if param {
                self.dtd.params.entry(name).or_insert(value);
            }
            return Ok(());
        }
        // An external entity: SYSTEM "..." or PUBLIC "..." "...", maybe NDATA.
        if c.eat("SYSTEM") {
            c.ws();
            c.literal();
        } else if c.eat("PUBLIC") {
            c.ws();
            c.literal();
            c.ws();
            c.literal();
        } else {
            return Err(self.err(
                at,
                format!("<!ENTITY {name}> needs a value or SYSTEM/PUBLIC ID"),
            ));
        }
        c.ws();
        if param {
            // External parameter entities can't be loaded here; treat as empty.
            self.dtd.params.entry(name).or_default();
        } else if c.eat("NDATA") {
            self.dtd.unparsed_entities.insert(name);
        }
        Ok(())
    }

    fn content_spec(&self, c: &mut Cur) -> Result<ContentSpec, String> {
        if c.eat("EMPTY") {
            return Ok(ContentSpec::Empty);
        }
        if c.eat("ANY") {
            return Ok(ContentSpec::Any);
        }
        if !c.eat("(") {
            return Err("expected EMPTY, ANY or a content model in parentheses".into());
        }
        c.ws();
        if c.eat("#PCDATA") {
            let mut names = Vec::new();
            loop {
                c.ws();
                if c.eat(")") {
                    break;
                }
                if !c.eat("|") {
                    return Err("expected '|' or ')' in mixed content".into());
                }
                c.ws();
                names.push(c.name().ok_or("expected an element name")?.to_owned());
            }
            if !c.eat("*") && !names.is_empty() {
                return Err("mixed content with element names must end with ')*'".into());
            }
            return Ok(ContentSpec::Mixed(names));
        }
        Ok(ContentSpec::Children(self.group(c)?))
    }

    /// Parses a group after its '(' (and the occurrence after its ')').
    fn group(&self, c: &mut Cur) -> Result<Cm, String> {
        let mut items = Vec::new();
        let mut sep = None;
        loop {
            c.ws();
            let item = if c.eat("(") {
                self.group(c)?
            } else {
                let n = c
                    .name()
                    .ok_or("expected an element name or '('")?
                    .to_owned();
                Cm::Name(n, occ(c))
            };
            items.push(item);
            c.ws();
            if c.eat(")") {
                break;
            }
            let s = if c.eat(",") {
                ','
            } else if c.eat("|") {
                '|'
            } else {
                return Err("expected ',', '|' or ')'".into());
            };
            if sep.is_some_and(|p| p != s) {
                return Err("a group can't mix ',' and '|'; add parentheses".into());
            }
            sep = Some(s);
        }
        let o = occ(c);
        Ok(if sep == Some('|') {
            Cm::Choice(items, o)
        } else {
            Cm::Seq(items, o)
        })
    }

    fn attdefs(&self, c: &mut Cur) -> Result<Vec<AttDef>, String> {
        let mut out = Vec::new();
        loop {
            c.ws();
            if c.rest().is_empty() {
                break;
            }
            let name = c.name().ok_or("expected an attribute name")?.to_owned();
            c.ws();
            let ty = if c.eat("CDATA") {
                AttType::Cdata
            } else if c.eat("IDREFS") {
                AttType::IdRefs
            } else if c.eat("IDREF") {
                AttType::IdRef
            } else if c.eat("ID") {
                AttType::Id
            } else if c.eat("ENTITIES") {
                AttType::Entities
            } else if c.eat("ENTITY") {
                AttType::Entity
            } else if c.eat("NMTOKENS") {
                AttType::NmTokens
            } else if c.eat("NMTOKEN") {
                AttType::NmToken
            } else if c.eat("NOTATION") {
                c.ws();
                if !c.eat("(") {
                    return Err("expected '(' after NOTATION".into());
                }
                AttType::Notation(token_list(c)?)
            } else if c.eat("(") {
                AttType::Enum(token_list(c)?)
            } else {
                return Err(format!("attribute \"{name}\" has an unknown type"));
            };
            c.ws();
            let default = if c.eat("#REQUIRED") {
                AttDefault::Required
            } else if c.eat("#IMPLIED") {
                AttDefault::Implied
            } else {
                let fixed = c.eat("#FIXED");
                c.ws();
                let v = c
                    .literal()
                    .ok_or_else(|| format!("attribute \"{name}\" needs #REQUIRED, #IMPLIED, #FIXED or a default value"))?
                    .to_owned();
                if fixed {
                    AttDefault::Fixed(v)
                } else {
                    AttDefault::Default(v)
                }
            };
            out.push(AttDef { name, ty, default });
        }
        Ok(out)
    }
}

/// The end of a declaration starting at `<!`: the index of its closing '>'.
fn decl_end(s: &str) -> Option<usize> {
    let mut quote = None;
    for (i, c) in s.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '>' => return Some(i),
            None => {}
        }
    }
    None
}

fn occ(c: &mut Cur) -> Occ {
    if c.eat("?") {
        Occ::Opt
    } else if c.eat("*") {
        Occ::Star
    } else if c.eat("+") {
        Occ::Plus
    } else {
        Occ::One
    }
}

/// `a | b | c)` after the '(' of an enumeration.
fn token_list(c: &mut Cur) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    loop {
        c.ws();
        out.push(
            c.nmtoken()
                .ok_or("expected a value in the list")?
                .to_owned(),
        );
        c.ws();
        if c.eat(")") {
            return Ok(out);
        }
        if !c.eat("|") {
            return Err("expected '|' or ')' in the list".into());
        }
    }
}

// ----------------------------------------------------------------------
// Matching content models

struct Matcher<'k> {
    kids: &'k [String],
    memo: HashMap<(*const Cm, usize), BTreeSet<usize>>,
    furthest: usize,
    expected: Vec<String>,
}

impl Matcher<'_> {
    fn note(&mut self, pos: usize, label: Option<&str>) {
        if pos > self.furthest {
            self.furthest = pos;
            self.expected.clear();
        }
        if pos == self.furthest
            && let Some(l) = label
        {
            let l = format!("<{l}>");
            if !self.expected.contains(&l) {
                self.expected.push(l);
            }
        }
    }

    fn once(&mut self, cm: &Cm, i: usize) -> BTreeSet<usize> {
        match cm {
            Cm::Name(n, _) => {
                if self.kids.get(i) == Some(n) {
                    self.note(i + 1, None);
                    BTreeSet::from([i + 1])
                } else {
                    self.note(i, Some(n));
                    BTreeSet::new()
                }
            }
            Cm::Seq(items, _) => {
                let mut cur = BTreeSet::from([i]);
                for it in items {
                    let mut next = BTreeSet::new();
                    for &p in &cur {
                        next.extend(self.ends(it, p));
                    }
                    cur = next;
                    if cur.is_empty() {
                        break;
                    }
                }
                cur
            }
            Cm::Choice(items, _) => {
                let mut out = BTreeSet::new();
                for it in items {
                    out.extend(self.ends(it, i));
                }
                out
            }
        }
    }

    fn ends(&mut self, cm: &Cm, i: usize) -> BTreeSet<usize> {
        let key = (cm as *const Cm, i);
        if let Some(r) = self.memo.get(&key) {
            return r.clone();
        }
        let o = match cm {
            Cm::Name(_, o) | Cm::Seq(_, o) | Cm::Choice(_, o) => *o,
        };
        let mut result = BTreeSet::new();
        if matches!(o, Occ::Opt | Occ::Star) {
            result.insert(i);
        }
        let first = self.once(cm, i);
        result.extend(first.iter().copied());
        if matches!(o, Occ::Star | Occ::Plus) {
            let mut frontier = first;
            while !frontier.is_empty() {
                let mut next = BTreeSet::new();
                for &p in &frontier {
                    next.extend(self.once(cm, p));
                }
                frontier = next.difference(&result).copied().collect();
                result.extend(next);
            }
        }
        self.memo.insert(key, result.clone());
        result
    }
}

// ----------------------------------------------------------------------
// Validating

struct Validator<'d, 't> {
    dtd: &'d Dtd,
    text: &'t str,
    problems: Vec<Problem>,
    ids: HashSet<String>,
    idrefs: Vec<(String, usize)>,
}

impl Validator<'_, '_> {
    fn problem(&mut self, offset: usize, message: impl Into<String>) {
        self.problems.push(Problem {
            message: message.into(),
            offset,
        });
    }

    /// The element's name exactly as written (with its prefix).
    fn raw_name(&self, n: Node) -> String {
        let rest = &self.text[n.range().start + 1..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .unwrap_or(rest.len());
        rest[..end].to_owned()
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

    fn element(&mut self, n: Node) {
        let name = self.raw_name(n);
        let at = n.range().start;
        let dtd = self.dtd;
        match dtd.elements.get(&name) {
            None => self.problem(at, format!("Element <{name}> isn't declared in the DTD.")),
            Some(ContentSpec::Any) => {}
            Some(ContentSpec::Empty) => {
                if let Some(c) = n.children().find(|c| c.is_element() || c.is_text()) {
                    self.problem(
                        c.range().start,
                        format!("Element <{name}> is declared EMPTY, so it can't have content."),
                    );
                }
            }
            Some(ContentSpec::Mixed(names)) => {
                for c in n.children().filter(Node::is_element) {
                    let cn = self.raw_name(c);
                    if !names.contains(&cn) {
                        let allowed = if names.is_empty() {
                            "it can only contain text".to_owned()
                        } else {
                            let list: Vec<String> =
                                names.iter().map(|n| format!("<{n}>")).collect();
                            format!("it allows text and {}", list.join(", "))
                        };
                        self.problem(
                            c.range().start,
                            format!("Element <{cn}> isn't allowed in <{name}>; {allowed}."),
                        );
                    }
                }
            }
            Some(ContentSpec::Children(cm)) => {
                if let Some(t) = n
                    .children()
                    .find(|c| c.is_text() && !c.text().unwrap_or("").chars().all(is_xml_space))
                {
                    let shown: String = t.text().unwrap_or("").trim().chars().take(30).collect();
                    self.problem(
                        t.range().start,
                        format!("Element <{name}> can't contain text (found \"{shown}\")."),
                    );
                }
                let kid_nodes: Vec<Node> = n.children().filter(Node::is_element).collect();
                let kids: Vec<String> = kid_nodes.iter().map(|k| self.raw_name(*k)).collect();
                let mut m = Matcher {
                    kids: &kids,
                    memo: HashMap::new(),
                    furthest: 0,
                    expected: Vec::new(),
                };
                if !m.ends(cm, 0).contains(&kids.len()) {
                    let list = |e: &[String]| {
                        if e.len() == 1 {
                            e[0].clone()
                        } else {
                            format!("one of {}", e.join(", "))
                        }
                    };
                    let pos = m.furthest.min(kids.len());
                    if pos < kids.len() {
                        let found = &kids[pos];
                        let msg = if m.expected.is_empty() {
                            format!("Element <{found}> isn't allowed here in <{name}>.")
                        } else {
                            format!("Expected {} but found <{found}>.", list(&m.expected))
                        };
                        self.problem(kid_nodes[pos].range().start, msg);
                    } else if m.expected.len() == 1 {
                        self.problem(
                            at,
                            format!(
                                "Element <{name}> is missing required child {}.",
                                m.expected[0]
                            ),
                        );
                    } else {
                        self.problem(
                            at,
                            format!(
                                "Element <{name}> is missing a required child: {}.",
                                list(&m.expected)
                            ),
                        );
                    }
                }
            }
        }
        self.attributes(n, &name);
    }

    fn attributes(&mut self, n: Node, name: &str) {
        let dtd = self.dtd;
        let defs: &[AttDef] = dtd.attlists.get(name).map_or(&[], Vec::as_slice);
        let mut present = Vec::new();
        for a in n.attributes() {
            let an = self.text[a.range_qname()].to_owned();
            let at = a.range().start;
            present.push(an.clone());
            let Some(def) = defs.iter().find(|d| d.name == an) else {
                self.problem(
                    at,
                    format!("Attribute \"{an}\" isn't declared for <{name}>."),
                );
                continue;
            };
            let raw = a.value();
            let value = if def.ty == AttType::Cdata {
                raw.to_owned()
            } else {
                raw.split(is_xml_space)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let shown = |v: &str| -> String { v.chars().take(40).collect() };
            let bad = |what: &str| {
                format!(
                    "Attribute \"{an}\" of <{name}>: \"{}\" isn't {what}.",
                    shown(&value)
                )
            };
            let problem = match &def.ty {
                AttType::Cdata => None,
                AttType::Id => {
                    if !is_name(&value) {
                        Some(bad("a valid ID (a name)"))
                    } else if !self.ids.insert(value.clone()) {
                        Some(format!("The ID \"{value}\" is used more than once."))
                    } else {
                        None
                    }
                }
                AttType::IdRef => {
                    if is_name(&value) {
                        self.idrefs.push((value.clone(), at));
                        None
                    } else {
                        Some(bad("a valid IDREF (a name)"))
                    }
                }
                AttType::IdRefs => {
                    if !value.is_empty() && value.split(' ').all(is_name) {
                        for v in value.split(' ') {
                            self.idrefs.push((v.to_owned(), at));
                        }
                        None
                    } else {
                        Some(bad("a list of IDREFs"))
                    }
                }
                AttType::Entity | AttType::Entities => {
                    let names: Vec<&str> = value.split(' ').filter(|s| !s.is_empty()).collect();
                    if names.is_empty()
                        || (def.ty == AttType::Entity && names.len() != 1)
                    {
                        Some(bad("an entity name"))
                    } else {
                        names
                            .iter()
                            .find(|e| !dtd.unparsed_entities.contains(**e))
                            .map(|e| format!("Attribute \"{an}\" of <{name}>: \"{e}\" isn't a declared unparsed entity."))
                    }
                }
                AttType::NmToken => (!is_nmtoken(&value)).then(|| bad("a valid name token")),
                AttType::NmTokens => {
                    (value.is_empty() || !value.split(' ').all(is_nmtoken))
                        .then(|| bad("a list of name tokens"))
                }
                AttType::Enum(list) | AttType::Notation(list) => {
                    (!list.contains(&value)).then(|| {
                        let options: Vec<String> = list.iter().map(|o| format!("\"{o}\"")).collect();
                        format!(
                            "Attribute \"{an}\" of <{name}>: \"{}\" isn't one of the allowed values: {}.",
                            shown(&value),
                            options.join(", ")
                        )
                    })
                }
            };
            if let Some(p) = problem {
                self.problem(at, p);
                continue;
            }
            if let AttDefault::Fixed(f) = &def.default {
                let f_norm = if def.ty == AttType::Cdata {
                    f.clone()
                } else {
                    f.split(is_xml_space)
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                if value != f_norm {
                    self.problem(
                        at,
                        format!("Attribute \"{an}\" of <{name}> must be \"{f}\" (it's #FIXED)."),
                    );
                }
            }
        }
        for d in defs {
            if d.default == AttDefault::Required
                && !present.contains(&d.name)
                && !d.name.starts_with("xmlns")
            {
                self.problem(
                    n.range().start,
                    format!(
                        "Element <{name}> is missing required attribute \"{}\".",
                        d.name
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DTD: &str = r#"
<!ENTITY % inline "b | i">
<!ELEMENT book (title, author+, (chapter | appendix)*, note?)>
<!ELEMENT title (#PCDATA)>
<!ELEMENT author (#PCDATA)>
<!ELEMENT chapter (#PCDATA | %inline;)*>
<!ELEMENT appendix ANY>
<!ELEMENT note EMPTY>
<!ELEMENT b (#PCDATA)>
<!ELEMENT i (#PCDATA)>
<!ATTLIST book
  id ID #REQUIRED
  lang NMTOKEN "en"
  kind (novel | poem | essay) #IMPLIED
  version CDATA #FIXED "1.0">
<!ATTLIST chapter ref IDREF #IMPLIED refs IDREFS #IMPLIED n NMTOKENS #IMPLIED>
<!-- a comment -->
<![IGNORE[ <!ELEMENT ignored (x)> ]]>
<![INCLUDE[ <!ATTLIST note pic ENTITY #IMPLIED> ]]>
<!ENTITY logo SYSTEM "logo.gif" NDATA gif>
"#;

    fn problems(doc: &str) -> Vec<String> {
        validate_dtd(doc, Some(DTD))
            .unwrap_or_else(|e| panic!("{e:?}"))
            .into_iter()
            .map(|p| p.message)
            .collect()
    }

    fn valid(doc: &str) {
        let p = problems(doc);
        assert!(p.is_empty(), "{doc}: {p:?}");
    }

    fn one(doc: &str, needle: &str) {
        let p = problems(doc);
        assert!(
            p.len() == 1 && p[0].contains(needle),
            "{doc}: expected one problem with {needle:?}, got {p:?}"
        );
    }

    #[test]
    fn valid_documents() {
        valid(r#"<book id="b1"><title>T</title><author>A</author></book>"#);
        valid(
            r#"<!DOCTYPE book SYSTEM "book.dtd"><book id="b1" kind="poem" version="1.0" lang="fr">
  <title>T</title><author>A</author><author>B</author>
  <chapter ref="b1">Text <b>bold</b> and <i>it</i></chapter>
  <appendix><b>anything</b> goes</appendix>
  <note pic="logo"/>
</book>"#,
        );
    }

    #[test]
    fn content_model_errors() {
        one(
            r#"<book id="b"><author>A</author></book>"#,
            "Expected <title> but found <author>.",
        );
        one(
            r#"<book id="b"><title>T</title></book>"#,
            "Element <book> is missing required child <author>.",
        );
        one(
            r#"<book id="b"><title>T</title><author>A</author><note/><chapter/></book>"#,
            "Element <chapter> isn't allowed here in <book>.",
        );
        one(
            r#"<book id="b"><title>T</title><author>A</author><title>X</title></book>"#,
            "Expected one of <author>, <chapter>, <appendix>, <note> but found <title>.",
        );
        one(
            r#"<book id="b">text<title>T</title><author>A</author></book>"#,
            "can't contain text",
        );
        one(
            r#"<book id="b"><title>T</title><author>A</author><chapter><title>x</title></chapter></book>"#,
            "Element <title> isn't allowed in <chapter>; it allows text and <b>, <i>.",
        );
        one(
            r#"<book id="b"><title>T<b>x</b></title><author>A</author></book>"#,
            "it can only contain text",
        );
        one(
            r#"<book id="b"><title>T</title><author>A</author><note>x</note></book>"#,
            "declared EMPTY",
        );
        one(
            r#"<book id="b"><title>T</title><author>A</author><appendix><zz/></appendix></book>"#,
            "Element <zz> isn't declared in the DTD.",
        );
    }

    #[test]
    fn attribute_errors() {
        one(
            r#"<book><title>T</title><author>A</author></book>"#,
            "Element <book> is missing required attribute \"id\".",
        );
        one(
            r#"<book id="b" color="red"><title>T</title><author>A</author></book>"#,
            "Attribute \"color\" isn't declared for <book>.",
        );
        one(
            r#"<book id="b" kind="play"><title>T</title><author>A</author></book>"#,
            "isn't one of the allowed values: \"novel\", \"poem\", \"essay\"",
        );
        one(
            r#"<book id="b" version="2.0"><title>T</title><author>A</author></book>"#,
            "must be \"1.0\" (it's #FIXED)",
        );
        one(
            r#"<book id="b" lang="a b"><title>T</title><author>A</author></book>"#,
            "valid name token",
        );
        one(
            r#"<book id="1b"><title>T</title><author>A</author></book>"#,
            "valid ID",
        );
        one(
            r#"<book id="b"><title>T</title><author>A</author><chapter ref="nope"/></book>"#,
            "No element has the ID \"nope\"",
        );
        one(
            r#"<book id="b"><title>T</title><author>A</author><chapter refs="b zz"/></book>"#,
            "\"zz\"",
        );
        one(
            r#"<book id="b"><title>T</title><author>A</author><note pic="nologo"/></book>"#,
            "isn't a declared unparsed entity",
        );
        valid(r#"<book id="b"><title>T</title><author>A</author><chapter n=" x  y "/></book>"#);
    }

    #[test]
    fn internal_subset_and_doctype() {
        let doc = r#"<?xml version="1.0"?>
<!-- lead -->
<!DOCTYPE list [
  <!ELEMENT list (item*)>
  <!ELEMENT item (#PCDATA)>
  <!ATTLIST item id ID #REQUIRED>
]>
<list><item id="a">x</item><item id="a">y</item></list>"#;
        let p = validate_dtd(doc, None).unwrap();
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(
            p[0].message
                .contains("The ID \"a\" is used more than once.")
        );
        assert!(doc[p[0].offset..].starts_with("id=\"a\">y"));
        let wrong_root = r#"<!DOCTYPE list [<!ELEMENT list EMPTY><!ELEMENT other EMPTY>]><other/>"#;
        let p = validate_dtd(wrong_root, None).unwrap();
        assert!(
            p[0].message
                .contains("doesn't match the DOCTYPE name <list>"),
            "{p:?}"
        );
        assert_eq!(
            doctype_system_id(r#"<!DOCTYPE a PUBLIC "-//X//Y" "a.dtd"><a/>"#),
            Some("a.dtd".into())
        );
        assert_eq!(
            doctype_system_id(r#"<?xml version="1.0"?><!DOCTYPE a SYSTEM 'b.dtd' [ ]><a/>"#),
            Some("b.dtd".into())
        );
        assert_eq!(doctype_system_id("<a/>"), None);
        // Internal declarations take precedence over the external DTD.
        let doc = r#"<!DOCTYPE note [<!ELEMENT note (#PCDATA)>]><note>hi</note>"#;
        assert!(
            validate_dtd(doc, Some("<!ELEMENT note EMPTY>"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn dtd_errors() {
        let e = validate_dtd("<a/>", None).unwrap_err();
        assert!(e.message.contains("no DOCTYPE"));
        let e = validate_dtd(r#"<!DOCTYPE a SYSTEM "a.dtd"><a/>"#, None).unwrap_err();
        assert!(
            e.message.contains("Load its external DTD (\"a.dtd\")"),
            "{e:?}"
        );
        let e = validate_dtd("<a/>", Some("<!ELEMENT a (b,c|d)>")).unwrap_err();
        assert!(e.message.contains("can't mix"), "{e:?}");
        let e = validate_dtd("<a/>", Some("<!ELEMENT a (#PCDATA|b)>")).unwrap_err();
        assert!(e.message.contains(")*"), "{e:?}");
        let e = validate_dtd("<a/>", Some("\n<!ATTLIST a x BOGUS #IMPLIED>")).unwrap_err();
        assert!(
            e.message.contains("unknown type") && e.message.contains("DTD line 2"),
            "{e:?}"
        );
        let e = validate_dtd("<a/>", Some("<!ELEMENT a %missing;>")).unwrap_err();
        assert!(e.message.contains("%missing;"), "{e:?}");
        let e = validate_dtd("<a/>", Some("<!ELEMENT a EMPTY")).unwrap_err();
        assert!(e.message.contains("never closed"), "{e:?}");
        let e = validate_dtd(r#"<!DOCTYPE a [<!ELEMENT a FOO>]><a/>"#, None).unwrap_err();
        assert!(e.offset.is_some(), "{e:?}");
        let e = validate_dtd("<a>", Some("<!ELEMENT a EMPTY>")).unwrap_err();
        assert!(e.message.contains("well-formed"), "{e:?}");
    }

    #[test]
    fn nested_repetition() {
        let dtd =
            "<!ELEMENT r ((a, b?)+, c*)> <!ELEMENT a EMPTY> <!ELEMENT b EMPTY> <!ELEMENT c EMPTY>";
        let ok = |doc: &str| validate_dtd(doc, Some(dtd)).unwrap().is_empty();
        assert!(ok("<r><a/></r>"));
        assert!(ok("<r><a/><b/><a/><a/><b/><c/><c/></r>"));
        assert!(!ok("<r><b/></r>"));
        assert!(!ok("<r><a/><b/><b/></r>"));
        assert!(!ok("<r><a/><c/><a/></r>"));
    }
}
