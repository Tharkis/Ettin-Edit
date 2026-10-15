//! XSLT 1.0 transforms, written from the W3C XSLT 1.0 recommendation.
//!
//! The stylesheet is compiled once into instructions (with its XPath
//! expressions compiled by `xpath::XPath`), then run against the document,
//! building a result tree that is serialized as XML, HTML or text.
//!
//! Not supported: `xsl:import`, `xsl:include`, `xsl:namespace-alias`,
//! `document()` and extension elements. Result tree fragments can be copied
//! and used as strings, but (as in XSLT 1.0) not navigated.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use roxmltree::{Document, Node, NodeType};

use super::xpath::{
    self, Engine, Env, FragAttr, FragElement, FragNode, Fragment, Value, XNode, XPath,
};
use super::{ToolError, line_col, xml};

const XSL_NS: &str = "http://www.w3.org/1999/XSL/Transform";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
/// How deeply templates may call each other before we call it runaway recursion.
const MAX_DEPTH: usize = 3000;

const XSLT_FUNCTIONS: &[&str] = &[
    "current",
    "key",
    "generate-id",
    "format-number",
    "system-property",
    "element-available",
    "function-available",
    "document",
    "unparsed-entity-uri",
];

const XSL_INSTRUCTIONS: &[&str] = &[
    "apply-templates",
    "apply-imports",
    "attribute",
    "call-template",
    "choose",
    "comment",
    "copy",
    "copy-of",
    "element",
    "fallback",
    "for-each",
    "if",
    "message",
    "number",
    "processing-instruction",
    "text",
    "value-of",
    "variable",
];

/// The transform's output and the text of any `xsl:message`s.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transformed {
    pub output: String,
    pub messages: Vec<String>,
}

/// Transforms `doc_text` with the XSLT 1.0 stylesheet `xslt_text`.
pub fn transform(doc_text: &str, xslt_text: &str) -> Result<String, ToolError> {
    transform_with_messages(doc_text, xslt_text).map(|t| t.output)
}

/// Like `transform`, also returning what `xsl:message` said.
pub fn transform_with_messages(doc_text: &str, xslt_text: &str) -> Result<Transformed, ToolError> {
    // Recursive stylesheets can nest deeply; give them a roomy stack of their own.
    std::thread::scope(|s| {
        match std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn_scoped(s, || run(doc_text, xslt_text))
        {
            Ok(handle) => handle
                .join()
                .unwrap_or_else(|_| Err(ToolError::new("The transform failed unexpectedly."))),
            Err(_) => run(doc_text, xslt_text),
        }
    })
}

/// Adds "(stylesheet line 3, column 7)" to an error that has a place.
fn placed(e: ToolError, text: &str, what: &str) -> ToolError {
    match e.offset {
        Some(o) => {
            let (line, col) = line_col(text, o);
            ToolError::at(
                format!("{} ({what} line {line}, column {col})", e.message),
                o,
            )
        }
        None => e,
    }
}

fn run(doc_text: &str, xslt_text: &str) -> Result<Transformed, ToolError> {
    let ss_doc = xml::parse_strict(xslt_text).map_err(|e| {
        placed(
            ToolError {
                message: format!("The stylesheet isn't well-formed XML: {}", e.message),
                offset: e.offset,
            },
            xslt_text,
            "stylesheet",
        )
    })?;
    let ss = Compiler::new(xslt_text)
        .compile(&ss_doc)
        .map_err(|e| placed(e, xslt_text, "stylesheet"))?;
    let doc = xml::parse_strict(doc_text).map_err(|e| {
        placed(
            ToolError {
                message: format!("The XML document isn't well-formed: {}", e.message),
                offset: e.offset,
            },
            doc_text,
            "document",
        )
    })?;
    let skip = stripped_text_nodes(&doc, &ss);
    let exec = Exec {
        ss: &ss,
        eng: Engine::new(&doc, skip),
        doc: &doc,
        vars: RefCell::new(Vec::new()),
        base: Cell::new(0),
        globals: RefCell::new(
            ss.globals
                .iter()
                .enumerate()
                .map(|(i, g)| (g.name.clone(), Global::Pending(i)))
                .collect(),
        ),
        current: Cell::new(None),
        keys: RefCell::new(HashMap::new()),
        building_keys: RefCell::new(HashSet::new()),
        messages: RefCell::new(Vec::new()),
        depth: Cell::new(0),
        pos: Cell::new(0),
    };
    let nodes = exec.run().map_err(|e| placed(e, xslt_text, "stylesheet"))?;
    Ok(Transformed {
        output: serialize(&nodes, &ss.output),
        messages: exec.messages.take(),
    })
}

// ----------------------------------------------------------------------
// The compiled stylesheet

type Body = Vec<(usize, Instr)>;
type Scope = Rc<Vec<(Option<String>, String)>>;

#[derive(Debug)]
enum AvtPart {
    Lit(String),
    Expr(XPath),
}

#[derive(Debug)]
struct Avt(Vec<AvtPart>);

#[derive(Debug)]
enum VarValue {
    Select(XPath),
    Body(Body),
    Empty,
}

#[derive(Debug)]
struct VarDecl {
    name: String,
    value: VarValue,
    pos: usize,
}

#[derive(Debug)]
struct Sort {
    select: XPath,
    order: Option<Avt>,
    data_type: Option<Avt>,
    case_order: Option<Avt>,
}

#[derive(Debug)]
struct LiteralElem {
    prefix: Option<String>,
    local: String,
    ns: Option<String>,
    ns_decls: Vec<(Option<String>, String)>,
    attrs: Vec<(Option<String>, String, Option<String>, Avt)>,
    sets: Vec<String>,
    body: Body,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Level {
    Single,
    Multiple,
    Any,
}

#[derive(Debug)]
struct NumberInstr {
    level: Level,
    count: Option<XPath>,
    from: Option<XPath>,
    value: Option<XPath>,
    format: Avt,
    grouping_separator: Option<Avt>,
    grouping_size: Option<Avt>,
}

#[derive(Debug)]
enum Instr {
    Text {
        text: String,
        raw: bool,
    },
    Literal(Box<LiteralElem>),
    ValueOf {
        select: XPath,
        raw: bool,
    },
    ApplyTemplates {
        select: Option<XPath>,
        mode: Option<String>,
        sorts: Vec<Sort>,
        params: Vec<VarDecl>,
    },
    ApplyImports,
    CallTemplate {
        name: String,
        params: Vec<VarDecl>,
    },
    ForEach {
        select: XPath,
        sorts: Vec<Sort>,
        body: Body,
    },
    If {
        test: XPath,
        body: Body,
    },
    Choose {
        whens: Vec<(XPath, Body)>,
        otherwise: Option<Body>,
    },
    Variable(VarDecl),
    Copy {
        sets: Vec<String>,
        body: Body,
    },
    CopyOf {
        select: XPath,
    },
    Element {
        name: Avt,
        namespace: Option<Avt>,
        scope: Scope,
        sets: Vec<String>,
        body: Body,
    },
    Attribute {
        name: Avt,
        namespace: Option<Avt>,
        scope: Scope,
        body: Body,
    },
    Comment(Body),
    Pi {
        name: Avt,
        body: Body,
    },
    Number(Box<NumberInstr>),
    Message {
        body: Body,
        terminate: bool,
    },
}

#[derive(Debug)]
struct Template {
    params: Vec<VarDecl>,
    body: Body,
}

#[derive(Debug)]
struct Rule {
    pattern: XPath,
    priority: f64,
    order: usize,
    template: usize,
}

#[derive(Debug)]
struct Key {
    name: String,
    pattern: XPath,
    use_: XPath,
}

#[derive(Debug)]
struct AttrSet {
    uses: Vec<String>,
    body: Body,
}

/// An element name test from `xsl:strip-space` / `xsl:preserve-space`.
#[derive(Debug)]
enum NameTest {
    Any,
    Ns(String),
    Name(Option<String>, String),
}

impl NameTest {
    /// How specific a match is (higher wins), or `None` if it doesn't match.
    fn score(&self, ns: Option<&str>, local: &str) -> Option<i32> {
        match self {
            NameTest::Any => Some(0),
            NameTest::Ns(u) => (ns == Some(u.as_str())).then_some(1),
            NameTest::Name(u, l) => (u.as_deref() == ns && l == local).then_some(2),
        }
    }
}

#[derive(Debug, Default)]
struct Output {
    method: Option<String>,
    indent: Option<bool>,
    omit_declaration: bool,
    encoding: Option<String>,
    standalone: Option<String>,
    doctype_public: Option<String>,
    doctype_system: Option<String>,
    cdata: Vec<(Option<String>, String)>,
}

#[derive(Clone, Debug)]
struct DecimalFormat {
    decimal: char,
    grouping: char,
    percent: char,
    per_mille: char,
    zero: char,
    digit: char,
    pattern_separator: char,
    minus: char,
    infinity: String,
    nan: String,
}

impl Default for DecimalFormat {
    fn default() -> Self {
        Self {
            decimal: '.',
            grouping: ',',
            percent: '%',
            per_mille: '\u{2030}',
            zero: '0',
            digit: '#',
            pattern_separator: ';',
            minus: '-',
            infinity: "Infinity".into(),
            nan: "NaN".into(),
        }
    }
}

#[derive(Debug, Default)]
struct Stylesheet {
    templates: Vec<Template>,
    /// Rules per mode (`""` is the default mode), best first.
    rules: HashMap<String, Vec<Rule>>,
    named: HashMap<String, usize>,
    globals: Vec<VarDecl>,
    keys: Vec<Key>,
    attr_sets: HashMap<String, AttrSet>,
    output: Output,
    strip: Vec<(NameTest, usize)>,
    preserve: Vec<(NameTest, usize)>,
    decimal_formats: HashMap<String, DecimalFormat>,
}

// ----------------------------------------------------------------------
// Compiling

fn is_xsl(n: Node, local: &str) -> bool {
    n.is_element() && n.tag_name().namespace() == Some(XSL_NS) && n.tag_name().name() == local
}

fn xsl_name<'a>(n: Node<'a, '_>) -> Option<&'a str> {
    (n.is_element() && n.tag_name().namespace() == Some(XSL_NS)).then(|| n.tag_name().name())
}

fn lookup_prefix(n: Node, prefix: &str) -> Option<String> {
    if prefix == "xml" {
        return Some(XML_NS.to_owned());
    }
    n.lookup_namespace_uri(Some(prefix)).map(str::to_owned)
}

fn in_scope(n: Node) -> Vec<(Option<String>, String)> {
    n.namespaces()
        .filter(|ns| ns.name() != Some("xml"))
        .map(|ns| (ns.name().map(str::to_owned), ns.uri().to_owned()))
        .collect()
}

fn split_qname(q: &str) -> (Option<&str>, &str) {
    match q.split_once(':') {
        Some((p, l)) => (Some(p), l),
        None => (None, q),
    }
}

fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

fn is_ncname(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '\u{b7}'))
}

fn is_qname(s: &str) -> bool {
    match s.split_once(':') {
        Some((p, l)) => is_ncname(p) && is_ncname(l),
        None => is_ncname(s),
    }
}

/// The prefix of a name as written in the source text, starting at `start`.
fn written_prefix(text: &str, start: usize) -> Option<String> {
    let rest = text.get(start..)?;
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '/' | '>' | '='))
        .unwrap_or(rest.len());
    rest[..end].split_once(':').map(|(p, _)| p.to_owned())
}

struct Compiler<'t> {
    text: &'t str,
    /// Namespace URIs excluded from literal result elements.
    exclude: Vec<String>,
    order: usize,
}

impl<'t> Compiler<'t> {
    fn new(text: &'t str) -> Self {
        Self {
            text,
            exclude: Vec::new(),
            order: 0,
        }
    }

    fn error(&self, n: Node, message: impl Into<String>) -> ToolError {
        ToolError::at(message, n.range().start)
    }

    fn label(n: Node) -> String {
        match xsl_name(n) {
            Some(l) => format!("xsl:{l}"),
            None => format!("<{}>", n.tag_name().name()),
        }
    }

    fn required<'a>(&self, n: Node<'a, '_>, attr: &str) -> Result<&'a str, ToolError> {
        n.attribute(attr)
            .ok_or_else(|| self.error(n, format!("{} needs a {attr} attribute.", Self::label(n))))
    }

    fn compile_xpath(
        &self,
        n: Node,
        attr: &str,
        src: &str,
        pattern: bool,
    ) -> Result<XPath, ToolError> {
        let resolve = |p: &str| lookup_prefix(n, p);
        let r = if pattern {
            XPath::compile_pattern(src, &resolve, XSLT_FUNCTIONS)
        } else {
            XPath::compile(src, &resolve, XSLT_FUNCTIONS)
        };
        r.map_err(|e| {
            let base = n
                .attribute_node(attr)
                .map_or(n.range().start, |a| a.range_value().start);
            ToolError::at(
                format!("{} {attr}: {}", Self::label(n), e.message),
                base + e.offset.unwrap_or(0).min(src.len()),
            )
        })
    }

    fn xpath(&self, n: Node, attr: &str) -> Result<XPath, ToolError> {
        let src = self.required(n, attr)?;
        self.compile_xpath(n, attr, src, false)
    }

    fn opt_xpath(&self, n: Node, attr: &str) -> Result<Option<XPath>, ToolError> {
        n.attribute(attr)
            .map(|src| self.compile_xpath(n, attr, src, false))
            .transpose()
    }

    fn opt_pattern(&self, n: Node, attr: &str) -> Result<Option<XPath>, ToolError> {
        n.attribute(attr)
            .map(|src| self.compile_xpath(n, attr, src, true))
            .transpose()
    }

    fn opt_avt(&self, n: Node, attr: &str) -> Result<Option<Avt>, ToolError> {
        let Some(a) = n.attribute_node(attr) else {
            return Ok(None);
        };
        self.avt_of(n, a.value(), a.range_value().start).map(Some)
    }

    fn avt_of(&self, n: Node, src: &str, base: usize) -> Result<Avt, ToolError> {
        let resolve = |p: &str| lookup_prefix(n, p);
        let mut parts = Vec::new();
        let mut lit = String::new();
        let chars: Vec<(usize, char)> = src.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            let (at, c) = chars[i];
            let next = chars.get(i + 1).map(|&(_, c)| c);
            match c {
                '{' if next == Some('{') => {
                    lit.push('{');
                    i += 2;
                }
                '{' => {
                    // Find the closing brace, skipping string literals.
                    let mut j = i + 1;
                    let mut quote = None;
                    while j < chars.len() {
                        let d = chars[j].1;
                        match quote {
                            Some(q) if d == q => quote = None,
                            Some(_) => {}
                            None if d == '"' || d == '\'' => quote = Some(d),
                            None if d == '}' => break,
                            None => {}
                        }
                        j += 1;
                    }
                    if j >= chars.len() {
                        return Err(ToolError::at(
                            "This '{' in an attribute value template is never closed.",
                            base + at,
                        ));
                    }
                    let expr_src = &src[at + 1..chars[j].0];
                    let x = XPath::compile(expr_src, &resolve, XSLT_FUNCTIONS).map_err(|e| {
                        ToolError::at(e.message, base + at + 1 + e.offset.unwrap_or(0))
                    })?;
                    if !lit.is_empty() {
                        parts.push(AvtPart::Lit(std::mem::take(&mut lit)));
                    }
                    parts.push(AvtPart::Expr(x));
                    i = j + 1;
                }
                '}' if next == Some('}') => {
                    lit.push('}');
                    i += 2;
                }
                '}' => {
                    return Err(ToolError::at(
                        "A '}' in an attribute value template must be doubled ('}}').",
                        base + at,
                    ));
                }
                c => {
                    lit.push(c);
                    i += 1;
                }
            }
        }
        if !lit.is_empty() {
            parts.push(AvtPart::Lit(lit));
        }
        Ok(Avt(parts))
    }

    fn names(&self, n: Node, attr: &str) -> Vec<String> {
        n.attribute(attr)
            .map(|s| s.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default()
    }

    fn excluded_uris(&self, n: Node, value: &str) -> Vec<String> {
        value
            .split_whitespace()
            .filter_map(|p| {
                if p == "#default" {
                    n.lookup_namespace_uri(None).map(str::to_owned)
                } else {
                    lookup_prefix(n, p)
                }
            })
            .collect()
    }

    fn compile(mut self, doc: &Document) -> Result<Stylesheet, ToolError> {
        let root = doc.root_element();
        let mut ss = Stylesheet::default();
        if !is_xsl(root, "stylesheet") && !is_xsl(root, "transform") {
            if root.attribute((XSL_NS, "version")).is_none() {
                return Err(self.error(
                    root,
                    "This isn't an XSLT stylesheet: the root element should be xsl:stylesheet or xsl:transform.",
                ));
            }
            // A simplified stylesheet: the root is a template for "/".
            let body = vec![(root.range().start, self.literal(root)?)];
            ss.templates.push(Template {
                params: Vec::new(),
                body,
            });
            let pattern = XPath::compile_pattern("/", &|_| None, &[]).expect("root pattern");
            ss.rules.entry(String::new()).or_default().push(Rule {
                priority: pattern.default_priority(),
                pattern,
                order: 0,
                template: 0,
            });
            return Ok(ss);
        }
        for attr in ["exclude-result-prefixes", "extension-element-prefixes"] {
            if let Some(v) = root.attribute(attr) {
                let uris = self.excluded_uris(root, v);
                self.exclude.extend(uris);
            }
        }
        for child in root.children().filter(Node::is_element) {
            let Some(name) = xsl_name(child) else {
                continue; // Top-level elements in other namespaces are data.
            };
            self.order += 1;
            match name {
                "template" => self.template(child, &mut ss)?,
                "variable" | "param" => {
                    let decl = self.var_decl(child)?;
                    ss.globals.push(decl);
                }
                "output" => self.output(child, &mut ss.output)?,
                "strip-space" | "preserve-space" => {
                    let elements = self.required(child, "elements")?;
                    for t in elements.split_whitespace() {
                        let test = if t == "*" {
                            NameTest::Any
                        } else if let Some(p) = t.strip_suffix(":*") {
                            NameTest::Ns(lookup_prefix(child, p).ok_or_else(|| {
                                self.error(child, format!("The prefix '{p}' isn't declared."))
                            })?)
                        } else {
                            let (p, l) = split_qname(t);
                            let ns = match p {
                                Some(p) => Some(lookup_prefix(child, p).ok_or_else(|| {
                                    self.error(child, format!("The prefix '{p}' isn't declared."))
                                })?),
                                None => None,
                            };
                            NameTest::Name(ns, l.to_owned())
                        };
                        let list = if name == "strip-space" {
                            &mut ss.strip
                        } else {
                            &mut ss.preserve
                        };
                        list.push((test, self.order));
                    }
                }
                "key" => {
                    let key = Key {
                        name: self.required(child, "name")?.to_owned(),
                        pattern: {
                            self.required(child, "match")?;
                            self.opt_pattern(child, "match")?.expect("match")
                        },
                        use_: self.xpath(child, "use")?,
                    };
                    ss.keys.push(key);
                }
                "attribute-set" => {
                    let set_name = self.required(child, "name")?.to_owned();
                    for c in child.children().filter(Node::is_element) {
                        if !is_xsl(c, "attribute") {
                            return Err(
                                self.error(c, "xsl:attribute-set can only contain xsl:attribute.")
                            );
                        }
                    }
                    let set = AttrSet {
                        uses: self.names(child, "use-attribute-sets"),
                        body: self.body(child)?,
                    };
                    match ss.attr_sets.get_mut(&set_name) {
                        // Sets with the same name merge.
                        Some(existing) => {
                            existing.uses.extend(set.uses);
                            existing.body.extend(set.body);
                        }
                        None => {
                            ss.attr_sets.insert(set_name, set);
                        }
                    }
                }
                "decimal-format" => {
                    let mut f = DecimalFormat::default();
                    let ch = |attr: &str, slot: &mut char| -> Result<(), ToolError> {
                        if let Some(v) = child.attribute(attr) {
                            let mut cs = v.chars();
                            match (cs.next(), cs.next()) {
                                (Some(c), None) => *slot = c,
                                _ => {
                                    return Err(self.error(
                                        child,
                                        format!("xsl:decimal-format {attr} must be one character."),
                                    ));
                                }
                            }
                        }
                        Ok(())
                    };
                    ch("decimal-separator", &mut f.decimal)?;
                    ch("grouping-separator", &mut f.grouping)?;
                    ch("percent", &mut f.percent)?;
                    ch("per-mille", &mut f.per_mille)?;
                    ch("zero-digit", &mut f.zero)?;
                    ch("digit", &mut f.digit)?;
                    ch("pattern-separator", &mut f.pattern_separator)?;
                    ch("minus-sign", &mut f.minus)?;
                    if let Some(v) = child.attribute("infinity") {
                        f.infinity = v.to_owned();
                    }
                    if let Some(v) = child.attribute("NaN") {
                        f.nan = v.to_owned();
                    }
                    let key = child.attribute("name").unwrap_or_default().to_owned();
                    ss.decimal_formats.insert(key, f);
                }
                "import" | "include" => {
                    return Err(self.error(
                        child,
                        format!(
                            "xsl:{name} isn't supported; put the templates in this stylesheet."
                        ),
                    ));
                }
                "namespace-alias" => {
                    return Err(self.error(child, "xsl:namespace-alias isn't supported."));
                }
                other => {
                    return Err(self.error(
                        child,
                        format!("xsl:{other} isn't allowed at the top level of a stylesheet."),
                    ));
                }
            }
        }
        for rules in ss.rules.values_mut() {
            // Best first: higher priority, then later in the stylesheet.
            rules.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(Ordering::Equal)
                    .then(b.order.cmp(&a.order))
            });
        }
        Ok(ss)
    }

    fn output(&self, n: Node, o: &mut Output) -> Result<(), ToolError> {
        if let Some(m) = n.attribute("method") {
            if !matches!(m, "xml" | "html" | "text") {
                return Err(self.error(
                    n,
                    format!("Output method '{m}' isn't supported; use xml, html or text."),
                ));
            }
            o.method = Some(m.to_owned());
        }
        let yes = |a: &str| n.attribute(a).map(|v| v.trim() == "yes");
        if let Some(v) = yes("indent") {
            o.indent = Some(v);
        }
        if let Some(v) = yes("omit-xml-declaration") {
            o.omit_declaration = v;
        }
        if let Some(v) = n.attribute("encoding") {
            o.encoding = Some(v.to_owned());
        }
        if let Some(v) = n.attribute("standalone") {
            o.standalone = Some(v.to_owned());
        }
        if let Some(v) = n.attribute("doctype-public") {
            o.doctype_public = Some(v.to_owned());
        }
        if let Some(v) = n.attribute("doctype-system") {
            o.doctype_system = Some(v.to_owned());
        }
        for q in self.names(n, "cdata-section-elements") {
            let (p, l) = split_qname(&q);
            let ns = match p {
                Some(p) => lookup_prefix(n, p),
                None => n.lookup_namespace_uri(None).map(str::to_owned),
            };
            o.cdata.push((ns, l.to_owned()));
        }
        Ok(())
    }

    fn template(&mut self, n: Node, ss: &mut Stylesheet) -> Result<(), ToolError> {
        let pattern = self.opt_pattern(n, "match")?;
        let name = n.attribute("name");
        if pattern.is_none() && name.is_none() {
            return Err(self.error(n, "xsl:template needs a match or name attribute."));
        }
        let priority = match n.attribute("priority") {
            Some(p) => Some(
                p.trim()
                    .parse::<f64>()
                    .map_err(|_| self.error(n, format!("The priority '{p}' isn't a number.")))?,
            ),
            None => None,
        };
        let mut params = Vec::new();
        for c in n.children().filter(Node::is_element) {
            if is_xsl(c, "param") {
                params.push(self.var_decl(c)?);
            } else {
                break;
            }
        }
        let body = self.body_after(n, Some("param"))?;
        let index = ss.templates.len();
        ss.templates.push(Template { params, body });
        if let Some(name) = name {
            ss.named.insert(name.to_owned(), index);
        }
        if let Some(pattern) = pattern {
            let mode = n.attribute("mode").unwrap_or_default().to_owned();
            for alt in pattern.alternatives() {
                let rule = Rule {
                    priority: priority.unwrap_or_else(|| alt.default_priority()),
                    pattern: alt,
                    order: self.order,
                    template: index,
                };
                ss.rules.entry(mode.clone()).or_default().push(rule);
            }
        }
        Ok(())
    }

    fn var_decl(&self, n: Node) -> Result<VarDecl, ToolError> {
        let name = self.required(n, "name")?.to_owned();
        let value = if n.has_attribute("select") {
            VarValue::Select(self.xpath(n, "select")?)
        } else if n.has_children() {
            VarValue::Body(self.body(n)?)
        } else {
            VarValue::Empty
        };
        Ok(VarDecl {
            name,
            value,
            pos: n.range().start,
        })
    }

    fn preserves_space(n: Node) -> bool {
        n.ancestors()
            .find_map(|a| a.attribute((XML_NS, "space")))
            .is_some_and(|v| v == "preserve")
    }

    /// Compiles an element's children as a sequence of instructions,
    /// skipping leading `xsl:{skip}` elements (handled by the caller).
    fn body_after(&self, n: Node, skip: Option<&str>) -> Result<Body, ToolError> {
        let mut out = Vec::new();
        let mut leading = skip.is_some();
        for c in n.children() {
            match c.node_type() {
                NodeType::Text => {
                    let t = c.text().unwrap_or_default();
                    if t.chars().all(is_xml_space) && !Self::preserves_space(n) {
                        continue;
                    }
                    out.push((
                        c.range().start,
                        Instr::Text {
                            text: t.to_owned(),
                            raw: false,
                        },
                    ));
                }
                NodeType::Element => {
                    if leading && skip.is_some_and(|s| is_xsl(c, s)) {
                        continue;
                    }
                    leading = false;
                    out.push((c.range().start, self.instruction(c)?));
                }
                _ => {}
            }
        }
        Ok(out)
    }

    fn body(&self, n: Node) -> Result<Body, ToolError> {
        self.body_after(n, None)
    }

    fn sorts_and_params(&self, n: Node) -> Result<(Vec<Sort>, Vec<VarDecl>), ToolError> {
        let mut sorts = Vec::new();
        let mut params = Vec::new();
        for c in n.children().filter(Node::is_element) {
            if is_xsl(c, "sort") {
                sorts.push(self.sort(c)?);
            } else if is_xsl(c, "with-param") {
                params.push(self.var_decl(c)?);
            } else {
                return Err(self.error(
                    c,
                    format!(
                        "{} can only contain xsl:sort and xsl:with-param.",
                        Self::label(n)
                    ),
                ));
            }
        }
        Ok((sorts, params))
    }

    fn sort(&self, c: Node) -> Result<Sort, ToolError> {
        Ok(Sort {
            select: match c.attribute("select") {
                Some(s) => self.compile_xpath(c, "select", s, false)?,
                None => self.compile_xpath(c, "select", ".", false)?,
            },
            order: self.opt_avt(c, "order")?,
            data_type: self.opt_avt(c, "data-type")?,
            case_order: self.opt_avt(c, "case-order")?,
        })
    }

    fn instruction(&self, n: Node) -> Result<Instr, ToolError> {
        let Some(name) = xsl_name(n) else {
            return self.literal(n);
        };
        let raw = || n.attribute("disable-output-escaping") == Some("yes");
        Ok(match name {
            "apply-templates" => {
                let (sorts, params) = self.sorts_and_params(n)?;
                Instr::ApplyTemplates {
                    select: self.opt_xpath(n, "select")?,
                    mode: n.attribute("mode").map(str::to_owned),
                    sorts,
                    params,
                }
            }
            "apply-imports" => Instr::ApplyImports,
            "call-template" => {
                let (sorts, params) = self.sorts_and_params(n)?;
                if !sorts.is_empty() {
                    return Err(self.error(n, "xsl:call-template can't contain xsl:sort."));
                }
                Instr::CallTemplate {
                    name: self.required(n, "name")?.to_owned(),
                    params,
                }
            }
            "value-of" => Instr::ValueOf {
                select: self.xpath(n, "select")?,
                raw: raw(),
            },
            "for-each" => {
                let mut sorts = Vec::new();
                for c in n.children().filter(Node::is_element) {
                    if is_xsl(c, "sort") {
                        sorts.push(self.sort(c)?);
                    } else {
                        break;
                    }
                }
                let body = self.body_after(n, Some("sort"))?;
                Instr::ForEach {
                    select: self.xpath(n, "select")?,
                    sorts,
                    body,
                }
            }
            "if" => Instr::If {
                test: self.xpath(n, "test")?,
                body: self.body(n)?,
            },
            "choose" => {
                let mut whens = Vec::new();
                let mut otherwise = None;
                for c in n.children().filter(Node::is_element) {
                    if is_xsl(c, "when") && otherwise.is_none() {
                        whens.push((self.xpath(c, "test")?, self.body(c)?));
                    } else if is_xsl(c, "otherwise") && otherwise.is_none() {
                        otherwise = Some(self.body(c)?);
                    } else {
                        return Err(self.error(
                            c,
                            "xsl:choose can only contain xsl:when elements and one final xsl:otherwise.",
                        ));
                    }
                }
                if whens.is_empty() {
                    return Err(self.error(n, "xsl:choose needs at least one xsl:when."));
                }
                Instr::Choose { whens, otherwise }
            }
            "variable" | "param" => Instr::Variable(self.var_decl(n)?),
            "text" => Instr::Text {
                text: n
                    .children()
                    .filter(Node::is_text)
                    .filter_map(|t| t.text())
                    .collect(),
                raw: raw(),
            },
            "copy" => Instr::Copy {
                sets: self.names(n, "use-attribute-sets"),
                body: self.body(n)?,
            },
            "copy-of" => Instr::CopyOf {
                select: self.xpath(n, "select")?,
            },
            "element" => {
                self.required(n, "name")?;
                Instr::Element {
                    name: self.opt_avt(n, "name")?.expect("name"),
                    namespace: self.opt_avt(n, "namespace")?,
                    scope: Rc::new(in_scope(n)),
                    sets: self.names(n, "use-attribute-sets"),
                    body: self.body(n)?,
                }
            }
            "attribute" => {
                self.required(n, "name")?;
                Instr::Attribute {
                    name: self.opt_avt(n, "name")?.expect("name"),
                    namespace: self.opt_avt(n, "namespace")?,
                    scope: Rc::new(in_scope(n)),
                    body: self.body(n)?,
                }
            }
            "comment" => Instr::Comment(self.body(n)?),
            "processing-instruction" => {
                self.required(n, "name")?;
                Instr::Pi {
                    name: self.opt_avt(n, "name")?.expect("name"),
                    body: self.body(n)?,
                }
            }
            "number" => {
                let level = match n.attribute("level").unwrap_or("single") {
                    "single" => Level::Single,
                    "multiple" => Level::Multiple,
                    "any" => Level::Any,
                    other => {
                        return Err(self.error(
                            n,
                            format!(
                                "xsl:number level '{other}' should be single, multiple or any."
                            ),
                        ));
                    }
                };
                let format = match self.opt_avt(n, "format")? {
                    Some(f) => f,
                    None => Avt(vec![AvtPart::Lit("1".into())]),
                };
                Instr::Number(Box::new(NumberInstr {
                    level,
                    count: self.opt_pattern(n, "count")?,
                    from: self.opt_pattern(n, "from")?,
                    value: self.opt_xpath(n, "value")?,
                    format,
                    grouping_separator: self.opt_avt(n, "grouping-separator")?,
                    grouping_size: self.opt_avt(n, "grouping-size")?,
                }))
            }
            "message" => Instr::Message {
                body: self.body(n)?,
                terminate: n.attribute("terminate") == Some("yes"),
            },
            "fallback" => Instr::Text {
                text: String::new(),
                raw: false,
            },
            "sort" | "with-param" | "when" | "otherwise" => {
                return Err(self.error(n, format!("xsl:{name} isn't allowed here.")));
            }
            other => {
                return Err(self.error(
                    n,
                    format!("xsl:{other} isn't a supported XSLT 1.0 instruction."),
                ));
            }
        })
    }

    fn literal(&self, n: Node) -> Result<Instr, ToolError> {
        let mut excluded = self.exclude.clone();
        excluded.push(XSL_NS.to_owned());
        for a in n
            .ancestors()
            .filter(|a| a.is_element() && xsl_name(*a).is_none())
        {
            for attr in ["exclude-result-prefixes", "extension-element-prefixes"] {
                if let Some(v) = a.attribute((XSL_NS, attr)) {
                    excluded.extend(self.excluded_uris(a, v));
                }
            }
        }
        let ns_decls = in_scope(n)
            .into_iter()
            .filter(|(_, uri)| !excluded.contains(uri))
            .collect();
        let mut attrs = Vec::new();
        let mut sets = Vec::new();
        for a in n.attributes() {
            if a.namespace() == Some(XSL_NS) {
                if a.name() == "use-attribute-sets" {
                    sets.extend(a.value().split_whitespace().map(str::to_owned));
                }
                continue;
            }
            let prefix = written_prefix(self.text, a.range_qname().start);
            let value = self.avt_of(n, a.value(), a.range_value().start)?;
            attrs.push((
                prefix,
                a.name().to_owned(),
                a.namespace().map(str::to_owned),
                value,
            ));
        }
        Ok(Instr::Literal(Box::new(LiteralElem {
            prefix: written_prefix(self.text, n.range().start + 1),
            local: n.tag_name().name().to_owned(),
            ns: n.tag_name().namespace().map(str::to_owned),
            ns_decls,
            attrs,
            sets,
            body: self.body(n)?,
        })))
    }
}

/// The whitespace-only text nodes `xsl:strip-space` removes from the source.
fn stripped_text_nodes(doc: &Document, ss: &Stylesheet) -> HashSet<roxmltree::NodeId> {
    let mut out = HashSet::new();
    if ss.strip.is_empty() {
        return out;
    }
    let best = |list: &[(NameTest, usize)], ns: Option<&str>, local: &str| {
        list.iter()
            .filter_map(|(t, order)| t.score(ns, local).map(|s| (s, *order)))
            .max()
    };
    for t in doc.descendants().filter(Node::is_text) {
        if !t.text().unwrap_or_default().chars().all(is_xml_space) {
            continue;
        }
        let Some(parent) = t.parent_element() else {
            continue;
        };
        if Compiler::preserves_space(parent) {
            continue;
        }
        let (ns, local) = (parent.tag_name().namespace(), parent.tag_name().name());
        let strip = best(&ss.strip, ns, local);
        let keep = best(&ss.preserve, ns, local);
        if strip.is_some() && strip > keep {
            out.insert(t.id());
        }
    }
    out
}

// ----------------------------------------------------------------------
// Running

/// The XSLT context: the current node, its position and the list size.
#[derive(Clone, Copy)]
struct Cx<'s, 'a, 'i> {
    node: XNode<'a, 'i>,
    pos: usize,
    size: usize,
    mode: &'s str,
}

enum Global<'a, 'i> {
    Pending(usize),
    Busy,
    Done(Value<'a, 'i>),
}

type KeyIndex<'a, 'i> = Rc<HashMap<String, Vec<XNode<'a, 'i>>>>;

/// The result tree under construction: an element's attributes and children.
struct Out {
    nodes: Vec<FragNode>,
    attrs: Option<Vec<FragAttr>>,
}

impl Out {
    fn root() -> Self {
        Self {
            nodes: Vec::new(),
            attrs: None,
        }
    }

    fn element() -> Self {
        Self {
            nodes: Vec::new(),
            attrs: Some(Vec::new()),
        }
    }

    fn text(&mut self, s: &str, raw: bool) {
        if s.is_empty() {
            return;
        }
        if let Some(FragNode::Text { text, raw: r }) = self.nodes.last_mut()
            && *r == raw
        {
            text.push_str(s);
            return;
        }
        self.nodes.push(FragNode::Text {
            text: s.to_owned(),
            raw,
        });
    }

    fn push(&mut self, n: FragNode) {
        match n {
            FragNode::Text { text, raw } => self.text(&text, raw),
            n => self.nodes.push(n),
        }
    }

    /// Adds an attribute, replacing one with the same name. Attributes after
    /// children (or outside an element) are ignored, as XSLT 1.0 allows.
    fn attr(&mut self, a: FragAttr) {
        if !self.nodes.is_empty() {
            return;
        }
        if let Some(attrs) = &mut self.attrs {
            match attrs
                .iter_mut()
                .find(|e| e.local == a.local && e.ns == a.ns)
            {
                Some(e) => *e = a,
                None => attrs.push(a),
            }
        }
    }
}

struct Exec<'s, 'a, 'i> {
    ss: &'s Stylesheet,
    eng: Engine<'a, 'i>,
    doc: &'a Document<'i>,
    /// Local variables and parameters; a template sees those from `base` up.
    vars: RefCell<Vec<(String, Value<'a, 'i>)>>,
    base: Cell<usize>,
    globals: RefCell<HashMap<String, Global<'a, 'i>>>,
    current: Cell<Option<XNode<'a, 'i>>>,
    keys: RefCell<HashMap<String, KeyIndex<'a, 'i>>>,
    building_keys: RefCell<HashSet<String>>,
    messages: RefCell<Vec<String>>,
    depth: Cell<usize>,
    /// Where in the stylesheet the running instruction is, for errors.
    pos: Cell<usize>,
}

impl<'s, 'a, 'i> Exec<'s, 'a, 'i> {
    fn err(&self, message: impl Into<String>) -> ToolError {
        ToolError::at(message, self.pos.get())
    }

    fn run(&self) -> Result<Vec<FragNode>, ToolError> {
        // Evaluate globals up front so their errors surface even if unused.
        for g in &self.ss.globals {
            self.global(&g.name).map_err(|m| self.err(m))?;
        }
        let mut out = Out::root();
        let root = XNode::Node(self.doc.root());
        self.apply(&[root], "", &[], &mut out)?;
        Ok(out.nodes)
    }

    fn eval(&self, x: &XPath, cx: &Cx<'s, 'a, 'i>) -> Result<Value<'a, 'i>, ToolError> {
        let saved = self.current.replace(Some(cx.node));
        let r = self.eng.eval(x, cx.node, cx.pos, cx.size, self);
        self.current.set(saved);
        r.map_err(|m| self.err(m))
    }

    fn eval_string(&self, x: &XPath, cx: &Cx<'s, 'a, 'i>) -> Result<String, ToolError> {
        Ok(self.eng.string(&self.eval(x, cx)?))
    }

    fn eval_nodes(&self, x: &XPath, cx: &Cx<'s, 'a, 'i>) -> Result<Vec<XNode<'a, 'i>>, ToolError> {
        match self.eval(x, cx)? {
            Value::Nodes(n) => Ok(n),
            Value::Frag(_) => {
                Err(self.err("A result tree fragment can't be used as a node-set in XSLT 1.0."))
            }
            _ => Err(self.err("This expression must select nodes.")),
        }
    }

    fn avt(&self, avt: &Avt, cx: &Cx<'s, 'a, 'i>) -> Result<String, ToolError> {
        let mut s = String::new();
        for p in &avt.0 {
            match p {
                AvtPart::Lit(l) => s.push_str(l),
                AvtPart::Expr(x) => s.push_str(&self.eval_string(x, cx)?),
            }
        }
        Ok(s)
    }

    fn matches(&self, pattern: &XPath, n: XNode<'a, 'i>) -> Result<bool, ToolError> {
        let saved = self.current.replace(Some(n));
        let r = self.eng.matches(pattern, n, self);
        self.current.set(saved);
        r.map_err(|m| self.err(m))
    }

    fn exec_body(&self, body: &Body, cx: &Cx<'s, 'a, 'i>, out: &mut Out) -> Result<(), ToolError> {
        let mark = self.vars.borrow().len();
        for (pos, instr) in body {
            self.pos.set(*pos);
            self.exec(instr, cx, out)?;
        }
        self.vars.borrow_mut().truncate(mark);
        Ok(())
    }

    /// Instantiates a body and returns its text (for attributes, comments...).
    fn body_string(&self, body: &Body, cx: &Cx<'s, 'a, 'i>) -> Result<String, ToolError> {
        let mut o = Out::root();
        self.exec_body(body, cx, &mut o)?;
        Ok(Fragment { nodes: o.nodes }.string_value())
    }

    fn var_value(&self, d: &VarDecl, cx: &Cx<'s, 'a, 'i>) -> Result<Value<'a, 'i>, ToolError> {
        self.pos.set(d.pos);
        Ok(match &d.value {
            VarValue::Select(x) => self.eval(x, cx)?,
            VarValue::Body(body) => {
                let mut o = Out::root();
                self.exec_body(body, cx, &mut o)?;
                Value::Frag(Rc::new(Fragment { nodes: o.nodes }))
            }
            VarValue::Empty => Value::Str(String::new()),
        })
    }

    fn global(&self, name: &str) -> Result<Value<'a, 'i>, String> {
        let index = match self.globals.borrow().get(name) {
            Some(Global::Done(v)) => return Ok(v.clone()),
            Some(Global::Busy) => {
                return Err(format!("The variable ${name} depends on itself."));
            }
            Some(Global::Pending(i)) => *i,
            None => return Err(format!("The variable ${name} isn't declared.")),
        };
        self.globals
            .borrow_mut()
            .insert(name.to_owned(), Global::Busy);
        let decl = &self.ss.globals[index];
        let saved_base = self.base.replace(self.vars.borrow().len());
        let saved_pos = self.pos.get();
        let cx = Cx {
            node: XNode::Node(self.doc.root()),
            pos: 1,
            size: 1,
            mode: "",
        };
        let r = self.var_value(decl, &cx);
        self.base.set(saved_base);
        self.pos.set(saved_pos);
        let v = r.map_err(|e| e.message)?;
        self.globals
            .borrow_mut()
            .insert(name.to_owned(), Global::Done(v.clone()));
        Ok(v)
    }

    fn params(
        &self,
        decls: &[VarDecl],
        cx: &Cx<'s, 'a, 'i>,
    ) -> Result<Vec<(String, Value<'a, 'i>)>, ToolError> {
        decls
            .iter()
            .map(|d| Ok((d.name.clone(), self.var_value(d, cx)?)))
            .collect()
    }

    fn find_rule(&self, n: XNode<'a, 'i>, mode: &str) -> Result<Option<usize>, ToolError> {
        let Some(rules) = self.ss.rules.get(mode) else {
            return Ok(None);
        };
        for r in rules {
            if self.matches(&r.pattern, n)? {
                return Ok(Some(r.template));
            }
        }
        Ok(None)
    }

    fn apply(
        &self,
        nodes: &[XNode<'a, 'i>],
        mode: &'s str,
        params: &[(String, Value<'a, 'i>)],
        out: &mut Out,
    ) -> Result<(), ToolError> {
        let size = nodes.len();
        for (i, &n) in nodes.iter().enumerate() {
            let cx = Cx {
                node: n,
                pos: i + 1,
                size,
                mode,
            };
            let saved_pos = self.pos.get();
            match self.find_rule(n, mode)? {
                Some(t) => self.invoke(t, &cx, params, out)?,
                None => self.builtin(&cx, out)?,
            }
            self.pos.set(saved_pos);
        }
        Ok(())
    }

    fn builtin(&self, cx: &Cx<'s, 'a, 'i>, out: &mut Out) -> Result<(), ToolError> {
        match cx.node {
            XNode::Node(x) if x.is_root() || x.is_element() => {
                let children = self.eng.children(cx.node);
                self.enter()?;
                let r = self.apply(&children, cx.mode, &[], out);
                self.leave();
                r
            }
            XNode::Node(x) if x.is_text() => {
                out.text(x.text().unwrap_or_default(), false);
                Ok(())
            }
            XNode::Attr(..) => {
                out.text(&self.eng.string_value(cx.node), false);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn enter(&self) -> Result<(), ToolError> {
        let d = self.depth.get() + 1;
        if d > MAX_DEPTH {
            return Err(
                self.err("Templates call each other too deeply; is there endless recursion?")
            );
        }
        self.depth.set(d);
        Ok(())
    }

    fn leave(&self) {
        self.depth.set(self.depth.get() - 1);
    }

    fn invoke(
        &self,
        index: usize,
        cx: &Cx<'s, 'a, 'i>,
        params: &[(String, Value<'a, 'i>)],
        out: &mut Out,
    ) -> Result<(), ToolError> {
        self.enter()?;
        let t = &self.ss.templates[index];
        let len = self.vars.borrow().len();
        let saved_base = self.base.replace(len);
        let mut result = Ok(());
        for p in &t.params {
            let v = match params.iter().rev().find(|(n, _)| *n == p.name) {
                Some((_, v)) => v.clone(),
                None => match self.var_value(p, cx) {
                    Ok(v) => v,
                    Err(e) => {
                        result = Err(e);
                        break;
                    }
                },
            };
            self.vars.borrow_mut().push((p.name.clone(), v));
        }
        if result.is_ok() {
            result = self.exec_body(&t.body, cx, out);
        }
        self.vars.borrow_mut().truncate(len);
        self.base.set(saved_base);
        self.leave();
        result
    }

    fn sort(
        &self,
        nodes: Vec<XNode<'a, 'i>>,
        sorts: &[Sort],
        cx: &Cx<'s, 'a, 'i>,
    ) -> Result<Vec<XNode<'a, 'i>>, ToolError> {
        if sorts.is_empty() || nodes.len() < 2 {
            return Ok(nodes);
        }
        enum K {
            Text(String, String),
            Num(f64),
        }
        struct Spec {
            descending: bool,
            number: bool,
            upper_first: bool,
        }
        let mut specs = Vec::new();
        for s in sorts {
            let get = |a: &Option<Avt>| -> Result<Option<String>, ToolError> {
                a.as_ref().map(|a| self.avt(a, cx)).transpose()
            };
            specs.push(Spec {
                descending: get(&s.order)?.as_deref() == Some("descending"),
                number: get(&s.data_type)?.as_deref() == Some("number"),
                upper_first: get(&s.case_order)?.as_deref() == Some("upper-first"),
            });
        }
        let size = nodes.len();
        let mut keyed = Vec::with_capacity(size);
        for (i, &n) in nodes.iter().enumerate() {
            let ncx = Cx {
                node: n,
                pos: i + 1,
                size,
                mode: cx.mode,
            };
            let mut keys = Vec::with_capacity(sorts.len());
            for (s, spec) in sorts.iter().zip(&specs) {
                let v = self.eval(&s.select, &ncx)?;
                keys.push(if spec.number {
                    K::Num(self.eng.number(&v))
                } else {
                    let t = self.eng.string(&v);
                    K::Text(t.to_lowercase(), t)
                });
            }
            keyed.push((keys, n));
        }
        keyed.sort_by(|(a, _), (b, _)| {
            for ((x, y), spec) in a.iter().zip(b).zip(&specs) {
                let o = match (x, y) {
                    (K::Num(x), K::Num(y)) => match (x.is_nan(), y.is_nan()) {
                        (true, true) => Ordering::Equal,
                        (true, false) => Ordering::Less,
                        (false, true) => Ordering::Greater,
                        _ => x.partial_cmp(y).unwrap_or(Ordering::Equal),
                    },
                    (K::Text(xl, x), K::Text(yl, y)) => xl.cmp(yl).then_with(|| {
                        // Same letters: order by case.
                        let o = x.cmp(y);
                        if spec.upper_first { o } else { o.reverse() }
                    }),
                    _ => Ordering::Equal,
                };
                let o = if spec.descending { o.reverse() } else { o };
                if o != Ordering::Equal {
                    return o;
                }
            }
            Ordering::Equal
        });
        Ok(keyed.into_iter().map(|(_, n)| n).collect())
    }

    fn attr_set(
        &self,
        name: &str,
        cx: &Cx<'s, 'a, 'i>,
        out: &mut Out,
        depth: usize,
    ) -> Result<(), ToolError> {
        let Some(set) = self.ss.attr_sets.get(name) else {
            return Err(self.err(format!("There's no attribute set named '{name}'.")));
        };
        if depth > 50 {
            return Err(self.err(format!("The attribute set '{name}' uses itself.")));
        }
        for u in &set.uses {
            self.attr_set(u, cx, out, depth + 1)?;
        }
        // Attribute sets see only global variables.
        let len = self.vars.borrow().len();
        let saved = self.base.replace(len);
        let r = self.exec_body(&set.body, cx, out);
        self.base.set(saved);
        r
    }

    fn copy_node(&self, n: XNode<'a, 'i>, out: &mut Out) {
        match n {
            XNode::Attr(e, i) => {
                if let Some(a) = e.attributes().nth(i) {
                    out.attr(self.copy_attr(a));
                }
            }
            XNode::Node(x) => match x.node_type() {
                NodeType::Root => {
                    for c in self.eng.children(n) {
                        self.copy_node(c, out);
                    }
                }
                NodeType::Element => {
                    let mut inner = Out::element();
                    for a in x.attributes() {
                        inner.attr(self.copy_attr(a));
                    }
                    for c in self.eng.children(n) {
                        self.copy_node(c, &mut inner);
                    }
                    out.push(FragNode::Element(self.shallow_element(x, inner)));
                }
                NodeType::Text => out.text(x.text().unwrap_or_default(), false),
                NodeType::Comment => {
                    out.push(FragNode::Comment(x.text().unwrap_or_default().to_owned()))
                }
                NodeType::PI => {
                    if let Some(pi) = x.pi() {
                        out.push(FragNode::Pi {
                            target: pi.target.to_owned(),
                            data: pi.value.unwrap_or_default().to_owned(),
                        });
                    }
                }
            },
        }
    }

    fn copy_attr(&self, a: roxmltree::Attribute) -> FragAttr {
        FragAttr {
            prefix: written_prefix(self.doc.input_text(), a.range_qname().start),
            local: a.name().to_owned(),
            ns: a.namespace().map(str::to_owned),
            value: a.value().to_owned(),
        }
    }

    fn shallow_element(&self, x: Node, inner: Out) -> FragElement {
        FragElement {
            prefix: written_prefix(self.doc.input_text(), x.range().start + 1),
            local: x.tag_name().name().to_owned(),
            ns: x.tag_name().namespace().map(str::to_owned),
            ns_decls: in_scope(x),
            attrs: inner.attrs.unwrap_or_default(),
            children: inner.nodes,
        }
    }

    /// Resolves an `xsl:element` / `xsl:attribute` name to (prefix, local, ns).
    #[allow(clippy::type_complexity)]
    fn computed_name(
        &self,
        name: &Avt,
        namespace: &Option<Avt>,
        scope: &Scope,
        cx: &Cx<'s, 'a, 'i>,
        element: bool,
    ) -> Result<(Option<String>, String, Option<String>), ToolError> {
        let qname = self.avt(name, cx)?.trim().to_owned();
        if !is_qname(&qname) {
            return Err(self.err(format!("'{qname}' isn't a valid name.")));
        }
        let (prefix, local) = split_qname(&qname);
        let ns = match namespace {
            Some(ns) => {
                let uri = self.avt(ns, cx)?;
                (!uri.is_empty()).then_some(uri)
            }
            None => match prefix {
                Some("xml") => Some(XML_NS.to_owned()),
                Some(p) => Some(
                    scope
                        .iter()
                        .find(|(sp, _)| sp.as_deref() == Some(p))
                        .map(|(_, u)| u.clone())
                        .ok_or_else(|| {
                            self.err(format!("The namespace prefix '{p}' isn't declared."))
                        })?,
                ),
                None if element => scope
                    .iter()
                    .find(|(sp, _)| sp.is_none())
                    .map(|(_, u)| u.clone())
                    .filter(|u| !u.is_empty()),
                None => None,
            },
        };
        let prefix = if ns.is_some() {
            prefix.map(str::to_owned)
        } else {
            None
        };
        Ok((prefix, local.to_owned(), ns))
    }

    fn exec(&self, instr: &Instr, cx: &Cx<'s, 'a, 'i>, out: &mut Out) -> Result<(), ToolError> {
        match instr {
            Instr::Text { text, raw } => out.text(text, *raw),
            Instr::Literal(l) => {
                let mut inner = Out::element();
                for s in &l.sets {
                    self.attr_set(s, cx, &mut inner, 0)?;
                }
                for (prefix, local, ns, value) in &l.attrs {
                    let value = self.avt(value, cx)?;
                    inner.attr(FragAttr {
                        prefix: prefix.clone(),
                        local: local.clone(),
                        ns: ns.clone(),
                        value,
                    });
                }
                self.exec_body(&l.body, cx, &mut inner)?;
                out.push(FragNode::Element(FragElement {
                    prefix: l.prefix.clone(),
                    local: l.local.clone(),
                    ns: l.ns.clone(),
                    ns_decls: l.ns_decls.clone(),
                    attrs: inner.attrs.unwrap_or_default(),
                    children: inner.nodes,
                }));
            }
            Instr::ValueOf { select, raw } => {
                let s = self.eval_string(select, cx)?;
                out.text(&s, *raw);
            }
            Instr::ApplyTemplates {
                select,
                mode,
                sorts,
                params,
            } => {
                let nodes = match select {
                    Some(x) => self.eval_nodes(x, cx)?,
                    None => self.eng.children(cx.node),
                };
                let nodes = self.sort(nodes, sorts, cx)?;
                let params = self.params(params, cx)?;
                self.apply(&nodes, mode.as_deref().unwrap_or(""), &params, out)?;
            }
            Instr::ApplyImports => self.builtin(cx, out)?,
            Instr::CallTemplate { name, params } => {
                let Some(&index) = self.ss.named.get(name) else {
                    return Err(self.err(format!("There's no template named '{name}'.")));
                };
                let params = self.params(params, cx)?;
                self.invoke(index, cx, &params, out)?;
            }
            Instr::ForEach {
                select,
                sorts,
                body,
            } => {
                let nodes = self.eval_nodes(select, cx)?;
                let nodes = self.sort(nodes, sorts, cx)?;
                let size = nodes.len();
                for (i, &n) in nodes.iter().enumerate() {
                    let ncx = Cx {
                        node: n,
                        pos: i + 1,
                        size,
                        mode: cx.mode,
                    };
                    self.exec_body(body, &ncx, out)?;
                }
            }
            Instr::If { test, body } => {
                if self.eng.boolean(&self.eval(test, cx)?) {
                    self.exec_body(body, cx, out)?;
                }
            }
            Instr::Choose { whens, otherwise } => {
                for (test, body) in whens {
                    if self.eng.boolean(&self.eval(test, cx)?) {
                        return self.exec_body(body, cx, out);
                    }
                }
                if let Some(body) = otherwise {
                    self.exec_body(body, cx, out)?;
                }
            }
            Instr::Variable(d) => {
                let v = self.var_value(d, cx)?;
                self.vars.borrow_mut().push((d.name.clone(), v));
            }
            Instr::Copy { sets, body } => match cx.node {
                XNode::Node(x) if x.is_element() => {
                    let mut inner = Out::element();
                    for s in sets {
                        self.attr_set(s, cx, &mut inner, 0)?;
                    }
                    self.exec_body(body, cx, &mut inner)?;
                    out.push(FragNode::Element(self.shallow_element(x, inner)));
                }
                XNode::Node(x) if x.is_root() => self.exec_body(body, cx, out)?,
                n => self.copy_node(n, out),
            },
            Instr::CopyOf { select } => match self.eval(select, cx)? {
                Value::Nodes(nodes) => {
                    for n in nodes {
                        self.copy_node(n, out);
                    }
                }
                Value::Frag(f) => {
                    for n in &f.nodes {
                        out.push(n.clone());
                    }
                }
                v => out.text(&self.eng.string(&v), false),
            },
            Instr::Element {
                name,
                namespace,
                scope,
                sets,
                body,
            } => {
                let (prefix, local, ns) = self.computed_name(name, namespace, scope, cx, true)?;
                let mut inner = Out::element();
                for s in sets {
                    self.attr_set(s, cx, &mut inner, 0)?;
                }
                self.exec_body(body, cx, &mut inner)?;
                out.push(FragNode::Element(FragElement {
                    prefix,
                    local,
                    ns,
                    ns_decls: Vec::new(),
                    attrs: inner.attrs.unwrap_or_default(),
                    children: inner.nodes,
                }));
            }
            Instr::Attribute {
                name,
                namespace,
                scope,
                body,
            } => {
                let (prefix, local, ns) = self.computed_name(name, namespace, scope, cx, false)?;
                if prefix.is_none() && local == "xmlns" {
                    return Err(self.err("xsl:attribute can't create an xmlns attribute."));
                }
                let value = self.body_string(body, cx)?;
                out.attr(FragAttr {
                    prefix,
                    local,
                    ns,
                    value,
                });
            }
            Instr::Comment(body) => {
                let mut text = self.body_string(body, cx)?;
                while text.contains("--") {
                    text = text.replace("--", "- -");
                }
                if text.ends_with('-') {
                    text.push(' ');
                }
                out.push(FragNode::Comment(text));
            }
            Instr::Pi { name, body } => {
                let target = self.avt(name, cx)?;
                if !is_ncname(&target) || target.eq_ignore_ascii_case("xml") {
                    return Err(self.err(format!(
                        "'{target}' isn't a valid processing-instruction name."
                    )));
                }
                let data = self.body_string(body, cx)?.replace("?>", "? >");
                out.push(FragNode::Pi {
                    target,
                    data: data.trim_start().to_owned(),
                });
            }
            Instr::Number(n) => {
                let s = self.number(n, cx)?;
                out.text(&s, false);
            }
            Instr::Message { body, terminate } => {
                let text = self.body_string(body, cx)?;
                if *terminate {
                    return Err(self.err(format!("The stylesheet stopped: {text}")));
                }
                self.messages.borrow_mut().push(text);
            }
        }
        Ok(())
    }

    fn number(&self, n: &NumberInstr, cx: &Cx<'s, 'a, 'i>) -> Result<String, ToolError> {
        let numbers: Vec<f64> = match &n.value {
            Some(v) => {
                let x = self.eng.number(&self.eval(v, cx)?);
                vec![x]
            }
            None => self
                .count_numbers(n, cx)?
                .into_iter()
                .map(|v| v as f64)
                .collect(),
        };
        let format = self.avt(&n.format, cx)?;
        let grouping = match (&n.grouping_separator, &n.grouping_size) {
            (Some(s), Some(z)) => {
                let sep = self.avt(s, cx)?;
                let size = self.avt(z, cx)?.trim().parse::<usize>().unwrap_or(0);
                (size > 0).then_some((sep, size))
            }
            _ => None,
        };
        Ok(format_number_list(&numbers, &format, grouping.as_ref()))
    }

    fn count_numbers(&self, n: &NumberInstr, cx: &Cx<'s, 'a, 'i>) -> Result<Vec<u64>, ToolError> {
        let current = cx.node;
        let same_kind = |m: XNode<'a, 'i>| -> bool {
            match (current, m) {
                (XNode::Attr(..), XNode::Attr(..)) => {
                    self.eng.local_name(current) == self.eng.local_name(m)
                        && self.eng.namespace(current) == self.eng.namespace(m)
                }
                (XNode::Node(a), XNode::Node(b)) => {
                    a.node_type() == b.node_type()
                        && match a.node_type() {
                            NodeType::Element => a.tag_name() == b.tag_name(),
                            NodeType::PI => a.pi().map(|p| p.target) == b.pi().map(|p| p.target),
                            _ => true,
                        }
                }
                _ => false,
            }
        };
        let counts = |m: XNode<'a, 'i>| -> Result<bool, ToolError> {
            match &n.count {
                Some(p) => self.matches(p, m),
                None => Ok(same_kind(m)),
            }
        };
        let is_from = |m: XNode<'a, 'i>| -> Result<bool, ToolError> {
            match &n.from {
                Some(p) => self.matches(p, m),
                None => Ok(false),
            }
        };
        let sibling_number = |m: XNode<'a, 'i>| -> Result<u64, ToolError> {
            let mut k = 1;
            for s in self.eng.preceding_siblings(m) {
                if counts(s)? {
                    k += 1;
                }
            }
            Ok(k)
        };
        match n.level {
            Level::Single | Level::Multiple => {
                let mut out = Vec::new();
                for a in self.eng.ancestors_or_self(current) {
                    if is_from(a)? {
                        break;
                    }
                    if counts(a)? {
                        out.push(sibling_number(a)?);
                        if n.level == Level::Single {
                            break;
                        }
                    }
                }
                out.reverse();
                Ok(out)
            }
            Level::Any => {
                let mut k = 0;
                for m in self.eng.all_nodes() {
                    if is_from(m)? {
                        k = 0;
                    }
                    if counts(m)? {
                        k += 1;
                    }
                    if m == current {
                        break;
                    }
                }
                Ok(if k == 0 { Vec::new() } else { vec![k] })
            }
        }
    }

    fn key_index(&self, name: &str) -> Result<KeyIndex<'a, 'i>, String> {
        if let Some(ix) = self.keys.borrow().get(name) {
            return Ok(ix.clone());
        }
        let decls: Vec<&Key> = self.ss.keys.iter().filter(|k| k.name == name).collect();
        if decls.is_empty() {
            return Err(format!("There's no xsl:key named '{name}'."));
        }
        if !self.building_keys.borrow_mut().insert(name.to_owned()) {
            return Err(format!("The key '{name}' depends on itself."));
        }
        let mut map: HashMap<String, Vec<XNode<'a, 'i>>> = HashMap::new();
        let saved = self.current.get();
        let mut result = Ok(());
        'outer: for n in self.eng.all_nodes() {
            for k in &decls {
                self.current.set(Some(n));
                let matched = match self.eng.matches(&k.pattern, n, self) {
                    Ok(m) => m,
                    Err(e) => {
                        result = Err(e);
                        break 'outer;
                    }
                };
                if !matched {
                    continue;
                }
                match self.eng.eval(&k.use_, n, 1, 1, self) {
                    Ok(Value::Nodes(values)) => {
                        for v in values {
                            map.entry(self.eng.string_value(v)).or_default().push(n);
                        }
                    }
                    Ok(v) => map.entry(self.eng.string(&v)).or_default().push(n),
                    Err(e) => {
                        result = Err(e);
                        break 'outer;
                    }
                }
            }
        }
        self.current.set(saved);
        self.building_keys.borrow_mut().remove(name);
        result?;
        for nodes in map.values_mut() {
            nodes.dedup();
        }
        let ix = Rc::new(map);
        self.keys.borrow_mut().insert(name.to_owned(), ix.clone());
        Ok(ix)
    }
}

impl<'a, 'i> Env<'a, 'i> for Exec<'_, 'a, 'i> {
    fn variable(&self, name: &str) -> Result<Value<'a, 'i>, String> {
        {
            let vars = self.vars.borrow();
            let base = self.base.get().min(vars.len());
            if let Some((_, v)) = vars[base..].iter().rev().find(|(n, _)| n == name) {
                return Ok(v.clone());
            }
        }
        self.global(name)
    }

    fn function(
        &self,
        name: &str,
        args: &[Value<'a, 'i>],
        context: XNode<'a, 'i>,
    ) -> Option<Result<Value<'a, 'i>, String>> {
        let arity = |min: usize, max: usize| -> Result<(), String> {
            if args.len() < min || args.len() > max {
                Err(format!("{name}() has the wrong number of arguments."))
            } else {
                Ok(())
            }
        };
        let s = |i: usize| self.eng.string(&args[i]);
        let result = (|| -> Result<Value<'a, 'i>, String> {
            match name {
                "current" => {
                    arity(0, 0)?;
                    Ok(Value::Nodes(self.current.get().into_iter().collect()))
                }
                "key" => {
                    arity(2, 2)?;
                    let ix = self.key_index(&s(0))?;
                    let mut found = Vec::new();
                    let mut lookup = |v: String| {
                        if let Some(nodes) = ix.get(&v) {
                            found.extend(nodes.iter().copied());
                        }
                    };
                    match &args[1] {
                        Value::Nodes(ns) => {
                            for &n in ns {
                                lookup(self.eng.string_value(n));
                            }
                        }
                        v => lookup(self.eng.string(v)),
                    }
                    xpath::sort_nodes(&mut found);
                    Ok(Value::Nodes(found))
                }
                "generate-id" => {
                    arity(0, 1)?;
                    let n = match args.first() {
                        None => Some(context),
                        Some(Value::Nodes(ns)) => ns.first().copied(),
                        Some(_) => return Err("generate-id() needs a node-set.".into()),
                    };
                    Ok(Value::Str(match n {
                        None => String::new(),
                        Some(XNode::Node(x)) => format!("n{}", x.id().get()),
                        Some(XNode::Attr(x, i)) => format!("n{}a{i}", x.id().get()),
                    }))
                }
                "format-number" => {
                    arity(2, 3)?;
                    let fmt_name = if args.len() == 3 { s(2) } else { String::new() };
                    let df = match self.ss.decimal_formats.get(&fmt_name) {
                        Some(f) => f.clone(),
                        None if fmt_name.is_empty() => DecimalFormat::default(),
                        None => {
                            return Err(format!(
                                "There's no xsl:decimal-format named '{fmt_name}'."
                            ));
                        }
                    };
                    Ok(Value::Str(format_decimal(
                        self.eng.number(&args[0]),
                        &s(1),
                        &df,
                    )))
                }
                "system-property" => {
                    arity(1, 1)?;
                    let prop = s(0);
                    Ok(match split_qname(&prop).1 {
                        "version" => Value::Num(1.0),
                        "vendor" => Value::Str("Ettin Edit".into()),
                        _ => Value::Str(String::new()),
                    })
                }
                "element-available" => {
                    arity(1, 1)?;
                    let q = s(0);
                    let (p, l) = split_qname(&q);
                    Ok(Value::Bool(p.is_some() && XSL_INSTRUCTIONS.contains(&l)))
                }
                "function-available" => {
                    arity(1, 1)?;
                    let f = s(0);
                    Ok(Value::Bool(
                        (xpath::is_core_function(&f) || XSLT_FUNCTIONS.contains(&f.as_str()))
                            && f != "document",
                    ))
                }
                "document" => Err(
                    "document() isn't supported: the transform works on the open document only."
                        .into(),
                ),
                "unparsed-entity-uri" => {
                    arity(1, 1)?;
                    Ok(Value::Str(String::new()))
                }
                _ => Err(String::new()),
            }
        })();
        match result {
            Err(e) if e.is_empty() => None,
            r => Some(r),
        }
    }
}

// ----------------------------------------------------------------------
// Number formatting

fn alpha(mut n: u64, upper: bool) -> String {
    let base = if upper { b'A' } else { b'a' };
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push((base + (n % 26) as u8) as char);
        n /= 26;
    }
    out.iter().rev().collect()
}

fn roman(mut n: u64, upper: bool) -> String {
    const TABLE: &[(u64, &str)] = &[
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for &(v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    if upper { out.to_uppercase() } else { out }
}

fn group_digits(digits: &str, grouping: Option<&(String, usize)>) -> String {
    let Some((sep, size)) = grouping else {
        return digits.to_owned();
    };
    let chars: Vec<char> = digits.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(*size) {
            out.push_str(sep);
        }
        out.push(*c);
    }
    out
}

fn format_one(n: u64, token: &str, grouping: Option<&(String, usize)>) -> String {
    match token {
        "a" | "A" if n > 0 => alpha(n, token == "A"),
        "i" | "I" if n > 0 && n < 4000 => roman(n, token == "I"),
        t => {
            let width = if t.chars().all(|c| c.is_ascii_digit()) {
                t.len()
            } else {
                1
            };
            group_digits(&format!("{n:0width$}"), grouping)
        }
    }
}

/// Formats numbers for `xsl:number` with a format string like "1.", "(a)" or "1.1".
fn format_number_list(numbers: &[f64], format: &str, grouping: Option<&(String, usize)>) -> String {
    // Split into alternating separators and alphanumeric tokens.
    let mut tokens: Vec<String> = Vec::new();
    let mut seps: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_token = false;
    for c in format.chars() {
        let alnum = c.is_alphanumeric();
        if alnum != in_token {
            if in_token {
                tokens.push(std::mem::take(&mut cur));
            } else {
                seps.push(std::mem::take(&mut cur));
            }
            in_token = alnum;
        }
        cur.push(c);
    }
    if in_token {
        tokens.push(cur);
        seps.push(String::new());
    } else {
        seps.push(cur);
    }
    // seps[0] is the prefix and the last is the suffix.
    if tokens.is_empty() {
        tokens.push("1".into());
        seps = vec![String::new(), String::new()];
    }
    let prefix = seps.first().cloned().unwrap_or_default();
    let suffix = seps.last().cloned().unwrap_or_default();
    let mut out = prefix;
    for (i, &x) in numbers.iter().enumerate() {
        if i > 0 {
            let sep = if i < tokens.len() {
                seps[i].clone()
            } else if tokens.len() > 1 {
                seps[tokens.len() - 1].clone()
            } else {
                ".".to_owned()
            };
            out.push_str(&sep);
        }
        let token = &tokens[i.min(tokens.len() - 1)];
        if x.is_nan() || x.is_infinite() || x < 0.0 {
            out.push_str(&xpath::number_to_string(x));
        } else {
            let n = (x + 0.5).floor().max(0.0) as u64;
            out.push_str(&format_one(n, token, grouping));
        }
    }
    out.push_str(&suffix);
    out
}

/// `format-number()`: formats `x` with a JDK-style decimal pattern.
fn format_decimal(x: f64, pattern: &str, df: &DecimalFormat) -> String {
    let (pos_pat, neg_pat) = match pattern.split_once(df.pattern_separator) {
        Some((p, n)) => (p, Some(n)),
        None => (pattern, None),
    };
    let is_special = |c: char| {
        c == df.digit || c == df.zero || c == df.grouping || c == df.decimal || c.is_ascii_digit()
    };
    // prefix, number part, suffix
    let split = |p: &str| -> (String, String, String) {
        let start = p.find(is_special).unwrap_or(p.len());
        let rest = &p[start..];
        let end = rest.find(|c: char| !is_special(c)).unwrap_or(rest.len());
        (
            p[..start].to_owned(),
            rest[..end].to_owned(),
            rest[end..].to_owned(),
        )
    };
    let (prefix, number, suffix) = split(pos_pat);
    if x.is_nan() {
        return df.nan.clone();
    }
    let (neg_prefix, neg_suffix) = match neg_pat {
        Some(n) => {
            let (p, _, s) = split(n);
            (p, s)
        }
        None => (format!("{}{prefix}", df.minus), suffix.clone()),
    };
    let negative = x < 0.0;
    let (pre, suf) = if negative {
        (neg_prefix, neg_suffix)
    } else {
        (prefix, suffix)
    };
    let mut value = x.abs();
    if pre.contains(df.percent) || suf.contains(df.percent) {
        value *= 100.0;
    } else if pre.contains(df.per_mille) || suf.contains(df.per_mille) {
        value *= 1000.0;
    }
    if value.is_infinite() {
        return format!("{pre}{}{suf}", df.infinity);
    }
    let (int_pat, frac_pat) = match number.split_once(df.decimal) {
        Some((i, f)) => (i.to_owned(), f.to_owned()),
        None => (number.clone(), String::new()),
    };
    let min_int = int_pat.chars().filter(|&c| c == df.zero).count();
    let grouping_size = int_pat.rfind(df.grouping).map(|g| {
        int_pat[g..]
            .chars()
            .skip(1)
            .filter(|&c| c != df.grouping)
            .count()
    });
    let min_frac = frac_pat.chars().filter(|&c| c == df.zero).count();
    let max_frac = frac_pat
        .chars()
        .filter(|&c| c == df.zero || c == df.digit)
        .count();
    let formatted = format!("{value:.max_frac$}");
    let (int_digits, frac_digits) = match formatted.split_once('.') {
        Some((i, f)) => (i.to_owned(), f.to_owned()),
        None => (formatted, String::new()),
    };
    let mut frac = frac_digits;
    while frac.len() > min_frac && frac.ends_with('0') {
        frac.pop();
    }
    let mut int = int_digits.trim_start_matches('0').to_owned();
    while int.len() < min_int {
        int.insert(0, '0');
    }
    if let Some(size) = grouping_size.filter(|&s| s > 0) {
        int = group_digits(&int, Some(&(df.grouping.to_string(), size)));
    }
    let mut out = pre;
    out.push_str(&int);
    if !frac.is_empty() {
        out.push(df.decimal);
        out.push_str(&frac);
    }
    if int.is_empty() && frac.is_empty() {
        out.push('0');
    }
    out.push_str(&suf);
    out
}

// ----------------------------------------------------------------------
// Serializing

#[derive(Clone, Copy, PartialEq)]
enum Method {
    Xml,
    Html,
    Text,
}

const HTML_VOID: &[&str] = &[
    "area", "base", "basefont", "br", "col", "embed", "frame", "hr", "img", "input", "isindex",
    "link", "meta", "param", "source", "track", "wbr",
];

fn serialize(nodes: &[FragNode], o: &Output) -> String {
    let method = match o.method.as_deref() {
        Some("html") => Method::Html,
        Some("text") => Method::Text,
        Some(_) => Method::Xml,
        None => {
            // HTML when the first element is <html> with only whitespace before it.
            let mut html = false;
            for n in nodes {
                match n {
                    FragNode::Text { text, .. } if text.chars().all(is_xml_space) => {}
                    FragNode::Element(e) => {
                        html = e.ns.is_none() && e.local.eq_ignore_ascii_case("html");
                        break;
                    }
                    FragNode::Text { .. } => break,
                    _ => {}
                }
            }
            if html { Method::Html } else { Method::Xml }
        }
    };
    if method == Method::Text {
        return Fragment {
            nodes: nodes.to_vec(),
        }
        .string_value();
    }
    let mut s = Serializer {
        out: String::new(),
        method,
        indent: o.indent.unwrap_or(method == Method::Html),
        cdata: &o.cdata,
        scope: Vec::new(),
        generated: 0,
    };
    if method == Method::Xml && !o.omit_declaration {
        s.out.push_str(&format!(
            "<?xml version=\"1.0\" encoding=\"{}\"",
            o.encoding.as_deref().unwrap_or("UTF-8")
        ));
        if let Some(sa) = &o.standalone {
            s.out.push_str(&format!(" standalone=\"{sa}\""));
        }
        s.out.push_str("?>");
        if !matches!(nodes.first(), Some(FragNode::Text { .. })) {
            s.out.push('\n');
        }
    }
    if o.doctype_system.is_some() || (method == Method::Html && o.doctype_public.is_some()) {
        let root = nodes
            .iter()
            .find_map(|n| match n {
                FragNode::Element(e) => Some(qualified(e.prefix.as_deref(), &e.local)),
                _ => None,
            })
            .unwrap_or_else(|| "html".into());
        s.out.push_str(&format!("<!DOCTYPE {root}"));
        match (&o.doctype_public, &o.doctype_system) {
            (Some(p), Some(sys)) => s.out.push_str(&format!(" PUBLIC \"{p}\" \"{sys}\"")),
            (Some(p), None) => s.out.push_str(&format!(" PUBLIC \"{p}\"")),
            (None, Some(sys)) => s.out.push_str(&format!(" SYSTEM \"{sys}\"")),
            (None, None) => {}
        }
        s.out.push_str(">\n");
    }
    let top_text = nodes.iter().any(|n| matches!(n, FragNode::Text { .. }));
    let indent = s.indent;
    s.children(nodes, 0, indent && !top_text, indent, false, false);
    if s.indent && !s.out.ends_with('\n') {
        s.out.push('\n');
    }
    s.out
}

fn qualified(prefix: Option<&str>, local: &str) -> String {
    match prefix {
        Some(p) if !p.is_empty() => format!("{p}:{local}"),
        _ => local.to_owned(),
    }
}

fn escape_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

fn escape_attr(s: &str, html: bool, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' if !html => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\n' if !html => out.push_str("&#10;"),
            '\r' if !html => out.push_str("&#13;"),
            '\t' if !html => out.push_str("&#9;"),
            c => out.push(c),
        }
    }
}

struct Serializer<'o> {
    out: String,
    method: Method,
    indent: bool,
    cdata: &'o [(Option<String>, String)],
    /// In-scope namespace bindings, innermost last.
    scope: Vec<(Option<String>, String)>,
    generated: usize,
}

impl Serializer<'_> {
    fn lookup(&self, prefix: Option<&str>) -> Option<&str> {
        if prefix == Some("xml") {
            return Some(XML_NS);
        }
        self.scope
            .iter()
            .rev()
            .find(|(p, _)| p.as_deref() == prefix)
            .map(|(_, u)| u.as_str())
    }

    fn newline(&mut self, depth: usize) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        for _ in 0..depth {
            self.out.push_str("  ");
        }
    }

    /// Writes children; `indent` puts each element on its own line, and
    /// `allow` says whether indenting is allowed deeper down.
    fn children(
        &mut self,
        nodes: &[FragNode],
        depth: usize,
        indent: bool,
        allow: bool,
        raw_text: bool,
        cdata: bool,
    ) {
        for n in nodes {
            match n {
                FragNode::Element(e) => {
                    if indent {
                        self.newline(depth);
                    }
                    self.element(e, depth, allow);
                }
                FragNode::Text { text, raw } => {
                    if *raw || raw_text {
                        self.out.push_str(text);
                    } else if cdata && self.method == Method::Xml {
                        self.out.push_str("<![CDATA[");
                        self.out.push_str(&text.replace("]]>", "]]]]><![CDATA[>"));
                        self.out.push_str("]]>");
                    } else {
                        escape_text(text, &mut self.out);
                    }
                }
                FragNode::Comment(c) => {
                    if indent {
                        self.newline(depth);
                    }
                    self.out.push_str("<!--");
                    self.out.push_str(c);
                    self.out.push_str("-->");
                }
                FragNode::Pi { target, data } => {
                    if indent {
                        self.newline(depth);
                    }
                    self.out.push_str("<?");
                    self.out.push_str(target);
                    if !data.is_empty() {
                        self.out.push(' ');
                        self.out.push_str(data);
                    }
                    self.out.push_str(if self.method == Method::Html {
                        ">"
                    } else {
                        "?>"
                    });
                }
            }
        }
    }

    fn bind(&mut self, decls: &mut Vec<(Option<String>, String)>, prefix: Option<&str>, uri: &str) {
        if self.lookup(prefix).unwrap_or("") != uri {
            self.scope.push((prefix.map(str::to_owned), uri.to_owned()));
            decls.push((prefix.map(str::to_owned), uri.to_owned()));
        }
    }

    fn element(&mut self, e: &FragElement, depth: usize, allow: bool) {
        let mark = self.scope.len();
        let mut decls: Vec<(Option<String>, String)> = Vec::new();
        // The element's own name comes first, so its prefix binding wins.
        let mut prefix = e.prefix.clone().filter(|p| !p.is_empty());
        match &e.ns {
            Some(uri) => {
                if prefix.as_deref() == Some("xml") && uri != XML_NS {
                    prefix = None;
                }
                if prefix.as_deref() != Some("xml") {
                    self.bind(&mut decls, prefix.as_deref(), uri);
                }
            }
            None => {
                prefix = None;
                self.bind(&mut decls, None, "");
            }
        }
        for (p, u) in &e.ns_decls {
            if p.as_deref() == Some("xml") || decls.iter().any(|(dp, _)| dp == p) {
                continue;
            }
            if p.is_none() && e.ns.is_none() {
                continue; // An element in no namespace keeps xmlns="".
            }
            self.bind(&mut decls, p.as_deref(), u);
        }
        let mut attrs = Vec::with_capacity(e.attrs.len());
        for a in &e.attrs {
            let name = match &a.ns {
                None => a.local.clone(),
                Some(uri) if uri == XML_NS => format!("xml:{}", a.local),
                Some(uri) => {
                    let wanted = a.prefix.clone().filter(|p| !p.is_empty() && p != "xml");
                    let p = match wanted {
                        Some(p) if self.lookup(Some(&p)) == Some(uri.as_str()) => p,
                        Some(p) if !decls.iter().any(|(dp, _)| dp.as_deref() == Some(&p)) => {
                            self.bind(&mut decls, Some(&p), uri);
                            p
                        }
                        _ => {
                            let existing = self
                                .scope
                                .iter()
                                .rev()
                                .find(|(sp, su)| {
                                    su == uri
                                        && sp.is_some()
                                        && self.lookup(sp.as_deref()) == Some(su.as_str())
                                })
                                .and_then(|(sp, _)| sp.clone());
                            match existing {
                                Some(p) => p,
                                None => {
                                    let p = loop {
                                        let p = format!("ns{}", self.generated);
                                        self.generated += 1;
                                        if self.lookup(Some(&p)).is_none() {
                                            break p;
                                        }
                                    };
                                    self.bind(&mut decls, Some(&p), uri);
                                    p
                                }
                            }
                        }
                    };
                    format!("{p}:{}", a.local)
                }
            };
            attrs.push((name, &a.value));
        }
        let html = self.method == Method::Html;
        let name = qualified(prefix.as_deref(), &e.local);
        self.out.push('<');
        self.out.push_str(&name);
        for (p, u) in &decls {
            match p {
                Some(p) => self.out.push_str(&format!(" xmlns:{p}=\"")),
                None => self.out.push_str(" xmlns=\""),
            }
            escape_attr(u, false, &mut self.out);
            self.out.push('"');
        }
        for (n, v) in &attrs {
            self.out.push(' ');
            self.out.push_str(n);
            self.out.push_str("=\"");
            escape_attr(v, html, &mut self.out);
            self.out.push('"');
        }
        let lower = e.local.to_ascii_lowercase();
        let html_element = html && e.ns.is_none();
        if html_element && HTML_VOID.contains(&lower.as_str()) {
            self.out.push('>');
            self.scope.truncate(mark);
            return;
        }
        if e.children.is_empty() && !html_element {
            self.out.push_str("/>");
            self.scope.truncate(mark);
            return;
        }
        self.out.push('>');
        let preserve = e.attrs.iter().any(|a| {
            a.ns.as_deref() == Some(XML_NS) && a.local == "space" && a.value == "preserve"
        }) || (html_element
            && matches!(lower.as_str(), "pre" | "script" | "style" | "textarea"));
        let has_text = e
            .children
            .iter()
            .any(|c| matches!(c, FragNode::Text { .. }));
        let child_allow = allow && !preserve;
        let child_indent = child_allow && !has_text;
        let raw = html_element && matches!(lower.as_str(), "script" | "style");
        let cdata = self
            .cdata
            .iter()
            .any(|(ns, l)| *ns == e.ns && *l == e.local);
        self.children(
            &e.children,
            depth + 1,
            child_indent,
            child_allow,
            raw,
            cdata,
        );
        if child_indent && !e.children.is_empty() {
            self.newline(depth);
        }
        self.out.push_str("</");
        self.out.push_str(&name);
        self.out.push('>');
        self.scope.truncate(mark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOKS: &str = r#"<?xml version="1.0"?>
<library>
  <book id="b1" year="2015"><title>Rust</title><author>Ann</author><price>30</price></book>
  <book id="b2" year="1999"><title>XML</title><author>Bob</author><price>25.5</price></book>
  <book id="b3" year="2015"><title>Egui</title><author>Ann</author><price>40</price></book>
</library>"#;

    fn sheet(body: &str) -> String {
        format!(
            r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
<xsl:output method="xml" omit-xml-declaration="yes"/>
{body}
</xsl:stylesheet>"#
        )
    }

    fn run_on(doc: &str, body: &str) -> String {
        transform(doc, &sheet(body)).unwrap_or_else(|e| panic!("{e:?}"))
    }

    fn run(body: &str) -> String {
        run_on(BOOKS, body)
    }

    fn err(body: &str) -> ToolError {
        transform(BOOKS, &sheet(body)).unwrap_err()
    }

    #[test]
    fn templates_value_of_and_apply_templates() {
        let out = run(
            r#"<xsl:template match="/"><list><xsl:apply-templates select="//book"/></list></xsl:template>
<xsl:template match="book"><item id="{@id}"><xsl:value-of select="title"/></item></xsl:template>"#,
        );
        assert_eq!(
            out,
            r#"<list><item id="b1">Rust</item><item id="b2">XML</item><item id="b3">Egui</item></list>"#
        );
    }

    #[test]
    fn built_in_rules_copy_text() {
        let out = transform("<a>x<b>y</b><!--c--><?p q?>z</a>", &sheet("")).unwrap();
        assert_eq!(out, "xyz");
    }

    #[test]
    fn priorities_and_conflicts() {
        let out = run(
            r#"<xsl:template match="/"><r><xsl:apply-templates select="//book"/></r></xsl:template>
<xsl:template match="*">[any]</xsl:template>
<xsl:template match="book">[book]</xsl:template>
<xsl:template match="book[@year='1999']">[old]</xsl:template>
<xsl:template match="library/book[3]" priority="2">[third]</xsl:template>"#,
        );
        assert_eq!(out, "<r>[book][old][third]</r>");
    }

    #[test]
    fn modes_and_union_patterns() {
        let out = run(
            r#"<xsl:template match="/"><r><xsl:apply-templates select="//book[1]/*" mode="m"/></r></xsl:template>
<xsl:template match="title | author" mode="m">(<xsl:value-of select="."/>)</xsl:template>
<xsl:template match="price" mode="m"/>
<xsl:template match="title">WRONG</xsl:template>"#,
        );
        assert_eq!(out, "<r>(Rust)(Ann)</r>");
    }

    #[test]
    fn sorting() {
        let out = run(
            r#"<xsl:template match="/"><xsl:for-each select="//book"><xsl:sort select="price" data-type="number" order="descending"/><xsl:value-of select="title"/>,</xsl:for-each>|<xsl:apply-templates select="//book"><xsl:sort select="author"/><xsl:sort select="title"/></xsl:apply-templates></xsl:template>
<xsl:template match="book"><xsl:value-of select="title"/>;</xsl:template>"#,
        );
        assert_eq!(out, "Egui,Rust,XML,|Egui;Rust;XML;");
        let out = run_on(
            "<r><w>b</w><w>B</w><w>a</w></r>",
            r#"<xsl:template match="/"><xsl:for-each select="//w"><xsl:sort case-order="upper-first"/><xsl:value-of select="."/></xsl:for-each></xsl:template>"#,
        );
        assert_eq!(out, "aBb");
    }

    #[test]
    fn params_and_call_template() {
        let out = run(r#"<xsl:param name="sep" select="'/'"/>
<xsl:template match="/"><xsl:call-template name="join"><xsl:with-param name="a" select="'x'"/></xsl:call-template>
<xsl:apply-templates select="//book[1]"><xsl:with-param name="n" select="7"/></xsl:apply-templates></xsl:template>
<xsl:template name="join"><xsl:param name="a"/><xsl:param name="b">dflt</xsl:param><xsl:value-of select="concat($a, $sep, $b)"/></xsl:template>
<xsl:template match="book"><xsl:param name="n" select="0"/>[<xsl:value-of select="$n * 2"/>]</xsl:template>"#);
        assert_eq!(out, "x/dflt[14]");
    }

    #[test]
    fn recursion_with_params() {
        let out = run(
            r#"<xsl:template match="/"><xsl:call-template name="count"><xsl:with-param name="n" select="5"/></xsl:call-template></xsl:template>
<xsl:template name="count"><xsl:param name="n"/><xsl:if test="$n > 0"><xsl:value-of select="$n"/><xsl:call-template name="count"><xsl:with-param name="n" select="$n - 1"/></xsl:call-template></xsl:if></xsl:template>"#,
        );
        assert_eq!(out, "54321");
    }

    #[test]
    fn variables_and_result_tree_fragments() {
        let out = run(r#"<xsl:variable name="total" select="sum(//price)"/>
<xsl:variable name="label">Total: <b>sum</b></xsl:variable>
<xsl:variable name="later" select="$total * 2"/>
<xsl:template match="/"><xsl:variable name="n" select="count(//book)"/><r><xsl:value-of select="$label"/>=<xsl:value-of select="$total"/>/<xsl:value-of select="$n"/>/<xsl:value-of select="$later"/><xsl:copy-of select="$label"/><xsl:if test="$label">!</xsl:if></r></xsl:template>"#);
        assert_eq!(out, "<r>Total: sum=95.5/3/191Total: <b>sum</b>!</r>");
    }

    #[test]
    fn choose_if_and_for_each() {
        let out = run(
            r#"<xsl:template match="/"><xsl:for-each select="//book"><xsl:choose><xsl:when test="price &gt; 35">expensive</xsl:when><xsl:when test="@year &lt; 2000">old</xsl:when><xsl:otherwise>ok</xsl:otherwise></xsl:choose><xsl:if test="position() != last()">,</xsl:if></xsl:for-each></xsl:template>"#,
        );
        assert_eq!(out, "ok,old,expensive");
    }

    #[test]
    fn copy_and_copy_of() {
        // The identity transform, with one change.
        let doc = r#"<a xmlns:p="urn:p" x="1"><p:b y="2">t<!--c--><?pi d?></p:b><c/></a>"#;
        let out = run_on(
            doc,
            r#"<xsl:template match="@*|node()"><xsl:copy><xsl:apply-templates select="@*|node()"/></xsl:copy></xsl:template>
<xsl:template match="c"><d/></xsl:template>"#,
        );
        assert_eq!(
            out,
            r#"<a xmlns:p="urn:p" x="1"><p:b y="2">t<!--c--><?pi d?></p:b><d/></a>"#
        );
        let out = run_on(
            doc,
            r#"<xsl:template match="/"><r><xsl:copy-of select="//*[local-name()='b']"/><xsl:copy-of select="count(//c)"/></r></xsl:template>"#,
        );
        assert_eq!(
            out,
            r#"<r><p:b xmlns:p="urn:p" y="2">t<!--c--><?pi d?></p:b>1</r>"#
        );
    }

    #[test]
    fn element_attribute_comment_pi_text() {
        let out = run(
            r#"<xsl:template match="/"><xsl:element name="{name(*)}-x"><xsl:attribute name="n"><xsl:value-of select="count(//book)"/></xsl:attribute><xsl:comment> note -- here </xsl:comment><xsl:processing-instruction name="go">now</xsl:processing-instruction><xsl:text>  a &lt; b  </xsl:text><xsl:text disable-output-escaping="yes">&lt;raw/&gt;</xsl:text></xsl:element></xsl:template>"#,
        );
        assert_eq!(
            out,
            "<library-x n=\"3\"><!-- note - - here --><?go now?>  a &lt; b  <raw/></library-x>"
        );
    }

    #[test]
    fn namespaces_in_output() {
        let out = run(
            r#"<xsl:template match="/"><h:root xmlns:h="urn:h"><xsl:element name="h:kid"/><xsl:element name="other" namespace="urn:o"><xsl:attribute name="q:a" namespace="urn:q">1</xsl:attribute></xsl:element><plain/></h:root></xsl:template>"#,
        );
        assert_eq!(
            out,
            r#"<h:root xmlns:h="urn:h"><h:kid/><other xmlns="urn:o" xmlns:q="urn:q" q:a="1"/><plain/></h:root>"#
        );
        // A default namespace on the stylesheet's literal elements.
        let out = transform(
            BOOKS,
            r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns="urn:d" xmlns:x="urn:x" exclude-result-prefixes="x">
<xsl:output omit-xml-declaration="yes"/>
<xsl:template match="/"><top><xsl:element name="e"/></top></xsl:template></xsl:stylesheet>"#,
        )
        .unwrap();
        assert_eq!(out, r#"<top xmlns="urn:d"><e/></top>"#);
    }

    #[test]
    fn source_namespaces_need_prefixes() {
        let doc = r#"<c:cat xmlns:c="urn:cat"><c:item>1</c:item><c:item>2</c:item></c:cat>"#;
        let out = transform(
            doc,
            r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:k="urn:cat" exclude-result-prefixes="k">
<xsl:output method="text"/>
<xsl:template match="/"><xsl:value-of select="count(//k:item)"/>,<xsl:value-of select="count(//item)"/></xsl:template></xsl:stylesheet>"#,
        )
        .unwrap();
        assert_eq!(out, "2,0");
    }

    #[test]
    fn attribute_sets() {
        let out = run(
            r#"<xsl:attribute-set name="base"><xsl:attribute name="a">1</xsl:attribute></xsl:attribute-set>
<xsl:attribute-set name="more" use-attribute-sets="base"><xsl:attribute name="b"><xsl:value-of select="count(//book)"/></xsl:attribute></xsl:attribute-set>
<xsl:template match="/"><x xsl:use-attribute-sets="more" a="override"/><xsl:element name="y" use-attribute-sets="base"/></xsl:template>"#,
        );
        assert_eq!(out, r#"<x a="override" b="3"/><y a="1"/>"#);
    }

    #[test]
    fn keys_ids_and_current() {
        let out = run(r#"<xsl:key name="by-author" match="book" use="author"/>
<xsl:template match="/"><xsl:for-each select="//book[generate-id() = generate-id(key('by-author', author)[1])]"><xsl:value-of select="author"/>:<xsl:value-of select="count(key('by-author', author))"/>;</xsl:for-each>
<xsl:for-each select="//book"><xsl:value-of select="count(//book[author = current()/author])"/></xsl:for-each></xsl:template>"#);
        assert_eq!(out, "Ann:2;Bob:1;212");
    }

    #[test]
    fn format_number_cases() {
        let df = DecimalFormat::default();
        assert_eq!(format_decimal(1234567.891, "#,##0.00", &df), "1,234,567.89");
        assert_eq!(format_decimal(0.5, "#.00", &df), ".50");
        assert_eq!(format_decimal(3.0, "000", &df), "003");
        assert_eq!(format_decimal(-2.5, "0.0", &df), "-2.5");
        assert_eq!(format_decimal(-2.0, "0;(0)", &df), "(2)");
        assert_eq!(format_decimal(0.256, "0%", &df), "26%");
        assert_eq!(format_decimal(12.0, "#.##", &df), "12");
        assert_eq!(format_decimal(f64::NAN, "0", &df), "NaN");
        assert_eq!(format_decimal(f64::INFINITY, "0", &df), "Infinity");
        let out = run(
            r#"<xsl:decimal-format name="eu" decimal-separator="," grouping-separator="."/>
<xsl:template match="/"><xsl:value-of select="format-number(1234.5, '#.##0,00', 'eu')"/>|<xsl:value-of select="format-number(sum(//price), '0.0')"/></xsl:template>"#,
        );
        assert_eq!(out, "1.234,50|95.5");
    }

    #[test]
    fn numbering() {
        let out = run(
            r#"<xsl:template match="/"><xsl:for-each select="//book"><xsl:number/>.<xsl:number format="a"/>.<xsl:number format="I"/>.<xsl:number value="position() * 10" format="001"/><xsl:text> </xsl:text></xsl:for-each></xsl:template>"#,
        );
        assert_eq!(out, "1.a.I.010 2.b.II.020 3.c.III.030 ");
        let doc = "<d><s><t/><t/></s><s><t/></s></d>";
        let out = run_on(
            doc,
            r#"<xsl:template match="/"><xsl:for-each select="//t"><xsl:number level="multiple" count="s|t" format="1.1"/>;<xsl:number level="any"/>,</xsl:for-each></xsl:template>"#,
        );
        assert_eq!(out, "1.1;1,1.2;2,2.1;3,");
        assert_eq!(format_number_list(&[27.0], "(A)", None), "(AA)");
        assert_eq!(
            format_number_list(&[1234567.0], "1", Some(&(",".into(), 3))),
            "1,234,567"
        );
    }

    #[test]
    fn messages() {
        let t = transform_with_messages(
            BOOKS,
            &sheet(r#"<xsl:template match="/"><xsl:message>seen <xsl:value-of select="count(//book)"/></xsl:message>done</xsl:template>"#),
        )
        .unwrap();
        assert_eq!(t.output, "done");
        assert_eq!(t.messages, ["seen 3"]);
        let e = err(
            r#"<xsl:template match="/"><xsl:message terminate="yes">bad input</xsl:message></xsl:template>"#,
        );
        assert!(e.message.contains("stopped: bad input"), "{e:?}");
    }

    #[test]
    fn strip_and_preserve_space() {
        let doc = "<r>\n  <a> </a>\n  <p> </p>\n</r>";
        let body = r#"<xsl:strip-space elements="*"/><xsl:preserve-space elements="p"/>
<xsl:template match="/"><xsl:value-of select="count(/r/node())"/>|<xsl:value-of select="count(//a/node())"/>|<xsl:value-of select="count(//p/node())"/></xsl:template>"#;
        assert_eq!(run_on(doc, body), "2|0|1");
        assert_eq!(
            run_on(
                doc,
                r#"<xsl:template match="/"><xsl:value-of select="count(/r/node())"/></xsl:template>"#
            ),
            "5"
        );
    }

    #[test]
    fn output_methods_and_indent() {
        let xml = transform(
            BOOKS,
            r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
<xsl:output method="xml" indent="yes" encoding="ISO-8859-1"/>
<xsl:template match="/"><list><xsl:for-each select="//book[position() &lt; 3]"><b><t><xsl:value-of select="title"/></t></b></xsl:for-each><m>a<i>b</i></m></list></xsl:template></xsl:stylesheet>"#,
        )
        .unwrap();
        assert_eq!(
            xml,
            "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<list>\n  <b>\n    <t>Rust</t>\n  </b>\n  <b>\n    <t>XML</t>\n  </b>\n  <m>a<i>b</i></m>\n</list>\n"
        );
        assert!(roxmltree::Document::parse(&xml).is_ok());
        let html = transform(
            BOOKS,
            r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
<xsl:template match="/"><html><head><meta charset="utf-8"/><script>if (a &lt; b) {}</script></head><body><p>A &amp; B<br/></p></body></html></xsl:template></xsl:stylesheet>"#,
        )
        .unwrap();
        assert_eq!(
            html,
            "<html>\n  <head>\n    <meta charset=\"utf-8\">\n    <script>if (a < b) {}</script>\n  </head>\n  <body>\n    <p>A &amp; B<br></p>\n  </body>\n</html>\n"
        );
        let text = transform(
            BOOKS,
            r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
<xsl:output method="text"/>
<xsl:template match="book"><xsl:value-of select="title"/> &amp; <xsl:text>&#10;</xsl:text></xsl:template></xsl:stylesheet>"#,
        )
        .unwrap();
        assert_eq!(text, "\n  Rust & \n\n  XML & \n\n  Egui & \n\n");
        let dt = transform(
            "<a/>",
            r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
<xsl:output doctype-system="a.dtd" cdata-section-elements="c"/>
<xsl:template match="/"><a><c>x &lt; y</c></a></xsl:template></xsl:stylesheet>"#,
        )
        .unwrap();
        assert_eq!(
            dt,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE a SYSTEM \"a.dtd\">\n<a><c><![CDATA[x < y]]></c></a>"
        );
    }

    #[test]
    fn simplified_stylesheet() {
        let out = transform(
            BOOKS,
            r#"<html xsl:version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><p><xsl:value-of select="count(//book)"/></p></html>"#,
        )
        .unwrap();
        assert_eq!(out, "<html>\n  <p>3</p>\n</html>\n");
    }

    #[test]
    fn functions() {
        let out = run(
            r#"<xsl:template match="/"><xsl:value-of select="system-property('xsl:version')"/>|<xsl:value-of select="function-available('key')"/>|<xsl:value-of select="element-available('xsl:number')"/>|<xsl:value-of select="string-length(generate-id(//book[2])) &gt; 0"/>|<xsl:value-of select="generate-id(//book[2]) = generate-id(//book[@id='b2'])"/></xsl:template>"#,
        );
        assert_eq!(out, "1|true|true|true|true");
    }

    #[test]
    fn errors() {
        let e = transform(BOOKS, "<x/>").unwrap_err();
        assert!(e.message.contains("isn't an XSLT stylesheet"));
        let e = transform(BOOKS, "<xsl:stylesheet").unwrap_err();
        assert!(e.message.contains("well-formed"), "{e:?}");
        let e = transform("<a>", &sheet("")).unwrap_err();
        assert!(e.message.contains("document"), "{e:?}");
        let e = err(r#"<xsl:template match="/"><xsl:value-of select="count(("/></xsl:template>"#);
        assert!(e.message.contains("xsl:value-of select"), "{e:?}");
        assert!(e.message.contains("stylesheet line 3"), "{e:?}");
        let e = err(r#"<xsl:template match="/"><xsl:value-of/></xsl:template>"#);
        assert!(e.message.contains("needs a select"), "{e:?}");
        let e = err(r#"<xsl:template match="/"><xsl:call-template name="nope"/></xsl:template>"#);
        assert!(e.message.contains("no template named"), "{e:?}");
        let e =
            err(r#"<xsl:template match="/"><xsl:value-of select="$undefined"/></xsl:template>"#);
        assert!(e.message.contains("isn't declared"), "{e:?}");
        let e = err(
            r#"<xsl:template match="/"><xsl:value-of select="document('x.xml')"/></xsl:template>"#,
        );
        assert!(e.message.contains("document() isn't supported"), "{e:?}");
        let e = err(r#"<xsl:template match="/"><xsl:apply-templates select="."/></xsl:template>"#);
        assert!(e.message.contains("too deeply"), "{e:?}");
        let e = err(r#"<xsl:template match="count(x)"/>"#);
        assert!(e.message.contains("match pattern"), "{e:?}");
        let e = err(r#"<xsl:include href="x.xsl"/>"#);
        assert!(e.message.contains("isn't supported"), "{e:?}");
        let e = err(r#"<xsl:template match="/"><a b="{oops"/></xsl:template>"#);
        assert!(e.message.contains("never closed"), "{e:?}");
        let e = err(r#"<xsl:variable name="v" select="$v"/>"#);
        assert!(e.message.contains("depends on itself"), "{e:?}");
        let e = err(
            r#"<xsl:template match="/"><xsl:variable name="f"><a/></xsl:variable><xsl:value-of select="$f/a"/></xsl:template>"#,
        );
        assert!(e.message.contains("result tree fragment"), "{e:?}");
    }

    #[test]
    fn output_is_well_formed_when_indented() {
        let out = transform(
            BOOKS,
            r#"<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
<xsl:output indent="yes"/>
<xsl:template match="@*|node()"><xsl:copy><xsl:apply-templates select="@*|node()"/></xsl:copy></xsl:template></xsl:stylesheet>"#,
        )
        .unwrap();
        let doc = roxmltree::Document::parse(&out).unwrap();
        assert_eq!(
            doc.descendants().filter(|n| n.has_tag_name("book")).count(),
            3
        );
    }
}
