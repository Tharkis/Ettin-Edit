//! A file's bytes for the hex view, edited as bytes rather than text.
//!
//! The bytes are a piece table: pieces of the original (in memory, or read
//! from the file a page at a time for very large files) and of the bytes
//! typed since. An edit only changes the list of pieces, so changing a byte
//! in a file of several gigabytes is as quick as in a small one, and undo
//! just puts an older list back.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Files up to this size are read into memory; bigger ones are read from
/// disk as they're shown.
pub const MEMORY_LIMIT: u64 = 64 << 20;
/// How many bytes are read from disk at a time.
const PAGE: usize = 64 << 10;
/// Pages kept in memory (16 MB).
const MAX_PAGES: usize = 256;
/// How much of a file is looked at to decide whether it's binary.
const SNIFF: usize = 8 << 10;

/// Where the original bytes come from.
enum Source {
    Memory(Arc<Vec<u8>>),
    File(Paged),
}

/// A file read a page at a time.
struct Paged {
    file: Mutex<File>,
    path: PathBuf,
    len: usize,
    pages: Mutex<HashMap<usize, Arc<Vec<u8>>>>,
}

impl Paged {
    fn new(file: File, path: PathBuf, len: usize) -> Self {
        Self {
            file: Mutex::new(file),
            path,
            len,
            pages: Mutex::new(HashMap::new()),
        }
    }

    fn page(&self, n: usize) -> io::Result<Arc<Vec<u8>>> {
        if let Some(p) = self.pages.lock().unwrap().get(&n) {
            return Ok(p.clone());
        }
        let start = n * PAGE;
        let len = PAGE.min(self.len.saturating_sub(start));
        let mut buf = vec![0; len];
        {
            let mut file = self.file.lock().unwrap();
            file.seek(SeekFrom::Start(start as u64))?;
            file.read_exact(&mut buf)?;
        }
        let page = Arc::new(buf);
        let mut pages = self.pages.lock().unwrap();
        if pages.len() >= MAX_PAGES {
            pages.clear();
        }
        pages.insert(n, page.clone());
        Ok(page)
    }

    /// Appends `range` of the file to `out`. A file that can't be read any
    /// more (deleted, or cut short by another program) gives zeros.
    fn read_into(&self, range: Range<usize>, out: &mut Vec<u8>) {
        let mut at = range.start;
        while at < range.end {
            let n = at / PAGE;
            let off = at - n * PAGE;
            let take = (PAGE - off).min(range.end - at);
            match self.page(n) {
                Ok(page) if page.len() >= off + take => {
                    out.extend_from_slice(&page[off..off + take])
                }
                _ => out.resize(out.len() + take, 0),
            }
            at += take;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Piece {
    /// From the bytes added since opening, rather than the original.
    added: bool,
    start: usize,
    len: usize,
}

/// A state to go back to.
#[derive(Clone)]
struct Step {
    pieces: Vec<Piece>,
    len: usize,
    version: u64,
}

pub struct ByteStore {
    source: Source,
    added: Vec<u8>,
    pieces: Vec<Piece>,
    len: usize,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// Identifies the bytes as they are now; every edit gets a new one.
    version: u64,
    next_version: u64,
    /// The version that matches the file on disk.
    saved: u64,
}

impl Default for ByteStore {
    fn default() -> Self {
        Self::from_bytes(Vec::new())
    }
}

/// Whether the start of a file looks like binary data rather than text: it
/// holds a NUL byte, and isn't UTF-16 (where NULs are normal).
pub fn looks_binary(head: &[u8]) -> bool {
    let head = &head[..head.len().min(SNIFF)];
    if !head.contains(&0) {
        return false;
    }
    let utf16 = crate::encoding::TextEncoding::detect(head, None);
    !matches!(utf16, crate::encoding::TextEncoding::Utf16 { .. })
}

/// Reads the first few kilobytes of a file, to see if it's binary.
pub fn sniff(path: &Path) -> io::Result<bool> {
    let mut head = Vec::with_capacity(SNIFF);
    File::open(path)?
        .take(SNIFF as u64)
        .read_to_end(&mut head)?;
    Ok(looks_binary(&head))
}

impl ByteStore {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        Self::with_source(Source::Memory(Arc::new(bytes)), len)
    }

    fn with_source(source: Source, len: usize) -> Self {
        let pieces = if len == 0 {
            Vec::new()
        } else {
            vec![Piece {
                added: false,
                start: 0,
                len,
            }]
        };
        Self {
            source,
            added: Vec::new(),
            pieces,
            len,
            undo: Vec::new(),
            redo: Vec::new(),
            version: 0,
            next_version: 1,
            saved: 0,
        }
    }

    /// Opens a file: small ones are read whole, big ones a page at a time.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        if len <= MEMORY_LIMIT {
            let mut bytes = Vec::with_capacity(len as usize);
            (&file).read_to_end(&mut bytes)?;
            return Ok(Self::from_bytes(bytes));
        }
        let len = usize::try_from(len)
            .map_err(|_| io::Error::new(io::ErrorKind::FileTooLarge, "file too large"))?;
        Ok(Self::with_source(
            Source::File(Paged::new(file, path.to_path_buf(), len)),
            len,
        ))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the bytes are read from disk as needed (a very large file).
    pub fn is_paged(&self) -> bool {
        matches!(self.source, Source::File(_))
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn is_modified(&self) -> bool {
        self.version != self.saved
    }

    pub fn mark_saved(&mut self) {
        self.saved = self.version;
    }

    /// Counts the bytes as changed (restored from a backup, say).
    pub fn mark_modified(&mut self) {
        self.saved = u64::MAX;
    }

    /// The bytes in `range` (cut to the length).
    pub fn read(&self, range: Range<usize>) -> Vec<u8> {
        let end = range.end.min(self.len);
        let start = range.start.min(end);
        let mut out = Vec::with_capacity(end - start);
        let mut at = 0;
        for p in &self.pieces {
            let (p_start, p_end) = (at, at + p.len);
            at = p_end;
            if p_end <= start {
                continue;
            }
            if p_start >= end {
                break;
            }
            let from = start.max(p_start) - p_start + p.start;
            let to = end.min(p_end) - p_start + p.start;
            if p.added {
                out.extend_from_slice(&self.added[from..to]);
            } else {
                match &self.source {
                    Source::Memory(b) => out.extend_from_slice(&b[from..to]),
                    Source::File(f) => f.read_into(from..to, &mut out),
                }
            }
        }
        out
    }

    pub fn get(&self, at: usize) -> Option<u8> {
        (at < self.len).then(|| self.read(at..at + 1)[0])
    }

    /// All the bytes.
    pub fn to_vec(&self) -> Vec<u8> {
        self.read(0..self.len)
    }

    /// Replaces `range` with `with`, as one step to undo.
    pub fn replace(&mut self, range: Range<usize>, with: &[u8]) {
        let end = range.end.min(self.len);
        let start = range.start.min(end);
        if start == end && with.is_empty() {
            return;
        }
        self.undo.push(self.step());
        self.redo.clear();
        let mut pieces = Vec::with_capacity(self.pieces.len() + 2);
        let mut at = 0;
        let mut inserted = false;
        let insert = |pieces: &mut Vec<Piece>, added: &mut Vec<u8>| {
            if !with.is_empty() {
                pieces.push(Piece {
                    added: true,
                    start: added.len(),
                    len: with.len(),
                });
                added.extend_from_slice(with);
            }
        };
        for p in &self.pieces {
            let (p_start, p_end) = (at, at + p.len);
            at = p_end;
            // The part before the range.
            if p_start < start {
                let keep = p_end.min(start) - p_start;
                pieces.push(Piece { len: keep, ..*p });
            }
            if !inserted && p_end >= start {
                insert(&mut pieces, &mut self.added);
                inserted = true;
            }
            // The part after the range.
            if p_end > end {
                let skip = end.max(p_start) - p_start;
                pieces.push(Piece {
                    added: p.added,
                    start: p.start + skip,
                    len: p.len - skip,
                });
            }
        }
        if !inserted {
            insert(&mut pieces, &mut self.added);
        }
        // Join pieces that follow on from each other (typing a run of bytes).
        pieces.dedup_by(|b, a| {
            if a.added == b.added && a.start + a.len == b.start {
                a.len += b.len;
                true
            } else {
                false
            }
        });
        self.pieces = pieces;
        self.len = self.len - (end - start) + with.len();
        self.version = self.next_version;
        self.next_version += 1;
    }

    fn step(&self) -> Step {
        Step {
            pieces: self.pieces.clone(),
            len: self.len,
            version: self.version,
        }
    }

    fn restore(&mut self, step: Step) {
        self.pieces = step.pieces;
        self.len = step.len;
        self.version = step.version;
    }

    pub fn undo(&mut self) -> bool {
        let Some(step) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.step());
        self.restore(step);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(step) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.step());
        self.restore(step);
        true
    }

    /// Writes the bytes, a megabyte at a time.
    pub fn write_to(&self, out: &mut impl Write) -> io::Result<()> {
        const CHUNK: usize = 1 << 20;
        let mut at = 0;
        while at < self.len {
            let end = (at + CHUNK).min(self.len);
            out.write_all(&self.read(at..end))?;
            at = end;
        }
        Ok(())
    }

    /// Saves to `path` through a temporary file next to it, so the file
    /// being read from is never overwritten while it's still needed. A big
    /// file is then read from its new copy, and its undo history starts again.
    pub fn save(&mut self, path: &Path) -> io::Result<()> {
        let tmp = temp_path(path);
        let written = File::create(&tmp).and_then(|file| {
            let mut out = BufWriter::new(file);
            self.write_to(&mut out)?;
            out.into_inner().map_err(|e| e.into_error())?.sync_all()
        });
        if let Err(e) = written {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        if !self.is_paged() {
            if let Err(e) = replace_file(&tmp, path) {
                let _ = fs::remove_file(&tmp);
                return Err(e);
            }
            self.mark_saved();
            return Ok(());
        }
        let mut replaced = replace_file(&tmp, path);
        if replaced.is_err()
            && let Source::File(paged) = &self.source
        {
            // Windows won't replace a file that's open: let go of it and try
            // again. If that fails too, the file is as it was, so open it again.
            let source_path = paged.path.clone();
            self.source = Source::Memory(Arc::default());
            replaced = replace_file(&tmp, path);
            if replaced.is_err() {
                let file = File::open(&source_path)?;
                self.source = Source::File(Paged::new(file, source_path, self.original_len()));
            }
        }
        if let Err(e) = replaced {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        *self = Self::open(path)?;
        Ok(())
    }

    /// The length of the original bytes (before any edits).
    fn original_len(&self) -> usize {
        match &self.source {
            Source::Memory(b) => b.len(),
            Source::File(f) => f.len,
        }
    }
}

fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.ettin-save"))
}

/// Moves `from` over `to`, keeping `to`'s permissions when it exists.
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    if let Ok(meta) = fs::metadata(to) {
        let _ = fs::set_permissions(from, meta.permissions());
    }
    fs::rename(from, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(bytes: &[u8]) -> ByteStore {
        ByteStore::from_bytes(bytes.to_vec())
    }

    #[test]
    fn edits_and_reads() {
        let mut s = store(b"hello world");
        s.replace(0..1, b"J");
        assert_eq!(s.to_vec(), b"Jello world");
        s.replace(5..5, b",");
        assert_eq!(s.to_vec(), b"Jello, world");
        s.replace(7..12, b"");
        assert_eq!(s.to_vec(), b"Jello, ");
        s.replace(7..7, b"there");
        assert_eq!(s.to_vec(), b"Jello, there");
        assert_eq!(s.len(), 12);
        assert_eq!(s.read(3..8), b"lo, t");
        assert_eq!(s.get(0), Some(b'J'));
        assert_eq!(s.get(12), None);
        // Any byte goes, valid text or not.
        s.replace(0..2, &[0xC3, 0x00]);
        assert_eq!(s.read(0..3), [0xC3, 0x00, b'l']);
    }

    #[test]
    fn typing_joins_pieces() {
        let mut s = store(&[0; 100]);
        for i in 10..60 {
            s.replace(i..i + 1, &[0xAA]);
        }
        assert_eq!(s.pieces.len(), 3);
        assert_eq!(s.read(9..11), [0, 0xAA]);
        assert_eq!(s.read(59..61), [0xAA, 0]);
    }

    #[test]
    fn undo_redo_and_modified() {
        let mut s = store(b"abc");
        assert!(!s.is_modified());
        s.replace(1..2, b"X");
        s.replace(3..3, b"!");
        assert_eq!(s.to_vec(), b"aXc!");
        assert!(s.is_modified());
        assert!(s.undo());
        assert_eq!(s.to_vec(), b"aXc");
        assert!(s.undo());
        assert_eq!(s.to_vec(), b"abc");
        assert!(!s.is_modified());
        assert!(!s.undo());
        assert!(s.redo());
        assert_eq!(s.to_vec(), b"aXc");
        s.mark_saved();
        assert!(!s.is_modified());
        assert!(s.undo());
        assert!(s.is_modified());
        // A new edit drops what could be redone.
        s.replace(0..0, b">");
        assert!(!s.redo());
        assert_eq!(s.to_vec(), b">abc");
    }

    #[test]
    fn spots_binary_files() {
        assert!(!looks_binary(b"plain text\n"));
        assert!(looks_binary(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"));
        // UTF-16 text is full of NULs but isn't binary.
        let utf16: Vec<u8> = "Hello, world\n"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        assert!(!looks_binary(&utf16));
    }

    #[test]
    fn big_files_are_read_in_pages_and_saved() {
        let dir = std::env::temp_dir().join(format!("ettin-bytes-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.bin");
        let data: Vec<u8> = (0..(PAGE * 3 + 123)).map(|i| (i % 251) as u8).collect();
        fs::write(&path, &data).unwrap();
        let mut s = ByteStore::open(&path).unwrap();
        // Pretend it's big, to go through the paging.
        s.source = Source::File(Paged::new(
            File::open(&path).unwrap(),
            path.clone(),
            data.len(),
        ));
        assert!(s.is_paged());
        let mid = PAGE - 2..PAGE + 2;
        assert_eq!(s.read(mid.clone()), data[mid]);
        s.replace(PAGE..PAGE + 1, &[1, 2, 3]);
        s.replace(5..10, b"");
        let mut want = data.clone();
        want.splice(PAGE..PAGE + 1, [1, 2, 3]);
        want.drain(5..10);
        assert_eq!(s.to_vec(), want);
        s.save(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), want);
        assert_eq!(s.to_vec(), want);
        assert!(!s.is_modified());
        assert!(!s.undo());
        assert!(!temp_path(&path).exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
