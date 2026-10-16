//! Hex view: a tab's text shown as the bytes it is saved as (in its encoding
//! and with its line endings), with byte edits turned back into text.
//!
//! A byte edit is only applied when the new bytes read back as text and save
//! as exactly those bytes again; otherwise saving would quietly write
//! something different from what the hex view showed.

use std::ops::Range;
use std::sync::Arc;

use crate::document::LineEnding;
use crate::encoding::TextEncoding;

/// Bytes shown per row.
pub const ROW: usize = 16;

/// Why some bytes can't become the tab's text.
#[derive(Debug, PartialEq, Eq)]
pub enum HexError {
    /// They aren't valid text in the tab's encoding.
    Invalid,
    /// They read back as text, but would save differently (line breaks that
    /// don't match the tab's line ending, or a changed byte-order mark).
    RoundTrip,
    /// A character in the text can't be written in the encoding.
    Unencodable(char),
}

/// The bytes `text` (with `\n` line breaks) is saved as.
pub fn text_to_bytes(
    text: &str,
    encoding: TextEncoding,
    line_ending: LineEnding,
) -> Result<Vec<u8>, HexError> {
    let text = if line_ending == LineEnding::Lf {
        std::borrow::Cow::Borrowed(text)
    } else {
        std::borrow::Cow::Owned(text.replace('\n', line_ending.as_str()))
    };
    encoding.encode(&text).map_err(HexError::Unencodable)
}

/// The text (with `\n` line breaks) that saves as exactly `bytes`.
pub fn bytes_to_text(
    bytes: &[u8],
    encoding: TextEncoding,
    line_ending: LineEnding,
) -> Result<String, HexError> {
    let (raw, lossy) = encoding.decode(bytes);
    if lossy {
        return Err(HexError::Invalid);
    }
    let text = if raw.contains('\r') {
        raw.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        raw
    };
    match text_to_bytes(&text, encoding, line_ending) {
        Ok(again) if again == bytes => Ok(text),
        _ => Err(HexError::RoundTrip),
    }
}

/// The smallest edit that turns `old` into `new`: a char range of `old` and
/// what replaces it.
pub fn diff(old: &str, new: &str) -> (Range<usize>, String) {
    let prefix = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .count();
    let old_rest: Vec<char> = old.chars().skip(prefix).collect();
    let new_rest: Vec<char> = new.chars().skip(prefix).collect();
    let suffix = old_rest
        .iter()
        .rev()
        .zip(new_rest.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let replaced = prefix..prefix + old_rest.len() - suffix;
    let text = new_rest[..new_rest.len() - suffix].iter().collect();
    (replaced, text)
}

/// Reads hex digits (spaces, commas and `0x` prefixes allowed) as bytes.
pub fn parse_hex(text: &str) -> Option<Vec<u8>> {
    let cleaned = text.replace("0x", " ").replace("0X", " ");
    let digits: String = cleaned
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    if digits.is_empty() || !digits.len().is_multiple_of(2) {
        return None;
    }
    (0..digits.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(digits.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Bytes as `48 65 6C`.
pub fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// One tab's hex view: its bytes, the byte cursor and the selection.
#[derive(Default)]
pub struct HexView {
    /// The buffer version, encoding and line ending the bytes were made from.
    source: Option<(u64, TextEncoding, LineEnding)>,
    pub bytes: Arc<Vec<u8>>,
    /// Byte the cursor is on (may equal the length: the end of the file).
    pub cursor: usize,
    /// Where a Shift-selection started.
    pub anchor: usize,
    /// The first hex digit typed for the byte at the cursor.
    pub pending: Option<u8>,
    /// Typing goes to the text column instead of the hex column.
    pub in_text: bool,
    /// A short note shown under the bytes, such as why an edit was refused.
    pub message: Option<String>,
    /// Bring the cursor into view on the next frame.
    pub scroll_to_cursor: bool,
    /// Cursor and anchor before this frame's edits, to go back to if the
    /// app refuses them.
    pub(crate) before_edit: Option<(usize, usize)>,
    /// Scroll position and visible height last frame.
    pub(crate) scroll: (f32, f32),
}

impl HexView {
    /// Makes the bytes again when the text, encoding or line ending changed.
    /// Fails with the first character the encoding can't hold.
    pub fn sync(
        &mut self,
        version: u64,
        encoding: TextEncoding,
        line_ending: LineEnding,
        text: impl FnOnce() -> Arc<String>,
    ) -> Result<(), char> {
        let key = (version, encoding, line_ending);
        if self.source == Some(key) {
            return Ok(());
        }
        self.source = Some(key);
        match text_to_bytes(&text(), encoding, line_ending) {
            Ok(bytes) => {
                self.bytes = Arc::new(bytes);
                self.cursor = self.cursor.min(self.bytes.len());
                self.anchor = self.anchor.min(self.bytes.len());
                Ok(())
            }
            Err(HexError::Unencodable(c)) => {
                self.bytes = Arc::default();
                self.cursor = 0;
                self.anchor = 0;
                Err(c)
            }
            Err(_) => Ok(()),
        }
    }

    /// Makes the bytes from the text again on the next frame (after a
    /// refused edit, so the view shows what will be saved), and puts the
    /// cursor back where it was before the edit.
    pub fn refresh(&mut self) {
        self.source = None;
        if let Some((cursor, anchor)) = self.before_edit.take() {
            self.cursor = cursor;
            self.anchor = anchor;
        }
    }

    /// The selected byte range (empty when nothing is selected).
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }

    /// Moves the cursor, extending the selection when `extend` is set.
    pub fn move_to(&mut self, to: usize, extend: bool) {
        self.cursor = to.min(self.bytes.len());
        if !extend {
            self.anchor = self.cursor;
        }
        self.pending = None;
        self.scroll_to_cursor = true;
    }

    /// The bytes with `range` replaced by `with`.
    pub fn replaced(&self, range: Range<usize>, with: &[u8]) -> Vec<u8> {
        let end = range.end.min(self.bytes.len());
        let start = range.start.min(end);
        let mut out = Vec::with_capacity(self.bytes.len() + with.len());
        out.extend_from_slice(&self.bytes[..start]);
        out.extend_from_slice(with);
        out.extend_from_slice(&self.bytes[end..]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UTF8: TextEncoding = TextEncoding::Utf8 { bom: false };

    #[test]
    fn bytes_follow_encoding_and_line_endings() {
        assert_eq!(
            text_to_bytes("a\nb", UTF8, LineEnding::CrLf).unwrap(),
            b"a\r\nb"
        );
        let bom = TextEncoding::Utf8 { bom: true };
        assert_eq!(
            text_to_bytes("é", bom, LineEnding::Lf).unwrap(),
            b"\xEF\xBB\xBF\xC3\xA9"
        );
        let le = TextEncoding::Utf16 {
            big_endian: false,
            bom: false,
        };
        assert_eq!(text_to_bytes("A", le, LineEnding::Lf).unwrap(), b"A\0");
    }

    #[test]
    fn edits_must_round_trip() {
        assert_eq!(
            bytes_to_text(b"a\r\nB", UTF8, LineEnding::CrLf).unwrap(),
            "a\nB"
        );
        // A lone CR in a Windows file would be saved as CR LF.
        assert_eq!(
            bytes_to_text(b"a\rb", UTF8, LineEnding::CrLf),
            Err(HexError::RoundTrip)
        );
        // Half a UTF-8 character.
        assert_eq!(
            bytes_to_text(b"\xC3", UTF8, LineEnding::Lf),
            Err(HexError::Invalid)
        );
        // é (C3 A9) to è (C3 A8) is fine.
        assert_eq!(
            bytes_to_text(b"\xC3\xA8", UTF8, LineEnding::Lf).unwrap(),
            "è"
        );
    }

    #[test]
    fn smallest_edit() {
        assert_eq!(diff("hello", "hallo"), (1..2, "a".to_owned()));
        assert_eq!(diff("abc", "abXc"), (2..2, "X".to_owned()));
        assert_eq!(diff("aaa", "aa"), (2..3, String::new()));
        assert_eq!(diff("é!", "è!"), (0..1, "è".to_owned()));
    }

    #[test]
    fn hex_text() {
        assert_eq!(
            parse_hex("48 65,6c 0x6C"),
            Some(vec![0x48, 0x65, 0x6C, 0x6C])
        );
        assert_eq!(parse_hex("4"), None);
        assert_eq!(parse_hex("zz"), None);
        assert_eq!(format_hex(&[0, 255, 16]), "00 FF 10");
    }
}
