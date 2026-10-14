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
        entries.push(submenu("Tools", self.tools_menu(m)));
        let plugins = self.plugin_commands_menu(m);
        if !plugins.is_empty() {
            entries.push(submenu("Plugins", plugins));
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
                        m.item_with_shortcut(
                            &c.title,
                            c.shortcut.as_ref(),
                            Action::PluginCommand {
                                plugin: p.id.clone(),
                                command: c.id.clone(),
                            },
                        )
                    })
                    .collect();
                submenu(p.name(), items)
            })
            .collect()
    }

    /// The Edit menu, enabled for tab `index`.
    fn edit_menu(&self, m: &mut MenuModel, index: usize) -> Vec<MenuEntry> {
        let editor = self.docs.get(index).map(|d| &d.editor);
        let read_only = editor.is_some_and(|e| e.buffer.read_only);
        let can_undo = editor.is_some_and(|e| e.buffer.can_undo());
        let can_redo = editor.is_some_and(|e| e.buffer.can_redo());
        let has_sel = editor.is_some_and(|e| e.selections.has_selection());
        let mut line_ops = vec![
            m.item("Duplicate", Action::Duplicate),
            m.item("Delete Lines", Action::DeleteLines),
            m.item("Move Up", Action::MoveLinesUp),
            m.item("Move Down", Action::MoveLinesDown),
            m.item("Join Lines", Action::JoinLines),
            m.item("Split Lines at Long-Line Column", Action::SplitLines),
            m.item("Insert Blank Line Above", Action::InsertBlankAbove),
            m.item("Insert Blank Line Below", Action::InsertBlankBelow),
            MenuEntry::Separator,
        ];
        for (label, kind) in [
            ("Lexicographically", SortKind::Lexical),
            ("Ignoring Case", SortKind::IgnoreCase),
            ("As Integers", SortKind::Integer),
            ("As Decimals", SortKind::Decimal),
            ("By Length", SortKind::Length),
        ] {
            for (dir, desc) in [("Ascending", false), ("Descending", true)] {
                line_ops.push(m.item(
                    &format!("Sort {label} {dir}"),
                    Action::SortLines { kind, desc },
                ));
            }
        }
        line_ops.extend([
            m.item("Reverse Line Order", Action::ReverseLines),
            MenuEntry::Separator,
            m.item("Remove Duplicate Lines", Action::RemoveDuplicateLines),
            m.item(
                "Remove Consecutive Duplicate Lines",
                Action::RemoveConsecutiveDuplicates,
            ),
            m.item(
                "Remove Empty Lines",
                Action::RemoveEmptyLines { blank: false },
            ),
            m.item(
                "Remove Empty Lines (Including Blank Ones)",
                Action::RemoveEmptyLines { blank: true },
            ),
        ]);
        vec![
            m.item_if("Undo", can_undo, Action::Undo),
            m.item_if("Redo", can_redo, Action::Redo),
            MenuEntry::Separator,
            m.item_if("Cut", has_sel, Action::Cut),
            m.item_if("Copy", has_sel, Action::Copy),
            m.item("Paste", Action::Paste),
            m.item("Select All", Action::SelectAll),
            MenuEntry::Separator,
            submenu("Line Operations", line_ops),
            submenu(
                "Convert Case To",
                vec![
                    m.item("UPPERCASE", Action::ConvertCase(CaseKind::Upper)),
                    m.item("lowercase", Action::ConvertCase(CaseKind::Lower)),
                    m.item("Title Case", Action::ConvertCase(CaseKind::Title)),
                    m.item("Sentence case", Action::ConvertCase(CaseKind::Sentence)),
                    m.item("iNVERT cASE", Action::ConvertCase(CaseKind::Invert)),
                ],
            ),
            submenu(
                "Comment",
                vec![
                    m.item("Toggle Line Comment", Action::ToggleLineComment),
                    m.item("Toggle Block Comment", Action::ToggleBlockComment),
                ],
            ),
            submenu(
                "Blank Operations",
                vec![
                    m.item("Trim Trailing Space", Action::Trim(TrimKind::Trailing)),
                    m.item("Trim Leading Space", Action::Trim(TrimKind::Leading)),
                    m.item(
                        "Trim Leading and Trailing Space",
                        Action::Trim(TrimKind::Both),
                    ),
                    m.item("EOL to Space", Action::EolToSpace),
                    MenuEntry::Separator,
                    m.item("Tabs to Spaces", Action::TabsToSpaces),
                    m.item(
                        "Leading Spaces to Tabs",
                        Action::SpacesToTabs { all: false },
                    ),
                    m.item("All Spaces to Tabs", Action::SpacesToTabs { all: true }),
                ],
            ),
            submenu(
                "Insert",
                vec![
                    m.item(
                        "Date and Time (Short)",
                        Action::InsertDateTime { long: false },
                    ),
                    m.item(
                        "Date and Time (Long)",
                        Action::InsertDateTime { long: true },
                    ),
                ],
            ),
            submenu(
                "Multi-Select",
                vec![
                    m.item("Select Next Occurrence", Action::SelectNextOccurrence),
                    m.item("Select All Occurrences", Action::SelectAllOccurrences),
                ],
            ),
            m.item("Column Editor…", Action::ColumnEditor),
            MenuEntry::Separator,
            m.toggle("Read-Only", read_only, Action::ToggleReadOnly),
        ]
    }

    /// The Tools menu.
    fn tools_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        use crate::tools::mime::MimeOp::*;
        let mut mime = Vec::new();
        for (i, group) in [
            &[
                ("Base64 Encode", Base64Encode),
                ("Base64 Encode with Line Breaks", Base64EncodeWrapped),
                ("Base64 Encode, URL-Safe", Base64UrlEncode),
                ("Base64 Decode", Base64Decode),
            ][..],
            &[
                ("URL Encode (Part of a URL)", UrlEncodeComponent),
                ("URL Encode (Whole URL)", UrlEncode),
                ("URL Decode", UrlDecode),
            ],
            &[
                ("Quoted-Printable Encode", QpEncode),
                ("Quoted-Printable Decode", QpDecode),
            ],
            &[
                ("HTML Encode", HtmlEncode),
                ("HTML Encode, Including Accents and Symbols", HtmlEncodeAll),
                ("HTML Decode", HtmlDecode),
            ],
            &[("SAML Decode", SamlDecode)],
        ]
        .into_iter()
        .enumerate()
        {
            if i > 0 {
                mime.push(MenuEntry::Separator);
            }
            for &(label, op) in group {
                mime.push(m.item(label, Action::Mime(op)));
            }
        }
        let json = vec![
            m.item("Format", Action::Json(JsonOp::Format)),
            m.item("Minify", Action::Json(JsonOp::Minify)),
            MenuEntry::Separator,
            m.item("Validate JSON", Action::Json(JsonOp::Validate)),
            m.item("Sort JSON Keys", Action::Json(JsonOp::SortKeys)),
            MenuEntry::Separator,
            m.toggle("JSON Tree", self.json_tree.open, Action::ToggleJsonTree),
        ];
        let xml = vec![
            m.item("Pretty Print", Action::Xml(XmlOp::PrettyPrint)),
            m.item(
                "Pretty Print, Attributes on New Lines",
                Action::Xml(XmlOp::PrettyPrintAttributes),
            ),
            m.item("Linearize", Action::Xml(XmlOp::Linearize)),
            MenuEntry::Separator,
            m.item("Check Syntax", Action::Xml(XmlOp::CheckSyntax)),
            m.toggle(
                "Check Syntax When Saving",
                self.settings.xml_check_on_save,
                Action::ToggleXmlCheckOnSave,
            ),
            MenuEntry::Separator,
            m.toggle("XPath Panel", self.xpath.open, Action::ToggleXPathPanel),
            m.item("Copy Current XPath", Action::CopyXPath),
            MenuEntry::Separator,
            m.item("Escape Special Characters", Action::Xml(XmlOp::Escape)),
            m.item("Unescape Special Characters", Action::Xml(XmlOp::Unescape)),
            m.toggle(
                "Close Tags Automatically",
                self.settings.close_tags,
                Action::ToggleCloseTags,
            ),
        ];
        let csv = vec![
            m.toggle("CsvQuery Panel", self.csv.open, Action::ToggleCsvQuery),
            m.item("Run Query", Action::RunCsvQuery),
            MenuEntry::Separator,
            m.item_if(
                "Copy Results",
                self.csv.result.is_some(),
                Action::CopyCsvResults,
            ),
            m.item_if(
                "Results to New Tab",
                self.csv.result.is_some(),
                Action::CsvResultsToNewTab,
            ),
        ];
        vec![
            submenu("Compare", self.compare_menu(m)),
            submenu("CsvQuery", csv),
            MenuEntry::Separator,
            submenu("MIME Tools", mime),
            submenu("JSON and JavaScript", json),
            submenu("XML Tools", xml),
        ]
    }

    /// Tools > Compare.
    fn compare_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        use crate::compare::CompareOpt;
        let saved = self.docs.get(self.active).is_some_and(|d| d.path.is_some());
        let open = self.compare.is_some();
        let label = if self.is_split() {
            "Compare the Two Views"
        } else {
            "Compare with Previous Tab"
        };
        let others: Vec<MenuEntry> = (0..self.docs.len())
            .filter(|&i| i != self.active)
            .map(|i| m.item(&self.docs[i].title(), Action::CompareWith(i)))
            .collect();
        let opts = self.settings.compare;
        let mut entries = vec![m.item(label, Action::CompareTabs)];
        if others.is_empty() {
            entries.push(m.item_if("Compare With", false, Action::CompareTabs));
        } else {
            entries.push(submenu("Compare With", others));
        }
        entries.extend([
            m.item_if("Compare to Last Save", saved, Action::CompareToSaved),
            m.item("Compare to Clipboard", Action::CompareToClipboard),
            m.item_if(
                "Compare to Last Commit (Git)",
                saved,
                Action::CompareToGitHead,
            ),
            m.item("Compare Selections", Action::CompareSelections),
            MenuEntry::Separator,
            m.item_if("Next Difference", open, Action::NextDifference),
            m.item_if("Previous Difference", open, Action::PrevDifference),
            m.item_if("Show Comparison", open, Action::ShowComparison),
            m.item_if("Export as Patch", open, Action::ExportComparePatch),
            m.item_if("Close Comparison", open, Action::CloseComparison),
            MenuEntry::Separator,
        ]);
        for (label, opt) in [
            ("Ignore Spaces and Tabs", CompareOpt::IgnoreWhitespace),
            ("Ignore Case", CompareOpt::IgnoreCase),
            ("Ignore Empty Lines", CompareOpt::IgnoreEmptyLines),
            ("Detect Moved Lines", CompareOpt::DetectMoves),
        ] {
            entries.push(m.toggle(label, opts.get(opt), Action::ToggleCompareOption(opt)));
        }
        entries
    }
}
