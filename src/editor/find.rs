//! Searching and navigating inside one document: find next/previous, replace,
//! marks, bookmarks and go to line.

use std::ops::Range;
use std::sync::Arc;

use super::Editor;
use super::buffer::{EditKind, Replacement};
use super::selection::{Selection, Selections};
use crate::search::{Hit, Query, line_hits};

/// Highlighting stops after this many matches, so a one-letter search in a
/// huge file stays responsive.
pub const MAX_HIGHLIGHTS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindOutcome {
    Found,
    /// Found after wrapping around the end (or start) of the document.
    Wrapped,
    NotFound,
}

impl Editor {
    /// The whole text as one string, rebuilt only after the text changes.
    pub fn text_snapshot(&mut self) -> Arc<String> {
        let version = self.buffer.version();
        if let Some((v, text)) = &self.text_cache
            && *v == version
        {
            return text.clone();
        }
        let text = Arc::new(self.buffer.rope().to_string());
        self.text_cache = Some((version, text.clone()));
        text
    }

    fn char_range(&self, bytes: Range<usize>) -> Range<usize> {
        let rope = self.buffer.rope();
        rope.byte_to_char(bytes.start)..rope.byte_to_char(bytes.end)
    }

    fn select_match(&mut self, range: Range<usize>) {
        self.set_selections(Selections::single(Selection::new(range.start, range.end)));
    }

    /// Selects the next (or previous) non-empty match after (before) the primary selection.
    pub fn find_next(&mut self, query: &Query, forward: bool, wrap: bool) -> FindOutcome {
        let text = self.text_snapshot();
        let rope = self.buffer.rope();
        let sel = self.selections.primary();
        let found = if forward {
            let from = rope.char_to_byte(sel.end());
            let after = query.find_iter_from(&text, from).find(|m| !m.is_empty());
            match after {
                Some(m) => Some((m, false)),
                None if wrap => query
                    .find_iter(&text)
                    .find(|m| !m.is_empty())
                    .map(|m| (m, true)),
                None => None,
            }
        } else {
            let before = rope.char_to_byte(sel.start());
            let mut last_before = None;
            let mut last = None;
            for m in query.find_iter(&text).filter(|m| !m.is_empty()) {
                if m.end <= before {
                    last_before = Some(m.clone());
                } else if !wrap {
                    break;
                }
                last = Some(m);
            }
            match (last_before, last) {
                (Some(m), _) => Some((m, false)),
                (None, Some(m)) if wrap => Some((m, true)),
                _ => None,
            }
        };
        match found {
            Some((m, wrapped)) => {
                let range = self.char_range(m);
                // Wrapping onto the match that is already selected is no news.
                let same = range == sel.range();
                self.select_match(range);
                if wrapped && !same {
                    FindOutcome::Wrapped
                } else {
                    FindOutcome::Found
                }
            }
            None => FindOutcome::NotFound,
        }
    }

    /// Search-as-you-type: selects the first match at or after `origin`
    /// (wrapping to the top), or puts the caret back at `origin`.
    pub fn find_incremental(&mut self, query: &Query, origin: usize, wrap: bool) -> bool {
        let text = self.text_snapshot();
        let from = self
            .buffer
            .rope()
            .char_to_byte(origin.min(self.buffer.len_chars()));
        let found = query
            .find_iter_from(&text, from)
            .find(|m| !m.is_empty())
            .or_else(|| {
                wrap.then(|| query.find_iter(&text).find(|m| !m.is_empty()))
                    .flatten()
            });
        match found {
            Some(m) => {
                let range = self.char_range(m);
                self.select_match(range);
                true
            }
            None => {
                self.set_selections(Selections::caret(origin.min(self.buffer.len_chars())));
                false
            }
        }
    }

    /// Char ranges of non-empty matches, at most `limit`. The flag says whether there were more.
    pub fn match_ranges(&mut self, query: &Query, limit: usize) -> (Vec<Range<usize>>, bool) {
        let text = self.text_snapshot();
        let mut out = Vec::new();
        // Convert byte offsets to char offsets in one pass.
        let mut byte = 0usize;
        let mut chars = 0usize;
        for m in query.find_iter(&text).filter(|m| !m.is_empty()) {
            if out.len() >= limit {
                return (out, true);
            }
            chars += text[byte..m.start].chars().count();
            let len = text[m.clone()].chars().count();
            out.push(chars..chars + len);
            chars += len;
            byte = m.end;
        }
        (out, false)
    }

    /// Matching lines, for the results panel.
    pub fn line_hits(&mut self, query: &Query, limit: usize) -> (Vec<Hit>, bool) {
        let text = self.text_snapshot();
        line_hits(query, &text, limit)
    }

    /// Replaces the selection if it is a match, then finds the next match.
    /// Returns whether a replacement was made.
    pub fn replace_current(
        &mut self,
        ctx: &egui::Context,
        query: &Query,
        template: &str,
        wrap: bool,
    ) -> (bool, FindOutcome) {
        let text = self.text_snapshot();
        let rope = self.buffer.rope();
        let sel = self.selections.primary();
        let bytes = rope.char_to_byte(sel.start())..rope.char_to_byte(sel.end());
        let mut replaced = false;
        if !sel.is_empty() && query.matches_exactly(&text, &bytes) {
            let with = query.expand(&text, &bytes, template);
            let n = with.chars().count();
            let before = self.selections.clone();
            let reps = vec![Replacement {
                range: sel.range(),
                text: with,
                caret: Selection::caret(n),
            }];
            let after = self
                .buffer
                .apply(&before, reps, EditKind::Other, Self::now(ctx));
            self.set_selections(after);
            replaced = true;
        }
        (replaced, self.find_next(query, true, wrap))
    }

    /// Replaces every match (in the selection only, when `in_selection`) as one undo step.
    /// Returns the number of replacements.
    pub fn replace_all(
        &mut self,
        ctx: &egui::Context,
        query: &Query,
        template: &str,
        in_selection: bool,
    ) -> usize {
        let text = self.text_snapshot();
        let rope = self.buffer.rope();
        let scope = if in_selection {
            let s = self.selections.primary();
            rope.char_to_byte(s.start())..rope.char_to_byte(s.end())
        } else {
            0..text.len()
        };
        let mut reps = Vec::new();
        let mut byte = 0usize;
        let mut chars = 0usize;
        for m in query.find_iter(&text) {
            if m.start < scope.start || m.end > scope.end {
                if m.start > scope.end {
                    break;
                }
                continue;
            }
            chars += text[byte..m.start].chars().count();
            let len = text[m.clone()].chars().count();
            let with = query.expand(&text, &m, template);
            reps.push(Replacement {
                range: chars..chars + len,
                caret: Selection::caret(0),
                text: with,
            });
            chars += len;
            byte = m.end;
        }
        let count = reps.len();
        if count == 0 {
            return 0;
        }
        // Keep the caret (or the selection being searched) where it was, shifted by the edits before it.
        let sel = self.selections.primary();
        let shift = |p: usize| -> usize {
            let mut q = p as isize;
            for r in &reps {
                if r.range.end <= p {
                    q += r.text.chars().count() as isize - r.range.len() as isize;
                } else if r.range.start < p {
                    q = q - (p - r.range.start) as isize + r.text.chars().count() as isize;
                }
            }
            q.max(0) as usize
        };
        let new_sel = Selection::new(shift(sel.anchor), shift(sel.head));
        let before = self.selections.clone();
        self.buffer
            .apply(&before, reps, EditKind::Other, Self::now(ctx));
        self.set_selections(Selections::single(new_sel));
        count
    }

    /// Puts a caret selection on every match. Returns how many.
    pub fn select_all_matches(&mut self, query: &Query) -> usize {
        let (ranges, _) = self.match_ranges(query, MAX_HIGHLIGHTS);
        if ranges.is_empty() {
            return 0;
        }
        let n = ranges.len();
        let sels = ranges
            .into_iter()
            .map(|r| Selection::new(r.start, r.end))
            .collect();
        self.set_selections(Selections::from_vec(sels, 0));
        n
    }

    /// Adds every match to the marks, and optionally bookmarks their lines. Returns how many.
    pub fn mark_all(&mut self, query: &Query, bookmark: bool) -> usize {
        let (ranges, _) = self.match_ranges(query, MAX_HIGHLIGHTS);
        if bookmark {
            for r in &ranges {
                self.bookmarks.insert(self.buffer.char_to_line(r.start));
            }
        }
        let n = ranges.len();
        self.marks.extend(ranges);
        self.marks.sort_by_key(|r| (r.start, r.end));
        self.marks.dedup();
        n
    }

    pub fn clear_marks(&mut self) {
        self.marks.clear();
    }

    /// Moves the caret to `line` and `col` (both 0-based, clamped) and scrolls there.
    pub fn go_to(&mut self, line: usize, col: usize) {
        let line = line.min(self.buffer.len_lines().saturating_sub(1));
        let pos = self.buffer.line_start(line) + col.min(self.buffer.line_len(line));
        self.set_selections(Selections::caret(pos));
    }

    /// Selects `len` chars at `line`/`col` (0-based), for jumping to a search result.
    pub fn select_at(&mut self, line: usize, col: usize, len: usize) {
        let line = line.min(self.buffer.len_lines().saturating_sub(1));
        let start = self.buffer.line_start(line) + col.min(self.buffer.line_len(line));
        let end = (start + len).min(self.buffer.len_chars());
        self.select_match(start..end);
    }

    /// The selected word, when the selection is exactly one whole word on one line.
    /// Used to highlight its other occurrences.
    pub fn selected_word(&self) -> Option<String> {
        if self.selections.len() != 1 {
            return None;
        }
        let s = self.selections.primary();
        if s.is_empty() || s.end() - s.start() > 200 {
            return None;
        }
        let rope = self.buffer.rope();
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let word: String = rope.slice(s.range()).chars().collect();
        if !word.chars().all(is_word) {
            return None;
        }
        let before = (s.start() > 0).then(|| rope.char(s.start() - 1));
        let after = (s.end() < rope.len_chars()).then(|| rope.char(s.end()));
        if before.is_some_and(is_word) || after.is_some_and(is_word) {
            return None;
        }
        Some(word)
    }

    /// The selection's text, or the word at the caret, for Find Selected.
    pub fn selection_or_word(&self) -> Option<String> {
        let s = self.selections.primary();
        let range = if s.is_empty() {
            let (a, b) = self.word_range(s.head);
            a..b
        } else {
            s.range()
        };
        let text = self.buffer.slice_string(range);
        (!text.trim().is_empty() && !text.contains('\n')).then_some(text)
    }

    // ------------------------------------------------------------------
    // Bookmarks

    pub fn toggle_bookmark(&mut self, line: usize) {
        if !self.bookmarks.remove(&line) {
            self.bookmarks.insert(line);
        }
    }

    pub fn toggle_bookmark_at_caret(&mut self) {
        let line = self.buffer.char_to_line(self.selections.primary().head);
        self.toggle_bookmark(line);
    }

    /// Moves to the next (or previous) bookmarked line, wrapping around.
    pub fn goto_bookmark(&mut self, forward: bool) -> bool {
        let line = self.buffer.char_to_line(self.selections.primary().head);
        let target = if forward {
            self.bookmarks
                .range(line + 1..)
                .next()
                .or_else(|| self.bookmarks.iter().next())
        } else {
            self.bookmarks
                .range(..line)
                .next_back()
                .or_else(|| self.bookmarks.iter().next_back())
        };
        match target.copied() {
            Some(l) => {
                self.go_to(l, 0);
                true
            }
            None => false,
        }
    }

    pub fn inverse_bookmarks(&mut self) {
        let all = 0..self.buffer.len_lines();
        self.bookmarks = all.filter(|l| !self.bookmarks.contains(l)).collect();
    }

    /// Text of the bookmarked lines, each ending with a newline.
    pub fn bookmarked_text(&self) -> String {
        let mut out = String::new();
        for &l in &self.bookmarks {
            if l < self.buffer.len_lines() {
                out.push_str(&self.buffer.line_text(l));
                out.push('\n');
            }
        }
        out
    }

    /// Deletes the bookmarked lines (or, with `keep_bookmarked`, every other line).
    pub fn remove_lines(&mut self, ctx: &egui::Context, keep_bookmarked: bool) {
        self.sync_caches();
        let n = self.buffer.len_lines();
        let doomed: Vec<usize> = (0..n)
            .filter(|l| self.bookmarks.contains(l) != keep_bookmarked)
            .collect();
        if doomed.is_empty() {
            return;
        }
        // Group consecutive lines into runs and delete each run with its newline.
        let mut reps = Vec::new();
        let mut i = 0;
        while i < doomed.len() {
            let first = doomed[i];
            let mut last = first;
            while i + 1 < doomed.len() && doomed[i + 1] == last + 1 {
                i += 1;
                last = doomed[i];
            }
            i += 1;
            let range = if last + 1 < n {
                self.buffer.line_start(first)..self.buffer.line_start(last + 1)
            } else if first > 0 {
                // The run reaches the end: take the newline before it instead.
                self.line_end(first - 1)..self.buffer.len_chars()
            } else {
                0..self.buffer.len_chars()
            };
            reps.push(Replacement {
                range,
                text: String::new(),
                caret: Selection::caret(0),
            });
        }
        let kept = n - doomed.len();
        let before = self.selections.clone();
        let after = self
            .buffer
            .apply(&before, reps, EditKind::Other, Self::now(ctx));
        let caret = after.primary().head.min(self.buffer.len_chars());
        self.set_selections(Selections::caret(caret));
        self.sync_caches();
        self.bookmarks = if keep_bookmarked {
            (0..kept.min(self.buffer.len_lines())).collect()
        } else {
            Default::default()
        };
    }
}

impl Query {
    /// Matches starting at or after byte `from`.
    pub fn find_iter_from<'a>(
        &'a self,
        text: &'a str,
        from: usize,
    ) -> impl Iterator<Item = Range<usize>> + 'a {
        let mut at = from;
        std::iter::from_fn(move || {
            if at > text.len() {
                return None;
            }
            let m = self.find_at(text, at)?;
            at = if m.is_empty() {
                m.end + text[m.end..].chars().next().map_or(1, char::len_utf8)
            } else {
                m.end
            };
            Some(m)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchOptions;

    fn query(p: &str) -> Query {
        Query::new(p, &SearchOptions::default()).unwrap().unwrap()
    }

    fn regex(p: &str) -> Query {
        let opts = SearchOptions {
            regex: true,
            ..Default::default()
        };
        Query::new(p, &opts).unwrap().unwrap()
    }

    #[test]
    fn find_next_and_previous_wrap() {
        let mut e = Editor::new("ab ab ab");
        let q = query("ab");
        assert_eq!(e.find_next(&q, true, true), FindOutcome::Found);
        assert_eq!(e.selections.primary().range(), 0..2);
        e.find_next(&q, true, true);
        e.find_next(&q, true, true);
        assert_eq!(e.selections.primary().range(), 6..8);
        assert_eq!(e.find_next(&q, true, false), FindOutcome::NotFound);
        assert_eq!(e.find_next(&q, true, true), FindOutcome::Wrapped);
        assert_eq!(e.selections.primary().range(), 0..2);
        assert_eq!(e.find_next(&q, false, true), FindOutcome::Wrapped);
        assert_eq!(e.selections.primary().range(), 6..8);
        assert_eq!(e.find_next(&q, false, true), FindOutcome::Found);
        assert_eq!(e.selections.primary().range(), 3..5);
    }

    #[test]
    fn char_offsets_with_multibyte_text() {
        let mut e = Editor::new("日本語 abc 日本");
        let q = query("日本");
        e.selections = Selections::caret(1);
        e.find_next(&q, true, true);
        assert_eq!(e.selections.primary().range(), 8..10);
        let (ranges, more) = e.match_ranges(&q, 10);
        assert_eq!(ranges, vec![0..2, 8..10]);
        assert!(!more);
    }

    #[test]
    fn replace_current_then_all_with_groups() {
        let ctx = egui::Context::default();
        let mut e = Editor::new("a=1 b=2 c=3");
        let q = regex(r"(\w)=(\d)");
        e.find_next(&q, true, true);
        let (replaced, _) = e.replace_current(&ctx, &q, r"\2:\1", true);
        assert!(replaced);
        assert_eq!(e.buffer.text(), "1:a b=2 c=3");
        assert_eq!(e.selections.primary().range(), 4..7);
        assert_eq!(e.replace_all(&ctx, &q, "$2$1", false), 2);
        assert_eq!(e.buffer.text(), "1:a 2b 3c");
        e.undo();
        assert_eq!(e.buffer.text(), "1:a b=2 c=3");
    }

    #[test]
    fn replace_all_in_selection_and_at_line_starts() {
        let ctx = egui::Context::default();
        let mut e = Editor::new("x x\nx x");
        e.selections = Selections::single(Selection::new(0, 3));
        assert_eq!(e.replace_all(&ctx, &query("x"), "yy", true), 2);
        assert_eq!(e.buffer.text(), "yy yy\nx x");
        assert_eq!(e.selections.primary().range(), 0..5);
        let mut e = Editor::new("a\nb");
        assert_eq!(e.replace_all(&ctx, &regex("^"), "> ", false), 2);
        assert_eq!(e.buffer.text(), "> a\n> b");
    }

    #[test]
    fn incremental_search_returns_to_origin() {
        let mut e = Editor::new("one two one");
        assert!(e.find_incremental(&query("one"), 1, true));
        assert_eq!(e.selections.primary().range(), 8..11);
        assert!(!e.find_incremental(&query("zzz"), 4, true));
        assert_eq!(e.selections.primary().range(), 4..4);
    }

    #[test]
    fn selected_word_needs_a_whole_word() {
        let mut e = Editor::new("foo foobar");
        e.selections = Selections::single(Selection::new(0, 3));
        assert_eq!(e.selected_word().as_deref(), Some("foo"));
        e.selections = Selections::single(Selection::new(4, 7));
        assert_eq!(e.selected_word(), None);
    }

    #[test]
    fn bookmarks_navigate_and_remove_lines() {
        let ctx = egui::Context::default();
        let mut e = Editor::new("a\nb\nc\nd");
        e.toggle_bookmark(1);
        e.toggle_bookmark(3);
        assert!(e.goto_bookmark(true));
        assert_eq!(e.caret_line_col().0, 2);
        assert!(e.goto_bookmark(true));
        assert_eq!(e.caret_line_col().0, 4);
        assert!(e.goto_bookmark(true));
        assert_eq!(e.caret_line_col().0, 2);
        assert_eq!(e.bookmarked_text(), "b\nd\n");
        e.remove_lines(&ctx, false);
        assert_eq!(e.buffer.text(), "a\nc");
        assert!(e.bookmarks.is_empty());

        let mut e = Editor::new("a\nb\nc");
        e.toggle_bookmark(1);
        e.remove_lines(&ctx, true);
        assert_eq!(e.buffer.text(), "b");
        assert_eq!(e.bookmarks.iter().copied().collect::<Vec<_>>(), vec![0]);
    }

    #[test]
    fn mark_all_bookmarks_lines() {
        let mut e = Editor::new("x\ny\nx");
        assert_eq!(e.mark_all(&query("x"), true), 2);
        assert_eq!(e.marks, vec![0..1, 4..5]);
        assert_eq!(e.bookmarks.iter().copied().collect::<Vec<_>>(), vec![0, 2]);
    }
}
