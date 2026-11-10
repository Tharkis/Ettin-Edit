//! The menus that are drawn in more than one place: Edit, Transform and
//! Tools are in the menu bar, and the editor's right-click menu offers parts
//! of them that fit what was clicked.

use egui::Ui;

use super::EttinApp;
use crate::actions::{self, Action};
use crate::editor::lines::SortKind;
use crate::menu::{self, MenuEntry};
use crate::tools::convert::DataFormat;
use crate::tools::json::JsonOp;
use crate::tools::xml::XmlOp;
use crate::transform_ui::DialogKind;
use crate::transforms::{self, Entry, GroupId};

/// A menu shown in the menu bar and in the right-click menu.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SharedMenu {
    Edit,
    Transform,
    Tools,
}

/// Collects the action behind each menu item; an item's id is its index.
pub(super) struct MenuModel<'c> {
    ctx: &'c egui::Context,
    shortcuts: Vec<(egui::KeyboardShortcut, Action)>,
    actions: Vec<Action>,
    /// The name an item has in the command palette when its menu label
    /// alone wouldn't say what it does ("YAML" is "Convert to YAML").
    names: Vec<Option<String>>,
}

impl<'c> MenuModel<'c> {
    pub fn new(ctx: &'c egui::Context) -> Self {
        Self {
            ctx,
            shortcuts: actions::all_shortcuts(),
            actions: Vec::new(),
            names: Vec::new(),
        }
    }

    pub fn ctx(&self) -> &egui::Context {
        self.ctx
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
            hint: None,
        };
        self.actions.push(action);
        self.names.push(None);
        entry
    }

    /// An item with a tooltip, shown while it is greyed out too.
    fn item_hint(
        &mut self,
        label: &str,
        enabled: bool,
        hint: Option<String>,
        action: Action,
    ) -> MenuEntry {
        let mut entry = self.entry(label, enabled, None, action);
        if let MenuEntry::Item { hint: h, .. } = &mut entry {
            *h = hint;
        }
        entry
    }

    /// An item with its own name in the command palette.
    pub fn item_named(
        &mut self,
        label: &str,
        name: &str,
        enabled: bool,
        action: Action,
    ) -> MenuEntry {
        let entry = self.entry(label, enabled, None, action);
        if let Some(last) = self.names.last_mut() {
            *last = Some(name.to_owned());
        }
        entry
    }

    /// A tickable item with its own name in the command palette.
    pub fn toggle_named(
        &mut self,
        label: &str,
        name: &str,
        checked: bool,
        action: Action,
    ) -> MenuEntry {
        let entry = self.entry(label, true, Some(checked), action);
        if let Some(last) = self.names.last_mut() {
            *last = Some(name.to_owned());
        }
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
            hint: None,
        };
        self.actions.push(action);
        self.names.push(None);
        entry
    }

    fn toggle(&mut self, label: &str, checked: bool, action: Action) -> MenuEntry {
        self.entry(label, true, Some(checked), action)
    }

    /// The action of a clicked item id.
    pub fn action(&self, id: usize) -> Option<Action> {
        self.actions.get(id).cloned()
    }

    /// The palette name of a clicked item id, when it has its own.
    pub fn name(&self, id: usize) -> Option<&str> {
        self.names.get(id).and_then(|n| n.as_deref())
    }
}

fn submenu(label: &str, entries: Vec<MenuEntry>) -> MenuEntry {
    MenuEntry::Submenu {
        label: label.to_owned(),
        entries,
    }
}

/// Catalogue entries as menu entries.
fn catalogue_entries(m: &mut MenuModel, entries: &[Entry]) -> Vec<MenuEntry> {
    entries
        .iter()
        .map(|e| match e {
            Entry::Item { label, action, .. } => m.item(label, action.clone()),
            Entry::Heading(text) => MenuEntry::Heading(text.clone()),
            Entry::Separator => MenuEntry::Separator,
            Entry::Submenu { label, entries } => submenu(label, catalogue_entries(m, entries)),
        })
        .collect()
}

/// A catalogue group's entries.
pub(super) fn group_entries(m: &mut MenuModel, id: GroupId) -> Vec<MenuEntry> {
    catalogue_entries(m, &transforms::group(id).entries)
}

/// What the Tools menu shows ticked or greyed out.
pub(super) struct ToolsState {
    pub json_tree: bool,
    pub xpath: bool,
    pub csv_open: bool,
    pub csv_result: bool,
    pub xml_check_on_save: bool,
    pub close_tags: bool,
}

/// The Tools menu: tools that look at the text without changing it.
/// Everything that changes the text is under Transform.
pub(super) fn tools_entries(
    m: &mut MenuModel,
    state: &ToolsState,
    compare: Vec<MenuEntry>,
) -> Vec<MenuEntry> {
    let csv = vec![
        m.toggle(
            &t!("csvquery-panel"),
            state.csv_open,
            Action::ToggleCsvQuery,
        ),
        m.item(&t!("run-query"), Action::RunCsvQuery),
        MenuEntry::Separator,
        m.item_if(
            &t!("copy-results"),
            state.csv_result,
            Action::CopyCsvResults,
        ),
        m.item_if(
            &t!("results-to-new-tab"),
            state.csv_result,
            Action::CsvResultsToNewTab,
        ),
    ];
    vec![
        submenu(&t!("menu-compare"), compare),
        submenu(&t!("menu-csvquery"), csv),
        MenuEntry::Separator,
        m.toggle(&t!("json-tree"), state.json_tree, Action::ToggleJsonTree),
        m.item(&t!("json-validate"), Action::Json(JsonOp::Validate)),
        MenuEntry::Separator,
        m.toggle(&t!("xpath-panel"), state.xpath, Action::ToggleXPathPanel),
        m.item(&t!("copy-current-xpath"), Action::CopyXPath),
        m.item_named(
            &t!("xml-check-syntax-named"),
            &t!("xml-check-syntax-named"),
            true,
            Action::Xml(XmlOp::CheckSyntax),
        ),
        m.item_named(
            &t!("xml-validate-named"),
            &t!("xml-validate-named"),
            true,
            Action::Xml(XmlOp::ValidateSchema),
        ),
        m.item_named(
            &t!("xml-validate-against-named"),
            &t!("xml-validate-against-named"),
            true,
            Action::Xml(XmlOp::ChooseSchema),
        ),
        m.item(&t!("xml-xsl-transform"), Action::Xml(XmlOp::XslTransform)),
        MenuEntry::Separator,
        m.toggle(
            &t!("xml-check-on-save"),
            state.xml_check_on_save,
            Action::ToggleXmlCheckOnSave,
        ),
        m.toggle(
            &t!("close-tags-automatically"),
            state.close_tags,
            Action::ToggleCloseTags,
        ),
    ]
}

impl EttinApp {
    /// Draws the Edit, Transform or Tools menu into an open menu-bar menu.
    pub(super) fn shared_menu(&mut self, ui: &mut Ui, which: SharedMenu) {
        let ctx = ui.ctx().clone();
        let mut m = MenuModel::new(&ctx);
        let entries = match which {
            SharedMenu::Edit => self.edit_menu(&mut m, self.active),
            SharedMenu::Transform => self.transform_menu(&mut m),
            SharedMenu::Tools => self.tools_menu(&mut m),
        };
        if let Some(action) = menu::show(ui, &entries).and_then(|id| m.action(id)) {
            self.pending.push(action);
        }
    }

    /// The right-click menu of tab `index`'s editor: Cut, Copy and Paste,
    /// then the commands that fit what was clicked (the selection, or the
    /// kind of file), then the whole Transform menu, Recipes and the rest
    /// of the Edit menu.
    pub(super) fn context_menu(&self, m: &mut MenuModel, index: usize) -> Vec<MenuEntry> {
        let doc = self.docs.get(index);
        let has_sel = doc.is_some_and(|d| d.editor.selections.has_selection());
        let mut entries = vec![
            m.item_if(&t!("cut"), has_sel, Action::Cut),
            m.item_if(&t!("copy"), has_sel, Action::Copy),
            m.item(&t!("paste"), Action::Paste),
        ];
        let section = self.context_section(m, index, has_sel);
        if !section.is_empty() {
            entries.push(MenuEntry::Separator);
            entries.extend(section);
        }
        // Plugin transforms made for this tab's language.
        let language = doc.map(|d| d.syntax.as_str()).unwrap_or_default();
        let suited: Vec<MenuEntry> = self
            .plugins
            .plugins
            .iter()
            .filter(|p| p.is_running())
            .flat_map(|p| {
                p.transforms
                    .iter()
                    .filter(|t| t.suits(language))
                    .map(|t| (p.id.clone(), t.id.clone(), t.title.clone()))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(plugin, transform, title)| {
                m.item(&title, Action::PluginTransform { plugin, transform })
            })
            .collect();
        if !suited.is_empty() {
            entries.push(MenuEntry::Separator);
            entries.push(MenuEntry::Heading(t!("menu-plugin-transforms")));
            entries.extend(suited);
        }
        entries.push(MenuEntry::Separator);
        entries.push(submenu(&t!("menu-transform"), self.transform_menu(m)));
        entries.push(submenu(&t!("menu-recipes"), self.recipes_menu(m)));
        entries.push(submenu(
            &t!("more-edit-commands"),
            self.more_edit_menu(m, index),
        ));
        let plugins = self.plugin_commands_menu(m);
        if !plugins.is_empty() {
            entries.push(submenu(&t!("menu-plugins"), plugins));
        }
        entries
    }

    /// The part of the right-click menu that fits what was clicked: a
    /// selection, or a JSON, XML or CSV file.
    fn context_section(&self, m: &mut MenuModel, index: usize, has_sel: bool) -> Vec<MenuEntry> {
        if has_sel {
            return vec![
                MenuEntry::Heading(t!("context-selection")),
                submenu(&t!("menu-convert-case"), group_entries(m, GroupId::Case)),
                submenu(&t!("menu-encode-decode"), group_entries(m, GroupId::Encode)),
                m.item(
                    &t!("sort-lines"),
                    Action::SortLines {
                        kind: SortKind::Lexical,
                        desc: false,
                    },
                ),
                m.item(&t!("remove-duplicate-lines"), Action::RemoveDuplicateLines),
                m.item(&t!("pipe-dots"), Action::TransformDialog(DialogKind::Pipe)),
            ];
        }
        let Some(doc) = self.docs.get(index) else {
            return Vec::new();
        };
        let Some(format) = DataFormat::from_language(&doc.syntax) else {
            return Vec::new();
        };
        let mut out = vec![MenuEntry::Heading(format.name().to_owned())];
        match format {
            DataFormat::Json => out.extend([
                m.item(&t!("json-format-named"), Action::Json(JsonOp::Format)),
                m.item(&t!("json-minify-named"), Action::Json(JsonOp::Minify)),
                m.item(&t!("json-sort-keys"), Action::Json(JsonOp::SortKeys)),
            ]),
            DataFormat::Xml => {
                out.push(m.item(&t!("xml-pretty-print"), Action::Xml(XmlOp::PrettyPrint)))
            }
            DataFormat::Csv => out.push(m.item(&t!("open-in-csvquery"), Action::ToggleCsvQuery)),
            _ => {}
        }
        out.push(submenu(&t!("menu-convert-to"), self.convert_menu(m)));
        match format {
            DataFormat::Json => out.push(m.toggle(
                &t!("json-tree"),
                self.json_tree.open,
                Action::ToggleJsonTree,
            )),
            DataFormat::Xml => out.push(m.toggle(
                &t!("xpath-panel"),
                self.xpath.open,
                Action::ToggleXPathPanel,
            )),
            _ => {}
        }
        out
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

    /// The Edit menu without Cut, Copy and Paste, which the right-click
    /// menu shows first.
    fn more_edit_menu(&self, m: &mut MenuModel, index: usize) -> Vec<MenuEntry> {
        let mut entries = self.edit_menu(m, index);
        entries.retain(|e| match e {
            MenuEntry::Item { id, .. } => !matches!(
                m.action(*id),
                Some(Action::Cut | Action::Copy | Action::Paste)
            ),
            _ => true,
        });
        // The separator Cut and Copy stood after.
        if let (Some(MenuEntry::Separator), Some(MenuEntry::Separator)) =
            (entries.get(2), entries.get(3))
        {
            entries.remove(2);
        }
        entries
    }

    /// The Edit menu, enabled for tab `index`. Line Operations repeats
    /// Transform > Lines for people used to finding them here.
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
        line_ops.extend(group_entries(m, GroupId::Lines));
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
                &t!("menu-comment"),
                vec![
                    m.item(&t!("toggle-line-comment"), Action::ToggleLineComment),
                    m.item(&t!("toggle-block-comment"), Action::ToggleBlockComment),
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

    /// The Transform menu: everything that turns text into other text.
    /// Filter Through Command, Recipes and Macros at the top, then the
    /// catalogue's groups (see [`crate::transforms`]).
    pub(super) fn transform_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        let mut out = group_entries(m, GroupId::Command);
        out.push(submenu(&t!("menu-recipes"), self.recipes_menu(m)));
        out.push(submenu(&t!("menu-macros"), self.macros_menu(m)));
        out.push(MenuEntry::Separator);
        for group in transforms::catalogue() {
            let entries = match group.id {
                GroupId::Command => continue,
                GroupId::ConvertTo => self.convert_menu(m),
                _ => catalogue_entries(m, &group.entries),
            };
            if entries.is_empty() {
                continue;
            }
            if group.id == GroupId::Plugins {
                out.push(MenuEntry::Separator);
            }
            out.push(submenu(&group.id.label(), entries));
        }
        out
    }

    /// Transform > Recipes.
    pub(super) fn recipes_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        let recording = self.recipe_recording.is_some();
        let last = self.last_recipe();
        let mut recipes = vec![
            MenuEntry::Note(t!("recipes-menu-note")),
            m.item(&t!("manage-recipes"), Action::Recipes),
            m.item(
                &crate::i18n::tr(if recording {
                    "stop-recording-recipe"
                } else {
                    "record-recipe"
                }),
                Action::ToggleRecipeRecording,
            ),
            m.item_hint(
                &t!("run-last-recipe"),
                last.is_some(),
                last.map(|r| r.name.clone()),
                Action::RunLastRecipe,
            ),
            m.item(&t!("open-recipes-folder"), Action::OpenRecipesFolder),
        ];
        if !self.recipes.is_empty() {
            recipes.push(MenuEntry::Separator);
            for r in &self.recipes {
                recipes.push(m.item(&r.name, Action::RunRecipe(r.id.clone())));
            }
            let all_tabs = self
                .recipes
                .iter()
                .map(|r| m.item(&r.name, Action::RunRecipeOnAllTabs(r.id.clone())))
                .collect();
            recipes.push(MenuEntry::Separator);
            recipes.push(submenu(&t!("run-recipe-on-all-tabs"), all_tabs));
        }
        recipes
    }

    /// Transform > Macros.
    fn macros_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        let recording = self.recorder.is_recording();
        let has_macro = !self.recorder.current.is_empty();
        let label = if recording {
            t!("stop-recording")
        } else {
            t!("start-recording")
        };
        let ready = !recording && has_macro;
        let as_recipe = ready && self.macro_recipe_block.is_none();
        let hint = if as_recipe {
            Some(t!("save-macro-as-recipe-hint"))
        } else if ready {
            self.macro_recipe_block.clone()
        } else {
            None
        };
        let mut out = vec![
            MenuEntry::Note(t!("macros-menu-note")),
            m.item(&label, Action::ToggleMacroRecording),
            m.item_if(&t!("playback"), ready, Action::PlayMacro),
            m.item_if(&t!("run-macro-multiple"), ready, Action::RunMacroMultiple),
            m.item_if(&t!("save-current-macro"), ready, Action::SaveMacro),
            m.item_hint(
                &t!("save-macro-as-recipe"),
                as_recipe,
                hint,
                Action::SaveMacroAsRecipe,
            ),
        ];
        if !self.settings.macros.is_empty() {
            out.push(MenuEntry::Separator);
            for (i, mac) in self.settings.macros.iter().enumerate() {
                out.push(m.item_if(&mac.name, !recording, Action::PlaySavedMacro(i)));
            }
            let delete = self
                .settings
                .macros
                .iter()
                .enumerate()
                .map(|(i, mac)| m.item(&mac.name, Action::DeleteSavedMacro(i)))
                .collect();
            out.push(submenu(&t!("delete-saved-macro"), delete));
        }
        out
    }

    /// The Tools menu.
    pub(super) fn tools_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        let compare = self.compare_menu(m);
        let state = ToolsState {
            json_tree: self.json_tree.open,
            xpath: self.xpath.open,
            csv_open: self.csv.open,
            csv_result: self.csv.result.is_some(),
            xml_check_on_save: self.settings.xml_check_on_save,
            close_tags: self.settings.close_tags,
        };
        tools_entries(m, &state, compare)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn actions_of(entries: &[MenuEntry], m: &MenuModel, out: &mut Vec<Action>) {
        for e in entries {
            match e {
                MenuEntry::Item { id, .. } => out.extend(m.action(*id)),
                MenuEntry::Submenu { entries, .. } => actions_of(entries, m, out),
                _ => {}
            }
        }
    }

    /// Tools only looks at the text: nothing in it is a transform.
    #[test]
    fn tools_has_no_transforms() {
        let ctx = egui::Context::default();
        let mut m = MenuModel::new(&ctx);
        let state = ToolsState {
            json_tree: false,
            xpath: false,
            csv_open: false,
            csv_result: false,
            xml_check_on_save: false,
            close_tags: false,
        };
        let entries = tools_entries(&mut m, &state, Vec::new());
        let mut actions = Vec::new();
        actions_of(&entries, &m, &mut actions);
        assert!(actions.contains(&Action::Xml(XmlOp::XslTransform)));
        for a in &actions {
            assert!(!transforms::is_transform(a), "{a:?} is in Tools");
        }
    }

    /// The Transform menu's groups and the recipe window's Add Step groups
    /// are both the catalogue's.
    #[test]
    fn menu_and_add_step_share_groups() {
        let ctx = egui::Context::default();
        let mut m = MenuModel::new(&ctx);
        for group in transforms::catalogue() {
            let entries = catalogue_entries(&mut m, &group.entries);
            let mut actions = Vec::new();
            actions_of(&entries, &m, &mut actions);
            assert_eq!(actions, group.actions());
        }
        let menu_groups: Vec<String> = transforms::catalogue()
            .iter()
            .filter(|g| !g.steps().is_empty())
            .map(|g| g.id.label())
            .collect();
        assert_eq!(crate::recipe_ui::add_step_groups(), menu_groups);
    }
}
