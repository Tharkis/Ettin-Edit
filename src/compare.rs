//! Comparing two texts line by line: which lines were added, removed, changed
//! or moved, aligned into rows for showing side by side, plus the changed
//! words inside changed lines and a unified patch.

use std::collections::HashMap;
use std::ops::Range;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use similar::{Algorithm, ChangeTag, DiffOp, TextDiff};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompareOptions {
    /// Lines that differ only in spaces and tabs count as the same.
    #[serde(default)]
    pub ignore_whitespace: bool,
    #[serde(default)]
    pub ignore_case: bool,
    /// Empty lines added or removed don't count as differences.
    #[serde(default)]
    pub ignore_empty_lines: bool,
    /// Lines removed in one place and added in another show as moved.
    #[serde(default = "yes")]
    pub detect_moves: bool,
}

fn yes() -> bool {
    true
}

/// One of the [`CompareOptions`] switches, for menu commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOpt {
    IgnoreWhitespace,
    IgnoreCase,
    IgnoreEmptyLines,
    DetectMoves,
}

impl CompareOptions {
    pub fn get(&self, opt: CompareOpt) -> bool {
        match opt {
            CompareOpt::IgnoreWhitespace => self.ignore_whitespace,
            CompareOpt::IgnoreCase => self.ignore_case,
            CompareOpt::IgnoreEmptyLines => self.ignore_empty_lines,
            CompareOpt::DetectMoves => self.detect_moves,
        }
    }

    pub fn toggle(&mut self, opt: CompareOpt) {
        let flag = match opt {
            CompareOpt::IgnoreWhitespace => &mut self.ignore_whitespace,
            CompareOpt::IgnoreCase => &mut self.ignore_case,
            CompareOpt::IgnoreEmptyLines => &mut self.ignore_empty_lines,
            CompareOpt::DetectMoves => &mut self.detect_moves,
        };
        *flag = !*flag;
    }
}

impl Default for CompareOptions {
    fn default() -> Self {
        Self {
            ignore_whitespace: false,
            ignore_case: false,
            ignore_empty_lines: false,
            detect_moves: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// The same on both sides.
    Same,
    /// On both sides, with differences inside the line.
    Changed,
    /// Only on the right.
    Added,
    /// Only on the left.
    Removed,
    /// Only on one side, but ignored by the options (an empty line).
    Ignored,
}

/// One row of the side-by-side view: a line on the left, the right, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub kind: RowKind,
    /// An added or removed line that appears on the other side elsewhere.
    pub moved: bool,
}

impl Row {
    pub fn is_difference(&self) -> bool {
        matches!(
            self.kind,
            RowKind::Changed | RowKind::Added | RowKind::Removed
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub changed: usize,
    pub added: usize,
    pub removed: usize,
    pub moved: usize,
}

impl Summary {
    pub fn is_empty(&self) -> bool {
        *self == Summary::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Comparison {
    pub rows: Vec<Row>,
    /// Runs of rows that differ, in order.
    pub hunks: Vec<Range<usize>>,
    pub summary: Summary,
    /// The diff gave up looking for the smallest difference (very large or
    /// very different texts); the result is still correct, just coarser.
    pub timed_out: bool,
}

impl Comparison {
    /// The same comparison with the two sides swapped.
    pub fn mirrored(mut self) -> Self {
        for row in &mut self.rows {
            std::mem::swap(&mut row.left, &mut row.right);
            row.kind = match row.kind {
                RowKind::Added => RowKind::Removed,
                RowKind::Removed => RowKind::Added,
                kind => kind,
            };
        }
        let s = &mut self.summary;
        std::mem::swap(&mut s.added, &mut s.removed);
        self
    }
}

/// What a line is compared by, after applying the options.
fn line_key(line: &str, opts: &CompareOptions) -> String {
    let mut key: String = if opts.ignore_whitespace {
        line.chars().filter(|c| !c.is_whitespace()).collect()
    } else {
        line.trim_end_matches(['\r', '\n']).to_owned()
    };
    if opts.ignore_case {
        key = key.to_lowercase();
    }
    key
}

/// How alike two lines are, from 0 (nothing shared) to 1 (the same).
fn similarity(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let ratio = |a: &str, b: &str| {
        TextDiff::configure()
            .algorithm(Algorithm::Myers)
            .timeout(Duration::from_millis(20))
            .diff_chars(a, b)
            .ratio()
    };
    let r = ratio(a, b);
    if r >= 0.5 {
        return r;
    }
    // A change of case is still an edit of the same line.
    r.max(ratio(&a.to_lowercase(), &b.to_lowercase()))
}

/// Compares two texts given as lines.
pub fn compare(left: &[String], right: &[String], opts: &CompareOptions) -> Comparison {
    let lkeys: Vec<String> = left.iter().map(|l| line_key(l, opts)).collect();
    let rkeys: Vec<String> = right.iter().map(|l| line_key(l, opts)).collect();
    // Empty lines are left out of the diff when they are ignored, and put
    // back as rows of their own.
    let keep = |keys: &[String]| -> Vec<usize> {
        (0..keys.len())
            .filter(|&i| !(opts.ignore_empty_lines && keys[i].trim().is_empty()))
            .collect()
    };
    let lidx = keep(&lkeys);
    let ridx = keep(&rkeys);
    let lseq: Vec<&str> = lidx.iter().map(|&i| lkeys[i].as_str()).collect();
    let rseq: Vec<&str> = ridx.iter().map(|&i| rkeys[i].as_str()).collect();

    let deadline = Instant::now() + Duration::from_secs(2);
    let ops =
        similar::capture_diff_slices_deadline(Algorithm::Patience, &lseq, &rseq, Some(deadline));
    let timed_out = Instant::now() >= deadline;

    let mut rows = Vec::with_capacity(left.len().max(right.len()));
    // Next line on each side not yet given a row.
    let mut next_l = 0;
    let mut next_r = 0;
    // Adds rows for skipped (ignored) lines before line `l` and `r`.
    let flush =
        |rows: &mut Vec<Row>, next_l: &mut usize, next_r: &mut usize, l: usize, r: usize| {
            while *next_l < l {
                rows.push(Row {
                    left: Some(*next_l),
                    right: None,
                    kind: RowKind::Ignored,
                    moved: false,
                });
                *next_l += 1;
            }
            while *next_r < r {
                rows.push(Row {
                    left: None,
                    right: Some(*next_r),
                    kind: RowKind::Ignored,
                    moved: false,
                });
                *next_r += 1;
            }
        };
    let lline = |k: usize| lidx.get(k).copied().unwrap_or(left.len());
    let rline = |k: usize| ridx.get(k).copied().unwrap_or(right.len());

    for op in &ops {
        match *op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for k in 0..len {
                    let (l, r) = (lline(old_index + k), rline(new_index + k));
                    flush(&mut rows, &mut next_l, &mut next_r, l, r);
                    // Lines equal once the options are applied count as the same.
                    rows.push(Row {
                        left: Some(l),
                        right: Some(r),
                        kind: RowKind::Same,
                        moved: false,
                    });
                    next_l = l + 1;
                    next_r = r + 1;
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for k in 0..old_len {
                    let l = lline(old_index + k);
                    let r = next_r;
                    flush(&mut rows, &mut next_l, &mut next_r, l, r);
                    rows.push(Row {
                        left: Some(l),
                        right: None,
                        kind: RowKind::Removed,
                        moved: false,
                    });
                    next_l = l + 1;
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for k in 0..new_len {
                    let r = rline(new_index + k);
                    let l = next_l;
                    flush(&mut rows, &mut next_l, &mut next_r, l, r);
                    rows.push(Row {
                        left: None,
                        right: Some(r),
                        kind: RowKind::Added,
                        moved: false,
                    });
                    next_r = r + 1;
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let ls: Vec<usize> = (0..old_len).map(|k| lline(old_index + k)).collect();
                let rs: Vec<usize> = (0..new_len).map(|k| rline(new_index + k)).collect();
                for (l, r) in pair_lines(&ls, &rs, &lkeys, &rkeys) {
                    let (to_l, to_r) = (l.unwrap_or(next_l), r.unwrap_or(next_r));
                    flush(&mut rows, &mut next_l, &mut next_r, to_l, to_r);
                    let kind = match (l, r) {
                        (Some(_), Some(_)) => RowKind::Changed,
                        (Some(_), None) => RowKind::Removed,
                        _ => RowKind::Added,
                    };
                    rows.push(Row {
                        left: l,
                        right: r,
                        kind,
                        moved: false,
                    });
                    if let Some(l) = l {
                        next_l = l + 1;
                    }
                    if let Some(r) = r {
                        next_r = r + 1;
                    }
                }
            }
        }
    }
    flush(&mut rows, &mut next_l, &mut next_r, left.len(), right.len());

    if opts.detect_moves {
        mark_moves(&mut rows, &lkeys, &rkeys);
    }
    let hunks = hunks(&rows);
    let mut summary = Summary::default();
    for row in &rows {
        match (row.kind, row.moved) {
            (RowKind::Changed, _) => summary.changed += 1,
            (RowKind::Added, false) => summary.added += 1,
            (RowKind::Removed, false) => summary.removed += 1,
            (RowKind::Removed, true) => summary.moved += 1,
            _ => {}
        }
    }
    Comparison {
        rows,
        hunks,
        summary,
        timed_out,
    }
}

/// Lines a diff replaced (`ls` by `rs`): pairs the ones that look like edits
/// of each other, keeping their order; the rest are removed or added.
fn pair_lines(
    ls: &[usize],
    rs: &[usize],
    lkeys: &[String],
    rkeys: &[String],
) -> Vec<(Option<usize>, Option<usize>)> {
    let mut out = Vec::new();
    // Large blocks are paired in order, without looking for the best match.
    if ls.len() * rs.len() > 2_500 {
        for k in 0..ls.len().max(rs.len()) {
            out.push((ls.get(k).copied(), rs.get(k).copied()));
        }
        return out;
    }
    // Best order-keeping set of pairs, scored by how alike they are
    // (dynamic programming over the two lists).
    const MIN: f32 = 0.3;
    let (n, m) = (ls.len(), rs.len());
    let sim: Vec<Vec<f32>> = ls
        .iter()
        .map(|&l| {
            rs.iter()
                .map(|&r| similarity(&lkeys[l], &rkeys[r]))
                .collect()
        })
        .collect();
    let mut best = vec![vec![0.0f32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            let mut v = best[i + 1][j].max(best[i][j + 1]);
            if sim[i][j] >= MIN {
                v = v.max(sim[i][j] + best[i + 1][j + 1]);
            }
            best[i][j] = v;
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && sim[i][j] >= MIN && best[i][j] == sim[i][j] + best[i + 1][j + 1] {
            out.push((Some(ls[i]), Some(rs[j])));
            i += 1;
            j += 1;
        } else if i < n && (j == m || best[i][j] == best[i + 1][j]) {
            out.push((Some(ls[i]), None));
            i += 1;
        } else {
            out.push((None, Some(rs[j])));
            j += 1;
        }
    }
    out
}

/// Marks removed lines that were added somewhere else (and those added
/// lines) as moved. Short lines such as `}` are too common to count.
fn mark_moves(rows: &mut [Row], lkeys: &[String], rkeys: &[String]) {
    let mut added: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        if row.kind == RowKind::Added
            && let Some(r) = row.right
        {
            let key = rkeys[r].trim();
            if key.chars().count() >= 4 {
                added.entry(key).or_default().push(i);
            }
        }
    }
    let mut moved = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        if row.kind == RowKind::Removed
            && let Some(l) = row.left
            && let Some(list) = added.get_mut(lkeys[l].trim())
            && let Some(j) = list.pop()
        {
            moved.push(i);
            moved.push(j);
        }
    }
    let pairs: Vec<(usize, usize)> = moved.chunks(2).map(|p| (p[0], p[1])).collect();
    for &(i, j) in &pairs {
        rows[i].moved = true;
        rows[j].moved = true;
    }
    // A moved block carries its short lines (`}`, blank lines) along: grow
    // each moved pair over neighbours that match on both sides.
    let same = |rows: &[Row], i: usize, j: usize| {
        let (a, b) = (rows[i], rows[j]);
        a.kind == RowKind::Removed
            && b.kind == RowKind::Added
            && !a.moved
            && !b.moved
            && a.left
                .zip(b.right)
                .is_some_and(|(l, r)| lkeys[l].trim() == rkeys[r].trim())
    };
    for &(i, j) in &pairs {
        let (mut a, mut b) = (i + 1, j + 1);
        while a < rows.len() && b < rows.len() && same(rows, a, b) {
            rows[a].moved = true;
            rows[b].moved = true;
            a += 1;
            b += 1;
        }
        let (mut a, mut b) = (i, j);
        while a > 0 && b > 0 && same(rows, a - 1, b - 1) {
            a -= 1;
            b -= 1;
            rows[a].moved = true;
            rows[b].moved = true;
        }
    }
}

fn hunks(rows: &[Row]) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        if !row.is_difference() {
            continue;
        }
        match out.last_mut() {
            // Ignored rows between two differences don't split them.
            Some(h) if rows[h.end..i].iter().all(|r| r.kind == RowKind::Ignored) => h.end = i + 1,
            _ => out.push(i..i + 1),
        }
    }
    out
}

/// The changed parts of two versions of a line, as char ranges into each.
/// Whitespace-only parts are left out when whitespace is ignored.
pub fn inline_changes(
    a: &str,
    b: &str,
    opts: &CompareOptions,
) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let (la, lb);
    let (a2, b2) = if opts.ignore_case {
        la = a.to_lowercase();
        lb = b.to_lowercase();
        // Lowercasing can change the number of chars; then compare as typed.
        if la.chars().count() == a.chars().count() && lb.chars().count() == b.chars().count() {
            (la.as_str(), lb.as_str())
        } else {
            (a, b)
        }
    } else {
        (a, b)
    };
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .timeout(Duration::from_millis(50))
        .diff_unicode_words(a2, b2);
    let mut left = Vec::new();
    let mut right = Vec::new();
    let (mut pa, mut pb) = (0, 0);
    for change in diff.iter_all_changes() {
        let value = change.value();
        let len = value.chars().count();
        let skip = opts.ignore_whitespace && value.trim().is_empty();
        match change.tag() {
            ChangeTag::Equal => {
                pa += len;
                pb += len;
            }
            ChangeTag::Delete => {
                if !skip {
                    push_range(&mut left, pa..pa + len);
                }
                pa += len;
            }
            ChangeTag::Insert => {
                if !skip {
                    push_range(&mut right, pb..pb + len);
                }
                pb += len;
            }
        }
    }
    (left, right)
}

/// Adds a range, joining it to the last one when they touch.
fn push_range(ranges: &mut Vec<Range<usize>>, r: Range<usize>) {
    match ranges.last_mut() {
        Some(last) if last.end == r.start => last.end = r.end,
        _ => ranges.push(r),
    }
}

/// The difference as a unified patch (`diff -u` format), from `left` to `right`.
pub fn unified_patch(left: &str, right: &str, left_name: &str, right_name: &str) -> String {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .timeout(Duration::from_secs(5))
        .diff_lines(left, right);
    let mut patch = diff
        .unified_diff()
        .context_radius(3)
        .header(left_name, right_name)
        .to_string();
    if patch.is_empty() {
        patch = format!("--- {left_name}\n+++ {right_name}\n");
    }
    patch
}

/// Splits text into lines for [`compare`] (without their line breaks).
pub fn split_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_owned())
        .collect();
    // A final line break doesn't start another line.
    if lines.len() > 1 && lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        split_lines(s)
    }

    fn kinds(c: &Comparison) -> Vec<(Option<usize>, Option<usize>, RowKind, bool)> {
        c.rows
            .iter()
            .map(|r| (r.left, r.right, r.kind, r.moved))
            .collect()
    }

    #[test]
    fn mirrored_swaps_sides() {
        let opts = CompareOptions::default();
        let c = compare(&lines("a\nb\nc"), &lines("a\nc\nd"), &opts);
        let m = c.clone().mirrored();
        assert_eq!(m.summary.added, c.summary.removed);
        assert_eq!(m.summary.removed, c.summary.added);
        for (r, o) in m.rows.iter().zip(&c.rows) {
            assert_eq!((r.left, r.right), (o.right, o.left));
        }
        assert!(
            m.rows
                .iter()
                .any(|r| r.kind == RowKind::Removed && r.left == Some(2))
        );
    }

    #[test]
    fn aligns_added_removed_and_changed_lines() {
        let a = lines("one\ntwo\nthree\nfour\n");
        let b = lines("one\ntwo point five\nthree\nnew\nfour\n");
        let c = compare(&a, &b, &CompareOptions::default());
        use RowKind::*;
        assert_eq!(
            kinds(&c),
            vec![
                (Some(0), Some(0), Same, false),
                (Some(1), Some(1), Changed, false),
                (Some(2), Some(2), Same, false),
                (None, Some(3), Added, false),
                (Some(3), Some(4), Same, false),
            ]
        );
        assert_eq!(c.hunks, vec![1..2, 3..4]);
        assert_eq!(
            c.summary,
            Summary {
                changed: 1,
                added: 1,
                removed: 0,
                moved: 0
            }
        );
    }

    #[test]
    fn unlike_lines_are_removed_and_added() {
        let a = lines("keep\nalpha beta gamma\nkeep2");
        let b = lines("keep\n1234567890\nkeep2");
        let c = compare(&a, &b, &CompareOptions::default());
        assert_eq!(c.summary.removed, 1);
        assert_eq!(c.summary.added, 1);
        assert_eq!(c.summary.changed, 0);
    }

    #[test]
    fn options_ignore_case_whitespace_and_empty_lines() {
        let a = lines("Hello  World\n\nend");
        let b = lines("hello world\nend");
        let none = compare(&a, &b, &CompareOptions::default());
        assert!(!none.summary.is_empty());
        let opts = CompareOptions {
            ignore_whitespace: true,
            ignore_case: true,
            ignore_empty_lines: true,
            detect_moves: false,
        };
        let c = compare(&a, &b, &opts);
        assert!(c.summary.is_empty(), "{:?}", kinds(&c));
        assert!(c.rows.iter().any(|r| r.kind == RowKind::Ignored));
        assert!(c.hunks.is_empty());
    }

    #[test]
    fn moved_lines_are_detected() {
        let a = lines("fn first() {\n}\na\nb\nc\nd");
        let b = lines("a\nb\nc\nd\nfn first() {\n}");
        let c = compare(&a, &b, &CompareOptions::default());
        // The `}` moves with the line before it.
        assert_eq!(c.summary.moved, 2, "{:?}", kinds(&c));
        assert_eq!(c.summary.added, 0);
        assert_eq!(c.summary.removed, 0);
        assert_eq!(c.rows.iter().filter(|r| r.moved).count(), 4);
    }

    #[test]
    fn identical_texts_have_no_differences() {
        let a = lines("x\ny\n");
        let c = compare(&a, &a, &CompareOptions::default());
        assert!(c.summary.is_empty());
        assert_eq!(c.rows.len(), 2);
    }

    #[test]
    fn inline_changes_mark_the_changed_words() {
        let (l, r) = inline_changes(
            "let total = a + b;",
            "let sum = a + b;",
            &CompareOptions::default(),
        );
        assert_eq!(l, vec![4..9]);
        assert_eq!(r, vec![4..7]);
        let (l, r) = inline_changes(
            "a  b",
            "a b",
            &CompareOptions {
                ignore_whitespace: true,
                ..Default::default()
            },
        );
        assert!(l.is_empty() && r.is_empty());
    }

    #[test]
    fn unified_patch_has_headers_and_hunks() {
        let p = unified_patch("a\nb\nc\n", "a\nB\nc\n", "old.txt", "new.txt");
        assert!(
            p.starts_with("--- old.txt\n+++ new.txt\n@@ -1,3 +1,3 @@\n"),
            "{p}"
        );
        assert!(p.contains("-b\n+B\n"));
    }

    #[test]
    fn split_lines_drops_the_final_break() {
        assert_eq!(split_lines("a\r\nb\n"), vec!["a", "b"]);
        assert_eq!(split_lines(""), vec![""]);
        assert_eq!(split_lines("\n"), vec![""]);
    }
}
