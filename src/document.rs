//! An open document: its text, where it lives on disk, and how it should be
//! written back (line endings, encoding).

use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::bytes::ByteStore;
use crate::editor::Editor;
use crate::encoding::TextEncoding;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    pub fn label(self) -> String {
        match self {
            LineEnding::Lf => t!("eol-unix"),
            LineEnding::CrLf => t!("eol-windows"),
            LineEnding::Cr => t!("eol-mac"),
        }
    }

    /// The short name shown at line ends: LF, CRLF or CR.
    pub fn short_label(self) -> &'static str {
        match self {
            LineEnding::Lf => "LF",
            LineEnding::CrLf => "CRLF",
            LineEnding::Cr => "CR",
        }
    }

    pub fn as_str(self) -> &'static str {
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
    pub(crate) fn detect(text: &str) -> Option<Self> {
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

/// When and how big the file was when it was last read or written, to notice
/// changes made by other programs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskStamp {
    pub modified: Option<SystemTime>,
    pub len: u64,
}

impl DiskStamp {
    pub fn read(path: &Path) -> io::Result<Self> {
        let meta = fs::metadata(path)?;
        Ok(Self {
            modified: meta.modified().ok(),
            len: meta.len(),
        })
    }
}

/// What happened to the file on disk since it was opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskChange {
    Modified,
    Deleted,
}

pub struct Document {
    pub id: u64,
    pub path: Option<PathBuf>,
    /// The text (line endings normalised to `\n`), selections and undo history.
    pub editor: Editor,
    /// Name of the syntect syntax used for highlighting.
    pub syntax: String,
    pub line_ending: LineEnding,
    pub encoding: TextEncoding,
    /// Set when the file contained bytes that could not be decoded.
    pub lossy: bool,
    pub pinned: bool,
    /// The file as last read or saved; `None` for new files.
    pub disk: Option<DiskStamp>,
    /// A change on disk the user hasn't dealt with yet.
    pub disk_change: Option<DiskChange>,
    /// Where unsaved changes are backed up, once they have been.
    pub backup: Option<PathBuf>,
    /// The buffer version the backup holds.
    pub backup_version: u64,
    pub untitled_number: usize,
    /// Which views of a split show this tab: `[main, other]`. Both for a
    /// document cloned into the other view.
    pub views: [bool; 2],
    /// The view whose carets are live in `editor`; the other view's are kept
    /// aside (see `Editor::swap_view`).
    pub live_view: usize,
    /// The bytes, when the tab is edited as bytes in the hex view: a binary
    /// file, or text that a hex edit left invalid. Saving writes these, and
    /// the text in `editor` isn't used.
    pub binary: Option<ByteStore>,
    /// The bytes came from the tab's text, so it turns back into text as
    /// soon as they're valid again.
    pub binary_from_text: bool,
}

impl Document {
    pub fn new_untitled(id: u64, number: usize) -> Self {
        Self {
            id,
            path: None,
            editor: Editor::new(""),
            syntax: "Plain Text".to_owned(),
            line_ending: LineEnding::platform_default(),
            encoding: TextEncoding::default(),
            lossy: false,
            pinned: false,
            disk: None,
            disk_change: None,
            backup: None,
            backup_version: 0,
            untitled_number: number,
            views: [true, false],
            live_view: 0,
            binary: None,
            binary_from_text: false,
        }
    }

    /// Opens a file, detecting its encoding. `hint` is an encoding the file
    /// had before, used when the bytes alone don't settle it.
    /// Binary files (see [`crate::bytes::looks_binary`]) open as bytes.
    pub fn open(id: u64, path: &Path, hint: Option<TextEncoding>) -> io::Result<Self> {
        if crate::bytes::sniff(path)? {
            return Self::open_bytes(id, path);
        }
        let bytes = fs::read(path)?;
        let disk = DiskStamp::read(path).ok();
        let encoding = TextEncoding::detect(&bytes, hint);
        let (text, line_ending, lossy) = decode(&bytes, encoding);

        let mut doc = Self::new_untitled(id, 0);
        doc.path = Some(path.to_path_buf());
        doc.editor = Editor::new(&text);
        doc.line_ending = line_ending;
        doc.encoding = encoding;
        doc.lossy = lossy;
        doc.disk = disk;
        Ok(doc)
    }

    /// Opens a file as bytes for the hex view, without reading it as text.
    /// Very large files are read from disk as they're shown.
    pub fn open_bytes(id: u64, path: &Path) -> io::Result<Self> {
        let mut doc = Self::new_untitled(id, 0);
        doc.binary = Some(ByteStore::open(path)?);
        doc.path = Some(path.to_path_buf());
        doc.disk = DiskStamp::read(path).ok();
        Ok(doc)
    }

    /// Reads the file again as one undoable edit. `encoding` forces an
    /// encoding; otherwise it is detected, preferring the current one.
    pub fn reload(&mut self, encoding: Option<TextEncoding>) -> io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        // A tab edited as bytes reads the bytes again, unless an encoding
        // was picked to read it as text.
        if self.binary.is_some() && encoding.is_none() {
            self.binary = Some(ByteStore::open(&path)?);
            self.binary_from_text = false;
            self.editor.buffer.mark_saved();
            self.disk = DiskStamp::read(&path).ok();
            self.disk_change = None;
            return Ok(());
        }
        self.binary = None;
        self.binary_from_text = false;
        let bytes = fs::read(&path)?;
        let encoding =
            encoding.unwrap_or_else(|| TextEncoding::detect(&bytes, Some(self.encoding)));
        let (text, line_ending, lossy) = decode(&bytes, encoding);
        self.editor.reload_text(&text);
        self.editor.buffer.mark_saved();
        self.encoding = encoding;
        self.line_ending = line_ending;
        self.lossy = lossy;
        self.disk = DiskStamp::read(&path).ok();
        self.disk_change = None;
        Ok(())
    }

    pub fn save_to(&mut self, path: &Path) -> io::Result<()> {
        let eol = self.line_ending.as_str();
        if let Some(bytes) = &mut self.binary {
            bytes.save(path)?;
        } else if self.encoding.is_utf8() {
            // Stream UTF-8 straight from the rope, so big files need no copy.
            let file = fs::File::create(path)?;
            let mut out = BufWriter::new(file);
            out.write_all(&self.encoding.encode("").unwrap_or_default())?;
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
        } else {
            let mut text = self.editor.buffer.text();
            if self.line_ending != LineEnding::Lf {
                text = text.replace('\n', eol);
            }
            let bytes = self.encoding.encode(&text).map_err(|c| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    t!(
                        "cant-encode-char",
                        char = c.to_string(),
                        code = format!("{:04X}", c as u32),
                        encoding = self.encoding.name()
                    ),
                )
            })?;
            fs::write(path, bytes)?;
        }
        self.path = Some(path.to_path_buf());
        self.editor.buffer.mark_saved();
        self.lossy = false;
        self.disk = DiskStamp::read(path).ok();
        self.disk_change = None;
        Ok(())
    }

    /// Checks the file on disk against what was last read or saved.
    pub fn check_disk(&self) -> Option<DiskChange> {
        let path = self.path.as_ref()?;
        let known = self.disk?;
        match DiskStamp::read(path) {
            Ok(now) if now != known => Some(DiskChange::Modified),
            Ok(_) => None,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Some(DiskChange::Deleted),
            Err(_) => None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.editor.buffer.is_modified() || self.binary.as_ref().is_some_and(|b| b.is_modified())
    }

    pub fn set_line_ending(&mut self, le: LineEnding) {
        if self.line_ending != le {
            self.line_ending = le;
            self.editor.buffer.mark_modified();
        }
    }

    pub fn set_encoding(&mut self, encoding: TextEncoding) {
        if self.encoding != encoding {
            self.encoding = encoding;
            self.editor.buffer.mark_modified();
        }
    }

    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            None => t!("untitled-name", number = self.untitled_number),
        }
    }

    pub fn is_pristine_untitled(&self) -> bool {
        self.path.is_none()
            && !self.is_dirty()
            && self.editor.buffer.is_empty()
            && self.binary.is_none()
    }

    pub fn line_count(&self) -> usize {
        self.editor.buffer.len_lines()
    }
}

/// Decodes file bytes and normalises line endings to `\n`.
fn decode(bytes: &[u8], encoding: TextEncoding) -> (String, LineEnding, bool) {
    let (raw, lossy) = encoding.decode(bytes);
    let line_ending = LineEnding::detect(&raw).unwrap_or_else(LineEnding::platform_default);
    let text = if raw.contains('\r') {
        raw.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        raw
    };
    (text, line_ending, lossy)
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
        let dir = std::env::temp_dir().join(format!("ettin-edit-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("crlf.txt");
        let original = b"\xEF\xBB\xBFone\r\ntwo\r\n";
        fs::write(&path, original).unwrap();

        let mut doc = Document::open(1, &path, None).unwrap();
        assert_eq!(doc.editor.buffer.text(), "one\ntwo\n");
        assert_eq!(doc.encoding, TextEncoding::Utf8 { bom: true });
        assert_eq!(doc.line_ending, LineEnding::CrLf);

        doc.save_to(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!doc.is_dirty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn legacy_encodings_and_reload() {
        let dir = std::env::temp_dir().join(format!("ettin-edit-test-enc-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("latin.txt");
        fs::write(&path, b"caf\xE9\r\nna\xEFve\r\n").unwrap();

        let mut doc = Document::open(1, &path, None).unwrap();
        assert_eq!(doc.editor.buffer.text(), "café\nnaïve\n");
        assert_eq!(
            doc.encoding,
            TextEncoding::Legacy(encoding_rs::WINDOWS_1252)
        );
        assert!(!doc.lossy);
        doc.save_to(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"caf\xE9\r\nna\xEFve\r\n");

        // A character the encoding lacks stops the save instead of losing it.
        doc.editor.reload_text("café €5 ж\n");
        assert!(doc.save_to(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"caf\xE9\r\nna\xEFve\r\n");

        // Another program changes the file; reloading is one undo step.
        fs::write(&path, b"new\r\n").unwrap();
        doc.disk = Some(DiskStamp {
            modified: None,
            len: 0,
        });
        assert_eq!(doc.check_disk(), Some(DiskChange::Modified));
        doc.reload(None).unwrap();
        assert_eq!(doc.editor.buffer.text(), "new\n");
        assert!(!doc.is_dirty());
        assert_eq!(doc.check_disk(), None);
        doc.editor.undo();
        assert_eq!(doc.editor.buffer.text(), "café €5 ж\n");

        fs::remove_file(&path).unwrap();
        assert_eq!(doc.check_disk(), Some(DiskChange::Deleted));
        fs::remove_dir_all(&dir).ok();
    }
}
