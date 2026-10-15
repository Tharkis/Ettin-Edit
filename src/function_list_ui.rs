//! The function list panel (View > Function List): the current document's
//! functions, classes and headings, to jump to with a click.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use egui::{Color32, RichText, ScrollArea, Ui};

use crate::function_list::{Symbol, SymbolKind, symbols};
use crate::theme::Palette;

/// Seconds to wait after an edit before reading the document again.
const REFRESH_DELAY: f64 = 0.4;

/// A document's id and text version.
type DocVersion = (u64, u64);

#[derive(Default)]
pub struct FunctionList {
    /// The document and version the list was made from.
    source: Option<DocVersion>,
    symbols: Vec<Symbol>,
    language: String,
    filter: String,
    /// A list being made in the background, and what it's for.
    pending: Option<(DocVersion, Receiver<Vec<Symbol>>)>,
    /// When the document last changed, to wait for typing to pause.
    changed_at: f64,
    /// Scroll the caret's symbol into view on the next draw.
    follow: bool,
}

fn badge(kind: SymbolKind) -> (&'static str, Color32) {
    match kind {
        SymbolKind::Function => ("ƒ", Color32::from_rgb(0x6c, 0xb6, 0xff)),
        SymbolKind::Method => ("m", Color32::from_rgb(0x6c, 0xb6, 0xff)),
        SymbolKind::Class => ("C", Color32::from_rgb(0xf0, 0xb4, 0x4c)),
        SymbolKind::Struct => ("S", Color32::from_rgb(0xf0, 0xb4, 0x4c)),
        SymbolKind::Enum => ("E", Color32::from_rgb(0xf0, 0xb4, 0x4c)),
        SymbolKind::Trait => ("I", Color32::from_rgb(0x9c, 0xd2, 0x7c)),
        SymbolKind::Module => ("M", Color32::from_rgb(0xc5, 0x8a, 0xf0)),
        SymbolKind::Section => ("§", Color32::from_rgb(0xc5, 0x8a, 0xf0)),
        SymbolKind::Heading => ("#", Color32::from_rgb(0xc5, 0x8a, 0xf0)),
        SymbolKind::Key => ("k", Color32::from_rgb(0x9a, 0xa5, 0xb1)),
    }
}

impl FunctionList {
    /// Keeps the list in step with the document: `doc` is its id and text
    /// version, and `text` is called only when the list needs remaking.
    /// Returns true while a list is being made.
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        doc: DocVersion,
        language: &str,
        text: impl FnOnce() -> Arc<String>,
    ) -> bool {
        let now = ctx.input(|i| i.time);
        if let Some((for_doc, rx)) = &self.pending
            && let Ok(list) = rx.try_recv()
        {
            let switched = self.source.is_none_or(|(id, _)| id != for_doc.0);
            self.source = Some(*for_doc);
            self.symbols = list;
            self.pending = None;
            self.follow = switched;
        }
        let pending_for = self.pending.as_ref().map(|(d, _)| *d);
        if self.source == Some(doc) || pending_for == Some(doc) {
            return self.pending.is_some();
        }
        // Another document reads at once; an edit waits for typing to pause.
        let same_doc = self.source.is_some_and(|(id, _)| id == doc.0);
        if same_doc && self.language == language {
            if self.changed_at == 0.0 {
                self.changed_at = now;
            }
            if now - self.changed_at < REFRESH_DELAY {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(REFRESH_DELAY));
                return false;
            }
        }
        self.changed_at = 0.0;
        self.language = language.to_owned();
        let text = text();
        let language = language.to_owned();
        let (tx, rx) = channel();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(symbols(&language, &text));
            repaint.request_repaint();
        });
        self.pending = Some((doc, rx));
        true
    }

    /// Draws the list. `caret_line` marks the symbol the caret is in; returns
    /// the line of a symbol that was clicked.
    pub fn ui(&mut self, ui: &mut Ui, caret_line: usize, palette: &Palette) -> Option<usize> {
        let mut jump = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("function-list")).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("⊙")
                    .on_hover_text(t!("show-caret-symbol"))
                    .clicked()
                {
                    self.follow = true;
                }
            });
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .hint_text(t!("filter-hint"))
                .desired_width(f32::INFINITY),
        );
        ui.add_space(2.0);
        let filter = self.filter.to_lowercase();
        // The symbol the caret is in: the last one that starts at or above it.
        let current = self.symbols.iter().rposition(|s| s.line <= caret_line);
        ScrollArea::both()
            .id_salt("function-list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                if self.symbols.is_empty() {
                    let text = if self.pending.is_some() {
                        t!("reading-document")
                    } else {
                        t!("nothing-found-in-document")
                    };
                    ui.label(RichText::new(text).weak());
                }
                for (i, symbol) in self.symbols.iter().enumerate() {
                    if !filter.is_empty() && !symbol.name.to_lowercase().contains(&filter) {
                        continue;
                    }
                    let is_current = current == Some(i);
                    let depth = if filter.is_empty() { symbol.depth } else { 0 };
                    let response = ui
                        .horizontal(|ui| {
                            ui.add_space(depth as f32 * 12.0);
                            let (letter, color) = badge(symbol.kind);
                            ui.label(RichText::new(letter).monospace().color(color))
                                .on_hover_text(format!("{:?}", symbol.kind));
                            let text = RichText::new(&symbol.name).color(if is_current {
                                palette.accent
                            } else {
                                palette.text
                            });
                            ui.add(
                                egui::Button::selectable(is_current, text)
                                    .frame_when_inactive(false)
                                    .truncate(),
                            )
                            .on_hover_text(t!("line-number", line = symbol.line + 1))
                        })
                        .inner;
                    if is_current && std::mem::take(&mut self.follow) {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if response.clicked() {
                        jump = Some(symbol.line);
                    }
                }
            });
        jump
    }
}
