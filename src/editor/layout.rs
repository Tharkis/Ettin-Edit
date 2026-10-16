//! Display columns, word wrap, and the index from visual rows to lines.
//!
//! The editor uses a monospace grid: every char has a width in columns
//! (tabs expand to the next tab stop, wide CJK characters take two).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use unicode_width::UnicodeWidthChar;

use super::buffer::{Buffer, LineEdit};

pub const DEFAULT_TAB_WIDTH: usize = 4;

/// Columns between tab stops, set from the preferences.
static TAB_WIDTH: AtomicUsize = AtomicUsize::new(DEFAULT_TAB_WIDTH);

pub fn tab_width() -> usize {
    TAB_WIDTH.load(Ordering::Relaxed)
}

pub fn set_tab_width(width: usize) {
    TAB_WIDTH.store(width.clamp(1, 16), Ordering::Relaxed);
}

/// Width of `c` in columns when it starts at column `col`.
#[inline]
pub fn char_cols(c: char, col: usize) -> usize {
    if (' '..='~').contains(&c) {
        return 1;
    }
    match c {
        '\t' => tab_width() - col % tab_width(),
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

/// How long one frame may spend re-wrapping long lines after the wrap width
/// changes. The rest carries over to the next frames.
pub const REWRAP_BUDGET: Duration = Duration::from_millis(4);

/// The row counts of every line for one wrap width.
struct WrapRows {
    wrap_cols: Option<usize>,
    rows: Vec<u32>,
    /// False for lines whose row count is only an estimate, because the wrap
    /// width changed and they have not been re-wrapped yet.
    exact: Vec<bool>,
    /// How many entries of `exact` are false.
    pending: usize,
    /// Where `refine` carries on scanning for estimated lines.
    cursor: usize,
    /// Bumped whenever the row counts change for any reason but an edit, so
    /// the view knows to keep its top line in place.
    generation: u64,
    /// Prefix sums of `rows` (skipping hidden lines), rebuilt lazily.
    starts: Vec<u64>,
    starts_valid: bool,
}

/// How many visual rows `line` takes; the same as `wrap_points(..).len()`.
pub fn wrap_rows(line: &[char], cols: usize) -> usize {
    if cols == 0 {
        return 1;
    }
    let mut rows = 1;
    let mut row_start = 0;
    let mut col = 0;
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
            rows += 1;
            row_start = at;
            last_break = None;
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
    rows
}

/// Per-line row counts and widths, so the view can find which line is at a
/// given scroll position without laying out the whole file.
///
/// Changing the wrap width doesn't re-read the text: a line's width is kept,
/// so lines that fit get one row straight away and lines that don't get an
/// estimate, re-wrapped exactly a few at a time (visible ones first). Rows for
/// two widths are kept, so a document shown in both views of a split at
/// different widths doesn't re-wrap every frame.
pub struct RowIndex {
    /// The tab width the widths were measured with.
    tab_width: usize,
    widths: Vec<u32>,
    /// Lines inside a folded region; they take no rows.
    hidden: Vec<bool>,
    any_hidden: bool,
    max_width: u32,
    max_width_valid: bool,
    /// Rows for the current wrap width.
    cur: WrapRows,
    /// Rows for the width used before, if any.
    alt: Option<WrapRows>,
}

impl WrapRows {
    fn new(wrap_cols: Option<usize>) -> Self {
        Self {
            wrap_cols,
            rows: Vec::new(),
            exact: Vec::new(),
            pending: 0,
            cursor: 0,
            generation: 0,
            starts: Vec::new(),
            starts_valid: false,
        }
    }

    /// Re-derives every row count from the line widths for a new wrap width,
    /// without touching the text.
    fn reset(&mut self, widths: &[u32], wrap_cols: Option<usize>) {
        self.wrap_cols = wrap_cols;
        self.rows.clear();
        self.exact.clear();
        self.pending = 0;
        self.cursor = 0;
        for &w in widths {
            match wrap_cols.filter(|&c| c > 0) {
                Some(c) if w as usize > c => {
                    // Breaking at spaces never takes fewer rows than this.
                    self.rows.push((w as usize).div_ceil(c) as u32);
                    self.exact.push(false);
                    self.pending += 1;
                }
                _ => {
                    self.rows.push(1);
                    self.exact.push(true);
                }
            }
        }
        self.changed();
    }

    fn changed(&mut self) {
        self.generation += 1;
        self.starts_valid = false;
    }

    fn set_exact(&mut self, line: usize, rows: u32) {
        if !self.exact[line] {
            self.exact[line] = true;
            self.pending -= 1;
        }
        if self.rows[line] != rows {
            self.rows[line] = rows;
            self.starts_valid = false;
        }
    }

    /// Splices in `new` unmeasured lines (row count 0) in place of `start..end`.
    fn splice(&mut self, start: usize, end: usize, new: usize) {
        self.pending -= self.exact[start..end].iter().filter(|&&e| !e).count();
        self.rows.splice(start..end, std::iter::repeat_n(0, new));
        self.exact
            .splice(start..end, std::iter::repeat_n(true, new));
        if start < self.cursor {
            self.cursor = start;
        }
        self.starts_valid = false;
    }

    fn ensure_starts(&mut self, hidden: &[bool], any_hidden: bool) {
        if self.starts_valid {
            return;
        }
        self.starts.clear();
        self.starts.reserve(self.rows.len() + 1);
        let mut acc = 0u64;
        for (i, &r) in self.rows.iter().enumerate() {
            self.starts.push(acc);
            if !(any_hidden && hidden[i]) {
                acc += r as u64;
            }
        }
        self.starts.push(acc);
        self.starts_valid = true;
    }
}

impl RowIndex {
    pub fn new(buffer: &Buffer, wrap_cols: Option<usize>) -> Self {
        let mut index = Self {
            tab_width: tab_width(),
            widths: Vec::new(),
            hidden: Vec::new(),
            any_hidden: false,
            max_width: 0,
            max_width_valid: false,
            cur: WrapRows::new(wrap_cols),
            alt: None,
        };
        index.rebuild(buffer);
        index
    }

    pub fn wrap_cols(&self) -> Option<usize> {
        self.cur.wrap_cols
    }

    /// Rows of a line `width` columns wide.
    fn line_rows(line: &[char], width: u32, wrap_cols: Option<usize>) -> u32 {
        match wrap_cols {
            Some(c) if width as usize > c => wrap_rows(line, c) as u32,
            _ => 1,
        }
    }

    fn read_line(buffer: &Buffer, line: usize, out: &mut Vec<char>) {
        out.clear();
        for chunk in buffer.rope().line(line).chunks() {
            out.extend(chunk.chars());
        }
        if out.last() == Some(&'\n') {
            out.pop();
        }
    }

    /// Re-measures every line from the text, exactly. Used on load, when the
    /// tab width changes, and after very large edits.
    pub fn rebuild(&mut self, buffer: &Buffer) {
        self.tab_width = tab_width();
        self.alt = None;
        let wrap_cols = self.cur.wrap_cols;
        let n = buffer.len_lines();
        let cur = &mut self.cur;
        self.widths.clear();
        cur.rows.clear();
        cur.exact.clear();
        self.widths.reserve(n);
        cur.rows.reserve(n);
        let mut line: Vec<char> = Vec::new();
        let mut push = |line: &[char]| {
            let w = line_cols(line.iter().copied()) as u32;
            self.widths.push(w);
            cur.rows.push(Self::line_rows(line, w, wrap_cols));
        };
        for c in buffer.rope().chars() {
            if c == '\n' {
                push(&line);
                line.clear();
            } else {
                line.push(c);
            }
        }
        push(&line);
        debug_assert_eq!(cur.rows.len(), n);
        cur.exact.resize(n, true);
        cur.pending = 0;
        cur.cursor = 0;
        cur.changed();
        self.hidden.resize(n, false);
        self.max_width_valid = false;
    }

    /// Switches to a new wrap width. Cheap: see the type's docs.
    pub fn set_wrap(&mut self, buffer: &Buffer, wrap_cols: Option<usize>) {
        if self.tab_width != tab_width() {
            self.cur.wrap_cols = wrap_cols;
            self.rebuild(buffer);
            return;
        }
        if self.cur.wrap_cols == wrap_cols {
            return;
        }
        if self.alt.as_ref().is_some_and(|a| a.wrap_cols == wrap_cols) {
            std::mem::swap(&mut self.cur, self.alt.as_mut().unwrap());
            return;
        }
        // Reuse the older set's memory for the new width.
        let mut rows = self.alt.take().unwrap_or_else(|| WrapRows::new(wrap_cols));
        rows.generation = self.cur.generation;
        rows.reset(&self.widths, wrap_cols);
        self.alt = Some(std::mem::replace(&mut self.cur, rows));
    }

    /// Whether some lines still have estimated row counts.
    pub fn is_settled(&self) -> bool {
        self.cur.pending == 0
    }

    /// Changes whenever row counts change other than through edits.
    pub fn generation(&self) -> u64 {
        self.cur.generation
    }

    /// Re-wraps estimated lines in `lines` exactly (the visible ones).
    pub fn ensure_exact(&mut self, buffer: &Buffer, lines: std::ops::Range<usize>) {
        if self.cur.pending == 0 {
            return;
        }
        let end = lines.end.min(self.cur.rows.len());
        let before = self.cur.pending;
        let mut chars = Vec::new();
        for line in lines.start.min(end)..end {
            if !self.cur.exact[line] {
                Self::read_line(buffer, line, &mut chars);
                let r = Self::line_rows(&chars, self.widths[line], self.cur.wrap_cols);
                self.cur.set_exact(line, r);
            }
        }
        if self.cur.pending != before {
            self.cur.changed();
        }
    }

    /// Re-wraps estimated lines from the top of the file for up to `budget`.
    /// Returns true while some remain.
    pub fn refine(&mut self, buffer: &Buffer, budget: Duration) -> bool {
        if self.cur.pending == 0 {
            return false;
        }
        let started = Instant::now();
        let before = self.cur.pending;
        let n = self.cur.rows.len();
        let mut chars = Vec::new();
        let mut done = 0usize;
        let mut lines = None;
        while self.cur.cursor < n && self.cur.pending > 0 {
            // Most lines usually fit; skip them without touching the text.
            let Some(skip) = self.cur.exact[self.cur.cursor..].iter().position(|&e| !e) else {
                self.cur.cursor = n;
                break;
            };
            let line = self.cur.cursor + skip;
            self.cur.cursor = line + 1;
            // Walking lines in order is cheaper than looking each one up.
            if lines.as_ref().is_none_or(|(next, _)| *next != line) {
                lines = Some((line, buffer.rope().lines_at(line)));
            }
            let (next, iter) = lines.as_mut().unwrap();
            *next += 1;
            chars.clear();
            if let Some(text) = iter.next() {
                for chunk in text.chunks() {
                    chars.extend(chunk.chars());
                }
            }
            if chars.last() == Some(&'\n') {
                chars.pop();
            }
            let r = Self::line_rows(&chars, self.widths[line], self.cur.wrap_cols);
            self.cur.set_exact(line, r);
            done += 1;
            if done.is_multiple_of(64) && started.elapsed() >= budget {
                break;
            }
        }
        if self.cur.pending != before {
            self.cur.changed();
        }
        self.cur.pending > 0
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
            let end = (e.start_line + e.old_lines).min(self.widths.len());
            let start = e.start_line.min(end);
            self.widths
                .splice(start..end, std::iter::repeat_n(0, e.new_lines));
            self.cur.splice(start, end, e.new_lines);
            if let Some(alt) = &mut self.alt {
                alt.splice(start, end, e.new_lines);
            }
            let hend = end.min(self.hidden.len());
            self.hidden.splice(
                start.min(hend)..hend,
                std::iter::repeat_n(false, e.new_lines),
            );
        }
        let n = buffer.len_lines();
        self.widths.resize(n, 0);
        self.hidden.resize(n, false);
        for set in std::iter::once(&mut self.cur).chain(self.alt.as_mut()) {
            set.rows.resize(n, 0);
            set.exact.resize(n, true);
        }
        let mut chars = Vec::new();
        for line in 0..n {
            if self.cur.rows[line] == 0 {
                Self::read_line(buffer, line, &mut chars);
                let w = line_cols(chars.iter().copied()) as u32;
                self.widths[line] = w;
                self.cur.rows[line] = Self::line_rows(&chars, w, self.cur.wrap_cols);
                if let Some(alt) = &mut self.alt {
                    alt.rows[line] = Self::line_rows(&chars, w, alt.wrap_cols);
                }
            }
        }
        self.max_width_valid = false;
    }

    /// Hides the lines in `ranges` (folded regions) and shows all others.
    pub fn set_hidden(&mut self, ranges: impl Iterator<Item = std::ops::Range<usize>>) {
        let n = self.widths.len();
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
        self.cur.starts_valid = false;
        if let Some(alt) = &mut self.alt {
            alt.starts_valid = false;
        }
    }

    pub fn is_hidden(&self, line: usize) -> bool {
        self.any_hidden && self.hidden.get(line).copied().unwrap_or(false)
    }

    fn starts(&mut self) -> &[u64] {
        self.cur.ensure_starts(&self.hidden, self.any_hidden);
        &self.cur.starts
    }

    pub fn total_rows(&mut self) -> usize {
        *self.starts().last().unwrap_or(&0) as usize
    }

    /// First visual row of `line`.
    pub fn line_to_row(&mut self, line: usize) -> usize {
        let n = self.widths.len();
        self.starts()[line.min(n)] as usize
    }

    /// The line containing visual `row`, and the row's index within that line.
    pub fn row_to_line(&mut self, row: usize) -> (usize, usize) {
        let row = row as u64;
        let n = self.widths.len();
        let starts = self.starts();
        // The last line starting at or before `row`; hidden lines take no rows,
        // so they share a start with the visible line after them.
        let mut line = starts[..n]
            .partition_point(|&s| s <= row)
            .saturating_sub(1)
            .min(n.saturating_sub(1));
        while line > 0 && self.is_hidden(line) {
            line -= 1;
        }
        let start = self.starts()[line];
        (line, row.saturating_sub(start) as usize)
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
        for (s, cols) in [
            ("hello world foo", 8),
            ("abcdefghij", 4),
            ("a\tb c\td 日本語 e", 5),
        ] {
            assert_eq!(
                wrap_rows(&chars(s), cols),
                wrap_points(&chars(s), Some(cols)).len()
            );
        }
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

    /// Row counts after `set_wrap` + `refine`, line by line.
    fn rows_per_line(index: &mut RowIndex) -> Vec<usize> {
        let n = index.widths.len();
        (0..n)
            .map(|l| index.line_to_row(l + 1) - index.line_to_row(l))
            .collect()
    }

    fn sample_text(lines: usize) -> String {
        (0..lines)
            .map(|i| match i % 4 {
                0 => "short".to_string(),
                1 => "word ".repeat(30),
                2 => format!("\t{}", "x".repeat(i % 97 + 20)),
                _ => "日本語のテキスト ".repeat(6),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn rewrap_settles_to_a_full_rebuild() {
        let buf = Buffer::new(&sample_text(400));
        let mut index = RowIndex::new(&buf, Some(120));
        for cols in [40, 17, 80, 40] {
            index.set_wrap(&buf, Some(cols));
            while index.refine(&buf, Duration::ZERO) {}
            assert!(index.is_settled());
            let mut fresh = RowIndex::new(&buf, Some(cols));
            assert_eq!(
                rows_per_line(&mut index),
                rows_per_line(&mut fresh),
                "{cols}"
            );
        }
    }

    #[test]
    fn rewrap_estimates_until_refined() {
        let buf = Buffer::new(&sample_text(40));
        let mut index = RowIndex::new(&buf, None);
        let before = index.generation();
        index.set_wrap(&buf, Some(20));
        assert!(!index.is_settled());
        assert_ne!(index.generation(), before);
        // Visible lines can be made exact on their own.
        index.ensure_exact(&buf, 0..8);
        let mut fresh = RowIndex::new(&buf, Some(20));
        assert_eq!(
            rows_per_line(&mut index)[..8],
            rows_per_line(&mut fresh)[..8]
        );
        assert!(!index.is_settled());
        while index.refine(&buf, Duration::ZERO) {}
        assert_eq!(rows_per_line(&mut index), rows_per_line(&mut fresh));
    }

    #[test]
    fn edits_while_rewrapping_and_two_widths() {
        let mut buf = Buffer::new(&sample_text(60));
        let mut index = RowIndex::new(&buf, Some(30));
        index.set_wrap(&buf, Some(15));
        index.refine(&buf, Duration::ZERO);
        buf.take_line_edits();
        let at = buf.line_start(10);
        let sels = Selections::caret(at);
        let reps = vec![Replacement {
            range: at..buf.line_start(14),
            text: "a b c d e f g h i j k l m n o p\nnew line here\n".into(),
            caret: Selection::caret(at),
        }];
        buf.apply(&sels, reps, EditKind::Other, 0.0);
        let edits = buf.take_line_edits();
        index.apply_edits(&buf, &edits);
        while index.refine(&buf, Duration::ZERO) {}
        let mut fresh = RowIndex::new(&buf, Some(15));
        assert_eq!(rows_per_line(&mut index), rows_per_line(&mut fresh));
        // Switching back to the earlier width reuses its rows, edits included.
        let generation = index.generation();
        index.set_wrap(&buf, Some(30));
        assert_ne!(index.generation(), generation);
        while index.refine(&buf, Duration::ZERO) {}
        let mut fresh = RowIndex::new(&buf, Some(30));
        assert_eq!(rows_per_line(&mut index), rows_per_line(&mut fresh));
        index.set_wrap(&buf, Some(15));
        assert!(index.is_settled());
        let mut fresh = RowIndex::new(&buf, Some(15));
        assert_eq!(rows_per_line(&mut index), rows_per_line(&mut fresh));
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
