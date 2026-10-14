//! An open document: its text, where it lives on disk, and how it should be
//! written back (line endings, byte-order mark).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::highlight::HighlightCache;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    pub fn label(self) -> &'static str {
        match self {
            LineEnding::Lf => "Unix (LF)",
            LineEnding::CrLf => "Windows (CR LF)",
            LineEnding::Cr => "Macintosh (CR)",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }

    pub fn platform_default() -> Self {
        if cfg!(windows) {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        }
    }

    /// Picks the most common line ending in `text`.
    fn detect(text: &str) -> Option<Self> {
        let crlf = text.matches("\r\n").count();
        let cr = text.matches('\r').count() - crlf;
        let lf = text.matches('\n').count() - crlf;
        if crlf == 0 && cr == 0 && lf == 0 {
            None
        } else if crlf >= lf && crlf >= cr {
            Some(LineEnding::CrLf)
        } else if cr > lf {
            Some(LineEnding::Cr)
        } else {
            Some(LineEnding::Lf)
        }
    }
}

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

pub struct Document {
    pub id: u64,
    pub path: Option<PathBuf>,
    /// Text with line endings normalised to `\n`.
    pub text: String,
    pub dirty: bool,
    /// Name of the syntect syntax used for highlighting.
    pub syntax: String,
    pub line_ending: LineEnding,
    pub has_bom: bool,
    /// Set when the file contained bytes that were not valid UTF-8.
    pub lossy: bool,
    untitled_number: usize,
    /// Cursor position as (line, column), both 1-based, plus selection length.
    pub cursor: (usize, usize),
    pub selection_chars: usize,
    pub highlight: HighlightCache,
    /// True until the editor has placed the cursor at the start of the document.
    pub fresh: bool,
}

impl Document {
    pub fn new_untitled(id: u64, number: usize) -> Self {
        Self {
            id,
            path: None,
            text: String::new(),
            dirty: false,
            syntax: "Plain Text".to_owned(),
            line_ending: LineEnding::platform_default(),
            has_bom: false,
            lossy: false,
            untitled_number: number,
            cursor: (1, 1),
            selection_chars: 0,
            highlight: HighlightCache::default(),
            fresh: true,
        }
    }

    pub fn open(id: u64, path: &Path) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        let (has_bom, body) = match bytes.strip_prefix(UTF8_BOM) {
            Some(rest) => (true, rest),
            None => (false, &bytes[..]),
        };
        let (raw, lossy) = match std::str::from_utf8(body) {
            Ok(s) => (s.to_owned(), false),
            Err(_) => (String::from_utf8_lossy(body).into_owned(), true),
        };
        let line_ending = LineEnding::detect(&raw).unwrap_or_else(LineEnding::platform_default);
        let text = raw.replace("\r\n", "\n").replace('\r', "\n");

        let mut doc = Self::new_untitled(id, 0);
        doc.path = Some(path.to_path_buf());
        doc.text = text;
        doc.line_ending = line_ending;
        doc.has_bom = has_bom;
        doc.lossy = lossy;
        Ok(doc)
    }

    pub fn save_to(&mut self, path: &Path) -> io::Result<()> {
        let body = match self.line_ending {
            LineEnding::Lf => self.text.clone(),
            other => self.text.replace('\n', other.as_str()),
        };
        let mut bytes = Vec::with_capacity(body.len() + 3);
        if self.has_bom {
            bytes.extend_from_slice(UTF8_BOM);
        }
        bytes.extend_from_slice(body.as_bytes());
        fs::write(path, bytes)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        self.lossy = false;
        Ok(())
    }

    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            None => format!("new {}", self.untitled_number),
        }
    }

    pub fn is_pristine_untitled(&self) -> bool {
        self.path.is_none() && !self.dirty && self.text.is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.text.split('\n').count()
    }

    /// Recomputes line/column from a character offset into `text`.
    pub fn update_cursor(&mut self, char_index: usize, selection_chars: usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.text.chars().enumerate() {
            if i >= char_index {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        self.cursor = (line, col);
        self.selection_chars = selection_chars;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_line_endings() {
        assert_eq!(LineEnding::detect("a\r\nb\r\n"), Some(LineEnding::CrLf));
        assert_eq!(LineEnding::detect("a\nb\n"), Some(LineEnding::Lf));
        assert_eq!(LineEnding::detect("a\rb\r"), Some(LineEnding::Cr));
        assert_eq!(LineEnding::detect("abc"), None);
    }

    #[test]
    fn round_trips_crlf_and_bom() {
        let dir = std::env::temp_dir().join(format!("nixpad-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("crlf.txt");
        let original = b"\xEF\xBB\xBFone\r\ntwo\r\n";
        fs::write(&path, original).unwrap();

        let mut doc = Document::open(1, &path).unwrap();
        assert_eq!(doc.text, "one\ntwo\n");
        assert!(doc.has_bom);
        assert_eq!(doc.line_ending, LineEnding::CrLf);

        doc.save_to(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cursor_position() {
        let mut doc = Document::new_untitled(1, 1);
        doc.text = "ab\ncd\n".to_owned();
        doc.update_cursor(4, 0);
        assert_eq!(doc.cursor, (2, 2));
    }
}
