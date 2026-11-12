//! The Transform menu's dialogs: Filter Through Command, Keep or Remove
//! Matching Lines, Add Prefix and Suffix, Number Lines and Reflow.
//!
//! Each one shows what it will do before it does it (green and red lines in
//! the editor, or Input and Output panes) and asks where the result goes:
//! this tab, a new tab or the clipboard.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};

use egui::{Align, Color32, Id, Key, Layout, Margin, RichText, Ui, Vec2};
use serde::{Deserialize, Serialize};

use crate::theme::Palette;
use crate::tools::pipe::RunError;
use crate::tools::fields::{FieldKind, FieldOp};
use crate::tools::transform::{self, LineMatcher, NumberPad, TransformOp};

/// Which dialog a Transform menu item opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogKind {
    Pipe,
    FilterLines { keep: bool },
    AffixLines,
    NumberLines,
    Reflow,
    /// A Columns and Fields dialog (Ettin Edit 2.6).
    Field(FieldKind),
}

impl DialogKind {
    /// The name its Result goes to choice is remembered under in settings.
    pub fn key(self) -> &'static str {
        match self {
            DialogKind::Pipe => "filter-through-command",
            DialogKind::FilterLines { .. } => "matching-lines",
            DialogKind::AffixLines => "prefix-suffix",
            DialogKind::NumberLines => "number-lines",
            DialogKind::Reflow => "reflow",
            DialogKind::Field(kind) => kind.keys().1,
        }
    }

    /// Where the result goes before the user has picked: a new tab for
    /// shell commands, so a surprising command can't spoil the text.
    pub fn default_dest(self) -> Dest {
        match self {
            // Pulling matches out shouldn't throw the rest of the text away.
            DialogKind::Pipe | DialogKind::Field(FieldKind::Extract) => Dest::NewTab,
            _ => Dest::ThisTab,
        }
    }
}

/// Where a transform's result goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dest {
    /// Replace the text it was run on.
    #[default]
    ThisTab,
    /// Open the result in a new tab; this tab is left alone.
    NewTab,
    /// Copy the result; the tab is left alone.
    Clipboard,
}

/// What the dialog asks the app to do.
pub enum DialogAction {
    Run(TransformOp, Dest),
    /// Stop the running command.
    Stop,
}

/// The text a dialog previews and the tab it comes from.
pub struct Source<'a> {
    /// Changes whenever the text or the selection changes.
    pub key: u64,
    /// The lines the dialog works on, one selection after another.
    pub text: &'a dyn Fn() -> String,
    /// How many lines that is.
    pub lines: usize,
    /// True when it is a selection rather than the whole tab.
    pub selection: bool,
    /// Where a command runs, and the file's path for `$ETTIN_FILE`.
    pub dir: Option<PathBuf>,
    pub file: Option<PathBuf>,
}

/// How many recent commands Filter Through Command remembers.
pub const MAX_COMMANDS: usize = 15;
/// The other dialogs preview this many lines.
pub const PREVIEW_LINES: usize = 200;
/// Keep or Remove Matching Lines waits this long after typing stops.
const DEBOUNCE: f64 = 0.15;
/// Above this many lines the match count is worked out in the background.
pub const BACKGROUND_LINES: usize = 50_000;

/// A filter's pattern, regex, match-case and keep settings.
type FilterKey = (String, bool, bool, bool);
/// What a match count is for: the text (by source key) and the filter.
type CountKey = (u64, FilterKey);

/// A finished command preview.
struct PipeResult {
    /// The command and the text it ran on.
    command: String,
    source: u64,
    input: String,
    output: Result<String, RunError>,
    seconds: f64,
}

/// A command preview that is still running.
struct PipePreviewJob {
    command: String,
    source: u64,
    input: String,
    cancel: Arc<AtomicBool>,
    rx: Receiver<(Result<String, RunError>, f64)>,
}

pub struct TransformDialog {
    pub kind: Option<DialogKind>,
    focus: bool,
    pub error: Option<String>,
    /// The command running now, while Filter Through Command waits for it.
    pub running: Option<String>,
    dest: Dest,
    command: String,
    pattern: String,
    regex: bool,
    match_case: bool,
    keep: bool,
    /// The settings being typed, and when they last changed (egui time).
    typed: Option<(FilterKey, f64)>,
    /// The settings the editor's marks and the count show, and the matcher.
    shown: Option<(FilterKey, Result<Arc<LineMatcher>, String>)>,
    /// The match count: (source key, filter) and result.
    count: Option<(CountKey, (usize, usize))>,
    /// A count running in the background, for big files.
    counting: Option<(CountKey, Receiver<(usize, usize)>)>,
    prefix: String,
    suffix: String,
    skip_empty: bool,
    start: String,
    step: String,
    separator: String,
    pad: NumberPad,
    width: String,
    /// The Columns and Fields settings, one for each dialog, kept while
    /// Ettin Edit runs.
    fields: Vec<FieldOp>,
    /// The live preview of the line dialogs: what it was worked out for,
    /// the input and the output (or why the settings are wrong).
    live: Option<(u64, String, Result<String, String>)>,
    pipe_result: Option<PipeResult>,
    pipe_job: Option<PipePreviewJob>,
}

impl Default for TransformDialog {
    fn default() -> Self {
        Self {
            kind: None,
            focus: false,
            error: None,
            running: None,
            dest: Dest::ThisTab,
            command: String::new(),
            pattern: String::new(),
            regex: false,
            match_case: false,
            keep: true,
            typed: None,
            shown: None,
            count: None,
            counting: None,
            prefix: String::new(),
            suffix: String::new(),
            skip_empty: true,
            start: "1".into(),
            step: "1".into(),
            separator: ". ".into(),
            pad: NumberPad::None,
            width: "80".into(),
            fields: FieldKind::ALL.iter().map(|k| k.default_op()).collect(),
            live: None,
            pipe_result: None,
            pipe_job: None,
        }
    }
}

fn field(ui: &mut Ui, label: &str, value: &mut String, width: f32) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 20.0], egui::Label::new(label));
        ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(width)
                .margin(Margin::symmetric(6, 3)),
        )
    })
    .inner
}

/// A read-only box of text for the Input and Output panes.
pub(crate) fn pane(
    ui: &mut Ui,
    id: &str,
    title: &str,
    text: &str,
    color: Option<Color32>,
    palette: &Palette,
) {
    ui.vertical(|ui| {
        ui.label(RichText::new(title).size(12.0).color(palette.text_dim));
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .inner_margin(Margin::same(4))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::both()
                    .id_salt(id)
                    .max_height(130.0)
                    .min_scrolled_height(130.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut text = RichText::new(text).monospace().size(12.0);
                        if let Some(c) = color {
                            text = text.color(c);
                        }
                        // Text keeps its lines; messages (in colour) wrap.
                        if color.is_some() {
                            ui.add(egui::Label::new(text).wrap());
                        } else {
                            ui.add(egui::Label::new(text).extend());
                        }
                    });
            });
    });
}

/// The first `n` lines of `text`.
pub(crate) fn first_lines(text: &str, n: usize) -> &str {
    match text.match_indices('\n').nth(n.saturating_sub(1)) {
        Some((i, _)) if n > 0 => &text[..i],
        _ => text,
    }
}

impl TransformDialog {
    /// Opens a dialog; `width` is the long-line column, for Reflow, and
    /// `dest` where its result went last time.
    pub fn show_dialog(&mut self, kind: DialogKind, width: usize, dest: Option<Dest>) {
        if self.running.is_some() {
            return;
        }
        if let DialogKind::FilterLines { keep } = kind {
            self.keep = keep;
        }
        if kind == DialogKind::Reflow {
            self.width = width.to_string();
        }
        self.kind = Some(kind);
        self.dest = dest.unwrap_or(kind.default_dest());
        self.focus = true;
        self.error = None;
        self.typed = None;
        self.shown = None;
        self.count = None;
        self.counting = None;
        self.live = None;
        self.pipe_result = None;
        self.cancel_preview();
    }

    /// Puts `pattern` in Keep or Remove Matching Lines, as the Welcome
    /// tab's guided tour does.
    pub fn set_pattern(&mut self, pattern: &str) {
        self.pattern = pattern.to_owned();
        self.regex = false;
        self.match_case = false;
    }

    pub fn is_open(&self) -> bool {
        self.kind.is_some()
    }

    pub fn close(&mut self) {
        self.kind = None;
        self.running = None;
        self.cancel_preview();
    }

    fn cancel_preview(&mut self) {
        if let Some(job) = self.pipe_job.take() {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// The matcher and keep setting for the editor's green and red lines,
    /// while Keep or Remove Matching Lines is open with a valid pattern.
    pub fn line_preview(&self) -> Option<(Arc<LineMatcher>, bool)> {
        if !matches!(self.kind, Some(DialogKind::FilterLines { .. })) {
            return None;
        }
        match &self.shown {
            Some(((pattern, _, _, keep), Ok(m))) if !pattern.is_empty() => Some((m.clone(), *keep)),
            _ => None,
        }
    }

    /// Whether the last command preview failed, so its output can't
    /// replace the text.
    fn preview_failed(&self) -> bool {
        self.pipe_result
            .as_ref()
            .is_some_and(|r| r.command == self.command.trim() && r.output.is_err())
    }

    /// Draws the open dialog. `source` is the text it works on and
    /// `history` the recent commands.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        palette: &Palette,
        history: &[String],
        source: &Source,
        editor_rect: Option<egui::Rect>,
    ) -> Option<DialogAction> {
        let kind = self.kind?;
        let mut result = None;
        let mut close = false;
        let mut enter = false;
        self.poll(ctx, source);
        let mut body = |ui: &mut Ui| {
            ui.set_width(match kind {
                DialogKind::Pipe => 640.0,
                DialogKind::FilterLines { .. } => 440.0,
                _ => 600.0,
            });
            let (title, hint) = match kind {
                DialogKind::Pipe => (t!("pipe-title"), t!("pipe-hint")),
                DialogKind::FilterLines { .. } => {
                    (t!("filter-lines-title"), t!("filter-lines-hint"))
                }
                DialogKind::AffixLines => (t!("affix-lines-title"), t!("affix-lines-hint")),
                DialogKind::NumberLines => (t!("number-lines-title"), t!("number-lines-hint")),
                DialogKind::Reflow => (t!("reflow-title"), t!("reflow-hint")),
                DialogKind::Field(k) => (
                    crate::i18n::tr(k.keys().1),
                    crate::i18n::tr(crate::fields_ui::hint_key(k)),
                ),
            };
            ui.heading(title);
            ui.add_space(4.0);
            ui.label(RichText::new(hint).size(12.0).color(palette.text_dim));
            ui.add_space(6.0);
            let first = match kind {
                DialogKind::Pipe => self.pipe_ui(ui, palette, history, source),
                DialogKind::FilterLines { .. } => self.filter_ui(ui, palette),
                DialogKind::AffixLines => {
                    let r = field(ui, &t!("affix-prefix"), &mut self.prefix, 200.0);
                    {
                        let r = field(ui, &t!("affix-suffix"), &mut self.suffix, 200.0);
                        enter |= submitted(ui, &r);
                    }
                    ui.checkbox(&mut self.skip_empty, t!("skip-empty-lines"));
                    r
                }
                DialogKind::NumberLines => {
                    let r = field(ui, &t!("initial-number"), &mut self.start, 80.0);
                    {
                        let r = field(ui, &t!("increase-by"), &mut self.step, 80.0);
                        enter |= submitted(ui, &r);
                    }
                    {
                        let r = field(ui, &t!("number-separator"), &mut self.separator, 80.0);
                        enter |= submitted(ui, &r);
                    }
                    ui.horizontal(|ui| {
                        for (p, label) in [
                            (NumberPad::None, "number-pad-none"),
                            (NumberPad::Spaces, "number-pad-spaces"),
                            (NumberPad::Zeros, "number-pad-zeros"),
                        ] {
                            ui.radio_value(&mut self.pad, p, crate::i18n::tr(label));
                        }
                    });
                    ui.checkbox(&mut self.skip_empty, t!("skip-empty-lines"));
                    r
                }
                DialogKind::Reflow => field(ui, &t!("reflow-width"), &mut self.width, 80.0),
                DialogKind::Field(k) => {
                    let op = self.field_mut(k);
                    let f = crate::fields_ui::form(ui, Id::new("field-dialog"), op);
                    enter |= f.enter;
                    f.first.expect("every Columns and Fields dialog has a text box")
                }
            };
            if matches!(
                kind,
                DialogKind::AffixLines
                    | DialogKind::NumberLines
                    | DialogKind::Reflow
                    | DialogKind::Field(_)
            ) {
                self.live_ui(ui, palette, kind, source);
            }
            if std::mem::take(&mut self.focus) {
                first.request_focus();
            }
            enter |= submitted(ui, &first);
            ui.add_space(6.0);
            self.dest_ui(ui, palette, kind);
            if let Some(e) = &self.error {
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(e).color(palette.modified).size(12.0));
                    });
            }
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 26.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if let Some(cmd) = &self.running {
                        if ui.button(t!("stop")).clicked() {
                            result = Some(DialogAction::Stop);
                        }
                        ui.spinner();
                        ui.label(
                            RichText::new(t!("pipe-running", command = cmd.clone()))
                                .color(palette.text_dim),
                        );
                        return;
                    }
                    if ui.button(t!("cancel")).clicked() {
                        close = true;
                    }
                    let ok = match kind {
                        DialogKind::Pipe => t!("pipe-run"),
                        DialogKind::FilterLines { .. } if self.keep => t!("filter-keep-button"),
                        DialogKind::FilterLines { .. } => t!("filter-remove-button"),
                        _ => t!("ok"),
                    };
                    let run = egui::Button::new(ok).fill(palette.accent.gamma_multiply(0.55));
                    if ui.add(run).clicked() || enter {
                        match self.request(kind) {
                            Ok(op) => {
                                result = Some(DialogAction::Run(op, self.dest));
                                // The command dialog stays open until the command ends.
                                close = kind != DialogKind::Pipe;
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                },
            );
        };
        // Keep or Remove Matching Lines sits at the top right of the text,
        // so the green and red lines stay in view; the others are modal.
        let should_close = match (kind, editor_rect) {
            (DialogKind::FilterLines { .. }, Some(rect)) => {
                egui::Window::new(t!("filter-lines-title"))
                    .id(Id::new("transform-dialog-side"))
                    .title_bar(false)
                    .collapsible(false)
                    .resizable(false)
                    .pivot(egui::Align2::RIGHT_TOP)
                    .fixed_pos(rect.right_top() + Vec2::new(-24.0, 44.0))
                    .show(ctx, |ui| body(ui));
                ctx.input(|i| i.key_pressed(Key::Escape))
            }
            _ => egui::Modal::new(Id::new("transform-dialog"))
                .show(ctx, |ui| body(ui))
                .should_close(),
        };
        if should_close && self.running.is_none() {
            close = true;
        }
        if close {
            self.kind = None;
            self.cancel_preview();
        }
        result
    }

    /// Result goes to: This tab / New tab / Clipboard.
    fn dest_ui(&mut self, ui: &mut Ui, palette: &Palette, kind: DialogKind) {
        // A command that failed its preview can't replace the text.
        let failed = kind == DialogKind::Pipe && self.preview_failed();
        if failed && self.dest == Dest::ThisTab {
            self.dest = Dest::NewTab;
        }
        ui.horizontal(|ui| {
            ui.label(t!("result-goes-to"));
            ui.add_enabled_ui(!failed, |ui| {
                ui.radio_value(&mut self.dest, Dest::ThisTab, t!("result-this-tab"))
                    .on_disabled_hover_text(t!("result-this-tab-failed"));
            });
            ui.radio_value(&mut self.dest, Dest::NewTab, t!("result-new-tab"));
            ui.radio_value(&mut self.dest, Dest::Clipboard, t!("result-clipboard"));
        });
        if self.dest != Dest::ThisTab {
            ui.label(
                RichText::new(t!("result-tab-unchanged"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
        }
    }

    /// Checks on background work: the debounced filter, its count, and a
    /// command preview.
    fn poll(&mut self, ctx: &egui::Context, source: &Source) {
        let now = ctx.input(|i| i.time);
        if matches!(self.kind, Some(DialogKind::FilterLines { .. })) {
            let key: FilterKey = (self.pattern.clone(), self.regex, self.match_case, self.keep);
            if self.typed.as_ref().is_none_or(|(k, _)| *k != key) {
                // The first time the marks show at once; after that they
                // wait until typing pauses.
                let at = if self.typed.is_none() {
                    now - DEBOUNCE
                } else {
                    now
                };
                self.typed = Some((key.clone(), at));
            }
            let (typed, since) = self.typed.clone().expect("set above");
            if self.shown.as_ref().is_none_or(|(k, _)| *k != typed) {
                if now - since >= DEBOUNCE {
                    let matcher = LineMatcher::new(&typed.0, typed.1, typed.2)
                        .map(Arc::new)
                        .map_err(|e| e.message);
                    self.shown = Some((typed, matcher));
                    ctx.request_repaint();
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                        DEBOUNCE - (now - since) + 0.01,
                    ));
                }
            }
            // Count the matches for what is shown.
            if let Some((shown, Ok(matcher))) = &self.shown {
                let want = (source.key, shown.clone());
                let have = self.count.as_ref().is_some_and(|(k, _)| *k == want)
                    || self.counting.as_ref().is_some_and(|(k, _)| *k == want);
                if !have && !shown.0.is_empty() {
                    if source.lines > BACKGROUND_LINES {
                        let text = (source.text)();
                        let matcher = matcher.clone();
                        let (tx, rx) = channel();
                        let repaint = ctx.clone();
                        std::thread::spawn(move || {
                            let _ = tx.send(transform::count_matching(&text, &matcher));
                            repaint.request_repaint();
                        });
                        self.counting = Some((want, rx));
                    } else {
                        let counted = transform::count_matching(&(source.text)(), matcher);
                        self.count = Some((want, counted));
                    }
                }
            }
            if let Some((key, rx)) = &self.counting
                && let Ok(counted) = rx.try_recv()
            {
                self.count = Some((key.clone(), counted));
                self.counting = None;
            }
        }
        if let Some(job) = &self.pipe_job
            && let Ok((output, seconds)) = job.rx.try_recv()
        {
            let job = self.pipe_job.take().expect("checked");
            self.pipe_result = Some(PipeResult {
                command: job.command,
                source: job.source,
                input: job.input,
                output,
                seconds,
            });
        } else if self.pipe_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    fn pipe_ui(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        history: &[String],
        source: &Source,
    ) -> egui::Response {
        let busy = self.running.is_some();
        let (r, preview) = ui
            .horizontal(|ui| {
                ui.label(t!("pipe-command"));
                let button_w = 100.0;
                let r = ui.add_enabled(
                    !busy,
                    egui::TextEdit::singleline(&mut self.command)
                        .hint_text("sort | uniq -c")
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width() - button_w)
                        .margin(Margin::symmetric(6, 4)),
                );
                let preview = ui
                    .add_enabled(
                        !busy && self.pipe_job.is_none() && !self.command.trim().is_empty(),
                        egui::Button::new(t!("pipe-preview")),
                    )
                    .on_hover_text(t!("pipe-preview-tip"))
                    .clicked();
                (r, preview)
            })
            .inner;
        if preview {
            self.start_preview(source);
        }
        if !history.is_empty() {
            ui.add_space(4.0);
            ui.add_enabled_ui(!busy, |ui| {
                egui::ComboBox::from_id_salt("pipe-history")
                    .selected_text(t!("pipe-recent"))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for cmd in history {
                            if ui.selectable_label(false, cmd).clicked() {
                                self.command = cmd.clone();
                                self.focus = true;
                            }
                        }
                    });
            });
        }
        ui.add_space(6.0);
        // The input is what the command will get; the output is the last
        // preview's, kept until the next Preview.
        let input_title = if source.selection {
            t!("pipe-input-selection")
        } else {
            t!("pipe-input-tab")
        };
        let input = match &self.pipe_result {
            Some(r) if r.source == source.key => r.input.clone(),
            _ => first_lines(&(source.text)(), 2000).to_owned(),
        };
        let (out_title, out_text, out_color) = match (&self.pipe_job, &self.pipe_result) {
            (Some(job), _) => (
                t!("pipe-output"),
                t!("pipe-previewing", command = job.command.clone()),
                Some(palette.text_dim),
            ),
            (None, Some(r)) => match &r.output {
                Ok(out) => (
                    t!(
                        "pipe-output-done",
                        seconds = format!("{:.2}", r.seconds),
                        code = 0
                    ),
                    out.clone(),
                    None,
                ),
                Err(RunError::Failed(code, stderr)) => (
                    t!(
                        "pipe-output-done",
                        seconds = format!("{:.2}", r.seconds),
                        code = code.map_or_else(|| "?".to_owned(), |c| c.to_string())
                    ),
                    if stderr.is_empty() {
                        t!("pipe-no-stderr")
                    } else {
                        stderr.clone()
                    },
                    Some(palette.modified),
                ),
                Err(RunError::Start(e)) => (
                    t!("pipe-output"),
                    t!("pipe-start-failed", error = e.clone()),
                    Some(palette.modified),
                ),
                Err(RunError::Stopped) => (t!("pipe-output"), String::new(), None),
            },
            (None, None) => (
                t!("pipe-output"),
                t!("pipe-output-hint"),
                Some(palette.text_dim),
            ),
        };
        ui.columns(2, |cols| {
            pane(
                &mut cols[0],
                "pipe-input",
                &input_title,
                &input,
                None,
                palette,
            );
            pane(
                &mut cols[1],
                "pipe-output",
                &out_title,
                &out_text,
                out_color,
                palette,
            );
        });
        if self
            .pipe_result
            .as_ref()
            .is_some_and(|r| r.command != self.command.trim() || r.source != source.key)
        {
            ui.label(
                RichText::new(t!("pipe-preview-stale"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
        }
        r
    }

    /// Runs the command on the text in the background, for the Output pane.
    fn start_preview(&mut self, source: &Source) {
        self.cancel_preview();
        let command = self.command.trim().to_owned();
        let input = (source.text)();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        {
            let command = command.clone();
            let input = input.clone();
            let cancel = cancel.clone();
            let dir = source.dir.clone();
            let file = source.file.clone();
            std::thread::spawn(move || {
                let started = std::time::Instant::now();
                let out = crate::tools::pipe::run(
                    &command,
                    dir.as_deref(),
                    file.as_deref(),
                    &input,
                    Some(&cancel),
                );
                let _ = tx.send((out, started.elapsed().as_secs_f64()));
            });
        }
        self.pipe_job = Some(PipePreviewJob {
            command,
            source: source.key,
            input: first_lines(&input, 2000).to_owned(),
            cancel,
            rx,
        });
    }

    fn filter_ui(&mut self, ui: &mut Ui, palette: &Palette) -> egui::Response {
        let r = ui
            .horizontal(|ui| {
                ui.label(t!("filter-pattern"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.pattern)
                        .desired_width(f32::INFINITY)
                        .margin(Margin::symmetric(6, 4)),
                )
            })
            .inner;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.match_case, t!("match-case"));
            ui.checkbox(&mut self.regex, t!("regular-expression"));
        });
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.keep, true, t!("filter-keep"));
            ui.radio_value(&mut self.keep, false, t!("filter-remove"));
        });
        let line = match &self.shown {
            Some(((pattern, ..), _)) if pattern.is_empty() => None,
            Some((_, Err(e))) => {
                Some((e.lines().last().unwrap_or("").to_owned(), palette.modified))
            }
            Some(((_, _, _, keep), Ok(_))) => match &self.count {
                Some((_, (hits, total))) => {
                    let kept = if *keep { *hits } else { total - hits };
                    Some((
                        format!(
                            "{} · {}",
                            t!("filter-kept-count", kept = kept, lines = *total),
                            t!("filter-marks-hint")
                        ),
                        palette.text_dim,
                    ))
                }
                None => Some((t!("filter-counting"), palette.text_dim)),
            },
            None => None,
        };
        if let Some((text, color)) = line {
            ui.label(RichText::new(text).size(12.0).color(color));
        }
        r
    }

    /// The Input and Output panes of the line dialogs, worked out on the
    /// first lines as you type.
    fn live_ui(&mut self, ui: &mut Ui, palette: &Palette, kind: DialogKind, source: &Source) {
        let op = self.request(kind);
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            source.key.hash(&mut h);
            match &op {
                Ok(op) => serde_json::to_string(op).unwrap_or_default().hash(&mut h),
                Err(e) => e.hash(&mut h),
            }
            h.finish()
        };
        if self.live.as_ref().is_none_or(|(k, _, _)| *k != key) {
            let all = (source.text)();
            let input = first_lines(&all, PREVIEW_LINES).to_owned();
            let output = op.and_then(|op| match &op {
                TransformOp::Field(f) => f.apply(&input).map_err(|e| e.message),
                op => Ok(preview_op(op, &input)),
            });
            self.live = Some((key, input, output));
        }
        let Some((_, input, output)) = &self.live else {
            return;
        };
        ui.add_space(6.0);
        let input_title = match (source.selection, source.lines > PREVIEW_LINES) {
            (true, false) => t!("pipe-input-selection"),
            (false, false) => t!("pipe-input-tab"),
            (_, true) => t!("preview-first-lines", lines = PREVIEW_LINES),
        };
        let (out, color) = match output {
            Ok(text) => (text.clone(), None),
            Err(e) => (e.clone(), Some(palette.modified)),
        };
        ui.columns(2, |cols| {
            pane(
                &mut cols[0],
                "live-input",
                &input_title,
                input,
                None,
                palette,
            );
            pane(
                &mut cols[1],
                "live-output",
                &t!("pipe-output"),
                &out,
                color,
                palette,
            );
        });
    }

    /// The settings of a Columns and Fields dialog.
    fn field_mut(&mut self, kind: FieldKind) -> &mut FieldOp {
        let i = match self.fields.iter().position(|op| op.kind() == Some(kind)) {
            Some(i) => i,
            None => {
                self.fields.push(kind.default_op());
                self.fields.len() - 1
            }
        };
        &mut self.fields[i]
    }

    fn request(&self, kind: DialogKind) -> Result<TransformOp, String> {
        let num = |s: &str, what: &str| {
            s.trim()
                .parse::<i64>()
                .map_err(|_| t!("must-be-whole-number", field = what))
        };
        Ok(match kind {
            DialogKind::Pipe => {
                let command = self.command.trim().to_owned();
                if command.is_empty() {
                    return Err(t!("pipe-no-command"));
                }
                TransformOp::Pipe { command }
            }
            DialogKind::FilterLines { .. } => {
                if self.pattern.is_empty() {
                    return Err(t!("filter-no-pattern"));
                }
                LineMatcher::new(&self.pattern, self.regex, self.match_case)
                    .map_err(|e| e.message)?;
                TransformOp::FilterLines {
                    pattern: self.pattern.clone(),
                    regex: self.regex,
                    match_case: self.match_case,
                    keep: self.keep,
                }
            }
            DialogKind::AffixLines => TransformOp::AffixLines {
                prefix: self.prefix.clone(),
                suffix: self.suffix.clone(),
                skip_empty: self.skip_empty,
            },
            DialogKind::NumberLines => TransformOp::NumberLines {
                start: num(&self.start, &t!("initial-number"))?,
                step: num(&self.step, &t!("increase-by"))?,
                separator: self.separator.clone(),
                pad: self.pad,
                skip_empty: self.skip_empty,
            },
            DialogKind::Reflow => {
                let width = num(&self.width, &t!("reflow-width"))?;
                TransformOp::Reflow {
                    width: width.clamp(1, 10_000) as usize,
                }
            }
            DialogKind::Field(k) => {
                let op = self
                    .fields
                    .iter()
                    .find(|op| op.kind() == Some(k))
                    .cloned()
                    .unwrap_or_else(|| k.default_op());
                op.check().map_err(|e| e.message)?;
                TransformOp::Field(op)
            }
        })
    }
}

/// What a line dialog's transform makes of `text`, for its Output pane.
pub fn preview_op(op: &TransformOp, text: &str) -> String {
    match op {
        TransformOp::AffixLines {
            prefix,
            suffix,
            skip_empty,
        } => transform::affix_lines(text, prefix, suffix, *skip_empty),
        TransformOp::NumberLines {
            start,
            step,
            separator,
            pad,
            skip_empty,
        } => transform::number_lines(text, *start, *step, separator, *pad, *skip_empty),
        TransformOp::Reflow { width } => transform::reflow(text, *width),
        TransformOp::FilterLines {
            pattern,
            regex,
            match_case,
            keep,
        } => match LineMatcher::new(pattern, *regex, *match_case) {
            Ok(m) => transform::filter_lines(text, &m, *keep),
            Err(e) => e.message,
        },
        TransformOp::Field(op) => op.apply(text).unwrap_or_else(|e| e.message),
        _ => text.to_owned(),
    }
}

fn submitted(ui: &Ui, r: &egui::Response) -> bool {
    r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_lines_cuts_after_n_lines() {
        assert_eq!(first_lines("a\nb\nc\n", 2), "a\nb");
        assert_eq!(first_lines("a\nb", 5), "a\nb");
        assert_eq!(first_lines("a\nb", 0), "a\nb");
    }

    #[test]
    fn previews_match_the_real_transforms() {
        let text = "one\n\ntwo\nthree";
        let op = TransformOp::AffixLines {
            prefix: "- ".into(),
            suffix: ";".into(),
            skip_empty: true,
        };
        assert_eq!(preview_op(&op, text), "- one;\n\n- two;\n- three;");
        let op = TransformOp::FilterLines {
            pattern: "t".into(),
            regex: false,
            match_case: false,
            keep: true,
        };
        assert_eq!(preview_op(&op, text), "two\nthree");
    }

    #[test]
    fn commands_send_results_to_a_new_tab_at_first() {
        assert_eq!(DialogKind::Pipe.default_dest(), Dest::NewTab);
        assert_eq!(DialogKind::Reflow.default_dest(), Dest::ThisTab);
        let mut d = TransformDialog::default();
        d.show_dialog(DialogKind::Pipe, 80, None);
        assert_eq!(d.dest, Dest::NewTab);
        d.close();
        d.show_dialog(DialogKind::Pipe, 80, Some(Dest::Clipboard));
        assert_eq!(d.dest, Dest::Clipboard);
    }

    #[cfg(unix)]
    #[test]
    fn command_preview_runs_without_changing_the_text() {
        let text = "b\na\nb\n".to_owned();
        let get = || text.clone();
        let source = Source {
            key: 7,
            text: &get,
            lines: 3,
            selection: false,
            dir: None,
            file: None,
        };
        let mut d = TransformDialog::default();
        d.show_dialog(DialogKind::Pipe, 80, None);
        d.command = "sort | uniq -c".into();
        d.start_preview(&source);
        let ctx = egui::Context::default();
        for _ in 0..200 {
            d.poll(&ctx, &source);
            if d.pipe_result.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let r = d.pipe_result.as_ref().expect("the preview finished");
        let out = r.output.as_ref().unwrap();
        assert!(out.contains("2 b") && out.contains("1 a"), "{out}");
        assert_eq!(text, "b\na\nb\n");
        // A failed preview turns This tab off.
        d.command = "exit 3".into();
        d.start_preview(&source);
        for _ in 0..200 {
            d.poll(&ctx, &source);
            if d.pipe_job.is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(d.preview_failed());
    }

    #[test]
    fn line_preview_is_only_for_matching_lines() {
        let mut d = TransformDialog::default();
        d.show_dialog(DialogKind::FilterLines { keep: true }, 80, None);
        d.set_pattern("ERROR");
        let key: FilterKey = ("ERROR".into(), false, false, true);
        d.shown = Some((
            key,
            Ok(Arc::new(LineMatcher::new("ERROR", false, false).unwrap())),
        ));
        let (m, keep) = d.line_preview().unwrap();
        assert!(keep && m.is_match("ERROR disk full") && !m.is_match("INFO done"));
        d.close();
        assert!(d.line_preview().is_none());
    }
}
