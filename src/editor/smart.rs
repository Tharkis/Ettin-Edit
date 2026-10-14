//! Typing helpers: auto-closed brackets and quotes, smart indentation, brace
//! and tag matching, word completion, selecting further occurrences, and
//! column (block) selection and insertion.

use std::ops::Range;

use super::Editor;
use super::buffer::{EditKind, Replacement};
use super::layout::{char_cols, tab_width};
use super::selection::{Selection, Selections};

/// How far brace and tag matching looks, in chars, so a stray bracket in a huge
/// file doesn't cost a full scan every frame.
const MATCH_SCAN_LIMIT: usize = 200_000;
/// Words for completion come from this many chars around the caret.
const COMPLETION_WINDOW: usize = 500_000;
const COMPLETION_MAX_ITEMS: usize = 50;
/// Completion opens once a word is this long (Ctrl+Space opens it sooner).
const COMPLETION_MIN_PREFIX: usize = 2;

/// Editing behaviour chosen in the settings and by the document's language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditOptions {
    pub auto_close: bool,
    pub smart_indent: bool,
    /// Tab inserts spaces instead of a tab character.
    pub use_spaces: bool,
    pub completion: bool,
    /// The language indents after a trailing colon (Python, YAML).
    pub colon_indents: bool,
    /// The language has markup tags to match (HTML, XML).
    pub markup: bool,
    /// Typing `>` after an opening tag adds the closing tag (markup only).
    pub close_tags: bool,
    /// HTML's empty elements (`<br>`, `<img>`) get no closing tag.
    pub html: bool,
}

impl Default for EditOptions {
    fn default() -> Self {
        Self {
            auto_close: true,
            smart_indent: true,
            use_spaces: false,
            completion: true,
            colon_indents: false,
            markup: false,
            close_tags: true,
            html: false,
        }
    }
}

/// The word-completion popup's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    /// Where the word being completed starts.
    pub start: usize,
    pub items: Vec<String>,
    pub selected: usize,
}

/// A rectangular selection, in visual rows and display columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ColumnSel {
    pub anchor: (usize, usize),
    pub head: (usize, usize),
}

/// Bracket highlights for the frame: both ends of a pair, or one unmatched bracket.
pub struct BraceMatch {
    pub ranges: Vec<Range<usize>>,
    pub matched: bool,
}

fn closer_of(c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

fn opener_of(c: char) -> Option<char> {
    match c {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        _ => None,
    }
}

/// HTML elements that never have content or a closing tag.
const HTML_VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// How far back to look for the `<` of the tag being closed.
const TAG_SCAN_LIMIT: usize = 4000;

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Editor {
    /// The name of the opening tag that a `>` typed at `pos` would finish,
    /// if it needs a closing tag.
    fn open_tag_before(&self, pos: usize) -> Option<String> {
        let text = self
            .buffer
            .slice_string(pos.saturating_sub(TAG_SCAN_LIMIT)..pos);
        let lt = text.rfind('<')?;
        let tag = &text[lt + 1..];
        if !tag.starts_with(|c: char| c.is_alphabetic() || c == '_') {
            return None; // `</`, `<!`, `<?`, or a lone `<`
        }
        let mut quote = None;
        for c in tag.chars() {
            match (quote, c) {
                (Some(q), c) if c == q => quote = None,
                (None, '"' | '\'') => quote = Some(c),
                (None, '>') => return None,
                _ => {}
            }
        }
        if quote.is_some() || tag.trim_end().ends_with('/') {
            return None;
        }
        let name: String = tag
            .chars()
            .take_while(|&c| c.is_alphanumeric() || matches!(c, '_' | ':' | '-' | '.'))
            .collect();
        if self.opts.html && HTML_VOID.contains(&name.to_ascii_lowercase().as_str()) {
            return None;
        }
        let closer = format!("</{name}>");
        let after = self
            .buffer
            .slice_string(pos..(pos + closer.chars().count()).min(self.buffer.len_chars()));
        (after != closer).then_some(name)
    }

    /// Typing `>` at the end of an opening tag: adds the closing tag after the
    /// caret. Returns false if no caret is at the end of an opening tag.
    pub(super) fn type_tag_end(&mut self, ctx: &egui::Context) -> bool {
        if self.selections.has_selection() {
            return false;
        }
        let names: Vec<(usize, String)> = self
            .selections
            .iter()
            .filter_map(|s| self.open_tag_before(s.head).map(|n| (s.head, n)))
            .collect();
        if names.is_empty() {
            return false;
        }
        self.edit(ctx, EditKind::Other, |_, s| {
            let text = match names.iter().find(|(h, _)| *h == s.head) {
                Some((_, name)) => format!("></{name}>"),
                None => ">".to_owned(),
            };
            Replacement {
                range: s.range(),
                text,
                caret: Selection::caret(1),
            }
        });
        true
    }

    /// One level of indentation.
    pub fn indent_unit(&self) -> String {
        if self.opts.use_spaces {
            " ".repeat(tab_width())
        } else {
            "\t".to_owned()
        }
    }

    fn char_at(&self, pos: usize) -> Option<char> {
        (pos < self.buffer.len_chars()).then(|| self.buffer.rope().char(pos))
    }

    fn char_before(&self, pos: usize) -> Option<char> {
        (pos > 0).then(|| self.buffer.rope().char(pos - 1))
    }

    /// The bracket matching the one at `pos`, scanning at most [`MATCH_SCAN_LIMIT`] chars.
    pub(super) fn matching_bracket(&self, pos: usize) -> Option<usize> {
        let rope = self.buffer.rope();
        let c = self.char_at(pos)?;
        let len = rope.len_chars();
        let mut depth = 0usize;
        if let Some(close) = closer_of(c) {
            let end = len.min(pos + MATCH_SCAN_LIMIT);
            for (i, ch) in rope.chars_at(pos).take(end - pos).enumerate() {
                if ch == c {
                    depth += 1;
                } else if ch == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(pos + i);
                    }
                }
            }
        } else if let Some(open) = opener_of(c) {
            let start = pos.saturating_sub(MATCH_SCAN_LIMIT);
            let mut chars = rope.chars_at(pos + 1);
            let mut i = pos + 1;
            while i > start {
                let ch = chars.prev()?;
                i -= 1;
                if ch == c {
                    depth += 1;
                } else if ch == open {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    /// The brackets (or markup tags) to highlight for the primary caret.
    pub fn brace_match(&self) -> Option<BraceMatch> {
        let sel = self.selections.primary();
        if !sel.is_empty() || self.selections.len() > 1 {
            return None;
        }
        let pos = sel.head;
        // The bracket before the caret wins, like most editors.
        let candidates = [pos.checked_sub(1), Some(pos)];
        for at in candidates.into_iter().flatten() {
            let Some(c) = self.char_at(at) else { continue };
            if closer_of(c).is_none() && opener_of(c).is_none() {
                continue;
            }
            return Some(match self.matching_bracket(at) {
                Some(other) => BraceMatch {
                    ranges: vec![
                        at.min(other)..at.min(other) + 1,
                        at.max(other)..at.max(other) + 1,
                    ],
                    matched: true,
                },
                None => BraceMatch {
                    ranges: std::iter::once(at..at + 1).collect(),
                    matched: false,
                },
            });
        }
        if self.opts.markup {
            return self.matching_tag(pos).map(|ranges| BraceMatch {
                ranges,
                matched: true,
            });
        }
        None
    }

    /// When `pos` is inside a markup tag, the name ranges of it and its partner.
    fn matching_tag(&self, pos: usize) -> Option<Vec<Range<usize>>> {
        let rope = self.buffer.rope();
        let len = rope.len_chars();
        // Find the `<` that starts the tag around the caret.
        let mut lt = None;
        let mut i = pos.min(len);
        let floor = pos.saturating_sub(2000);
        while i > floor {
            i -= 1;
            match rope.char(i) {
                '<' => {
                    lt = Some(i);
                    break;
                }
                '>' if i + 1 < pos => return None,
                _ => {}
            }
        }
        let lt = lt?;
        let closing = self.char_at(lt + 1) == Some('/');
        let name_start = lt + 1 + closing as usize;
        let mut name_end = name_start;
        while name_end < len {
            let c = rope.char(name_end);
            if c.is_alphanumeric() || matches!(c, '-' | '_' | ':' | '.') {
                name_end += 1;
            } else {
                break;
            }
        }
        if name_end == name_start {
            return None;
        }
        let name = self.buffer.slice_string(name_start..name_end);
        // The end of this tag; self-closing tags have no partner.
        let mut gt = name_end;
        while gt < len.min(name_end + 5000) && rope.char(gt) != '>' {
            gt += 1;
        }
        if gt >= len || rope.char(gt) != '>' || pos > gt + 1 {
            return None;
        }
        if !closing && rope.char(gt - 1) == '/' {
            return None;
        }
        let pattern = format!(r"<(/?){}(?:[\s/>]|$)", regex::escape(&name));
        let re = regex::Regex::new(&pattern).ok()?;
        let own = name_start..name_end;
        if !closing {
            let end = len.min(gt + MATCH_SCAN_LIMIT);
            let text = self.buffer.slice_string(gt + 1..end);
            let mut depth = 1usize;
            for m in re.captures_iter(&text) {
                let whole = m.get(0)?;
                let is_close = !m[1].is_empty();
                // Skip `<name/>`.
                let tail_start = whole.end();
                let self_closing = !is_close && {
                    let rest = &text[tail_start.saturating_sub(1)..];
                    let end = rest.find('>').unwrap_or(0);
                    rest[..end].ends_with('/')
                };
                if self_closing {
                    continue;
                }
                if is_close {
                    depth -= 1;
                    if depth == 0 {
                        let start = gt + 1 + text[..whole.start()].chars().count() + 2;
                        return Some(vec![own, start..start + (name_end - name_start)]);
                    }
                } else {
                    depth += 1;
                }
            }
        } else {
            let start = lt.saturating_sub(MATCH_SCAN_LIMIT);
            let text = self.buffer.slice_string(start..lt);
            let tags: Vec<(usize, bool)> = re
                .captures_iter(&text)
                .filter_map(|m| {
                    let whole = m.get(0)?;
                    let is_close = !m[1].is_empty();
                    if !is_close {
                        let rest = &text[whole.end().saturating_sub(1)..];
                        let end = rest.find('>').unwrap_or(0);
                        if rest[..end].ends_with('/') {
                            return None;
                        }
                    }
                    Some((whole.start(), is_close))
                })
                .collect();
            let mut depth = 1usize;
            for &(at, is_close) in tags.iter().rev() {
                if is_close {
                    depth += 1;
                } else {
                    depth -= 1;
                    if depth == 0 {
                        let s = start + text[..at].chars().count() + 1;
                        return Some(vec![s..s + (name_end - name_start), own]);
                    }
                }
            }
        }
        None
    }

    /// Typing one char with auto-close on: wraps selections in brackets or
    /// quotes, inserts the closing half, or steps over a closer already there.
    /// Returns false when the char should just be inserted.
    pub(super) fn type_with_pairs(&mut self, ctx: &egui::Context, c: char) -> bool {
        let quote = matches!(c, '"' | '\'' | '`');
        let close = closer_of(c).or(quote.then_some(c));
        let sels: Vec<Selection> = self.selections.iter().copied().collect();

        // Step over a closer that is already there.
        if (opener_of(c).is_some() || quote)
            && sels
                .iter()
                .all(|s| s.is_empty() && self.char_at(s.head) == Some(c))
            && (!quote || sels.iter().all(|s| self.quote_is_open(s.head, c)))
        {
            self.move_horizontal(true, false, false);
            return true;
        }
        let Some(close) = close else { return false };

        if sels.iter().all(|s| !s.is_empty()) {
            self.edit(ctx, EditKind::Other, |ed, s| {
                let inner = ed.buffer.slice_string(s.range());
                let n = inner.chars().count();
                Replacement {
                    range: s.range(),
                    text: format!("{c}{inner}{close}"),
                    caret: if s.head >= s.anchor {
                        Selection::new(1, n + 1)
                    } else {
                        Selection::new(n + 1, 1)
                    },
                }
            });
            return true;
        }
        let pair_ok = sels.iter().all(|s| {
            let next_ok = self.char_at(s.end()).is_none_or(|n| {
                n.is_whitespace() || matches!(n, ')' | ']' | '}' | ',' | ';' | ':' | '.')
            });
            let prev_ok = !quote
                || (self
                    .char_before(s.start())
                    .is_none_or(|p| !is_word(p) && p != c)
                    && !self.quote_is_open(s.start(), c));
            s.is_empty() && next_ok && prev_ok
        });
        if !pair_ok {
            return false;
        }
        self.edit(ctx, EditKind::Insert, |_, s| Replacement {
            range: s.range(),
            text: format!("{c}{close}"),
            caret: Selection::caret(1),
        });
        true
    }

    /// True when the line before `pos` has an odd number of `quote` chars, so
    /// the next one closes a string.
    fn quote_is_open(&self, pos: usize, quote: char) -> bool {
        let line = self.buffer.char_to_line(pos);
        let start = self.buffer.line_start(line);
        let before = self.buffer.slice_string(start..pos);
        let mut n = 0;
        let mut escaped = false;
        for ch in before.chars() {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                n += 1;
            }
        }
        n % 2 == 1
    }

    /// Backspace between an empty pair such as `()` or `""` removes both halves.
    pub(super) fn delete_pair(&mut self, ctx: &egui::Context) -> bool {
        let all_pairs = self.selections.iter().all(|s| {
            s.is_empty()
                && match (self.char_before(s.head), self.char_at(s.head)) {
                    (Some(a), Some(b)) => {
                        closer_of(a) == Some(b) || (matches!(a, '"' | '\'' | '`') && a == b)
                    }
                    _ => false,
                }
        });
        if !all_pairs {
            return false;
        }
        self.edit(ctx, EditKind::Delete, |_, s| Replacement {
            range: s.head - 1..s.head + 1,
            text: String::new(),
            caret: Selection::caret(0),
        });
        true
    }

    /// Enter with smart indentation: one level deeper after an opening bracket
    /// (or a colon in Python), and a closing bracket right after the caret goes
    /// to its own line.
    pub(super) fn smart_newline(&mut self, ctx: &egui::Context) {
        let unit = self.indent_unit();
        let unit_len = unit.chars().count();
        let colon = self.opts.colon_indents;
        self.edit(ctx, EditKind::Other, |ed, s| {
            let line = ed.buffer.char_to_line(s.start());
            let start = ed.buffer.line_start(line);
            let indent_end = ed.first_non_blank(line).min(s.start());
            let indent = ed.buffer.slice_string(start..indent_end);
            let before = ed.buffer.slice_string(start..s.start());
            let before = before.trim_end();
            let last = before.chars().last();
            let opens =
                last.is_some_and(|c| closer_of(c).is_some()) || (colon && last == Some(':'));
            // Drop trailing spaces left behind on the line being split.
            let trimmed_start = start + before.chars().count();
            let range_start = if trimmed_start >= indent_end {
                trimmed_start
            } else {
                s.start()
            };
            if !opens {
                let text = format!("\n{indent}");
                let n = text.chars().count();
                return Replacement {
                    range: range_start.min(s.start())..s.end(),
                    text,
                    caret: Selection::caret(n),
                };
            }
            let next = ed.char_at(s.end());
            if let (Some(open), Some(next)) = (last, next)
                && closer_of(open) == Some(next)
            {
                let text = format!("\n{indent}{unit}\n{indent}");
                let caret = 1 + indent.chars().count() + unit_len;
                return Replacement {
                    range: range_start.min(s.start())..s.end(),
                    text,
                    caret: Selection::caret(caret),
                };
            }
            let text = format!("\n{indent}{unit}");
            let n = text.chars().count();
            Replacement {
                range: range_start.min(s.start())..s.end(),
                text,
                caret: Selection::caret(n),
            }
        });
    }

    /// Typing a closing bracket on a line that is only indentation lines it up
    /// with the line that opened the bracket.
    pub(super) fn type_closer_outdented(&mut self, ctx: &egui::Context, c: char) -> bool {
        let Some(open) = opener_of(c) else {
            return false;
        };
        let sels: Vec<Selection> = self.selections.iter().copied().collect();
        let mut reps = Vec::new();
        for s in &sels {
            if !s.is_empty() {
                return false;
            }
            let line = self.buffer.char_to_line(s.head);
            let start = self.buffer.line_start(line);
            if self.first_non_blank(line) != s.head || s.head == start {
                return false;
            }
            // Find the opener this closer will match.
            let rope = self.buffer.rope();
            let mut depth = 0usize;
            let mut i = start;
            let floor = start.saturating_sub(MATCH_SCAN_LIMIT);
            let mut found = None;
            let mut chars = rope.chars_at(start);
            while i > floor {
                let Some(ch) = chars.prev() else { break };
                i -= 1;
                if ch == c {
                    depth += 1;
                } else if ch == open {
                    if depth == 0 {
                        found = Some(i);
                        break;
                    }
                    depth -= 1;
                }
            }
            let Some(at) = found else { return false };
            let open_line = self.buffer.char_to_line(at);
            let ls = self.buffer.line_start(open_line);
            let indent = self
                .buffer
                .slice_string(ls..self.first_non_blank(open_line));
            let n = indent.chars().count();
            reps.push(Replacement {
                range: start..s.head,
                text: format!("{indent}{c}"),
                caret: Selection::caret(n + 1),
            });
        }
        let before = self.selections.clone();
        let after = self
            .buffer
            .apply(&before, reps, EditKind::Insert, Self::now(ctx));
        self.set_selections(after);
        true
    }

    // ------------------------------------------------------------------
    // Word completion

    /// Opens or refreshes the completion list for the word before the caret.
    pub fn update_completion(&mut self, forced: bool) {
        if self.selections.len() != 1 || !self.selections.primary().is_empty() {
            self.completion = None;
            return;
        }
        let pos = self.selections.primary().head;
        let rope = self.buffer.rope();
        let mut start = pos;
        while start > 0 && is_word(rope.char(start - 1)) {
            start -= 1;
        }
        let prefix_len = pos - start;
        if prefix_len == 0 || (!forced && prefix_len < COMPLETION_MIN_PREFIX) {
            self.completion = None;
            return;
        }
        // Don't pop up in the middle of a word.
        if self.char_at(pos).is_some_and(is_word) && !forced {
            self.completion = None;
            return;
        }
        let prefix = self.buffer.slice_string(start..pos);
        let lower = prefix.to_lowercase();
        let from = pos.saturating_sub(COMPLETION_WINDOW);
        let to = (pos + COMPLETION_WINDOW).min(rope.len_chars());
        let mut words: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut word = String::new();
        let mut word_start = from;
        for (i, c) in rope.slice(from..to).chars().enumerate() {
            if is_word(c) {
                if word.is_empty() {
                    word_start = from + i;
                }
                word.push(c);
                continue;
            }
            if !word.is_empty() {
                let w = std::mem::take(&mut word);
                if word_start != start
                    && w.chars().count() > prefix_len
                    && w.to_lowercase().starts_with(&lower)
                    && seen.insert(w.clone())
                {
                    words.push(w);
                }
            }
        }
        if !word.is_empty()
            && word_start != start
            && word.chars().count() > prefix_len
            && word.to_lowercase().starts_with(&lower)
            && seen.insert(word.clone())
        {
            words.push(word);
        }
        if words.is_empty() {
            self.completion = None;
            return;
        }
        // Exact-case matches first, then shorter words, then alphabetical.
        words.sort_by(|a, b| {
            (!a.starts_with(&prefix), a.chars().count(), a.as_str()).cmp(&(
                !b.starts_with(&prefix),
                b.chars().count(),
                b.as_str(),
            ))
        });
        words.truncate(COMPLETION_MAX_ITEMS);
        let selected = self
            .completion
            .as_ref()
            .filter(|c| c.start == start)
            .and_then(|c| c.items.get(c.selected))
            .and_then(|w| words.iter().position(|x| x == w))
            .unwrap_or(0);
        self.completion = Some(Completion {
            start,
            items: words,
            selected,
        });
    }

    /// Replaces the word before the caret with the chosen completion.
    pub fn accept_completion(&mut self, ctx: &egui::Context, index: Option<usize>) {
        let Some(c) = self.completion.take() else {
            return;
        };
        let Some(word) = c.items.get(index.unwrap_or(c.selected)) else {
            return;
        };
        let pos = self.selections.primary().head;
        if pos < c.start {
            return;
        }
        let n = word.chars().count();
        let reps = vec![Replacement {
            range: c.start..pos,
            text: word.clone(),
            caret: Selection::caret(n),
        }];
        let before = self.selections.clone();
        let after = self
            .buffer
            .apply(&before, reps, EditKind::Other, Self::now(ctx));
        self.set_selections(after);
    }

    // ------------------------------------------------------------------
    // Occurrences

    /// Selects the word at the caret, or adds the next occurrence of the
    /// selected text as another selection.
    pub fn select_next_occurrence(&mut self) {
        let primary = self.selections.primary();
        if primary.is_empty() {
            let (a, b) = self.word_range(primary.head);
            if a < b && is_word(self.buffer.rope().char(a)) {
                let mut sels = self.selections.clone();
                sels.map(|s| {
                    if s == primary {
                        Selection::new(a, b)
                    } else {
                        s
                    }
                });
                self.set_selections(sels);
            }
            return;
        }
        let needle = self.buffer.slice_string(primary.range());
        let text = self.text_snapshot();
        let rope = self.buffer.rope();
        // Search after the last selection, wrapping to the top.
        let last_end = self.selections.iter().map(|s| s.end()).max().unwrap_or(0);
        let from = rope.char_to_byte(last_end);
        let taken: Vec<Range<usize>> = self.selections.iter().map(|s| s.range()).collect();
        let found = text[from..]
            .match_indices(needle.as_str())
            .map(|(i, _)| from + i)
            .chain(text[..from].match_indices(needle.as_str()).map(|(i, _)| i))
            .map(|b| {
                let start = rope.byte_to_char(b);
                start..start + (primary.end() - primary.start())
            })
            .find(|r| !taken.contains(r));
        if let Some(r) = found {
            let mut sels = self.selections.clone();
            sels.push(Selection::new(r.start, r.end), true);
            self.set_selections(sels);
        }
    }

    /// Selects every occurrence of the selected text (or of the word at the caret).
    pub fn select_all_occurrences(&mut self) -> usize {
        if self.selections.primary().is_empty() {
            self.select_next_occurrence();
        }
        let primary = self.selections.primary();
        if primary.is_empty() {
            return 0;
        }
        let needle = self.buffer.slice_string(primary.range());
        let text = self.text_snapshot();
        let rope = self.buffer.rope();
        let n = primary.end() - primary.start();
        let sels: Vec<Selection> = text
            .match_indices(needle.as_str())
            .take(super::MAX_HIGHLIGHTS)
            .map(|(b, _)| {
                let start = rope.byte_to_char(b);
                Selection::new(start, start + n)
            })
            .collect();
        let count = sels.len();
        let primary_idx = sels
            .iter()
            .position(|s| s.start() == primary.start())
            .unwrap_or(0);
        self.set_selections(Selections::from_vec(sels, primary_idx));
        count
    }

    // ------------------------------------------------------------------
    // Column editing

    /// Inserts `text(i)` at each caret; with a single caret, at the caret's
    /// column on every line from there to the end of the document (short lines
    /// are padded with spaces).
    pub fn column_insert(&mut self, ctx: &egui::Context, text: impl Fn(usize) -> String) {
        if self.selections.len() > 1 {
            let reps: Vec<Replacement> = self
                .selections
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let t = text(i);
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
            return;
        }
        let head = self.selections.primary().head;
        let first = self.buffer.char_to_line(head);
        let ls = self.buffer.line_start(first);
        let col = super::layout::line_cols(self.buffer.rope().slice(ls..head).chars());
        let mut reps = Vec::new();
        for (i, line) in (first..self.buffer.len_lines()).enumerate() {
            let start = self.buffer.line_start(line);
            let mut c = 0;
            let mut p = start;
            let end = self.line_end(line);
            let rope = self.buffer.rope();
            while p < end && c < col {
                let w = char_cols(rope.char(p), c);
                if c + w > col {
                    break;
                }
                c += w;
                p += 1;
            }
            let pad = " ".repeat(col.saturating_sub(c));
            reps.push(Replacement {
                range: p..p,
                text: format!("{pad}{}", text(i)),
                caret: Selection::caret(0),
            });
        }
        let before = self.selections.clone();
        let insert_len = reps.first().map_or(0, |r| r.text.chars().count());
        let after = Selections::caret(head + insert_len);
        let after = self
            .buffer
            .apply_explicit(&before, reps, after, Self::now(ctx));
        self.set_selections(after);
    }
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
    fn closing_tags_are_added() {
        let ctx = egui::Context::default();
        let typed = |text: &str, html: bool| {
            let mut e = ed(text, text.chars().count());
            e.opts.markup = true;
            e.opts.html = html;
            e.type_text(&ctx, ">");
            (e.buffer.text(), e.selections.primary().head)
        };
        assert_eq!(
            typed("<item id=\"a>b\"", false),
            ("<item id=\"a>b\"></item>".into(), 15)
        );
        assert_eq!(typed("<ns:x-y", false).0, "<ns:x-y></ns:x-y>");
        assert_eq!(typed("<a/", false).0, "<a/>");
        assert_eq!(typed("</a", false).0, "</a>");
        assert_eq!(typed("<!--", false).0, "<!-->");
        assert_eq!(typed("<a title='x", false).0, "<a title='x>");
        assert_eq!(typed("a <", false).0, "a <>");
        assert_eq!(typed("<br", true).0, "<br>");
        assert_eq!(typed("<br", false).0, "<br></br>");
        let mut e = ed("<p</p>", 2);
        e.opts.markup = true;
        e.type_text(&ctx, ">");
        assert_eq!(
            e.buffer.text(),
            "<p></p>",
            "an existing closing tag isn't doubled"
        );
    }

    #[test]
    fn auto_close_wraps_and_steps_over() {
        let ctx = egui::Context::default();
        let mut e = ed("x", 1);
        e.type_text(&ctx, "(");
        assert_eq!(e.buffer.text(), "x()");
        e.type_text(&ctx, "a");
        e.type_text(&ctx, ")");
        assert_eq!(e.buffer.text(), "x(a)");
        assert_eq!(e.selections.primary().head, 4);
        // Quotes don't pair after a word char (apostrophes).
        let mut e = ed("don", 3);
        e.type_text(&ctx, "'");
        assert_eq!(e.buffer.text(), "don'");
        let mut e = ed("say hi", 0);
        e.selections = Selections::single(Selection::new(4, 6));
        e.type_text(&ctx, "\"");
        assert_eq!(e.buffer.text(), "say \"hi\"");
        assert_eq!(e.selections.primary().range(), 5..7);
        // Backspace removes an empty pair.
        let mut e = ed("", 0);
        e.type_text(&ctx, "[");
        e.delete(&ctx, false, false);
        assert_eq!(e.buffer.text(), "");
    }

    #[test]
    fn smart_enter_and_closer() {
        let ctx = egui::Context::default();
        let mut e = ed("fn x() {}", 8);
        e.newline(&ctx);
        assert_eq!(e.buffer.text(), "fn x() {\n\t\n}");
        assert_eq!(e.selections.primary().head, 10);
        let mut e = ed("if x:", 5);
        e.opts.colon_indents = true;
        e.opts.use_spaces = true;
        e.newline(&ctx);
        assert_eq!(e.buffer.text(), "if x:\n    ");
        let mut e = ed("  a {\n\t\t", 8);
        e.opts.auto_close = false;
        e.type_text(&ctx, "}");
        assert_eq!(e.buffer.text(), "  a {\n  }");
    }

    #[test]
    fn brace_and_tag_matching() {
        let e = ed("f(a[1], {b})", 2);
        let m = e.brace_match().unwrap();
        assert!(m.matched);
        assert_eq!(m.ranges, vec![1..2, 11..12]);
        let e = ed("(()", 1);
        assert!(!e.brace_match().unwrap().matched);
        let mut e = ed("<a><b/><a>x</a></a>", 1);
        e.opts.markup = true;
        assert_eq!(e.brace_match().unwrap().ranges, vec![1..2, 17..18]);
        e.selections = Selections::caret(18);
        assert_eq!(e.brace_match().unwrap().ranges, vec![1..2, 17..18]);
    }

    #[test]
    fn completion_from_document_words() {
        let ctx = egui::Context::default();
        let mut e = ed("alpha alphabet beta al", 22);
        e.update_completion(false);
        let c = e.completion.clone().unwrap();
        assert_eq!(c.items, vec!["alpha", "alphabet"]);
        e.accept_completion(&ctx, Some(1));
        assert_eq!(e.buffer.text(), "alpha alphabet beta alphabet");
    }

    #[test]
    fn next_and_all_occurrences() {
        let mut e = ed("foo bar foo foo", 1);
        e.select_next_occurrence();
        assert_eq!(e.selections.primary().range(), 0..3);
        e.select_next_occurrence();
        assert_eq!(e.selections.len(), 2);
        assert_eq!(e.selections.primary().range(), 8..11);
        assert_eq!(e.select_all_occurrences(), 3);
    }

    #[test]
    fn column_insert_pads_short_lines() {
        let ctx = egui::Context::default();
        let mut e = ed("abcd\nx\nefgh", 2);
        e.column_insert(&ctx, |i| i.to_string());
        assert_eq!(e.buffer.text(), "ab0cd\nx 1\nef2gh");
    }
}
