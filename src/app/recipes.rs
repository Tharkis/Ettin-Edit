//! Running, recording and keeping recipes (Transform > Recipes).

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};

use super::EttinApp;
use crate::actions::Action;
use crate::recipe_ui::{CommandConfirm, ConfirmAnswer, PREVIEW_CHARS, PreviewInput, RecipeEvent};
use crate::recipes::{self, Env, Failure, Recipe, Step};
use crate::search::SearchOptions;
use crate::tools::json::JsonOp;
use crate::tools::transform::TransformOp;
use crate::tools::xml::XmlOp;

/// Seconds a recipe may take before its progress and Stop button show.
const SHOW_RUNNING_AFTER: f64 = 0.3;

/// One tab a running recipe works on.
struct JobDoc {
    id: u64,
    version: u64,
    ranges: Vec<Range<usize>>,
    /// Select the new text afterwards (the recipe ran on selections).
    select: bool,
}

/// A recipe running on a thread.
pub(super) struct RecipeJob {
    name: String,
    docs: Vec<JobDoc>,
    cancel: Arc<AtomicBool>,
    /// The new text of each range of each tab, or the step that failed.
    output: Receiver<Vec<Result<Vec<String>, Failure>>>,
    started: f64,
    all_tabs: bool,
}

/// The recipe step a command adds while a recipe is being recorded.
/// Replace All takes the find bar's text and options.
pub(super) fn step_for(
    action: &Action,
    find: &str,
    replace: &str,
    opts: &SearchOptions,
) -> Option<Step> {
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

/// The command key `key` should become now that recipes have ids: a
/// shortcut or toolbar button saved before 2.0.2 names its recipe, and gets
/// that recipe's id instead. `None` when it needs no change.
pub(super) fn migrated_recipe_key(key: &str, recipes: &[Recipe]) -> Option<String> {
    let action: Action = serde_json::from_str(key).ok()?;
    let (name, all_tabs) = match action {
        Action::RunRecipe(n) => (n, false),
        Action::RunRecipeOnAllTabs(n) => (n, true),
        _ => return None,
    };
    if recipes.iter().any(|r| r.id == name) {
        return None;
    }
    let id = recipes::find(recipes, &name)?.id.clone();
    let action = if all_tabs {
        Action::RunRecipeOnAllTabs(id)
    } else {
        Action::RunRecipe(id)
    };
    Some(crate::actions::command_key(&action))
}

impl EttinApp {
    /// Points shortcuts and toolbar buttons that name a recipe at its id.
    pub(super) fn migrate_recipe_refs(&mut self) {
        let keys: Vec<String> = self.settings.shortcuts.keys().cloned().collect();
        let mut shortcuts = false;
        for key in keys {
            if let Some(new) = migrated_recipe_key(&key, &self.recipes)
                && let Some(list) = self.settings.shortcuts.remove(&key)
            {
                self.settings.shortcuts.entry(new).or_insert(list);
                shortcuts = true;
            }
        }
        if shortcuts {
            let errors = crate::actions::set_overrides(&self.settings.shortcuts);
            self.errors.extend(errors);
        }
        let mut toolbar = false;
        for item in self
            .toolbar
            .layout
            .tabs
            .iter_mut()
            .flat_map(|t| t.groups.iter_mut())
            .flat_map(|g| g.items.iter_mut())
        {
            if let Some(new) = migrated_recipe_key(&item.command, &self.recipes) {
                item.command = new;
                toolbar = true;
            }
        }
        if toolbar {
            self.toolbar.refresh();
            if let Err(e) = self.toolbar.layout.save() {
                self.errors.push(e);
            }
        }
    }

    /// Reads the recipes folder again, keeping the selected recipe selected.
    pub(super) fn reload_recipes(&mut self) {
        let selected = self
            .recipe_window
            .selected
            .and_then(|i| self.recipes.get(i))
            .and_then(|r| r.path.clone());
        let Some(dir) = recipes::recipes_dir() else {
            return;
        };
        let (list, errors) = recipes::load_all(&dir);
        self.errors.extend(errors);
        self.recipes = list;
        self.migrate_recipe_refs();
        self.recipe_window.selected = selected.and_then(|p| {
            self.recipes
                .iter()
                .position(|r| r.path.as_ref() == Some(&p))
        });
    }

    /// Saves the recipe at `index`, reporting a failure in the window.
    fn save_recipe(&mut self, index: usize) {
        let Some(dir) = recipes::recipes_dir() else {
            return;
        };
        if let Some(r) = self.recipes.get_mut(index) {
            self.recipe_window.message = recipes::save(r, &dir).err();
        }
    }

    /// A name no recipe has yet, starting from `base`.
    fn unused_recipe_name(&self, base: &str) -> String {
        let taken = |n: &str| self.recipes.iter().any(|r| r.name.eq_ignore_ascii_case(n));
        if !taken(base) {
            return base.to_owned();
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|n| !taken(n))
            .expect("some number is free")
    }

    /// Adds a recipe, saves it and shows it in the window with its name
    /// ready to change.
    fn add_recipe(&mut self, mut recipe: Recipe) {
        recipe.path = None;
        recipe.name = self.unused_recipe_name(recipe.name.trim());
        // A copy (Duplicate, or importing a recipe that is already here)
        // gets its own id, so shortcuts stay with the original.
        if recipe.id.trim().is_empty() || self.recipes.iter().any(|r| r.id == recipe.id) {
            recipe.id = recipes::new_id(&recipe.name);
        }
        self.recipes.push(recipe);
        let index = self.recipes.len() - 1;
        self.save_recipe(index);
        self.recipe_window.show_window(Some(index));
        self.recipe_window.focus_name = true;
    }

    pub(super) fn toggle_recipe_recording(&mut self, ctx: &egui::Context) {
        match self.recipe_recording.take() {
            None => self.recipe_recording = Some(Vec::new()),
            Some(steps) if steps.is_empty() => {
                self.notify(ctx, t!("recipe-nothing-recorded"), None);
            }
            Some(steps) => {
                let mut recipe = Recipe::new(&t!("recipe-recorded-name"));
                recipe.steps = steps.into_iter().map(Into::into).collect();
                self.add_recipe(recipe);
            }
        }
    }

    /// Adds the step for `action` to the recipe being recorded.
    pub(super) fn record_recipe_step(&mut self, action: &Action) {
        if self.recipe_recording.is_none() {
            return;
        }
        let mut step = step_for(
            action,
            &self.find_bar.find,
            &self.find_bar.replace,
            &self.settings.search,
        );
        // Converting a tab whose format was picked with Read as keeps that
        // choice in the recipe.
        if let Some(Step::ConvertTo {
            from: from @ None, ..
        }) = &mut step
            && let Some(doc) = self.docs.get(self.active)
        {
            *from = self.read_as.get(&doc.id).copied();
        }
        if let (Some(steps), Some(step)) = (&mut self.recipe_recording, step) {
            steps.push(step);
        }
    }

    /// Runs the recipe called `name` on the selections (or the whole text) of
    /// the active tab, or on every open tab. Recipes with shell commands ask
    /// first.
    pub(super) fn run_recipe(&mut self, ctx: &egui::Context, name: &str, all_tabs: bool) {
        if self.recipes.is_empty() {
            self.reload_recipes();
        }
        let Some(recipe) = recipes::find(&self.recipes, name).cloned() else {
            return self.recipe_notice(ctx, t!("recipe-not-found", name = name.to_owned()), true);
        };
        let commands = recipe.commands();
        if !commands.is_empty()
            && !self
                .settings
                .allowed_recipes
                .contains(&recipe.fingerprint())
        {
            self.recipe_confirm = Some(CommandConfirm {
                recipe,
                all_tabs,
                remember: false,
            });
            return;
        }
        self.start_recipe(ctx, recipe, all_tabs);
    }

    fn recipe_notice(&mut self, ctx: &egui::Context, text: String, error: bool) {
        self.notice = Some(crate::text_tools_ui::Notice {
            text,
            error,
            since: Some(ctx.input(|i| i.time)),
            jump: None,
        });
    }

    fn start_recipe(&mut self, ctx: &egui::Context, recipe: Recipe, all_tabs: bool) {
        if self.recipe_job.is_some() || self.docs.is_empty() {
            return;
        }
        if let Some(i) = recipe
            .steps
            .iter()
            .position(|s| s.enabled && s.op.check().is_err())
        {
            let message = recipe.steps[i].op.check().unwrap_err();
            return self.recipe_notice(
                ctx,
                t!(
                    "recipe-failed",
                    name = recipe.name.clone(),
                    step = i + 1,
                    message = message
                ),
                true,
            );
        }
        let indices: Vec<usize> = if all_tabs {
            (0..self.docs.len()).collect()
        } else {
            vec![self.active]
        };
        let mut docs = Vec::new();
        let mut inputs = Vec::new();
        for i in indices {
            let doc = &self.docs[i];
            let hex = self.hex_views.contains_key(&doc.id) || doc.binary.is_some();
            if doc.editor.buffer.read_only || hex {
                if !all_tabs {
                    return self.recipe_notice(ctx, t!("tool-read-only"), true);
                }
                continue;
            }
            let select = !all_tabs && doc.editor.selections.has_selection();
            let ranges = if all_tabs {
                std::iter::once(0..doc.editor.buffer.len_chars()).collect()
            } else {
                doc.editor.tool_targets()
            };
            let texts: Vec<String> = ranges
                .iter()
                .map(|r| doc.editor.buffer.slice_string(r.clone()))
                .collect();
            let dir = doc
                .path
                .as_deref()
                .and_then(|p| p.parent())
                .map(|p| p.to_owned())
                .or_else(dirs::home_dir);
            inputs.push((texts, doc.editor.indent_unit(), dir, doc.path.clone()));
            docs.push(JobDoc {
                id: doc.id,
                version: doc.editor.buffer.version(),
                ranges,
                select,
            });
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let stop = cancel.clone();
        let steps = recipe.clone();
        std::thread::spawn(move || {
            let results = inputs
                .into_iter()
                .map(
                    |(texts, indent, dir, file): (
                        Vec<String>,
                        String,
                        Option<PathBuf>,
                        Option<PathBuf>,
                    )| {
                        let env = Env {
                            indent: &indent,
                            dir: dir.as_deref(),
                            file: file.as_deref(),
                            cancel: Some(&stop),
                        };
                        texts
                            .iter()
                            .map(|t| steps.run(t, &env))
                            .collect::<Result<Vec<String>, Failure>>()
                    },
                )
                .collect();
            let _ = tx.send(results);
        });
        self.recipe_job = Some(RecipeJob {
            name: recipe.name,
            docs,
            cancel,
            output: rx,
            started: ctx.input(|i| i.time),
            all_tabs,
        });
        self.poll_recipe(ctx);
    }

    /// Applies a finished recipe's results, each tab as one undo step.
    pub(super) fn poll_recipe(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.recipe_job else {
            return;
        };
        let Ok(results) = job.output.try_recv() else {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
            let elapsed = ctx.input(|i| i.time) - job.started;
            if elapsed > SHOW_RUNNING_AFTER
                && crate::recipe_ui::running_ui(ctx, &self.palette(), &job.name)
            {
                job.cancel.store(true, Ordering::Relaxed);
            }
            return;
        };
        let job = self.recipe_job.take().expect("running");
        let mut changed = 0;
        let mut skipped = 0;
        let mut failure = None;
        for (doc, result) in job.docs.into_iter().zip(results) {
            let outputs = match result {
                Ok(o) => o,
                Err(f) => {
                    failure.get_or_insert(f);
                    continue;
                }
            };
            let Some(index) = self.docs.iter().position(|d| d.id == doc.id) else {
                continue;
            };
            let editor = &mut self.docs[index].editor;
            if editor.buffer.version() != doc.version {
                skipped += 1;
                continue;
            }
            let edits: Vec<(Range<usize>, String)> = doc
                .ranges
                .into_iter()
                .zip(outputs)
                .filter(|(r, out)| editor.buffer.slice_string(r.clone()) != *out)
                .collect();
            if !edits.is_empty() {
                editor.replace_ranges(ctx, edits, doc.select);
                changed += 1;
            }
        }
        self.focus_editor = true;
        let text = if let Some(f) = failure {
            Some((
                t!(
                    "recipe-failed",
                    name = job.name.clone(),
                    step = f.step + 1,
                    message = f.message
                ),
                true,
            ))
        } else if skipped > 0 {
            Some((t!("pipe-text-changed"), true))
        } else if job.all_tabs && changed > 0 {
            Some((
                t!(
                    "recipe-changed-tabs",
                    name = job.name.clone(),
                    count = changed
                ),
                false,
            ))
        } else if changed == 0 {
            Some((t!("recipe-no-change", name = job.name.clone()), false))
        } else {
            None
        };
        if let Some((text, error)) = text {
            self.recipe_notice(ctx, text, error);
        }
    }

    /// The recipe window, the question before commands run, and a running
    /// recipe.
    pub(super) fn recipes_ui(&mut self, ctx: &egui::Context) {
        self.poll_recipe(ctx);
        let palette = self.palette();
        if let Some(confirm) = &mut self.recipe_confirm
            && let Some(answer) = confirm.ui(ctx, &palette)
        {
            let confirm = self.recipe_confirm.take().expect("open");
            if let ConfirmAnswer::Run = answer {
                if confirm.remember {
                    let print = confirm.recipe.fingerprint();
                    if !self.settings.allowed_recipes.contains(&print) {
                        self.settings.allowed_recipes.push(print);
                    }
                }
                self.start_recipe(ctx, confirm.recipe, confirm.all_tabs);
            }
        }
        if !self.recipe_window.open {
            return;
        }
        let (key, selection, indent) = match self.docs.get(self.active) {
            Some(doc) => {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                (doc.id, doc.editor.buffer.version()).hash(&mut h);
                for r in doc.editor.tool_targets() {
                    (r.start, r.end).hash(&mut h);
                }
                (
                    h.finish(),
                    doc.editor.selections.has_selection(),
                    doc.editor.indent_unit(),
                )
            }
            None => (0, false, String::new()),
        };
        let docs = &self.docs;
        let active = self.active;
        let text = move || -> String {
            let Some(doc) = docs.get(active) else {
                return String::new();
            };
            let mut out = String::new();
            let mut left = PREVIEW_CHARS;
            for r in doc.editor.tool_targets() {
                if left == 0 {
                    break;
                }
                let end = r.end.min(r.start + left);
                left -= end - r.start;
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&doc.editor.buffer.slice_string(r.start..end));
            }
            out
        };
        let input = PreviewInput {
            key,
            text: &text,
            selection,
            indent: &indent,
        };
        let shortcut = self
            .recipe_window
            .selected
            .and_then(|i| self.recipes.get(i))
            .and_then(|r| {
                crate::actions::all_shortcuts()
                    .into_iter()
                    .find(|(_, a)| *a == Action::RunRecipe(r.id.clone()))
            })
            .map(|(s, _)| ctx.format_shortcut(&s));
        let events = self.recipe_window.ui(
            ctx,
            &palette,
            &mut self.recipes,
            &input,
            shortcut.as_deref(),
        );
        for event in events {
            match event {
                RecipeEvent::Changed(i) => self.save_recipe(i),
                RecipeEvent::New => self.add_recipe(Recipe::new(&t!("recipe-new-name"))),
                RecipeEvent::Duplicate(i) => {
                    if let Some(r) = self.recipes.get(i).cloned() {
                        self.add_recipe(r);
                    }
                }
                RecipeEvent::Delete(i) => {
                    if i < self.recipes.len() {
                        let r = self.recipes.remove(i);
                        if let Some(path) = r.path
                            && let Err(e) = std::fs::remove_file(&path)
                        {
                            self.errors.push(e.to_string());
                        }
                    }
                }
                RecipeEvent::Import => self.import_recipe(),
                RecipeEvent::Export(i) => self.export_recipe(i),
                RecipeEvent::OpenFolder => self.run(ctx, Action::OpenRecipesFolder),
                RecipeEvent::Reload => self.reload_recipes(),
                RecipeEvent::SetShortcut {
                    index,
                    shortcut,
                    force,
                } => self.set_recipe_shortcut(ctx, index, shortcut, force),
                RecipeEvent::Run { index, all_tabs } => {
                    if let Some(id) = self.recipes.get(index).map(|r| r.id.clone()) {
                        self.run_recipe(ctx, &id, all_tabs);
                    }
                }
            }
        }
    }

    /// Gives a recipe a shortcut (or takes it away), checking first that no
    /// other command uses it; with `force`, the other command loses it.
    fn set_recipe_shortcut(
        &mut self,
        ctx: &egui::Context,
        index: usize,
        shortcut: Option<egui::KeyboardShortcut>,
        force: bool,
    ) {
        use crate::shortcut_ui::{current, store, used_by};
        let Some(id) = self.recipes.get(index).map(|r| r.id.clone()) else {
            return;
        };
        let action = Action::RunRecipe(id);
        let rows = self.shortcut_rows(ctx);
        let defaults = rows
            .iter()
            .find(|r| r.action == action)
            .map(|r| r.defaults.clone())
            .unwrap_or_default();
        let saved = &mut self.settings.shortcuts;
        let list: Vec<egui::KeyboardShortcut> = shortcut.into_iter().collect();
        if let Some(s) = shortcut
            && let Some(other) = used_by(&rows, saved, &action, &s)
        {
            if !force {
                self.recipe_window.conflict = Some(crate::recipe_ui::ShortcutConflict {
                    index,
                    shortcut: s,
                    other: other.path.clone(),
                });
                return;
            }
            let rest: Vec<_> = current(other, saved)
                .into_iter()
                .filter(|c| !crate::plugins::shortcut::same(c, &s))
                .collect();
            store(saved, &other.defaults, &other.action, &rest);
        }
        store(saved, &defaults, &action, &list);
        let errors = crate::actions::set_overrides(&self.settings.shortcuts);
        self.errors.extend(errors);
        self.toolbar.refresh();
    }

    fn import_recipe(&mut self) {
        let Some(paths) = rfd::FileDialog::new()
            .set_title(t!("recipe-import"))
            .add_filter(t!("recipe-file-filter"), &["json"])
            .pick_files()
        else {
            return;
        };
        for path in paths {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| Recipe::from_json(&t))
            {
                Ok(r) => self.add_recipe(r),
                Err(e) => {
                    self.recipe_window.message = Some(t!(
                        "recipe-read-failed",
                        path = path.display().to_string(),
                        error = e
                    ))
                }
            }
        }
    }

    fn export_recipe(&mut self, index: usize) {
        let Some(recipe) = self.recipes.get(index) else {
            return;
        };
        let name = recipe
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "recipe.json".into());
        let Some(path) = rfd::FileDialog::new()
            .set_title(t!("recipe-export"))
            .set_file_name(name)
            .add_filter(t!("recipe-file-filter"), &["json"])
            .save_file()
        else {
            return;
        };
        if let Err(e) = std::fs::write(&path, recipe.to_json()) {
            self.recipe_window.message = Some(t!(
                "recipe-save-failed",
                path = path.display().to_string(),
                error = e.to_string()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::lines::TrimKind;

    #[test]
    fn shortcuts_by_name_move_to_the_id() {
        let mut r = Recipe::new("Clean API dump");
        r.id = "clean-api-dump-7f3a".into();
        let recipes = vec![r];
        let old = crate::actions::command_key(&Action::RunRecipe("Clean API dump".into()));
        assert_eq!(
            migrated_recipe_key(&old, &recipes),
            Some(r#"{"RunRecipe":"clean-api-dump-7f3a"}"#.into())
        );
        let all = crate::actions::command_key(&Action::RunRecipeOnAllTabs("clean api dump".into()));
        assert_eq!(
            migrated_recipe_key(&all, &recipes),
            Some(r#"{"RunRecipeOnAllTabs":"clean-api-dump-7f3a"}"#.into())
        );
        // Already an id, another command, or a recipe that is gone: as is.
        let new = crate::actions::command_key(&Action::RunRecipe("clean-api-dump-7f3a".into()));
        assert_eq!(migrated_recipe_key(&new, &recipes), None);
        assert_eq!(migrated_recipe_key(r#""Save""#, &recipes), None);
        let gone = crate::actions::command_key(&Action::RunRecipe("Gone".into()));
        assert_eq!(migrated_recipe_key(&gone, &recipes), None);

        // Renaming keeps the shortcut and the toolbar button: both use the
        // id, which still finds the recipe.
        let mut recipes = recipes;
        recipes[0].name = "Scrub API dump".into();
        assert_eq!(migrated_recipe_key(&new, &recipes), None);
        assert_eq!(
            recipes::find(&recipes, "clean-api-dump-7f3a").map(|r| r.name.as_str()),
            Some("Scrub API dump")
        );
    }

    #[test]
    fn recording_turns_commands_into_steps() {
        let opts = SearchOptions {
            regex: true,
            extended: true,
            ..Default::default()
        };
        assert_eq!(
            step_for(&Action::ReplaceAll, "a+", "b", &opts),
            Some(Step::Replace {
                find: "a+".into(),
                replace: "b".into(),
                regex: true,
                match_case: false,
                whole_word: false,
                extended: false,
            })
        );
        assert_eq!(step_for(&Action::ReplaceAll, "", "b", &opts), None);
        assert_eq!(
            step_for(&Action::Trim(TrimKind::Both), "", "", &opts),
            Some(Step::Trim {
                kind: TrimKind::Both
            })
        );
        assert_eq!(
            step_for(
                &Action::Transform(TransformOp::Pipe {
                    command: "sort".into()
                }),
                "",
                "",
                &opts
            ),
            Some(Step::Command {
                command: "sort".into()
            })
        );
        assert_eq!(step_for(&Action::Duplicate, "", "", &opts), None);
        assert_eq!(
            step_for(&Action::Xml(XmlOp::CheckSyntax), "", "", &opts),
            None
        );
    }
}
