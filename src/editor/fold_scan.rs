//! Grammar-aware folding: fold regions found by running the document's
//! grammar over it, so blocks follow brackets, tags, `begin … end` style
//! keywords, LaTeX environments, comments and `#region` markers instead of
//! indentation. Brackets inside strings and comments don't count.
//!
//! Pairing openers with closers needs the whole document, so the scan runs on
//! a background thread over a snapshot of the text (a rope clone is cheap).
//! Until a scan finishes, the previous regions move with edits.
//!
//! Languages where indentation is the structure (Python, YAML, plain text)
//! keep folding by indentation, as does any document the grammar finds no
//! blocks in.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use ropey::Rope;
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};

use super::buffer::{Buffer, LineEdit};

/// Bigger documents fold by indentation; a full grammar pass would take too long.
const MAX_SCAN_LINES: usize = 300_000;
const MAX_SCAN_CHARS: usize = 30_000_000;
/// Longer lines are skipped (the highlighter shows them plain too).
const MAX_LINE_BYTES: usize = 20_000;
/// While typing in a document that takes a while to scan, wait this long
/// after the last edit before scanning again.
const SCAN_DELAY: Duration = Duration::from_millis(300);
/// Scans quicker than this run after every edit.
const QUICK_SCAN: Duration = Duration::from_millis(25);

/// How a language folds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Style {
    Indent,
    Grammar(Rules),
    /// Headings and fenced code blocks.
    Markdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rules {
    brackets: Brackets,
    words: Option<&'static Words>,
    /// `[section]` headers fold down to the next one (INI, TOML).
    sections: bool,
    /// HTML's empty elements (`<br>`, `<img>`) never close.
    html: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Brackets {
    All,
    /// Only inside embedded code, such as `<script>` in HTML.
    Embedded,
    None,
}

/// Keywords that open and close blocks, for languages that end blocks with words.
#[derive(Debug, PartialEq, Eq)]
struct Words {
    open: &'static [&'static str],
    close: &'static [&'static str],
    ignore_case: bool,
    /// Openers that only open when they start a statement (Ruby's `x = 1 if y`
    /// doesn't), or follow an `=`.
    statement_only: &'static [&'static str],
    /// After one of these on the same line, `do` belongs to the loop.
    loop_heads: &'static [&'static str],
}

static RUBY: Words = Words {
    open: &[
        "def", "class", "module", "if", "unless", "while", "until", "case", "begin", "do", "for",
    ],
    close: &["end"],
    ignore_case: false,
    statement_only: &["if", "unless", "while", "until"],
    loop_heads: &["while", "until", "for"],
};
static LUA: Words = Words {
    open: &["function", "if", "do", "repeat"],
    close: &["end", "until"],
    ignore_case: false,
    statement_only: &[],
    loop_heads: &[],
};
static SHELL: Words = Words {
    open: &["if", "case", "do"],
    close: &["fi", "esac", "done"],
    ignore_case: false,
    statement_only: &[],
    loop_heads: &[],
};
static PASCAL: Words = Words {
    open: &["begin", "case", "record", "try", "repeat", "asm"],
    close: &["end", "until"],
    ignore_case: true,
    statement_only: &[],
    loop_heads: &[],
};
static CMAKE: Words = Words {
    open: &["if", "foreach", "while", "function", "macro", "block"],
    close: &[
        "endif",
        "endforeach",
        "endwhile",
        "endfunction",
        "endmacro",
        "endblock",
    ],
    ignore_case: true,
    statement_only: &[],
    loop_heads: &[],
};

/// Picks how a grammar's documents fold.
pub(super) fn style_for(syntax: &SyntaxReference) -> Style {
    let scope = syntax.scope.build_string();
    let starts = |p: &str| scope == p || scope.starts_with(&format!("{p}."));
    const INDENTED: &[&str] = &[
        "text.plain",
        "source.python",
        "source.yaml",
        "source.haskell",
        "source.coffee",
        "source.makefile",
        "text.restructuredtext",
        "source.sass",
        "source.nim",
    ];
    if syntax.name == crate::highlight::PLAIN_TEXT || INDENTED.iter().any(|p| starts(p)) {
        return Style::Indent;
    }
    if starts("text.html.markdown") {
        return Style::Markdown;
    }
    let mut rules = Rules {
        brackets: Brackets::All,
        words: None,
        sections: false,
        html: false,
    };
    if starts("source.ini") {
        rules.sections = true;
        rules.brackets = Brackets::None;
    } else if starts("source.toml") {
        rules.sections = true;
    } else if starts("source.ruby") {
        rules.words = Some(&RUBY);
    } else if starts("source.lua") {
        rules.words = Some(&LUA);
    } else if starts("source.shell") {
        rules.words = Some(&SHELL);
    } else if starts("source.pascal") {
        rules.words = Some(&PASCAL);
    } else if starts("source.cmake") {
        rules.words = Some(&CMAKE);
    } else if scope.starts_with("text.") && !starts("text.tex") {
        // Markup: tags fold; brackets in the text between them mean nothing.
        rules.brackets = Brackets::Embedded;
        rules.html = scope.starts_with("text.html");
    }
    Style::Grammar(rules)
}

/// Fold regions of a whole document: first line → last hidden line, with
/// how deeply each line is nested.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldRegions {
    pub starts: BTreeMap<usize, usize>,
    /// `depth[l]`: regions that hide line `l` (start above it, end at or below it).
    depth: Vec<u32>,
    ends: BTreeSet<usize>,
}

impl FoldRegions {
    fn new(regions: impl IntoIterator<Item = (usize, usize)>, lines: usize) -> Self {
        let mut starts: BTreeMap<usize, usize> = BTreeMap::new();
        for (h, e) in regions {
            let e = e.min(lines.saturating_sub(1));
            if e > h {
                let end = starts.entry(h).or_insert(e);
                *end = (*end).max(e);
            }
        }
        let mut r = Self {
            starts,
            ..Self::default()
        };
        r.rebuild(lines);
        r
    }

    fn rebuild(&mut self, lines: usize) {
        let mut diff = vec![0i64; lines + 2];
        for (&h, &e) in &self.starts {
            diff[h + 1] += 1;
            diff[(e + 1).min(lines + 1)] -= 1;
        }
        let mut depth = 0i64;
        self.depth = diff
            .iter()
            .map(|d| {
                depth += d;
                depth.max(0) as u32
            })
            .collect();
        self.ends = self.starts.values().copied().collect();
    }

    pub fn depth(&self, line: usize) -> u32 {
        self.depth.get(line).copied().unwrap_or(0)
    }

    pub fn ends_at(&self, line: usize) -> bool {
        self.ends.contains(&line)
    }

    /// Nesting level of the region starting at `line` (1 = outermost).
    pub fn level(&self, line: usize) -> usize {
        self.depth(line) as usize + 1
    }

    /// Moves the regions through line edits (an approximation until the next scan).
    fn map_edits(&mut self, edits: &[LineEdit], lines: usize) {
        let mut regions: Vec<(usize, usize)> = self.starts.iter().map(|(&h, &e)| (h, e)).collect();
        for e in edits {
            let end = e.start_line + e.old_lines;
            let delta = e.new_lines as isize - e.old_lines as isize;
            let shift = |x: usize| (x as isize + delta).max(0) as usize;
            for r in &mut regions {
                if r.0 >= end {
                    *r = (shift(r.0), shift(r.1));
                } else if r.1 >= e.start_line {
                    r.1 = shift(r.1.max(end.saturating_sub(1)));
                }
            }
        }
        *self = Self::new(regions, lines);
    }
}

/// What a finished scan sends back: the regions, how long it took, and the
/// state to resume from next time.
type ScanResult = (Option<FoldRegions>, Duration, Option<Arc<ScanCache>>);

struct Job {
    version: u64,
    cancel: Arc<AtomicBool>,
    rx: mpsc::Receiver<ScanResult>,
    /// The first line edited since the job started.
    dirty: usize,
}

/// Keeps a document's grammar regions up to date in the background.
#[derive(Default)]
pub(super) struct FoldScanner {
    key: Option<(String, usize)>,
    syntaxes: Option<Arc<SyntaxSet>>,
    style: Option<Style>,
    /// Grammar regions; `None` folds by indentation.
    regions: Option<Arc<FoldRegions>>,
    /// The buffer version the regions match (edits since a scan are mapped in).
    found_at: Option<u64>,
    job: Option<Job>,
    /// The last scan's saved state, and the first line edited since.
    cache: Option<(Arc<ScanCache>, usize)>,
    last_edit: Option<Instant>,
    last_cost: Duration,
}

impl FoldScanner {
    /// Sets the language; a different one forgets the regions.
    pub fn configure(&mut self, syntax: &SyntaxReference, syntaxes: &Arc<SyntaxSet>, plain: bool) {
        let key = (syntax.name.clone(), Arc::as_ptr(syntaxes) as usize);
        if self.key.as_ref() == Some(&key) {
            return;
        }
        self.reset();
        self.key = Some(key);
        self.syntaxes = Some(syntaxes.clone());
        self.style = Some(if plain {
            Style::Indent
        } else {
            style_for(syntax)
        });
    }

    /// Forgets the regions (after the whole text was replaced).
    pub fn reset(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.regions = None;
        self.found_at = None;
        self.cache = None;
    }

    pub fn regions(&self) -> Option<&FoldRegions> {
        self.regions.as_deref()
    }

    fn scans(&self) -> bool {
        self.style.is_some_and(|s| s != Style::Indent)
    }

    /// Moves the regions through edits until the next scan.
    pub fn edited(&mut self, edits: &[LineEdit], lines: usize) {
        self.last_edit = Some(Instant::now());
        if let Some(first) = edits.iter().map(|e| e.start_line).min() {
            if let Some((_, dirty)) = &mut self.cache {
                *dirty = (*dirty).min(first);
            }
            if let Some(job) = &mut self.job {
                job.dirty = job.dirty.min(first);
            }
        }
        if let Some(r) = &mut self.regions {
            Arc::make_mut(r).map_edits(edits, lines);
        }
    }

    /// Picks up a finished scan and starts the next one when the text changed.
    /// Call once per frame.
    pub fn poll(&mut self, buffer: &Buffer, ctx: &egui::Context) {
        if !self.scans() {
            return;
        }
        let version = buffer.version();
        if let Some(job) = &self.job {
            match job.rx.try_recv() {
                Ok((regions, cost, cache)) => {
                    let job_version = job.version;
                    // Still a good starting point when the text changed meanwhile.
                    self.cache = cache.map(|c| (c, job.dirty));
                    self.job = None;
                    self.last_cost = cost;
                    if job_version == version {
                        self.regions = regions.map(Arc::new);
                        self.found_at = Some(version);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    if job.version == version {
                        return;
                    }
                    job.cancel.store(true, Ordering::Relaxed);
                    self.job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.job = None,
            }
        }
        if self.found_at == Some(version) || self.job.is_some() {
            return;
        }
        if self.found_at.is_some() && self.last_cost > QUICK_SCAN {
            let since = self.last_edit.map_or(SCAN_DELAY, |t| t.elapsed());
            if since < SCAN_DELAY {
                ctx.request_repaint_after(SCAN_DELAY - since);
                return;
            }
        }
        self.start(buffer, Some(ctx.clone()));
    }

    fn start(&mut self, buffer: &Buffer, ctx: Option<egui::Context>) {
        let (Some(style), Some(syntaxes), Some((name, _))) =
            (self.style, self.syntaxes.clone(), self.key.clone())
        else {
            return;
        };
        let version = buffer.version();
        if buffer.len_lines() > MAX_SCAN_LINES || buffer.len_chars() > MAX_SCAN_CHARS {
            self.regions = None;
            self.found_at = Some(version);
            return;
        }
        let rope = buffer.rope().clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let flag = cancel.clone();
        let resume = self.cache.clone();
        std::thread::spawn(move || {
            let t = Instant::now();
            let Some(syntax) = syntaxes.find_syntax_by_name(&name) else {
                return;
            };
            let resume = resume.as_ref().map(|(c, dirty)| (c.as_ref(), *dirty));
            let (regions, cache) = scan_resuming(&rope, &syntaxes, syntax, style, &flag, resume);
            if flag.load(Ordering::Relaxed) {
                return;
            }
            let _ = tx.send((regions, t.elapsed(), cache.map(Arc::new)));
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
        self.job = Some(Job {
            version,
            cancel,
            rx,
            dirty: usize::MAX,
        });
    }

    /// Brings the regions up to date now, waiting for the scan if needed
    /// (for Fold All and the other whole-document commands).
    pub fn finish_now(&mut self, buffer: &Buffer) {
        if !self.scans() || self.found_at == Some(buffer.version()) {
            return;
        }
        if self
            .job
            .as_ref()
            .is_none_or(|j| j.version != buffer.version())
        {
            if let Some(job) = self.job.take() {
                job.cancel.store(true, Ordering::Relaxed);
            }
            self.start(buffer, None);
        }
        if let Some(job) = self.job.take() {
            if let Ok((regions, cost, cache)) = job.rx.recv() {
                self.last_cost = cost;
                self.regions = regions.map(Arc::new);
                self.cache = cache.map(|c| (c, job.dirty));
            }
            self.found_at = Some(buffer.version());
        }
    }
}

// ---------------------------------------------------------------------------
// Scanning

/// The fold regions of `text`, or `None` to fold it by indentation.
#[cfg(test)]
pub(super) fn scan(
    text: &Rope,
    syntaxes: &SyntaxSet,
    syntax: &SyntaxReference,
    style: Style,
    cancel: &AtomicBool,
) -> Option<FoldRegions> {
    scan_resuming(text, syntaxes, syntax, style, cancel, None).0
}

/// Like [`scan`], resuming from an earlier scan's state where the text before
/// `dirty` (the first edited line) is unchanged. Also returns the state to
/// resume from next time.
fn scan_resuming(
    text: &Rope,
    syntaxes: &SyntaxSet,
    syntax: &SyntaxReference,
    style: Style,
    cancel: &AtomicBool,
    resume: Option<(&ScanCache, usize)>,
) -> (Option<FoldRegions>, Option<ScanCache>) {
    let (regions, cache) = match style {
        Style::Indent => return (None, None),
        Style::Markdown => (markdown(text), None),
        Style::Grammar(rules) => {
            match GrammarScan::new(rules).run(text, syntaxes, syntax, cancel, resume) {
                Some((regions, cache)) => (regions, Some(cache)),
                None => return (None, None),
            }
        }
    };
    let regions = FoldRegions::new(regions, text.len_lines());
    ((!regions.starts.is_empty()).then_some(regions), cache)
}

/// How often the scan saves its state, in lines.
const CHECKPOINT_LINES: usize = 128;

/// The scan's state at the start of a line.
#[derive(Clone)]
struct Checkpoint {
    line: usize,
    parse: ParseState,
    scopes: ScopeStack,
    stack: Vec<Open>,
    tag: Option<Pending>,
    env: Option<Pending>,
    markers: Vec<usize>,
    /// How many regions had been found by then.
    regions: usize,
}

/// What a scan leaves behind so the next one can skip the unchanged start.
pub(super) struct ScanCache {
    checkpoints: Vec<Checkpoint>,
    regions: Vec<(usize, usize)>,
    comment_only: Vec<bool>,
    marker: Vec<bool>,
    blank: Vec<bool>,
    sections: Vec<usize>,
}

fn line_text(text: &Rope, line: usize, buf: &mut String) {
    buf.clear();
    for chunk in text.line(line).chunks() {
        buf.push_str(chunk);
    }
    while buf.ends_with('\n') || buf.ends_with('\r') {
        buf.pop();
    }
}

/// The last non-blank line in `from..to`, if any.
fn last_text_line(blank: &[bool], from: usize, to: usize) -> Option<usize> {
    (from..to.min(blank.len())).rev().find(|&l| !blank[l])
}

/// Markdown: a heading folds down to the next heading of the same or a higher
/// level; fenced code blocks fold too.
fn markdown(text: &Rope) -> Vec<(usize, usize)> {
    let n = text.len_lines();
    let mut regions = Vec::new();
    let mut headings: Vec<(usize, usize)> = Vec::new();
    let mut blank = vec![false; n];
    let mut fence: Option<(char, usize, usize)> = None;
    let mut buf = String::new();
    for (l, is_blank) in blank.iter_mut().enumerate() {
        line_text(text, l, &mut buf);
        let t = buf.trim_start();
        *is_blank = t.is_empty();
        let fence_char = t.chars().next().filter(|c| *c == '`' || *c == '~');
        let run = fence_char.map_or(0, |c| t.chars().take_while(|&x| x == c).count());
        if let Some((c, len, open)) = fence {
            if fence_char == Some(c) && run >= len && t[run..].trim().is_empty() {
                regions.push((open, l.saturating_sub(1)));
                fence = None;
            }
            continue;
        }
        if run >= 3 {
            fence = fence_char.map(|c| (c, run, l));
            continue;
        }
        let level = t.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&level) && t[level..].chars().next().is_none_or(char::is_whitespace) {
            headings.push((l, level));
        }
    }
    for (i, &(h, level)) in headings.iter().enumerate() {
        let next = headings[i + 1..]
            .iter()
            .find(|&&(_, lv)| lv <= level)
            .map_or(n, |&(l, _)| l);
        if let Some(end) = last_text_line(&blank, h + 1, next) {
            regions.push((h, end));
        }
    }
    regions
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Kind {
    Bracket(char),
    Tag(String),
    Word,
    Env(String),
    /// A user-defined language's `folding` pair.
    Block,
}

impl Kind {
    /// Brackets pair up only within one tag, keyword block or environment.
    fn bounds_brackets(&self) -> bool {
        !matches!(self, Kind::Bracket(_))
    }
}

#[derive(Clone)]
struct Open {
    kind: Kind,
    line: usize,
}

/// A tag or `\begin`/`\end` being read, until its name (and for tags, its `>`) arrives.
#[derive(Clone)]
struct Pending {
    close: bool,
    line: usize,
    first: bool,
    name: Option<String>,
}

/// Scope prefixes the scan looks for, made once per scan.
struct Scopes {
    comment: Scope,
    string: Scope,
    character: Scope,
    tag_begin: Scope,
    tag_end: Scope,
    tag_name: Scope,
    keyword: Scope,
    source: Scope,
    env_begin: Scope,
    env_end: Scope,
    env_name: Scope,
    block_begin: Scope,
    block_end: Scope,
}

impl Scopes {
    fn new() -> Self {
        let s = |name: &str| Scope::new(name).expect("valid scope");
        Self {
            comment: s("comment"),
            string: s("string"),
            character: s("constant.character"),
            tag_begin: s("punctuation.definition.tag.begin"),
            tag_end: s("punctuation.definition.tag.end"),
            tag_name: s("entity.name.tag"),
            keyword: s("keyword"),
            source: s("source"),
            env_begin: s("support.function.begin.latex"),
            env_end: s("support.function.end.latex"),
            env_name: s("variable.parameter.function.latex"),
            block_begin: s("punctuation.section.block.begin"),
            block_end: s("punctuation.section.block.end"),
        }
    }
}

/// What the scopes of one stretch of text say about it.
#[derive(Default)]
struct Class {
    comment: bool,
    string: bool,
    tag_begin: bool,
    tag_end: bool,
    tag_name: bool,
    keyword: bool,
    embedded: bool,
    env_begin: bool,
    env_end: bool,
    env_name: bool,
    block_begin: bool,
    block_end: bool,
    user: bool,
}

struct GrammarScan {
    rules: Rules,
    scopes: Scopes,
    user_root: Scope,
    stack: Vec<Open>,
    regions: Vec<(usize, usize)>,
    tag: Option<Pending>,
    env: Option<Pending>,
    markers: Vec<usize>,
}

const HTML_EMPTY: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr", "command", "keygen",
];

/// Per-line state while scanning.
#[derive(Default)]
struct LineState {
    has_code: bool,
    has_comment: bool,
    /// Byte offset of the first code (non-comment, non-space) char.
    first_code: Option<usize>,
    last_code_char: Option<char>,
    loop_head: bool,
}

impl GrammarScan {
    fn new(rules: Rules) -> Self {
        Self {
            rules,
            scopes: Scopes::new(),
            user_root: Scope::new("source.user").expect("valid scope"),
            stack: Vec::new(),
            regions: Vec::new(),
            tag: None,
            env: None,
            markers: Vec::new(),
        }
    }

    fn classify(&self, stack: &ScopeStack) -> Class {
        let s = &self.scopes;
        let mut c = Class::default();
        for (i, scope) in stack.as_slice().iter().enumerate() {
            let has = |p: &Scope| p.is_prefix_of(*scope);
            c.comment |= has(&s.comment);
            c.string |= has(&s.string) || has(&s.character);
            c.tag_begin |= has(&s.tag_begin);
            c.tag_end |= has(&s.tag_end);
            c.tag_name |= has(&s.tag_name);
            c.keyword |= has(&s.keyword);
            c.embedded |= i > 0 && has(&s.source);
            c.env_begin |= has(&s.env_begin);
            c.env_end |= has(&s.env_end);
            c.env_name |= has(&s.env_name);
            c.block_begin |= has(&s.block_begin);
            c.block_end |= has(&s.block_end);
            c.user |= i == 0 && self.user_root.is_prefix_of(*scope);
        }
        c
    }

    /// Closes the innermost open item `matches` accepts, unless `bounded`
    /// stops the search first. The region ends on the line above the closer
    /// when the closer starts its line, so the closing line stays visible.
    fn close(
        &mut self,
        line: usize,
        first: bool,
        matches: impl Fn(&Kind) -> bool,
        bounded: impl Fn(&Kind) -> bool,
    ) {
        let mut i = self.stack.len();
        while i > 0 {
            i -= 1;
            let kind = &self.stack[i].kind;
            if matches(kind) {
                let open = self.stack[i].line;
                self.stack.truncate(i);
                let end = if first { line.saturating_sub(1) } else { line };
                if end > open {
                    self.regions.push((open, end));
                }
                return;
            }
            if bounded(kind) {
                return;
            }
        }
    }

    fn run(
        mut self,
        text: &Rope,
        syntaxes: &SyntaxSet,
        syntax: &SyntaxReference,
        cancel: &AtomicBool,
        resume: Option<(&ScanCache, usize)>,
    ) -> Option<(Vec<(usize, usize)>, ScanCache)> {
        let n = text.len_lines();
        let mut parse = ParseState::new(syntax);
        let mut scopes = ScopeStack::new();
        let mut comment_only = vec![false; n];
        let mut marker = vec![false; n];
        let mut blank = vec![true; n];
        let mut sections = Vec::new();
        let mut checkpoints: Vec<Checkpoint> = Vec::new();
        let mut first = 0;
        if let Some((cache, dirty)) = resume
            && let Some(i) = cache
                .checkpoints
                .iter()
                .rposition(|c| c.line <= dirty.min(n))
        {
            let cp = &cache.checkpoints[i];
            first = cp.line;
            parse = cp.parse.clone();
            scopes = cp.scopes.clone();
            self.stack = cp.stack.clone();
            self.tag = cp.tag.clone();
            self.env = cp.env.clone();
            self.markers = cp.markers.clone();
            self.regions = cache.regions[..cp.regions].to_vec();
            checkpoints = cache.checkpoints[..=i].to_vec();
            comment_only[..first].copy_from_slice(&cache.comment_only[..first]);
            marker[..first].copy_from_slice(&cache.marker[..first]);
            blank[..first].copy_from_slice(&cache.blank[..first]);
            sections = cache
                .sections
                .iter()
                .copied()
                .filter(|&l| l < first)
                .collect();
        }
        let mut buf = String::new();
        for l in first..n {
            if l % 512 == 0 && cancel.load(Ordering::Relaxed) {
                return None;
            }
            if l % CHECKPOINT_LINES == 0 && checkpoints.last().is_none_or(|c| c.line < l) {
                checkpoints.push(Checkpoint {
                    line: l,
                    parse: parse.clone(),
                    scopes: scopes.clone(),
                    stack: self.stack.clone(),
                    tag: self.tag.clone(),
                    env: self.env.clone(),
                    markers: self.markers.clone(),
                    regions: self.regions.len(),
                });
            }
            buf.clear();
            for chunk in text.line(l).chunks() {
                buf.push_str(chunk);
            }
            if !buf.ends_with('\n') {
                buf.push('\n');
            }
            if buf.len() > MAX_LINE_BYTES {
                blank[l] = false;
                continue;
            }
            let Ok(ops) = parse.parse_line(&buf, syntaxes) else {
                continue;
            };
            let bracket_free = !self
                .stack
                .iter()
                .any(|o| matches!(o.kind, Kind::Bracket(_)));
            let mut st = LineState::default();
            let mut pos = 0;
            for (i, op) in ops {
                if i > pos {
                    let class = self.classify(&scopes);
                    self.segment(l, &buf, pos, i, &class, &mut st);
                    pos = i;
                }
                // A broken grammar must not stop the scan.
                let _ = scopes.apply(&op);
            }
            if pos < buf.len() {
                let class = self.classify(&scopes);
                self.segment(l, &buf, pos, buf.len(), &class, &mut st);
            }
            let trimmed = buf.trim();
            blank[l] = !st.has_code && !st.has_comment;
            comment_only[l] = st.has_comment && !st.has_code;
            if comment_only[l] || trimmed.starts_with('#') {
                match region_marker(trimmed) {
                    Some(true) => {
                        self.markers.push(l);
                        marker[l] = true;
                    }
                    Some(false) => {
                        if let Some(open) = self.markers.pop()
                            && l > open + 1
                        {
                            self.regions.push((open, l - 1));
                        }
                        marker[l] = true;
                    }
                    None => {}
                }
            }
            if self.rules.sections && bracket_free && st.has_code && trimmed.starts_with('[') {
                sections.push(l);
            }
        }
        let mut cache = ScanCache {
            checkpoints,
            regions: self.regions.clone(),
            comment_only: Vec::new(),
            marker: Vec::new(),
            blank: Vec::new(),
            sections: sections.clone(),
        };
        // Runs of whole-line comments (license headers, doc comments, block comments).
        let mut run_start = None;
        for l in 0..=n {
            let in_run = l < n && comment_only[l] && !marker[l];
            match (in_run, run_start) {
                (true, None) => run_start = Some(l),
                (false, Some(s)) => {
                    if l - 1 > s {
                        self.regions.push((s, l - 1));
                    }
                    run_start = None;
                }
                _ => {}
            }
        }
        for (i, &h) in sections.iter().enumerate() {
            let next = sections.get(i + 1).copied().unwrap_or(n);
            if let Some(end) = last_text_line(&blank, h + 1, next) {
                self.regions.push((h, end));
            }
        }
        cache.comment_only = comment_only;
        cache.marker = marker;
        cache.blank = blank;
        Some((self.regions, cache))
    }

    /// Handles one stretch of a line that has the same scopes.
    fn segment(&mut self, l: usize, line: &str, a: usize, b: usize, c: &Class, st: &mut LineState) {
        let text = &line[a..b];
        if c.comment {
            st.has_comment = true;
            return;
        }
        if let Some(off) = text.find(|ch: char| !ch.is_whitespace()) {
            st.has_code = true;
            st.first_code.get_or_insert(a + off);
        }
        let first = st.first_code == Some(a + text.len() - text.trim_start().len());
        if c.string {
            st.last_code_char = text.trim_end().chars().last().or(st.last_code_char);
            return;
        }

        // Tags: `<name …>` opens, `</name>` closes, `<name/>` does neither.
        if c.tag_begin && self.tag.is_none() {
            let t = text.trim();
            if t == "<" || t == "</" {
                self.tag = Some(Pending {
                    close: t == "</",
                    line: l,
                    first,
                    name: None,
                });
            }
        } else if c.tag_name {
            if let Some(p) = &mut self.tag
                && p.name.is_none()
            {
                let name = text.trim();
                p.name = Some(if self.rules.html {
                    name.to_lowercase()
                } else {
                    name.to_owned()
                });
            }
        } else if c.tag_end
            && let Some(p) = self.tag.take()
        {
            let name = p.name.unwrap_or_default();
            if p.close {
                self.close(
                    p.line,
                    p.first,
                    |k| *k == Kind::Tag(name.clone()),
                    |_| false,
                );
            } else if !text.trim_start().starts_with('/')
                && !(self.rules.html && HTML_EMPTY.contains(&name.as_str()))
            {
                self.stack.push(Open {
                    kind: Kind::Tag(name),
                    line: p.line,
                });
            }
        }

        // LaTeX environments: the name comes in braces after `\begin` or `\end`.
        if (c.env_begin || c.env_end) && self.env.is_none() {
            self.env = Some(Pending {
                close: c.env_end,
                line: l,
                first,
                name: None,
            });
        } else if c.env_name
            && let Some(p) = &mut self.env
        {
            p.name = Some(text.trim().to_owned());
        }

        if c.user && (c.block_begin || c.block_end) {
            if c.block_begin {
                self.stack.push(Open {
                    kind: Kind::Block,
                    line: l,
                });
            } else {
                self.close(l, first, |k| *k == Kind::Block, |_| false);
            }
            st.last_code_char = text.trim_end().chars().last().or(st.last_code_char);
            return;
        }

        if c.keyword
            && let Some(words) = self.rules.words
        {
            self.words(words, l, line, a, text, st);
        }

        let brackets = match self.rules.brackets {
            Brackets::All => true,
            Brackets::Embedded => c.embedded,
            Brackets::None => false,
        };
        if brackets && !c.tag_begin && !c.tag_end {
            for (off, ch) in text.char_indices() {
                let first = st.first_code == Some(a + off);
                match ch {
                    '(' | '[' | '{' => self.stack.push(Open {
                        kind: Kind::Bracket(ch),
                        line: l,
                    }),
                    ')' | ']' | '}' => {
                        let open = match ch {
                            ')' => '(',
                            ']' => '[',
                            _ => '{',
                        };
                        self.close(
                            l,
                            first,
                            |k| *k == Kind::Bracket(open),
                            Kind::bounds_brackets,
                        );
                        if ch == '}' {
                            self.finish_env();
                        }
                    }
                    _ => {}
                }
            }
        } else if text.contains('}') {
            self.finish_env();
        }
        st.last_code_char = text.trim_end().chars().last().or(st.last_code_char);
    }

    /// Opens or closes a LaTeX environment once its name's closing brace is read.
    fn finish_env(&mut self) {
        let Some(name) = self.env.as_ref().and_then(|p| p.name.clone()) else {
            return;
        };
        let p = self.env.take().expect("checked above");
        if p.close {
            self.close(
                p.line,
                p.first,
                |k| *k == Kind::Env(name.clone()),
                |_| false,
            );
        } else {
            self.stack.push(Open {
                kind: Kind::Env(name),
                line: p.line,
            });
        }
    }

    fn words(
        &mut self,
        words: &Words,
        l: usize,
        line: &str,
        a: usize,
        text: &str,
        st: &mut LineState,
    ) {
        let mut rest = text;
        let mut base = a;
        while let Some(start) = rest.find(|c: char| c.is_alphanumeric() || c == '_') {
            let len = rest[start..]
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len() - start);
            let pos = base + start;
            let raw = &rest[start..start + len];
            let word = if words.ignore_case {
                raw.to_lowercase()
            } else {
                raw.to_owned()
            };
            let first = st.first_code == Some(pos);
            if words.close.contains(&word.as_str()) {
                self.close(l, first, |k| *k == Kind::Word, |k| *k != Kind::Word);
            } else if words.open.contains(&word.as_str()) {
                let starts_statement = first
                    || line[..pos].trim_end().ends_with('=')
                    || st.last_code_char.is_some_and(|c| c == ';' || c == '(');
                let allowed = if words.statement_only.contains(&word.as_str()) {
                    starts_statement
                } else {
                    !(word == "do" && st.loop_head)
                };
                if allowed {
                    if words.loop_heads.contains(&word.as_str()) {
                        st.loop_head = true;
                    }
                    self.stack.push(Open {
                        kind: Kind::Word,
                        line: l,
                    });
                }
            }
            rest = &rest[start + len..];
            base = pos + len;
        }
    }
}

/// `Some(true)` for a `#region` style line, `Some(false)` for its end.
fn region_marker(trimmed: &str) -> Option<bool> {
    let t = trimmed
        .trim_start_matches(|c: char| "/#*;<!-'%".contains(c) || c.is_whitespace())
        .to_lowercase();
    let t = t.strip_prefix("pragma").map_or(t.as_str(), str::trim_start);
    let word_end = |rest: &str| rest.chars().next().is_none_or(|c| !c.is_alphanumeric());
    if let Some(rest) = t.strip_prefix("endregion") {
        return word_end(rest).then_some(false);
    }
    if let Some(rest) = t.strip_prefix("end region") {
        return word_end(rest).then_some(false);
    }
    if let Some(rest) = t.strip_prefix("region") {
        return word_end(rest).then_some(true);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions(language: &str, text: &str) -> Vec<(usize, usize)> {
        let h = crate::highlight::tests::shared();
        let syntax = h.syntaxes.find_syntax_by_name(language).expect(language);
        let style = style_for(syntax);
        scan(
            &Rope::from_str(text),
            &h.syntaxes,
            syntax,
            style,
            &AtomicBool::new(false),
        )
        .map(|r| r.starts.into_iter().collect())
        .unwrap_or_default()
    }

    #[test]
    fn braces_skip_strings_and_comments() {
        let code = "fn a() {\n    let s = \"{\";\n    // }\n    if x {\n        y();\n    }\n}\nfn b() {}\n";
        assert_eq!(regions("Rust", code), vec![(0, 5), (3, 4)]);
    }

    #[test]
    fn closing_line_stays_visible_and_else_chains() {
        let code = "if (a) {\n  x();\n} else {\n  y();\n}\n";
        assert_eq!(regions("JavaScript", code), vec![(0, 1), (2, 3)]);
        // A closer after text on its line is hidden with the region.
        let call = "foo(\n  1,\n  2)\nbar();\n";
        assert_eq!(regions("JavaScript", call), vec![(0, 2)]);
    }

    #[test]
    fn json_nesting() {
        let json = "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {\n    \"c\": \"}\"\n  }\n}\n";
        assert_eq!(regions("JSON", json), vec![(0, 7), (1, 3), (5, 6)]);
    }

    #[test]
    fn comment_blocks_and_region_markers() {
        let code = "/*\n * Licence\n */\nint x;\n// #region Helpers\nint a;\nint b;\n// #endregion\n/// one\n/// two\nint y;\n";
        assert_eq!(regions("C", code), vec![(0, 2), (4, 6), (8, 9)]);
        let cs = "#region R\nint x;\nint y;\n#endregion\n";
        assert_eq!(regions("C#", cs), vec![(0, 2)]);
    }

    #[test]
    fn xml_and_html_tags() {
        let xml = "<?xml version=\"1.0\"?>\n<a>\n  <b x=\"1\"/>\n  <c>\n    t\n  </c>\n</a>\n";
        assert_eq!(regions("XML", xml), vec![(1, 5), (3, 4)]);
        // Empty elements and unclosed <p> don't break the pairing; script code folds.
        let html = "<div>\n<br>\n<p>text (\n<ul>\n<li>a\n</ul>\n</div>\n<script>\nif (a) {\n  b();\n}\n</script>\n";
        assert_eq!(regions("HTML", html), vec![(0, 5), (3, 4), (7, 10), (8, 9)]);
    }

    #[test]
    fn keyword_blocks() {
        let ruby = "def a(x)\n  y = 1 if x\n  while x do\n    z\n  end\n  [1].each do |v|\n    p v\n  end\nend\n";
        assert_eq!(regions("Ruby", ruby), vec![(0, 7), (2, 3), (5, 6)]);
        let lua =
            "function f()\n  if a then\n    b()\n  end\n  for i = 1, 2 do\n    c()\n  end\nend\n";
        assert_eq!(regions("Lua", lua), vec![(0, 6), (1, 2), (4, 5)]);
        let sh = "if [ a ]; then\n  echo\nfi\nfor x in a; do\n  echo $x\ndone\n";
        assert_eq!(
            regions("Bourne Again Shell (bash)", sh),
            vec![(0, 1), (3, 4)]
        );
    }

    #[test]
    fn sections_markdown_and_latex() {
        let ini = "; top\n[a]\nx=1\ny=2\n\n[b]\nz=3\n";
        assert_eq!(regions("INI", ini), vec![(1, 3), (5, 6)]);
        let toml = "[a]\nx = [\n  1,\n  2,\n]\n\n[b]\ny = 1\n";
        assert_eq!(regions("TOML", toml), vec![(0, 4), (1, 3), (6, 7)]);
        let md = "# A\ntext\n## B\nmore\n```\n# not a heading\n```\n\n# C\nend\n";
        assert_eq!(
            regions("Markdown", md),
            vec![(0, 6), (2, 6), (4, 5), (8, 9)]
        );
        let tex =
            "\\begin{document}\n\\begin{itemize}\n\\item a\n\\end{itemize}\n\\end{document}\n";
        assert_eq!(regions("LaTeX", tex), vec![(0, 3), (1, 2)]);
    }

    #[test]
    fn user_language_folding_pairs() {
        let json = r##"{"name": "Fold Test", "extensions": ["ft"], "line_comment": "#",
            "keywords": {"keyword": ["if", "end", "then"]},
            "folding": [["{", "}"], ["if", "end"]]}"##;
        let yaml = crate::user_lang::to_sublime_syntax(json).unwrap();
        let def = syntect::parsing::SyntaxDefinition::load_from_str(&yaml, true, None).unwrap();
        let mut builder = syntect::parsing::SyntaxSetBuilder::new();
        builder.add(def);
        let set = builder.build();
        let syntax = set.find_syntax_by_name("Fold Test").unwrap();
        let text = "if a then\n  x { # if\n    y\n  }\nend\nappend\n";
        let found = scan(
            &Rope::from_str(text),
            &set,
            syntax,
            style_for(syntax),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            found.starts.into_iter().collect::<Vec<_>>(),
            vec![(0, 3), (1, 2)]
        );
    }

    #[test]
    fn indentation_languages_are_left_alone() {
        assert!(regions("Python", "def a():\n    pass\n").is_empty());
        let h = crate::highlight::tests::shared();
        assert_eq!(
            style_for(h.syntaxes.find_syntax_plain_text()),
            Style::Indent
        );
    }

    #[test]
    fn rescans_resume_from_the_unchanged_start() {
        let h = crate::highlight::tests::shared();
        let syntax = h.syntaxes.find_syntax_by_name("Rust").unwrap();
        let style = style_for(syntax);
        let block = "fn a() {\n    /* x\n    y */\n    b();\n}\n";
        let before = block.repeat(100);
        let no = AtomicBool::new(false);
        let (_, cache) = scan_resuming(
            &Rope::from_str(&before),
            &h.syntaxes,
            syntax,
            style,
            &no,
            None,
        );
        let cache = cache.unwrap();
        // Open a comment on line 301; everything after it changes.
        let mut lines: Vec<&str> = before.lines().collect();
        lines.insert(301, "/*");
        let after = lines.join("\n") + "\n";
        let rope = Rope::from_str(&after);
        let resumed = scan_resuming(&rope, &h.syntaxes, syntax, style, &no, Some((&cache, 301))).0;
        let fresh = scan(&rope, &h.syntaxes, syntax, style, &no);
        assert_eq!(resumed, fresh);
        assert!(resumed.unwrap().starts.len() < 200);
    }

    #[test]
    fn regions_move_with_edits() {
        let mut r = FoldRegions::new([(2, 5), (3, 4)], 10);
        assert_eq!((r.depth(3), r.depth(4), r.depth(6)), (1, 2, 0));
        assert_eq!(r.level(3), 2);
        r.map_edits(
            &[LineEdit {
                start_line: 0,
                old_lines: 1,
                new_lines: 3,
            }],
            12,
        );
        assert_eq!(
            r.starts.into_iter().collect::<Vec<_>>(),
            vec![(4, 7), (5, 6)]
        );
    }
}
