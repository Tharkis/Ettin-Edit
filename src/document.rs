//! An open document: its text, where it lives on disk, and how it should be
//! written back (line endings, byte-order mark).

use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::editor::Editor;

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
    /// The text (line endings normalised to `\n`), selections and undo history.
    pub editor: Editor,
    /// Name of the syntect syntax used for highlighting.
    pub syntax: String,
    pub line_ending: LineEnding,
    pub has_bom: bool,
    /// Set when the file contained bytes that were not valid UTF-8.
    pub lossy: bool,
    untitled_number: usize,
}

impl Document {
    pub fn new_untitled(id: u64, number: usize) -> Self {
        Self {
            id,
            path: None,
            editor: Editor::new(""),
            syntax: "Plain Text".to_owned(),
            line_ending: LineEnding::platform_default(),
            has_bom: false,
            lossy: false,
            untitled_number: number,
        }
    }

    pub fn open(id: u64, path: &Path) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        let (has_bom, body) = match bytes.strip_prefix(UTF8_BOM) {
            Some(rest) => (true, rest),
            None => (false, &bytes[..]),
        };
        let (raw, lossy) = match std::str::from_utf8(body) {
            Ok(s) => (std::borrow::Cow::Borrowed(s), false),
            Err(_) => (String::from_utf8_lossy(body), true),
        };
        let line_ending = LineEnding::detect(&raw).unwrap_or_else(LineEnding::platform_default);
        let text = if raw.contains('\r') {
            raw.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            raw.into_owned()
        };

        let mut doc = Self::new_untitled(id, 0);
        doc.path = Some(path.to_path_buf());
        doc.editor = Editor::new(&text);
        doc.line_ending = line_ending;
        doc.has_bom = has_bom;
        doc.lossy = lossy;
        Ok(doc)
    }

    pub fn save_to(&mut self, path: &Path) -> io::Result<()> {
        let file = fs::File::create(path)?;
        let mut out = BufWriter::new(file);
        if self.has_bom {
            out.write_all(UTF8_BOM)?;
        }
        let eol = self.line_ending.as_str();
        for chunk in self.editor.buffer.rope().chunks() {
            if self.line_ending == LineEnding::Lf {
                out.write_all(chunk.as_bytes())?;
            } else {
                for (i, part) in chunk.split('\n').enumerate() {
                    if i > 0 {
                        out.write_all(eol.as_bytes())?;
                    }
                    out.write_all(part.as_bytes())?;
                }
            }
        }
        out.flush()?;
        self.path = Some(path.to_path_buf());
        self.editor.buffer.mark_saved();
        self.lossy = false;
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.editor.buffer.is_modified()
    }

    pub fn set_line_ending(&mut self, le: LineEnding) {
        if self.line_ending != le {
            self.line_ending = le;
            self.editor.buffer.mark_modified();
        }
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
        self.path.is_none() && !self.is_dirty() && self.editor.buffer.is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.editor.buffer.len_lines()
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
        assert_eq!(doc.editor.buffer.text(), "one\ntwo\n");
        assert!(doc.has_bom);
        assert_eq!(doc.line_ending, LineEnding::CrLf);

        doc.save_to(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!doc.is_dirty());
        fs::remove_dir_all(&dir).ok();
    }
}
