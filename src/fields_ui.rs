//! The settings form of the Columns and Fields commands (Ettin Edit 2.6),
//! shared by their Transform dialogs and the Recipes window's step editor,
//! plus their names and one-line descriptions.

use egui::{Id, Margin, Response, RichText, Ui};

use crate::i18n::tr;
use crate::tools::fields::{FieldOp, FieldOrder};

/// What drawing the form did.
#[derive(Default)]
pub struct Form {
    pub changed: bool,
    /// The first text box, for the keyboard focus and Enter.
    pub first: Option<Response>,
    /// Enter was pressed in a text box.
    pub enter: bool,
}

const LABEL_W: f32 = 130.0;

fn text(ui: &mut Ui, id: Id, label: &str, value: &mut String, hint: &str, form: &mut Form) {
    ui.horizontal(|ui| {
        ui.add_sized([LABEL_W, 20.0], egui::Label::new(label));
        let r = ui.add(
            egui::TextEdit::singleline(value)
                .id(id.with(label))
                .font(egui::TextStyle::Monospace)
                .hint_text(hint)
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(6, 3)),
        );
        form.changed |= r.changed();
        form.enter |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if form.first.is_none() {
            form.first = Some(r);
        }
    });
}

fn number(ui: &mut Ui, label: &str, value: &mut usize, min: usize, form: &mut Form) {
    ui.horizontal(|ui| {
        ui.add_sized([LABEL_W, 20.0], egui::Label::new(label));
        form.changed |= ui
            .add(egui::DragValue::new(value).range(min..=999))
            .changed();
    });
}

fn note(ui: &mut Ui, text: String) {
    ui.label(RichText::new(text).size(12.0).weak());
}

/// The settings of `op`. `id` keeps text boxes apart between steps.
pub fn form(ui: &mut Ui, id: Id, op: &mut FieldOp) -> Form {
    let mut f = Form::default();
    let delim_hint = tr("field-delimiter-hint");
    match op {
        FieldOp::ExtractMatches {
            pattern,
            match_case,
            group,
            unique,
        } => {
            text(ui, id, &tr("field-pattern"), pattern, r"ERR-\d+", &mut f);
            number(ui, &tr("field-group"), group, 0, &mut f);
            note(ui, tr("field-group-hint"));
            ui.horizontal(|ui| {
                f.changed |= ui.checkbox(match_case, tr("match-case")).changed();
                f.changed |= ui.checkbox(unique, tr("field-unique")).changed();
            });
        }
        FieldOp::AlignColumns { delimiter, spaced } => {
            text(ui, id, &tr("field-delimiter"), delimiter, &delim_hint, &mut f);
            f.changed |= ui.checkbox(spaced, tr("field-spaced")).changed();
        }
        FieldOp::SelectFields {
            delimiter,
            fields,
            remove,
            output_delimiter,
        } => {
            text(ui, id, &tr("field-fields"), fields, "2,1 · 3-5 · 4-", &mut f);
            text(ui, id, &tr("field-delimiter"), delimiter, &delim_hint, &mut f);
            ui.horizontal(|ui| {
                f.changed |= ui.radio_value(remove, false, tr("field-keep")).changed();
                f.changed |= ui.radio_value(remove, true, tr("field-remove")).changed();
            });
            text(
                ui,
                id,
                &tr("field-output-delimiter"),
                output_delimiter,
                &tr("field-output-same"),
                &mut f,
            );
        }
        FieldOp::SortByField {
            delimiter,
            field,
            order,
            descending,
        } => {
            text(ui, id, &tr("field-delimiter"), delimiter, &delim_hint, &mut f);
            number(ui, &tr("field-field"), field, 1, &mut f);
            ui.horizontal(|ui| {
                for (o, key) in [
                    (FieldOrder::Natural, "field-order-natural"),
                    (FieldOrder::Text, "field-order-text"),
                    (FieldOrder::Number, "field-order-number"),
                ] {
                    f.changed |= ui.radio_value(order, o, tr(key)).changed();
                }
            });
            f.changed |= ui.checkbox(descending, tr("recipe-descending")).changed();
        }
        FieldOp::SplitLines { separator, trim } => {
            text(ui, id, &tr("field-split-at"), separator, ",", &mut f);
            f.changed |= ui.checkbox(trim, tr("field-trim")).changed();
        }
        FieldOp::JoinWith {
            separator,
            quote,
            skip_empty,
        } => {
            text(ui, id, &tr("field-separator"), separator, ", ", &mut f);
            text(ui, id, &tr("field-quote"), quote, "'", &mut f);
            note(ui, tr("field-quote-hint"));
            f.changed |= ui.checkbox(skip_empty, tr("skip-empty-lines")).changed();
        }
        FieldOp::EscapeString { language } | FieldOp::UnescapeString { language } => {
            let mut changed = false;
            egui::ComboBox::from_id_salt(id.with("language"))
                .selected_text(language.name())
                .show_ui(ui, |ui| {
                    for k in crate::tools::fields::StringKind::ALL {
                        changed |= ui.selectable_value(language, k, k.name()).changed();
                    }
                });
            f.changed |= changed;
        }
        FieldOp::UnixTimeToDate { utc } => {
            f.changed |= ui.radio_value(utc, true, tr("field-utc")).changed();
            f.changed |= ui.radio_value(utc, false, tr("field-local")).changed();
        }
        FieldOp::DateToUnixTime { millis } => {
            f.changed |= ui.radio_value(millis, false, tr("field-seconds")).changed();
            f.changed |= ui.radio_value(millis, true, tr("field-millis")).changed();
        }
    }
    f
}

/// The translation key of a dialog's explanation.
pub fn hint_key(kind: crate::tools::fields::FieldKind) -> &'static str {
    use crate::tools::fields::FieldKind;
    match kind {
        FieldKind::Extract => "field-hint-extract",
        FieldKind::Align => "field-hint-align",
        FieldKind::Select => "field-hint-select",
        FieldKind::SortBy => "field-hint-sort",
        FieldKind::Split => "field-hint-split",
        FieldKind::Join => "field-hint-join",
    }
}

fn shown(s: &str) -> String {
    let s = s.replace('\t', "\\t");
    if s.is_empty() {
        tr("field-spaces")
    } else if s.chars().count() > 30 {
        s.chars().take(30).collect::<String>() + "…"
    } else {
        s
    }
}

/// Its name, as in the menu (without settings).
pub fn name(op: &FieldOp) -> String {
    if let Some(kind) = op.kind() {
        return tr(kind.keys().1);
    }
    match op {
        FieldOp::EscapeString { .. } => tr("menu-escape-string"),
        FieldOp::UnescapeString { .. } => tr("menu-unescape-string"),
        _ => summary(op),
    }
}

/// A one-line description with its settings, for recipe steps.
pub fn summary(op: &FieldOp) -> String {
    match op {
        FieldOp::ExtractMatches { pattern, .. } => {
            t!("recipe-step-extract", pattern = shown(pattern))
        }
        FieldOp::AlignColumns { delimiter, .. } => {
            t!("recipe-step-align", delimiter = shown(delimiter))
        }
        FieldOp::SelectFields { fields, remove, .. } => {
            if *remove {
                t!("recipe-step-remove-fields", fields = fields.clone())
            } else {
                t!("recipe-step-select-fields", fields = fields.clone())
            }
        }
        FieldOp::SortByField { field, .. } => t!("recipe-step-sort-field", field = *field),
        FieldOp::SplitLines { separator, .. } => {
            t!("recipe-step-split", separator = shown(separator))
        }
        FieldOp::JoinWith {
            separator, quote, ..
        } => t!(
            "recipe-step-join-with",
            separator = shown(separator),
            quote = quote.clone()
        ),
        FieldOp::EscapeString { language } => {
            t!("recipe-step-escape", language = language.name())
        }
        FieldOp::UnescapeString { language } => {
            t!("recipe-step-unescape", language = language.name())
        }
        FieldOp::UnixTimeToDate { utc: true } => tr("unix-to-date-utc"),
        FieldOp::UnixTimeToDate { utc: false } => tr("unix-to-date-local"),
        FieldOp::DateToUnixTime { millis: false } => tr("date-to-unix-seconds"),
        FieldOp::DateToUnixTime { millis: true } => tr("date-to-unix-millis"),
    }
}
