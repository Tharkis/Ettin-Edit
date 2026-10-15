//! Code folding. Regions come from the language's grammar where it has
//! blocks (brackets, tags, keywords; see `fold_scan`), and otherwise from
//! indentation: a line folds when the lines after it are indented deeper,
//! and the region runs until the indentation comes back. Either way, a
//! closing bracket on its own line stays visible.

use std::collections::BTreeMap;

use super::Editor;
use super::buffer::LineEdit;
use super::layout::char_cols;

/// How many blank lines to look past when checking whether a line starts a fold.
const BLANK_LOOKAHEAD: usize = 1000;
/// How far above the view to look for the indentation regions start from.
const MARK_LOOKBEHIND: usize = 20_000;

/// What the fold column shows beside one line: a line from the region it is
/// in, the box of a region it starts, and the end tick of a region.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FoldMark {
    /// The line starts a region (a box).
    pub header: bool,
    /// The fold line comes in from above.
    pub above: bool,
    /// The fold line carries on below.
    pub below: bool,
    /// A region ends on this line (a tick to the right).
    pub end: bool,
}

impl Editor {
    /// Display width of a line's indentation, or `None` for a blank line.
    fn indent_cols(&self, line: usize) -> Option<usize> {
        let mut col = 0;
        for c in self.buffer.rope().line(line).chars() {
            match c {
                ' ' | '\t' => col += char_cols(c, col),
                '\n' | '\r' => return None,
                _ => return Some(col),
            }
        }
        None
    }

    fn next_non_blank(&self, line: usize, limit: usize) -> Option<(usize, usize)> {
        let n = self.buffer.len_lines();
        (line + 1..n.min(line + 1 + limit)).find_map(|l| self.indent_cols(l).map(|i| (l, i)))
    }

    /// Fold-column marks for `lines`, which must be in order and may skip
    /// lines hidden in folds.
    pub fn fold_marks(&self, lines: &[usize]) -> Vec<FoldMark> {
        if let Some(r) = self.fold_scan.regions() {
            return lines
                .iter()
                .map(|&l| {
                    let header = r.starts.contains_key(&l);
                    let next = self.folds.get(&l).map_or(l + 1, |&end| end + 1);
                    FoldMark {
                        header,
                        above: r.depth(l) > 0,
                        below: r.depth(next) > 0,
                        end: !header && r.ends_at(l),
                    }
                })
                .collect();
        }
        let Some(&first) = lines.first() else {
            return Vec::new();
        };
        // The shallowest indentation before the current line: a line is in a
        // region when something above it is indented less.
        let mut shallowest = usize::MAX;
        for l in (first.saturating_sub(MARK_LOOKBEHIND)..first).rev() {
            if let Some(i) = self.indent_cols(l) {
                shallowest = shallowest.min(i);
                if shallowest == 0 {
                    break;
                }
            }
        }
        let next_indent = |l: usize| self.next_non_blank(l, BLANK_LOOKAHEAD).map(|(_, i)| i);
        lines
            .iter()
            .map(|&l| {
                let own = self.indent_cols(l);
                let next = next_indent(l);
                let effective = own.or(next).unwrap_or(0);
                let inside = shallowest < effective;
                let mut mark = FoldMark {
                    above: inside,
                    ..FoldMark::default()
                };
                if let Some(own) = own {
                    let after = shallowest.min(own);
                    if let Some(&end) = self.folds.get(&l) {
                        mark.header = true;
                        mark.below = next_indent(end).is_some_and(|n| after < n);
                    } else if next.is_some_and(|n| n > own) {
                        mark.header = true;
                        mark.below = true;
                    } else if inside {
                        let n = next.unwrap_or(0);
                        mark.end = n < own;
                        mark.below = after < n;
                    }
                    shallowest = after;
                } else {
                    mark.below = inside;
                }
                mark
            })
            .collect()
    }

    /// True when `line` starts a foldable region.
    pub fn is_fold_start(&self, line: usize) -> bool {
        if let Some(r) = self.fold_scan.regions() {
            return r.starts.contains_key(&line);
        }
        let Some(own) = self.indent_cols(line) else {
            return false;
        };
        self.next_non_blank(line, BLANK_LOOKAHEAD)
            .is_some_and(|(_, i)| i > own)
    }

    /// The last line of the region `line` starts, if it starts one.
    pub fn fold_end(&self, line: usize) -> Option<usize> {
        if let Some(r) = self.fold_scan.regions() {
            return r.starts.get(&line).copied();
        }
        let own = self.indent_cols(line)?;
        let (first, indent) = self.next_non_blank(line, BLANK_LOOKAHEAD)?;
        if indent <= own {
            return None;
        }
        let mut last = first;
        for l in first + 1..self.buffer.len_lines() {
            match self.indent_cols(l) {
                Some(i) if i <= own => break,
                Some(_) => last = l,
                None => {}
            }
        }
        Some(last)
    }

    pub fn is_folded(&self, line: usize) -> bool {
        self.folds.contains_key(&line)
    }

    /// Shows or hides the lines folded away.
    pub(super) fn refresh_hidden(&mut self) {
        self.rows
            .set_hidden(self.folds.iter().map(|(&h, &end)| h + 1..end + 1));
    }

    /// Folds or unfolds the region starting at `line`. Returns false if there is none.
    pub fn toggle_fold(&mut self, line: usize) -> bool {
        self.sync_caches();
        if self.folds.remove(&line).is_some() {
            self.refresh_hidden();
            return true;
        }
        let Some(end) = self.fold_end(line) else {
            return false;
        };
        self.folds.insert(line, end);
        self.refresh_hidden();
        self.move_carets_out_of_folds();
        true
    }

    /// The innermost region around `line` (or starting at it).
    fn enclosing_fold(&self, line: usize) -> Option<usize> {
        if self.is_fold_start(line) {
            return Some(line);
        }
        if let Some(r) = self.fold_scan.regions() {
            return r
                .starts
                .range(..line)
                .rev()
                .find(|&(_, &end)| end >= line)
                .map(|(&h, _)| h);
        }
        let mut target = self.indent_cols(line);
        let mut l = line;
        // For a blank line, use the indentation of the next text line.
        if target.is_none() {
            target = self.next_non_blank(line, BLANK_LOOKAHEAD).map(|(_, i)| i);
        }
        let target = target?;
        while l > 0 {
            l -= 1;
            if let Some(i) = self.indent_cols(l)
                && i < target
            {
                return self.fold_end(l).filter(|&end| end >= line).map(|_| l);
            }
        }
        None
    }

    /// Folds (or unfolds) the region around the primary caret.
    pub fn fold_current(&mut self, fold: bool) {
        self.sync_caches();
        let line = self.buffer.char_to_line(self.selections.primary().head);
        if fold {
            if let Some(h) = self.enclosing_fold(line)
                && !self.folds.contains_key(&h)
                && let Some(end) = self.fold_end(h)
            {
                self.folds.insert(h, end);
                self.refresh_hidden();
                self.move_carets_out_of_folds();
            }
        } else {
            let before = self.folds.len();
            self.folds
                .retain(|&h, &mut end| !(h <= line && line <= end));
            if self.folds.len() != before {
                self.refresh_hidden();
            }
        }
    }

    /// Folds every region in the document, nested ones included.
    pub fn fold_all(&mut self) {
        self.sync_caches();
        self.fold_scan.finish_now(&self.buffer);
        self.folds = self.all_folds(None);
        self.refresh_hidden();
        self.move_carets_out_of_folds();
    }

    /// Folds the regions at nesting `level` (1 = outermost) and unfolds the rest.
    pub fn fold_level(&mut self, level: usize) {
        self.sync_caches();
        self.fold_scan.finish_now(&self.buffer);
        self.folds = self.all_folds(Some(level));
        self.refresh_hidden();
        self.move_carets_out_of_folds();
    }

    pub fn unfold_all(&mut self) {
        self.folds.clear();
        self.refresh_hidden();
    }

    /// Every fold region in one pass, optionally only those at one nesting level.
    fn all_folds(&self, level: Option<usize>) -> BTreeMap<usize, usize> {
        if let Some(r) = self.fold_scan.regions() {
            return r
                .starts
                .iter()
                .filter(|&(&h, _)| level.is_none_or(|lv| r.level(h) == lv))
                .map(|(&h, &e)| (h, e))
                .collect();
        }
        let mut out = BTreeMap::new();
        // Open regions: (header line, header indent, last text line so far).
        let mut stack: Vec<(usize, usize, usize)> = Vec::new();
        let mut prev: Option<(usize, usize)> = None;
        let n = self.buffer.len_lines();
        let mut close = |stack: &mut Vec<(usize, usize, usize)>, upto: Option<usize>| {
            while let Some(&(h, ind, last)) = stack.last() {
                if upto.is_some_and(|i| i > ind) {
                    break;
                }
                stack.pop();
                let depth = stack.len() + 1;
                if last > h && level.is_none_or(|lv| lv == depth) {
                    out.insert(h, last);
                }
            }
        };
        for l in 0..n {
            let Some(indent) = self.indent_cols(l) else {
                continue;
            };
            close(&mut stack, Some(indent));
            if let Some((pl, pi)) = prev
                && indent > pi
                && stack.last().is_none_or(|t| t.0 != pl)
            {
                stack.push((pl, pi, l));
            }
            for entry in stack.iter_mut() {
                entry.2 = l;
            }
            prev = Some((l, indent));
        }
        close(&mut stack, None);
        out
    }

    /// Keeps carets off hidden lines: one that lands in a fold goes to the end
    /// of the fold's first line.
    pub(super) fn move_carets_out_of_folds(&mut self) {
        if self.folds.is_empty() {
            return;
        }
        let mut sels = self.selections.clone();
        let mut moved = false;
        sels.map(|s| {
            let line = self.buffer.char_to_line(s.head);
            if !self.rows.is_hidden(line) {
                return s;
            }
            moved = true;
            let header = self.outer_header(line);
            super::selection::Selection::caret(self.line_end(header))
        });
        if moved {
            self.set_selections(sels);
        }
    }

    /// The first line of the outermost fold hiding `line`.
    fn outer_header(&self, line: usize) -> usize {
        self.folds
            .range(..line)
            .filter(|&(_, &end)| end >= line)
            .map(|(&h, _)| h)
            .next()
            .unwrap_or(line)
    }

    /// Unfolds every fold that hides a caret (after a jump or a search, say).
    pub(super) fn reveal_carets(&mut self) {
        if self.folds.is_empty() {
            return;
        }
        let lines: Vec<usize> = self
            .selections
            .iter()
            .map(|s| self.buffer.char_to_line(s.head))
            .filter(|&l| self.rows.is_hidden(l))
            .collect();
        if lines.is_empty() {
            return;
        }
        self.folds
            .retain(|&h, &mut end| !lines.iter().any(|&l| h < l && l <= end));
        self.refresh_hidden();
    }

    /// Where horizontal movement lands when `pos` is on a hidden line: past the
    /// fold going forward, or at the end of its first line going back.
    pub(super) fn skip_hidden(&self, pos: usize, forward: bool) -> usize {
        let line = self.buffer.char_to_line(pos);
        if !self.rows.is_hidden(line) {
            return pos;
        }
        let header = self.outer_header(line);
        if forward {
            let mut end = self.folds.get(&header).copied().unwrap_or(line);
            // An outer fold may reach further than this one.
            while let Some((_, &e)) = self
                .folds
                .range(..=end)
                .find(|&(&h, &e)| h <= end && e > end)
            {
                end = e;
            }
            if end + 1 < self.buffer.len_lines() {
                self.buffer.line_start(end + 1)
            } else {
                self.buffer.len_chars()
            }
        } else {
            self.line_end(header)
        }
    }

    /// Moves folds through line edits; an edit inside a fold unfolds it.
    pub(super) fn map_folds(&mut self, edits: &[LineEdit]) {
        if self.folds.is_empty() {
            return;
        }
        let mut folds: Vec<(usize, usize)> = self.folds.iter().map(|(&h, &e)| (h, e)).collect();
        for e in edits {
            let end = e.start_line + e.old_lines;
            let header_only = e.old_lines == 1 && e.new_lines == 1;
            folds.retain(|&(h, last)| {
                let touches = e.start_line <= last && end > h;
                !touches || (header_only && e.start_line == h)
            });
            for f in &mut folds {
                if f.0 >= end {
                    f.0 = f.0 + e.new_lines - e.old_lines;
                    f.1 = f.1 + e.new_lines - e.old_lines;
                }
            }
        }
        self.folds = folds.into_iter().collect();
        self.refresh_hidden();
    }
}

#[cfg(test)]
mod tests {
    use super::super::selection::Selections;
    use super::*;

    const CODE: &str = "fn a() {\n    if x {\n        y();\n    }\n\n    z();\n}\nfn b() {}";

    #[test]
    fn regions_follow_indentation() {
        let e = Editor::new(CODE);
        assert!(e.is_fold_start(0));
        assert_eq!(e.fold_end(0), Some(5));
        assert_eq!(e.fold_end(1), Some(2));
        assert!(!e.is_fold_start(2));
        assert!(!e.is_fold_start(7));
        assert_eq!(
            e.all_folds(None).into_iter().collect::<Vec<_>>(),
            vec![(0, 5), (1, 2)]
        );
        assert_eq!(
            e.all_folds(Some(2)).into_iter().collect::<Vec<_>>(),
            vec![(1, 2)]
        );
    }

    #[test]
    fn fold_column_marks() {
        let mut e = Editor::new(CODE);
        let m = |header, above, below, end| FoldMark {
            header,
            above,
            below,
            end,
        };
        let lines: Vec<usize> = (0..8).collect();
        assert_eq!(
            e.fold_marks(&lines),
            vec![
                m(true, false, true, false),   // fn a() {
                m(true, true, true, false),    //     if x {
                m(false, true, true, true),    //         y();  (inner region ends)
                m(false, true, true, false),   //     }
                m(false, true, true, false),   // (blank)
                m(false, true, false, true),   //     z();  (outer region ends)
                m(false, false, false, false), // }
                m(false, false, false, false), // fn b() {}
            ]
        );
        // Folded: the hidden lines are skipped and the line still joins below.
        e.toggle_fold(1);
        assert_eq!(
            e.fold_marks(&[1, 3, 4]),
            vec![
                m(true, true, true, false),
                m(false, true, true, false),
                m(false, true, true, false),
            ]
        );
        // Starting part-way down still knows it is inside a region.
        assert_eq!(e.fold_marks(&[5]), vec![m(false, true, false, true)]);
    }

    #[test]
    fn folding_hides_rows_and_moves_carets() {
        let mut e = Editor::new(CODE);
        e.selections = Selections::caret(e.buffer.line_start(2) + 3);
        assert!(e.toggle_fold(0));
        assert_eq!(e.rows.total_rows(), 3);
        // The caret was inside, so it moved to the end of the first line.
        assert_eq!(e.selections.primary().head, 8);
        assert_eq!(e.skip_hidden(9, true), e.buffer.line_start(6));
        e.unfold_all();
        assert_eq!(e.rows.total_rows(), 8);
    }

    #[test]
    fn edits_move_or_open_folds() {
        let ctx = egui::Context::default();
        let mut e = Editor::new(CODE);
        e.toggle_fold(1);
        e.selections = Selections::caret(0);
        e.insert_text(&ctx, "\n");
        e.sync_caches();
        assert!(e.is_folded(2));
        // Typing inside the folded region opens it.
        e.selections = Selections::caret(e.buffer.line_start(3));
        e.insert_text(&ctx, "x");
        e.sync_caches();
        assert!(e.folds.is_empty());
    }
}
