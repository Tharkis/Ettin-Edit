//! Settings that take effect while Ettin Edit runs: applying changed
//! settings, following the system's light/dark setting, the interface size,
//! reloading settings files edited outside the app, and Export / Import
//! Settings.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use egui::{Id, RichText};

use super::EttinApp;
use crate::actions::Action;
use crate::settings::{self, Settings};
use crate::watch::Kind;

/// Seconds between asks for the desktop's light/dark setting when the
/// window system doesn't report it (X11).
const PROBE_EVERY: f64 = 3.0;

/// Asks the desktop for its light/dark setting in the background.
#[derive(Default)]
pub(super) struct SystemThemeProbe {
    rx: Option<Receiver<Option<bool>>>,
    last: f64,
    /// True for dark, once known.
    dark: Option<bool>,
}

/// An import waiting for the user to say yes.
pub(super) struct ImportConfirm {
    path: PathBuf,
    archive: crate::settings_sync::Archive,
}

impl EttinApp {
    /// Makes changes to the settings take effect: the interface language,
    /// editor font, shortcuts, keymap, theme and shell. `old` is how the
    /// settings were before.
    pub(super) fn apply_settings(&mut self, ctx: &egui::Context, old: &Settings) {
        if self.settings.ui_language != old.ui_language {
            let errors = crate::i18n::set_language(&self.settings.ui_language);
            self.errors.extend(errors);
            // The toolbar keeps its labels; build them again.
            self.toolbar.refresh();
        }
        if self.settings.editor_font != old.editor_font
            && let Err(e) = crate::fonts::install(ctx, &self.settings.editor_font)
        {
            self.errors.push(e);
            self.settings.editor_font = old.editor_font.clone();
            let _ = crate::fonts::install(ctx, &self.settings.editor_font);
        }
        if self.settings.keymap != old.keymap || self.settings.shortcuts != old.shortcuts {
            crate::actions::set_keymap(self.settings.keymap);
            let errors = crate::actions::set_overrides(&self.settings.shortcuts);
            self.errors.extend(errors);
        }
        if self.settings.shell != old.shell {
            crate::tools::pipe::set_shell(&self.settings.shell);
        }
        let name = self.settings.theme.clone();
        if self.themes[self.theme_idx].name != name && self.themes.iter().any(|t| t.name == name) {
            self.run(ctx, Action::SetTheme(name));
        }
    }

    /// Each frame: the interface size, the system's light/dark setting and
    /// settings files changed on disk.
    pub(super) fn settings_frame(&mut self, ctx: &egui::Context) {
        let zoom = self.settings.ui_scale.clamp(80, 200) as f32 / 100.0;
        if (ctx.zoom_factor() - zoom).abs() > 0.001 {
            ctx.set_zoom_factor(zoom);
        }
        self.follow_system_theme(ctx);
        let now = ctx.input(|i| i.time);
        let Some(watcher) = &mut self.watcher else {
            return;
        };
        let (ready, waiting) = watcher.take_ready(now);
        if waiting {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        for (kind, path) in ready {
            self.reload_changed(ctx, kind, &path);
        }
    }

    /// True for dark, false for light, `None` when the system doesn't say.
    fn system_dark(&mut self, ctx: &egui::Context) -> Option<bool> {
        if let Some(theme) = ctx.system_theme() {
            return Some(theme == egui::Theme::Dark);
        }
        let probe = &mut self.theme_probe;
        if let Some(rx) = &probe.rx
            && let Ok(answer) = rx.try_recv()
        {
            probe.dark = answer;
            probe.rx = None;
        }
        let now = ctx.input(|i| i.time);
        if probe.rx.is_none() && (probe.last == 0.0 || now - probe.last >= PROBE_EVERY) {
            probe.last = now.max(0.001);
            let (tx, rx) = channel();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(crate::system::desktop_prefers_dark());
                ctx.request_repaint();
            });
            probe.rx = Some(rx);
        }
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(PROBE_EVERY));
        probe.dark
    }

    /// With "Follow the system's light/dark setting", uses the light or
    /// dark theme of the pair.
    fn follow_system_theme(&mut self, ctx: &egui::Context) {
        if !self.settings.follow_system_theme {
            return;
        }
        let Some(dark) = self.system_dark(ctx) else {
            return;
        };
        let want = if dark {
            self.settings.dark_theme.clone()
        } else {
            self.settings.light_theme.clone()
        };
        if want != self.settings.theme && self.themes.iter().any(|t| t.name == want) {
            self.run(ctx, Action::SetTheme(want));
        }
    }

    /// A theme picked from a menu while following the system becomes the
    /// theme for the system's current mode.
    pub(super) fn theme_picked(&mut self, name: &str) {
        if !self.settings.follow_system_theme {
            return;
        }
        let dark = self.theme_probe.dark.or_else(|| {
            self.themes
                .iter()
                .find(|t| t.name == name)
                .map(|t| t.palette.dark)
        });
        match dark {
            Some(true) => self.settings.dark_theme = name.to_owned(),
            Some(false) => self.settings.light_theme = name.to_owned(),
            None => {}
        }
    }

    /// The path as shown in messages: relative to the settings folder.
    fn short_path(path: &Path) -> String {
        settings::config_dir()
            .and_then(|d| path.strip_prefix(d).ok().map(Path::to_path_buf))
            .unwrap_or_else(|| path.to_path_buf())
            .display()
            .to_string()
            .replace('\\', "/")
    }

    fn live_error(&mut self, path: &Path, error: &str) {
        let place = crate::watch::error_place(error);
        let detail = match place {
            Some((line, col)) => t!("live-reload-place", line = line, column = col),
            None => error.to_owned(),
        };
        self.notice = Some(crate::text_tools_ui::Notice {
            text: t!(
                "live-reload-failed",
                file = Self::short_path(path),
                error = detail
            ),
            error: true,
            since: None,
            jump: None,
            open: Some((path.to_path_buf(), place.map_or(1, |p| p.0))),
        });
    }

    fn live_ok(&mut self, ctx: &egui::Context, path: &Path) {
        self.notify(
            ctx,
            t!("live-reloaded", file = Self::short_path(path)),
            None,
        );
    }

    /// Reloads one settings file that changed on disk. A file that can't be
    /// read leaves the last good version in use and says what's wrong.
    fn reload_changed(&mut self, ctx: &egui::Context, kind: Kind, path: &Path) {
        // Other files' problems were reported when they changed or at
        // start-up; don't bring them up again in a window.
        let errors = self.errors.len();
        self.reload_one(ctx, kind, path);
        if kind != Kind::Settings {
            self.errors.truncate(errors);
        }
    }

    fn reload_one(&mut self, ctx: &egui::Context, kind: Kind, path: &Path) {
        let text = std::fs::read_to_string(path).ok();
        match kind {
            Kind::Settings => {
                let Some(text) = text else { return };
                match Settings::from_json(&text) {
                    Ok(new) => {
                        let old = std::mem::replace(&mut self.settings, new);
                        self.apply_settings(ctx, &old);
                        self.live_ok(ctx, path);
                    }
                    Err(e) => self.live_error(path, &e),
                }
            }
            Kind::Theme => {
                if let Some(text) = &text
                    && let Err(e) = crate::theme::check_source(text)
                {
                    return self.live_error(path, &e);
                }
                self.reload_themes();
                self.apply_style(ctx);
                self.live_ok(ctx, path);
            }
            Kind::Language => {
                if text.is_some()
                    && let Err(e) = crate::highlight::check_user_file(path)
                {
                    return self.live_error(path, &e);
                }
                self.reload_languages();
                self.live_ok(ctx, path);
            }
            Kind::Translation => {
                if let Some(text) = &text
                    && let Err(e) = crate::i18n::check_ftl(text)
                {
                    return self.live_error(path, &e);
                }
                let errors = crate::i18n::set_language(&self.settings.ui_language);
                self.toolbar.refresh();
                match errors.first() {
                    Some(e) => self.live_error(path, e),
                    None => self.live_ok(ctx, path),
                }
            }
            Kind::Recipe => {
                if let Some(text) = &text
                    && let Err(e) = crate::recipes::Recipe::from_json(text)
                {
                    return self.live_error(path, &e);
                }
                self.reload_recipes();
                self.live_ok(ctx, path);
            }
        }
    }

    /// Before the settings are written: if `settings.json` was edited
    /// outside the app and not read yet, read it first so the edit isn't
    /// lost.
    pub(super) fn catch_up_settings_file(&mut self, ctx: Option<&egui::Context>) {
        let Some(path) = settings::settings_path() else {
            return;
        };
        if !crate::watch::changed_since_own_write(&path) {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(new) = Settings::from_json(&text)
        {
            let old = std::mem::replace(&mut self.settings, new);
            if let Some(ctx) = ctx {
                self.apply_settings(ctx, &old);
            }
        }
    }

    /// The installed plugins, for an export.
    fn plugin_refs(&self) -> Vec<crate::settings_sync::PluginRef> {
        self.plugins
            .plugins
            .iter()
            .map(|p| crate::settings_sync::PluginRef {
                id: p.id.clone(),
                version: p
                    .manifest
                    .as_ref()
                    .map(|m| m.version.clone())
                    .unwrap_or_default(),
            })
            .collect()
    }

    pub(super) fn export_settings(&mut self, ctx: &egui::Context) {
        let Some(config) = settings::config_dir() else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(t!("export-settings"))
            .set_file_name(crate::settings_sync::default_file_name())
            .add_filter(t!("settings-file-type"), &["zip"])
            .save_file()
        else {
            return;
        };
        let plugins = self.plugin_refs();
        match crate::settings_sync::export(&config, &self.settings.to_json(), &plugins, &path) {
            Ok(count) => self.notify(
                ctx,
                t!(
                    "settings-exported",
                    count = count,
                    file = path.display().to_string()
                ),
                None,
            ),
            Err(e) => self.errors.push(t!(
                "settings-export-failed",
                file = path.display().to_string(),
                error = e
            )),
        }
    }

    pub(super) fn import_settings(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(t!("import-settings"))
            .add_filter(t!("settings-file-type"), &["zip"])
            .pick_file()
        else {
            return;
        };
        self.open_settings_import(path);
    }

    /// Reads an export and asks before importing it.
    pub(super) fn open_settings_import(&mut self, path: PathBuf) {
        match crate::settings_sync::read(&path) {
            Ok(archive) => self.import_confirm = Some(ImportConfirm { path, archive }),
            Err(e) => self.errors.push(t!(
                "settings-import-failed",
                file = path.display().to_string(),
                error = e
            )),
        }
    }

    /// The question before an import, listing what it replaces.
    pub(super) fn import_dialog(&mut self, ctx: &egui::Context) {
        let Some(confirm) = &self.import_confirm else {
            return;
        };
        let Some(config) = settings::config_dir() else {
            self.import_confirm = None;
            return;
        };
        let replaces = confirm.archive.replaces(&config);
        let [settings_files, themes, recipes, languages] = confirm.archive.counts();
        let installed: Vec<String> = self.plugins.plugins.iter().map(|p| p.id.clone()).collect();
        let missing: Vec<String> = confirm
            .archive
            .plugins
            .iter()
            .filter(|p| !installed.contains(&p.id))
            .map(|p| format!("{} {}", p.id, p.version))
            .collect();
        let palette = self.palette();
        let mut answer = None;
        egui::Modal::new(Id::new("import-settings")).show(ctx, |ui| {
            ui.set_width(460.0);
            ui.heading(t!("import-settings"));
            ui.add_space(4.0);
            ui.label(
                RichText::new(t!(
                    "import-from",
                    file = confirm.path.display().to_string(),
                    version = confirm.archive.version.clone()
                ))
                .small()
                .weak(),
            );
            ui.add_space(6.0);
            ui.label(t!(
                "import-contents",
                settings = settings_files,
                themes = themes,
                recipes = recipes,
                languages = languages
            ));
            if replaces.is_empty() {
                ui.label(t!("import-replaces-nothing"));
            } else {
                ui.label(t!("import-replaces", count = replaces.len()));
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        for name in &replaces {
                            ui.label(RichText::new(format!("  {name}")).monospace().small());
                        }
                    });
                ui.label(RichText::new(t!("import-backup-note")).small().weak());
            }
            if !missing.is_empty() {
                ui.add_space(4.0);
                ui.label(t!("import-plugins-missing", plugins = missing.join(", ")));
            }
            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new(t!("import")).fill(palette.accent.gamma_multiply(0.6)))
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui.button(t!("cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    answer = Some(false);
                }
            });
        });
        match answer {
            Some(true) => {
                let confirm = self.import_confirm.take().expect("open");
                self.finish_import(ctx, &config, &confirm.archive);
            }
            Some(false) => self.import_confirm = None,
            None => {}
        }
    }

    fn finish_import(
        &mut self,
        ctx: &egui::Context,
        config: &Path,
        archive: &crate::settings_sync::Archive,
    ) {
        match archive.apply(config) {
            Ok(backup) => {
                if let Some(text) =
                    settings::settings_path().and_then(|p| std::fs::read_to_string(p).ok())
                    && let Ok(new) = Settings::from_json(&text)
                {
                    let old = std::mem::replace(&mut self.settings, new);
                    self.apply_settings(ctx, &old);
                }
                let (toolbar, error) = super::ribbon::Toolbar::load();
                self.toolbar.layout = toolbar.layout;
                self.errors.extend(error);
                self.reload_themes();
                self.reload_languages();
                self.reload_recipes();
                let name = self.settings.theme.clone();
                self.run(ctx, Action::SetTheme(name));
                let text = match backup {
                    Some(dir) => t!(
                        "settings-imported-backup",
                        folder = dir.display().to_string()
                    ),
                    None => t!("settings-imported"),
                };
                self.notify(ctx, text, None);
            }
            Err(e) => self.errors.push(t!(
                "settings-import-failed",
                file = config.display().to_string(),
                error = e
            )),
        }
    }

    pub(super) fn open_settings_folder(&mut self) {
        let Some(dir) = settings::config_dir() else {
            return;
        };
        let _ = std::fs::create_dir_all(&dir);
        if let Err(e) = crate::system::open_folder(&dir) {
            self.errors.push(e);
        }
    }
}
