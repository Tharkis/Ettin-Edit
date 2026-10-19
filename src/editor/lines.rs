//! Line and text tools: duplicate, delete, move, join, split, sort, remove
//! duplicates or blank lines, trim, tabs and spaces, case conversion and
//! comments. Each command is one undo step.

use std::cmp::Ordering;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::Editor;
use super::buffer::Replacement;
use super::layout::{char_cols, tab_width};
use super::selection::{Selection, Selections};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortKind {
    /// By text, A before a.
    Lexical,
    /// By text, ignoring case.
    IgnoreCase,
    /// By the integer at the start of the line.
    Integer,
    /// By the decimal number at the start of the line (`.` as the separator).
    Decimal,
    /// By length.
    Length,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseKind {
    Upper,
    Lower,
    /// First letter of every word upper case.
    Title,
    /// First letter of every sentence upper case.
    Sentence,
    /// Swaps upper and lower case.
    Invert,
    /// Identifier cases: each line's words joined as one name.
    /// `camelCase`
    Camel,
    /// `PascalCase`
    Pascal,
    /// `snake_case`
    Snake,
    /// `kebab-case`
    Kebab,
    /// `CONSTANT_CASE`
    Constant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrimKind {
    Trailing,
    Leading,
    Both,
}

/// One replacement over whole lines, with the selections to leave inside it,
/// relative to the start of the new text.
struct BlockEdit {
    range: Range<usize>,
    text: String,
    sels: Vec<Selection>,
}

/// Comment tokens for a language: line comment, and block comment start/end.
pub struct CommentTokens {
    pub line: Option<&'static str>,
    pub block: Option<(&'static str, &'static str)>,
}

/// Comment tokens for the syntect language `name`.
pub fn comment_tokens(name: &str) -> CommentTokens {
    let c = (Some("//"), Some(("/*", "*/")));
    let hash = (Some("#"), None);
    let xml = (None, Some(("<!--", "-->")));
    let (line, block) = match name {
        "Rust" | "C" | "C++" | "C#" | "Java" | "JavaScript" | "JavaScript (Babel)"
        | "TypeScript" | "TypeScriptReact" | "Go" | "Objective-C" | "Objective-C++" | "Scala"
        | "Swift" | "Kotlin" | "Dart" | "D" | "Groovy" | "JSON" | "PHP" | "ActionScript"
        | "Zig" => c,
        "CSS" => (None, Some(("/*", "*/"))),
        "Sass" | "SCSS" | "Less" => c,
        "Python"
        | "Ruby"
        | "Perl"
        | "Shell-Unix-Generic"
        | "Bourne Again Shell (bash)"
        | "R"
        | "YAML"
        | "TOML"
        | "Makefile"
        | "CMake"
        | "Dockerfile"
        | "PowerShell"
        | "Nim"
        | "Elixir"
        | "Julia"
        | "Crystal"
        | "Tcl"
        | "fish"
        | "Git Config"
        | "Git Ignore"
        | "Python (Ansible)"
        | "Properties" => hash,
        "HTML" | "XML" | "Markdown" | "MultiMarkdown" | "SVG" | "Vue Component" => xml,
        "SQL" | "Lua" | "Haskell" | "Ada" | "Elm" => (Some("--"), None),
        "Batch File" => (Some("REM"), None),
        "Lisp" | "Clojure" | "INI" | "Assembly x86 (NASM)" | "Scheme" => (Some(";"), None),
        "MATLAB" | "LaTeX" | "TeX" | "Erlang" | "Prolog" => (Some("%"), None),
        "Visual Basic" | "VBScript" | "ASP" => (Some("'"), None),
        "Fortran" => (Some("!"), None),
        "OCaml" | "F#" | "Pascal" | "Delphi" => (None, Some(("(*", "*)"))),
        _ => (Some("#"), None),
    };
    CommentTokens { line, block }
}

fn leading_ws(s: &str) -> &str {
    &s[..s.len() - s.trim_start_matches([' ', '\t']).len()]
}

/// The number at the start of `s` (after spaces), for numeric sorting.
fn leading_number(s: &str, decimal: bool) -> Option<f64> {
    let t = s.trim_start();
    let mut end = 0;
    let bytes = t.as_bytes();
    if end < bytes.len() && (bytes[end] == b'-' || bytes[end] == b'+') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if decimal && end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end == digits_start || (end == digits_start + 1 && bytes[digits_start] == b'.') {
        return None;
    }
    t[..end].parse().ok()
}

pub fn sort_lines(lines: &mut [String], kind: SortKind, desc: bool) {
    let cmp = |a: &String, b: &String| -> Ordering {
        match kind {
            SortKind::Lexical => a.cmp(b),
            SortKind::IgnoreCase => a
                .to_lowercase()
                .cmp(&b.to_lowercase())
                .then_with(|| a.cmp(b)),
            SortKind::Integer | SortKind::Decimal => {
                let decimal = kind == SortKind::Decimal;
                match (leading_number(a, decimal), leading_number(b, decimal)) {
                    (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
                    // Lines without a number go first.
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                    (None, None) => a.cmp(b),
                }
            }
            SortKind::Length => a.chars().count().cmp(&b.chars().count()),
        }
    };
    lines.sort_by(|a, b| if desc { cmp(b, a) } else { cmp(a, b) });
}

/// Converts the case of `text`.
pub fn convert_case(text: &str, kind: CaseKind) -> String {
    match kind {
        CaseKind::Upper => text.to_uppercase(),
        CaseKind::Lower => text.to_lowercase(),
        CaseKind::Invert => text
            .chars()
            .flat_map(|c| -> Box<dyn Iterator<Item = char>> {
                if c.is_uppercase() {
                    Box::new(c.to_lowercase())
                } else if c.is_lowercase() {
                    Box::new(c.to_uppercase())
                } else {
                    Box::new(std::iter::once(c))
                }
            })
            .collect(),
        CaseKind::Title => {
            let mut out = String::with_capacity(text.len());
            let mut at_word_start = true;
            for c in text.chars() {
                if c.is_alphanumeric() {
                    if at_word_start {
                        out.extend(c.to_uppercase());
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    at_word_start = false;
                } else {
                    out.push(c);
                    // Apostrophes keep words like "don't" together.
                    at_word_start = c != '\'' && c != '’';
                }
            }
            out
        }
        CaseKind::Camel
        | CaseKind::Pascal
        | CaseKind::Snake
        | CaseKind::Kebab
        | CaseKind::Constant => text
            .split('\n')
            .map(|line| identifier_case(line, kind))
            .collect::<Vec<_>>()
            .join("\n"),
        CaseKind::Sentence => {
            let mut out = String::with_capacity(text.len());
            let mut at_sentence_start = true;
            for c in text.chars() {
                if c.is_alphabetic() {
                    if at_sentence_start {
                        out.extend(c.to_uppercase());
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    at_sentence_start = false;
                } else {
                    if matches!(c, '.' | '!' | '?' | '\n') {
                        at_sentence_start = true;
                    }
                    out.push(c);
                }
            }
            out
        }
    }
}

/// The words of an identifier or phrase: split at anything that isn't a
/// letter or digit, and where case changes (`parseHTTPRequest` is `parse`,
/// `HTTP`, `Request`).
pub fn identifier_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    for part in text.split(|c: char| !c.is_alphanumeric()) {
        let chars: Vec<char> = part.chars().collect();
        let mut start = 0;
        for i in 1..chars.len() {
            let (prev, c) = (chars[i - 1], chars[i]);
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if c.is_uppercase()
                && (prev.is_lowercase() || prev.is_numeric() || (prev.is_uppercase() && next_lower))
            {
                words.push(chars[start..i].iter().collect());
                start = i;
            }
        }
        if start < chars.len() {
            words.push(chars[start..].iter().collect());
        }
    }
    words
}

/// One line as an identifier, keeping the spaces around it.
fn identifier_case(line: &str, kind: CaseKind) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return line.to_owned();
    }
    let lead = &line[..line.len() - line.trim_start().len()];
    let tail = &line[lead.len() + trimmed.len()..];
    let words = identifier_words(trimmed);
    let capital = |w: &str| {
        let mut c = w.chars();
        c.next().map_or(String::new(), |f| {
            f.to_uppercase()
                .chain(c.flat_map(char::to_lowercase))
                .collect()
        })
    };
    let name = match kind {
        CaseKind::Camel => words
            .iter()
            .enumerate()
            .map(|(i, w)| if i == 0 { w.to_lowercase() } else { capital(w) })
            .collect(),
        CaseKind::Pascal => words.iter().map(|w| capital(w)).collect(),
        CaseKind::Snake => words
            .iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join("_"),
        CaseKind::Kebab => words
            .iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join("-"),
        _ => words
            .iter()
            .map(|w| w.to_uppercase())
            .collect::<Vec<_>>()
            .join("_"),
    };
    format!("{lead}{name}{tail}")
}

/// Expands tabs to spaces, keeping the text aligned.
/// Removes repeated lines, keeping the first; with `consecutive`, only
/// lines equal to the one before them.
pub fn remove_duplicates(lines: Vec<String>, consecutive: bool) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for l in lines {
        let dup = if consecutive {
            out.last() == Some(&l)
        } else {
            !seen.insert(l.clone())
        };
        if !dup {
            out.push(l);
        }
    }
    out
}

/// Removes empty lines, and with `blank` also lines of only spaces and tabs.
/// Leaves one empty line when nothing else is left.
pub fn remove_empty(lines: Vec<String>, blank: bool) -> Vec<String> {
    let out: Vec<String> = lines
        .into_iter()
        .filter(|l| {
            if blank {
                !l.trim().is_empty()
            } else {
                !l.is_empty()
            }
        })
        .collect();
    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

pub fn trim_line(line: &str, kind: TrimKind) -> String {
    match kind {
        TrimKind::Trailing => line.trim_end_matches([' ', '\t']).to_owned(),
        TrimKind::Leading => line.trim_start_matches([' ', '\t']).to_owned(),
        TrimKind::Both => line.trim_matches([' ', '\t']).to_owned(),
    }
}

/// Leading spaces (or, with `all`, every run of spaces that reaches a tab
/// stop) as tabs.
pub fn spaces_to_tabs(line: &str, all: bool) -> String {
    compress_spaces(&expand_tabs_leading(line, all), all)
}

pub fn expand_tabs(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut col = 0;
    for c in line.chars() {
        let w = char_cols(c, col);
        if c == '\t' {
            out.extend(std::iter::repeat_n(' ', w));
        } else {
            out.push(c);
        }
        col += w;
    }
    out
}

/// Turns runs of spaces that reach a tab stop into tabs; only in the
/// indentation unless `all`.
fn compress_spaces(line: &str, all: bool) -> String {
    let mut out = String::with_capacity(line.len());
    let mut col = 0usize;
    let mut pending = 0usize;
    let mut in_indent = true;
    for c in line.chars() {
        if c == ' ' && (all || in_indent) {
            pending += 1;
            col += 1;
            if col.is_multiple_of(tab_width()) {
                out.push(if pending > 1 { '\t' } else { ' ' });
                pending = 0;
            }
            continue;
        }
        out.extend(std::iter::repeat_n(' ', pending));
        pending = 0;
        if c == '\t' {
            col += tab_width() - col % tab_width();
        } else {
            col += char_cols(c, col);
            in_indent = false;
        }
        out.push(c);
    }
    out.extend(std::iter::repeat_n(' ', pending));
    out
}

/// Greedy word wrap of one line at `width` columns, keeping its indentation.
fn wrap_line(line: &str, width: usize) -> Vec<String> {
    let indent = leading_ws(line);
    let indent_cols = super::layout::line_cols(indent.chars());
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut cur_cols = indent_cols;
    for word in line[indent.len()..].split(' ').filter(|w| !w.is_empty()) {
        let w = word.chars().count();
        if !cur.is_empty() && cur_cols + 1 + w > width {
            out.push(format!("{indent}{cur}"));
            cur.clear();
            cur_cols = indent_cols;
        }
        if !cur.is_empty() {
            cur.push(' ');
            cur_cols += 1;
        }
        cur.push_str(word);
        cur_cols += w;
    }
    if !cur.is_empty() || out.is_empty() {
        out.push(format!("{indent}{cur}"));
    }
    out
}

impl Editor {
    /// Lines touched by each selection, merged when they overlap or touch,
    /// as inclusive `(first, last)` pairs. A selection that ends at the very
    /// start of a line doesn't include that line.
    pub(super) fn line_blocks(&self) -> Vec<(usize, usize)> {
        let mut blocks: Vec<(usize, usize)> = Vec::new();
        for s in self.selections.iter() {
            let first = self.buffer.char_to_line(s.start());
            let mut last = self.buffer.char_to_line(s.end());
            if last > first && s.end() == self.buffer.line_start(last) {
                last -= 1;
            }
            match blocks.last_mut() {
                Some(b) if first <= b.1 + 1 => b.1 = b.1.max(last),
                _ => blocks.push((first, last)),
            }
        }
        blocks
    }

    /// The selected lines, or the whole document when nothing is selected.
    fn blocks_or_all(&self) -> (Vec<(usize, usize)>, bool) {
        if self.selections.has_selection() {
            (self.line_blocks(), true)
        } else {
            (vec![(0, self.buffer.len_lines() - 1)], false)
        }
    }

    fn lines_range(&self, first: usize, last: usize) -> Range<usize> {
        self.buffer.line_start(first)..self.line_end(last)
    }

    fn lines_of(&self, first: usize, last: usize) -> Vec<String> {
        (first..=last).map(|l| self.buffer.line_text(l)).collect()
    }

    /// Applies block edits (sorted, non-overlapping) as one undo step.
    fn apply_blocks(&mut self, ctx: &egui::Context, mut edits: Vec<BlockEdit>) {
        if edits.is_empty() {
            return;
        }
        edits.sort_by_key(|e| e.range.start);
        let mut delta = 0isize;
        let mut reps = Vec::with_capacity(edits.len());
        let mut sels = Vec::new();
        for e in edits {
            let new_start = (e.range.start as isize + delta) as usize;
            sels.extend(e.sels.iter().map(|s| Selection {
                anchor: new_start + s.anchor,
                head: new_start + s.head,
                goal_col: None,
            }));
            delta += e.text.chars().count() as isize - e.range.len() as isize;
            reps.push(Replacement {
                range: e.range,
                text: e.text,
                caret: Selection::caret(0),
            });
        }
        let before = self.selections.clone();
        let primary = before.primary_index().min(sels.len().saturating_sub(1));
        let after = Selections::from_vec(sels, primary);
        let after = self
            .buffer
            .apply_explicit(&before, reps, after, Self::now(ctx));
        self.set_selections(after);
    }

    /// Selections inside `first..=last`, relative to the start of that range.
    fn sels_in_lines(&self, first: usize, last: usize, offset: isize) -> Vec<Selection> {
        let range = self.lines_range(first, last);
        self.selections
            .iter()
            .filter(|s| s.start() >= range.start && s.end() <= range.end + 1)
            .map(|s| {
                let rel = |p: usize| ((p - range.start) as isize + offset) as usize;
                Selection::new(rel(s.anchor), rel(s.head))
            })
            .collect()
    }

    /// Replaces each block's lines with `f(lines)`, as one undo step. A block that
    /// came from a selection stays selected; otherwise the caret keeps its line
    /// and column where it can.
    fn transform_lines(&mut self, ctx: &egui::Context, f: impl Fn(Vec<String>) -> Vec<String>) {
        let (blocks, selected) = self.blocks_or_all();
        let caret = self.selections.primary().head;
        let caret_line = self.buffer.char_to_line(caret);
        let caret_col = caret - self.buffer.line_start(caret_line);
        let mut edits = Vec::new();
        for (first, last) in blocks {
            let old = self.lines_of(first, last);
            let new = f(old.clone());
            if new == old {
                continue;
            }
            let text = new.join("\n");
            let len = text.chars().count();
            let sels = if selected {
                vec![Selection::new(0, len)]
            } else {
                let line = (caret_line - first).min(new.len().saturating_sub(1));
                let offset: usize = new[..line].iter().map(|l| l.chars().count() + 1).sum();
                let col = caret_col.min(new.get(line).map_or(0, |l| l.chars().count()));
                vec![Selection::caret(offset + col)]
            };
            edits.push(BlockEdit {
                range: self.lines_range(first, last),
                text,
                sels,
            });
        }
        self.apply_blocks(ctx, edits);
    }

    /// Duplicates the selected text, or the caret lines when nothing is selected.
    pub fn duplicate(&mut self, ctx: &egui::Context) {
        let edits = if self.selections.has_selection() {
            self.selections
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| {
                    let t = self.buffer.slice_string(s.range());
                    let n = s.end() - s.start();
                    let sel = if s.head >= s.anchor {
                        Selection::new(0, n)
                    } else {
                        Selection::new(n, 0)
                    };
                    BlockEdit {
                        range: s.range(),
                        text: format!("{t}{t}"),
                        sels: vec![sel],
                    }
                })
                .collect()
        } else {
            self.line_blocks()
                .into_iter()
                .map(|(first, last)| {
                    let t = self.buffer.slice_string(self.lines_range(first, last));
                    BlockEdit {
                        range: self.lines_range(first, last),
                        text: format!("{t}\n{t}"),
                        sels: self.sels_in_lines(first, last, 0),
                    }
                })
                .collect()
        };
        self.apply_blocks(ctx, edits);
    }

    /// Deletes every line a selection or caret is on.
    pub fn delete_lines(&mut self, ctx: &egui::Context) {
        let n = self.buffer.len_lines();
        let edits = self
            .line_blocks()
            .into_iter()
            .map(|(first, last)| {
                let range = if last + 1 < n {
                    self.buffer.line_start(first)..self.buffer.line_start(last + 1)
                } else if first > 0 {
                    self.line_end(first - 1)..self.buffer.len_chars()
                } else {
                    0..self.buffer.len_chars()
                };
                BlockEdit {
                    range,
                    text: String::new(),
                    sels: vec![Selection::caret(0)],
                }
            })
            .collect();
        self.apply_blocks(ctx, edits);
    }

    /// Moves the selected lines up or down by one line.
    pub fn move_lines(&mut self, ctx: &egui::Context, up: bool) {
        let n = self.buffer.len_lines();
        let blocks = self.line_blocks();
        if blocks
            .iter()
            .any(|&(first, last)| (up && first == 0) || (!up && last + 1 >= n))
        {
            return;
        }
        let edits = blocks
            .into_iter()
            .map(|(first, last)| {
                let block = self.buffer.slice_string(self.lines_range(first, last));
                if up {
                    let prev = self.buffer.line_text(first - 1);
                    BlockEdit {
                        range: self.lines_range(first - 1, last),
                        text: format!("{block}\n{prev}"),
                        sels: self.sels_in_lines(first, last, 0),
                    }
                } else {
                    let next = self.buffer.line_text(last + 1);
                    let shift = next.chars().count() as isize + 1;
                    BlockEdit {
                        range: self.lines_range(first, last + 1),
                        text: format!("{next}\n{block}"),
                        sels: self.sels_in_lines(first, last, shift),
                    }
                }
            })
            .collect();
        self.apply_blocks(ctx, edits);
        // Bookmarks follow the moved lines.
        self.sync_caches();
    }

    /// Joins the selected lines into one (or the caret line with the next).
    pub fn join_lines(&mut self, ctx: &egui::Context) {
        let n = self.buffer.len_lines();
        let edits = self
            .line_blocks()
            .into_iter()
            .filter_map(|(first, last)| {
                let last = if last == first { first + 1 } else { last };
                if last >= n {
                    return None;
                }
                let mut joined = self.buffer.line_text(first).trim_end().to_owned();
                for l in first + 1..=last {
                    let part = self.buffer.line_text(l);
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }
                    if !joined.is_empty() && !joined.ends_with([' ', '\t']) {
                        joined.push(' ');
                    }
                    joined.push_str(part);
                }
                let len = joined.chars().count();
                Some(BlockEdit {
                    range: self.lines_range(first, last),
                    text: joined,
                    sels: vec![Selection::caret(len)],
                })
            })
            .collect();
        self.apply_blocks(ctx, edits);
    }

    /// Re-wraps the selected lines (or the caret line) at `width` columns.
    pub fn split_lines(&mut self, ctx: &egui::Context, width: usize) {
        let width = width.max(10);
        let edits = self
            .line_blocks()
            .into_iter()
            .filter_map(|(first, last)| {
                let old = self.lines_of(first, last);
                let new: Vec<String> = old.iter().flat_map(|l| wrap_line(l, width)).collect();
                if new == old {
                    return None;
                }
                let text = new.join("\n");
                let len = text.chars().count();
                Some(BlockEdit {
                    range: self.lines_range(first, last),
                    text,
                    sels: vec![Selection::caret(len)],
                })
            })
            .collect();
        self.apply_blocks(ctx, edits);
    }

    pub fn sort_lines(&mut self, ctx: &egui::Context, kind: SortKind, desc: bool) {
        self.transform_lines(ctx, |mut lines| {
            sort_lines(&mut lines, kind, desc);
            lines
        });
    }

    pub fn reverse_lines(&mut self, ctx: &egui::Context) {
        self.transform_lines(ctx, |mut lines| {
            lines.reverse();
            lines
        });
    }

    /// Removes repeated lines, keeping the first; with `consecutive`, only
    /// lines equal to the one before them.
    pub fn remove_duplicate_lines(&mut self, ctx: &egui::Context, consecutive: bool) {
        self.transform_lines(ctx, |lines| remove_duplicates(lines, consecutive));
    }

    /// Removes empty lines, and with `blank` also lines of only spaces and tabs.
    pub fn remove_empty_lines(&mut self, ctx: &egui::Context, blank: bool) {
        self.transform_lines(ctx, |lines| remove_empty(lines, blank));
    }

    pub fn trim_lines(&mut self, ctx: &egui::Context, kind: TrimKind) {
        self.transform_lines(ctx, |lines| {
            lines.iter().map(|l| trim_line(l, kind)).collect()
        });
    }

    /// Joins the selected lines (or all) with spaces, without trimming.
    pub fn eol_to_space(&mut self, ctx: &egui::Context) {
        self.transform_lines(ctx, |lines| vec![lines.join(" ")]);
    }

    pub fn tabs_to_spaces(&mut self, ctx: &egui::Context) {
        self.transform_lines(ctx, |lines| lines.iter().map(|l| expand_tabs(l)).collect());
    }

    pub fn spaces_to_tabs(&mut self, ctx: &egui::Context, all: bool) {
        self.transform_lines(ctx, |lines| {
            lines.iter().map(|l| spaces_to_tabs(l, all)).collect()
        });
    }

    /// Inserts an empty line above or below each caret line and moves the caret there.
    pub fn insert_blank_line(&mut self, ctx: &egui::Context, above: bool) {
        let edits = self
            .line_blocks()
            .into_iter()
            .map(|(first, last)| {
                if above {
                    let indent = leading_ws(&self.buffer.line_text(first)).to_owned();
                    let n = indent.chars().count();
                    let at = self.buffer.line_start(first);
                    BlockEdit {
                        range: at..at,
                        text: format!("{indent}\n"),
                        sels: vec![Selection::caret(n)],
                    }
                } else {
                    let indent = leading_ws(&self.buffer.line_text(last)).to_owned();
                    let text = format!("\n{indent}");
                    let n = text.chars().count();
                    let at = self.line_end(last);
                    BlockEdit {
                        range: at..at,
                        text,
                        sels: vec![Selection::caret(n)],
                    }
                }
            })
            .collect();
        self.apply_blocks(ctx, edits);
    }

    /// Converts the case of each selection, or of the word at a caret.
    pub fn convert_case(&mut self, ctx: &egui::Context, kind: CaseKind) {
        let mut edits = Vec::new();
        for s in self.selections.iter() {
            let range = if s.is_empty() {
                let (a, b) = self.word_range(s.head);
                if a == b || !self.buffer.rope().char(a).is_alphanumeric() {
                    continue;
                }
                a..b
            } else {
                s.range()
            };
            let old = self.buffer.slice_string(range.clone());
            let new = convert_case(&old, kind);
            let len = new.chars().count();
            let sel = if s.is_empty() {
                Selection::caret((s.head - range.start).min(len))
            } else if s.head >= s.anchor {
                Selection::new(0, len)
            } else {
                Selection::new(len, 0)
            };
            edits.push(BlockEdit {
                range,
                text: new,
                sels: vec![sel],
            });
        }
        self.apply_blocks(ctx, edits);
    }

    /// Comments or uncomments the selected lines with the language's line comment.
    /// Falls back to a block comment for languages that only have those.
    pub fn toggle_line_comment(&mut self, ctx: &egui::Context, tokens: &CommentTokens) {
        let Some(token) = tokens.line else {
            if tokens.block.is_some() {
                self.toggle_block_comment(ctx, tokens);
            }
            return;
        };
        let blocks = self.line_blocks();
        let all_commented = blocks.iter().all(|&(first, last)| {
            (first..=last).all(|l| {
                let t = self.buffer.line_text(l);
                t.trim().is_empty() || t.trim_start().starts_with(token)
            })
        }) && blocks.iter().any(|&(first, last)| {
            (first..=last).any(|l| !self.buffer.line_text(l).trim().is_empty())
        });
        let mut edits = Vec::new();
        for (first, last) in blocks {
            let old = self.lines_of(first, last);
            let indent = old
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| leading_ws(l).chars().count())
                .min()
                .unwrap_or(0);
            let new: Vec<String> = old
                .iter()
                .map(|l| {
                    if all_commented {
                        let ws = leading_ws(l);
                        let rest = &l[ws.len()..];
                        match rest.strip_prefix(token) {
                            Some(r) => format!("{ws}{}", r.strip_prefix(' ').unwrap_or(r)),
                            None => l.clone(),
                        }
                    } else if l.trim().is_empty() && old.len() > 1 {
                        l.clone()
                    } else {
                        let split = l.char_indices().nth(indent).map_or(l.len(), |(i, _)| i);
                        format!("{}{token} {}", &l[..split], &l[split..])
                    }
                })
                .collect();
            let range = self.lines_range(first, last);
            // Keep each caret on its text: shift it by what changed on its line.
            let sels = self
                .selections
                .iter()
                .filter(|s| s.start() >= range.start && s.end() <= range.end)
                .map(|s| {
                    let map = |p: usize| {
                        let line = self.buffer.char_to_line(p);
                        let col = p - self.buffer.line_start(line);
                        let i = line - first;
                        let before: usize = new[..i].iter().map(|l| l.chars().count() + 1).sum();
                        let old_len = old[i].chars().count();
                        let new_len = new[i].chars().count();
                        let col = if new_len >= old_len {
                            if col >= indent.min(old_len) && new_len != old_len {
                                col + new_len - old_len
                            } else {
                                col
                            }
                        } else {
                            col.saturating_sub(old_len - new_len)
                        };
                        before + col.min(new_len)
                    };
                    Selection::new(map(s.anchor), map(s.head))
                })
                .collect();
            edits.push(BlockEdit {
                range,
                text: new.join("\n"),
                sels,
            });
        }
        self.apply_blocks(ctx, edits);
    }

    /// Wraps each selection (or the caret line's text) in a block comment, or
    /// removes one that is already there.
    pub fn toggle_block_comment(&mut self, ctx: &egui::Context, tokens: &CommentTokens) {
        let Some((open, close)) = tokens.block else {
            self.toggle_line_comment(
                ctx,
                &CommentTokens {
                    line: tokens.line,
                    block: None,
                },
            );
            return;
        };
        let (on, cn) = (open.chars().count(), close.chars().count());
        let len = self.buffer.len_chars();
        let mut edits = Vec::new();
        for s in self.selections.iter() {
            let range = if s.is_empty() {
                let line = self.buffer.char_to_line(s.head);
                self.first_non_blank(line)..self.line_end(line)
            } else {
                s.range()
            };
            let text = self.buffer.slice_string(range.clone());
            let trimmed = text.trim();
            // Already commented: either the text itself or the chars around it.
            if trimmed.starts_with(open) && trimmed.ends_with(close) && trimmed.len() >= on + cn {
                let lead = text.len() - text.trim_start().len();
                let inner = &trimmed[open.len()..trimmed.len() - close.len()];
                let inner = inner.strip_prefix(' ').unwrap_or(inner);
                let inner = inner.strip_suffix(' ').unwrap_or(inner);
                let new = format!("{}{inner}{}", &text[..lead], &text[lead + trimmed.len()..]);
                let n = new.chars().count();
                edits.push(BlockEdit {
                    range,
                    text: new,
                    sels: vec![Selection::new(0, n)],
                });
                continue;
            }
            // The comment markers sit just outside the selection, with or without a space.
            let around = [(on + 1, cn + 1, " "), (on, cn, "")]
                .into_iter()
                .find(|&(b, a, sp)| {
                    range.start >= b
                        && range.end + a <= len
                        && self.buffer.slice_string(range.start - b..range.start)
                            == format!("{open}{sp}")
                        && self.buffer.slice_string(range.end..range.end + a)
                            == format!("{sp}{close}")
                });
            if let Some((b, a, _)) = around {
                let n = text.chars().count();
                edits.push(BlockEdit {
                    range: range.start - b..range.end + a,
                    text,
                    sels: vec![Selection::new(0, n)],
                });
                continue;
            }
            if text.is_empty() {
                continue;
            }
            let n = text.chars().count();
            edits.push(BlockEdit {
                range,
                text: format!("{open} {text} {close}"),
                sels: vec![Selection::new(on + 1, on + 1 + n)],
            });
        }
        self.apply_blocks(ctx, edits);
    }

    /// What a text tool works on: each selection, or the whole text when
    /// nothing is selected.
    pub fn tool_targets(&self) -> Vec<Range<usize>> {
        if self.selections.has_selection() {
            self.selections
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.range())
                .collect()
        } else {
            std::iter::once(0..self.buffer.len_chars()).collect()
        }
    }

    /// Replaces ranges with new text as one undo step. With `select`, each
    /// new text is selected; otherwise the caret keeps its offset (clamped).
    pub fn replace_ranges(
        &mut self,
        ctx: &egui::Context,
        edits: Vec<(Range<usize>, String)>,
        select: bool,
    ) {
        let caret = self.selections.primary().head;
        let blocks = edits
            .into_iter()
            .map(|(range, text)| {
                let len = text.chars().count();
                let sel = if select {
                    Selection::new(0, len)
                } else {
                    Selection::caret(caret.saturating_sub(range.start).min(len))
                };
                BlockEdit {
                    range,
                    text,
                    sels: vec![sel],
                }
            })
            .collect();
        self.apply_blocks(ctx, blocks);
    }

    /// Selects a range of chars and scrolls to it.
    pub fn select_range(&mut self, range: Range<usize>) {
        let len = self.buffer.len_chars();
        self.set_selections(Selections::single(Selection::new(
            range.start.min(len),
            range.end.min(len),
        )));
        self.reveal_carets();
    }
}

/// Expands tabs in the indentation (or everywhere with `all`) before tabs are
/// rebuilt, so mixed indentation comes out consistent.
fn expand_tabs_leading(line: &str, all: bool) -> String {
    if all {
        return expand_tabs(line);
    }
    let ws = leading_ws(line);
    format!("{}{}", expand_tabs(ws), &line[ws.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(text: &str, sels: Vec<Selection>) -> Editor {
        let mut e = Editor::new(text);
        e.selections = Selections::from_vec(sels, 0);
        e
    }

    fn text(e: &Editor) -> String {
        e.buffer.text()
    }

    #[test]
    fn duplicate_and_delete_lines() {
        let ctx = egui::Context::default();
        let mut e = ed("a\nb\nc", vec![Selection::caret(2)]);
        e.duplicate(&ctx);
        assert_eq!(text(&e), "a\nb\nb\nc");
        assert_eq!(e.selections.primary().head, 2);
        e.delete_lines(&ctx);
        assert_eq!(text(&e), "a\nb\nc");
        let mut e = ed("a\nb", vec![Selection::caret(3)]);
        e.delete_lines(&ctx);
        assert_eq!(text(&e), "a");
    }

    #[test]
    fn duplicate_selection_text() {
        let ctx = egui::Context::default();
        let mut e = ed("abc", vec![Selection::new(0, 2)]);
        e.duplicate(&ctx);
        assert_eq!(text(&e), "ababc");
        assert_eq!(e.selections.primary().range(), 0..2);
    }

    #[test]
    fn move_lines_up_and_down() {
        let ctx = egui::Context::default();
        let mut e = ed("one\ntwo\nthree", vec![Selection::caret(5)]);
        e.move_lines(&ctx, true);
        assert_eq!(text(&e), "two\none\nthree");
        assert_eq!(e.selections.primary().head, 1);
        e.move_lines(&ctx, true);
        assert_eq!(text(&e), "two\none\nthree");
        e.move_lines(&ctx, false);
        e.move_lines(&ctx, false);
        assert_eq!(text(&e), "one\nthree\ntwo");
        assert_eq!(e.selections.primary().head, 11);
        e.buffer.undo();
        assert_eq!(text(&e), "one\ntwo\nthree");
    }

    #[test]
    fn join_and_split() {
        let ctx = egui::Context::default();
        let mut e = ed("foo  \n   bar\nbaz", vec![Selection::new(0, 15)]);
        e.join_lines(&ctx);
        assert_eq!(text(&e), "foo bar baz");
        let mut e = ed("  one two three four", vec![Selection::caret(0)]);
        e.split_lines(&ctx, 12);
        assert_eq!(text(&e), "  one two\n  three four");
    }

    #[test]
    fn sorting() {
        let ctx = egui::Context::default();
        let mut e = ed("b\nA\na\nB", vec![Selection::caret(0)]);
        e.sort_lines(&ctx, SortKind::Lexical, false);
        assert_eq!(text(&e), "A\nB\na\nb");
        e.sort_lines(&ctx, SortKind::IgnoreCase, true);
        assert_eq!(text(&e), "b\nB\na\nA");
        let mut e = ed("10 x\n9 y\n-2\nnone\n1.5", vec![Selection::caret(0)]);
        e.sort_lines(&ctx, SortKind::Integer, false);
        assert_eq!(text(&e), "none\n-2\n1.5\n9 y\n10 x");
        e.sort_lines(&ctx, SortKind::Decimal, true);
        assert_eq!(text(&e), "10 x\n9 y\n1.5\n-2\nnone");
    }

    #[test]
    fn sort_only_the_selected_lines() {
        let ctx = egui::Context::default();
        let mut e = ed("z\nc\nb\na", vec![Selection::new(2, 6)]);
        e.sort_lines(&ctx, SortKind::Lexical, false);
        assert_eq!(text(&e), "z\nb\nc\na");
        assert_eq!(e.selections.primary().range(), 2..5);
    }

    #[test]
    fn duplicates_blanks_and_trim() {
        let ctx = egui::Context::default();
        let mut e = ed("a\nb\na\na\n\n  \nb", vec![Selection::caret(0)]);
        e.remove_duplicate_lines(&ctx, true);
        assert_eq!(text(&e), "a\nb\na\n\n  \nb");
        e.remove_empty_lines(&ctx, false);
        assert_eq!(text(&e), "a\nb\na\n  \nb");
        e.remove_empty_lines(&ctx, true);
        assert_eq!(text(&e), "a\nb\na\nb");
        e.remove_duplicate_lines(&ctx, false);
        assert_eq!(text(&e), "a\nb");
        let mut e = ed("  x  \n\ty\t", vec![Selection::caret(0)]);
        e.trim_lines(&ctx, TrimKind::Trailing);
        assert_eq!(text(&e), "  x\n\ty");
        e.trim_lines(&ctx, TrimKind::Leading);
        assert_eq!(text(&e), "x\ny");
    }

    #[test]
    fn tabs_and_spaces() {
        let ctx = egui::Context::default();
        let mut e = ed("\tx\ty\n  \tz", vec![Selection::caret(0)]);
        e.tabs_to_spaces(&ctx);
        assert_eq!(text(&e), "    x   y\n    z");
        e.spaces_to_tabs(&ctx, false);
        assert_eq!(text(&e), "\tx   y\n\tz");
        e.spaces_to_tabs(&ctx, true);
        assert_eq!(text(&e), "\tx\ty\n\tz");
    }

    #[test]
    fn case_conversion() {
        assert_eq!(convert_case("hello wORLD", CaseKind::Upper), "HELLO WORLD");
        assert_eq!(convert_case("don't stop", CaseKind::Title), "Don't Stop");
        assert_eq!(
            convert_case("one. TWO! three", CaseKind::Sentence),
            "One. Two! Three"
        );
        assert_eq!(convert_case("aBc", CaseKind::Invert), "AbC");
        assert_eq!(convert_case("user name", CaseKind::Camel), "userName");
        assert_eq!(
            convert_case("parseHTTPRequest2", CaseKind::Snake),
            "parse_http_request2"
        );
        assert_eq!(
            convert_case("  max-retry count\nfoo_bar", CaseKind::Constant),
            "  MAX_RETRY_COUNT\nFOO_BAR"
        );
        assert_eq!(
            convert_case("XMLHttpRequest", CaseKind::Kebab),
            "xml-http-request"
        );
        assert_eq!(convert_case("some_value", CaseKind::Pascal), "SomeValue");
        assert_eq!(convert_case("\n", CaseKind::Camel), "\n");
        let ctx = egui::Context::default();
        let mut e = ed("foo bar", vec![Selection::caret(5)]);
        e.convert_case(&ctx, CaseKind::Upper);
        assert_eq!(text(&e), "foo BAR");
        assert_eq!(e.selections.primary().head, 5);
    }

    #[test]
    fn line_comments_toggle() {
        let ctx = egui::Context::default();
        let tokens = comment_tokens("Rust");
        let mut e = ed("  let a;\n\n    let b;", vec![Selection::new(0, 20)]);
        e.toggle_line_comment(&ctx, &tokens);
        assert_eq!(text(&e), "  // let a;\n\n  //   let b;");
        e.toggle_line_comment(&ctx, &tokens);
        assert_eq!(text(&e), "  let a;\n\n    let b;");
        let mut e = ed("x = 1", vec![Selection::caret(2)]);
        e.toggle_line_comment(&ctx, &comment_tokens("Python"));
        assert_eq!(text(&e), "# x = 1");
        assert_eq!(e.selections.primary().head, 4);
    }

    #[test]
    fn block_comments_toggle() {
        let ctx = egui::Context::default();
        let tokens = comment_tokens("C");
        let mut e = ed("int a = 1;", vec![Selection::new(4, 9)]);
        e.toggle_block_comment(&ctx, &tokens);
        assert_eq!(text(&e), "int /* a = 1 */;");
        e.toggle_block_comment(&ctx, &tokens);
        assert_eq!(text(&e), "int a = 1;");
        let mut e = ed("<b>x</b>", vec![Selection::caret(1)]);
        e.toggle_line_comment(&ctx, &comment_tokens("HTML"));
        assert_eq!(text(&e), "<!-- <b>x</b> -->");
    }

    #[test]
    fn blank_lines_above_and_below() {
        let ctx = egui::Context::default();
        let mut e = ed("  a\nb", vec![Selection::caret(2)]);
        e.insert_blank_line(&ctx, false);
        assert_eq!(text(&e), "  a\n  \nb");
        assert_eq!(e.selections.primary().head, 6);
        e.insert_blank_line(&ctx, true);
        assert_eq!(text(&e), "  a\n  \n  \nb");
        assert_eq!(e.selections.primary().head, 6);
    }
}
