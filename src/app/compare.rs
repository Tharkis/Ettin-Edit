//! Compare: starting a comparison of two tabs, a tab and its saved file, the
//! clipboard or the last commit, keeping it current while the tabs are
//! edited, and the Compare tab that shows it.

use egui::{CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use super::NixPadApp;
use crate::actions::Action;
use crate::compare_ui::{CompareAction, CompareView, Side, Source};
use crate::document::Document;
use crate::theme::Palette;

/// Seconds to wait after an edit before comparing again.
const RECOMPARE_DELAY: f64 = 0.3;
/// Seconds to wait for the clipboard text before giving up.
const CLIPBOARD_WAIT: f64 = 1.5;

impl NixPadApp {
    fn doc_side(&self, index: usize) -> Side {
        let doc = &self.docs[index];
        Side::new(
            Source::Doc(doc.id),
            doc.title(),
            &doc.editor.buffer.text(),
            doc.editor.buffer.version(),
        )
    }

    /// The non-empty selections of the tab active in `view`, in text order.
    fn selections_in(&self, view: usize) -> Vec<Side> {
        let Some(index) = self.active_in(view) else {
            return Vec::new();
        };
        let doc = &self.docs[index];
        let editor = &doc.editor;
        let sels = match editor.other_view_selections() {
            Some(other) if doc.live_view != view => other,
            _ => &editor.selections,
        };
        let mut ranges: Vec<_> = sels
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.range())
            .collect();
        ranges.sort_by_key(|r| r.start);
        ranges
            .into_iter()
            .map(|r| {
                let line = editor.buffer.char_to_line(r.start);
                let title = format!("{} (selection at line {})", doc.title(), line + 1);
                let mut side = Side::new(Source::Text, title, &editor.buffer.slice_string(r), 0);
                side.first_line = line;
                side
            })
            .collect()
    }

    /// Two selections to compare: the first two in the active tab, else the
    /// one in each view.
    fn two_selections(&self) -> Option<(Side, Side)> {
        let mut here = self.selections_in(self.split.focused);
        if here.len() >= 2 {
            let second = here.swap_remove(1);
            return Some((here.swap_remove(0), second));
        }
        if !self.is_split() {
            return None;
        }
        let left = self.selections_in(0).into_iter().next()?;
        let right = self.selections_in(1).into_iter().next()?;
        Some((left, right))
    }

    /// Shows a new comparison, replacing the one open.
    fn start_compare(&mut self, left: Side, right: Side) {
        let view = CompareView::new(left, right, self.settings.compare);
        if view.result.summary.is_empty() {
            self.notify_plain("The two sides are the same.");
        }
        self.compare = Some(view);
    }

    fn notify_plain(&mut self, text: &str) {
        self.notice = Some(crate::text_tools_ui::Notice {
            text: text.to_owned(),
            error: false,
            since: None,
            jump: None,
        });
    }

    fn notify_error(&mut self, text: String) {
        self.notice = Some(crate::text_tools_ui::Notice {
            text,
            error: true,
            since: None,
            jump: None,
        });
    }

    pub(super) fn run_compare(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::CompareTabs => {
                let pair = if self.is_split() {
                    self.active_in(0).zip(self.active_in(1))
                } else {
                    // The tab used before this one (else the tab next to it),
                    // then this one.
                    self.previous_tab
                        .and_then(|id| self.docs.iter().position(|d| d.id == id))
                        .filter(|&i| i != self.active)
                        .or_else(|| {
                            let id = self.neighbour_in(0, self.active)?;
                            self.docs.iter().position(|d| d.id == id)
                        })
                        .map(|i| (i.min(self.active), i.max(self.active)))
                };
                match pair {
                    Some((l, r)) if l != r => {
                        let (left, right) = (self.doc_side(l), self.doc_side(r));
                        self.start_compare(left, right);
                    }
                    _ => self.notify_error(
                        "Open a second tab to compare with, or pick one in Tools > Compare > Compare With.".into(),
                    ),
                }
            }
            Action::CompareWith(i) => {
                if i < self.docs.len() && i != self.active {
                    let (left, right) = (self.doc_side(self.active), self.doc_side(i));
                    self.start_compare(left, right);
                }
            }
            Action::CompareToSaved => {
                let doc = &self.docs[self.active];
                let Some(path) = doc.path.clone() else {
                    self.notify_error("This tab hasn't been saved yet.".into());
                    return;
                };
                match Document::open(0, &path, Some(doc.encoding)) {
                    Ok(saved) => {
                        let left = Side::new(
                            Source::Text,
                            format!("{} (saved)", doc.title()),
                            &saved.editor.buffer.text(),
                            0,
                        );
                        let right = self.doc_side(self.active);
                        self.start_compare(left, right);
                    }
                    Err(e) => self.notify_error(format!("Couldn't read the saved file: {e}")),
                }
            }
            Action::CompareToGitHead => {
                let doc = &self.docs[self.active];
                let Some(path) = doc.path.clone() else {
                    self.notify_error("This tab hasn't been saved yet.".into());
                    return;
                };
                let title = format!("{} (last commit)", doc.title());
                match crate::system::git_head_text(&path) {
                    Ok(text) => {
                        let left = Side::new(Source::Text, title, &text, 0);
                        let right = self.doc_side(self.active);
                        self.start_compare(left, right);
                    }
                    Err(e) => self.notify_error(e),
                }
            }
            Action::CompareSelections => match self.two_selections() {
                Some((left, right)) => self.start_compare(left, right),
                None => self.notify_error(
                    "Select two pieces of text: Ctrl+drag for a second selection, or select in each view."
                        .into(),
                ),
            },
            Action::CompareToClipboard => {
                // The text arrives as a paste event (see `take_clipboard`).
                self.clipboard_compare = Some(ctx.input(|i| i.time));
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                ctx.request_repaint();
            }
            Action::NextDifference | Action::PrevDifference => {
                if let Some(c) = &mut self.compare {
                    c.shown = true;
                    c.step(action == Action::NextDifference);
                }
            }
            Action::ShowComparison => {
                if let Some(c) = &mut self.compare {
                    c.shown = true;
                }
            }
            Action::CloseComparison => self.compare = None,
            Action::ExportComparePatch => self.export_patch(),
            Action::ToggleCompareOption(opt) => {
                self.settings.compare.toggle(opt);
                if let Some(c) = &mut self.compare {
                    c.opts = self.settings.compare;
                    c.recompute();
                }
            }
            _ => {}
        }
    }

    /// Picks up the clipboard text asked for by Compare to Clipboard, before
    /// the editor would paste it.
    pub(super) fn take_clipboard(&mut self, ctx: &egui::Context) {
        let Some(asked) = self.clipboard_compare else {
            return;
        };
        let text = ctx.input_mut(|i| {
            let pos = i
                .events
                .iter()
                .position(|e| matches!(e, egui::Event::Paste(_)))?;
            match i.events.remove(pos) {
                egui::Event::Paste(t) => Some(t),
                _ => None,
            }
        });
        if let Some(text) = text {
            self.clipboard_compare = None;
            if let Some(i) = self.docs.get(self.active).map(|_| self.active) {
                let left = Side::new(Source::Text, "Clipboard".into(), &text, 0);
                let right = self.doc_side(i);
                self.start_compare(left, right);
            }
        } else if ctx.input(|i| i.time) - asked > CLIPBOARD_WAIT {
            self.clipboard_compare = None;
            self.notify_error("The clipboard has no text to compare.".into());
        } else {
            ctx.request_repaint();
        }
    }

    fn export_patch(&mut self) {
        let Some(c) = &self.compare else {
            return;
        };
        let join = |lines: &[String]| {
            let mut t = lines.join("\n");
            t.push('\n');
            t
        };
        let patch = crate::compare::unified_patch(
            &join(&c.left.lines),
            &join(&c.right.lines),
            &c.left.title,
            &c.right.title,
        );
        self.new_tab();
        let doc = &mut self.docs[self.active];
        doc.editor = crate::editor::Editor::new(&patch);
        doc.editor.buffer.mark_modified();
        doc.syntax = "Diff".into();
    }

    /// Takes the latest text of compared tabs and compares again once
    /// editing pauses. A closed tab's side keeps its last text.
    fn refresh_compare(&mut self, ctx: &egui::Context) {
        let Some(c) = &mut self.compare else {
            return;
        };
        let now = ctx.input(|i| i.time);
        let mut changed = false;
        for side in [&mut c.left, &mut c.right] {
            let Source::Doc(id) = side.source else {
                continue;
            };
            match self.docs.iter().find(|d| d.id == id) {
                None => {
                    side.source = Source::Text;
                    side.title.push_str(" (closed)");
                }
                Some(doc) if doc.editor.buffer.version() != side.version => changed = true,
                _ => {}
            }
        }
        if changed {
            let since = *c.stale_since.get_or_insert(now);
            if now - since < RECOMPARE_DELAY {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(RECOMPARE_DELAY));
                return;
            }
            for side in [&mut c.left, &mut c.right] {
                if let Source::Doc(id) = side.source
                    && let Some(doc) = self.docs.iter().find(|d| d.id == id)
                {
                    side.lines = crate::compare::split_lines(&doc.editor.buffer.text());
                    side.version = doc.editor.buffer.version();
                }
            }
            c.recompute();
        }
    }

    /// Draws the comparison in place of the editors.
    pub(super) fn compare_area(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.refresh_compare(&ctx);
        let palette = self.palette();
        let font_size = self.settings.font_size;
        let Some(c) = &mut self.compare else {
            return;
        };
        let action = c.ui(ui, &palette, font_size);
        self.settings.compare = c.opts;
        match action {
            Some(CompareAction::Jump { doc, line }) => {
                if let Some(i) = self.docs.iter().position(|d| d.id == doc) {
                    self.select(i);
                    self.docs[i].editor.select_at(line, 0, 0);
                }
            }
            Some(CompareAction::Swap) => c.swap(),
            Some(CompareAction::ExportPatch) => self.export_patch(),
            Some(CompareAction::Close) => self.compare = None,
            None => {}
        }
    }

    /// The Compare tab at the start of the tab bar, while a comparison is open.
    pub(super) fn compare_tab(&mut self, ui: &mut Ui, height: f32, palette: &Palette) {
        let Some(c) = &self.compare else {
            return;
        };
        let shown = c.shown;
        let title = format!("Compare: {} | {}", c.left.title, c.right.title);
        let font = FontId::proportional(13.0);
        let color = if shown {
            palette.text
        } else {
            palette.text_dim
        };
        let galley = ui.painter().layout_no_wrap(title, font, color);
        let pad = 12.0;
        let close = 18.0;
        let width = (pad + galley.size().x + 8.0 + close + 8.0).min(360.0);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        let close_rect = Rect::from_center_size(
            Pos2::new(rect.right() - 8.0 - close / 2.0, rect.center().y),
            Vec2::splat(close),
        );
        let close_response = ui.interact(
            close_rect,
            ui.id().with("compare-tab-close"),
            Sense::click(),
        );
        let radius = CornerRadius {
            nw: 6,
            ne: 6,
            sw: 0,
            se: 0,
        };
        let painter = ui.painter();
        let bg = if shown {
            palette.editor_bg
        } else if response.hovered() {
            palette.tab_hover_bg
        } else {
            palette.tab_bg
        };
        painter.rect_filled(rect, radius, bg);
        if shown {
            painter.rect_filled(
                Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), 2.0)),
                radius,
                palette.accent,
            );
        }
        let text_clip =
            Rect::from_min_max(rect.min, Pos2::new(close_rect.left() - 4.0, rect.bottom()));
        painter.with_clip_rect(text_clip).galley(
            Pos2::new(rect.left() + pad, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
        if close_response.hovered() {
            painter.rect_filled(
                close_rect,
                CornerRadius::same(4),
                palette.tab_hover_bg.gamma_multiply(1.6),
            );
        }
        let c = close_rect.center();
        let d = 4.0;
        let stroke = Stroke::new(
            1.4,
            if close_response.hovered() {
                palette.text
            } else {
                palette.text_dim
            },
        );
        painter.line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], stroke);
        painter.line_segment([c + Vec2::new(-d, d), c + Vec2::new(d, -d)], stroke);
        if close_response.clicked() || response.middle_clicked() {
            self.pending.push(Action::CloseComparison);
        } else if response.clicked() {
            self.pending.push(Action::ShowComparison);
        }
    }
}
