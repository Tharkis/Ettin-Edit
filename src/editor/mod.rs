//! Ettin Edit's text editor engine: a rope buffer, multiple selections, editing
//! commands, and a view that only lays out the rows on screen.

pub mod buffer;
mod find;
mod fold;
pub mod highlight;
pub mod layout;
pub mod lines;
pub mod selection;
mod smart;
mod view;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::Arc;

use unicode_segmentation::UnicodeSegmentation;

use buffer::{Buffer, EditKind, Replacement};
use highlight::LineHighlighter;
use layout::RowIndex;
use selection::{Selection, Selections};

pub use find::{FindOutcome, MAX_HIGHLIGHTS};
pub use smart::EditOptions;
pub use view::ViewConfig;

pub struct Editor {
    pub buffer: Buffer,
    pub selections: Selections,
    rows: RowIndex,
    highlighter: LineHighlighter,
    /// Scroll the primary caret into view on the next frame.
    scroll_to_caret: bool,
    /// Where a mouse drag started, as a selection to extend from.
    drag_origin: Option<Selection>,
    drag_mode: DragMode,
    /// IME composition text shown at the caret while typing in an input method.
    preedit: Option<String>,
    /// Rows that fit on screen, for Page Up / Page Down.
    page_rows: usize,
    /// Rows to scroll the view by on the next frame (Page Up / Page Down move the view with the caret).
    pending_scroll_rows: isize,
    /// Time, position and count of the last click, to detect double and triple clicks.
    last_click: Option<(f64, egui::Pos2, u8)>,
    /// The text as one string for searching, with the buffer version it was taken at.
    text_cache: Option<(u64, Arc<String>)>,
    /// Matches of the find bar's search, highlighted while the bar is open (char ranges, sorted).
    pub search_hits: Arc<Vec<Range<usize>>>,
    /// Text marked with Mark All (char ranges, sorted). Marks move with edits.
    pub marks: Vec<Range<usize>>,
    /// Bookmarked lines (0-based). Bookmarks move with edits.
    pub bookmarks: BTreeSet<usize>,
    /// Typing behaviour (auto-close, indentation, completion), set by the app.
    pub opts: EditOptions,
    /// The word-completion list, while it is open.
    pub completion: Option<smart::Completion>,
    /// The rectangular selection being made, and the selections it produced.
    column: Option<(smart::ColumnSel, Selections)>,
    /// Where an Alt+drag rectangular selection started (visual row, column).
    column_drag: Option<(usize, usize)>,
    /// Folded regions: first line → last hidden line.
    folds: BTreeMap<usize, usize>,
    /// The carets and scrolling of the other view, when the document is shown
    /// in both views of a split. Swapped in with [`Editor::swap_view`].
    other_view: Option<Box<ViewState>>,
}

/// What each view of a document keeps for itself: carets, scrolling and
/// mouse state. The text, undo history, folds and marks are shared.
struct ViewState {
    selections: Selections,
    scroll_to_caret: bool,
    drag_origin: Option<Selection>,
    drag_mode: Option<DragMode>,
    preedit: Option<String>,
    page_rows: usize,
    pending_scroll_rows: isize,
    last_click: Option<(f64, egui::Pos2, u8)>,
    completion: Option<smart::Completion>,
    column: Option<(smart::ColumnSel, Selections)>,
    column_drag: Option<(usize, usize)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragMode {
    Char,
    Word,
    Line,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Space,
    Word,
    Punct,
}

fn class(c: char) -> CharClass {
    if c.is_whitespace() {
        CharClass::Space
    } else if c.is_alphanumeric() || c == '_' {
        CharClass::Word
    } else {
        CharClass::Punct
    }
}

impl Editor {
    pub fn new(text: &str) -> Self {
        let buffer = Buffer::new(text);
        let rows = RowIndex::new(&buffer, None);
        Self {
            buffer,
            selections: Selections::caret(0),
            rows,
            highlighter: LineHighlighter::default(),
            scroll_to_caret: true,
            drag_origin: None,
            drag_mode: DragMode::Char,
            preedit: None,
            page_rows: 20,
            pending_scroll_rows: 0,
            last_click: None,
            text_cache: None,
            search_hits: Arc::default(),
            marks: Vec::new(),
            bookmarks: BTreeSet::new(),
            opts: EditOptions::default(),
            completion: None,
            column: None,
            column_drag: None,
            folds: BTreeMap::new(),
            other_view: None,
        }
    }

    /// Gives the document a second view (for showing it in both views of a
    /// split), starting where this one is.
    pub fn add_other_view(&mut self) {
        self.other_view = Some(Box::new(ViewState {
            selections: self.selections.clone(),
            scroll_to_caret: true,
            drag_origin: None,
            drag_mode: Some(DragMode::Char),
            preedit: None,
            page_rows: self.page_rows,
            pending_scroll_rows: 0,
            last_click: None,
            completion: None,
            column: None,
            column_drag: None,
        }));
    }

    /// Drops the second view.
    pub fn remove_other_view(&mut self) {
        self.other_view = None;
    }

    pub fn has_other_view(&self) -> bool {
        self.other_view.is_some()
    }

    /// The selections of the other view, when the document is shown in both.
    pub fn other_view_selections(&self) -> Option<&Selections> {
        self.other_view.as_deref().map(|v| &v.selections)
    }

    /// Swaps the live view state with the other view's, so editing and drawing
    /// act on the other view. Does nothing without a second view.
    pub fn swap_view(&mut self) {
        let Some(other) = self.other_view.as_deref_mut() else {
            return;
        };
        std::mem::swap(&mut self.selections, &mut other.selections);
        std::mem::swap(&mut self.scroll_to_caret, &mut other.scroll_to_caret);
        std::mem::swap(&mut self.drag_origin, &mut other.drag_origin);
        let mode = other.drag_mode.replace(self.drag_mode);
        self.drag_mode = mode.unwrap_or(DragMode::Char);
        std::mem::swap(&mut self.preedit, &mut other.preedit);
        std::mem::swap(&mut self.page_rows, &mut other.page_rows);
        std::mem::swap(
            &mut self.pending_scroll_rows,
            &mut other.pending_scroll_rows,
        );
        std::mem::swap(&mut self.last_click, &mut other.last_click);
        std::mem::swap(&mut self.completion, &mut other.completion);
        std::mem::swap(&mut self.column, &mut other.column);
        std::mem::swap(&mut self.column_drag, &mut other.column_drag);
        self.selections.clamp(self.buffer.len_chars());
    }

    /// Line and column (both 1-based) of the primary caret. The column counts chars.
    pub fn caret_line_col(&self) -> (usize, usize) {
        let head = self.selections.primary().head;
        let line = self.buffer.char_to_line(head);
        (line + 1, head - self.buffer.line_start(line) + 1)
    }

    /// Total number of selected chars across all selections.
    pub fn selected_chars(&self) -> usize {
        self.selections.iter().map(|s| s.end() - s.start()).sum()
    }

    pub fn caret_count(&self) -> usize {
        self.selections.len()
    }

    fn now(ctx: &egui::Context) -> f64 {
        ctx.input(|i| i.time)
    }

    fn set_selections(&mut self, sels: Selections) {
        self.selections = sels;
        self.selections.clamp(self.buffer.len_chars());
        self.scroll_to_caret = true;
    }

    // ------------------------------------------------------------------
    // Positions

    fn line_end(&self, line: usize) -> usize {
        self.buffer.line_start(line) + self.buffer.line_len(line)
    }

    fn prev_grapheme(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let line = self.buffer.char_to_line(pos);
        let start = self.buffer.line_start(line);
        if pos == start {
            return pos - 1;
        }
        let prefix = self.buffer.slice_string(start..pos);
        let last = prefix
            .graphemes(true)
            .next_back()
            .map_or(1, |g| g.chars().count());
        pos - last
    }

    fn next_grapheme(&self, pos: usize) -> usize {
        let len = self.buffer.len_chars();
        if pos >= len {
            return len;
        }
        let line = self.buffer.char_to_line(pos);
        let end = self.line_end(line);
        if pos >= end {
            return (pos + 1).min(len);
        }
        let rest = self.buffer.slice_string(pos..end);
        let first = rest.graphemes(true).next().map_or(1, |g| g.chars().count());
        pos + first
    }

    fn prev_word(&self, pos: usize) -> usize {
        let rope = self.buffer.rope();
        let mut p = pos;
        while p > 0 && class(rope.char(p - 1)) == CharClass::Space {
            p -= 1;
        }
        if p > 0 {
            let c = class(rope.char(p - 1));
            while p > 0 && class(rope.char(p - 1)) == c {
                p -= 1;
            }
        }
        p
    }

    fn next_word(&self, pos: usize) -> usize {
        let rope = self.buffer.rope();
        let len = rope.len_chars();
        let mut p = pos;
        if p < len {
            let c = class(rope.char(p));
            if c != CharClass::Space {
                while p < len && class(rope.char(p)) == c {
                    p += 1;
                }
            }
        }
        while p < len && rope.char(p) != '\n' && class(rope.char(p)) == CharClass::Space {
            p += 1;
        }
        if p == pos && p < len {
            p += 1;
        }
        p
    }

    /// The word (or run of punctuation or spaces) around `pos`.
    fn word_range(&self, pos: usize) -> (usize, usize) {
        let rope = self.buffer.rope();
        let len = rope.len_chars();
        let line = self.buffer.char_to_line(pos);
        let (ls, le) = (self.buffer.line_start(line), self.line_end(line));
        let probe = if pos < le {
            pos
        } else if pos > ls {
            pos - 1
        } else {
            return (pos, pos);
        };
        let c = class(rope.char(probe));
        let mut a = probe;
        while a > ls && class(rope.char(a - 1)) == c {
            a -= 1;
        }
        let mut b = probe;
        while b < le.min(len) && class(rope.char(b)) == c {
            b += 1;
        }
        (a, b)
    }

    /// Start of the line's text after leading whitespace.
    fn first_non_blank(&self, line: usize) -> usize {
        let start = self.buffer.line_start(line);
        let end = self.line_end(line);
        let rope = self.buffer.rope();
        let mut p = start;
        while p < end && matches!(rope.char(p), ' ' | '\t') {
            p += 1;
        }
        p
    }

    // ------------------------------------------------------------------
    // Movement

    pub fn move_horizontal(&mut self, forward: bool, by_word: bool, extend: bool) {
        let mut sels = self.selections.clone();
        sels.map(|s| {
            if !extend && !s.is_empty() {
                let p = if forward { s.end() } else { s.start() };
                return Selection::caret(p);
            }
            let p = match (forward, by_word) {
                (true, false) => self.next_grapheme(s.head),
                (false, false) => self.prev_grapheme(s.head),
                (true, true) => self.next_word(s.head),
                (false, true) => self.prev_word(s.head),
            };
            s.moved_to(self.skip_hidden(p, forward), extend)
        });
        self.set_selections(sels);
    }

    pub fn move_home(&mut self, extend: bool, whole_doc: bool) {
        let mut sels = self.selections.clone();
        sels.map(|s| {
            if whole_doc {
                return s.moved_to(0, extend);
            }
            let line = self.buffer.char_to_line(s.head);
            let smart = self.first_non_blank(line);
            let target = if s.head == smart {
                self.buffer.line_start(line)
            } else {
                smart
            };
            s.moved_to(target, extend)
        });
        self.set_selections(sels);
    }

    pub fn move_end(&mut self, extend: bool, whole_doc: bool) {
        let len = self.buffer.len_chars();
        let mut sels = self.selections.clone();
        sels.map(|s| {
            let target = if whole_doc {
                len
            } else {
                self.line_end(self.buffer.char_to_line(s.head))
            };
            s.moved_to(target, extend)
        });
        self.set_selections(sels);
    }

    pub fn select_all(&mut self) {
        self.set_selections(Selections::single(Selection::new(
            0,
            self.buffer.len_chars(),
        )));
    }

    // ------------------------------------------------------------------
    // Editing

    fn edit(
        &mut self,
        ctx: &egui::Context,
        kind: EditKind,
        f: impl Fn(&Self, Selection) -> Replacement,
    ) {
        let reps: Vec<Replacement> = self.selections.iter().map(|&s| f(self, s)).collect();
        let before = self.selections.clone();
        let after = self.buffer.apply(&before, reps, kind, Self::now(ctx));
        self.set_selections(after);
    }

    pub fn insert_text(&mut self, ctx: &egui::Context, text: &str) {
        let n = text.chars().count();
        let kind = if n == 1 && !text.starts_with(char::is_whitespace) {
            EditKind::Insert
        } else {
            EditKind::Other
        };
        let kind = if text == " " { EditKind::Insert } else { kind };
        self.edit(ctx, kind, |_, s| Replacement {
            range: s.range(),
            text: text.to_owned(),
            caret: Selection::caret(n),
        });
    }

    /// Text typed at the keyboard: like [`Self::insert_text`], plus auto-closed
    /// pairs and closing brackets that line up with their opener.
    pub fn type_text(&mut self, ctx: &egui::Context, text: &str) {
        let mut chars = text.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            if c == '>' && self.opts.markup && self.opts.close_tags && self.type_tag_end(ctx) {
                return;
            }
            if self.opts.auto_close && self.type_with_pairs(ctx, c) {
                return;
            }
            if self.opts.smart_indent
                && matches!(c, '}' | ']' | ')')
                && self.type_closer_outdented(ctx, c)
            {
                return;
            }
        }
        self.insert_text(ctx, text);
    }

    /// Pastes `text`; one line per caret when the line count matches the caret count.
    pub fn paste(&mut self, ctx: &egui::Context, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = text.split('\n').collect();
        let per_caret = self.selections.len() > 1 && lines.len() == self.selections.len();
        let reps: Vec<Replacement> = self
            .selections
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let t = if per_caret {
                    lines[i].to_owned()
                } else {
                    text.clone()
                };
                let n = t.chars().count();
                Replacement {
                    range: s.range(),
                    text: t,
                    caret: Selection::caret(n),
                }
            })
            .collect();
        let before = self.selections.clone();
        let after = self
            .buffer
            .apply(&before, reps, EditKind::Other, Self::now(ctx));
        self.set_selections(after);
    }

    pub fn newline(&mut self, ctx: &egui::Context) {
        if self.opts.smart_indent {
            self.smart_newline(ctx);
            return;
        }
        self.edit(ctx, EditKind::Other, |ed, s| {
            // Carry the current line's indentation onto the new line.
            let line = ed.buffer.char_to_line(s.start());
            let start = ed.buffer.line_start(line);
            let indent_end = ed.first_non_blank(line).min(s.start());
            let indent = ed.buffer.slice_string(start..indent_end);
            let text = format!("\n{indent}");
            let n = text.chars().count();
            Replacement {
                range: s.range(),
                text,
                caret: Selection::caret(n),
            }
        });
    }

    pub fn delete(&mut self, ctx: &egui::Context, forward: bool, by_word: bool) {
        if !forward && !by_word && self.opts.auto_close && self.delete_pair(ctx) {
            return;
        }
        self.edit(ctx, EditKind::Delete, |ed, s| {
            let range = if !s.is_empty() {
                s.range()
            } else if forward {
                let to = if by_word {
                    ed.next_word(s.head)
                } else {
                    ed.next_grapheme(s.head)
                };
                s.head..to
            } else {
                let from = if by_word {
                    ed.prev_word(s.head)
                } else {
                    ed.prev_grapheme(s.head)
                };
                from..s.head
            };
            Replacement {
                range,
                text: String::new(),
                caret: Selection::caret(0),
            }
        });
    }

    /// Tab: inserts a tab, or indents every line a multi-line selection touches.
    pub fn indent(&mut self, ctx: &egui::Context, outdent: bool) {
        let multi_line = self
            .selections
            .iter()
            .any(|s| self.buffer.char_to_line(s.start()) != self.buffer.char_to_line(s.end()));
        if !outdent && !multi_line {
            if !self.opts.use_spaces {
                self.insert_text(ctx, "\t");
                return;
            }
            // Spaces up to the next tab stop.
            self.edit(ctx, EditKind::Insert, |ed, s| {
                let line = ed.buffer.char_to_line(s.start());
                let ls = ed.buffer.line_start(line);
                let col = layout::line_cols(ed.buffer.rope().slice(ls..s.start()).chars());
                let n = layout::tab_width() - col % layout::tab_width();
                Replacement {
                    range: s.range(),
                    text: " ".repeat(n),
                    caret: Selection::caret(n),
                }
            });
            return;
        }
        // Collect each touched line once.
        let mut lines: Vec<usize> = Vec::new();
        for s in self.selections.iter() {
            let first = self.buffer.char_to_line(s.start());
            let mut last = self.buffer.char_to_line(s.end());
            if last > first && s.end() == self.buffer.line_start(last) {
                last -= 1;
            }
            for l in first..=last {
                if lines.last() != Some(&l) {
                    lines.push(l);
                }
            }
        }
        lines.dedup();
        let rope = self.buffer.rope();
        let mut reps = Vec::new();
        for &l in &lines {
            let start = self.buffer.line_start(l);
            if outdent {
                let mut n = 0;
                let end = self.line_end(l);
                while start + n < end && n < layout::tab_width() && rope.char(start + n) == ' ' {
                    n += 1;
                }
                if n == 0 && start < end && rope.char(start) == '\t' {
                    n = 1;
                }
                if n > 0 {
                    reps.push(Replacement {
                        range: start..start + n,
                        text: String::new(),
                        caret: Selection::caret(0),
                    });
                }
            } else if self.buffer.line_len(l) > 0 {
                let unit = self.indent_unit();
                let n = unit.chars().count();
                reps.push(Replacement {
                    range: start..start,
                    text: unit,
                    caret: Selection::caret(n),
                });
            }
        }
        if reps.is_empty() {
            return;
        }
        // Shift each selection end by what was inserted or removed before it.
        let shifts: Vec<(usize, isize)> = reps
            .iter()
            .map(|r| {
                (
                    r.range.start,
                    r.text.chars().count() as isize - r.range.len() as isize,
                )
            })
            .collect();
        let shift = |p: usize, caret: bool| -> usize {
            let mut q = p as isize;
            for &(at, d) in &shifts {
                if at < p || (at == p && d > 0 && caret) {
                    q += if d < 0 {
                        -((p - at).min((-d) as usize) as isize)
                    } else {
                        d
                    };
                }
            }
            q.max(0) as usize
        };
        let before = self.selections.clone();
        let new_sels: Vec<Selection> = before
            .iter()
            .map(|s| Selection::new(shift(s.anchor, s.is_empty()), shift(s.head, s.is_empty())))
            .collect();
        let time = Self::now(ctx);
        self.buffer.apply(&before, reps, EditKind::Other, time);
        self.set_selections(Selections::from_vec(new_sels, before.primary_index()));
    }

    pub fn undo(&mut self) {
        if let Some(sels) = self.buffer.undo() {
            self.set_selections(sels);
        }
    }

    pub fn redo(&mut self) {
        if let Some(sels) = self.buffer.redo() {
            self.set_selections(sels);
        }
    }

    /// Text of all selections, one per line.
    pub fn selected_text(&self) -> Option<String> {
        if !self.selections.has_selection() {
            return None;
        }
        let parts: Vec<String> = self
            .selections
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| self.buffer.slice_string(s.range()))
            .collect();
        Some(parts.join("\n"))
    }

    pub fn cut(&mut self, ctx: &egui::Context) -> Option<String> {
        let text = self.selected_text()?;
        self.edit(ctx, EditKind::Other, |_, s| Replacement {
            range: s.range(),
            text: String::new(),
            caret: Selection::caret(0),
        });
        Some(text)
    }

    pub fn collapse_selections(&mut self) {
        let mut sels = self.selections.clone();
        if sels.len() > 1 {
            sels.collapse_to_primary();
        } else {
            sels.map(|s| Selection::caret(s.head));
        }
        self.set_selections(sels);
    }

    /// Replaces the whole text as one undoable step (reloading a file that
    /// changed on disk), keeping the carets where they still fit.
    pub fn reload_text(&mut self, text: &str) {
        if *self.buffer.rope() == text {
            return;
        }
        let read_only = std::mem::replace(&mut self.buffer.read_only, false);
        let before = self.selections.clone();
        let mut after = before.clone();
        after.clamp(text.chars().count());
        let replacement = Replacement {
            range: 0..self.buffer.len_chars(),
            text: text.to_owned(),
            caret: Selection::caret(0),
        };
        self.buffer
            .apply_explicit(&before, vec![replacement], after.clone(), 0.0);
        self.buffer.read_only = read_only;
        self.set_selections(after);
        self.scroll_to_caret = false;
    }

    // ------------------------------------------------------------------
    // Edits from plugins

    /// Replaces char ranges as one undo step; ranges must not overlap.
    /// Carets move with the text. False if the text is read-only.
    pub fn edit_ranges(
        &mut self,
        ctx: &egui::Context,
        mut edits: Vec<(Range<usize>, String)>,
    ) -> bool {
        if self.buffer.read_only {
            return false;
        }
        let len = self.buffer.len_chars();
        for (r, _) in &mut edits {
            r.end = r.end.min(len);
            r.start = r.start.min(r.end);
        }
        edits.sort_by_key(|(r, _)| (r.start, r.end));
        let sizes: Vec<(Range<usize>, usize)> = edits
            .iter()
            .map(|(r, t)| (r.clone(), t.chars().count()))
            .collect();
        let before = self.selections.clone();
        let mut after = before.clone();
        after.map(|s| Selection {
            anchor: map_through(s.anchor, &sizes),
            head: map_through(s.head, &sizes),
            goal_col: None,
        });
        let reps = edits
            .into_iter()
            .map(|(range, text)| Replacement {
                range,
                text,
                caret: Selection::caret(0),
            })
            .collect();
        let after = self
            .buffer
            .apply_explicit(&before, reps, after, Self::now(ctx));
        self.set_selections(after);
        true
    }

    /// Replaces the whole text as one undo step, keeping the carets where
    /// they still fit. False if the text is read-only.
    pub fn set_text_undoable(&mut self, ctx: &egui::Context, text: &str) -> bool {
        if self.buffer.read_only {
            return false;
        }
        if *self.buffer.rope() == text {
            return true;
        }
        let before = self.selections.clone();
        let mut after = before.clone();
        after.clamp(text.chars().count());
        let replacement = Replacement {
            range: 0..self.buffer.len_chars(),
            text: text.to_owned(),
            caret: Selection::caret(0),
        };
        let after = self
            .buffer
            .apply_explicit(&before, vec![replacement], after, Self::now(ctx));
        self.set_selections(after);
        true
    }

    /// Replaces each selection, in document order, with a text and selects
    /// it; a single text replaces every selection. One undo step.
    pub fn replace_selections(
        &mut self,
        ctx: &egui::Context,
        texts: &[String],
    ) -> Result<(), String> {
        if self.buffer.read_only {
            return Err("The document is read-only".to_owned());
        }
        let n = self.selections.len();
        if texts.len() != 1 && texts.len() != n {
            return Err(format!("Got {} texts for {n} selections", texts.len()));
        }
        let before = self.selections.clone();
        let reps = before
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let text = texts[if texts.len() == 1 { 0 } else { i }].clone();
                let len = text.chars().count();
                Replacement {
                    range: s.range(),
                    text,
                    caret: Selection::new(0, len),
                }
            })
            .collect();
        let after = self
            .buffer
            .apply(&before, reps, EditKind::Other, Self::now(ctx));
        self.set_selections(after);
        Ok(())
    }

    /// Sets the selections from (anchor, head) pairs; the first is the main
    /// one. Scrolls to it.
    pub fn select_ranges(&mut self, ranges: &[(usize, usize)]) {
        let len = self.buffer.len_chars();
        let sels = ranges
            .iter()
            .map(|&(a, h)| Selection::new(a.min(len), h.min(len)))
            .collect();
        self.set_selections(Selections::from_vec(sels, 0));
        self.reveal_carets();
    }

    /// Replaces the whole text (e.g. reload); clears history.
    #[allow(dead_code)]
    pub fn replace_all_text(&mut self, text: &str) {
        let wrap = self.rows.wrap_cols();
        self.buffer = Buffer::new(text);
        self.rows = RowIndex::new(&self.buffer, wrap);
        self.highlighter = LineHighlighter::default();
        self.selections = Selections::caret(0);
        self.marks.clear();
        self.bookmarks.clear();
        self.folds.clear();
    }
}

/// Where a position ends up after replacing `edits` (sorted ranges, each with
/// the length of its new text). Inside a replaced range it moves to the end.
fn map_through(pos: usize, edits: &[(Range<usize>, usize)]) -> usize {
    let mut delta: isize = 0;
    for (r, n) in edits {
        if pos < r.start || (pos == r.start && r.start < r.end) {
            break;
        }
        if pos >= r.end {
            delta += *n as isize - r.len() as isize;
            continue;
        }
        return (r.start as isize + delta) as usize + n;
    }
    (pos as isize + delta) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(text: &str, caret: usize) -> Editor {
        let mut e = Editor::new(text);
        e.selections = Selections::caret(caret);
        e
    }

    #[test]
    fn word_movement() {
        let e = ed("foo.bar  baz", 0);
        assert_eq!(e.next_word(0), 3);
        assert_eq!(e.next_word(3), 4);
        assert_eq!(e.next_word(4), 9);
        assert_eq!(e.prev_word(12), 9);
        assert_eq!(e.prev_word(9), 4);
    }

    #[test]
    fn graphemes_move_as_one() {
        let e = ed("e\u{301}x", 0);
        assert_eq!(e.next_grapheme(0), 2);
        assert_eq!(e.prev_grapheme(2), 0);
    }

    #[test]
    fn newline_keeps_indent_and_backspace_deletes() {
        let ctx = egui::Context::default();
        let mut e = ed("    let x", 9);
        e.newline(&ctx);
        assert_eq!(e.buffer.text(), "    let x\n    ");
        e.delete(&ctx, false, false);
        assert_eq!(e.buffer.text(), "    let x\n   ");
    }

    #[test]
    fn indent_and_outdent_lines() {
        let ctx = egui::Context::default();
        let mut e = ed("a\nb\nc", 0);
        e.selections = Selections::single(Selection::new(0, 3));
        e.indent(&ctx, false);
        assert_eq!(e.buffer.text(), "\ta\n\tb\nc");
        assert_eq!(e.selections.primary().range(), 0..5);
        e.indent(&ctx, true);
        assert_eq!(e.buffer.text(), "a\nb\nc");
    }

    #[test]
    fn multi_caret_paste_distributes_lines() {
        let ctx = egui::Context::default();
        let mut e = ed("x\ny", 0);
        e.selections = Selections::from_vec(vec![Selection::caret(1), Selection::caret(3)], 0);
        e.paste(&ctx, "1\n2");
        assert_eq!(e.buffer.text(), "x1\ny2");
    }

    #[test]
    fn plugin_edits_move_carets() {
        let ctx = egui::Context::default();
        let mut e = ed("hello world", 8);
        assert!(e.edit_ranges(&ctx, vec![(0..5, "hi".into()), (11..11, "!".into())]));
        assert_eq!(e.buffer.text(), "hi world!");
        assert_eq!(e.selections.primary().head, 5);
        e.undo();
        assert_eq!(e.buffer.text(), "hello world");

        e.selections = Selections::from_vec(vec![Selection::new(0, 5), Selection::new(6, 11)], 0);
        e.replace_selections(&ctx, &["A".into(), "B".into()])
            .unwrap();
        assert_eq!(e.buffer.text(), "A B");
        assert!(
            e.replace_selections(&ctx, &["x".into(), "y".into(), "z".into()])
                .is_err()
        );
        e.replace_selections(&ctx, &["-".into()]).unwrap();
        assert_eq!(e.buffer.text(), "- -");

        assert!(e.set_text_undoable(&ctx, "new"));
        assert_eq!(e.buffer.text(), "new");
        e.select_ranges(&[(3, 1)]);
        assert_eq!(e.selections.primary().range(), 1..3);
        e.buffer.read_only = true;
        assert!(!e.set_text_undoable(&ctx, "x"));
        assert!(!e.edit_ranges(&ctx, vec![(0..1, "x".into())]));
    }

    #[test]
    fn map_through_edits() {
        let edits = [(2..4, 1), (6..6, 3)];
        assert_eq!(map_through(1, &edits), 1);
        assert_eq!(map_through(2, &edits), 2);
        assert_eq!(map_through(3, &edits), 3);
        assert_eq!(map_through(5, &edits), 4);
        assert_eq!(map_through(6, &edits), 8);
    }

    #[test]
    fn smart_home() {
        let mut e = ed("   abc", 5);
        e.move_home(false, false);
        assert_eq!(e.selections.primary().head, 3);
        e.move_home(false, false);
        assert_eq!(e.selections.primary().head, 0);
    }
}
