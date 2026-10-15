//! Placeholder until the language branch brings the real parsers.
#![allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Module,
    Section,
    Heading,
    Key,
}
#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub depth: usize,
}
pub fn symbols(_language: &str, text: &str) -> Vec<Symbol> {
    text.lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let t = l.trim_start();
            let rest = t
                .strip_prefix("pub fn ")
                .or_else(|| t.strip_prefix("fn "))?;
            Some(Symbol {
                name: rest.split('(').next()?.to_owned(),
                kind: SymbolKind::Function,
                line: i,
                depth: (l.len() - t.len()) / 4,
            })
        })
        .collect()
}
