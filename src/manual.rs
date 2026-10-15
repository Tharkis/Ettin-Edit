//! Help > User Manual: the manual in `docs/manual/src`, built into the app and
//! shown in a window with its contents, search and links. The same Markdown
//! is published as the web manual with mdBook (see `docs/manual/README.md`).

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontId, Id, RichText, Sense, Stroke, Ui, Vec2};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::theme::Palette;

include!(concat!(env!("OUT_DIR"), "/manual_pages.rs"));

/// Where the web version of the manual lives.
pub const WEB_URL: &str = "https://tharkis.github.io/Ettin-Edit/";

/// The page a web address opens for `file` (`files.md` is `files.html`).
pub fn web_url(file: &str) -> String {
    format!(
        "{WEB_URL}{}",
        file.trim_end_matches(".md").to_owned() + ".html"
    )
}

// -------------------------------------------------------------------------
// Parsing

#[derive(Clone, Copy, Default, PartialEq)]
struct Style {
    strong: bool,
    emph: bool,
    strike: bool,
    code: bool,
    kbd: bool,
}

#[derive(Clone)]
struct Span {
    text: String,
    style: Style,
    link: Option<String>,
}

type Inline = Vec<Span>;

enum Block {
    Heading {
        level: u8,
        text: Inline,
        anchor: String,
    },
    Paragraph(Inline),
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Code(String),
    Quote(Vec<Block>),
    Table {
        head: Vec<Inline>,
        rows: Vec<Vec<Inline>>,
    },
    Rule,
}

/// Collects text runs and their styles from inline events.
#[derive(Default)]
struct InlineBuilder {
    spans: Inline,
    strong: u32,
    emph: u32,
    strike: u32,
    kbd: bool,
    links: Vec<String>,
}

impl InlineBuilder {
    fn style(&self) -> Style {
        Style {
            strong: self.strong > 0,
            emph: self.emph > 0,
            strike: self.strike > 0,
            code: false,
            kbd: self.kbd,
        }
    }

    fn push(&mut self, text: &str, code: bool) {
        let mut style = self.style();
        style.code = code;
        let link = self.links.last().cloned();
        if let Some(last) = self.spans.last_mut()
            && last.style == style
            && last.link == link
        {
            last.text += text;
            return;
        }
        self.spans.push(Span {
            text: text.to_owned(),
            style,
            link,
        });
    }

    /// Handles an inline event; false for anything else.
    fn event(&mut self, event: &Event) -> bool {
        match event {
            Event::Start(Tag::Strong) => self.strong += 1,
            Event::End(TagEnd::Strong) => self.strong = self.strong.saturating_sub(1),
            Event::Start(Tag::Emphasis) => self.emph += 1,
            Event::End(TagEnd::Emphasis) => self.emph = self.emph.saturating_sub(1),
            Event::Start(Tag::Strikethrough) => self.strike += 1,
            Event::End(TagEnd::Strikethrough) => self.strike = self.strike.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => self.links.push(dest_url.to_string()),
            Event::End(TagEnd::Link) => {
                self.links.pop();
            }
            // Images are shown as their description.
            Event::Start(Tag::Image { .. }) => self.emph += 1,
            Event::End(TagEnd::Image) => self.emph = self.emph.saturating_sub(1),
            Event::Text(t) => self.push(t, self.kbd),
            Event::Code(t) => self.push(t, true),
            Event::SoftBreak => self.push(" ", false),
            Event::HardBreak => self.push("\n", false),
            Event::InlineHtml(html) => match html.trim() {
                "<kbd>" => self.kbd = true,
                "</kbd>" => self.kbd = false,
                _ => {}
            },
            _ => return false,
        }
        true
    }
}

/// Inline content up to the end of the block it's in.
fn inline<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> Inline {
    let mut b = InlineBuilder::default();
    for e in events.by_ref() {
        if !b.event(&e) && matches!(e, Event::End(_)) {
            break;
        }
    }
    b.spans
}

/// Blocks up to the end of the container they're in (or of the page).
fn blocks<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> Vec<Block> {
    let mut out = Vec::new();
    // Text directly inside a tight list item.
    let mut loose = InlineBuilder::default();
    let flush = |loose: &mut InlineBuilder, out: &mut Vec<Block>| {
        if !loose.spans.is_empty() {
            out.push(Block::Paragraph(std::mem::take(&mut loose.spans)));
        }
    };
    while let Some(e) = events.next() {
        if loose.event(&e) {
            continue;
        }
        match e {
            Event::End(_) => break,
            Event::Rule => {
                flush(&mut loose, &mut out);
                out.push(Block::Rule);
            }
            Event::Start(tag) => {
                flush(&mut loose, &mut out);
                match tag {
                    Tag::Paragraph => out.push(Block::Paragraph(inline(events))),
                    Tag::Heading { level, .. } => {
                        let text = inline(events);
                        let anchor = anchor_for(&plain(&text));
                        out.push(Block::Heading {
                            level: level as u8,
                            text,
                            anchor,
                        });
                    }
                    Tag::List(start) => {
                        let mut items = Vec::new();
                        while let Some(e) = events.next() {
                            match e {
                                Event::Start(Tag::Item) => items.push(blocks(events)),
                                Event::End(_) => break,
                                _ => {}
                            }
                        }
                        out.push(Block::List { start, items });
                    }
                    Tag::CodeBlock(_) => {
                        let mut text = String::new();
                        for e in events.by_ref() {
                            match e {
                                Event::Text(t) => text += &t,
                                Event::End(_) => break,
                                _ => {}
                            }
                        }
                        out.push(Block::Code(text.trim_end_matches('\n').to_owned()));
                    }
                    Tag::BlockQuote(_) => out.push(Block::Quote(blocks(events))),
                    Tag::Table(_) => out.push(table(events)),
                    // HTML blocks and anything else are skipped.
                    _ => skip(events),
                }
            }
            _ => {}
        }
    }
    flush(&mut loose, &mut out);
    out
}

/// Skips to the end of the element just started.
fn skip<'a>(events: &mut impl Iterator<Item = Event<'a>>) {
    let mut depth = 1;
    for e in events.by_ref() {
        match e {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
            _ => {}
        }
    }
}

fn table<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> Block {
    let mut head = Vec::new();
    let mut rows = Vec::new();
    let mut row: Vec<Inline> = Vec::new();
    while let Some(e) = events.next() {
        match e {
            Event::Start(Tag::TableCell) => row.push(inline(events)),
            Event::End(TagEnd::TableHead) => head = std::mem::take(&mut row),
            Event::End(TagEnd::TableRow) => rows.push(std::mem::take(&mut row)),
            Event::End(TagEnd::Table) => break,
            _ => {}
        }
    }
    Block::Table { head, rows }
}

fn parse(markdown: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    blocks(&mut Parser::new_ext(markdown, options))
}

fn plain(inline: &Inline) -> String {
    inline.iter().map(|s| s.text.as_str()).collect()
}

/// A heading's link target, made the way mdBook makes it, so `page.md#anchor`
/// links work in the app and on the web.
fn anchor_for(heading: &str) -> String {
    heading
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                Some(c.to_ascii_lowercase())
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

/// All the text of some blocks, for searching.
fn block_text(blocks: &[Block], out: &mut String) {
    for b in blocks {
        match b {
            Block::Heading { text, .. } | Block::Paragraph(text) => {
                out.push_str(&plain(text));
            }
            Block::List { items, .. } => items.iter().for_each(|i| block_text(i, out)),
            Block::Code(text) => out.push_str(text),
            Block::Quote(inner) => block_text(inner, out),
            Block::Table { head, rows } => {
                for cell in head.iter().chain(rows.iter().flatten()) {
                    out.push_str(&plain(cell));
                    out.push(' ');
                }
            }
            Block::Rule => {}
        }
        out.push('\n');
    }
}

// -------------------------------------------------------------------------
// The book

struct Page {
    file: &'static str,
    title: String,
    blocks: Vec<Block>,
    /// Every heading anchor on the page, in order.
    anchors: Vec<String>,
    /// The page's text in lower case, for searching.
    text: String,
}

/// A line of the contents: a part heading, or a link to a page.
struct Entry {
    title: String,
    page: Option<usize>,
    depth: usize,
}

struct Book {
    pages: Vec<Page>,
    contents: Vec<Entry>,
}

impl Book {
    fn load() -> Self {
        let mut pages: Vec<Page> = PAGES
            .iter()
            .map(|&(file, markdown)| {
                let mut blocks = parse(markdown);
                number_anchors(&mut blocks);
                let mut anchors = Vec::new();
                collect_anchors(&blocks, &mut anchors);
                let mut text = String::new();
                block_text(&blocks, &mut text);
                let title = blocks
                    .iter()
                    .find_map(|b| match b {
                        Block::Heading { text, .. } => Some(plain(text)),
                        _ => None,
                    })
                    .unwrap_or_else(|| file.to_owned());
                Page {
                    file,
                    title,
                    blocks,
                    anchors,
                    text: text.to_lowercase(),
                }
            })
            .collect();
        let mut contents = Vec::new();
        for line in SUMMARY.lines() {
            let trimmed = line.trim_start();
            if let Some(part) = trimmed.strip_prefix("# ") {
                if part.trim() != "Summary" {
                    contents.push(Entry {
                        title: part.trim().to_owned(),
                        page: None,
                        depth: 0,
                    });
                }
                continue;
            }
            let (Some(open), Some(mid)) = (trimmed.find('['), trimmed.find("](")) else {
                continue;
            };
            let file = trimmed[mid + 2..].trim_end().trim_end_matches(')');
            let Some(page) = pages.iter().position(|p| p.file == file) else {
                continue;
            };
            let indent = line.len() - trimmed.len();
            contents.push(Entry {
                title: trimmed[open + 1..mid].to_owned(),
                page: Some(page),
                depth: indent / 2 + usize::from(trimmed.starts_with('-')),
            });
            // The contents' name for a page wins over its first heading.
            pages[page].title = trimmed[open + 1..mid].to_owned();
        }
        Self { pages, contents }
    }

    fn page_index(&self, file: &str) -> Option<usize> {
        self.pages.iter().position(|p| p.file == file)
    }
}

/// Makes repeated headings' anchors unique the way mdBook does (`a`, `a-1`).
fn number_anchors(blocks: &mut [Block]) {
    let mut seen: std::collections::HashMap<String, usize> = Default::default();
    for b in blocks.iter_mut() {
        if let Block::Heading { anchor, .. } = b {
            let n = seen.entry(anchor.clone()).or_default();
            if *n > 0 {
                *anchor = format!("{anchor}-{n}");
            }
            *n += 1;
        }
    }
}

fn collect_anchors(blocks: &[Block], out: &mut Vec<String>) {
    for b in blocks {
        match b {
            Block::Heading { anchor, .. } => out.push(anchor.clone()),
            Block::Quote(inner) => collect_anchors(inner, out),
            Block::List { items, .. } => items.iter().for_each(|i| collect_anchors(i, out)),
            _ => {}
        }
    }
}

/// Where a link goes.
#[derive(Debug, PartialEq)]
enum Target {
    Page(String, Option<String>),
    Web(String),
}

fn target(href: &str) -> Target {
    if href.contains("://") || href.starts_with("mailto:") {
        return Target::Web(href.to_owned());
    }
    let href = href.trim_start_matches("./");
    match href.split_once('#') {
        Some((file, anchor)) => Target::Page(file.to_owned(), Some(anchor.to_owned())),
        None => Target::Page(href.to_owned(), None),
    }
}

// -------------------------------------------------------------------------
// The window

/// Where the reader is, and how they got there.
#[derive(Default)]
struct Nav {
    page: usize,
    back: Vec<usize>,
    forward: Vec<usize>,
    /// Scroll to this heading on the next frame.
    anchor: Option<String>,
    /// Scroll to the top on the next frame.
    to_top: bool,
    /// Scroll to the first search hit on the next frame.
    to_hit: bool,
    search: String,
}

#[derive(Default)]
pub struct ManualWindow {
    pub open: bool,
    book: Option<Book>,
    nav: Nav,
}

impl ManualWindow {
    /// Opens the manual at `file` (a page of the manual) and `anchor`.
    pub fn show(&mut self, file: &str, anchor: Option<&str>) {
        let book = self.book.get_or_insert_with(Book::load);
        let page = book.page_index(file).unwrap_or(0);
        if self.open && page != self.nav.page {
            self.nav.back.push(self.nav.page);
            self.nav.forward.clear();
        }
        self.nav.page = page;
        self.nav.anchor = anchor.map(str::to_owned);
        self.nav.to_top = anchor.is_none();
        self.open = true;
    }

    /// Draws the window. Returns an error to show, if opening a web page failed.
    pub fn ui(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<String> {
        if !self.open {
            return None;
        }
        let book = self.book.get_or_insert_with(Book::load);
        let nav = &mut self.nav;
        let mut open = true;
        let mut error = None;
        egui::Window::new("User Manual")
            .id(Id::new("user-manual"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_size([900.0, 640.0])
            .min_size([520.0, 300.0])
            .collapsible(false)
            .resizable(true)
            .open(&mut open)
            .show(ctx, |ui| {
                let mut go: Option<String> = None;
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!nav.back.is_empty(), egui::Button::new("←"))
                        .on_hover_text("Back")
                        .clicked()
                        && let Some(p) = nav.back.pop()
                    {
                        nav.forward.push(nav.page);
                        nav.page = p;
                        nav.to_top = true;
                    }
                    if ui
                        .add_enabled(!nav.forward.is_empty(), egui::Button::new("→"))
                        .on_hover_text("Forward")
                        .clicked()
                        && let Some(p) = nav.forward.pop()
                    {
                        nav.back.push(nav.page);
                        nav.page = p;
                        nav.to_top = true;
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut nav.search)
                            .id(Id::new("manual-search"))
                            .hint_text("Search the manual")
                            .desired_width(220.0),
                    );
                    if !nav.search.is_empty() && ui.small_button("×").clicked() {
                        nav.search.clear();
                    }
                    let file = book.pages[nav.page].file;
                    if ui
                        .button("Open on the Web")
                        .on_hover_text(web_url(file))
                        .clicked()
                        && let Err(e) = crate::system::open_url(&web_url(file))
                    {
                        error = Some(format!("Couldn't open the web manual: {e}"));
                    }
                });
                ui.separator();
                let terms: Vec<String> = nav
                    .search
                    .to_lowercase()
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect();
                // Panels share the window's space exactly, so the window keeps
                // the size the reader gave it.
                egui::Panel::left("manual-contents")
                    .resizable(true)
                    .default_size(210.0)
                    .min_size(140.0)
                    .frame(egui::Frame::new().inner_margin(egui::Margin {
                        left: 0,
                        right: 6,
                        top: 0,
                        bottom: 0,
                    }))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("manual-contents")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if terms.is_empty() {
                                    contents(ui, book, nav, palette);
                                } else {
                                    search_results(ui, book, nav, &terms);
                                }
                            });
                    });
                egui::CentralPanel::no_frame()
                    .frame(egui::Frame::new().inner_margin(egui::Margin {
                        left: 10,
                        right: 0,
                        top: 0,
                        bottom: 0,
                    }))
                    .show(ui, |ui| {
                        let page = &book.pages[nav.page];
                        let mut r = Render {
                            palette,
                            terms: &terms,
                            link: None,
                            nav,
                            id: 0,
                        };
                        let mut area = egui::ScrollArea::vertical()
                            .id_salt("manual-page")
                            .auto_shrink([false, false]);
                        if r.nav.to_top {
                            area = area.vertical_scroll_offset(0.0);
                            r.nav.to_top = false;
                        }
                        area.show(ui, |ui| {
                            ui.set_max_width(ui.available_width().min(760.0));
                            ui.spacing_mut().item_spacing.y = 8.0;
                            r.blocks(ui, &page.blocks, 0);
                            ui.add_space(24.0);
                        });
                        r.nav.to_hit = false;
                        go = r.link;
                    });
                if let Some(href) = go {
                    match target(&href) {
                        Target::Web(url) => {
                            if let Err(e) = crate::system::open_url(&url) {
                                error = Some(format!("Couldn't open {url}: {e}"));
                            }
                        }
                        Target::Page(file, anchor) => {
                            let page = if file.is_empty() {
                                Some(nav.page)
                            } else {
                                book.page_index(&file)
                            };
                            if let Some(page) = page {
                                if page != nav.page {
                                    nav.back.push(nav.page);
                                    nav.forward.clear();
                                    nav.page = page;
                                }
                                // A heading that isn't there opens the page at the top.
                                let anchor =
                                    anchor.filter(|a| book.pages[page].anchors.contains(a));
                                nav.to_top = anchor.is_none();
                                nav.anchor = anchor;
                            }
                        }
                    }
                }
            });
        self.open = open;
        error
    }
}

fn contents(ui: &mut Ui, book: &Book, nav: &mut Nav, palette: &Palette) {
    for entry in &book.contents {
        match entry.page {
            None => {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(entry.title.to_uppercase())
                        .small()
                        .color(palette.text_dim),
                );
            }
            Some(page) => {
                ui.horizontal(|ui| {
                    ui.add_space(entry.depth.saturating_sub(1) as f32 * 12.0);
                    if ui
                        .selectable_label(nav.page == page, &entry.title)
                        .clicked()
                        && nav.page != page
                    {
                        nav.back.push(nav.page);
                        nav.forward.clear();
                        nav.page = page;
                        nav.to_top = true;
                    }
                });
            }
        }
    }
}

fn search_results(ui: &mut Ui, book: &Book, nav: &mut Nav, terms: &[String]) {
    let mut hits: Vec<(usize, usize)> = book
        .pages
        .iter()
        .enumerate()
        .filter(|(_, p)| terms.iter().all(|t| p.text.contains(t.as_str())))
        .map(|(i, p)| {
            let count: usize = terms
                .iter()
                .map(|t| p.text.matches(t.as_str()).count())
                .sum();
            // Pages whose title matches come first.
            let title = p.title.to_lowercase();
            let bonus = if terms.iter().any(|t| title.contains(t.as_str())) {
                10_000
            } else {
                0
            };
            (i, count + bonus)
        })
        .collect();
    hits.sort_by_key(|&(i, n)| (std::cmp::Reverse(n), i));
    if hits.is_empty() {
        ui.label(RichText::new("Nothing matches").weak());
    }
    for (page, _) in hits {
        if ui
            .selectable_label(nav.page == page, &book.pages[page].title)
            .clicked()
        {
            if nav.page != page {
                nav.back.push(nav.page);
                nav.forward.clear();
                nav.page = page;
            }
            nav.to_hit = true;
            nav.to_top = false;
        }
    }
}

/// Draws a page.
struct Render<'a> {
    palette: &'a Palette,
    terms: &'a [String],
    /// A link the reader clicked.
    link: Option<String>,
    nav: &'a mut Nav,
    /// Counts widgets that need their own ids.
    id: usize,
}

const BODY: f32 = 14.5;
const CODE: f32 = 13.0;

impl Render<'_> {
    fn blocks(&mut self, ui: &mut Ui, blocks: &[Block], depth: usize) {
        for block in blocks {
            self.block(ui, block, depth);
        }
    }

    fn block(&mut self, ui: &mut Ui, block: &Block, depth: usize) {
        match block {
            Block::Heading {
                level,
                text,
                anchor,
            } => {
                let size = match level {
                    1 => 26.0,
                    2 => 20.0,
                    3 => 17.0,
                    _ => 15.0,
                };
                ui.add_space(if *level == 1 { 2.0 } else { 10.0 });
                let color = if *level <= 2 {
                    self.palette.accent
                } else {
                    strong_color(self.palette)
                };
                let rect = self.inline(ui, text, size, color);
                if *level <= 2 {
                    let y = rect.bottom() + 3.0;
                    ui.painter().hline(
                        rect.left()..=ui.max_rect().right(),
                        y,
                        Stroke::new(1.0, self.palette.border),
                    );
                    ui.add_space(4.0);
                }
                if self.nav.anchor.as_deref() == Some(anchor.as_str()) {
                    ui.scroll_to_rect(rect, Some(Align::TOP));
                    self.nav.anchor = None;
                }
            }
            Block::Paragraph(text) => {
                self.inline(ui, text, BODY, self.palette.text);
            }
            Block::List { start, items } => {
                for (i, item) in items.iter().enumerate() {
                    let marker = match start {
                        Some(n) => format!("{}.", n + i as u64),
                        None if depth.is_multiple_of(2) => "•".to_owned(),
                        None => "◦".to_owned(),
                    };
                    // The item goes in a box exactly as wide as the room
                    // left after the marker, so it never pushes wider.
                    let top = ui.cursor().min;
                    let inner = egui::Rect::from_min_size(
                        top + Vec2::new(22.0, 0.0),
                        Vec2::new((ui.available_width() - 22.0).max(0.0), f32::INFINITY),
                    );
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(inner)
                            .layout(egui::Layout::top_down(Align::Min)),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            self.blocks(ui, item, depth + 1);
                        },
                    );
                    ui.painter().text(
                        top + Vec2::new(16.0, 1.0),
                        egui::Align2::RIGHT_TOP,
                        marker,
                        FontId::proportional(BODY),
                        self.palette.text_dim,
                    );
                }
            }
            Block::Code(text) => self.code_block(ui, text),
            Block::Quote(inner) => {
                let response = egui::Frame::new()
                    .fill(self.palette.tab_bg)
                    .inner_margin(egui::Margin {
                        left: 14,
                        right: 10,
                        top: 8,
                        bottom: 8,
                    })
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.blocks(ui, inner, depth);
                    })
                    .response;
                let r = response.rect;
                ui.painter().vline(
                    r.left() + 2.0,
                    r.top()..=r.bottom(),
                    Stroke::new(3.0, self.palette.accent),
                );
            }
            Block::Table { head, rows } => self.table(ui, head, rows),
            Block::Rule => {
                ui.separator();
            }
        }
    }

    fn code_block(&mut self, ui: &mut Ui, text: &str) {
        self.id += 1;
        let id = self.id;
        egui::Frame::new()
            .fill(self.palette.tab_bg)
            .stroke(Stroke::new(1.0, self.palette.border))
            .inner_margin(8.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // The Copy button sits in the top-right corner, outside the
                // layout, so the block never asks for more width.
                let copy_w = 56.0;
                let corner = ui.max_rect().right_top();
                egui::ScrollArea::horizontal()
                    .id_salt(("manual-code", id))
                    .max_width((ui.available_width() - copy_w).max(0.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let mut job = LayoutJob::default();
                        job.append(
                            text,
                            0.0,
                            TextFormat::simple(FontId::monospace(CODE), self.palette.editor_fg),
                        );
                        job.wrap.max_width = f32::INFINITY;
                        ui.add(egui::Label::new(job).selectable(true));
                    });
                let button = egui::Rect::from_min_max(
                    corner + Vec2::new(-copy_w + 8.0, 0.0),
                    corner + Vec2::new(0.0, 18.0),
                );
                if ui
                    .place(button, egui::Button::new(RichText::new("Copy").small()))
                    .on_hover_text("Copy this to the clipboard")
                    .clicked()
                {
                    ui.ctx().copy_text(text.to_owned());
                }
            });
    }

    fn table(&mut self, ui: &mut Ui, head: &[Inline], rows: &[Vec<Inline>]) {
        self.id += 1;
        let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
        if columns == 0 {
            return;
        }
        // Columns share the width by how much text they hold.
        let mut weight = vec![1.0f32; columns];
        for row in std::iter::once(head).chain(rows.iter().map(Vec::as_slice)) {
            for (i, cell) in row.iter().enumerate() {
                let len = plain(cell).chars().count() as f32;
                weight[i] = weight[i].max(len.min(60.0));
            }
        }
        // The table is painted by hand at the width there is, so it never
        // asks for more room and grows the window.
        let gap = 14.0;
        let pad = 4.0;
        let full = ui.available_width();
        let total = (full - gap * (columns as f32 - 1.0) - 2.0 * pad).max(0.0);
        let sum: f32 = weight.iter().sum();
        let mut widths: Vec<f32> = weight.iter().map(|w| (total * w / sum).max(48.0)).collect();
        let used: f32 = widths.iter().sum();
        if used > total && used > 0.0 {
            widths.iter_mut().for_each(|w| *w *= total / used);
        }
        let all = std::iter::once((true, head)).chain(rows.iter().map(|r| (false, r.as_slice())));
        for (n, (is_head, row)) in all.enumerate() {
            let color = if is_head {
                strong_color(self.palette)
            } else {
                self.palette.text
            };
            let cells: Vec<Laid> = (0..columns)
                .map(|i| {
                    let empty = Inline::new();
                    let cell = row.get(i).unwrap_or(&empty);
                    self.lay_out(ui, cell, BODY - 0.5, color, widths[i])
                })
                .collect();
            let height = cells.iter().map(|c| c.galley.size().y).fold(0.0, f32::max) + 6.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(full, height), Sense::hover());
            if is_head {
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom(),
                    Stroke::new(1.0, self.palette.border),
                );
            } else if n % 2 == 0 {
                ui.painter().rect_filled(rect, 2.0, self.palette.tab_bg);
            }
            let mut x = rect.left() + pad;
            for (i, cell) in cells.into_iter().enumerate() {
                let at =
                    egui::Rect::from_min_size(egui::pos2(x, rect.top() + 3.0), cell.galley.size());
                let id = Id::new(("manual-cell", self.id, n, i));
                let response = ui.interact(at, id, Sense::click());
                self.paint(ui, cell, at, &response);
                x += widths[i] + gap;
            }
        }
    }

    /// Lays out and draws a run of styled text, wrapped to the width there
    /// is. Links in it can be clicked.
    fn inline(&mut self, ui: &mut Ui, text: &Inline, size: f32, color: Color32) -> egui::Rect {
        let laid = self.lay_out(ui, text, size, color, ui.available_width());
        let (rect, response) = ui.allocate_exact_size(laid.galley.size(), Sense::click());
        self.paint(ui, laid, rect, &response);
        rect
    }

    /// Wraps a run of styled text to a width.
    fn lay_out(&self, ui: &Ui, text: &Inline, size: f32, color: Color32, width: f32) -> Laid {
        let strong = strong_color(self.palette);
        let mut job = LayoutJob::default();
        // (char range, link) of each link.
        let mut links: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        let mut chars = 0;
        let mut hit = false;
        for span in text {
            let s = span.style;
            let mut format = if s.code || s.kbd {
                let mut f = TextFormat::simple(FontId::monospace(size - 1.5), self.palette.text);
                f.background = self.palette.line_highlight;
                if s.kbd {
                    f.color = strong;
                }
                f
            } else {
                TextFormat::simple(FontId::proportional(size), color)
            };
            if s.strong && !s.code {
                format.color = strong;
            }
            format.italics = s.emph;
            if s.strike {
                format.strikethrough = Stroke::new(1.0, format.color);
            }
            if span.link.is_some() {
                format.color = self.palette.accent;
                format.underline = Stroke::new(1.0, self.palette.accent.gamma_multiply(0.5));
            }
            let n = span.text.chars().count();
            if let Some(link) = &span.link {
                links.push((chars..chars + n, link.clone()));
            }
            chars += n;
            // Search hits get the find colour.
            let ranges = hits(&span.text, self.terms);
            hit |= !ranges.is_empty();
            let mut at = 0;
            for r in ranges {
                job.append(&span.text[at..r.start], 0.0, format.clone());
                let mut marked = format.clone();
                marked.background = self.palette.find_hit;
                job.append(&span.text[r.clone()], 0.0, marked);
                at = r.end;
            }
            job.append(&span.text[at..], 0.0, format);
        }
        job.wrap.max_width = width;
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        Laid {
            galley,
            links,
            hit,
            color,
        }
    }

    /// Draws laid-out text in a rect, following a click on a link in it.
    fn paint(&mut self, ui: &mut Ui, laid: Laid, rect: egui::Rect, response: &egui::Response) {
        let Laid {
            galley,
            links,
            hit,
            color,
        } = laid;
        if !links.is_empty()
            && let Some(pos) = response.hover_pos()
        {
            let index = galley.cursor_from_pos(pos - rect.min).index.0;
            let over = links
                .iter()
                .find(|(r, _)| r.contains(&index) || (index > 0 && r.end == index));
            if let Some((_, href)) = over {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                if response.clicked() {
                    self.link = Some(href.clone());
                }
                if href.contains("://") {
                    response.clone().on_hover_text(href.as_str());
                }
            }
        }
        ui.painter().galley(rect.min, galley, color);
        if hit && self.nav.to_hit {
            ui.scroll_to_rect(rect, Some(Align::Center));
            self.nav.to_hit = false;
        }
    }
}

/// Text wrapped and ready to draw.
struct Laid {
    galley: std::sync::Arc<egui::Galley>,
    /// (char range, link) of each link.
    links: Vec<(std::ops::Range<usize>, String)>,
    /// Whether it holds a search hit.
    hit: bool,
    color: Color32,
}

/// Bold text: there's no bold font, so it stands out by being brighter
/// (darker on a light theme).
fn strong_color(palette: &Palette) -> Color32 {
    let to = if palette.dark {
        Color32::WHITE
    } else {
        Color32::BLACK
    };
    let t = palette.text;
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * 0.65).round() as u8;
    Color32::from_rgb(mix(t.r(), to.r()), mix(t.g(), to.g()), mix(t.b(), to.b()))
}

/// Byte ranges in `text` where any of `terms` (lower case) is found.
fn hits(text: &str, terms: &[String]) -> Vec<std::ops::Range<usize>> {
    if terms.is_empty() {
        return Vec::new();
    }
    let lower = text.to_lowercase();
    // Lower-casing can change lengths outside ASCII; then don't mark.
    if lower.len() != text.len() {
        return Vec::new();
    }
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    for t in terms {
        let mut from = 0;
        while let Some(i) = lower[from..].find(t.as_str()) {
            let start = from + i;
            ranges.push(start..start + t.len());
            from = start + t.len();
        }
    }
    ranges.sort_by_key(|r| r.start);
    // Merge overlaps, and keep ranges on character boundaries.
    let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
    for r in ranges {
        if !text.is_char_boundary(r.start) || !text.is_char_boundary(r.end) {
            continue;
        }
        match merged.last_mut() {
            Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
            _ => merged.push(r),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every page in docs/manual/src is in SUMMARY.md, so it's in the app
    /// and on the web.
    #[test]
    fn every_page_is_in_the_contents() {
        let book = Book::load();
        assert!(book.pages.len() > 10);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/manual/src");
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if name.ends_with(".md") && name != "SUMMARY.md" {
                assert!(
                    book.page_index(&name).is_some(),
                    "docs/manual/src/{name} is not in SUMMARY.md"
                );
            }
        }
        for page in &book.pages {
            assert!(
                book.contents
                    .iter()
                    .any(|e| e.page.is_some_and(|i| book.pages[i].file == page.file)),
                "{} is not in the contents",
                page.file
            );
        }
    }

    fn links(blocks: &[Block], out: &mut Vec<String>) {
        let inline_links = |text: &Inline, out: &mut Vec<String>| {
            out.extend(text.iter().filter_map(|s| s.link.clone()));
        };
        for b in blocks {
            match b {
                Block::Heading { text, .. } | Block::Paragraph(text) => inline_links(text, out),
                Block::List { items, .. } => items.iter().for_each(|i| links(i, out)),
                Block::Quote(inner) => links(inner, out),
                Block::Table { head, rows } => {
                    for cell in head.iter().chain(rows.iter().flatten()) {
                        inline_links(cell, out);
                    }
                }
                Block::Code(_) | Block::Rule => {}
            }
        }
    }

    /// Every link between pages leads to a page, and to a heading on it.
    #[test]
    fn links_between_pages_work() {
        let book = Book::load();
        let mut broken = Vec::new();
        for page in &book.pages {
            let mut found = Vec::new();
            links(&page.blocks, &mut found);
            for href in found {
                let Target::Page(file, anchor) = target(&href) else {
                    continue;
                };
                let to = if file.is_empty() {
                    Some(page)
                } else {
                    book.page_index(&file).map(|i| &book.pages[i])
                };
                match (to, anchor) {
                    (None, _) => broken.push(format!("{}: {href}", page.file)),
                    (Some(to), Some(a)) if !to.anchors.contains(&a) => {
                        broken.push(format!("{}: {href} (no such heading)", page.file))
                    }
                    _ => {}
                }
            }
        }
        assert!(broken.is_empty(), "broken links:\n{}", broken.join("\n"));
    }

    /// The shortcut reference lists every built-in shortcut.
    #[test]
    fn shortcut_reference_is_complete() {
        let page = PAGES
            .iter()
            .find(|(f, _)| *f == "shortcuts.md")
            .expect("shortcuts.md")
            .1;
        let mut missing = Vec::new();
        for (shortcut, action) in crate::actions::default_shortcuts()
            .into_iter()
            .chain(crate::actions::editor_shortcuts())
        {
            // Written the way people read them: Ctrl+, rather than Ctrl+Comma.
            let text = crate::plugins::shortcut::to_text(&shortcut)
                .replace("Comma", ",")
                .replace("Equals", "=")
                .replace("Minus", "-");
            if !page.contains(&format!("<kbd>{text}</kbd>")) {
                missing.push(format!("{text} ({action:?})"));
            }
        }
        assert!(
            missing.is_empty(),
            "docs/manual/src/shortcuts.md is missing:\n{}",
            missing.join("\n")
        );
    }

    #[test]
    fn anchors_match_mdbook() {
        assert_eq!(anchor_for("Find in Files"), "find-in-files");
        assert_eq!(anchor_for("JSON tree (Ctrl+Alt+J)"), "json-tree-ctrlaltj");
        assert_eq!(
            anchor_for("User-defined languages (.lang.json)"),
            "user-defined-languages-langjson"
        );
    }

    #[test]
    fn parses_blocks() {
        let blocks = parse(
            "# Title\n\nSome **bold** and [a link](files.md#top).\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```\ncode\n```\n\n> note\n",
        );
        assert!(matches!(&blocks[0], Block::Heading { level: 1, anchor, .. } if anchor == "title"));
        let Block::Paragraph(p) = &blocks[1] else {
            panic!()
        };
        assert!(p.iter().any(|s| s.style.strong && s.text == "bold"));
        assert!(p.iter().any(|s| s.link.as_deref() == Some("files.md#top")));
        assert!(matches!(&blocks[2], Block::List { items, .. } if items.len() == 2));
        assert!(
            matches!(&blocks[3], Block::Table { head, rows } if head.len() == 2 && rows.len() == 1)
        );
        assert!(matches!(&blocks[4], Block::Code(c) if c == "code"));
        assert!(matches!(&blocks[5], Block::Quote(_)));
    }

    #[test]
    fn finds_hits() {
        assert_eq!(hits("Find in Files", &["fi".into()]), vec![0..2, 8..10]);
        assert!(hits("abc", &[]).is_empty());
    }

    #[test]
    fn web_addresses() {
        assert_eq!(web_url("files.md"), format!("{WEB_URL}files.html"));
        assert_eq!(
            target("https://example.com"),
            Target::Web("https://example.com".into())
        );
        assert_eq!(
            target("search.md#regex"),
            Target::Page("search.md".into(), Some("regex".into()))
        );
    }
}
