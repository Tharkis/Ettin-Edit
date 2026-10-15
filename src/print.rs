//! Printing and HTML export: the document as an HTML page with its syntax
//! colours. Printing opens the page in the browser, which shows its print
//! dialog and preview, so it works the same on Linux and Windows.

use std::fmt::Write as _;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Documents bigger than this are printed without colours, which is much faster.
const MAX_COLOURED_BYTES: usize = 4 * 1024 * 1024;

pub struct HtmlOptions<'a> {
    pub title: &'a str,
    pub line_numbers: bool,
    /// Use the theme's colours for the text; otherwise everything is black.
    pub colours: bool,
    /// Light background with the text darkened where needed, for paper.
    pub for_print: bool,
    pub tab_width: usize,
    pub font_size: f32,
}

fn escape(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\r' | '\n' => {}
            c => out.push(c),
        }
    }
}

fn hex(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

fn luminance(c: Color) -> f32 {
    (0.299 * c.r as f32 + 0.587 * c.g as f32 + 0.114 * c.b as f32) / 255.0
}

/// Darkens a colour until it reads on white paper.
fn for_paper(c: Color) -> Color {
    let l = luminance(c);
    if l <= 0.55 {
        return c;
    }
    let k = 0.55 / l;
    let scale = |v: u8| (v as f32 * k).round() as u8;
    Color {
        r: scale(c.r),
        g: scale(c.g),
        b: scale(c.b),
        a: 255,
    }
}

/// Replaces tabs with spaces up to the next tab stop.
fn expand_tabs(line: &str, start_col: &mut usize, tab_width: usize) -> String {
    let mut out = String::with_capacity(line.len());
    for c in line.chars() {
        if c == '\t' {
            let n = tab_width - *start_col % tab_width;
            out.extend(std::iter::repeat_n(' ', n));
            *start_col += n;
        } else if c != '\r' && c != '\n' {
            out.push(c);
            *start_col += 1;
        }
    }
    out
}

/// The document as a complete HTML page.
pub fn to_html(
    text: &str,
    syntax: &SyntaxReference,
    syntaxes: &SyntaxSet,
    theme: &Theme,
    opts: &HtmlOptions,
) -> String {
    let settings = &theme.settings;
    let background = if opts.for_print {
        Color::WHITE
    } else {
        settings.background.unwrap_or(Color::WHITE)
    };
    let mut foreground = settings.foreground.unwrap_or(Color::BLACK);
    if opts.for_print {
        foreground = for_paper(foreground);
    }
    let gutter = if opts.for_print {
        Color {
            r: 0x88,
            g: 0x88,
            b: 0x88,
            a: 255,
        }
    } else {
        settings.gutter_foreground.unwrap_or(foreground)
    };
    let colours = opts.colours && text.len() <= MAX_COLOURED_BYTES;
    let lines = text.split_inclusive('\n').count().max(1);
    let digits = lines.to_string().len();

    let mut html = String::with_capacity(text.len() * 3 + 2048);
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>");
    escape(opts.title, &mut html);
    let _ = write!(
        html,
        "</title>\n<style>\n\
         body {{ margin: 0; padding: 12px 16px; background: {bg}; color: {fg}; }}\n\
         main {{ font: {size}px/1.35 'Cascadia Mono', 'DejaVu Sans Mono', Consolas, monospace; }}\n\
         .l {{ display: flex; white-space: pre-wrap; overflow-wrap: anywhere; }}\n\
         .n {{ flex: none; width: {num_w}ch; padding-right: 1.5ch; text-align: right; color: {gutter}; user-select: none; }}\n\
         .t {{ flex: 1; min-width: 0; }}\n\
         .b {{ font-weight: bold; }} .i {{ font-style: italic; }} .u {{ text-decoration: underline; }}\n\
         @media print {{ body {{ padding: 0; }} .l {{ break-inside: avoid; }} }}\n\
         </style>\n",
        bg = hex(background),
        fg = hex(foreground),
        size = opts.font_size,
        num_w = digits,
        gutter = hex(gutter),
    );
    if opts.for_print {
        html.push_str(
            "<script>window.addEventListener('load', () => setTimeout(() => window.print(), 200));</script>\n",
        );
    }
    html.push_str("</head>\n<body>\n<main>\n");

    let mut highlighter = colours.then(|| HighlightLines::new(syntax, theme));
    for (i, line) in text.split_inclusive('\n').enumerate() {
        html.push_str("<div class=\"l\">");
        if opts.line_numbers {
            let _ = write!(html, "<span class=\"n\">{}</span>", i + 1);
        }
        html.push_str("<span class=\"t\">");
        let mut col = 0;
        let pieces = highlighter
            .as_mut()
            .and_then(|h| h.highlight_line(line, syntaxes).ok());
        match pieces {
            Some(pieces) => {
                for (style, piece) in pieces {
                    let piece = expand_tabs(piece, &mut col, opts.tab_width);
                    if piece.is_empty() {
                        continue;
                    }
                    let mut colour = style.foreground;
                    if opts.for_print {
                        colour = for_paper(colour);
                    }
                    let mut classes = String::new();
                    if style.font_style.contains(FontStyle::BOLD) {
                        classes.push_str(" b");
                    }
                    if style.font_style.contains(FontStyle::ITALIC) {
                        classes.push_str(" i");
                    }
                    if style.font_style.contains(FontStyle::UNDERLINE) {
                        classes.push_str(" u");
                    }
                    if colour == foreground && classes.is_empty() {
                        escape(&piece, &mut html);
                        continue;
                    }
                    let _ = write!(html, "<span style=\"color:{}\"", hex(colour));
                    if !classes.is_empty() {
                        let _ = write!(html, " class=\"{}\"", classes.trim());
                    }
                    html.push('>');
                    escape(&piece, &mut html);
                    html.push_str("</span>");
                }
            }
            None => escape(&expand_tabs(line, &mut col, opts.tab_width), &mut html),
        }
        // Keeps empty lines their full height.
        if col == 0 {
            html.push(' ');
        }
        html.push_str("</span></div>\n");
    }
    html.push_str("</main>\n</body>\n</html>\n");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use syntect::highlighting::ThemeSet;

    fn opts(for_print: bool) -> HtmlOptions<'static> {
        HtmlOptions {
            title: "a <b> & c",
            line_numbers: true,
            colours: true,
            for_print,
            tab_width: 4,
            font_size: 12.0,
        }
    }

    #[test]
    fn writes_escaped_coloured_lines() {
        let ss = SyntaxSet::load_defaults_newlines();
        let themes = ThemeSet::load_defaults();
        let theme = &themes.themes["base16-eighties.dark"];
        let syntax = ss.find_syntax_by_name("Rust").unwrap();
        let html = to_html(
            "fn main() {\n\tlet x = 1 < 2;\n}\n",
            syntax,
            &ss,
            theme,
            &opts(true),
        );
        assert!(html.contains("<title>a &lt;b&gt; &amp; c</title>"));
        assert!(html.contains("<span class=\"n\">3</span>"));
        assert!(html.contains("&lt;"));
        assert!(html.contains("<span style=\"color:#"));
        assert!(html.contains("window.print()"));
        // The tab became four spaces.
        assert!(html.contains("<span class=\"t\">    "));
        let html = to_html("x\n", syntax, &ss, theme, &opts(false));
        assert!(!html.contains("window.print()"));
    }

    #[test]
    fn darkens_light_colours_for_paper() {
        let light = Color {
            r: 240,
            g: 240,
            b: 200,
            a: 255,
        };
        assert!(luminance(for_paper(light)) <= 0.56);
        let dark = Color {
            r: 20,
            g: 40,
            b: 60,
            a: 255,
        };
        assert_eq!(for_paper(dark), dark);
    }

    #[test]
    fn expands_tabs_to_tab_stops() {
        let mut col = 0;
        assert_eq!(expand_tabs("ab\tc\td", &mut col, 4), "ab  c   d");
        assert_eq!(col, 9);
    }
}
