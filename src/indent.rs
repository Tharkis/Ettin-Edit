//! How a document is indented: worked out from its text when it opens, from
//! the language's own setting, or from the general preferences.

use crate::settings::Settings;

/// Lines looked at when guessing a file's indentation.
pub const DETECT_LINES: usize = 1000;

/// Tab width and whether Tab types spaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Indent {
    pub tab_width: usize,
    pub use_spaces: bool,
}

/// What a file's own indentation says. `tab_width` is `None` for files
/// indented with tabs (how wide a tab shows is the user's choice).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Detected {
    pub use_spaces: bool,
    pub tab_width: Option<usize>,
}

/// Guesses the indentation from the first [`DETECT_LINES`] indented lines.
/// `None` when the text has too few indented lines to tell.
pub fn detect(text: &str) -> Option<Detected> {
    let mut tabs = 0usize;
    let mut spaces = 0usize;
    // How often each step between one line's indent and the next appears.
    let mut steps = [0usize; 9];
    let mut prev = 0usize;
    let mut seen = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let lead: &str = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        if lead.is_empty() {
            prev = 0;
            continue;
        }
        seen += 1;
        if seen > DETECT_LINES {
            break;
        }
        if lead.starts_with('\t') {
            tabs += 1;
            continue;
        }
        let width = lead.chars().take_while(|&c| c == ' ').count();
        // A line inside a block comment (` * text`) says little.
        if line.trim_start().starts_with('*') {
            continue;
        }
        spaces += 1;
        let step = width.abs_diff(prev);
        if (2..=8).contains(&step) {
            steps[step] += 1;
        }
        prev = width;
    }
    if tabs + spaces < 3 {
        return None;
    }
    if tabs > spaces {
        return Some(Detected {
            use_spaces: false,
            tab_width: None,
        });
    }
    let (best, count) = steps
        .iter()
        .enumerate()
        .max_by_key(|&(w, n)| (*n, std::cmp::Reverse(w)))?;
    if *count == 0 {
        return None;
    }
    Some(Detected {
        use_spaces: true,
        tab_width: Some(best),
    })
}

/// The indentation for a document in `language`: what the file uses (when
/// detection is on and found something), else the language's own setting,
/// else the general preference. `own` is a choice made for this tab with
/// View > Indent with Spaces.
pub fn effective(
    settings: &Settings,
    language: &str,
    detected: Option<Detected>,
    own: Option<bool>,
) -> Indent {
    let lang = settings.language_overrides.get(&language.to_lowercase());
    let mut indent = Indent {
        tab_width: settings.tab_width,
        use_spaces: settings.use_spaces,
    };
    if let Some(l) = lang {
        if let Some(w) = l.tab_width {
            indent.tab_width = w;
        }
        if let Some(t) = l.use_tabs {
            indent.use_spaces = !t;
        }
    }
    if settings.detect_indent
        && let Some(d) = detected
    {
        indent.use_spaces = d.use_spaces;
        if let Some(w) = d.tab_width {
            indent.tab_width = w;
        }
    }
    if let Some(spaces) = own {
        indent.use_spaces = spaces;
    }
    indent.tab_width = indent.tab_width.clamp(1, 16);
    indent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_four_spaces_in_python() {
        let py = "import os\n\ndef f(x):\n    if x:\n        return 1\n    return 2\n\nclass A:\n    def g(self):\n        pass\n";
        assert_eq!(
            detect(py),
            Some(Detected {
                use_spaces: true,
                tab_width: Some(4)
            })
        );
    }

    #[test]
    fn detects_two_spaces_and_tabs() {
        let json = "{\n  \"a\": {\n    \"b\": 1,\n    \"c\": [\n      2\n    ]\n  }\n}\n";
        assert_eq!(detect(json).and_then(|d| d.tab_width), Some(2));
        let make = "all: build\n\nbuild:\n\tcargo build\n\tcargo test\n\nclean:\n\trm -rf target\n";
        assert_eq!(
            detect(make),
            Some(Detected {
                use_spaces: false,
                tab_width: None
            })
        );
        assert_eq!(detect("one\ntwo\n  three\n"), None);
    }

    #[test]
    fn languages_and_detection_override_the_defaults() {
        let s = Settings::default();
        // JSON and Makefiles from the built-in language settings.
        assert_eq!(
            effective(&s, "JSON", None, None),
            Indent {
                tab_width: 2,
                use_spaces: true
            }
        );
        assert!(!effective(&s, "Makefile", None, None).use_spaces);
        // A plain language uses the general setting (tabs, 4 wide).
        assert_eq!(
            effective(&s, "Rust", None, None),
            Indent {
                tab_width: 4,
                use_spaces: false
            }
        );
        // A 4-space Python file.
        let found = detect("def f():\n    a = 1\n    if a:\n        b = 2\n");
        assert_eq!(
            effective(&s, "Python", found, None),
            Indent {
                tab_width: 4,
                use_spaces: true
            }
        );
        // With detection off the file's own style is ignored.
        let off = Settings {
            detect_indent: false,
            ..Settings::default()
        };
        assert!(!effective(&off, "Python", found, None).use_spaces);
        // The tab's own choice wins.
        assert!(!effective(&s, "Python", found, Some(false)).use_spaces);
    }
}
