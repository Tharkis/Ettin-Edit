//! Carets and selections. Offsets are char indices into the rope.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Where the selection started; stays put while extending.
    pub anchor: usize,
    /// Where the caret is.
    pub head: usize,
    /// Display column to aim for when moving up and down through shorter lines.
    pub goal_col: Option<usize>,
}

impl Selection {
    pub fn caret(pos: usize) -> Self {
        Self {
            anchor: pos,
            head: pos,
            goal_col: None,
        }
    }

    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            goal_col: None,
        }
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn range(&self) -> Range<usize> {
        self.start()..self.end()
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Moves the caret; drags the anchor along unless `extend`.
    pub fn moved_to(self, head: usize, extend: bool) -> Self {
        Self {
            anchor: if extend { self.anchor } else { head },
            head,
            goal_col: None,
        }
    }
}

/// One or more selections, kept sorted and non-overlapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selections {
    ranges: Vec<Selection>,
    /// Index of the main caret (the one the view follows).
    primary: usize,
}

impl Selections {
    pub fn single(sel: Selection) -> Self {
        Self {
            ranges: vec![sel],
            primary: 0,
        }
    }

    pub fn caret(pos: usize) -> Self {
        Self::single(Selection::caret(pos))
    }

    pub fn from_vec(ranges: Vec<Selection>, primary: usize) -> Self {
        let mut s = Self {
            primary: primary.min(ranges.len().saturating_sub(1)),
            ranges,
        };
        s.normalize();
        s
    }

    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        self.ranges.iter()
    }

    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    pub fn primary(&self) -> Selection {
        self.ranges[self.primary]
    }

    pub fn primary_index(&self) -> usize {
        self.primary
    }

    pub fn has_selection(&self) -> bool {
        self.ranges.iter().any(|s| !s.is_empty())
    }

    /// Applies `f` to every selection, then re-sorts and merges.
    pub fn map(&mut self, mut f: impl FnMut(Selection) -> Selection) {
        for s in &mut self.ranges {
            *s = f(*s);
        }
        self.normalize();
    }

    pub fn push(&mut self, sel: Selection, make_primary: bool) {
        self.ranges.push(sel);
        if make_primary {
            self.primary = self.ranges.len() - 1;
        }
        self.normalize();
    }

    /// Keeps only the primary selection.
    pub fn collapse_to_primary(&mut self) {
        let p = self.primary();
        self.ranges = vec![p];
        self.primary = 0;
    }

    /// Clamps every selection to `0..=len`.
    pub fn clamp(&mut self, len: usize) {
        self.map(|s| Selection {
            anchor: s.anchor.min(len),
            head: s.head.min(len),
            goal_col: s.goal_col,
        });
    }

    fn normalize(&mut self) {
        if self.ranges.is_empty() {
            self.ranges.push(Selection::caret(0));
            self.primary = 0;
            return;
        }
        let primary = self.ranges[self.primary.min(self.ranges.len() - 1)];
        self.ranges.sort_by_key(|s| (s.start(), s.end()));
        let mut merged: Vec<Selection> = Vec::with_capacity(self.ranges.len());
        let mut new_primary = 0;
        for s in self.ranges.drain(..) {
            let is_primary = s == primary;
            if let Some(last) = merged.last_mut() {
                let same_caret = s.is_empty() && last.is_empty() && s.start() == last.start();
                if s.start() < last.end() || same_caret {
                    // Merge, keeping the direction of the earlier selection.
                    let start = last.start().min(s.start());
                    let end = last.end().max(s.end());
                    *last = if last.head >= last.anchor {
                        Selection::new(start, end)
                    } else {
                        Selection::new(end, start)
                    };
                    if is_primary {
                        new_primary = merged.len() - 1;
                    }
                    continue;
                }
            }
            if is_primary {
                new_primary = merged.len();
            }
            merged.push(s);
        }
        self.ranges = merged;
        self.primary = new_primary.min(self.ranges.len() - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_overlapping_and_duplicate_carets() {
        let s = Selections::from_vec(
            vec![
                Selection::new(5, 10),
                Selection::new(0, 2),
                Selection::new(8, 12),
                Selection::caret(1),
                Selection::caret(20),
                Selection::caret(20),
            ],
            4,
        );
        let ranges: Vec<_> = s.iter().map(|s| s.range()).collect();
        assert_eq!(ranges, vec![0..2, 5..12, 20..20]);
        assert_eq!(s.primary().range(), 20..20);
    }

    #[test]
    fn keeps_adjacent_selections_apart() {
        let s = Selections::from_vec(vec![Selection::new(0, 3), Selection::new(3, 6)], 0);
        assert_eq!(s.len(), 2);
    }
}
