//! The text of a document as a rope, plus its undo/redo history.
//!
//! Every change goes through [`Buffer::apply`], which records it for undo and
//! reports which lines changed so the view can update its caches.

use std::ops::Range;

use ropey::Rope;

use super::selection::{Selection, Selections};

/// One replacement, in char offsets, as applied to the rope at that moment.
#[derive(Clone, Debug)]
struct Change {
    start: usize,
    removed: String,
    inserted: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// Typing; consecutive typing merges into one undo step.
    Insert,
    /// Backspace/delete; consecutive deletes merge into one undo step.
    Delete,
    /// Anything else (paste, indent, line operations): always its own step.
    Other,
}

struct Transaction {
    id: u64,
    changes: Vec<Change>,
    before: Selections,
    after: Selections,
    kind: EditKind,
    time: f64,
}

/// Lines touched by an edit: `old_lines` lines starting at `start_line` were
/// replaced by `new_lines` lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEdit {
    pub start_line: usize,
    pub old_lines: usize,
    pub new_lines: usize,
}

/// A char-level change: `removed` chars at `start` were replaced by `inserted` chars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharEdit {
    pub start: usize,
    pub removed: usize,
    pub inserted: usize,
}

/// One replacement requested by an editing command.
pub struct Replacement {
    pub range: Range<usize>,
    pub text: String,
    /// The selection to leave behind, relative to the start of the inserted text.
    pub caret: Selection,
}

const MERGE_WINDOW_SECS: f64 = 1.0;

pub struct Buffer {
    rope: Rope,
    undo: Vec<Transaction>,
    redo: Vec<Transaction>,
    next_id: u64,
    saved_id: u64,
    line_edits: Vec<LineEdit>,
    char_edits: Vec<CharEdit>,
    /// Bumped on every change, so caches of the text know when they are stale.
    version: u64,
    /// When set, edits and undo/redo are refused.
    pub read_only: bool,
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            undo: Vec::new(),
            redo: Vec::new(),
            next_id: 1,
            saved_id: 0,
            line_edits: Vec::new(),
            char_edits: Vec::new(),
            version: 0,
            read_only: false,
        }
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    #[cfg(test)]
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// The text of `line` without its trailing newline.
    pub fn line_text(&self, line: usize) -> String {
        let slice = self.rope.line(line);
        let mut s = slice.to_string();
        if s.ends_with('\n') {
            s.pop();
        }
        s
    }

    /// Number of chars in `line`, excluding the newline.
    pub fn line_len(&self, line: usize) -> usize {
        let slice = self.rope.line(line);
        let n = slice.len_chars();
        if n > 0 && slice.char(n - 1) == '\n' {
            n - 1
        } else {
            n
        }
    }

    pub fn line_start(&self, line: usize) -> usize {
        self.rope.line_to_char(line)
    }

    pub fn char_to_line(&self, pos: usize) -> usize {
        self.rope.char_to_line(pos.min(self.rope.len_chars()))
    }

    pub fn slice_string(&self, range: Range<usize>) -> String {
        self.rope.slice(range).to_string()
    }

    /// True when the text differs from what was last saved.
    pub fn is_modified(&self) -> bool {
        self.undo.last().map_or(0, |t| t.id) != self.saved_id
    }

    pub fn mark_saved(&mut self) {
        self.saved_id = self.undo.last().map_or(0, |t| t.id);
    }

    /// Forces the buffer to count as modified (e.g. after changing line endings).
    pub fn mark_modified(&mut self) {
        self.saved_id = u64::MAX;
    }

    /// Line ranges changed since the last call, oldest first.
    pub fn take_line_edits(&mut self) -> Vec<LineEdit> {
        std::mem::take(&mut self.line_edits)
    }

    /// Char ranges changed since the last call, oldest first.
    pub fn take_char_edits(&mut self) -> Vec<CharEdit> {
        std::mem::take(&mut self.char_edits)
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Applies non-overlapping replacements (sorted by start) and records them as one undo step.
    /// Returns the new selections.
    pub fn apply(
        &mut self,
        before: &Selections,
        mut replacements: Vec<Replacement>,
        kind: EditKind,
        time: f64,
    ) -> Selections {
        if self.read_only {
            return before.clone();
        }
        replacements.sort_by_key(|r| (r.range.start, r.range.end));
        let mut changes = Vec::with_capacity(replacements.len());
        let mut new_sels = Vec::with_capacity(replacements.len());
        let mut delta: isize = 0;
        let mut last_end = 0usize;
        for r in replacements {
            // Overlapping ranges are clipped so every change applies cleanly.
            let start = r.range.start.max(last_end);
            let end = r.range.end.max(start);
            last_end = end;
            let start = (start as isize + delta) as usize;
            let end = (end as isize + delta) as usize;
            let change = self.replace(start, end, &r.text);
            let inserted_len = r.text.chars().count();
            delta += inserted_len as isize - (end - start) as isize;
            new_sels.push(Selection {
                anchor: start + r.caret.anchor,
                head: start + r.caret.head,
                goal_col: r.caret.goal_col,
            });
            changes.push(change);
        }
        let primary = before.primary_index().min(new_sels.len().saturating_sub(1));
        let after = Selections::from_vec(new_sels, primary);
        if changes
            .iter()
            .all(|c| c.removed.is_empty() && c.inserted.is_empty())
        {
            return after;
        }
        self.record(changes, before.clone(), after.clone(), kind, time);
        after
    }

    /// Applies non-overlapping replacements and leaves `after` as the selections,
    /// for commands that work out where the carets go themselves.
    pub fn apply_explicit(
        &mut self,
        before: &Selections,
        replacements: Vec<Replacement>,
        after: Selections,
        time: f64,
    ) -> Selections {
        if self.read_only {
            return before.clone();
        }
        let version = self.version;
        self.apply(before, replacements, EditKind::Other, time);
        let mut after = after;
        after.clamp(self.rope.len_chars());
        if self.version != version
            && let Some(last) = self.undo.last_mut()
        {
            last.after = after.clone();
        }
        after
    }

    fn replace(&mut self, start: usize, end: usize, text: &str) -> Change {
        let removed = self.rope.slice(start..end).to_string();
        let start_line = self.rope.char_to_line(start);
        self.rope.remove(start..end);
        self.rope.insert(start, text);
        self.line_edits.push(LineEdit {
            start_line,
            old_lines: removed.matches('\n').count() + 1,
            new_lines: text.matches('\n').count() + 1,
        });
        self.char_edits.push(CharEdit {
            start,
            removed: end - start,
            inserted: text.chars().count(),
        });
        self.version += 1;
        Change {
            start,
            removed,
            inserted: text.to_owned(),
        }
    }

    fn record(
        &mut self,
        changes: Vec<Change>,
        before: Selections,
        after: Selections,
        kind: EditKind,
        time: f64,
    ) {
        self.redo.clear();
        if kind != EditKind::Other
            && let Some(last) = self.undo.last_mut()
        {
            let mergeable = last.kind == kind
                && time - last.time < MERGE_WINDOW_SECS
                && last.after == before
                && last.id != self.saved_id
                && !(kind == EditKind::Insert && changes.iter().any(|c| c.inserted.contains('\n')));
            if mergeable {
                last.changes.extend(changes);
                last.after = after;
                last.time = time;
                return;
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.undo.push(Transaction {
            id,
            changes,
            before,
            after,
            kind,
            time,
        });
    }

    /// Reverts the last undo step, returning the selections to restore.
    pub fn undo(&mut self) -> Option<Selections> {
        if self.read_only {
            return None;
        }
        let t = self.undo.pop()?;
        for c in t.changes.iter().rev() {
            let end = c.start + c.inserted.chars().count();
            self.replace(c.start, end, &c.removed);
        }
        let before = t.before.clone();
        self.redo.push(t);
        Some(before)
    }

    pub fn redo(&mut self) -> Option<Selections> {
        if self.read_only {
            return None;
        }
        let t = self.redo.pop()?;
        for c in &t.changes {
            let end = c.start + c.removed.chars().count();
            self.replace(c.start, end, &c.inserted);
        }
        let after = t.after.clone();
        self.undo.push(t);
        Some(after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_at(buf: &mut Buffer, sels: &Selections, text: &str, time: f64) -> Selections {
        let n = text.chars().count();
        let reps = sels
            .iter()
            .map(|s| Replacement {
                range: s.range(),
                text: text.to_owned(),
                caret: Selection::caret(n),
            })
            .collect();
        buf.apply(sels, reps, EditKind::Insert, time)
    }

    #[test]
    fn multi_caret_insert_and_undo() {
        let mut buf = Buffer::new("ab\ncd\n");
        let sels = Selections::from_vec(vec![Selection::caret(1), Selection::caret(4)], 0);
        let sels = insert_at(&mut buf, &sels, "XY", 0.0);
        assert_eq!(buf.text(), "aXYb\ncXYd\n");
        assert_eq!(sels.iter().map(|s| s.head).collect::<Vec<_>>(), vec![3, 8]);
        assert!(buf.is_modified());

        let restored = buf.undo().unwrap();
        assert_eq!(buf.text(), "ab\ncd\n");
        assert_eq!(
            restored.iter().map(|s| s.head).collect::<Vec<_>>(),
            vec![1, 4]
        );
        assert!(!buf.is_modified());

        buf.redo().unwrap();
        assert_eq!(buf.text(), "aXYb\ncXYd\n");
    }

    #[test]
    fn typing_merges_into_one_undo_step() {
        let mut buf = Buffer::new("");
        let mut sels = Selections::caret(0);
        for (i, ch) in "hello".chars().enumerate() {
            sels = insert_at(&mut buf, &sels, &ch.to_string(), i as f64 * 0.1);
        }
        assert_eq!(buf.text(), "hello");
        buf.undo();
        assert_eq!(buf.text(), "");
    }

    #[test]
    fn typing_after_a_pause_is_a_new_step() {
        let mut buf = Buffer::new("");
        let sels = insert_at(&mut buf, &Selections::caret(0), "a", 0.0);
        insert_at(&mut buf, &sels, "b", 5.0);
        buf.undo();
        assert_eq!(buf.text(), "a");
    }

    #[test]
    fn saved_state_tracks_undo() {
        let mut buf = Buffer::new("x");
        let sels = insert_at(&mut buf, &Selections::caret(1), "y", 0.0);
        buf.mark_saved();
        assert!(!buf.is_modified());
        insert_at(&mut buf, &sels, "z", 0.1);
        assert!(buf.is_modified());
        buf.undo();
        assert!(!buf.is_modified());
    }

    #[test]
    fn reports_line_edits() {
        let mut buf = Buffer::new("one\ntwo\nthree");
        buf.take_line_edits();
        let sels = Selections::single(Selection::new(2, 9));
        let reps = vec![Replacement {
            range: 2..9,
            text: "X\nY\nZ\nW".into(),
            caret: Selection::caret(7),
        }];
        buf.apply(&sels, reps, EditKind::Other, 0.0);
        assert_eq!(
            buf.take_line_edits(),
            vec![LineEdit {
                start_line: 0,
                old_lines: 3,
                new_lines: 4
            }]
        );
    }
}
