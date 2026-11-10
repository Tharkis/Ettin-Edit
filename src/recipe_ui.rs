//! Transform > Recipes: the window that lists, edits, previews and runs
//! recipes, and the question asked before a recipe runs shell commands.

use egui::{Align, Color32, Id, Key, KeyboardShortcut, Layout, Margin, RichText, Ui, Vec2};
use egui_phosphor::regular as ph;

use crate::editor::lines::{CaseKind, SortKind, TrimKind};
use crate::recipes::{Env, Preview, Recipe, RecipeStep, Step};
use crate::search_ui::icon_button;
use crate::theme::Palette;
use crate::tools::convert::DataFormat;
use crate::tools::json::JsonOp;
use crate::tools::mime::MimeOp;
use crate::tools::transform::{HashKind, NumberPad};
use crate::tools::xml::XmlOp;

/// The preview runs on at most this many characters of the text.
pub const PREVIEW_CHARS: usize = 20_000;

/// What the window asks the app to do.
pub enum RecipeEvent {
    /// The recipe at this index was edited; save it.
    Changed(usize),
    New,
    Duplicate(usize),
    Delete(usize),
    Import,
    Export(usize),
    OpenFolder,
    Reload,
    Run {
        index: usize,
        all_tabs: bool,
    },
    /// Give the recipe this shortcut (`None` takes its shortcut away).
    /// With `force`, take it from the command that has it now.
    SetShortcut {
        index: usize,
        shortcut: Option<KeyboardShortcut>,
        force: bool,
    },
}

/// A picked shortcut that another command already uses.
pub struct ShortcutConflict {
    pub index: usize,
    pub shortcut: KeyboardShortcut,
    /// The other command's menu path.
    pub other: String,
}

/// The text the preview starts from.
pub struct PreviewInput<'a> {
    /// Changes when the text or selection changes.
    pub key: u64,
    /// The text, at most [`PREVIEW_CHARS`] characters.
    pub text: &'a dyn Fn() -> String,
    /// True when it is the selection rather than the whole document.
    pub selection: bool,
    pub indent: &'a str,
}

#[derive(Default)]
pub struct RecipeWindow {
    pub open: bool,
    pub selected: Option<usize>,
    /// The step whose settings are showing.
    expanded: Option<usize>,
    /// Asking whether to delete the selected recipe.
    confirm_delete: bool,
    pub focus_name: bool,
    /// Put the keyboard in the open step's first box.
    focus_step: bool,
    /// What the preview was worked out for, and its result.
    preview: Option<(u64, String, Preview)>,
    /// A message for the user, such as a failed save.
    pub message: Option<String>,
    /// Waiting for the selected recipe's new shortcut.
    capturing: bool,
    /// A key press taken at the start of the frame, before the editor sees it.
    grabbed: Option<Result<KeyboardShortcut, String>>,
    /// The picked shortcut is in use; asking whether to move it.
    pub conflict: Option<ShortcutConflict>,
    shortcut_error: Option<String>,
    /// The search box at the top of Add Step, and the highlighted match.
    step_query: String,
    step_pick: usize,
    focus_step_search: bool,
    /// The selected recipe's shortcut, as menus show it.
    shortcut_text: Option<String>,
    /// Preview on the recipe's sample text rather than the active tab.
    use_sample: bool,
    /// Show the text as it is after this step (`None`: after the last).
    show_after: Option<usize>,
}

fn tr(id: &str) -> String {
    crate::i18n::tr(id)
}

fn sort_label(kind: SortKind, desc: bool) -> String {
    let id = match kind {
        SortKind::Lexical => "sort-lexical",
        SortKind::IgnoreCase => "sort-ignore-case",
        SortKind::Integer => "sort-integer",
        SortKind::Decimal => "sort-decimal",
        SortKind::Length => "sort-length",
    };
    tr(&format!("{id}-{}", if desc { "desc" } else { "asc" }))
}

fn case_label(kind: CaseKind) -> String {
    tr(match kind {
        CaseKind::Upper => "case-upper",
        CaseKind::Lower => "case-lower",
        CaseKind::Title => "case-title",
        CaseKind::Sentence => "case-sentence",
        CaseKind::Invert => "case-invert",
        CaseKind::Camel => "case-camel",
        CaseKind::Pascal => "case-pascal",
        CaseKind::Snake => "case-snake",
        CaseKind::Kebab => "case-kebab",
        CaseKind::Constant => "case-constant",
    })
}

const CASES: [CaseKind; 10] = [
    CaseKind::Upper,
    CaseKind::Lower,
    CaseKind::Title,
    CaseKind::Sentence,
    CaseKind::Invert,
    CaseKind::Camel,
    CaseKind::Pascal,
    CaseKind::Snake,
    CaseKind::Kebab,
    CaseKind::Constant,
];

fn trim_label(kind: TrimKind) -> String {
    tr(match kind {
        TrimKind::Trailing => "trim-trailing",
        TrimKind::Leading => "trim-leading",
        TrimKind::Both => "trim-both",
    })
}

fn mime_label(kind: MimeOp) -> String {
    tr(match kind {
        MimeOp::Base64Encode => "mime-base64-encode",
        MimeOp::Base64EncodeWrapped => "mime-base64-encode-wrapped",
        MimeOp::Base64UrlEncode => "mime-base64-url-encode",
        MimeOp::Base64Decode => "mime-base64-decode",
        MimeOp::UrlEncodeComponent => "mime-url-encode-component",
        MimeOp::UrlEncode => "mime-url-encode",
        MimeOp::UrlDecode => "mime-url-decode",
        MimeOp::QpEncode => "mime-qp-encode",
        MimeOp::QpDecode => "mime-qp-decode",
        MimeOp::HtmlEncode => "mime-html-encode",
        MimeOp::HtmlEncodeAll => "mime-html-encode-all",
        MimeOp::HtmlDecode => "mime-html-decode",
        MimeOp::SamlDecode => "mime-saml-decode",
    })
}

const MIME_OPS: [MimeOp; 13] = [
    MimeOp::Base64Encode,
    MimeOp::Base64EncodeWrapped,
    MimeOp::Base64UrlEncode,
    MimeOp::Base64Decode,
    MimeOp::UrlEncodeComponent,
    MimeOp::UrlEncode,
    MimeOp::UrlDecode,
    MimeOp::QpEncode,
    MimeOp::QpDecode,
    MimeOp::HtmlEncode,
    MimeOp::HtmlEncodeAll,
    MimeOp::HtmlDecode,
    MimeOp::SamlDecode,
];

const HASHES: [HashKind; 5] = [
    HashKind::Md5,
    HashKind::Sha1,
    HashKind::Sha256,
    HashKind::Sha512,
    HashKind::Crc32,
];

fn json_label(kind: JsonOp) -> String {
    let what = tr(match kind {
        JsonOp::Format => "json-format",
        JsonOp::Minify => "json-minify",
        JsonOp::Validate => "json-validate",
        JsonOp::SortKeys => "json-sort-keys",
    });
    t!("recipe-step-json", action = what)
}

const JSON_OPS: [JsonOp; 3] = [JsonOp::Format, JsonOp::Minify, JsonOp::SortKeys];

fn xml_label(kind: XmlOp) -> String {
    let what = tr(match kind {
        XmlOp::PrettyPrint => "xml-pretty-print",
        XmlOp::PrettyPrintAttributes => "xml-pretty-print-attributes",
        XmlOp::Linearize => "xml-linearize",
        XmlOp::Escape => "xml-escape",
        XmlOp::Unescape => "xml-unescape",
        XmlOp::CheckSyntax => "xml-check-syntax",
        XmlOp::ValidateSchema => "xml-validate",
        XmlOp::ChooseSchema => "xml-validate-against",
        XmlOp::XslTransform => "xml-xsl-transform",
    });
    t!("recipe-step-xml", action = what)
}

const XML_OPS: [XmlOp; 5] = [
    XmlOp::PrettyPrint,
    XmlOp::PrettyPrintAttributes,
    XmlOp::Linearize,
    XmlOp::Escape,
    XmlOp::Unescape,
];

/// Text shown in a step's title, kept short.
fn short(s: &str) -> String {
    let one: String = s.replace('\n', "↵").replace('\t', "→");
    if one.chars().count() > 40 {
        one.chars().take(40).collect::<String>() + "…"
    } else {
        one
    }
}

/// A step's one-line description.
pub fn step_label(step: &Step) -> String {
    match step {
        Step::Replace { find, replace, .. } => t!(
            "recipe-step-replace",
            find = short(find),
            replace = short(replace)
        ),
        Step::FilterLines { pattern, keep, .. } => {
            if *keep {
                t!("recipe-step-keep", pattern = short(pattern))
            } else {
                t!("recipe-step-remove", pattern = short(pattern))
            }
        }
        Step::CountDistinct => tr("count-distinct-lines"),
        Step::AffixLines { prefix, suffix, .. } => t!(
            "recipe-step-affix",
            prefix = short(prefix),
            suffix = short(suffix)
        ),
        Step::NumberLines { start, step, .. } => {
            t!("recipe-step-number", start = *start, step = *step)
        }
        Step::Reflow { width } => t!("recipe-step-reflow", width = *width),
        Step::Convert { from, to } => t!("convert-to", from = from.name(), to = to.name()),
        Step::ConvertTo { to, from: None } => {
            t!("convert-to-format", format = target_name(*to))
        }
        Step::ConvertTo {
            to,
            from: Some(from),
        } => {
            t!("convert-to", from = from.name(), to = target_name(*to))
        }
        Step::Hash { kind } => t!("recipe-step-hash", kind = kind.name()),
        Step::HexEncode => tr("hex-encode"),
        Step::HexDecode => tr("hex-decode"),
        Step::JwtDecode => tr("jwt-decode"),
        Step::SortLines { kind, descending } => sort_label(*kind, *descending),
        Step::ReverseLines => tr("reverse-lines"),
        Step::RemoveDuplicates { consecutive } => tr(if *consecutive {
            "remove-consecutive-duplicates"
        } else {
            "remove-duplicate-lines"
        }),
        Step::RemoveEmptyLines { blank } => tr(if *blank {
            "remove-blank-lines"
        } else {
            "remove-empty-lines"
        }),
        Step::Trim { kind } => trim_label(*kind),
        Step::JoinLines => tr("recipe-step-join"),
        Step::TabsToSpaces => tr("tabs-to-spaces"),
        Step::SpacesToTabs { all } => tr(if *all {
            "all-spaces-to-tabs"
        } else {
            "leading-spaces-to-tabs"
        }),
        Step::Case { kind } => case_label(*kind),
        Step::Mime { kind } => mime_label(*kind),
        Step::Json { kind } => json_label(*kind),
        Step::Xml { kind } => xml_label(*kind),
        Step::Command { command } => t!("recipe-step-command", command = short(command)),
    }
}

/// A target format's name: "Markdown table" for Markdown.
fn target_name(f: DataFormat) -> String {
    match f {
        DataFormat::Markdown => tr("convert-markdown-table"),
        f => f.name().to_owned(),
    }
}

/// A step's name in the Add Step menu, before it has settings.
fn catalog_label(step: &Step) -> String {
    match step {
        Step::Replace { .. } => tr("recipe-group-replace"),
        Step::FilterLines { keep: true, .. } => tr("keep-matching-lines"),
        Step::FilterLines { keep: false, .. } => tr("remove-matching-lines"),
        Step::AffixLines { .. } => tr("affix-lines"),
        Step::NumberLines { .. } => tr("number-lines"),
        Step::Reflow { .. } => tr("reflow"),
        Step::Command { .. } => tr("pipe-title"),
        _ => step_label(step),
    }
}

/// The steps offered by Add Step, by group: the Transform menu's groups
/// (see [`crate::transforms`]), each with the steps a recipe can do.
fn catalog() -> Vec<(String, Vec<Step>)> {
    crate::transforms::catalogue()
        .into_iter()
        .map(|g| (g.id.label(), g.steps()))
        .filter(|(_, steps)| !steps.is_empty())
        .collect()
}

/// The names of the Add Step groups.
#[cfg(test)]
pub fn add_step_groups() -> Vec<String> {
    catalog().into_iter().map(|(g, _)| g).collect()
}

/// The steps whose name (or group) has every word of `query`, in catalog
/// order, with their group's name.
fn search_steps(groups: Vec<(String, Vec<Step>)>, query: &str) -> Vec<(String, Step)> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut out = Vec::new();
    for (group, steps) in groups {
        for step in steps {
            let hay = format!("{} {}", catalog_label(&step), group).to_lowercase();
            if words.iter().all(|w| hay.contains(w)) {
                out.push((group.clone(), step));
            }
        }
    }
    out
}

/// A drop-down list; true when the value changed.
fn combo<T: PartialEq + Copy>(ui: &mut Ui, id: Id, value: &mut T, options: &[(T, String)]) -> bool {
    let mut changed = false;
    let current = options
        .iter()
        .find(|(v, _)| v == value)
        .map(|(_, l)| l.clone())
        .unwrap_or_default();
    egui::ComboBox::from_id_salt(id)
        .selected_text(current)
        .width(260.0)
        .show_ui(ui, |ui| {
            for (v, label) in options {
                if ui.selectable_label(v == value, label).clicked() && v != value {
                    *value = *v;
                    changed = true;
                }
            }
        });
    changed
}

/// A labelled text box. It has a fixed id, so it keeps the keyboard focus
/// when the rows above it change as you type.
fn text_field(ui: &mut Ui, id: Id, label: &str, value: &mut String, focus: &mut bool) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([120.0, 20.0], egui::Label::new(label));
        let edit = egui::TextEdit::singleline(value)
            .id(id.with(label))
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(6, 3));
        let r = ui.add(edit);
        if std::mem::take(focus) {
            r.request_focus();
        }
        r.changed()
    })
    .inner
}

fn labelled(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> bool) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([120.0, 20.0], egui::Label::new(label));
        add(ui)
    })
    .inner
}

/// The settings of one step; true when something changed.
fn step_settings(ui: &mut Ui, id: Id, step: &mut Step, palette: &Palette, focus: bool) -> bool {
    let mut changed = false;
    // Only the first text box takes the focus.
    let mut focus = focus;
    match step {
        Step::Replace {
            find,
            replace,
            regex,
            match_case,
            whole_word,
            extended,
        } => {
            changed |= text_field(ui, id, &tr("find-what"), find, &mut focus);
            changed |= text_field(ui, id, &tr("recipe-replace-with"), replace, &mut focus);
            ui.horizontal_wrapped(|ui| {
                changed |= ui.checkbox(match_case, tr("match-case")).changed();
                changed |= ui.checkbox(whole_word, tr("whole-word")).changed();
                changed |= ui.checkbox(regex, tr("regular-expression")).changed();
                ui.add_enabled_ui(!*regex, |ui| {
                    changed |= ui.checkbox(extended, tr("extended-search-short")).changed();
                });
            });
        }
        Step::FilterLines {
            pattern,
            regex,
            match_case,
            keep,
        } => {
            changed |= text_field(ui, id, &tr("recipe-pattern"), pattern, &mut focus);
            ui.horizontal(|ui| {
                changed |= ui.checkbox(match_case, tr("match-case")).changed();
                changed |= ui.checkbox(regex, tr("regular-expression")).changed();
            });
            ui.horizontal(|ui| {
                changed |= ui.radio_value(keep, true, tr("filter-keep")).changed();
                changed |= ui.radio_value(keep, false, tr("filter-remove")).changed();
            });
        }
        Step::AffixLines {
            prefix,
            suffix,
            skip_empty,
        } => {
            changed |= text_field(ui, id, &tr("affix-prefix"), prefix, &mut focus);
            changed |= text_field(ui, id, &tr("affix-suffix"), suffix, &mut focus);
            changed |= ui.checkbox(skip_empty, tr("skip-empty-lines")).changed();
        }
        Step::NumberLines {
            start,
            step,
            separator,
            pad,
            skip_empty,
        } => {
            changed |= labelled(ui, &tr("initial-number"), |ui| {
                ui.add(egui::DragValue::new(start)).changed()
            });
            changed |= labelled(ui, &tr("increase-by"), |ui| {
                ui.add(egui::DragValue::new(step)).changed()
            });
            changed |= text_field(ui, id, &tr("number-separator"), separator, &mut focus);
            ui.horizontal(|ui| {
                for (p, label) in [
                    (NumberPad::None, "number-pad-none"),
                    (NumberPad::Spaces, "number-pad-spaces"),
                    (NumberPad::Zeros, "number-pad-zeros"),
                ] {
                    changed |= ui.radio_value(pad, p, tr(label)).changed();
                }
            });
            changed |= ui.checkbox(skip_empty, tr("skip-empty-lines")).changed();
        }
        Step::Reflow { width } => {
            changed |= labelled(ui, &tr("reflow-width"), |ui| {
                ui.add(egui::DragValue::new(width).range(1..=10_000))
                    .changed()
            });
        }
        Step::Convert { from, to } => {
            let sources: Vec<_> = DataFormat::SOURCES
                .iter()
                .map(|f| (*f, f.name().to_owned()))
                .collect();
            let targets: Vec<_> = DataFormat::TARGETS
                .iter()
                .map(|f| (*f, f.name().to_owned()))
                .collect();
            changed |= labelled(ui, &tr("recipe-convert-from"), |ui| {
                combo(ui, id.with("from"), from, &sources)
            });
            changed |= labelled(ui, &tr("recipe-convert-to"), |ui| {
                combo(ui, id.with("to"), to, &targets)
            });
        }
        Step::ConvertTo { to, from } => {
            let mut sources = vec![(None, tr("read-as-auto"))];
            sources.extend(
                DataFormat::SOURCES
                    .iter()
                    .map(|f| (Some(*f), f.name().to_owned())),
            );
            let targets: Vec<_> = DataFormat::TARGETS
                .iter()
                .map(|f| (*f, target_name(*f)))
                .collect();
            changed |= labelled(ui, &tr("menu-read-as"), |ui| {
                combo(ui, id.with("from"), from, &sources)
            });
            changed |= labelled(ui, &tr("recipe-convert-to"), |ui| {
                combo(ui, id.with("to"), to, &targets)
            });
        }
        Step::Hash { kind } => {
            let options: Vec<_> = HASHES.iter().map(|k| (*k, k.name().to_owned())).collect();
            changed |= combo(ui, id.with("hash"), kind, &options);
            ui.label(
                RichText::new(tr("recipe-hash-hint"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
        }
        Step::SortLines { kind, descending } => {
            let options: Vec<_> = [
                SortKind::Lexical,
                SortKind::IgnoreCase,
                SortKind::Integer,
                SortKind::Decimal,
                SortKind::Length,
            ]
            .iter()
            .map(|k| (*k, sort_label(*k, false)))
            .collect();
            changed |= combo(ui, id.with("sort"), kind, &options);
            changed |= ui.checkbox(descending, tr("recipe-descending")).changed();
        }
        Step::RemoveDuplicates { consecutive } => {
            changed |= ui
                .checkbox(consecutive, tr("recipe-only-consecutive"))
                .changed();
        }
        Step::RemoveEmptyLines { blank } => {
            changed |= ui.checkbox(blank, tr("recipe-include-blank")).changed();
        }
        Step::Trim { kind } => {
            let options: Vec<_> = [TrimKind::Trailing, TrimKind::Leading, TrimKind::Both]
                .iter()
                .map(|k| (*k, trim_label(*k)))
                .collect();
            changed |= combo(ui, id.with("trim"), kind, &options);
        }
        Step::SpacesToTabs { all } => {
            changed |= ui.checkbox(all, tr("all-spaces-to-tabs")).changed();
        }
        Step::Case { kind } => {
            let options: Vec<_> = CASES.iter().map(|k| (*k, case_label(*k))).collect();
            changed |= combo(ui, id.with("case"), kind, &options);
        }
        Step::Mime { kind } => {
            let options: Vec<_> = MIME_OPS.iter().map(|k| (*k, mime_label(*k))).collect();
            changed |= combo(ui, id.with("mime"), kind, &options);
        }
        Step::Json { kind } => {
            let options: Vec<_> = JSON_OPS.iter().map(|k| (*k, json_label(*k))).collect();
            changed |= combo(ui, id.with("json"), kind, &options);
        }
        Step::Xml { kind } => {
            let options: Vec<_> = XML_OPS.iter().map(|k| (*k, xml_label(*k))).collect();
            changed |= combo(ui, id.with("xml"), kind, &options);
        }
        Step::Command { command } => {
            changed |= text_field(ui, id, &tr("recipe-command"), command, &mut focus);
            ui.label(
                RichText::new(tr("recipe-command-hint"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
        }
        Step::CountDistinct => {
            ui.label(
                RichText::new(tr("recipe-count-hint"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
        }
        Step::HexEncode
        | Step::HexDecode
        | Step::JwtDecode
        | Step::ReverseLines
        | Step::JoinLines
        | Step::TabsToSpaces => {
            ui.label(
                RichText::new(tr("recipe-no-settings"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
        }
    }
    changed
}

/// A box of text for the preview.
fn preview_box(ui: &mut Ui, id: &str, text: &str, width: f32, color: Option<Color32>) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .inner_margin(Margin::same(4))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.set_width(width - 8.0);
            egui::ScrollArea::both()
                .id_salt(id)
                .max_height(110.0)
                .min_scrolled_height(110.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut text = RichText::new(text).monospace().size(12.0);
                    if let Some(c) = color {
                        text = text.color(c);
                    }
                    ui.add(egui::Label::new(text).extend());
                });
        });
}

/// The recipe's sample text, editable. Returns true when it changed.
fn sample_box(ui: &mut Ui, sample: &mut String, width: f32) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .inner_margin(Margin::same(4))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.set_width(width - 8.0);
            egui::ScrollArea::vertical()
                .id_salt("recipe-sample")
                .max_height(110.0)
                .min_scrolled_height(110.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    changed = ui
                        .add(
                            egui::TextEdit::multiline(sample)
                                .id(Id::new("recipe-sample-text"))
                                .font(egui::TextStyle::Monospace)
                                .frame(egui::Frame::NONE)
                                .desired_rows(6)
                                .desired_width(f32::INFINITY),
                        )
                        .changed();
                });
        });
    changed
}

impl RecipeWindow {
    pub fn show_window(&mut self, select: Option<usize>) {
        self.open = true;
        if select.is_some() {
            self.selected = select;
            self.expanded = None;
            self.show_after = None;
        }
        self.confirm_delete = false;
        self.preview = None;
    }

    /// While waiting for a shortcut, takes this frame's key presses so
    /// nothing else acts on them. Call before anything else reads input.
    pub fn grab_keys(&mut self, ctx: &egui::Context) {
        if !self.open || !self.capturing {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape) && i.modifiers.is_none()) {
            self.capturing = false;
        } else if let Some(result) = crate::shortcut_ui::captured(ctx) {
            self.grabbed = Some(result);
        }
        crate::shortcut_ui::swallow_keys(ctx);
    }

    /// Draws the window. `shortcut` is the selected recipe's shortcut, as
    /// shown in menus.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        palette: &Palette,
        recipes: &mut [Recipe],
        input: &PreviewInput,
        shortcut: Option<&str>,
    ) -> Vec<RecipeEvent> {
        let mut events = Vec::new();
        if !self.open {
            return events;
        }
        if self.selected.is_some_and(|i| i >= recipes.len()) {
            self.selected = None;
        }
        if self.capturing
            && let Some(result) = self.grabbed.take()
        {
            self.capturing = false;
            match (result, self.selected) {
                (Ok(s), Some(index)) => {
                    self.shortcut_error = None;
                    events.push(RecipeEvent::SetShortcut {
                        index,
                        shortcut: Some(s),
                        force: false,
                    });
                }
                (Err(e), _) => self.shortcut_error = Some(e),
                _ => {}
            }
        }
        self.shortcut_text = shortcut.map(str::to_owned);
        let mut open = true;
        egui::Window::new(tr("recipes-title"))
            .id(Id::new("recipes-window"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size([860.0, 590.0])
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(210.0, 540.0),
                        Layout::top_down(Align::Min),
                        |ui| self.list_ui(ui, palette, recipes, &mut events),
                    );
                    ui.separator();
                    ui.vertical(|ui| match self.selected {
                        Some(i) => self.recipe_ui(ui, palette, recipes, i, input, &mut events),
                        None => {
                            ui.add_space(40.0);
                            ui.label(RichText::new(tr("recipes-pick")).color(palette.text_dim));
                        }
                    });
                });
                ui.separator();
                self.bottom_bar(ui, recipes, &mut events);
            });
        if !open {
            self.open = false;
            self.capturing = false;
            self.conflict = None;
        }
        events
    }

    /// New, Duplicate, Delete… on the left and Run on the right. The
    /// buttons have text labels; when they don't fit (a long translation),
    /// they show only their icons, with the labels as tooltips.
    fn bottom_bar(&mut self, ui: &mut Ui, recipes: &[Recipe], events: &mut Vec<RecipeEvent>) {
        #[derive(Clone, Copy)]
        enum Do {
            New,
            Duplicate,
            Delete,
            Export,
            Import,
            Folder,
            Reload,
        }
        let selected = self.selected.filter(|i| *i < recipes.len());
        let buttons = [
            (ph::PLUS, "recipe-new-short", "recipe-new", true, Do::New),
            (
                ph::COPY,
                "recipe-duplicate",
                "recipe-duplicate",
                selected.is_some(),
                Do::Duplicate,
            ),
            (
                ph::TRASH,
                "recipe-delete-dots",
                "recipe-delete-dots",
                selected.is_some(),
                Do::Delete,
            ),
            (
                ph::EXPORT,
                "recipe-export-short",
                "recipe-export",
                selected.is_some(),
                Do::Export,
            ),
            (
                ph::DOWNLOAD_SIMPLE,
                "recipe-import-short",
                "recipe-import",
                true,
                Do::Import,
            ),
            (
                ph::FOLDER_OPEN,
                "recipe-folder",
                "open-recipes-folder",
                true,
                Do::Folder,
            ),
            (
                ph::ARROW_CLOCKWISE,
                "recipe-reload",
                "reload-recipes",
                true,
                Do::Reload,
            ),
        ];
        let run = format!("{} {}", ph::PLAY, tr("run"));
        let all = tr("recipe-run-all-tabs");
        let pad = ui.spacing().button_padding.x * 2.0 + ui.spacing().item_spacing.x;
        let width = |ui: &Ui, text: &str| {
            let font = egui::TextStyle::Button.resolve(ui.style());
            ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font, Color32::WHITE))
                .size()
                .x
                + pad
        };
        let needed: f32 = buttons
            .iter()
            .map(|(icon, label, ..)| width(ui, &format!("{icon} {}", tr(label))))
            .sum::<f32>()
            + width(ui, &run)
            + width(ui, &all)
            + 16.0;
        let labels = needed <= ui.available_width();
        let can_run = selected.is_some_and(|i| !recipes[i].name.trim().is_empty());
        ui.horizontal(|ui| {
            for (icon, label, tip, enabled, what) in buttons {
                let clicked = ui
                    .add_enabled_ui(enabled, |ui| {
                        if labels {
                            ui.button(format!("{icon} {}", tr(label)))
                                .on_hover_text(tr(tip))
                                .clicked()
                        } else {
                            icon_button(ui, icon, &tr(tip))
                        }
                    })
                    .inner;
                if !clicked {
                    continue;
                }
                match what {
                    Do::New => events.push(RecipeEvent::New),
                    Do::Duplicate => events.extend(selected.map(RecipeEvent::Duplicate)),
                    Do::Delete => self.confirm_delete = true,
                    Do::Export => events.extend(selected.map(RecipeEvent::Export)),
                    Do::Import => events.push(RecipeEvent::Import),
                    Do::Folder => events.push(RecipeEvent::OpenFolder),
                    Do::Reload => events.push(RecipeEvent::Reload),
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_enabled_ui(can_run, |ui| {
                    let Some(index) = selected else {
                        return;
                    };
                    let run = egui::Button::new(RichText::new(run).strong())
                        .fill(ui.visuals().selection.bg_fill);
                    if ui.add(run).clicked() {
                        events.push(RecipeEvent::Run {
                            index,
                            all_tabs: false,
                        });
                    }
                    if ui.button(all).clicked() {
                        events.push(RecipeEvent::Run {
                            index,
                            all_tabs: true,
                        });
                    }
                });
            });
        });
    }

    fn list_ui(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        recipes: &[Recipe],
        events: &mut Vec<RecipeEvent>,
    ) {
        egui::ScrollArea::vertical()
            .id_salt("recipe-list")
            .max_height(470.0)
            .min_scrolled_height(470.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if recipes.is_empty() {
                    ui.label(RichText::new(tr("recipes-none")).color(palette.text_dim));
                }
                for (i, r) in recipes.iter().enumerate() {
                    let mut label = r.name.clone();
                    if !r.commands().is_empty() {
                        label = format!("{label}  {}", ph::TERMINAL);
                    }
                    let resp = ui.add(
                        egui::Button::selectable(self.selected == Some(i), label)
                            .min_size(Vec2::new(ui.available_width(), 22.0)),
                    );
                    let resp = if r.description.is_empty() {
                        resp
                    } else {
                        resp.on_hover_text(&r.description)
                    };
                    if resp.clicked() && self.selected != Some(i) {
                        self.selected = Some(i);
                        self.expanded = None;
                        self.confirm_delete = false;
                        self.preview = None;
                    }
                    if resp.double_clicked() {
                        events.push(RecipeEvent::Run {
                            index: i,
                            all_tabs: false,
                        });
                    }
                }
            });
        ui.add_space(6.0);
        if self.confirm_delete
            && let Some(i) = self.selected
        {
            ui.label(t!("recipe-delete-question", name = recipes[i].name.clone()));
            ui.horizontal(|ui| {
                if ui.button(tr("recipe-delete")).clicked() {
                    events.push(RecipeEvent::Delete(i));
                    self.confirm_delete = false;
                    self.selected = None;
                }
                if ui.button(tr("cancel")).clicked() {
                    self.confirm_delete = false;
                }
            });
        }
    }

    /// Shortcut: the recipe's shortcut with Set… and Clear, then its id.
    fn shortcut_row(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        index: usize,
        id: &str,
        shortcut: Option<&str>,
        events: &mut Vec<RecipeEvent>,
    ) {
        ui.horizontal(|ui| {
            ui.add_sized([90.0, 20.0], egui::Label::new(tr("shortcut")));
            let shown = if self.capturing {
                RichText::new(tr("press-shortcut")).color(palette.accent)
            } else {
                match shortcut {
                    Some(s) => RichText::new(s).monospace(),
                    None => RichText::new(tr("recipe-no-shortcut")).color(palette.text_dim),
                }
            };
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(4)
                .inner_margin(Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.set_min_width(170.0);
                    ui.label(shown);
                });
            if !self.capturing && ui.button(tr("set-shortcut")).clicked() {
                self.capturing = true;
                self.conflict = None;
                self.shortcut_error = None;
            }
            if ui
                .add_enabled(
                    shortcut.is_some() && !self.capturing,
                    egui::Button::new(tr("recipe-clear-shortcut")),
                )
                .clicked()
            {
                events.push(RecipeEvent::SetShortcut {
                    index,
                    shortcut: None,
                    force: false,
                });
            }
        });
        if let Some(c) = &self.conflict
            && c.index == index
        {
            let text = t!(
                "shortcut-in-use",
                shortcut = ui.ctx().format_shortcut(&c.shortcut),
                command = c.other.clone()
            );
            let mut decided = None;
            egui::Frame::new()
                .fill(palette.find_hit)
                .corner_radius(4)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(text);
                        if ui.button(tr("move-it-here")).clicked() {
                            decided = Some(true);
                        }
                        if ui.button(tr("cancel")).clicked() {
                            decided = Some(false);
                        }
                    });
                });
            if let Some(take) = decided {
                let c = self.conflict.take().expect("checked above");
                if take {
                    events.push(RecipeEvent::SetShortcut {
                        index,
                        shortcut: Some(c.shortcut),
                        force: true,
                    });
                }
            }
        }
        if let Some(e) = &self.shortcut_error {
            ui.label(RichText::new(e).color(palette.modified).size(12.0));
        }
        ui.horizontal(|ui| {
            ui.add_space(94.0);
            ui.label(
                RichText::new(t!("recipe-id", id = id.to_owned()))
                    .size(12.0)
                    .color(palette.text_dim),
            )
            .on_hover_text(tr("recipe-id-hint"));
        });
    }

    /// Add Step's menu: a search box over every step, or the steps by
    /// group while the box is empty. Returns the step picked.
    fn add_step_menu(&mut self, ui: &mut Ui, palette: &Palette) -> Option<Step> {
        let edit = ui.add(
            egui::TextEdit::singleline(&mut self.step_query)
                .id(Id::new("recipe-step-search"))
                .hint_text(tr("recipe-step-search"))
                .desired_width(260.0),
        );
        if std::mem::take(&mut self.focus_step_search) {
            edit.request_focus();
        }
        if edit.changed() {
            self.step_pick = 0;
        }
        let query = self.step_query.trim().to_lowercase();
        let groups = catalog();
        if query.is_empty() {
            let mut picked = None;
            for (group, steps) in groups {
                ui.menu_button(group, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .show(ui, |ui| {
                            for step in steps {
                                if ui.button(catalog_label(&step)).clicked() {
                                    picked = Some(step);
                                }
                            }
                        });
                });
            }
            if picked.is_some() {
                ui.close();
            }
            return picked;
        }
        let found = search_steps(groups, &query);
        if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
            self.step_pick = (self.step_pick + 1).min(found.len().saturating_sub(1));
        }
        if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
            self.step_pick = self.step_pick.saturating_sub(1);
        }
        let mut picked = None;
        if ui.input(|i| i.key_pressed(Key::Enter)) {
            picked = found.get(self.step_pick).map(|(_, s)| s.clone());
        }
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                if found.is_empty() {
                    ui.label(RichText::new(tr("nothing-matches")).color(palette.text_dim));
                }
                let mut last_group = "";
                for (i, (group, step)) in found.iter().enumerate() {
                    if group != last_group {
                        ui.label(
                            RichText::new(group.to_uppercase())
                                .size(11.0)
                                .color(palette.text_dim),
                        );
                        last_group = group;
                    }
                    let r = ui.add(
                        egui::Button::selectable(i == self.step_pick, catalog_label(step))
                            .min_size(Vec2::new(260.0, 0.0)),
                    );
                    if i == self.step_pick
                        && ui
                            .input(|i| i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::ArrowUp))
                    {
                        r.scroll_to_me(None);
                    }
                    if r.clicked() {
                        picked = Some(step.clone());
                    }
                }
            });
        if picked.is_some() {
            ui.close();
        }
        picked
    }

    fn recipe_ui(
        &mut self,
        ui: &mut Ui,
        palette: &Palette,
        recipes: &mut [Recipe],
        index: usize,
        input: &PreviewInput,
        events: &mut Vec<RecipeEvent>,
    ) {
        let mut changed = false;
        let taken = recipes
            .iter()
            .enumerate()
            .any(|(j, r)| j != index && r.name.eq_ignore_ascii_case(&recipes[index].name));
        let recipe = &mut recipes[index];
        ui.horizontal(|ui| {
            ui.add_sized([90.0, 20.0], egui::Label::new(tr("recipe-name")));
            let r = ui.add(
                egui::TextEdit::singleline(&mut recipe.name)
                    .id(Id::new("recipe-name"))
                    .desired_width(f32::INFINITY)
                    .margin(Margin::symmetric(6, 3)),
            );
            if std::mem::take(&mut self.focus_name) {
                r.request_focus();
            }
            if r.changed() && !recipe.name.trim().is_empty() {
                changed = true;
            }
        });
        if recipe.name.trim().is_empty() {
            ui.label(
                RichText::new(tr("recipe-no-name"))
                    .color(palette.modified)
                    .size(12.0),
            );
        } else if taken {
            ui.label(
                RichText::new(tr("recipe-name-taken"))
                    .color(palette.modified)
                    .size(12.0),
            );
        }
        let shortcut = self.shortcut_text.clone();
        self.shortcut_row(ui, palette, index, &recipe.id, shortcut.as_deref(), events);
        ui.horizontal(|ui| {
            ui.add_sized([90.0, 20.0], egui::Label::new(tr("recipe-description")));
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut recipe.description)
                        .id(Id::new("recipe-description"))
                        .desired_width(f32::INFINITY)
                        .margin(Margin::symmetric(6, 3)),
                )
                .changed();
        });
        ui.add_space(6.0);
        ui.label(RichText::new(tr("recipe-steps")).strong());

        // The step the last preview failed at, to show beside that step.
        let failed_at = self
            .preview
            .as_ref()
            .and_then(|(_, _, p)| p.text.as_ref().err().map(|f| (f.step, f.message.clone())));
        if self.show_after.is_some_and(|i| i >= recipe.steps.len()) {
            self.show_after = None;
        }

        // The steps, each with a switch, its settings when open, and buttons
        // to move or remove it.
        let mut action: Option<(usize, i32)> = None; // (step, -1 up / 1 down / 0 remove)
        let count = recipe.steps.len();
        egui::ScrollArea::vertical()
            .id_salt(("recipe-steps", index))
            .max_height(150.0)
            .min_scrolled_height(150.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if count == 0 {
                    ui.label(RichText::new(tr("recipe-no-steps")).color(palette.text_dim));
                }
                for (i, step) in recipe.steps.iter_mut().enumerate() {
                    let open = self.expanded == Some(i);
                    ui.horizontal(|ui| {
                        changed |= ui.checkbox(&mut step.enabled, "").changed();
                        ui.label(RichText::new(format!("{}.", i + 1)).color(palette.text_dim));
                        let arrow = if open {
                            ph::CARET_DOWN
                        } else {
                            ph::CARET_RIGHT
                        };
                        let icon = if step.op.is_command() {
                            format!(" {}", ph::TERMINAL)
                        } else {
                            String::new()
                        };
                        let color = if step.enabled {
                            palette.text
                        } else {
                            palette.text_dim
                        };
                        // The buttons first, from the right, so the title takes
                        // exactly the room left and rows never grow the list.
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icon_button(ui, ph::X, &tr("recipe-remove-step")) {
                                action = Some((i, 0));
                            }
                            ui.add_enabled_ui(i + 1 < count, |ui| {
                                if icon_button(ui, ph::ARROW_DOWN, &tr("recipe-move-down")) {
                                    action = Some((i, 1));
                                }
                            });
                            ui.add_enabled_ui(i > 0, |ui| {
                                if icon_button(ui, ph::ARROW_UP, &tr("recipe-move-up")) {
                                    action = Some((i, -1));
                                }
                            });
                            let width = ui.available_width().max(80.0);
                            let (rect, resp) = ui
                                .allocate_exact_size(Vec2::new(width, 22.0), egui::Sense::click());
                            if open || resp.hovered() {
                                let fill = if open {
                                    ui.visuals().selection.bg_fill
                                } else {
                                    ui.visuals().widgets.hovered.weak_bg_fill
                                };
                                ui.painter().rect_filled(rect, 4.0, fill);
                            }
                            let font = egui::TextStyle::Body.resolve(ui.style());
                            let mut job = egui::text::LayoutJob::simple_singleline(
                                format!("{arrow}{icon}  {}", step_label(&step.op)),
                                font,
                                color,
                            );
                            job.wrap = egui::text::TextWrapping::truncate_at_width(width - 12.0);
                            if !step.enabled
                                && let Some(section) = job.sections.first_mut()
                            {
                                section.format.strikethrough = egui::Stroke::new(1.0, color);
                            }
                            let galley = ui.fonts_mut(|f| f.layout_job(job));
                            let pos = rect.left_center() + Vec2::new(6.0, -galley.size().y / 2.0);
                            ui.painter().galley(pos, galley, color);
                            if resp.clicked() {
                                self.expanded = if open { None } else { Some(i) };
                                // The preview follows the step you pick.
                                self.show_after = Some(i);
                            }
                        });
                    });
                    if let Err(e) = step.op.check() {
                        ui.label(RichText::new(e).color(palette.modified).size(12.0));
                    } else if let Some(message) = failed_at
                        .as_ref()
                        .and_then(|(at, m)| (*at == i).then_some(m))
                    {
                        // Where the preview failed, under the step that failed.
                        ui.label(
                            RichText::new(format!("{} {message}", ph::WARNING))
                                .color(palette.modified)
                                .size(12.0),
                        );
                    }
                    if open {
                        egui::Frame::new()
                            .inner_margin(Margin {
                                left: 28,
                                right: 4,
                                top: 2,
                                bottom: 6,
                            })
                            .show(ui, |ui| {
                                let focus = std::mem::take(&mut self.focus_step);
                                changed |= step_settings(
                                    ui,
                                    Id::new(("recipe-step", index, i)),
                                    &mut step.op,
                                    palette,
                                    focus,
                                );
                            });
                    }
                }
            });
        if let Some((i, dir)) = action {
            match dir {
                0 => {
                    recipe.steps.remove(i);
                    self.expanded = None;
                }
                -1 => {
                    recipe.steps.swap(i, i - 1);
                    self.expanded = self.expanded.map(|e| if e == i { i - 1 } else { e });
                }
                _ => {
                    recipe.steps.swap(i, i + 1);
                    self.expanded = self.expanded.map(|e| if e == i { i + 1 } else { e });
                }
            }
            changed = true;
        }
        let add = ui.menu_button(format!("{} {}", ph::PLUS, tr("recipe-add-step")), |ui| {
            self.add_step_menu(ui, palette)
        });
        if add.response.clicked() {
            self.step_query.clear();
            self.step_pick = 0;
            self.focus_step_search = true;
        }
        if let Some(Some(step)) = add.inner {
            let needs_settings = matches!(
                step,
                Step::Replace { .. }
                    | Step::FilterLines { .. }
                    | Step::AffixLines { .. }
                    | Step::Command { .. }
            );
            recipe.steps.push(RecipeStep::from(step));
            if needs_settings {
                self.expanded = Some(recipe.steps.len() - 1);
                self.focus_step = true;
            }
            changed = true;
        }

        // The preview: the recipe run on the text, without its commands,
        // up to the step picked in "Show output after".
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("recipe-preview-input")).strong());
            let tab = if input.selection {
                tr("recipe-preview-selection")
            } else {
                tr("recipe-preview-active-tab")
            };
            ui.radio_value(&mut self.use_sample, false, tab);
            ui.radio_value(&mut self.use_sample, true, tr("recipe-sample-text"));
            ui.add_space(8.0);
            ui.label(tr("recipe-show-after"));
            let name = |i: Option<usize>| match i {
                Some(i) => t!("recipe-after-step", step = i + 1),
                None => tr("recipe-after-last-step"),
            };
            egui::ComboBox::from_id_salt(("recipe-show-after", index))
                .selected_text(name(self.show_after))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.show_after, None, name(None));
                    for (i, step) in recipe.steps.iter().enumerate() {
                        let label = format!("{}  {}", name(Some(i)), short(&step_label(&step.op)));
                        ui.selectable_value(&mut self.show_after, Some(i), label);
                    }
                });
        });
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            if self.use_sample {
                recipe.sample.hash(&mut h);
            } else {
                input.key.hash(&mut h);
            }
            self.use_sample.hash(&mut h);
            self.show_after.hash(&mut h);
            recipe.to_json().hash(&mut h);
            h.finish()
        };
        if self.preview.as_ref().is_none_or(|(k, _, _)| *k != key) {
            let before = if self.use_sample {
                recipe.sample.chars().take(PREVIEW_CHARS).collect()
            } else {
                (input.text)()
            };
            let env = Env {
                indent: input.indent,
                ..Default::default()
            };
            let result = recipe.preview_until(&before, &env, self.show_after);
            self.preview = Some((key, before, result));
        }
        let half = (ui.available_width() - 8.0) / 2.0;
        let mut sample_changed = false;
        if let Some((_, before, result)) = &self.preview {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    let label = if self.use_sample {
                        tr("recipe-sample-hint")
                    } else {
                        tr("recipe-before")
                    };
                    ui.label(RichText::new(label).size(12.0).color(palette.text_dim));
                    if self.use_sample {
                        sample_changed = sample_box(ui, &mut recipe.sample, half);
                    } else {
                        preview_box(ui, "recipe-before", before, half, None);
                    }
                });
                ui.vertical(|ui| {
                    let label = match self.show_after {
                        Some(i) => t!("recipe-output-after-step", step = i + 1),
                        None => tr("recipe-after"),
                    };
                    ui.label(RichText::new(label).size(12.0).color(palette.text_dim));
                    match &result.text {
                        Ok(after) => preview_box(ui, "recipe-after", after, half, None),
                        Err(f) => preview_box(
                            ui,
                            "recipe-after",
                            &t!(
                                "recipe-step-failed",
                                step = f.step + 1,
                                message = f.message.clone()
                            ),
                            half,
                            Some(palette.modified),
                        ),
                    }
                });
            });
            if let Some(i) = result.stopped_at {
                ui.label(
                    RichText::new(t!("recipe-preview-stopped", step = i + 1))
                        .size(12.0)
                        .color(palette.text_dim),
                );
            }
        }
        changed |= sample_changed;
        if let Some(m) = &self.message {
            ui.label(RichText::new(m).color(palette.modified).size(12.0));
        }
        if changed && !recipes[index].name.trim().is_empty() {
            events.push(RecipeEvent::Changed(index));
        }
    }
}

/// Asking before a recipe runs shell commands.
pub struct CommandConfirm {
    pub recipe: Recipe,
    pub all_tabs: bool,
    /// Don't ask again while the recipe's commands stay the same.
    pub remember: bool,
}

/// The answer to a [`CommandConfirm`].
pub enum ConfirmAnswer {
    Run,
    Cancel,
}

impl CommandConfirm {
    pub fn ui(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<ConfirmAnswer> {
        let mut answer = None;
        let response = egui::Modal::new(Id::new("recipe-confirm")).show(ctx, |ui| {
            ui.set_width(460.0);
            ui.heading(tr("recipe-confirm-title"));
            ui.add_space(4.0);
            ui.label(t!("recipe-confirm-text", name = self.recipe.name.clone()));
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .inner_margin(Margin::same(6))
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            for c in self.recipe.commands() {
                                ui.label(RichText::new(c).monospace());
                            }
                        });
                });
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr("recipe-confirm-hint"))
                    .size(12.0)
                    .color(palette.text_dim),
            );
            ui.checkbox(&mut self.remember, tr("recipe-confirm-remember"));
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 26.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if ui.button(tr("cancel")).clicked() {
                        answer = Some(ConfirmAnswer::Cancel);
                    }
                    if ui.button(tr("recipe-confirm-run")).clicked() {
                        answer = Some(ConfirmAnswer::Run);
                    }
                },
            );
        });
        if response.should_close() && answer.is_none() {
            answer = Some(ConfirmAnswer::Cancel);
        }
        answer
    }
}

/// While a recipe takes a moment: its name and a Stop button. True when
/// Stop was pressed.
pub fn running_ui(ctx: &egui::Context, palette: &Palette, name: &str) -> bool {
    let mut stop = false;
    egui::Modal::new(Id::new("recipe-running")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new(t!("recipe-running", name = name.to_owned())).color(palette.text),
            );
        });
        ui.add_space(6.0);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 26.0),
            Layout::right_to_left(Align::Center),
            |ui| {
                if ui.button(tr("stop")).clicked() {
                    stop = true;
                }
            },
        );
    });
    stop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_step_is_allowed_and_labelled() {
        for (group, steps) in catalog() {
            assert!(!group.is_empty());
            for step in steps {
                assert!(!step_label(&step).is_empty());
                // Steps that need text start empty; everything else runs as is.
                let needs_text = matches!(
                    step,
                    Step::Replace { .. } | Step::FilterLines { .. } | Step::Command { .. }
                );
                assert_eq!(step.check().is_err(), needs_text, "{step:?}");
            }
        }
    }
}
