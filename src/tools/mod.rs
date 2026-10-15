//! The built-in text tools: MIME Tools (Base64, URL, quoted-printable, HTML
//! entities, SAML), JSON and JavaScript tools, XML Tools with XPath, and
//! CsvQuery (reading delimited text and querying it with SQL).
//! Everything here works on plain strings; the app applies the results to
//! the editor.

pub mod csv;
pub mod dtd;
pub mod js;
pub mod json;
pub mod mime;
pub mod sql;
pub mod xml;
pub mod xpath;
pub mod xsd;
pub mod xslt;

use std::fmt;

/// A tool that couldn't finish, with where in its input the problem is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolError {
    pub message: String,
    /// Byte offset into the tool's input, when the problem has a place.
    pub offset: Option<usize>,
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            offset: None,
        }
    }

    pub fn at(message: impl Into<String>, offset: usize) -> Self {
        Self {
            message: message.into(),
            offset: Some(offset),
        }
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

pub type ToolResult = Result<String, ToolError>;

/// A validation problem in a document (XSD or DTD), with where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub message: String,
    /// Byte offset into the validated document.
    pub offset: usize,
}

/// 1-based line and column (in chars) of a byte offset.
pub fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let offset = floor_char_boundary(text, offset.min(text.len()));
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, before[line_start..].chars().count() + 1)
}

pub fn floor_char_boundary(text: &str, mut i: usize) -> usize {
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    #[test]
    fn line_col_counts_chars() {
        assert_eq!(super::line_col("ab\ncé\nx", 0), (1, 1));
        assert_eq!(super::line_col("ab\ncé\nx", 6), (2, 3));
        assert_eq!(super::line_col("ab\ncé\nx", 7), (3, 1));
    }
}
