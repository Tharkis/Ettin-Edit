//! XML Tools: pretty print, linearize, syntax check, escape and unescape, and
//! the XPath of the element at the caret.
//!
//! Formatting uses its own tokenizer, which only needs tags to balance, so it
//! keeps entity references, DOCTYPEs and attribute text exactly as written.
//! The syntax check is strict and uses `roxmltree`.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::{ToolError, ToolResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmlOp {
    PrettyPrint,
    /// Pretty print with each attribute on its own line.
    PrettyPrintAttributes,
    Linearize,
    CheckSyntax,
    Escape,
    Unescape,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ItemKind {
    /// `<?...?>`, including the XML declaration.
    Pi,
    Comment,
    CData,
    /// `<!DOCTYPE ...>` and other `<!...>` declarations.
    Declaration,
    Start {
        name: Range<usize>,
        attrs: Vec<Attr>,
        self_closing: bool,
    },
    End {
        name: Range<usize>,
    },
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Attr {
    /// The whole `name="value"`.
    range: Range<usize>,
    name: Range<usize>,
    value: Range<usize>,
}

#[derive(Clone, Debug)]
struct Item {
    kind: ItemKind,
    range: Range<usize>,
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == ':' || (c as u32) > 0x7f
}

fn is_name_char(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-' || c == '.'
}

struct Lexer<'a> {
    s: &'a str,
    i: usize,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek().filter(|c| c.is_whitespace()) {
            self.i += c.len_utf8();
        }
    }

    fn name(&mut self) -> Option<Range<usize>> {
        let start = self.i;
        let first = self.peek().filter(|&c| is_name_start(c))?;
        self.i += first.len_utf8();
        while let Some(c) = self.peek().filter(|&c| is_name_char(c)) {
            self.i += c.len_utf8();
        }
        Some(start..self.i)
    }

    fn until(&mut self, end: &str, start: usize, what: &str) -> Result<(), ToolError> {
        match self.s[self.i..].find(end) {
            Some(p) => {
                self.i += p + end.len();
                Ok(())
            }
            None => Err(ToolError::at(
                format!("This {what} is never closed."),
                start,
            )),
        }
    }

    fn unexpected(&self, expected: &str) -> ToolError {
        match self.peek() {
            Some(c) => ToolError::at(format!("Expected {expected}, found '{c}'."), self.i),
            None => ToolError::at(format!("Expected {expected}, but the text ended."), self.i),
        }
    }

    fn item(&mut self) -> Result<Item, ToolError> {
        let start = self.i;
        let rest = &self.s[self.i..];
        let kind = if !rest.starts_with('<') {
            self.i += rest.find('<').unwrap_or(rest.len());
            ItemKind::Text
        } else if rest.starts_with("<?") {
            self.i += 2;
            self.until("?>", start, "processing instruction")?;
            ItemKind::Pi
        } else if rest.starts_with("<!--") {
            self.i += 4;
            self.until("-->", start, "comment")?;
            ItemKind::Comment
        } else if rest.starts_with("<![CDATA[") {
            self.i += 9;
            self.until("]]>", start, "CDATA section")?;
            ItemKind::CData
        } else if rest.starts_with("<!") {
            // Skip to the matching '>', past quoted text and an internal subset.
            self.i += 2;
            let mut bracket = 0;
            let mut quote = None;
            loop {
                let Some(c) = self.peek() else {
                    return Err(ToolError::at("This declaration is never closed.", start));
                };
                self.i += c.len_utf8();
                match (quote, c) {
                    (Some(q), c) if c == q => quote = None,
                    (Some(_), _) => {}
                    (None, '"' | '\'') => quote = Some(c),
                    (None, '[') => bracket += 1,
                    (None, ']') => bracket -= 1,
                    (None, '>') if bracket <= 0 => break,
                    _ => {}
                }
            }
            ItemKind::Declaration
        } else if rest.starts_with("</") {
            self.i += 2;
            let name = self.name().ok_or_else(|| self.unexpected("a tag name"))?;
            self.skip_ws();
            if self.peek() != Some('>') {
                return Err(self.unexpected("'>'"));
            }
            self.i += 1;
            ItemKind::End { name }
        } else {
            self.i += 1;
            let name = self.name().ok_or_else(|| self.unexpected("a tag name"))?;
            let mut attrs = Vec::new();
            let self_closing;
            loop {
                let before_ws = self.i;
                self.skip_ws();
                match self.peek() {
                    Some('>') => {
                        self.i += 1;
                        self_closing = false;
                        break;
                    }
                    Some('/') if self.s[self.i..].starts_with("/>") => {
                        self.i += 2;
                        self_closing = true;
                        break;
                    }
                    _ => {}
                }
                if self.i == before_ws {
                    return Err(self.unexpected("a space, '>' or '/>'"));
                }
                let attr_start = self.i;
                let attr_name = self
                    .name()
                    .ok_or_else(|| self.unexpected("an attribute name"))?;
                self.skip_ws();
                if self.peek() != Some('=') {
                    return Err(self.unexpected("'=' after the attribute name"));
                }
                self.i += 1;
                self.skip_ws();
                let quote = self
                    .peek()
                    .filter(|&c| c == '"' || c == '\'')
                    .ok_or_else(|| self.unexpected("a quoted attribute value"))?;
                let value_start = self.i;
                self.i += 1;
                match self.s[self.i..].find(quote) {
                    Some(p) => self.i += p + 1,
                    None => {
                        return Err(ToolError::at(
                            "This attribute value is never closed.",
                            value_start,
                        ));
                    }
                }
                attrs.push(Attr {
                    range: attr_start..self.i,
                    name: attr_name,
                    value: value_start..self.i,
                });
            }
            ItemKind::Start {
                name,
                attrs,
                self_closing,
            }
        };
        Ok(Item {
            kind,
            range: start..self.i,
        })
    }
}

/// An element and what's inside it, as indexes into the item list.
#[derive(Debug)]
struct Element {
    start: usize,
    /// The end tag, or `None` for `<a/>`.
    end: Option<usize>,
    children: Vec<Child>,
}

#[derive(Debug)]
enum Child {
    Element(Element),
    Item(usize),
}

struct Tree<'a> {
    text: &'a str,
    items: Vec<Item>,
    top: Vec<Child>,
}

/// Builds the element tree. With `close_at_end`, elements still open at the
/// end of the text are closed there instead of being an error (for text that
/// is being typed).
fn parse_tree(text: &str, close_at_end: bool) -> Result<Tree<'_>, ToolError> {
    let mut lexer = Lexer { s: text, i: 0 };
    if text.starts_with('\u{feff}') {
        lexer.i = 3;
    }
    let mut items = Vec::new();
    while lexer.i < text.len() {
        items.push(lexer.item()?);
    }
    // Open elements, innermost last, each with the children gathered so far.
    let mut open: Vec<Element> = Vec::new();
    let mut top = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let child = match &item.kind {
            ItemKind::Start {
                self_closing: false,
                ..
            } => {
                open.push(Element {
                    start: index,
                    end: None,
                    children: Vec::new(),
                });
                continue;
            }
            ItemKind::Start { .. } => Child::Element(Element {
                start: index,
                end: None,
                children: Vec::new(),
            }),
            ItemKind::End { name } => {
                let Some(mut element) = open.pop() else {
                    return Err(ToolError::at(
                        format!("</{}> has no matching start tag.", &text[name.clone()]),
                        item.range.start,
                    ));
                };
                let open_name = start_name(text, &items[element.start]);
                if open_name != &text[name.clone()] {
                    return Err(ToolError::at(
                        format!(
                            "Expected </{open_name}> to close <{open_name}>, found </{}>.",
                            &text[name.clone()]
                        ),
                        item.range.start,
                    ));
                }
                element.end = Some(index);
                Child::Element(element)
            }
            _ => Child::Item(index),
        };
        match open.last_mut() {
            Some(parent) => parent.children.push(child),
            None => top.push(child),
        }
    }
    if close_at_end {
        while let Some(element) = open.pop() {
            match open.last_mut() {
                Some(parent) => parent.children.push(Child::Element(element)),
                None => top.push(Child::Element(element)),
            }
        }
    }
    if let Some(element) = open.pop() {
        let name = start_name(text, &items[element.start]);
        return Err(ToolError::at(
            format!("<{name}> is never closed."),
            items[element.start].range.start,
        ));
    }
    Ok(Tree { text, items, top })
}

fn start_name<'a>(text: &'a str, item: &Item) -> &'a str {
    match &item.kind {
        ItemKind::Start { name, .. } | ItemKind::End { name } => &text[name.clone()],
        _ => "",
    }
}

struct Printer<'a> {
    tree: &'a Tree<'a>,
    /// One level of indentation, or `None` to linearize.
    unit: Option<&'a str>,
    attrs_on_lines: bool,
    out: String,
}

impl<'a> Printer<'a> {
    fn text(&self, range: &Range<usize>) -> &'a str {
        let text: &'a str = self.tree.text;
        &text[range.clone()]
    }

    fn line(&mut self, depth: usize) {
        if let Some(unit) = self.unit {
            if !self.out.is_empty() {
                self.out.push('\n');
            }
            for _ in 0..depth {
                self.out.push_str(unit);
            }
        }
    }

    fn is_blank(&self, child: &Child) -> bool {
        match child {
            Child::Item(i) => {
                let item = &self.tree.items[*i];
                item.kind == ItemKind::Text && self.text(&item.range).trim().is_empty()
            }
            Child::Element(_) => false,
        }
    }

    fn start_tag(&mut self, item: usize, depth: usize) {
        let item = &self.tree.items[item];
        let ItemKind::Start {
            name,
            attrs,
            self_closing,
        } = &item.kind
        else {
            return;
        };
        let mut tag = format!("<{}", self.text(name));
        let on_lines = self.attrs_on_lines && attrs.len() > 1 && self.unit.is_some();
        for attr in attrs {
            if on_lines {
                tag.push('\n');
                for _ in 0..=depth {
                    tag.push_str(self.unit.unwrap_or_default());
                }
            } else {
                tag.push(' ');
            }
            tag.push_str(self.text(&attr.name));
            tag.push('=');
            tag.push_str(self.text(&attr.value));
        }
        tag.push_str(if *self_closing { "/>" } else { ">" });
        self.out.push_str(&tag);
    }

    fn preserves_space(&self, element: &Element) -> bool {
        let ItemKind::Start { attrs, .. } = &self.tree.items[element.start].kind else {
            return false;
        };
        attrs.iter().any(|a| {
            self.text(&a.name) == "xml:space"
                && self.text(&a.value).trim_matches(['"', '\'']) == "preserve"
        })
    }

    fn element(&mut self, element: &Element, depth: usize) {
        let items = &self.tree.items;
        let Some(end) = element.end else {
            self.start_tag(element.start, depth);
            return;
        };
        let has_markup_children = element.children.iter().any(|c| match c {
            Child::Element(_) => true,
            Child::Item(i) => items[*i].kind != ItemKind::Text,
        });
        let has_text = element.children.iter().any(|c| match c {
            Child::Item(i) => {
                items[*i].kind == ItemKind::Text && !self.text(&items[*i].range).trim().is_empty()
            }
            Child::Element(_) => false,
        });
        // Mixed content and xml:space="preserve" keep their inside as written.
        if (has_text && has_markup_children) || self.preserves_space(element) {
            self.start_tag(element.start, depth);
            let inner = items[element.start].range.end..items[end].range.start;
            self.out.push_str(&self.tree.text[inner]);
            self.out.push_str(self.text(&items[end].range));
            return;
        }
        self.start_tag(element.start, depth);
        if !has_markup_children {
            // Text only (or empty): stays on one line, exactly as written.
            let inner = items[element.start].range.end..items[end].range.start;
            let inner = &self.tree.text[inner];
            if !inner.trim().is_empty() || self.unit.is_some() {
                self.out.push_str(inner);
            }
            self.out.push_str(self.text(&items[end].range));
            return;
        }
        self.children(&element.children, depth + 1);
        self.line(depth);
        self.out.push_str(self.text(&items[end].range));
    }

    fn children(&mut self, children: &[Child], depth: usize) {
        for child in children {
            if self.is_blank(child) {
                continue;
            }
            self.line(depth);
            match child {
                Child::Element(e) => self.element(e, depth),
                Child::Item(i) => {
                    let range = self.tree.items[*i].range.clone();
                    let mut text = self.text(&range);
                    if self.tree.items[*i].kind == ItemKind::Text && self.unit.is_some() {
                        text = text.trim();
                    }
                    self.out.push_str(text);
                }
            }
        }
    }
}

/// Indents the XML with `unit` per level. Elements holding only text stay
/// on one line; mixed content and `xml:space="preserve"` are left as written.
pub fn pretty_print(text: &str, unit: &str, attrs_on_lines: bool) -> ToolResult {
    let tree = parse_tree(text, false)?;
    let mut printer = Printer {
        tree: &tree,
        unit: Some(unit),
        attrs_on_lines,
        out: String::with_capacity(text.len() * 5 / 4),
    };
    printer.children(&tree.top, 0);
    Ok(printer.out)
}

/// Removes the line breaks and indentation between tags.
pub fn linearize(text: &str) -> ToolResult {
    let tree = parse_tree(text, false)?;
    let mut printer = Printer {
        tree: &tree,
        unit: None,
        attrs_on_lines: false,
        out: String::with_capacity(text.len()),
    };
    printer.children(&tree.top, 0);
    Ok(printer.out)
}

/// Converts roxmltree's row and column (1-based, in chars) to a byte offset.
fn text_pos_offset(text: &str, pos: roxmltree::TextPos) -> usize {
    let line_start: usize = text
        .split_inclusive('\n')
        .take(pos.row.saturating_sub(1) as usize)
        .map(str::len)
        .sum();
    let line = &text[line_start.min(text.len())..];
    line_start
        + line
            .char_indices()
            .nth(pos.col.saturating_sub(1) as usize)
            .map_or(line.len(), |(i, _)| i)
}

pub fn parse_options<'a>() -> roxmltree::ParsingOptions<'a> {
    roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    }
}

/// Parses strictly, with errors placed in the text.
pub fn parse_strict(text: &str) -> Result<roxmltree::Document<'_>, ToolError> {
    roxmltree::Document::parse_with_options(text, parse_options()).map_err(|e| {
        let pos = e.pos();
        if matches!(e, roxmltree::Error::UnknownToken(_)) {
            // roxmltree's "unknown token" covers several cases; name the likely one.
            let offset = text_pos_offset(text, pos);
            let rest = &text[offset..];
            let message = if rest.starts_with("<!") {
                "Unrecognized '<!' markup."
            } else if rest.starts_with('<') {
                "Unexpected element. A document has exactly one root element."
            } else {
                "Text isn't allowed outside the root element."
            };
            return ToolError::at(message, offset);
        }
        let mut message = e.to_string();
        // roxmltree ends its messages with " at row:col"; the caller shows the place.
        if let Some(at) = message.rfind(" at ") {
            message.truncate(at);
        }
        let mut chars = message.chars();
        let message = match chars.next() {
            Some(c) => c.to_uppercase().chain(chars).collect::<String>() + ".",
            None => message,
        };
        ToolError::at(message, text_pos_offset(text, pos))
    })
}

pub fn check(text: &str) -> Result<(), ToolError> {
    parse_strict(text).map(|_| ())
}

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// Decodes the five predefined entities and numeric character references.
pub fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest
            .find(';')
            .filter(|&s| s > 1 && s < 12)
            .and_then(|semi| {
                let name = &rest[1..semi];
                let c = match name {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ => name.strip_prefix('#').and_then(|num| {
                        match num.strip_prefix('x') {
                            Some(hex) => u32::from_str_radix(hex, 16).ok(),
                            None => num.parse().ok(),
                        }
                        .and_then(char::from_u32)
                    }),
                };
                c.map(|c| (c, semi + 1))
            });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Where the elements are, for finding the XPath at the caret.
pub struct Outline {
    elements: Vec<OutlineElement>,
}

struct OutlineElement {
    /// From the start tag's `<` to the end tag's `>`.
    range: Range<usize>,
    /// The start tag alone.
    tag: Range<usize>,
    /// The step for this element, like `item[2]` (the position only when
    /// it has siblings of the same name).
    step: String,
    parent: Option<usize>,
    attrs: Vec<(Range<usize>, String)>,
}

impl Outline {
    /// Builds the outline, or `None` if an end tag doesn't match. Elements
    /// not closed yet run to the end of the text.
    pub fn new(text: &str) -> Option<Self> {
        let tree = parse_tree(text, true).ok()?;
        let mut elements = Vec::new();
        fn walk(
            tree: &Tree,
            children: &[Child],
            parent: Option<usize>,
            out: &mut Vec<OutlineElement>,
        ) {
            let names: Vec<&str> = children
                .iter()
                .filter_map(|c| match c {
                    Child::Element(e) => Some(start_name(tree.text, &tree.items[e.start])),
                    Child::Item(_) => None,
                })
                .collect();
            let mut seen = 0;
            for child in children {
                let Child::Element(e) = child else { continue };
                let name = names[seen];
                let position = names[..seen].iter().filter(|n| **n == name).count() + 1;
                let total = names.iter().filter(|n| **n == name).count();
                seen += 1;
                let start = &tree.items[e.start];
                let end = match (e.end, &start.kind) {
                    (Some(i), _) => tree.items[i].range.end,
                    (
                        None,
                        ItemKind::Start {
                            self_closing: true, ..
                        },
                    ) => start.range.end,
                    // Not closed yet: it runs past the end of the text, so a
                    // caret at the very end is still inside.
                    (None, _) => usize::MAX,
                };
                let attrs = match &start.kind {
                    ItemKind::Start { attrs, .. } => attrs
                        .iter()
                        .map(|a| (a.range.clone(), tree.text[a.name.clone()].to_owned()))
                        .collect(),
                    _ => Vec::new(),
                };
                let id = out.len();
                out.push(OutlineElement {
                    range: start.range.start..end,
                    tag: start.range.clone(),
                    step: if total > 1 {
                        format!("{name}[{position}]")
                    } else {
                        name.to_owned()
                    },
                    parent,
                    attrs,
                });
                walk(tree, &e.children, Some(id), out);
            }
        }
        walk(&tree, &tree.top, None, &mut elements);
        Some(Self { elements })
    }

    /// The XPath of the innermost element at `offset` (and of the attribute,
    /// if the offset is inside one), like `/catalog/book[2]/@id`.
    pub fn path_at(&self, offset: usize) -> Option<String> {
        // Elements are in document order, so the last one containing the offset is the innermost.
        let (id, element) = self
            .elements
            .iter()
            .enumerate()
            .rfind(|(_, e)| e.range.contains(&offset))?;
        let mut steps = Vec::new();
        let mut cur = Some(id);
        while let Some(i) = cur {
            steps.push(self.elements[i].step.as_str());
            cur = self.elements[i].parent;
        }
        steps.reverse();
        let mut path = format!("/{}", steps.join("/"));
        if element.tag.contains(&offset)
            && let Some((_, name)) = element.attrs.iter().find(|(r, _)| r.contains(&offset))
        {
            path.push_str("/@");
            path.push_str(name);
        }
        Some(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_prints_and_linearizes() {
        let src = "<?xml version=\"1.0\"?>\n<!DOCTYPE r [<!ENTITY e \"x\">]><r a='1'  b=\"2\"><!-- c --><x>text &e; </x><y/><z>\n</z><p>mixed <b>bold</b> text</p></r>";
        let pretty = pretty_print(src, "  ", false).unwrap();
        assert_eq!(
            pretty,
            "<?xml version=\"1.0\"?>\n<!DOCTYPE r [<!ENTITY e \"x\">]>\n<r a='1' b=\"2\">\n  <!-- c -->\n  <x>text &e; </x>\n  <y/>\n  <z>\n</z>\n  <p>mixed <b>bold</b> text</p>\n</r>"
        );
        let attrs = pretty_print("<r a='1' b='2'><s c='3'/></r>", "\t", true).unwrap();
        assert_eq!(attrs, "<r\n\ta='1'\n\tb='2'>\n\t<s c='3'/>\n</r>");
        assert_eq!(
            linearize(&pretty).unwrap(),
            "<?xml version=\"1.0\"?><!DOCTYPE r [<!ENTITY e \"x\">]><r a='1' b=\"2\"><!-- c --><x>text &e; </x><y/><z></z><p>mixed <b>bold</b> text</p></r>"
        );
        let preserve = "<r><pre xml:space=\"preserve\">  <a/>\n  b </pre></r>";
        assert_eq!(
            pretty_print(preserve, "  ", false).unwrap(),
            "<r>\n  <pre xml:space=\"preserve\">  <a/>\n  b </pre>\n</r>"
        );
    }

    #[test]
    fn structure_errors_have_positions() {
        let cases = [
            ("<a><b></a>", 6, "Expected </b>"),
            ("<a>", 0, "never closed"),
            ("</a>", 0, "no matching"),
            ("<a x=1/>", 5, "quoted"),
            ("<a x='1/>", 5, "never closed"),
            ("<a><!-- x</a>", 3, "comment"),
            ("<a b='1'c='2'/>", 8, "a space"),
        ];
        for (text, offset, words) in cases {
            let e = pretty_print(text, "  ", false).unwrap_err();
            assert_eq!(e.offset, Some(offset), "{text}: {}", e.message);
            assert!(e.message.contains(words), "{text}: {}", e.message);
        }
    }

    #[test]
    fn strict_check() {
        assert!(check("<a><b x='1'/>&amp;</a>").is_ok());
        assert!(check("<!DOCTYPE a [<!ENTITY e \"x\">]><a>&e;</a>").is_ok());
        let e = check("<a>\n  <b>&nbsp;</b>\n</a>").unwrap_err();
        assert_eq!(
            super::super::line_col("<a>\n  <b>&nbsp;</b>\n</a>", e.offset.unwrap()).0,
            2
        );
        assert!(e.message.ends_with('.'));
        assert!(e.message.chars().next().unwrap().is_uppercase());
        let e = check("<a></b>").unwrap_err();
        assert!(e.offset.is_some());
    }

    #[test]
    fn escape_round_trip() {
        let s = "a < b && c > \"d\" 'e'";
        assert_eq!(
            escape(s),
            "a &lt; b &amp;&amp; c &gt; &quot;d&quot; &apos;e&apos;"
        );
        assert_eq!(unescape(&escape(s)), s);
        assert_eq!(unescape("&#65;&#x42;&bogus;&"), "AB&bogus;&");
    }

    #[test]
    fn xpath_at_offset() {
        let src = "<root><item id='1'><name>a</name></item><item id='2'><name>b</name></item><other/></root>";
        let outline = Outline::new(src).unwrap();
        let at = |needle: &str| outline.path_at(src.find(needle).unwrap() + 1);
        assert_eq!(at("<root").as_deref(), Some("/root"));
        assert_eq!(at("id='2'").as_deref(), Some("/root/item[2]/@id"));
        assert_eq!(at(">b<").as_deref(), Some("/root/item[2]/name"));
        assert_eq!(at("<other").as_deref(), Some("/root/other"));
        assert_eq!(outline.path_at(src.len() + 5), None);
    }

    #[test]
    fn xpath_while_typing() {
        let src = "<root><item a='1'>x</item><item>y";
        let outline = Outline::new(src).unwrap();
        assert_eq!(outline.path_at(src.len()).as_deref(), Some("/root/item[2]"));
        assert_eq!(
            outline.path_at(src.find("a=").unwrap()).as_deref(),
            Some("/root/item[1]/@a")
        );
        assert!(Outline::new("<root></item>").is_none());
        assert!(check(src).is_err());
    }

    #[test]
    fn unknown_tokens_are_explained() {
        let err = check("<a/>\nx").unwrap_err();
        assert_eq!(err.message, "Text isn't allowed outside the root element.");
        assert_eq!(err.offset, Some(5));
        let err = check("<a/><b/>").unwrap_err();
        assert!(err.message.contains("one root element"));
        assert_eq!(err.offset, Some(4));
    }
}
