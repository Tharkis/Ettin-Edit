//! An XPath 1.0 evaluator over `roxmltree` documents, for the XPath panel.
//!
//! It covers the whole XPath 1.0 language (all axes but `namespace`, every
//! core function, plus `ends-with`, `lower-case` and `upper-case`). Namespace
//! prefixes are the ones declared in the document; elements in a default
//! namespace match unprefixed names, so `//item` works without setting up a
//! prefix first.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::rc::Rc;

use roxmltree::{Document, Node, NodeType};

use super::ToolError;

// ----------------------------------------------------------------------
// Results

/// One node an expression selected, ready to list and jump to.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeResult {
    /// Byte range in the document text.
    pub range: Range<usize>,
    /// What kind of node: `<item>`, `@id`, `text()`, `comment()`.
    pub label: String,
    /// The node's string value.
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Nodes(Vec<NodeResult>),
    Number(f64),
    String(String),
    Boolean(bool),
}

/// Evaluates `expr` against the document, with the root as the context node.
pub fn evaluate(doc: &Document, expr: &str) -> Result<Outcome, ToolError> {
    let parsed = Parser::new(expr)?.parse()?;
    let ev = Evaluator::new(doc);
    let root = XNode::Node(doc.root());
    let value = ev
        .eval(
            &parsed,
            &Ctx {
                node: root,
                pos: 1,
                size: 1,
                env: None,
            },
        )
        .map_err(ToolError::new)?;
    Ok(match value {
        Value::Nodes(nodes) => Outcome::Nodes(nodes.into_iter().map(|n| ev.result(n)).collect()),
        Value::Num(n) => Outcome::Number(n),
        Value::Str(s) => Outcome::String(s),
        Value::Bool(b) => Outcome::Boolean(b),
        Value::Frag(f) => Outcome::String(f.string_value()),
    })
}

pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 { "Infinity" } else { "-Infinity" }.into()
    } else if n == n.trunc() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

// ----------------------------------------------------------------------
// Compiled expressions, for XSLT

const FRAG_PATH: &str = "A result tree fragment can't be navigated or filtered in XSLT 1.0; use it as a string or with xsl:copy-of.";

/// A result tree fragment: nodes an XSLT template built, held as a value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fragment {
    pub nodes: Vec<FragNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FragNode {
    Element(FragElement),
    /// Text; `raw` text is written without escaping.
    Text {
        text: String,
        raw: bool,
    },
    Comment(String),
    Pi {
        target: String,
        data: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FragElement {
    pub prefix: Option<String>,
    pub local: String,
    pub ns: Option<String>,
    /// Namespace declarations to carry (prefix, URI); `None` is the default namespace.
    pub ns_decls: Vec<(Option<String>, String)>,
    pub attrs: Vec<FragAttr>,
    pub children: Vec<FragNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FragAttr {
    pub prefix: Option<String>,
    pub local: String,
    pub ns: Option<String>,
    pub value: String,
}

impl Fragment {
    pub fn string_value(&self) -> String {
        fn walk(nodes: &[FragNode], out: &mut String) {
            for n in nodes {
                match n {
                    FragNode::Text { text, .. } => out.push_str(text),
                    FragNode::Element(e) => walk(&e.children, out),
                    _ => {}
                }
            }
        }
        let mut out = String::new();
        walk(&self.nodes, &mut out);
        out
    }
}

/// Variables and extra functions an expression can use (XSLT supplies them).
pub trait Env<'a, 'i> {
    /// The value of `$name`, or an error naming the problem.
    fn variable(&self, name: &str) -> Result<Value<'a, 'i>, String>;
    /// Calls a function outside the core library. `None` means it doesn't exist.
    fn function(
        &self,
        name: &str,
        args: &[Value<'a, 'i>],
        context: XNode<'a, 'i>,
    ) -> Option<Result<Value<'a, 'i>, String>>;
}

/// An expression compiled once and evaluated many times, with namespace
/// prefixes resolved when compiling (unprefixed names mean no namespace).
#[derive(Clone, Debug, PartialEq)]
pub struct XPath {
    expr: Expr,
}

impl XPath {
    /// Compiles `src`; `resolve` maps a prefix to its namespace URI, and
    /// `extra_functions` are function names allowed besides the core library.
    pub fn compile(
        src: &str,
        resolve: &dyn Fn(&str) -> Option<String>,
        extra_functions: &[&str],
    ) -> Result<Self, ToolError> {
        let mut expr = Parser::with_functions(src, extra_functions)?.parse()?;
        resolve_names(&mut expr, resolve).map_err(ToolError::new)?;
        Ok(Self { expr })
    }

    /// Compiles an XSLT match pattern.
    pub fn compile_pattern(
        src: &str,
        resolve: &dyn Fn(&str) -> Option<String>,
        extra_functions: &[&str],
    ) -> Result<Self, ToolError> {
        let x = Self::compile(src, resolve, extra_functions)?;
        if !is_pattern(&x.expr) {
            return Err(ToolError::new(format!(
                "'{src}' isn't a valid match pattern."
            )));
        }
        Ok(x)
    }

    /// The alternatives of a union pattern (`a | b`), each on its own.
    pub fn alternatives(&self) -> Vec<XPath> {
        fn walk(e: &Expr, out: &mut Vec<XPath>) {
            match e {
                Expr::Union(a, b) => {
                    walk(a, out);
                    walk(b, out);
                }
                other => out.push(XPath {
                    expr: other.clone(),
                }),
            }
        }
        let mut out = Vec::new();
        walk(&self.expr, &mut out);
        out
    }

    /// The default priority of a single (non-union) pattern, per XSLT 1.0 5.5.
    pub fn default_priority(&self) -> f64 {
        match &self.expr {
            Expr::Path(Start::Context, steps)
                if steps.len() == 1
                    && steps[0].predicates.is_empty()
                    && matches!(steps[0].axis, Axis::Child | Axis::Attribute) =>
            {
                match &steps[0].test {
                    Test::Name { .. } | Test::Resolved { .. } | Test::Pi(Some(_)) => 0.0,
                    Test::AnyIn(_) | Test::AnyInUri(_) => -0.25,
                    _ => -0.5,
                }
            }
            _ => 0.5,
        }
    }
}

fn resolve_names(e: &mut Expr, resolve: &dyn Fn(&str) -> Option<String>) -> Result<(), String> {
    let lookup =
        |p: &str| resolve(p).ok_or_else(|| format!("The namespace prefix '{p}' isn't declared."));
    let steps = |steps: &mut Vec<Step>| -> Result<(), String> {
        for s in steps {
            s.test = match std::mem::replace(&mut s.test, Test::Node) {
                Test::Name { prefix, local } => Test::Resolved {
                    uri: prefix.as_deref().map(lookup).transpose()?,
                    local,
                },
                Test::AnyIn(p) => Test::AnyInUri(lookup(&p)?),
                t => t,
            };
            for p in &mut s.predicates {
                resolve_names(p, resolve)?;
            }
        }
        Ok(())
    };
    match e {
        Expr::Or(a, b)
        | Expr::And(a, b)
        | Expr::Compare(_, a, b)
        | Expr::Arith(_, a, b)
        | Expr::Union(a, b) => {
            resolve_names(a, resolve)?;
            resolve_names(b, resolve)?;
        }
        Expr::Negate(a) => resolve_names(a, resolve)?,
        Expr::Path(start, s) => {
            if let Start::Expr(x) = start {
                resolve_names(x, resolve)?;
            }
            steps(s)?;
        }
        Expr::Filter(x, preds) => {
            resolve_names(x, resolve)?;
            for p in preds {
                resolve_names(p, resolve)?;
            }
        }
        Expr::Call(_, args) => {
            for a in args {
                resolve_names(a, resolve)?;
            }
        }
        Expr::Literal(_) | Expr::Number(_) | Expr::Var(_) => {}
    }
    Ok(())
}

fn is_pattern(e: &Expr) -> bool {
    let key_or_id = |e: &Expr| {
        matches!(e, Expr::Call(name, args)
            if (name == "id" || name == "key")
                && args.iter().all(|a| matches!(a, Expr::Literal(_) | Expr::Var(_))))
    };
    match e {
        Expr::Union(a, b) => is_pattern(a) && is_pattern(b),
        Expr::Call(..) => key_or_id(e),
        Expr::Path(start, steps) => {
            let start_ok = match start {
                Start::Root | Start::Context => true,
                Start::Expr(x) => key_or_id(x),
            };
            start_ok
                && (!steps.is_empty() || !matches!(start, Start::Context))
                && steps.iter().all(|s| match s.axis {
                    Axis::Child | Axis::Attribute => true,
                    Axis::DescendantOrSelf => s.test == Test::Node && s.predicates.is_empty(),
                    _ => false,
                })
        }
        _ => false,
    }
}

/// Evaluates compiled expressions and patterns against one document.
pub struct Engine<'a, 'i> {
    ev: Evaluator<'a, 'i>,
}

impl<'a, 'i> Engine<'a, 'i> {
    /// `skip` lists text nodes to treat as absent (stripped whitespace).
    pub fn new(doc: &'a Document<'i>, skip: HashSet<roxmltree::NodeId>) -> Self {
        Self {
            ev: Evaluator {
                doc,
                prefixes: HashMap::new(),
                defaults: HashSet::new(),
                skip: skip.into_iter().map(|id| id.get()).collect(),
            },
        }
    }

    pub fn eval(
        &self,
        x: &XPath,
        node: XNode<'a, 'i>,
        pos: usize,
        size: usize,
        env: &dyn Env<'a, 'i>,
    ) -> Result<Value<'a, 'i>, String> {
        self.ev.eval(
            &x.expr,
            &Ctx {
                node,
                pos,
                size,
                env: Some(env),
            },
        )
    }

    /// True if `node` matches the pattern.
    pub fn matches(
        &self,
        pattern: &XPath,
        node: XNode<'a, 'i>,
        env: &dyn Env<'a, 'i>,
    ) -> Result<bool, String> {
        self.ev.match_pattern(&pattern.expr, node, env)
    }

    pub fn string_value(&self, n: XNode<'a, 'i>) -> String {
        self.ev.string_value(n)
    }

    pub fn string(&self, v: &Value) -> String {
        self.ev.string(v)
    }

    pub fn number(&self, v: &Value) -> f64 {
        self.ev.number(v)
    }

    pub fn boolean(&self, v: &Value) -> bool {
        self.ev.boolean(v)
    }

    pub fn local_name(&self, n: XNode<'a, 'i>) -> &'a str {
        self.ev.local_name(n)
    }

    pub fn namespace(&self, n: XNode<'a, 'i>) -> Option<&'a str> {
        self.ev.namespace(n)
    }

    /// The node's children (without stripped whitespace).
    pub fn children(&self, n: XNode<'a, 'i>) -> Vec<XNode<'a, 'i>> {
        self.ev.axis(n, Axis::Child)
    }

    /// The node's preceding siblings, nearest first.
    pub fn preceding_siblings(&self, n: XNode<'a, 'i>) -> Vec<XNode<'a, 'i>> {
        self.ev.axis(n, Axis::PrecedingSibling)
    }

    pub fn ancestors_or_self(&self, n: XNode<'a, 'i>) -> Vec<XNode<'a, 'i>> {
        self.ev.axis(n, Axis::AncestorOrSelf)
    }

    /// All nodes in document order (with attributes after their element).
    pub fn all_nodes(&self) -> Vec<XNode<'a, 'i>> {
        let mut out = Vec::new();
        for n in self.ev.doc.root().descendants() {
            if n.is_text() && self.ev.skip.contains(&n.id().get()) {
                continue;
            }
            out.push(XNode::Node(n));
            if n.is_element() {
                out.extend((0..n.attributes().len()).map(|i| XNode::Attr(n, i)));
            }
        }
        out
    }
}

impl<'a, 'i> Evaluator<'a, 'i> {
    fn match_pattern(
        &self,
        e: &Expr,
        n: XNode<'a, 'i>,
        env: &dyn Env<'a, 'i>,
    ) -> Result<bool, String> {
        match e {
            Expr::Union(a, b) => {
                Ok(self.match_pattern(a, n, env)? || self.match_pattern(b, n, env)?)
            }
            Expr::Call(..) => self.node_in(e, n, env),
            Expr::Path(start, steps) => self.match_steps(n, start, steps, env),
            _ => Err("This isn't a match pattern.".into()),
        }
    }

    fn node_in(&self, e: &Expr, n: XNode<'a, 'i>, env: &dyn Env<'a, 'i>) -> Result<bool, String> {
        let ctx = Ctx {
            node: n,
            pos: 1,
            size: 1,
            env: Some(env),
        };
        Ok(match self.eval(e, &ctx)? {
            Value::Nodes(nodes) => nodes.contains(&n),
            _ => false,
        })
    }

    fn match_steps(
        &self,
        n: XNode<'a, 'i>,
        start: &Start,
        steps: &[Step],
        env: &dyn Env<'a, 'i>,
    ) -> Result<bool, String> {
        let Some((last, rest)) = steps.split_last() else {
            return match start {
                Start::Root => Ok(matches!(n, XNode::Node(x) if x.is_root())),
                Start::Context => Ok(true),
                Start::Expr(e) => self.node_in(e, n, env),
            };
        };
        if last.axis == Axis::DescendantOrSelf {
            let mut cur = Some(n);
            while let Some(m) = cur {
                if self.match_steps(m, start, rest, env)? {
                    return Ok(true);
                }
                cur = self.axis(m, Axis::Parent).into_iter().next();
            }
            return Ok(false);
        }
        let right_kind = match n {
            XNode::Attr(..) => last.axis == Axis::Attribute,
            XNode::Node(x) => {
                last.axis == Axis::Child && !self.skip.contains(&x.id().get()) && !x.is_root()
            }
        };
        if !right_kind || !self.matches(n, &last.test, last.axis)? {
            return Ok(false);
        }
        let Some(parent) = self.axis(n, Axis::Parent).into_iter().next() else {
            return Ok(false);
        };
        if !last.predicates.is_empty() {
            let mut candidates = Vec::new();
            for m in self.axis(parent, last.axis) {
                if self.matches(m, &last.test, last.axis)? {
                    candidates.push(m);
                }
            }
            if !self
                .filter(candidates, &last.predicates, Some(env))?
                .contains(&n)
            {
                return Ok(false);
            }
        }
        self.match_steps(parent, start, rest, env)
    }
}

// ----------------------------------------------------------------------
// Lexer

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Slash,
    DoubleSlash,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    DoubleDot,
    At,
    Comma,
    DoubleColon,
    Pipe,
    Plus,
    Minus,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `*`, `and`, `or`, `mod`, `div` used as operators.
    Op(&'static str),
    Literal(String),
    Number(f64),
    Var(String),
    /// A name test: `*`, `prefix:*` or a (qualified) name.
    Name(String),
    Function(String),
    NodeType(String),
    Axis(String),
}

impl Tok {
    /// True if a `*` or name after this token is an operator (XPath 1.0, 3.7).
    fn makes_operator(&self) -> bool {
        !matches!(
            self,
            Tok::At
                | Tok::DoubleColon
                | Tok::LParen
                | Tok::LBracket
                | Tok::Comma
                | Tok::Op(_)
                | Tok::Slash
                | Tok::DoubleSlash
                | Tok::Pipe
                | Tok::Plus
                | Tok::Minus
                | Tok::Eq
                | Tok::Ne
                | Tok::Lt
                | Tok::Le
                | Tok::Gt
                | Tok::Ge
        )
    }
}

fn is_ncname_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ncname_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '\u{b7}')
}

fn lex(src: &str) -> Result<Vec<(Tok, usize)>, ToolError> {
    let mut out: Vec<(Tok, usize)> = Vec::new();
    let b = src.as_bytes();
    let mut i = 0;
    while i < src.len() {
        let c = src[i..].chars().next().unwrap_or(' ');
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        let start = i;
        let prev_operand = out.last().is_some_and(|(t, _)| t.makes_operator());
        let two = src.get(i..i + 2).unwrap_or("");
        let tok = match c {
            '/' if two == "//" => {
                i += 2;
                Tok::DoubleSlash
            }
            '/' => {
                i += 1;
                Tok::Slash
            }
            '(' => {
                i += 1;
                Tok::LParen
            }
            ')' => {
                i += 1;
                Tok::RParen
            }
            '[' => {
                i += 1;
                Tok::LBracket
            }
            ']' => {
                i += 1;
                Tok::RBracket
            }
            '.' if two == ".." => {
                i += 2;
                Tok::DoubleDot
            }
            '.' if !b.get(i + 1).is_some_and(u8::is_ascii_digit) => {
                i += 1;
                Tok::Dot
            }
            '@' => {
                i += 1;
                Tok::At
            }
            ',' => {
                i += 1;
                Tok::Comma
            }
            ':' if two == "::" => {
                i += 2;
                Tok::DoubleColon
            }
            '|' => {
                i += 1;
                Tok::Pipe
            }
            '+' => {
                i += 1;
                Tok::Plus
            }
            '-' => {
                i += 1;
                Tok::Minus
            }
            '=' => {
                i += 1;
                Tok::Eq
            }
            '!' if two == "!=" => {
                i += 2;
                Tok::Ne
            }
            '<' if two == "<=" => {
                i += 2;
                Tok::Le
            }
            '<' => {
                i += 1;
                Tok::Lt
            }
            '>' if two == ">=" => {
                i += 2;
                Tok::Ge
            }
            '>' => {
                i += 1;
                Tok::Gt
            }
            '"' | '\'' => {
                let Some(end) = src[i + 1..].find(c) else {
                    return Err(ToolError::at("This string is never closed.", i));
                };
                let s = src[i + 1..i + 1 + end].to_owned();
                i += end + 2;
                Tok::Literal(s)
            }
            '0'..='9' | '.' => {
                while b.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
                if b.get(i) == Some(&b'.') {
                    i += 1;
                    while b.get(i).is_some_and(u8::is_ascii_digit) {
                        i += 1;
                    }
                }
                Tok::Number(src[start..i].parse().unwrap_or(f64::NAN))
            }
            '$' => {
                i += 1;
                let name_start = i;
                while let Some(d) = src[i..]
                    .chars()
                    .next()
                    .filter(|&d| is_ncname_char(d) || d == ':')
                {
                    i += d.len_utf8();
                }
                Tok::Var(src[name_start..i].to_owned())
            }
            '*' if prev_operand => {
                i += 1;
                Tok::Op("*")
            }
            '*' => {
                i += 1;
                Tok::Name("*".into())
            }
            c if is_ncname_start(c) => {
                let ncname = |i: &mut usize| {
                    while let Some(d) = src[*i..].chars().next().filter(|&d| is_ncname_char(d)) {
                        *i += d.len_utf8();
                    }
                };
                ncname(&mut i);
                let first = &src[start..i];
                if prev_operand {
                    match first {
                        "and" | "or" | "mod" | "div" => {
                            out.push((
                                Tok::Op(match first {
                                    "and" => "and",
                                    "or" => "or",
                                    "mod" => "mod",
                                    _ => "div",
                                }),
                                start,
                            ));
                            continue;
                        }
                        _ => {
                            return Err(ToolError::at(
                                format!("Expected an operator, found '{first}'."),
                                start,
                            ));
                        }
                    }
                }
                // A prefix: `p:name` or `p:*` (but not the `::` of an axis).
                if src[i..].starts_with(':') && !src[i..].starts_with("::") {
                    let after = src[i + 1..].chars().next();
                    if after == Some('*') {
                        i += 2;
                    } else if after.is_some_and(is_ncname_start) {
                        i += 1;
                        ncname(&mut i);
                    }
                }
                let name = &src[start..i];
                let rest = src[i..].trim_start();
                if rest.starts_with("::") {
                    Tok::Axis(name.to_owned())
                } else if rest.starts_with('(') {
                    match name {
                        "comment" | "text" | "processing-instruction" | "node" => {
                            Tok::NodeType(name.to_owned())
                        }
                        _ => Tok::Function(name.to_owned()),
                    }
                } else {
                    Tok::Name(name.to_owned())
                }
            }
            _ => return Err(ToolError::at(format!("Unexpected '{c}'."), i)),
        };
        out.push((tok, start));
    }
    Ok(out)
}

// ----------------------------------------------------------------------
// Parser

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    Ancestor,
    AncestorOrSelf,
    Attribute,
    Child,
    Descendant,
    DescendantOrSelf,
    Following,
    FollowingSibling,
    Namespace,
    Parent,
    Preceding,
    PrecedingSibling,
    Itself,
}

impl Axis {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "ancestor" => Axis::Ancestor,
            "ancestor-or-self" => Axis::AncestorOrSelf,
            "attribute" => Axis::Attribute,
            "child" => Axis::Child,
            "descendant" => Axis::Descendant,
            "descendant-or-self" => Axis::DescendantOrSelf,
            "following" => Axis::Following,
            "following-sibling" => Axis::FollowingSibling,
            "namespace" => Axis::Namespace,
            "parent" => Axis::Parent,
            "preceding" => Axis::Preceding,
            "preceding-sibling" => Axis::PrecedingSibling,
            "self" => Axis::Itself,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Test {
    /// `*`
    Any,
    /// `prefix:*`
    AnyIn(String),
    Name {
        prefix: Option<String>,
        local: String,
    },
    Node,
    Text,
    Comment,
    Pi(Option<String>),
    /// A name whose prefix was resolved when compiling (`None`: no namespace).
    Resolved {
        uri: Option<String>,
        local: String,
    },
    /// `prefix:*` with the prefix resolved when compiling.
    AnyInUri(String),
}

#[derive(Clone, Debug, PartialEq)]
struct Step {
    axis: Axis,
    test: Test,
    predicates: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
enum Start {
    Root,
    Context,
    Expr(Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Compare(Cmp, Box<Expr>, Box<Expr>),
    Arith(char, Box<Expr>, Box<Expr>),
    Negate(Box<Expr>),
    Union(Box<Expr>, Box<Expr>),
    Path(Start, Vec<Step>),
    Filter(Box<Expr>, Vec<Expr>),
    Literal(String),
    Number(f64),
    Var(String),
    Call(String, Vec<Expr>),
}

struct Parser<'f> {
    toks: Vec<(Tok, usize)>,
    i: usize,
    len: usize,
    /// Function names allowed besides the core library.
    extra: &'f [&'f str],
}

fn descendant_or_self() -> Step {
    Step {
        axis: Axis::DescendantOrSelf,
        test: Test::Node,
        predicates: Vec::new(),
    }
}

impl<'f> Parser<'f> {
    fn new(src: &str) -> Result<Self, ToolError> {
        Self::with_functions(src, &[])
    }

    fn with_functions(src: &str, extra: &'f [&'f str]) -> Result<Self, ToolError> {
        if src.trim().is_empty() {
            return Err(ToolError::new(
                "Type an XPath expression, like //item[@id='1'].",
            ));
        }
        Ok(Self {
            toks: lex(src)?,
            i: 0,
            len: src.len(),
            extra,
        })
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i).map(|(t, _)| t)
    }

    fn offset(&self) -> usize {
        self.toks.get(self.i).map_or(self.len, |(_, o)| *o)
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn error(&self, expected: &str) -> ToolError {
        match self.peek() {
            Some(_) => ToolError::at(format!("Expected {expected} here."), self.offset()),
            None => ToolError::at(
                format!("Expected {expected}, but the expression ended."),
                self.len,
            ),
        }
    }

    fn parse(mut self) -> Result<Expr, ToolError> {
        let e = self.or()?;
        if self.peek().is_some() {
            return Err(self.error("an operator or the end"));
        }
        Ok(e)
    }

    fn or(&mut self) -> Result<Expr, ToolError> {
        let mut e = self.and()?;
        while self.eat(&Tok::Op("or")) {
            e = Expr::Or(Box::new(e), Box::new(self.and()?));
        }
        Ok(e)
    }

    fn and(&mut self) -> Result<Expr, ToolError> {
        let mut e = self.equality()?;
        while self.eat(&Tok::Op("and")) {
            e = Expr::And(Box::new(e), Box::new(self.equality()?));
        }
        Ok(e)
    }

    fn equality(&mut self) -> Result<Expr, ToolError> {
        let mut e = self.relational()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Eq) => Cmp::Eq,
                Some(Tok::Ne) => Cmp::Ne,
                _ => return Ok(e),
            };
            self.i += 1;
            e = Expr::Compare(op, Box::new(e), Box::new(self.relational()?));
        }
    }

    fn relational(&mut self) -> Result<Expr, ToolError> {
        let mut e = self.additive()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Lt) => Cmp::Lt,
                Some(Tok::Le) => Cmp::Le,
                Some(Tok::Gt) => Cmp::Gt,
                Some(Tok::Ge) => Cmp::Ge,
                _ => return Ok(e),
            };
            self.i += 1;
            e = Expr::Compare(op, Box::new(e), Box::new(self.additive()?));
        }
    }

    fn additive(&mut self) -> Result<Expr, ToolError> {
        let mut e = self.multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Plus) => '+',
                Some(Tok::Minus) => '-',
                _ => return Ok(e),
            };
            self.i += 1;
            e = Expr::Arith(op, Box::new(e), Box::new(self.multiplicative()?));
        }
    }

    fn multiplicative(&mut self) -> Result<Expr, ToolError> {
        let mut e = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op("*")) => '*',
                Some(Tok::Op("div")) => '/',
                Some(Tok::Op("mod")) => '%',
                _ => return Ok(e),
            };
            self.i += 1;
            e = Expr::Arith(op, Box::new(e), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, ToolError> {
        if self.eat(&Tok::Minus) {
            return Ok(Expr::Negate(Box::new(self.unary()?)));
        }
        let mut e = self.path()?;
        while self.eat(&Tok::Pipe) {
            e = Expr::Union(Box::new(e), Box::new(self.path()?));
        }
        Ok(e)
    }

    fn path(&mut self) -> Result<Expr, ToolError> {
        match self.peek() {
            Some(
                Tok::Var(_) | Tok::LParen | Tok::Literal(_) | Tok::Number(_) | Tok::Function(_),
            ) => {
                let primary = self.primary()?;
                let predicates = self.predicates()?;
                let e = if predicates.is_empty() {
                    primary
                } else {
                    Expr::Filter(Box::new(primary), predicates)
                };
                if matches!(self.peek(), Some(Tok::Slash | Tok::DoubleSlash)) {
                    let steps = self.relative_steps(true)?;
                    Ok(Expr::Path(Start::Expr(Box::new(e)), steps))
                } else {
                    Ok(e)
                }
            }
            Some(Tok::Slash) => {
                self.i += 1;
                let steps = if self.starts_step() {
                    self.relative_steps(false)?
                } else {
                    Vec::new()
                };
                Ok(Expr::Path(Start::Root, steps))
            }
            Some(Tok::DoubleSlash) => {
                self.i += 1;
                let mut steps = vec![descendant_or_self()];
                steps.extend(self.relative_steps(false)?);
                Ok(Expr::Path(Start::Root, steps))
            }
            _ => Ok(Expr::Path(Start::Context, self.relative_steps(false)?)),
        }
    }

    fn starts_step(&self) -> bool {
        matches!(
            self.peek(),
            Some(
                Tok::Name(_)
                    | Tok::NodeType(_)
                    | Tok::Axis(_)
                    | Tok::At
                    | Tok::Dot
                    | Tok::DoubleDot
            )
        )
    }

    /// Steps separated by `/` or `//`. With `leading_separator`, the first
    /// step also comes after one (as in `$x/a`).
    fn relative_steps(&mut self, leading_separator: bool) -> Result<Vec<Step>, ToolError> {
        let mut steps = Vec::new();
        let mut need_separator = leading_separator;
        loop {
            if need_separator {
                match self.peek() {
                    Some(Tok::Slash) => self.i += 1,
                    Some(Tok::DoubleSlash) => {
                        self.i += 1;
                        steps.push(descendant_or_self());
                    }
                    _ => return Ok(steps),
                }
            }
            steps.push(self.step()?);
            need_separator = true;
        }
    }

    fn step(&mut self) -> Result<Step, ToolError> {
        if self.eat(&Tok::Dot) {
            return Ok(Step {
                axis: Axis::Itself,
                test: Test::Node,
                predicates: Vec::new(),
            });
        }
        if self.eat(&Tok::DoubleDot) {
            return Ok(Step {
                axis: Axis::Parent,
                test: Test::Node,
                predicates: Vec::new(),
            });
        }
        let axis = match self.peek().cloned() {
            Some(Tok::At) => {
                self.i += 1;
                Axis::Attribute
            }
            Some(Tok::Axis(name)) => {
                let offset = self.offset();
                self.i += 1;
                if !self.eat(&Tok::DoubleColon) {
                    return Err(self.error("'::'"));
                }
                Axis::from_name(&name)
                    .ok_or_else(|| ToolError::at(format!("'{name}' isn't an axis."), offset))?
            }
            _ => Axis::Child,
        };
        let test = match self.peek().cloned() {
            Some(Tok::Name(name)) => {
                self.i += 1;
                if name == "*" {
                    Test::Any
                } else if let Some(prefix) = name.strip_suffix(":*") {
                    Test::AnyIn(prefix.to_owned())
                } else {
                    match name.split_once(':') {
                        Some((p, l)) => Test::Name {
                            prefix: Some(p.to_owned()),
                            local: l.to_owned(),
                        },
                        None => Test::Name {
                            prefix: None,
                            local: name,
                        },
                    }
                }
            }
            Some(Tok::NodeType(kind)) => {
                self.i += 1;
                if !self.eat(&Tok::LParen) {
                    return Err(self.error("'('"));
                }
                let test = match kind.as_str() {
                    "node" => Test::Node,
                    "text" => Test::Text,
                    "comment" => Test::Comment,
                    _ => match self.peek().cloned() {
                        Some(Tok::Literal(target)) => {
                            self.i += 1;
                            Test::Pi(Some(target))
                        }
                        _ => Test::Pi(None),
                    },
                };
                if !self.eat(&Tok::RParen) {
                    return Err(self.error("')'"));
                }
                test
            }
            _ => return Err(self.error("a node name or test")),
        };
        Ok(Step {
            axis,
            test,
            predicates: self.predicates()?,
        })
    }

    fn predicates(&mut self) -> Result<Vec<Expr>, ToolError> {
        let mut out = Vec::new();
        while self.eat(&Tok::LBracket) {
            out.push(self.or()?);
            if !self.eat(&Tok::RBracket) {
                return Err(self.error("']'"));
            }
        }
        Ok(out)
    }

    fn primary(&mut self) -> Result<Expr, ToolError> {
        let tok = self.peek().cloned();
        self.i += 1;
        Ok(match tok {
            Some(Tok::Var(name)) => Expr::Var(name),
            Some(Tok::Literal(s)) => Expr::Literal(s),
            Some(Tok::Number(n)) => Expr::Number(n),
            Some(Tok::LParen) => {
                let e = self.or()?;
                if !self.eat(&Tok::RParen) {
                    return Err(self.error("')'"));
                }
                e
            }
            Some(Tok::Function(name)) => {
                let offset = self.toks[self.i - 1].1;
                if !FUNCTIONS.contains(&name.as_str()) && !self.extra.contains(&name.as_str()) {
                    return Err(ToolError::at(format!("Unknown function {name}()."), offset));
                }
                self.eat(&Tok::LParen);
                let mut args = Vec::new();
                if !self.eat(&Tok::RParen) {
                    loop {
                        args.push(self.or()?);
                        if self.eat(&Tok::RParen) {
                            break;
                        }
                        if !self.eat(&Tok::Comma) {
                            return Err(self.error("',' or ')'"));
                        }
                    }
                }
                Expr::Call(name, args)
            }
            _ => {
                self.i -= 1;
                return Err(self.error("an expression"));
            }
        })
    }
}

/// True for the XPath core functions (and the few extras this evaluator adds).
pub fn is_core_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}

const FUNCTIONS: &[&str] = &[
    "last",
    "position",
    "count",
    "id",
    "local-name",
    "namespace-uri",
    "name",
    "string",
    "concat",
    "starts-with",
    "ends-with",
    "contains",
    "substring-before",
    "substring-after",
    "substring",
    "string-length",
    "normalize-space",
    "translate",
    "lower-case",
    "upper-case",
    "boolean",
    "not",
    "true",
    "false",
    "lang",
    "number",
    "sum",
    "floor",
    "ceiling",
    "round",
];

// ----------------------------------------------------------------------
// Evaluation

/// A node in the XPath data model: a tree node, or an element's attribute
/// (by index).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum XNode<'a, 'i> {
    Node(Node<'a, 'i>),
    Attr(Node<'a, 'i>, usize),
}

impl<'a, 'i> XNode<'a, 'i> {
    /// Sorts in document order: an element, then its attributes, then its children.
    pub fn order(&self) -> (u32, usize) {
        match self {
            XNode::Node(n) => (n.id().get(), 0),
            XNode::Attr(n, i) => (n.id().get(), i + 1),
        }
    }
}

/// A value an expression produces. `Frag` is an XSLT result tree fragment.
#[derive(Clone, Debug)]
pub enum Value<'a, 'i> {
    Nodes(Vec<XNode<'a, 'i>>),
    Num(f64),
    Str(String),
    Bool(bool),
    Frag(Rc<Fragment>),
}

struct Ctx<'c, 'a, 'i> {
    node: XNode<'a, 'i>,
    pos: usize,
    size: usize,
    env: Option<&'c dyn Env<'a, 'i>>,
}

struct Evaluator<'a, 'i> {
    doc: &'a Document<'i>,
    prefixes: HashMap<&'a str, &'a str>,
    defaults: HashSet<&'a str>,
    /// Text nodes treated as absent (whitespace stripped by XSLT).
    skip: HashSet<u32>,
}

pub fn sort_nodes(nodes: &mut Vec<XNode>) {
    nodes.sort_by_key(XNode::order);
    nodes.dedup_by_key(|n| n.order());
}

fn string_to_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r'));
    let digits = t.strip_prefix('-').unwrap_or(t);
    let valid = !digits.is_empty()
        && digits != "."
        && digits.chars().all(|c| c.is_ascii_digit() || c == '.')
        && digits.matches('.').count() <= 1;
    if valid {
        t.parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

impl<'a, 'i> Evaluator<'a, 'i> {
    fn new(doc: &'a Document<'i>) -> Self {
        let mut prefixes = HashMap::new();
        let mut defaults = HashSet::new();
        prefixes.insert("xml", "http://www.w3.org/XML/1998/namespace");
        for node in doc.descendants().filter(Node::is_element) {
            for ns in node.namespaces() {
                match ns.name() {
                    Some(p) => {
                        prefixes.entry(p).or_insert(ns.uri());
                    }
                    None => {
                        defaults.insert(ns.uri());
                    }
                }
            }
        }
        Self {
            doc,
            prefixes,
            defaults,
            skip: HashSet::new(),
        }
    }

    fn result(&self, n: XNode<'a, 'i>) -> NodeResult {
        let (range, label) = match n {
            XNode::Attr(e, i) => {
                let a = e.attributes().nth(i).expect("attribute index");
                (a.range(), format!("@{}", self.qname(n)))
            }
            XNode::Node(x) => (
                x.range(),
                match x.node_type() {
                    NodeType::Root => "/".to_owned(),
                    NodeType::Element => format!("<{}>", self.qname(n)),
                    NodeType::Text => "text()".to_owned(),
                    NodeType::Comment => "comment()".to_owned(),
                    NodeType::PI => format!("<?{}?>", x.pi().map_or("", |p| p.target)),
                },
            ),
        };
        let value = self.string_value(n);
        NodeResult {
            range,
            label,
            value,
        }
    }

    fn string_value(&self, n: XNode) -> String {
        match n {
            XNode::Attr(e, i) => e
                .attributes()
                .nth(i)
                .map(|a| a.value().to_owned())
                .unwrap_or_default(),
            XNode::Node(x) => match x.node_type() {
                NodeType::Root | NodeType::Element => x
                    .descendants()
                    .filter(|t| t.is_text() && !self.skip.contains(&t.id().get()))
                    .filter_map(|t| t.text())
                    .collect(),
                NodeType::Text | NodeType::Comment => x.text().unwrap_or_default().to_owned(),
                NodeType::PI => x.pi().and_then(|p| p.value).unwrap_or_default().to_owned(),
            },
        }
    }

    fn local_name(&self, n: XNode<'a, 'i>) -> &'a str {
        match n {
            XNode::Attr(e, i) => e.attributes().nth(i).map_or("", |a| a.name()),
            XNode::Node(x) => match x.node_type() {
                NodeType::Element => x.tag_name().name(),
                NodeType::PI => x.pi().map_or("", |p| p.target),
                _ => "",
            },
        }
    }

    fn namespace(&self, n: XNode<'a, 'i>) -> Option<&'a str> {
        match n {
            XNode::Attr(e, i) => e.attributes().nth(i).and_then(|a| a.namespace()),
            XNode::Node(x) if x.is_element() => x.tag_name().namespace(),
            XNode::Node(_) => None,
        }
    }

    fn qname(&self, n: XNode<'a, 'i>) -> String {
        let local = self.local_name(n);
        let element = match n {
            XNode::Attr(e, _) | XNode::Node(e) => e,
        };
        match self.namespace(n).and_then(|uri| element.lookup_prefix(uri)) {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}:{local}"),
            _ => local.to_owned(),
        }
    }

    fn axis(&self, n: XNode<'a, 'i>, axis: Axis) -> Vec<XNode<'a, 'i>> {
        let mut out = self.axis_all(n, axis);
        if !self.skip.is_empty() {
            out.retain(|m| match m {
                XNode::Node(x) => !self.skip.contains(&x.id().get()),
                XNode::Attr(..) => true,
            });
        }
        out
    }

    fn axis_all(&self, n: XNode<'a, 'i>, axis: Axis) -> Vec<XNode<'a, 'i>> {
        let node = XNode::Node;
        match (axis, n) {
            (Axis::Itself, _) => vec![n],
            (Axis::Child, XNode::Node(x)) => x.children().map(node).collect(),
            (Axis::Descendant, XNode::Node(x)) => x.descendants().skip(1).map(node).collect(),
            (Axis::DescendantOrSelf, XNode::Node(x)) => x.descendants().map(node).collect(),
            (Axis::DescendantOrSelf, XNode::Attr(..)) => vec![n],
            (Axis::Parent, XNode::Node(x)) => x.parent().map(node).into_iter().collect(),
            (Axis::Parent, XNode::Attr(e, _)) => vec![node(e)],
            (Axis::Ancestor, XNode::Node(x)) => x.ancestors().skip(1).map(node).collect(),
            (Axis::Ancestor, XNode::Attr(e, _)) => e.ancestors().map(node).collect(),
            (Axis::AncestorOrSelf, XNode::Node(x)) => x.ancestors().map(node).collect(),
            (Axis::AncestorOrSelf, XNode::Attr(e, _)) => {
                std::iter::once(n).chain(e.ancestors().map(node)).collect()
            }
            (Axis::FollowingSibling, XNode::Node(x)) => {
                x.next_siblings().skip(1).map(node).collect()
            }
            (Axis::PrecedingSibling, XNode::Node(x)) => {
                x.prev_siblings().skip(1).map(node).collect()
            }
            (Axis::Following, _) => {
                let mut out = Vec::new();
                let start = match n {
                    XNode::Node(x) => x,
                    XNode::Attr(e, _) => {
                        out.extend(e.descendants().skip(1).map(node));
                        e
                    }
                };
                for a in start.ancestors() {
                    for s in a.next_siblings().skip(1) {
                        out.extend(s.descendants().map(node));
                    }
                }
                out
            }
            (Axis::Preceding, _) => {
                let target = match n {
                    XNode::Node(x) | XNode::Attr(x, _) => x,
                };
                let ancestors: HashSet<u32> = target.ancestors().map(|a| a.id().get()).collect();
                let mut out: Vec<XNode> = self
                    .doc
                    .root()
                    .descendants()
                    .take_while(|d| d.id() != target.id())
                    .filter(|d| !ancestors.contains(&d.id().get()))
                    .map(node)
                    .collect();
                out.reverse();
                out
            }
            (Axis::Attribute, XNode::Node(x)) if x.is_element() => (0..x.attributes().len())
                .map(|i| XNode::Attr(x, i))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn resolve(&self, prefix: &str) -> Result<&'a str, String> {
        self.prefixes.get(prefix).copied().ok_or_else(|| {
            format!("The namespace prefix '{prefix}' isn't declared in this document.")
        })
    }

    fn matches(&self, n: XNode<'a, 'i>, test: &Test, axis: Axis) -> Result<bool, String> {
        let principal = match (n, axis) {
            (XNode::Attr(..), Axis::Attribute) => true,
            (XNode::Node(x), a) if a != Axis::Attribute => x.is_element(),
            _ => false,
        };
        Ok(match test {
            Test::Node => true,
            Test::Text => matches!(n, XNode::Node(x) if x.is_text()),
            Test::Comment => matches!(n, XNode::Node(x) if x.is_comment()),
            Test::Pi(target) => match n {
                XNode::Node(x) => x
                    .pi()
                    .is_some_and(|p| target.as_ref().is_none_or(|t| t == p.target)),
                _ => false,
            },
            Test::Any => principal,
            Test::AnyInUri(uri) => principal && self.namespace(n) == Some(uri.as_str()),
            Test::Resolved { uri, local } => {
                principal && self.local_name(n) == local && self.namespace(n) == uri.as_deref()
            }
            Test::AnyIn(prefix) => {
                let uri = self.resolve(prefix)?;
                principal && self.namespace(n) == Some(uri)
            }
            Test::Name { prefix, local } => {
                // Resolve first, so an undeclared prefix is an error even with no matches.
                let uri = prefix.as_deref().map(|p| self.resolve(p)).transpose()?;
                if !principal || self.local_name(n) != local {
                    false
                } else {
                    match (prefix, n) {
                        (Some(_), _) => self.namespace(n) == uri,
                        (None, XNode::Attr(..)) => self.namespace(n).is_none(),
                        (None, XNode::Node(_)) => self
                            .namespace(n)
                            .is_none_or(|uri| self.defaults.contains(uri)),
                    }
                }
            }
        })
    }

    fn filter(
        &self,
        nodes: Vec<XNode<'a, 'i>>,
        predicates: &[Expr],
        env: Option<&dyn Env<'a, 'i>>,
    ) -> Result<Vec<XNode<'a, 'i>>, String> {
        let mut nodes = nodes;
        for pred in predicates {
            let size = nodes.len();
            let mut kept = Vec::with_capacity(size);
            for (i, n) in nodes.into_iter().enumerate() {
                let ctx = Ctx {
                    node: n,
                    pos: i + 1,
                    size,
                    env,
                };
                let keep = match self.eval(pred, &ctx)? {
                    Value::Num(num) => num == (i + 1) as f64,
                    v => self.boolean(&v),
                };
                if keep {
                    kept.push(n);
                }
            }
            nodes = kept;
        }
        Ok(nodes)
    }

    fn eval(&self, e: &Expr, c: &Ctx<'_, 'a, 'i>) -> Result<Value<'a, 'i>, String> {
        Ok(match e {
            Expr::Or(a, b) => Value::Bool(self.to_bool(a, c)? || self.to_bool(b, c)?),
            Expr::And(a, b) => Value::Bool(self.to_bool(a, c)? && self.to_bool(b, c)?),
            Expr::Compare(op, a, b) => {
                let (a, b) = (self.eval(a, c)?, self.eval(b, c)?);
                Value::Bool(self.compare(*op, &a, &b))
            }
            Expr::Arith(op, a, b) => {
                let (a, b) = (self.to_num(a, c)?, self.to_num(b, c)?);
                Value::Num(match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => a / b,
                    _ => a % b,
                })
            }
            Expr::Negate(a) => Value::Num(-self.to_num(a, c)?),
            Expr::Union(a, b) => match (self.eval(a, c)?, self.eval(b, c)?) {
                (Value::Nodes(mut x), Value::Nodes(y)) => {
                    x.extend(y);
                    sort_nodes(&mut x);
                    Value::Nodes(x)
                }
                _ => return Err("Both sides of '|' must select nodes.".into()),
            },
            Expr::Path(start, steps) => {
                let mut nodes = match start {
                    Start::Root => {
                        let root = match c.node {
                            XNode::Node(x) | XNode::Attr(x, _) => x.document().root(),
                        };
                        vec![XNode::Node(root)]
                    }
                    Start::Context => vec![c.node],
                    Start::Expr(e) => match self.eval(e, c)? {
                        Value::Nodes(n) => n,
                        Value::Frag(_) => return Err(FRAG_PATH.into()),
                        _ => return Err("Only a node-set can be followed by '/'.".into()),
                    },
                };
                for step in steps {
                    let mut next = Vec::new();
                    for &n in &nodes {
                        let mut candidates = Vec::new();
                        for m in self.axis(n, step.axis) {
                            if self.matches(m, &step.test, step.axis)? {
                                candidates.push(m);
                            }
                        }
                        next.extend(self.filter(candidates, &step.predicates, c.env)?);
                    }
                    sort_nodes(&mut next);
                    nodes = next;
                }
                Value::Nodes(nodes)
            }
            Expr::Filter(primary, predicates) => match self.eval(primary, c)? {
                Value::Nodes(n) => Value::Nodes(self.filter(n, predicates, c.env)?),
                Value::Frag(_) => return Err(FRAG_PATH.into()),
                _ => return Err("Only a node-set can have a [predicate].".into()),
            },
            Expr::Literal(s) => Value::Str(s.clone()),
            Expr::Number(n) => Value::Num(*n),
            Expr::Var(name) => match c.env {
                Some(env) => env.variable(name)?,
                None => return Err(format!("Variables aren't supported (${name}).")),
            },
            Expr::Call(name, args) => self.call(name, args, c)?,
        })
    }

    fn to_bool(&self, e: &Expr, c: &Ctx<'_, 'a, 'i>) -> Result<bool, String> {
        Ok(self.boolean(&self.eval(e, c)?))
    }

    fn to_num(&self, e: &Expr, c: &Ctx<'_, 'a, 'i>) -> Result<f64, String> {
        Ok(self.number(&self.eval(e, c)?))
    }

    fn to_str(&self, e: &Expr, c: &Ctx<'_, 'a, 'i>) -> Result<String, String> {
        Ok(self.string(&self.eval(e, c)?))
    }

    fn boolean(&self, v: &Value) -> bool {
        match v {
            Value::Nodes(n) => !n.is_empty(),
            Value::Num(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::Frag(_) => true,
        }
    }

    fn number(&self, v: &Value) -> f64 {
        match v {
            Value::Num(n) => *n,
            Value::Bool(b) => f64::from(u8::from(*b)),
            _ => string_to_number(&self.string(v)),
        }
    }

    fn string(&self, v: &Value) -> String {
        match v {
            Value::Nodes(n) => n.first().map(|&n| self.string_value(n)).unwrap_or_default(),
            Value::Num(n) => number_to_string(*n),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Frag(f) => f.string_value(),
        }
    }

    fn compare(&self, op: Cmp, a: &Value, b: &Value) -> bool {
        let num_cmp = |x: f64, y: f64| match op {
            Cmp::Eq => x == y,
            Cmp::Ne => x != y,
            Cmp::Lt => x < y,
            Cmp::Le => x <= y,
            Cmp::Gt => x > y,
            Cmp::Ge => x >= y,
        };
        match (a, b) {
            (Value::Nodes(x), Value::Nodes(y)) => {
                let ys: Vec<String> = y.iter().map(|&n| self.string_value(n)).collect();
                x.iter().any(|&n| {
                    let s = self.string_value(n);
                    ys.iter().any(|t| match op {
                        Cmp::Eq => s == *t,
                        Cmp::Ne => s != *t,
                        _ => num_cmp(string_to_number(&s), string_to_number(t)),
                    })
                })
            }
            (Value::Nodes(x), other) | (other, Value::Nodes(x)) => {
                let flipped = matches!(a, Value::Nodes(_))
                    .then_some(op)
                    .unwrap_or(match op {
                        Cmp::Lt => Cmp::Gt,
                        Cmp::Le => Cmp::Ge,
                        Cmp::Gt => Cmp::Lt,
                        Cmp::Ge => Cmp::Le,
                        o => o,
                    });
                let node_first = |nv: Value, o: &Value| self.compare_atomic(flipped, &nv, o);
                match other {
                    Value::Bool(_) => node_first(Value::Bool(!x.is_empty()), other),
                    Value::Num(_) => x.iter().any(|&n| {
                        node_first(Value::Num(string_to_number(&self.string_value(n))), other)
                    }),
                    _ => x
                        .iter()
                        .any(|&n| node_first(Value::Str(self.string_value(n)), other)),
                }
            }
            _ => self.compare_atomic(op, a, b),
        }
    }

    fn compare_atomic(&self, op: Cmp, a: &Value, b: &Value) -> bool {
        match op {
            Cmp::Eq | Cmp::Ne => {
                let equal = if matches!(a, Value::Bool(_)) || matches!(b, Value::Bool(_)) {
                    self.boolean(a) == self.boolean(b)
                } else if matches!(a, Value::Num(_)) || matches!(b, Value::Num(_)) {
                    self.number(a) == self.number(b)
                } else {
                    self.string(a) == self.string(b)
                };
                equal == (op == Cmp::Eq)
            }
            _ => {
                let (x, y) = (self.number(a), self.number(b));
                match op {
                    Cmp::Lt => x < y,
                    Cmp::Le => x <= y,
                    Cmp::Gt => x > y,
                    _ => x >= y,
                }
            }
        }
    }

    fn call(
        &self,
        name: &str,
        args: &[Expr],
        c: &Ctx<'_, 'a, 'i>,
    ) -> Result<Value<'a, 'i>, String> {
        if !FUNCTIONS.contains(&name)
            && let Some(env) = c.env
        {
            let mut values = Vec::with_capacity(args.len());
            for a in args {
                values.push(self.eval(a, c)?);
            }
            return env
                .function(name, &values, c.node)
                .unwrap_or_else(|| Err(format!("Unknown function {name}().")));
        }
        let arity = |min: usize, max: usize| -> Result<(), String> {
            if args.len() < min || args.len() > max {
                let expected = if min == max {
                    format!("{min}")
                } else if max == usize::MAX {
                    format!("at least {min}")
                } else {
                    format!("{min} to {max}")
                };
                Err(format!(
                    "{name}() takes {expected} argument{}, not {}.",
                    if max == 1 && min == 1 { "" } else { "s" },
                    args.len()
                ))
            } else {
                Ok(())
            }
        };
        let nodes_arg = |i: usize| -> Result<Vec<XNode<'a, 'i>>, String> {
            match self.eval(&args[i], c)? {
                Value::Nodes(n) => Ok(n),
                Value::Frag(_) => Err(format!("{name}() needs a node-set. {FRAG_PATH}")),
                _ => Err(format!("{name}() needs a node-set.")),
            }
        };
        // The optional node-set argument, or the context node.
        let node_or_context = || -> Result<Option<XNode<'a, 'i>>, String> {
            if args.is_empty() {
                Ok(Some(c.node))
            } else {
                Ok(nodes_arg(0)?.first().copied())
            }
        };
        let str_or_context = |i: usize| -> Result<String, String> {
            if args.len() > i {
                self.to_str(&args[i], c)
            } else {
                Ok(self.string_value(c.node))
            }
        };
        Ok(match name {
            "last" => {
                arity(0, 0)?;
                Value::Num(c.size as f64)
            }
            "position" => {
                arity(0, 0)?;
                Value::Num(c.pos as f64)
            }
            "count" => {
                arity(1, 1)?;
                Value::Num(nodes_arg(0)?.len() as f64)
            }
            "id" => {
                arity(1, 1)?;
                let ids: Vec<String> = match self.eval(&args[0], c)? {
                    Value::Nodes(n) => n
                        .iter()
                        .flat_map(|&n| {
                            self.string_value(n)
                                .split_whitespace()
                                .map(str::to_owned)
                                .collect::<Vec<_>>()
                        })
                        .collect(),
                    v => self
                        .string(&v)
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect(),
                };
                let mut found: Vec<XNode> = self
                    .doc
                    .descendants()
                    .filter(|n| {
                        n.is_element()
                            && n.attributes()
                                .any(|a| a.name() == "id" && ids.iter().any(|id| id == a.value()))
                    })
                    .map(XNode::Node)
                    .collect();
                sort_nodes(&mut found);
                Value::Nodes(found)
            }
            "local-name" | "namespace-uri" | "name" => {
                arity(0, 1)?;
                Value::Str(match node_or_context()? {
                    None => String::new(),
                    Some(n) => match name {
                        "local-name" => self.local_name(n).to_owned(),
                        "namespace-uri" => self.namespace(n).unwrap_or_default().to_owned(),
                        _ => self.qname(n),
                    },
                })
            }
            "string" => {
                arity(0, 1)?;
                Value::Str(str_or_context(0)?)
            }
            "concat" => {
                arity(2, usize::MAX)?;
                let mut s = String::new();
                for a in args {
                    s.push_str(&self.to_str(a, c)?);
                }
                Value::Str(s)
            }
            "starts-with" | "ends-with" | "contains" | "substring-before" | "substring-after" => {
                arity(2, 2)?;
                let (s, t) = (self.to_str(&args[0], c)?, self.to_str(&args[1], c)?);
                match name {
                    "starts-with" => Value::Bool(s.starts_with(&t)),
                    "ends-with" => Value::Bool(s.ends_with(&t)),
                    "contains" => Value::Bool(s.contains(&t)),
                    "substring-before" => {
                        Value::Str(s.find(&t).map(|i| s[..i].to_owned()).unwrap_or_default())
                    }
                    _ => Value::Str(
                        s.find(&t)
                            .map(|i| s[i + t.len()..].to_owned())
                            .unwrap_or_default(),
                    ),
                }
            }
            "substring" => {
                arity(2, 3)?;
                let s = self.to_str(&args[0], c)?;
                let start = self.to_num(&args[1], c)?.round();
                let end = if args.len() == 3 {
                    start + self.to_num(&args[2], c)?.round()
                } else {
                    f64::INFINITY
                };
                // XPath positions count from 1; NaN comparisons are false, so NaN selects nothing.
                Value::Str(
                    s.chars()
                        .enumerate()
                        .filter(|&(i, _)| {
                            let p = (i + 1) as f64;
                            p >= start && p < end
                        })
                        .map(|(_, ch)| ch)
                        .collect(),
                )
            }
            "string-length" => {
                arity(0, 1)?;
                Value::Num(str_or_context(0)?.chars().count() as f64)
            }
            "normalize-space" => {
                arity(0, 1)?;
                Value::Str(
                    str_or_context(0)?
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            }
            "translate" => {
                arity(3, 3)?;
                let s = self.to_str(&args[0], c)?;
                let from: Vec<char> = self.to_str(&args[1], c)?.chars().collect();
                let to: Vec<char> = self.to_str(&args[2], c)?.chars().collect();
                Value::Str(
                    s.chars()
                        .filter_map(|ch| match from.iter().position(|&f| f == ch) {
                            Some(i) => to.get(i).copied(),
                            None => Some(ch),
                        })
                        .collect(),
                )
            }
            "lower-case" | "upper-case" => {
                arity(1, 1)?;
                let s = self.to_str(&args[0], c)?;
                Value::Str(if name == "lower-case" {
                    s.to_lowercase()
                } else {
                    s.to_uppercase()
                })
            }
            "boolean" => {
                arity(1, 1)?;
                Value::Bool(self.to_bool(&args[0], c)?)
            }
            "not" => {
                arity(1, 1)?;
                Value::Bool(!self.to_bool(&args[0], c)?)
            }
            "true" | "false" => {
                arity(0, 0)?;
                Value::Bool(name == "true")
            }
            "lang" => {
                arity(1, 1)?;
                let want = self.to_str(&args[0], c)?.to_lowercase();
                let element = match c.node {
                    XNode::Node(x) | XNode::Attr(x, _) => x,
                };
                let lang = element
                    .ancestors()
                    .find_map(|a| a.attribute(("http://www.w3.org/XML/1998/namespace", "lang")));
                Value::Bool(lang.is_some_and(|l| {
                    let l = l.to_lowercase();
                    l == want || l.starts_with(&format!("{want}-"))
                }))
            }
            "number" => {
                arity(0, 1)?;
                Value::Num(if args.is_empty() {
                    string_to_number(&self.string_value(c.node))
                } else {
                    self.to_num(&args[0], c)?
                })
            }
            "sum" => {
                arity(1, 1)?;
                Value::Num(
                    nodes_arg(0)?
                        .iter()
                        .map(|&n| string_to_number(&self.string_value(n)))
                        .sum(),
                )
            }
            "floor" | "ceiling" | "round" => {
                arity(1, 1)?;
                let n = self.to_num(&args[0], c)?;
                Value::Num(match name {
                    "floor" => n.floor(),
                    "ceiling" => n.ceil(),
                    // XPath rounds halves up, towards positive infinity.
                    _ => (n + 0.5).floor(),
                })
            }
            _ => return Err(format!("Unknown function {name}().")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0"?>
<catalog xmlns="urn:books" xmlns:x="urn:extra">
  <!-- first -->
  <book id="b1" lang="en"><title>Rust</title><price>30</price></book>
  <book id="b2"><title>XML</title><price>25.5</price><x:note>old</x:note></book>
  <book id="b3"><title>Egui</title><price>40</price></book>
  <?app go?>
</catalog>"#;

    fn eval(expr: &str) -> Outcome {
        let doc = roxmltree::Document::parse(XML).unwrap();
        evaluate(&doc, expr).unwrap_or_else(|e| panic!("{expr}: {e}"))
    }

    fn labels(expr: &str) -> Vec<String> {
        match eval(expr) {
            Outcome::Nodes(n) => n
                .into_iter()
                .map(|r| {
                    let value: Vec<&str> = r.value.split_whitespace().collect();
                    format!("{}={}", r.label, value.join(" "))
                })
                .collect(),
            other => panic!("{expr}: not nodes: {other:?}"),
        }
    }

    #[test]
    fn paths_axes_and_predicates() {
        assert_eq!(
            labels("/catalog/book/title"),
            ["<title>=Rust", "<title>=XML", "<title>=Egui"]
        );
        assert_eq!(labels("//book[2]/@id"), ["@id=b2"]);
        assert_eq!(labels("//book[last()]/title"), ["<title>=Egui"]);
        assert_eq!(
            labels("//book[price > 28]/title/text()"),
            ["text()=Rust", "text()=Egui"]
        );
        assert_eq!(labels("//title[. = 'XML']/../@id"), ["@id=b2"]);
        assert_eq!(labels("//x:note"), ["<x:note>=old"]);
        assert_eq!(labels("//x:*"), ["<x:note>=old"]);
        assert_eq!(labels("//comment()"), ["comment()=first"]);
        assert_eq!(labels("//processing-instruction('app')"), ["<?app?>=go"]);
        assert_eq!(
            labels("//book[1]/following-sibling::book/@id"),
            ["@id=b2", "@id=b3"]
        );
        assert_eq!(
            labels("//book[3]/preceding-sibling::book[1]/@id"),
            ["@id=b2"]
        );
        assert_eq!(
            labels("//price[1]/ancestor::*"),
            [
                "<catalog>=Rust30 XML25.5old Egui40",
                "<book>=Rust30",
                "<book>=XML25.5old",
                "<book>=Egui40"
            ]
        );
        assert_eq!(
            labels("//title[.='XML']/following::title"),
            ["<title>=Egui"]
        );
        assert_eq!(labels("//title[.='XML']/preceding::price"), ["<price>=30"]);
        assert_eq!(
            labels("//book[@id='b1' or @id='b3']/@id | //book[2]/@id"),
            ["@id=b1", "@id=b2", "@id=b3"]
        );
        assert_eq!(labels("(//title)[2]"), ["<title>=XML"]);
        assert_eq!(labels("//book/@*[name()='lang']"), ["@lang=en"]);
        assert_eq!(labels("id('b3')/title"), ["<title>=Egui"]);
        assert_eq!(
            labels("/descendant::book[position() mod 2 = 1]/@id"),
            ["@id=b1", "@id=b3"]
        );
        assert_eq!(labels("//book[not(x:note)]/@id"), ["@id=b1", "@id=b3"]);
        assert_eq!(labels("/"), ["/=Rust30 XML25.5old Egui40"]);
    }

    #[test]
    fn values_and_functions() {
        assert_eq!(eval("count(//book)"), Outcome::Number(3.0));
        assert_eq!(eval("sum(//price)"), Outcome::Number(95.5));
        assert_eq!(
            eval("sum(//price) div count(//price) * 2"),
            Outcome::Number(95.5 / 3.0 * 2.0)
        );
        assert_eq!(
            eval("string(//book[2]/title)"),
            Outcome::String("XML".into())
        );
        assert_eq!(
            eval("concat('a', 1, true())"),
            Outcome::String("a1true".into())
        );
        assert_eq!(
            eval("substring('12345', 1.5, 2.6)"),
            Outcome::String("234".into())
        );
        assert_eq!(
            eval("substring('12345', 0, 3)"),
            Outcome::String("12".into())
        );
        assert_eq!(
            eval("substring-after('a=b=c', '=')"),
            Outcome::String("b=c".into())
        );
        assert_eq!(
            eval("translate('bar', 'abc', 'AB')"),
            Outcome::String("BAr".into())
        );
        assert_eq!(
            eval("normalize-space('  a   b ')"),
            Outcome::String("a b".into())
        );
        assert_eq!(
            eval("round(2.5) + floor(-1.5) + ceiling(0.2)"),
            Outcome::Number(3.0 - 2.0 + 1.0)
        );
        assert_eq!(eval("//price = 25.5"), Outcome::Boolean(true));
        assert_eq!(eval("//price != 30"), Outcome::Boolean(true));
        assert_eq!(eval("//book/@id = 'b9'"), Outcome::Boolean(false));
        assert_eq!(eval("10 > //price"), Outcome::Boolean(false));
        assert_eq!(eval("-(3 - 5) * 2"), Outcome::Number(4.0));
        assert_eq!(eval("5 mod 3 = 2 and 1 < 2"), Outcome::Boolean(true));
        assert_eq!(eval("local-name(/*)"), Outcome::String("catalog".into()));
        assert_eq!(
            eval("namespace-uri(//x:note)"),
            Outcome::String("urn:extra".into())
        );
        assert_eq!(eval("lang('en')"), Outcome::Boolean(false));
        assert_eq!(
            eval("upper-case(//book[1]/title)"),
            Outcome::String("RUST".into())
        );
        assert_eq!(number_to_string(3.0), "3");
        assert_eq!(number_to_string(0.5), "0.5");
        assert_eq!(number_to_string(-0.0), "0");
        assert_eq!(number_to_string(f64::NAN), "NaN");
    }

    #[test]
    fn number_outcome_nan() {
        match eval("number('abc')") {
            Outcome::Number(n) => assert!(n.is_nan()),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn errors() {
        let doc = roxmltree::Document::parse(XML).unwrap();
        let err = |e: &str| evaluate(&doc, e).unwrap_err();
        assert_eq!(err("//book[").offset, Some(7));
        assert!(err("//book[").message.contains("ended"));
        assert!(err("foo(1)").message.contains("Unknown function"));
        assert!(err("//y:thing").message.contains("'y'"));
        assert!(err("count(1)").message.contains("node-set"));
        assert!(err("count()").message.contains("1 argument"));
        assert!(err("bogus::x").message.contains("isn't an axis"));
        assert!(err("'abc").message.contains("never closed"));
        assert!(err("").message.contains("Type an XPath"));
        assert!(err("1 2").message.contains("operator"));
        assert!(err("$v").message.contains("Variables"));
    }

    #[test]
    fn ranges_point_at_nodes() {
        let doc = roxmltree::Document::parse(XML).unwrap();
        let Outcome::Nodes(n) = evaluate(&doc, "//book[2]/@id").unwrap() else {
            panic!()
        };
        assert_eq!(&XML[n[0].range.clone()], "id=\"b2\"");
        let Outcome::Nodes(n) = evaluate(&doc, "//book[3]/title").unwrap() else {
            panic!()
        };
        assert_eq!(&XML[n[0].range.clone()], "<title>Egui</title>");
    }
}
