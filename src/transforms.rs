//! The transform catalogue: every command that turns text into other text,
//! in the groups the Transform menu shows. The Transform menu, the editor's
//! right-click menu and the recipe window's Add Step menu are all built from
//! it, so they can't drift apart.

use crate::actions::Action;
use crate::editor::lines::{CaseKind, SortKind, TrimKind};
use crate::i18n::tr;
use crate::macros::MacroStep;
use crate::recipes::Step;
use crate::search::SearchOptions;
use crate::tools::convert::DataFormat;
use crate::tools::fields::{FieldKind, FieldOp, StringKind};
use crate::tools::json::JsonOp;
use crate::tools::mime::MimeOp;
use crate::tools::transform::{HashKind, NumberPad, TransformOp};
use crate::tools::xml::XmlOp;
use crate::transform_ui::DialogKind;

/// A group of the Transform menu, in menu order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupId {
    FindReplace,
    Lines,
    /// Columns and Fields (Ettin Edit 2.6).
    Columns,
    Case,
    ConvertTo,
    Encode,
    JsonXml,
    /// Filter Through Command; at the top of the Transform menu rather
    /// than in a submenu of its own.
    Command,
    /// Transforms that plugins add (Ettin Edit 2.3); hidden while empty.
    Plugins,
}

impl GroupId {
    pub fn label(self) -> String {
        tr(match self {
            GroupId::FindReplace => "recipe-group-replace",
            GroupId::Lines => "menu-transform-lines",
            GroupId::Columns => "menu-columns",
            GroupId::Case => "menu-convert-case",
            GroupId::ConvertTo => "menu-convert-to",
            GroupId::Encode => "recipe-group-encode",
            GroupId::JsonXml => "recipe-group-format",
            GroupId::Command => "recipe-group-command",
            GroupId::Plugins => "menu-plugin-transforms",
        })
    }
}

pub enum Entry {
    /// A menu item, and the recipe step that does the same thing when
    /// there is one (a dialog's step starts with empty settings).
    Item {
        label: String,
        action: Action,
        step: Option<Step>,
    },
    Heading(String),
    Separator,
    Submenu {
        label: String,
        entries: Vec<Entry>,
    },
}

pub struct Group {
    pub id: GroupId,
    pub entries: Vec<Entry>,
}

impl Group {
    /// The recipe steps of the group, in menu order.
    pub fn steps(&self) -> Vec<Step> {
        fn walk(entries: &[Entry], out: &mut Vec<Step>) {
            for e in entries {
                match e {
                    Entry::Item {
                        step: Some(step), ..
                    } => out.push(step.clone()),
                    Entry::Submenu { entries, .. } => walk(entries, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.entries, &mut out);
        out
    }

    /// The menu actions of the group, in menu order.
    #[cfg(test)]
    pub fn actions(&self) -> Vec<Action> {
        fn walk(entries: &[Entry], out: &mut Vec<Action>) {
            for e in entries {
                match e {
                    Entry::Item { action, .. } => out.push(action.clone()),
                    Entry::Submenu { entries, .. } => walk(entries, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.entries, &mut out);
        out
    }
}

fn item(label: &str, action: Action, step: Option<Step>) -> Entry {
    Entry::Item {
        label: tr(label),
        action,
        step,
    }
}

fn named(label: String, action: Action, step: Option<Step>) -> Entry {
    Entry::Item {
        label,
        action,
        step,
    }
}

pub const CASES: [(&str, CaseKind); 10] = [
    ("case-upper", CaseKind::Upper),
    ("case-lower", CaseKind::Lower),
    ("case-title", CaseKind::Title),
    ("case-sentence", CaseKind::Sentence),
    ("case-invert", CaseKind::Invert),
    ("case-camel", CaseKind::Camel),
    ("case-pascal", CaseKind::Pascal),
    ("case-snake", CaseKind::Snake),
    ("case-kebab", CaseKind::Kebab),
    ("case-constant", CaseKind::Constant),
];

/// MIME Tools' encodings, in groups.
pub const MIME: [&[(&str, MimeOp)]; 5] = [
    &[
        ("mime-base64-encode", MimeOp::Base64Encode),
        ("mime-base64-encode-wrapped", MimeOp::Base64EncodeWrapped),
        ("mime-base64-url-encode", MimeOp::Base64UrlEncode),
        ("mime-base64-decode", MimeOp::Base64Decode),
    ],
    &[
        ("mime-url-encode-component", MimeOp::UrlEncodeComponent),
        ("mime-url-encode", MimeOp::UrlEncode),
        ("mime-url-decode", MimeOp::UrlDecode),
    ],
    &[
        ("mime-qp-encode", MimeOp::QpEncode),
        ("mime-qp-decode", MimeOp::QpDecode),
    ],
    &[
        ("mime-html-encode", MimeOp::HtmlEncode),
        ("mime-html-encode-all", MimeOp::HtmlEncodeAll),
        ("mime-html-decode", MimeOp::HtmlDecode),
    ],
    &[("mime-saml-decode", MimeOp::SamlDecode)],
];

pub const HASHES: [HashKind; 5] = [
    HashKind::Md5,
    HashKind::Sha1,
    HashKind::Sha256,
    HashKind::Sha512,
    HashKind::Crc32,
];

/// A Convert to target's name in menus.
pub fn target_label(f: DataFormat) -> String {
    match f {
        DataFormat::Markdown => tr("convert-markdown-table"),
        f => f.name().to_owned(),
    }
}

fn convert_to(to: DataFormat) -> Action {
    Action::Transform(TransformOp::ConvertTo { to, from: None })
}

fn lines() -> Vec<Entry> {
    let sorts = [
        (["sort-lexical-asc", "sort-lexical-desc"], SortKind::Lexical),
        (
            ["sort-ignore-case-asc", "sort-ignore-case-desc"],
            SortKind::IgnoreCase,
        ),
        (["sort-integer-asc", "sort-integer-desc"], SortKind::Integer),
        (["sort-decimal-asc", "sort-decimal-desc"], SortKind::Decimal),
        (["sort-length-asc", "sort-length-desc"], SortKind::Length),
        (["sort-natural-asc", "sort-natural-desc"], SortKind::Natural),
    ]
    .into_iter()
    .flat_map(|(labels, kind)| {
        labels
            .into_iter()
            .zip([false, true])
            .map(move |(label, desc)| (label, kind, desc))
    })
    .map(|(label, kind, desc)| {
        // One Sort Lines step in Add Step; its settings pick the order.
        let step = (kind == SortKind::Lexical && !desc).then_some(Step::SortLines {
            kind,
            descending: false,
        });
        item(label, Action::SortLines { kind, desc }, step)
    })
    .collect();
    vec![
        item(
            "keep-matching-lines-dots",
            Action::TransformDialog(DialogKind::FilterLines { keep: true }),
            Some(Step::FilterLines {
                pattern: String::new(),
                regex: false,
                match_case: false,
                keep: true,
            }),
        ),
        item(
            "remove-matching-lines-dots",
            Action::TransformDialog(DialogKind::FilterLines { keep: false }),
            Some(Step::FilterLines {
                pattern: String::new(),
                regex: false,
                match_case: false,
                keep: false,
            }),
        ),
        item(
            "count-distinct-lines",
            Action::Transform(TransformOp::CountLines),
            Some(Step::CountDistinct),
        ),
        Entry::Separator,
        Entry::Submenu {
            label: tr("menu-sort-lines"),
            entries: sorts,
        },
        item(
            "reverse-lines",
            Action::ReverseLines,
            Some(Step::ReverseLines),
        ),
        item(
            "remove-duplicate-lines",
            Action::RemoveDuplicateLines,
            Some(Step::RemoveDuplicates { consecutive: false }),
        ),
        item(
            "remove-consecutive-duplicates",
            Action::RemoveConsecutiveDuplicates,
            None,
        ),
        item(
            "remove-empty-lines",
            Action::RemoveEmptyLines { blank: false },
            None,
        ),
        item(
            "remove-blank-lines",
            Action::RemoveEmptyLines { blank: true },
            Some(Step::RemoveEmptyLines { blank: true }),
        ),
        Entry::Submenu {
            label: tr("menu-trim"),
            entries: vec![
                item(
                    "trim-both",
                    Action::Trim(TrimKind::Both),
                    Some(Step::Trim {
                        kind: TrimKind::Both,
                    }),
                ),
                item("trim-trailing", Action::Trim(TrimKind::Trailing), None),
                item("trim-leading", Action::Trim(TrimKind::Leading), None),
            ],
        },
        item("eol-to-space", Action::EolToSpace, Some(Step::JoinLines)),
        Entry::Separator,
        item(
            "affix-lines-dots",
            Action::TransformDialog(DialogKind::AffixLines),
            Some(Step::AffixLines {
                prefix: String::new(),
                suffix: String::new(),
                skip_empty: true,
            }),
        ),
        item(
            "number-lines-dots",
            Action::TransformDialog(DialogKind::NumberLines),
            Some(Step::NumberLines {
                start: 1,
                step: 1,
                separator: ". ".into(),
                pad: NumberPad::None,
                skip_empty: false,
            }),
        ),
        item(
            "reflow-dots",
            Action::TransformDialog(DialogKind::Reflow),
            Some(Step::Reflow { width: 80 }),
        ),
        Entry::Separator,
        item(
            "tabs-to-spaces",
            Action::TabsToSpaces,
            Some(Step::TabsToSpaces),
        ),
        item(
            "leading-spaces-to-tabs",
            Action::SpacesToTabs { all: false },
            Some(Step::SpacesToTabs { all: false }),
        ),
        item(
            "all-spaces-to-tabs",
            Action::SpacesToTabs { all: true },
            None,
        ),
    ]
}

/// Columns and Fields: the dialogs, then Escape, Unescape and Unix Time.
fn columns() -> Vec<Entry> {
    let dialog = |kind: FieldKind| {
        item(
            kind.keys().0,
            Action::TransformDialog(DialogKind::Field(kind)),
            Some(Step::Field(kind.default_op())),
        )
    };
    let op = |label: &str, op: FieldOp| {
        item(
            label,
            Action::Transform(TransformOp::Field(op.clone())),
            Some(Step::Field(op)),
        )
    };
    let strings = |unescape: bool| -> Vec<Entry> {
        StringKind::ALL
            .iter()
            .map(|&language| {
                let op = if unescape {
                    FieldOp::UnescapeString { language }
                } else {
                    FieldOp::EscapeString { language }
                };
                named(
                    language.name().to_owned(),
                    Action::Transform(TransformOp::Field(op.clone())),
                    Some(Step::Field(op)),
                )
            })
            .collect()
    };
    vec![
        dialog(FieldKind::Extract),
        Entry::Separator,
        dialog(FieldKind::Align),
        dialog(FieldKind::Select),
        dialog(FieldKind::SortBy),
        Entry::Separator,
        dialog(FieldKind::Split),
        dialog(FieldKind::Join),
        Entry::Separator,
        Entry::Submenu {
            label: tr("menu-escape-string"),
            entries: strings(false),
        },
        Entry::Submenu {
            label: tr("menu-unescape-string"),
            entries: strings(true),
        },
        Entry::Submenu {
            label: tr("menu-unix-time"),
            entries: vec![
                op("unix-to-date-utc", FieldOp::UnixTimeToDate { utc: true }),
                op("unix-to-date-local", FieldOp::UnixTimeToDate { utc: false }),
                op("date-to-unix-seconds", FieldOp::DateToUnixTime { millis: false }),
                op("date-to-unix-millis", FieldOp::DateToUnixTime { millis: true }),
            ],
        },
    ]
}

fn cases() -> Vec<Entry> {
    let mut out = Vec::new();
    for (i, &(label, kind)) in CASES.iter().enumerate() {
        if i == 5 {
            out.push(Entry::Separator);
        }
        out.push(item(
            label,
            Action::ConvertCase(kind),
            Some(Step::Case { kind }),
        ));
    }
    out
}

fn encode() -> Vec<Entry> {
    let mut out = Vec::new();
    for (i, group) in MIME.iter().enumerate() {
        if i > 0 {
            out.push(Entry::Separator);
        }
        for &(label, kind) in group.iter() {
            out.push(item(label, Action::Mime(kind), Some(Step::Mime { kind })));
        }
    }
    out.extend([
        Entry::Separator,
        item(
            "hex-encode",
            Action::Transform(TransformOp::HexEncode),
            Some(Step::HexEncode),
        ),
        item(
            "hex-decode",
            Action::Transform(TransformOp::HexDecode),
            Some(Step::HexDecode),
        ),
        item(
            "jwt-decode",
            Action::Transform(TransformOp::JwtDecode),
            Some(Step::JwtDecode),
        ),
        Entry::Separator,
        Entry::Submenu {
            label: tr("menu-hash"),
            entries: HASHES
                .iter()
                .map(|&kind| {
                    named(
                        kind.name().to_owned(),
                        Action::Transform(TransformOp::Hash(kind)),
                        Some(Step::Hash { kind }),
                    )
                })
                .collect(),
        },
    ]);
    out
}

fn json_xml() -> Vec<Entry> {
    vec![
        Entry::Heading(tr("toolbar-group-json")),
        item(
            "json-format-named",
            Action::Json(JsonOp::Format),
            Some(Step::Json {
                kind: JsonOp::Format,
            }),
        ),
        item(
            "json-minify-named",
            Action::Json(JsonOp::Minify),
            Some(Step::Json {
                kind: JsonOp::Minify,
            }),
        ),
        item(
            "json-sort-keys",
            Action::Json(JsonOp::SortKeys),
            Some(Step::Json {
                kind: JsonOp::SortKeys,
            }),
        ),
        Entry::Heading(tr("toolbar-group-xml")),
        item(
            "xml-pretty-print",
            Action::Xml(XmlOp::PrettyPrint),
            Some(Step::Xml {
                kind: XmlOp::PrettyPrint,
            }),
        ),
        item(
            "xml-pretty-print-attributes",
            Action::Xml(XmlOp::PrettyPrintAttributes),
            Some(Step::Xml {
                kind: XmlOp::PrettyPrintAttributes,
            }),
        ),
        item(
            "xml-linearize",
            Action::Xml(XmlOp::Linearize),
            Some(Step::Xml {
                kind: XmlOp::Linearize,
            }),
        ),
        item(
            "xml-escape",
            Action::Xml(XmlOp::Escape),
            Some(Step::Xml {
                kind: XmlOp::Escape,
            }),
        ),
        item(
            "xml-unescape",
            Action::Xml(XmlOp::Unescape),
            Some(Step::Xml {
                kind: XmlOp::Unescape,
            }),
        ),
    ]
}

/// Every transform, by group, in menu order.
pub fn catalogue() -> Vec<Group> {
    let group = |id, entries| Group { id, entries };
    vec![
        group(
            GroupId::FindReplace,
            vec![item(
                "replace-dots",
                Action::Replace,
                Some(Step::Replace {
                    find: String::new(),
                    replace: String::new(),
                    regex: false,
                    match_case: false,
                    whole_word: false,
                    extended: false,
                }),
            )],
        ),
        group(GroupId::Lines, lines()),
        group(GroupId::Columns, columns()),
        group(GroupId::Case, cases()),
        group(
            GroupId::ConvertTo,
            DataFormat::TARGETS
                .iter()
                .map(|&to| {
                    named(
                        target_label(to),
                        convert_to(to),
                        Some(Step::ConvertTo { to, from: None }),
                    )
                })
                .collect(),
        ),
        group(GroupId::Encode, encode()),
        group(GroupId::JsonXml, json_xml()),
        group(
            GroupId::Command,
            vec![item(
                "pipe-dots",
                Action::TransformDialog(DialogKind::Pipe),
                Some(Step::Command {
                    command: String::new(),
                }),
            )],
        ),
        group(GroupId::Plugins, plugin_entries()),
    ]
}

/// The running plugins' transforms, under a heading for each plugin.
fn plugin_entries() -> Vec<Entry> {
    let mut out = Vec::new();
    let mut last = None;
    for t in crate::plugins::known_transforms() {
        if last.as_ref() != Some(&t.plugin) {
            out.push(Entry::Heading(t.plugin_name.clone()));
            last = Some(t.plugin.clone());
        }
        out.push(named(
            t.title.clone(),
            Action::PluginTransform {
                plugin: t.plugin.clone(),
                transform: t.id.clone(),
            },
            Some(Step::Plugin {
                plugin: t.plugin,
                transform: t.id,
                options: t.options,
            }),
        ));
    }
    out
}

/// The group `id` of the catalogue.
pub fn group(id: GroupId) -> Group {
    catalogue()
        .into_iter()
        .find(|g| g.id == id)
        .expect("every group is in the catalogue")
}

/// Whether `action` is one of the catalogue's transforms.
#[cfg(test)]
pub fn is_transform(action: &Action) -> bool {
    catalogue().iter().any(|g| g.actions().contains(action))
}

/// The recipe step a command adds while a recipe is being recorded, or that
/// a macro step becomes in a recipe. Replace All takes the find bar's text
/// and options.
pub fn step_for(action: &Action, find: &str, replace: &str, opts: &SearchOptions) -> Option<Step> {
    Some(match action {
        Action::ReplaceAll if !find.is_empty() => Step::Replace {
            find: find.to_owned(),
            replace: replace.to_owned(),
            regex: opts.regex,
            match_case: opts.match_case,
            whole_word: opts.whole_word,
            extended: opts.extended && !opts.regex,
        },
        Action::Transform(op) => match op.clone() {
            TransformOp::FilterLines {
                pattern,
                regex,
                match_case,
                keep,
            } => Step::FilterLines {
                pattern,
                regex,
                match_case,
                keep,
            },
            TransformOp::CountLines => Step::CountDistinct,
            TransformOp::AffixLines {
                prefix,
                suffix,
                skip_empty,
            } => Step::AffixLines {
                prefix,
                suffix,
                skip_empty,
            },
            TransformOp::NumberLines {
                start,
                step,
                separator,
                pad,
                skip_empty,
            } => Step::NumberLines {
                start,
                step,
                separator,
                pad,
                skip_empty,
            },
            TransformOp::Reflow { width } => Step::Reflow { width },
            TransformOp::Convert { from, to } => Step::Convert { from, to },
            TransformOp::ConvertTo { to, from } => Step::ConvertTo { to, from },
            TransformOp::Hash(kind) => Step::Hash { kind },
            TransformOp::HexEncode => Step::HexEncode,
            TransformOp::HexDecode => Step::HexDecode,
            TransformOp::JwtDecode => Step::JwtDecode,
            TransformOp::Pipe { command } => Step::Command { command },
            TransformOp::Field(op) => Step::Field(op),
        },
        Action::SortLines { kind, desc } => Step::SortLines {
            kind: *kind,
            descending: *desc,
        },
        Action::ReverseLines => Step::ReverseLines,
        Action::RemoveDuplicateLines => Step::RemoveDuplicates { consecutive: false },
        Action::RemoveConsecutiveDuplicates => Step::RemoveDuplicates { consecutive: true },
        Action::RemoveEmptyLines { blank } => Step::RemoveEmptyLines { blank: *blank },
        Action::Trim(kind) => Step::Trim { kind: *kind },
        Action::EolToSpace => Step::JoinLines,
        Action::TabsToSpaces => Step::TabsToSpaces,
        Action::SpacesToTabs { all } => Step::SpacesToTabs { all: *all },
        Action::ConvertCase(kind) => Step::Case { kind: *kind },
        Action::Mime(kind) => Step::Mime { kind: *kind },
        Action::Json(kind @ (JsonOp::Format | JsonOp::Minify | JsonOp::SortKeys)) => {
            Step::Json { kind: *kind }
        }
        Action::Xml(
            kind @ (XmlOp::PrettyPrint
            | XmlOp::PrettyPrintAttributes
            | XmlOp::Linearize
            | XmlOp::Escape
            | XmlOp::Unescape),
        ) => Step::Xml { kind: *kind },
        _ => return None,
    })
}

/// The recipe steps a recorded macro becomes, or the index of the first
/// macro step that has no recipe step (typing, moving the caret, a
/// command that isn't a transform).
pub fn macro_steps(steps: &[MacroStep]) -> Result<Vec<Step>, usize> {
    steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            match s {
                MacroStep::Action(a) => step_for(a, "", "", &SearchOptions::default()),
                MacroStep::ReplaceAll {
                    find,
                    replace,
                    options,
                } => step_for(&Action::ReplaceAll, find, replace, options),
                _ => None,
            }
            .ok_or(i)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_actions() -> Vec<Action> {
        catalogue().iter().flat_map(|g| g.actions()).collect()
    }

    /// Every command that a recipe step can do is in exactly one group.
    #[test]
    fn every_transform_is_in_exactly_one_group() {
        let actions = all_actions();
        for (i, a) in actions.iter().enumerate() {
            assert!(!actions[..i].contains(a), "{a:?} is listed twice");
        }
        let mut expected: Vec<Action> = vec![
            Action::Replace,
            Action::TransformDialog(DialogKind::Pipe),
            Action::TransformDialog(DialogKind::FilterLines { keep: true }),
            Action::TransformDialog(DialogKind::FilterLines { keep: false }),
            Action::TransformDialog(DialogKind::AffixLines),
            Action::TransformDialog(DialogKind::NumberLines),
            Action::TransformDialog(DialogKind::Reflow),
            Action::Transform(TransformOp::CountLines),
            Action::Transform(TransformOp::HexEncode),
            Action::Transform(TransformOp::HexDecode),
            Action::Transform(TransformOp::JwtDecode),
            Action::ReverseLines,
            Action::RemoveDuplicateLines,
            Action::RemoveConsecutiveDuplicates,
            Action::RemoveEmptyLines { blank: false },
            Action::RemoveEmptyLines { blank: true },
            Action::EolToSpace,
            Action::TabsToSpaces,
            Action::SpacesToTabs { all: false },
            Action::SpacesToTabs { all: true },
            Action::Json(JsonOp::Format),
            Action::Json(JsonOp::Minify),
            Action::Json(JsonOp::SortKeys),
        ];
        for kind in [TrimKind::Both, TrimKind::Leading, TrimKind::Trailing] {
            expected.push(Action::Trim(kind));
        }
        for kind in [
            SortKind::Lexical,
            SortKind::IgnoreCase,
            SortKind::Integer,
            SortKind::Decimal,
            SortKind::Length,
            SortKind::Natural,
        ] {
            for desc in [false, true] {
                expected.push(Action::SortLines { kind, desc });
            }
        }
        expected.extend(CASES.iter().map(|(_, k)| Action::ConvertCase(*k)));
        expected.extend(
            MIME.iter()
                .flat_map(|g| g.iter().map(|(_, k)| Action::Mime(*k))),
        );
        expected.extend(
            HASHES
                .iter()
                .map(|k| Action::Transform(TransformOp::Hash(*k))),
        );
        expected.extend(DataFormat::TARGETS.iter().map(|&to| convert_to(to)));
        for op in [
            XmlOp::PrettyPrint,
            XmlOp::PrettyPrintAttributes,
            XmlOp::Linearize,
            XmlOp::Escape,
            XmlOp::Unescape,
        ] {
            expected.push(Action::Xml(op));
        }
        for kind in FieldKind::ALL {
            expected.push(Action::TransformDialog(DialogKind::Field(kind)));
        }
        for language in StringKind::ALL {
            expected.push(Action::Transform(TransformOp::Field(FieldOp::EscapeString {
                language,
            })));
            expected.push(Action::Transform(TransformOp::Field(
                FieldOp::UnescapeString { language },
            )));
        }
        for op in [
            FieldOp::UnixTimeToDate { utc: true },
            FieldOp::UnixTimeToDate { utc: false },
            FieldOp::DateToUnixTime { millis: false },
            FieldOp::DateToUnixTime { millis: true },
        ] {
            expected.push(Action::Transform(TransformOp::Field(op)));
        }
        for a in &expected {
            assert!(actions.contains(a), "{a:?} isn't in the Transform menu");
        }
        assert_eq!(actions.len(), expected.len(), "{actions:?}");
    }

    /// A menu item's step is the step recording that command gives, so a
    /// recipe step and its menu item do the same thing.
    #[test]
    fn items_and_their_steps_agree() {
        let same = |a: &Step, b: &Step| std::mem::discriminant(a) == std::mem::discriminant(b);
        for group in catalogue() {
            fn walk(entries: &[Entry], same: &dyn Fn(&Step, &Step) -> bool) {
                for e in entries {
                    match e {
                        Entry::Item {
                            action,
                            step: Some(step),
                            ..
                        } => {
                            if let Some(recorded) =
                                step_for(action, "x", "y", &SearchOptions::default())
                            {
                                assert!(same(&recorded, step), "{action:?}");
                            }
                        }
                        Entry::Submenu { entries, .. } => walk(entries, same),
                        _ => {}
                    }
                }
            }
            walk(&group.entries, &same);
        }
    }

    /// Every kind of recipe step (but the 2.0.0 Convert one) can be added
    /// from the catalogue.
    #[test]
    fn every_step_kind_is_offered() {
        let steps: Vec<Step> = catalogue().iter().flat_map(|g| g.steps()).collect();
        let kinds = [
            Step::CountDistinct,
            Step::HexEncode,
            Step::HexDecode,
            Step::JwtDecode,
            Step::ReverseLines,
            Step::JoinLines,
            Step::TabsToSpaces,
            Step::Reflow { width: 1 },
            Step::Hash {
                kind: HashKind::Md5,
            },
            Step::ConvertTo {
                to: DataFormat::Json,
                from: None,
            },
            Step::Command {
                command: String::new(),
            },
            Step::SpacesToTabs { all: false },
            Step::RemoveDuplicates { consecutive: false },
            Step::RemoveEmptyLines { blank: true },
            Step::Trim {
                kind: TrimKind::Both,
            },
            Step::Case {
                kind: CaseKind::Upper,
            },
            Step::Mime {
                kind: MimeOp::UrlDecode,
            },
            Step::Json {
                kind: JsonOp::Format,
            },
            Step::Xml {
                kind: XmlOp::Escape,
            },
            Step::SortLines {
                kind: SortKind::Lexical,
                descending: false,
            },
            Step::Field(FieldKind::Extract.default_op()),
        ];
        for k in &kinds {
            assert!(
                steps
                    .iter()
                    .any(|s| std::mem::discriminant(s) == std::mem::discriminant(k)),
                "{k:?}"
            );
        }
    }

    #[test]
    fn plugin_transforms_wait_for_plugins() {
        assert!(group(GroupId::Plugins).entries.is_empty());
    }

    /// Sort Lines + Trim + Replace All, recorded as a macro, is a three-step
    /// recipe; typing isn't.
    #[test]
    fn a_macro_of_transforms_becomes_a_recipe() {
        let options = SearchOptions {
            regex: true,
            ..Default::default()
        };
        let steps = vec![
            MacroStep::Action(Action::SortLines {
                kind: SortKind::Lexical,
                desc: false,
            }),
            MacroStep::Action(Action::Trim(TrimKind::Both)),
            MacroStep::ReplaceAll {
                find: "a+".into(),
                replace: "b".into(),
                options,
            },
        ];
        let recipe = macro_steps(&steps).unwrap();
        assert_eq!(
            recipe,
            vec![
                Step::SortLines {
                    kind: SortKind::Lexical,
                    descending: false
                },
                Step::Trim {
                    kind: TrimKind::Both
                },
                Step::Replace {
                    find: "a+".into(),
                    replace: "b".into(),
                    regex: true,
                    match_case: false,
                    whole_word: false,
                    extended: false,
                },
            ]
        );
        let mut typed = steps.clone();
        typed.insert(1, MacroStep::Text("hello".into()));
        assert_eq!(macro_steps(&typed), Err(1));
        // Replace All from before 2.1 didn't keep its text.
        assert_eq!(
            macro_steps(&[MacroStep::Action(Action::ReplaceAll)]),
            Err(0)
        );
    }
}
