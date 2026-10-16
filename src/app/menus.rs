//! The menus that are drawn in more than one place: Edit and Tools are in
//! the menu bar and also in the editor's right-click menu.

use egui::Ui;

use super::EttinApp;
use crate::actions::{self, Action};
use crate::editor::lines::{CaseKind, SortKind, TrimKind};
use crate::menu::{self, MenuEntry};
use crate::tools::json::JsonOp;
use crate::tools::xml::XmlOp;

/// Collects the action behind each menu item; an item's id is its index.
pub(super) struct MenuModel<'c> {
    ctx: &'c egui::Context,
    shortcuts: Vec<(egui::KeyboardShortcut, Action)>,
    actions: Vec<Action>,
}

impl<'c> MenuModel<'c> {
    pub fn new(ctx: &'c egui::Context) -> Self {
        Self {
            ctx,
            shortcuts: actions::all_shortcuts(),
            actions: Vec::new(),
        }
    }

    fn entry(
        &mut self,
        label: &str,
        enabled: bool,
        checked: Option<bool>,
        action: Action,
    ) -> MenuEntry {
        let entry = MenuEntry::Item {
            label: label.to_owned(),
            shortcut: actions::shortcut_label_in(&self.shortcuts, self.ctx, &action),
            enabled,
            checked,
            id: self.actions.len(),
        };
        self.actions.push(action);
        entry
    }

    fn item(&mut self, label: &str, action: Action) -> MenuEntry {
        self.entry(label, true, None, action)
    }

    fn item_if(&mut self, label: &str, enabled: bool, action: Action) -> MenuEntry {
        self.entry(label, enabled, None, action)
    }

    /// An item whose shortcut isn't one of Ettin Edit's own (a plugin's).
    fn item_with_shortcut(
        &mut self,
        label: &str,
        shortcut: Option<&egui::KeyboardShortcut>,
        action: Action,
    ) -> MenuEntry {
        let entry = MenuEntry::Item {
            label: label.to_owned(),
            shortcut: shortcut.map(|s| self.ctx.format_shortcut(s)),
            enabled: true,
            checked: None,
            id: self.actions.len(),
        };
        self.actions.push(action);
        entry
    }

    fn toggle(&mut self, label: &str, checked: bool, action: Action) -> MenuEntry {
        self.entry(label, true, Some(checked), action)
    }

    /// The action of a clicked item id.
    pub fn action(&self, id: usize) -> Option<Action> {
        self.actions.get(id).cloned()
    }
}

fn submenu(label: &str, entries: Vec<MenuEntry>) -> MenuEntry {
    MenuEntry::Submenu {
        label: label.to_owned(),
        entries,
    }
}

impl EttinApp {
    /// Draws the Edit or Tools menu into an open menu-bar menu.
    pub(super) fn shared_menu(&mut self, ui: &mut Ui, tools: bool) {
        let ctx = ui.ctx().clone();
        let mut m = MenuModel::new(&ctx);
        let entries = if tools {
            self.tools_menu(&mut m)
        } else {
            self.edit_menu(&mut m, self.active)
        };
        if let Some(action) = menu::show(ui, &entries).and_then(|id| m.action(id)) {
            self.pending.push(action);
        }
    }

    /// The right-click menu of tab `index`'s editor: the Edit menu, then Tools.
    pub(super) fn context_menu(&self, m: &mut MenuModel, index: usize) -> Vec<MenuEntry> {
        let mut entries = self.edit_menu(m, index);
        entries.push(MenuEntry::Separator);
        entries.push(submenu(&t!("menu-tools"), self.tools_menu(m)));
        let plugins = self.plugin_commands_menu(m);
        if !plugins.is_empty() {
            entries.push(submenu(&t!("menu-plugins"), plugins));
        }
        entries
    }

    /// The running plugins' commands, one submenu per plugin.
    fn plugin_commands_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        self.plugins
            .plugins
            .iter()
            .filter(|p| p.is_running() && !p.commands.is_empty())
            .map(|p| {
                let items = p
                    .commands
                    .iter()
                    .map(|c| {
                        let action = Action::PluginCommand {
                            plugin: p.id.clone(),
                            command: c.id.clone(),
                        };
                        let shortcut = actions::effective_shortcut(&action, c.shortcut);
                        m.item_with_shortcut(&c.title, shortcut.as_ref(), action)
                    })
                    .collect();
                submenu(p.name(), items)
            })
            .collect()
    }

    /// The Edit menu, enabled for tab `index`.
    pub(super) fn edit_menu(&self, m: &mut MenuModel, index: usize) -> Vec<MenuEntry> {
        let editor = self.docs.get(index).map(|d| &d.editor);
        let read_only = editor.is_some_and(|e| e.buffer.read_only);
        let can_undo = editor.is_some_and(|e| e.buffer.can_undo());
        let can_redo = editor.is_some_and(|e| e.buffer.can_redo());
        let has_sel = editor.is_some_and(|e| e.selections.has_selection());
        let mut line_ops = vec![
            m.item(&t!("duplicate"), Action::Duplicate),
            m.item(&t!("delete-lines"), Action::DeleteLines),
            m.item(&t!("move-lines-up"), Action::MoveLinesUp),
            m.item(&t!("move-lines-down"), Action::MoveLinesDown),
            m.item(&t!("join-lines"), Action::JoinLines),
            m.item(&t!("split-lines"), Action::SplitLines),
            m.item(&t!("insert-blank-above"), Action::InsertBlankAbove),
            m.item(&t!("insert-blank-below"), Action::InsertBlankBelow),
            MenuEntry::Separator,
        ];
        for (labels, kind) in [
            (["sort-lexical-asc", "sort-lexical-desc"], SortKind::Lexical),
            (
                ["sort-ignore-case-asc", "sort-ignore-case-desc"],
                SortKind::IgnoreCase,
            ),
            (["sort-integer-asc", "sort-integer-desc"], SortKind::Integer),
            (["sort-decimal-asc", "sort-decimal-desc"], SortKind::Decimal),
            (["sort-length-asc", "sort-length-desc"], SortKind::Length),
        ] {
            for (label, desc) in labels.into_iter().zip([false, true]) {
                line_ops.push(m.item(&crate::i18n::tr(label), Action::SortLines { kind, desc }));
            }
        }
        line_ops.extend([
            m.item(&t!("reverse-lines"), Action::ReverseLines),
            MenuEntry::Separator,
            m.item(&t!("remove-duplicate-lines"), Action::RemoveDuplicateLines),
            m.item(
                &t!("remove-consecutive-duplicates"),
                Action::RemoveConsecutiveDuplicates,
            ),
            m.item(
                &t!("remove-empty-lines"),
                Action::RemoveEmptyLines { blank: false },
            ),
            m.item(
                &t!("remove-blank-lines"),
                Action::RemoveEmptyLines { blank: true },
            ),
        ]);
        vec![
            m.item_if(&t!("undo"), can_undo, Action::Undo),
            m.item_if(&t!("redo"), can_redo, Action::Redo),
            MenuEntry::Separator,
            m.item_if(&t!("cut"), has_sel, Action::Cut),
            m.item_if(&t!("copy"), has_sel, Action::Copy),
            m.item(&t!("paste"), Action::Paste),
            m.item(&t!("select-all"), Action::SelectAll),
            MenuEntry::Separator,
            submenu(&t!("menu-line-operations"), line_ops),
            submenu(
                &t!("menu-convert-case"),
                vec![
                    m.item(&t!("case-upper"), Action::ConvertCase(CaseKind::Upper)),
                    m.item(&t!("case-lower"), Action::ConvertCase(CaseKind::Lower)),
                    m.item(&t!("case-title"), Action::ConvertCase(CaseKind::Title)),
                    m.item(
                        &t!("case-sentence"),
                        Action::ConvertCase(CaseKind::Sentence),
                    ),
                    m.item(&t!("case-invert"), Action::ConvertCase(CaseKind::Invert)),
                ],
            ),
            submenu(
                &t!("menu-comment"),
                vec![
                    m.item(&t!("toggle-line-comment"), Action::ToggleLineComment),
                    m.item(&t!("toggle-block-comment"), Action::ToggleBlockComment),
                ],
            ),
            submenu(
                &t!("menu-blank-operations"),
                vec![
                    m.item(&t!("trim-trailing"), Action::Trim(TrimKind::Trailing)),
                    m.item(&t!("trim-leading"), Action::Trim(TrimKind::Leading)),
                    m.item(&t!("trim-both"), Action::Trim(TrimKind::Both)),
                    m.item(&t!("eol-to-space"), Action::EolToSpace),
                    MenuEntry::Separator,
                    m.item(&t!("tabs-to-spaces"), Action::TabsToSpaces),
                    m.item(
                        &t!("leading-spaces-to-tabs"),
                        Action::SpacesToTabs { all: false },
                    ),
                    m.item(
                        &t!("all-spaces-to-tabs"),
                        Action::SpacesToTabs { all: true },
                    ),
                ],
            ),
            submenu(
                &t!("menu-insert"),
                vec![
                    m.item(
                        &t!("date-time-short"),
                        Action::InsertDateTime { long: false },
                    ),
                    m.item(&t!("date-time-long"), Action::InsertDateTime { long: true }),
                ],
            ),
            submenu(
                &t!("menu-multi-select"),
                vec![
                    m.item(&t!("select-next-occurrence"), Action::SelectNextOccurrence),
                    m.item(&t!("select-all-occurrences"), Action::SelectAllOccurrences),
                ],
            ),
            submenu(
                &t!("menu-auto-completion"),
                vec![
                    m.item(&t!("complete-word"), Action::CompleteWord),
                    m.item(&t!("show-parameter-hint"), Action::ShowParameterHint),
                ],
            ),
            m.item(&t!("column-editor"), Action::ColumnEditor),
            m.toggle(
                &t!("column-mode"),
                self.column_mode,
                Action::ToggleColumnMode,
            ),
            MenuEntry::Separator,
            m.toggle(&t!("read-only"), read_only, Action::ToggleReadOnly),
        ]
    }

    /// The Tools menu.
    pub(super) fn tools_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        use crate::tools::mime::MimeOp::*;
        let mut mime = Vec::new();
        for (i, group) in [
            &[
                ("mime-base64-encode", Base64Encode),
                ("mime-base64-encode-wrapped", Base64EncodeWrapped),
                ("mime-base64-url-encode", Base64UrlEncode),
                ("mime-base64-decode", Base64Decode),
            ][..],
            &[
                ("mime-url-encode-component", UrlEncodeComponent),
                ("mime-url-encode", UrlEncode),
                ("mime-url-decode", UrlDecode),
            ],
            &[("mime-qp-encode", QpEncode), ("mime-qp-decode", QpDecode)],
            &[
                ("mime-html-encode", HtmlEncode),
                ("mime-html-encode-all", HtmlEncodeAll),
                ("mime-html-decode", HtmlDecode),
            ],
            &[("mime-saml-decode", SamlDecode)],
        ]
        .into_iter()
        .enumerate()
        {
            if i > 0 {
                mime.push(MenuEntry::Separator);
            }
            for &(label, op) in group {
                mime.push(m.item(&crate::i18n::tr(label), Action::Mime(op)));
            }
        }
        let json = vec![
            m.item(&t!("json-format"), Action::Json(JsonOp::Format)),
            m.item(&t!("json-minify"), Action::Json(JsonOp::Minify)),
            MenuEntry::Separator,
            m.item(&t!("json-validate"), Action::Json(JsonOp::Validate)),
            m.item(&t!("json-sort-keys"), Action::Json(JsonOp::SortKeys)),
            MenuEntry::Separator,
            m.toggle(
                &t!("json-tree"),
                self.json_tree.open,
                Action::ToggleJsonTree,
            ),
        ];
        let xml = vec![
            m.item(&t!("xml-pretty-print"), Action::Xml(XmlOp::PrettyPrint)),
            m.item(
                &t!("xml-pretty-print-attributes"),
                Action::Xml(XmlOp::PrettyPrintAttributes),
            ),
            m.item(&t!("xml-linearize"), Action::Xml(XmlOp::Linearize)),
            MenuEntry::Separator,
            m.item(&t!("xml-check-syntax"), Action::Xml(XmlOp::CheckSyntax)),
            m.toggle(
                &t!("xml-check-on-save"),
                self.settings.xml_check_on_save,
                Action::ToggleXmlCheckOnSave,
            ),
            m.item(&t!("xml-validate"), Action::Xml(XmlOp::ValidateSchema)),
            m.item(
                &t!("xml-validate-against"),
                Action::Xml(XmlOp::ChooseSchema),
            ),
            m.item(&t!("xml-xsl-transform"), Action::Xml(XmlOp::XslTransform)),
            MenuEntry::Separator,
            m.toggle(
                &t!("xpath-panel"),
                self.xpath.open,
                Action::ToggleXPathPanel,
            ),
            m.item(&t!("copy-current-xpath"), Action::CopyXPath),
            MenuEntry::Separator,
            m.item(&t!("xml-escape"), Action::Xml(XmlOp::Escape)),
            m.item(&t!("xml-unescape"), Action::Xml(XmlOp::Unescape)),
            m.toggle(
                &t!("close-tags-automatically"),
                self.settings.close_tags,
                Action::ToggleCloseTags,
            ),
        ];
        let csv = vec![
            m.toggle(&t!("csvquery-panel"), self.csv.open, Action::ToggleCsvQuery),
            m.item(&t!("run-query"), Action::RunCsvQuery),
            MenuEntry::Separator,
            m.item_if(
                &t!("copy-results"),
                self.csv.result.is_some(),
                Action::CopyCsvResults,
            ),
            m.item_if(
                &t!("results-to-new-tab"),
                self.csv.result.is_some(),
                Action::CsvResultsToNewTab,
            ),
        ];
        vec![
            submenu(&t!("menu-compare"), self.compare_menu(m)),
            submenu(&t!("menu-csvquery"), csv),
            MenuEntry::Separator,
            submenu(&t!("menu-mime-tools"), mime),
            submenu(&t!("menu-json"), json),
            submenu(&t!("menu-xml-tools"), xml),
        ]
    }

    /// Tools > Compare.
    fn compare_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        use crate::compare::CompareOpt;
        let saved = self.docs.get(self.active).is_some_and(|d| d.path.is_some());
        let open = self.compare.is_some();
        let label = if self.is_split() {
            t!("compare-two-views")
        } else {
            t!("compare-previous-tab")
        };
        let others: Vec<MenuEntry> = (0..self.docs.len())
            .filter(|&i| i != self.active)
            .map(|i| m.item(&self.docs[i].title(), Action::CompareWith(i)))
            .collect();
        let opts = self.settings.compare;
        let mut entries = vec![m.item(&label, Action::CompareTabs)];
        if others.is_empty() {
            entries.push(m.item_if(&t!("compare-with"), false, Action::CompareTabs));
        } else {
            entries.push(submenu(&t!("compare-with"), others));
        }
        entries.extend([
            m.item_if(&t!("compare-to-saved"), saved, Action::CompareToSaved),
            m.item(&t!("compare-to-clipboard"), Action::CompareToClipboard),
            m.item_if(&t!("compare-to-git"), saved, Action::CompareToGitHead),
            m.item(&t!("compare-selections"), Action::CompareSelections),
            MenuEntry::Separator,
            m.item_if(&t!("next-difference"), open, Action::NextDifference),
            m.item_if(&t!("previous-difference"), open, Action::PrevDifference),
            m.item_if(&t!("show-comparison"), open, Action::ShowComparison),
            m.item_if(&t!("export-as-patch"), open, Action::ExportComparePatch),
            m.item_if(&t!("close-comparison"), open, Action::CloseComparison),
            MenuEntry::Separator,
        ]);
        for (label, opt) in [
            ("compare-ignore-whitespace", CompareOpt::IgnoreWhitespace),
            ("compare-ignore-case", CompareOpt::IgnoreCase),
            ("compare-ignore-empty-lines", CompareOpt::IgnoreEmptyLines),
            ("compare-detect-moves", CompareOpt::DetectMoves),
        ] {
            entries.push(m.toggle(
                &crate::i18n::tr(label),
                opts.get(opt),
                Action::ToggleCompareOption(opt),
            ));
        }
        entries
    }
}
