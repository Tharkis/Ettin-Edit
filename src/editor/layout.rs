//! Display columns, word wrap, and the index from visual rows to lines.
//!
//! The editor uses a monospace grid: every char has a width in columns
//! (tabs expand to the next tab stop, wide CJK characters take two).

use unicode_width::UnicodeWidthChar;

use super::buffer::{Buffer, LineEdit};

pub const TAB_WIDTH: usize = 4;

/// Width of `c` in columns when it starts at column `col`.
pub fn char_cols(c: char, col: usize) -> usize {
    match c {
        '\t' => TAB_WIDTH - col % TAB_WIDTH,
        c if c.is_control() => 1,
        c => c.width().unwrap_or(1),
    }
}

/// Total display width of a line, in columns.
pub fn line_cols(chars: impl Iterator<Item = char>) -> usize {
    chars.fold(0, |col, c| col + char_cols(c, col))
}

/// Char offsets (within the line) where each visual row starts. Always starts with 0.
/// With `cols == None` the line is a single row.
pub fn wrap_points(line: &[char], cols: Option<usize>) -> Vec<usize> {
    let mut points = vec![0];
    let Some(cols) = cols.filter(|&c| c > 0) else {
        return points;
    };
    let mut row_start = 0;
    let mut col = 0;
    // The last place a row could break: just after a space.
    let mut last_break: Option<usize> = None;
    let mut i = 0;
    while i < line.len() {
        let c = line[i];
        let w = char_cols(c, col);
        if col + w > cols && i > row_start {
            let at = match last_break {
                Some(b) if b > row_start && !c.is_whitespace() => b,
                _ => i,
            };
            points.push(at);
            row_start = at;
            last_break = None;
            // Recount the columns of the chars carried over to the new row.
            col = 0;
            for &c in &line[row_start..i] {
                col += char_cols(c, col);
            }
            continue;
        }
        col += w;
        if c == ' ' || c == '\t' {
            last_break = Some(i + 1);
        }
        i += 1;
    }
    points
}

/// Per-line row counts and widths, so the view can find which line is at a
/// given scroll position without laying out the whole file.
pub struct RowIndex {
    wrap_cols: Option<usize>,
    rows: Vec<u32>,
    widths: Vec<u32>,
    /// Lines inside a folded region; they take no rows.
    hidden: Vec<bool>,
    any_hidden: bool,
    /// Prefix sums of `rows` (skipping hidden lines), rebuilt lazily.
    starts: Vec<u64>,
    starts_valid: bool,
    max_width: u32,
    max_width_valid: bool,
}

impl RowIndex {
    pub fn new(buffer: &Buffer, wrap_cols: Option<usize>) -> Self {
        let mut index = Self {
            wrap_cols,
            rows: Vec::new(),
            widths: Vec::new(),
            hidden: Vec::new(),
            any_hidden: false,
            starts: Vec::new(),
            starts_valid: false,
            max_width: 0,
            max_width_valid: false,
        };
        index.rebuild(buffer);
        index
    }

    pub fn wrap_cols(&self) -> Option<usize> {
        self.wrap_cols
    }

    fn measure(line: &[char], wrap_cols: Option<usize>) -> (u32, u32) {
        let width = line_cols(line.iter().copied()) as u32;
        let rows = match wrap_cols {
            Some(c) if width as usize > c => wrap_points(line, Some(c)).len() as u32,
            _ => 1,
        };
        (rows, width)
    }

    fn measure_line(buffer: &Buffer, line: usize, wrap_cols: Option<usize>) -> (u32, u32) {
        let chars: Vec<char> = buffer
            .rope()
            .line(line)
            .chars()
            .filter(|&c| c != '\n')
            .collect();
        Self::measure(&chars, wrap_cols)
    }

    pub fn rebuild(&mut self, buffer: &Buffer) {
        let n = buffer.len_lines();
        self.rows.clear();
        self.widths.clear();
        self.rows.reserve(n);
        self.widths.reserve(n);
        let mut line: Vec<char> = Vec::new();
        for c in buffer.rope().chars() {
            if c == '\n' {
                let (r, w) = Self::measure(&line, self.wrap_cols);
                self.rows.push(r);
                self.widths.push(w);
                line.clear();
            } else {
                line.push(c);
            }
        }
        let (r, w) = Self::measure(&line, self.wrap_cols);
        self.rows.push(r);
        self.widths.push(w);
        debug_assert_eq!(self.rows.len(), n);
        self.hidden.resize(n, false);
        self.starts_valid = false;
        self.max_width_valid = false;
    }

    pub fn set_wrap(&mut self, buffer: &Buffer, wrap_cols: Option<usize>) {
        if self.wrap_cols != wrap_cols {
            self.wrap_cols = wrap_cols;
            self.rebuild(buffer);
        }
    }

    /// Brings the index up to date after edits.
    pub fn apply_edits(&mut self, buffer: &Buffer, edits: &[LineEdit]) {
        if edits.is_empty() {
            return;
        }
        // Thousands of edits at once (Replace All): re-measuring everything is
        // cheaper than splicing the arrays once per edit.
        if edits.len() > 2000 {
            self.rebuild(buffer);
            return;
        }
        // Edits were applied one after another, so each splice sees the line
        // numbering left by the previous one. Spliced-in lines are marked 0 and
        // measured once, against the final text.
        for e in edits {
            let end = (e.start_line + e.old_lines).min(self.rows.len());
            let start = e.start_line.min(end);
            self.rows
                .splice(start..end, std::iter::repeat_n(0, e.new_lines));
            self.widths
                .splice(start..end, std::iter::repeat_n(0, e.new_lines));
            let hend = end.min(self.hidden.len());
            self.hidden.splice(
                start.min(hend)..hend,
                std::iter::repeat_n(false, e.new_lines),
            );
        }
        let n = buffer.len_lines();
        self.rows.resize(n, 0);
        self.widths.resize(n, 0);
        self.hidden.resize(n, false);
        for line in 0..n {
            if self.rows[line] == 0 {
                let (r, w) = Self::measure_line(buffer, line, self.wrap_cols);
                self.rows[line] = r;
                self.widths[line] = w;
            }
        }
        self.starts_valid = false;
        self.max_width_valid = false;
    }

    /// Hides the lines in `ranges` (folded regions) and shows all others.
    pub fn set_hidden(&mut self, ranges: impl Iterator<Item = std::ops::Range<usize>>) {
        let n = self.rows.len();
        self.hidden.clear();
        self.hidden.resize(n, false);
        self.any_hidden = false;
        for r in ranges {
            let (a, b) = (r.start.min(n), r.end.min(n));
            if a < b {
                self.hidden[a..b].fill(true);
                self.any_hidden = true;
            }
        }
        self.starts_valid = false;
    }

    pub fn is_hidden(&self, line: usize) -> bool {
        self.any_hidden && self.hidden.get(line).copied().unwrap_or(false)
    }

    fn ensure_starts(&mut self) {
        if self.starts_valid {
            return;
        }
        self.starts.clear();
        self.starts.reserve(self.rows.len() + 1);
        let mut acc = 0u64;
        for (i, &r) in self.rows.iter().enumerate() {
            self.starts.push(acc);
            if !(self.any_hidden && self.hidden[i]) {
                acc += r as u64;
            }
        }
        self.starts.push(acc);
        self.starts_valid = true;
    }

    pub fn total_rows(&mut self) -> usize {
        self.ensure_starts();
        *self.starts.last().unwrap_or(&0) as usize
    }

    /// First visual row of `line`.
    pub fn line_to_row(&mut self, line: usize) -> usize {
        self.ensure_starts();
        self.starts[line.min(self.rows.len())] as usize
    }

    /// The line containing visual `row`, and the row's index within that line.
    pub fn row_to_line(&mut self, row: usize) -> (usize, usize) {
        self.ensure_starts();
        let row = row as u64;
        let n = self.rows.len();
        // The last line starting at or before `row`; hidden lines take no rows,
        // so they share a start with the visible line after them.
        let mut line = self.starts[..n]
            .partition_point(|&s| s <= row)
            .saturating_sub(1)
            .min(n.saturating_sub(1));
        while line > 0 && self.is_hidden(line) {
            line -= 1;
        }
        (line, row.saturating_sub(self.starts[line]) as usize)
    }

    pub fn max_width(&mut self) -> usize {
        if !self.max_width_valid {
            self.max_width = self.widths.iter().copied().max().unwrap_or(0);
            self.max_width_valid = true;
        }
        self.max_width as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::buffer::{EditKind, Replacement};
    use crate::editor::selection::{Selection, Selections};

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn tabs_and_wide_chars() {
        assert_eq!(line_cols("a\tb".chars()), 5);
        assert_eq!(line_cols("日本".chars()), 4);
    }

    #[test]
    fn wraps_at_spaces_when_possible() {
        assert_eq!(
            wrap_points(&chars("hello world foo"), Some(8)),
            vec![0, 6, 12]
        );
        assert_eq!(wrap_points(&chars("abcdefghij"), Some(4)), vec![0, 4, 8]);
        assert_eq!(wrap_points(&chars("short"), Some(80)), vec![0]);
        assert_eq!(wrap_points(&chars("short"), None), vec![0]);
    }

    #[test]
    fn row_index_tracks_edits() {
        let mut buf = Buffer::new("aaaa aaaa\nb\nc");
        let mut index = RowIndex::new(&buf, Some(5));
        assert_eq!(index.total_rows(), 4);
        assert_eq!(index.row_to_line(1), (0, 1));
        assert_eq!(index.row_to_line(2), (1, 0));
        assert_eq!(index.line_to_row(2), 3);

        buf.take_line_edits();
        let sels = Selections::caret(11);
        let reps = vec![Replacement {
            range: 11..11,
            text: "\nnew\n".into(),
            caret: Selection::caret(5),
        }];
        buf.apply(&sels, reps, EditKind::Other, 0.0);
        let edits = buf.take_line_edits();
        index.apply_edits(&buf, &edits);
        assert_eq!(index.total_rows(), 6);
        assert_eq!(index.row_to_line(5), (4, 0));
        assert_eq!(index.max_width(), 9);
    }

    #[test]
    fn hidden_lines_take_no_rows() {
        let buf = Buffer::new("a\nb\nc\nd\ne");
        let mut index = RowIndex::new(&buf, None);
        index.set_hidden(std::iter::once(1..3));
        assert_eq!(index.total_rows(), 3);
        assert_eq!(index.row_to_line(0), (0, 0));
        assert_eq!(index.row_to_line(1), (3, 0));
        assert_eq!(index.line_to_row(3), 1);
        index.set_hidden(std::iter::once(3..5));
        assert_eq!(index.total_rows(), 3);
        assert_eq!(index.row_to_line(2), (2, 0));
    }
}
