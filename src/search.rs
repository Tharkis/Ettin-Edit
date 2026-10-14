//! Search: compiling a query from the find options, finding matches in text,
//! expanding replacement templates, and searching files on disk.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchOptions {
    pub match_case: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub wrap: bool,
}

/// A compiled search.
#[derive(Clone, Debug)]
pub struct Query {
    re: Regex,
    whole_word: bool,
    is_regex: bool,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Query {
    /// Compiles `pattern`. Returns `Ok(None)` for an empty pattern, and the
    /// regex error as text when the pattern is invalid.
    pub fn new(pattern: &str, opts: &SearchOptions) -> Result<Option<Query>, String> {
        if pattern.is_empty() {
            return Ok(None);
        }
        let source = if opts.regex {
            pattern.to_owned()
        } else {
            regex::escape(pattern)
        };
        let re = RegexBuilder::new(&source)
            .case_insensitive(!opts.match_case)
            .multi_line(true)
            .size_limit(50 << 20)
            .build()
            .map_err(|e| match e {
                regex::Error::Syntax(s) => s.lines().last().unwrap_or("invalid regex").to_owned(),
                other => other.to_string(),
            })?;
        Ok(Some(Query {
            re,
            whole_word: opts.whole_word,
            is_regex: opts.regex,
        }))
    }

    fn word_bounded(&self, text: &str, m: &Range<usize>) -> bool {
        if !self.whole_word {
            return true;
        }
        let before = text[..m.start].chars().next_back();
        let after = text[m.end..].chars().next();
        !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
    }

    /// The first match starting at or after byte `from`.
    pub fn find_at(&self, text: &str, from: usize) -> Option<Range<usize>> {
        let mut at = from;
        while at <= text.len() {
            let m = self.re.find_at(text, at)?.range();
            if self.word_bounded(text, &m) {
                return Some(m);
            }
            // Try again one char further on.
            at = m.start + text[m.start..].chars().next().map_or(1, char::len_utf8);
        }
        None
    }

    /// All matches in order, including empty ones (which only regexes produce).
    pub fn find_iter<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Range<usize>> + 'a {
        let mut at = 0usize;
        let mut last_end: Option<usize> = None;
        std::iter::from_fn(move || {
            loop {
                if at > text.len() {
                    return None;
                }
                let m = self.find_at(text, at)?;
                // An empty match right where the previous match ended is skipped,
                // the same rule the regex crate uses.
                if m.is_empty() && last_end == Some(m.start) {
                    at = m.end + text[m.end..].chars().next().map_or(1, char::len_utf8);
                    continue;
                }
                at = if m.is_empty() {
                    m.end + text[m.end..].chars().next().map_or(1, char::len_utf8)
                } else {
                    m.end
                };
                last_end = Some(m.end);
                return Some(m);
            }
        })
    }

    /// The match that is exactly `range`, if there is one (used by Replace to
    /// check that the selection is still a match).
    pub fn matches_exactly(&self, text: &str, range: &Range<usize>) -> bool {
        self.find_at(text, range.start).is_some_and(|m| m == *range)
    }

    /// The replacement text for match `m`. Regex templates understand `$1`,
    /// `${name}`, `\1`..`\9`, `\n`, `\t` and `\\`; plain searches insert
    /// `template` as typed.
    pub fn expand(&self, text: &str, m: &Range<usize>, template: &str) -> String {
        if !self.is_regex {
            return template.to_owned();
        }
        let Some(caps) = self.re.captures_at(text, m.start) else {
            return template.to_owned();
        };
        let mut out = String::new();
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some(d @ '0'..='9') => {
                        let i = d as usize - '0' as usize;
                        out.push_str(caps.get(i).map_or("", |g| g.as_str()));
                    }
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                },
                '$' => {
                    let mut name = String::new();
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        for c in chars.by_ref() {
                            if c == '}' {
                                break;
                            }
                            name.push(c);
                        }
                    } else if chars.peek() == Some(&'$') {
                        chars.next();
                        out.push('$');
                        continue;
                    } else {
                        while let Some(&c) = chars.peek() {
                            if c.is_ascii_digit() {
                                name.push(c);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    if name.is_empty() {
                        out.push('$');
                    } else if let Ok(i) = name.parse::<usize>() {
                        out.push_str(caps.get(i).map_or("", |g| g.as_str()));
                    } else {
                        out.push_str(caps.name(&name).map_or("", |g| g.as_str()));
                    }
                }
                c => out.push(c),
            }
        }
        out
    }
}

// -------------------------------------------------------------------------
// Results shared by "find all" in documents and Find in Files.

/// One line with matches.
#[derive(Clone, Debug)]
pub struct Hit {
    /// 0-based line number.
    pub line: usize,
    /// Char column where the first match starts, and its length in chars.
    pub col: usize,
    pub len: usize,
    /// The line's text (trimmed, and cut short if very long) and the byte
    /// ranges of the matches in it.
    pub text: String,
    pub highlights: Vec<Range<usize>>,
    /// Number of matches on the line (some may fall outside a cut-short preview).
    pub matches: usize,
}

const MAX_PREVIEW_CHARS: usize = 300;

/// Every match in `text`, grouped by line, at most `limit` matches.
/// Returns the lines and whether the limit cut them short.
pub fn line_hits(query: &Query, text: &str, limit: usize) -> (Vec<Hit>, bool) {
    let mut hits: Vec<Hit> = Vec::new();
    let mut window: Option<Window> = None;
    let mut count = 0usize;
    let mut line = 0usize;
    let mut line_start = 0usize;
    let mut scanned = 0usize;
    for m in query.find_iter(text) {
        if m.is_empty() {
            continue;
        }
        if count >= limit {
            return (hits, true);
        }
        count += 1;
        // Advance the line counter to the match.
        let mut new_line = false;
        for (i, b) in text.as_bytes()[scanned..m.start].iter().enumerate() {
            if *b == b'\n' {
                line += 1;
                line_start = scanned + i + 1;
                new_line = true;
            }
        }
        scanned = m.start;
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |i| line_start + i);
        let (start, end) = (m.start - line_start, m.end.min(line_end) - line_start);
        if let Some(last) = hits.last_mut().filter(|h| h.line == line && !new_line) {
            last.matches += 1;
            if let Some(r) = window.as_ref().and_then(|w| w.map(start, end)) {
                last.highlights.push(r);
            }
            continue;
        }
        let w = Window::new(&text[line_start..line_end], start, end);
        let highlights = w.map(start, end).into_iter().collect();
        hits.push(Hit {
            line,
            col: text[line_start..m.start].chars().count(),
            len: text[m.clone()].chars().count(),
            text: w.text.clone(),
            highlights,
            matches: 1,
        });
        window = Some(w);
    }
    (hits, false)
}

/// The part of a line shown in the results: leading whitespace dropped, and
/// a long line cut down to a window around its first match.
struct Window {
    text: String,
    /// Byte range of the line that is shown, and how far it moved in `text`.
    from: usize,
    to: usize,
    prefix: usize,
}

impl Window {
    fn new(line: &str, start: usize, end: usize) -> Self {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let start = start.min(line.len());
        let end = end.clamp(start, line.len());
        let indent = line.len() - line.trim_start().len();
        if line.chars().count() <= MAX_PREVIEW_CHARS {
            let from = indent.min(start);
            return Self {
                text: line[from..].to_owned(),
                from,
                to: line.len(),
                prefix: 0,
            };
        }
        // Keep up to 60 chars before the match.
        let mut from = start;
        for _ in 0..60 {
            match line[..from].chars().next_back() {
                Some(c) => from -= c.len_utf8(),
                None => break,
            }
        }
        let from = from.max(indent.min(start));
        let mut to = from;
        for c in line[from..].chars().take(MAX_PREVIEW_CHARS) {
            to += c.len_utf8();
        }
        let to = to.max(end.min(from + 4 * MAX_PREVIEW_CHARS));
        let to = (0..=to)
            .rev()
            .find(|&i| line.is_char_boundary(i))
            .unwrap_or(from);
        let prefix = if from > indent { "…" } else { "" };
        Self {
            text: format!("{prefix}{}", &line[from..to]),
            from,
            to,
            prefix: prefix.len(),
        }
    }

    /// Where line bytes `start..end` appear in the shown text, if they do.
    fn map(&self, start: usize, end: usize) -> Option<Range<usize>> {
        if start < self.from || start >= self.to {
            return None;
        }
        let s = start - self.from + self.prefix;
        let e = end.min(self.to) - self.from + self.prefix;
        Some(s..e.max(s))
    }
}

// -------------------------------------------------------------------------
// Find in Files.

#[derive(Clone, Debug)]
pub struct FileSearch {
    pub dir: PathBuf,
    /// File name patterns such as `*.rs *.toml`; a leading `!` excludes. Empty means all files.
    pub filters: String,
    pub subfolders: bool,
    pub hidden: bool,
}

pub enum FileSearchEvent {
    /// Matches in one file.
    File { path: PathBuf, hits: Vec<Hit> },
    /// The search finished (or was stopped).
    Done {
        files_searched: usize,
        truncated: bool,
    },
}

/// Files bigger than this are skipped.
const MAX_FILE_BYTES: u64 = 200 << 20;
pub const MAX_HITS: usize = 20_000;

struct Filters {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl Filters {
    fn parse(s: &str) -> Self {
        let mut include = Vec::new();
        let mut exclude = Vec::new();
        for part in s
            .split(|c: char| c == ';' || c == ',' || c.is_whitespace())
            .filter(|p| !p.is_empty())
        {
            match part.strip_prefix('!') {
                Some(p) if !p.is_empty() => exclude.push(p.to_lowercase()),
                Some(_) => {}
                None => include.push(part.to_lowercase()),
            }
        }
        Self { include, exclude }
    }

    fn accepts(&self, name: &str) -> bool {
        let name = name.to_lowercase();
        if self.exclude.iter().any(|p| glob_match(p, &name)) {
            return false;
        }
        self.include.is_empty() || self.include.iter().any(|p| glob_match(p, &name))
    }
}

/// `*` and `?` wildcard match on a whole name.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let (mut pi, mut ni) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ni));
            pi += 1;
        } else if let Some((sp, sn)) = star {
            pi = sp + 1;
            ni = sn + 1;
            star = Some((sp, sn + 1));
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Searches files under `search.dir`, sending one event per file with matches
/// and a final `Done`. Runs until finished or `cancel` is set.
pub fn search_files(
    search: FileSearch,
    query: Query,
    cancel: Arc<AtomicBool>,
    tx: Sender<FileSearchEvent>,
) {
    let filters = Filters::parse(&search.filters);
    let mut stack = vec![search.dir.clone()];
    let mut files_searched = 0usize;
    let mut total_hits = 0usize;
    let mut truncated = false;
    'walk: while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        let mut subdirs = Vec::new();
        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                break 'walk;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                if search.subfolders && (search.hidden || !name.starts_with('.')) {
                    subdirs.push(path);
                }
                continue;
            }
            // Symlinks are followed to files only, so a link loop can't trap the search.
            let is_file = kind.is_file() || (kind.is_symlink() && path.is_file());
            if !is_file || !filters.accepts(&name) || (!search.hidden && name.starts_with('.')) {
                continue;
            }
            let Some(text) = read_text(&path) else {
                continue;
            };
            files_searched += 1;
            let (hits, cut) = line_hits(&query, &text, MAX_HITS - total_hits);
            total_hits += hits.iter().map(|h| h.matches).sum::<usize>();
            if !hits.is_empty() && tx.send(FileSearchEvent::File { path, hits }).is_err() {
                return;
            }
            if cut || total_hits >= MAX_HITS {
                truncated = true;
                break 'walk;
            }
        }
        // Visit subfolders in name order.
        stack.extend(subdirs.into_iter().rev());
    }
    let _ = tx.send(FileSearchEvent::Done {
        files_searched,
        truncated,
    });
}

/// Reads a file as text, or `None` for binary or huge files.
fn read_text(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if bytes[..bytes.len().min(8000)].contains(&0) {
        return None;
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    Some(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(p: &str, opts: SearchOptions) -> Query {
        Query::new(p, &opts).unwrap().unwrap()
    }

    fn all(query: &Query, text: &str) -> Vec<Range<usize>> {
        query.find_iter(text).collect()
    }

    #[test]
    fn plain_search_ignores_case_by_default() {
        let query = q("ab", SearchOptions::default());
        assert_eq!(all(&query, "ab AB a.b"), vec![0..2, 3..5]);
        let query = q("a.b", SearchOptions::default());
        assert_eq!(all(&query, "axb a.b"), vec![4..7]);
    }

    #[test]
    fn match_case_and_whole_word() {
        let opts = SearchOptions {
            match_case: true,
            whole_word: true,
            ..Default::default()
        };
        let query = q("cat", opts);
        assert_eq!(all(&query, "cat Cat concat cat_ cat."), vec![0..3, 20..23]);
    }

    #[test]
    fn whole_word_retries_after_a_rejected_match() {
        let opts = SearchOptions {
            whole_word: true,
            regex: true,
            ..Default::default()
        };
        let query = q("a+", opts);
        assert_eq!(all(&query, "baa aa"), vec![4..6]);
    }

    #[test]
    fn regex_lines_and_empty_matches() {
        let opts = SearchOptions {
            regex: true,
            ..Default::default()
        };
        let query = q("^", opts);
        assert_eq!(all(&query, "a\nb\n"), vec![0..0, 2..2, 4..4]);
        let query = q("x*", opts);
        assert_eq!(all(&query, "axxb"), vec![0..0, 1..3, 4..4]);
    }

    #[test]
    fn invalid_regex_reports_an_error() {
        let opts = SearchOptions {
            regex: true,
            ..Default::default()
        };
        assert!(Query::new("(", &opts).is_err());
        assert!(Query::new("", &opts).unwrap().is_none());
    }

    #[test]
    fn replacement_templates() {
        let opts = SearchOptions {
            regex: true,
            ..Default::default()
        };
        let query = q(r"(\w+)=(?P<v>\d+)", opts);
        let text = "x key=42";
        let m = query.find_at(text, 0).unwrap();
        assert_eq!(query.expand(text, &m, r"\2:\1"), "42:key");
        assert_eq!(query.expand(text, &m, "$2-${v}-$$"), "42-42-$");
        assert_eq!(query.expand(text, &m, r"a\nb\t\\"), "a\nb\t\\");
        // Plain searches insert the text as typed.
        let plain = q("key", SearchOptions::default());
        let m = plain.find_at(text, 0).unwrap();
        assert_eq!(plain.expand(text, &m, r"$1\n"), r"$1\n");
    }

    #[test]
    fn hits_carry_line_and_column() {
        let query = q("é", SearchOptions::default());
        let (hits, cut) = line_hits(&query, "abc\nxéy é\n\né", 10);
        assert!(!cut);
        let pos: Vec<(usize, usize)> = hits.iter().map(|h| (h.line, h.col)).collect();
        assert_eq!(pos, vec![(1, 1), (3, 0)]);
        assert_eq!(hits[0].matches, 2);
        let marked: Vec<&str> = hits[0]
            .highlights
            .iter()
            .map(|r| &hits[0].text[r.clone()])
            .collect();
        assert_eq!(marked, vec!["é", "é"]);
        let (hits, cut) = line_hits(&query, "é é é", 2);
        assert_eq!(hits[0].matches, 2);
        assert!(cut);
    }

    #[test]
    fn long_lines_are_previewed_around_the_match() {
        let line = format!("{}needle{}", "a".repeat(1000), "b".repeat(1000));
        let query = q("needle", SearchOptions::default());
        let (hits, _) = line_hits(&query, &line, 10);
        let h = &hits[0];
        assert!(h.text.chars().count() <= 301);
        assert_eq!(&h.text[h.highlights[0].clone()], "needle");
        assert_eq!(h.col, 1000);
        // Leading indentation is dropped from short lines.
        let (hits, _) = line_hits(&query, "\t  a needle", 10);
        assert_eq!(hits[0].text, "a needle");
        assert_eq!(&hits[0].text[hits[0].highlights[0].clone()], "needle");
    }

    #[test]
    fn globs_and_filters() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "main.rsx"));
        assert!(glob_match("a?c*", "abcdef"));
        assert!(glob_match("*", "anything"));
        let f = Filters::parse("*.rs; *.TOML !build.rs");
        assert!(f.accepts("main.rs"));
        assert!(f.accepts("Cargo.toml"));
        assert!(!f.accepts("build.rs"));
        assert!(!f.accepts("README.md"));
        assert!(Filters::parse("").accepts("x"));
    }

    #[test]
    fn searches_a_folder() {
        let dir = std::env::temp_dir().join(format!("nixpad-fif-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("a.txt"), "one needle\ntwo\nneedle").unwrap();
        std::fs::write(dir.join("sub/b.txt"), "\u{FEFF}needle").unwrap();
        std::fs::write(dir.join(".git/c.txt"), "needle").unwrap();
        std::fs::write(dir.join("bin.txt"), b"needle\0").unwrap();
        std::fs::write(dir.join("d.log"), "needle").unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let search = FileSearch {
            dir: dir.clone(),
            filters: "*.txt".into(),
            subfolders: true,
            hidden: false,
        };
        search_files(
            search,
            q("needle", SearchOptions::default()),
            Arc::new(AtomicBool::new(false)),
            tx,
        );
        let mut found = Vec::new();
        let mut done = None;
        for ev in rx {
            match ev {
                FileSearchEvent::File { path, hits } => found.push((
                    path.strip_prefix(&dir).unwrap().to_path_buf(),
                    hits.iter().map(|h| h.line).collect::<Vec<_>>(),
                )),
                FileSearchEvent::Done { files_searched, .. } => done = Some(files_searched),
            }
        }
        assert_eq!(
            found,
            vec![
                (PathBuf::from("a.txt"), vec![0, 2]),
                (PathBuf::from("sub/b.txt"), vec![0]),
            ]
        );
        assert_eq!(done, Some(2));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
