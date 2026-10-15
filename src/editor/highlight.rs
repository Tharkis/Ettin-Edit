//! Incremental syntax highlighting.
//!
//! The parser state is saved every [`CHECKPOINT_EVERY`] lines, so an edit only
//! re-parses from the checkpoint before it, and jumping into a big file parses
//! forward a slice at a time instead of freezing the window.

use std::collections::HashMap;
use std::sync::Arc;

use egui::Color32;
use syntect::highlighting::{
    FontStyle, HighlightState, Highlighter, RangedHighlightIterator, Theme,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

use super::buffer::Buffer;

const CHECKPOINT_EVERY: usize = 64;
/// Lines longer than this are shown unstyled; parsing them would stall typing.
const MAX_LINE_BYTES: usize = 20_000;
const MAX_CACHED_LINES: usize = 20_000;
/// When a line is far past the parsed region, parsing restarts this many lines
/// above it from a clean state. Nearly every grammar has re-synced by then.
const RESYNC_LINES: usize = 150;
/// Lines highlighted ahead of the requested line in that case, to cover the screen.
const RESYNC_AHEAD: usize = 200;

#[derive(Clone, Copy, Debug)]
pub struct Span {
    /// Byte range within the line (without its newline).
    pub start: u32,
    pub end: u32,
    pub color: Color32,
    pub italic: bool,
    pub underline: bool,
}

pub type LineSpans = Arc<Vec<Span>>;

#[derive(Clone)]
struct Checkpoint {
    parse: ParseState,
    highlight: HighlightState,
}

pub struct LineHighlighter {
    key: (String, String),
    plain: bool,
    /// `checkpoints[k]` is the state at the start of line `k * CHECKPOINT_EVERY`.
    checkpoints: Vec<Checkpoint>,
    spans: HashMap<usize, LineSpans>,
}

impl Default for LineHighlighter {
    fn default() -> Self {
        Self {
            key: (String::new(), String::new()),
            plain: true,
            checkpoints: Vec::new(),
            spans: HashMap::new(),
        }
    }
}

impl LineHighlighter {
    /// Resets everything when the language or theme changes.
    pub fn configure(
        &mut self,
        syntax: &SyntaxReference,
        theme_name: &str,
        theme: &Theme,
        plain: bool,
    ) {
        let key = (syntax.name.clone(), theme_name.to_owned());
        if key != self.key || plain != self.plain {
            self.key = key;
            self.plain = plain;
            self.spans.clear();
            self.checkpoints.clear();
            let highlighter = Highlighter::new(theme);
            self.checkpoints.push(Checkpoint {
                parse: ParseState::new(syntax),
                highlight: HighlightState::new(&highlighter, ScopeStack::new()),
            });
        }
    }

    /// True for plain text, which has no colours.
    pub fn is_plain(&self) -> bool {
        self.plain
    }

    fn syntax_ref<'s>(&self, syntaxes: &'s SyntaxSet) -> &'s SyntaxReference {
        syntaxes
            .find_syntax_by_name(&self.key.0)
            .unwrap_or_else(|| syntaxes.find_syntax_plain_text())
    }

    /// Forgets everything from `line` on.
    pub fn invalidate_from(&mut self, line: usize) {
        let keep = line / CHECKPOINT_EVERY + 1;
        self.checkpoints.truncate(keep.max(1));
        self.spans.retain(|&l, _| l < line);
    }

    /// Styled spans for `line`, or `None` if reaching it would take more than
    /// `budget` lines of parsing (the caller shows it plain and asks for another frame).
    pub fn line_spans(
        &mut self,
        line: usize,
        buffer: &Buffer,
        syntaxes: &SyntaxSet,
        theme: &Theme,
        budget: &mut usize,
    ) -> Option<LineSpans> {
        if self.plain {
            return None;
        }
        if let Some(s) = self.spans.get(&line) {
            return Some(s.clone());
        }
        let wanted_checkpoint = line / CHECKPOINT_EVERY;
        let have = self.checkpoints.len() - 1;
        let from_checkpoint = wanted_checkpoint.min(have);
        let start_line = from_checkpoint * CHECKPOINT_EVERY;
        let distance = line - start_line;
        if distance > RESYNC_LINES * 2 {
            // Far from any saved state (e.g. after jumping to the end of a big
            // file): highlight from a clean state a little above instead of
            // parsing everything in between.
            let from = line.saturating_sub(RESYNC_LINES);
            let to = line + RESYNC_AHEAD;
            *budget = budget.saturating_sub(to - from);
            self.parse_range(from, to, None, buffer, syntaxes, theme);
            return self.spans.get(&line).cloned();
        }
        if distance + 1 > *budget {
            // Out of time this frame: show it plain and finish next frame.
            *budget = 0;
            return None;
        }
        *budget -= distance + 1;
        self.parse_range(
            start_line,
            line + 1,
            Some(from_checkpoint),
            buffer,
            syntaxes,
            theme,
        );
        self.spans.get(&line).cloned()
    }

    fn parse_range(
        &mut self,
        from: usize,
        to: usize,
        checkpoint: Option<usize>,
        buffer: &Buffer,
        syntaxes: &SyntaxSet,
        theme: &Theme,
    ) {
        if self.spans.len() > MAX_CACHED_LINES {
            self.spans.clear();
        }
        let highlighter = Highlighter::new(theme);
        // With no checkpoint, start fresh and record no checkpoints (the state is approximate).
        let mut state = match checkpoint {
            Some(k) => self.checkpoints[k].clone(),
            None => Checkpoint {
                parse: ParseState::new(self.syntax_ref(syntaxes)),
                highlight: HighlightState::new(&highlighter, ScopeStack::new()),
            },
        };
        let to = to.min(buffer.len_lines());
        let mut text = String::new();
        for line in from..to {
            if checkpoint.is_some()
                && line % CHECKPOINT_EVERY == 0
                && line / CHECKPOINT_EVERY == self.checkpoints.len()
            {
                self.checkpoints.push(state.clone());
            }
            text.clear();
            for chunk in buffer.rope().line(line).chunks() {
                text.push_str(chunk);
            }
            if !text.ends_with('\n') {
                text.push('\n');
            }
            let content_len = text.len() - 1;
            if content_len > MAX_LINE_BYTES {
                self.spans.insert(line, Arc::new(Vec::new()));
                continue;
            }
            let ops = match state.parse.parse_line(&text, syntaxes) {
                Ok(ops) => ops,
                Err(_) => {
                    self.spans.insert(line, Arc::new(Vec::new()));
                    continue;
                }
            };
            let mut spans = Vec::new();
            for (style, _, range) in
                RangedHighlightIterator::new(&mut state.highlight, &ops, &text, &highlighter)
            {
                let end = range.end.min(content_len);
                if range.start >= end {
                    continue;
                }
                let fg = style.foreground;
                spans.push(Span {
                    start: range.start as u32,
                    end: end as u32,
                    color: Color32::from_rgba_unmultiplied(fg.r, fg.g, fg.b, fg.a),
                    italic: style.font_style.contains(FontStyle::ITALIC),
                    underline: style.font_style.contains(FontStyle::UNDERLINE),
                });
            }
            self.spans.insert(line, Arc::new(spans));
        }
    }
}
