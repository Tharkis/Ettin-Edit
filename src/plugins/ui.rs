//! Plugin windows: Manage Plugins, the plugin log, the permission prompt and
//! the text prompt plugins can show.

use std::collections::BTreeMap;

use egui::{Align, Frame, Id, Key, Layout, Margin, RichText, ScrollArea, Ui};

use super::store::{DEFAULT_INDEX_URL, IndexEntry, IndexState, Store, Tab, is_newer};
use super::{PluginHost, Status};
use crate::settings::PluginPrefs;
use crate::theme::Palette;

/// What the user asked for in Manage Plugins.
pub enum ManagerAction {
    SetEnabled(String, bool),
    SetTrust(String, bool),
    Start(String),
    Stop(String),
    Allow(String),
    OpenFolder(String),
    Reload,
    OpenPluginsFolder,
    InstallSamples,
    ShowLog,
    ShowGuide,
    /// Read the plugin index again.
    RefreshIndex,
    /// Install or update a plugin from the index, by id.
    Install(String),
    InstallFromFile,
    InstallFromFolder,
    /// Move a plugin to the trash (already confirmed).
    Remove(String),
}

/// What Manage Plugins shows and changes.
pub struct ManagerView<'a> {
    pub host: &'a PluginHost,
    pub prefs: &'a BTreeMap<String, PluginPrefs>,
    pub python: &'a mut String,
    pub node: &'a mut String,
    pub index_url: &'a mut String,
    pub store: &'a mut Store,
    pub palette: &'a Palette,
}

/// The Manage Plugins window. Returns what was clicked.
pub fn manager(ctx: &egui::Context, open: &mut bool, view: ManagerView) -> Vec<ManagerAction> {
    let ManagerView {
        host,
        prefs,
        python,
        node,
        index_url,
        store,
        palette,
    } = view;
    let mut actions = Vec::new();
    egui::Window::new("Plugins")
        .id(Id::new("plugin-manager"))
        .open(open)
        .default_size([640.0, 500.0])
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(
                RichText::new(
                    "Plugins run as separate programs with your access to files and the network. Only run plugins you trust.",
                )
                .color(palette.text_dim),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let installed = format!("Installed ({})", host.plugins.len());
                ui.selectable_value(&mut store.tab, Tab::Installed, installed);
                let available = match &store.index {
                    IndexState::Loaded(list) => format!("Available ({})", list.len()),
                    _ => "Available".to_owned(),
                };
                ui.selectable_value(&mut store.tab, Tab::Available, available);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let idle = store.busy.is_none();
                    if ui
                        .add_enabled(idle, egui::Button::new("Install from Folder…"))
                        .on_hover_text("Copy a plugin's folder (one with a plugin.json) into the plugins folder")
                        .clicked()
                    {
                        actions.push(ManagerAction::InstallFromFolder);
                    }
                    if ui
                        .add_enabled(idle, egui::Button::new("Install from File…"))
                        .on_hover_text("Install a plugin from a .zip file")
                        .clicked()
                    {
                        actions.push(ManagerAction::InstallFromFile);
                    }
                });
            });
            ui.separator();
            if let Some(what) = &store.busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(what);
                });
            }
            let mut dismiss = false;
            if let Some(error) = &store.error {
                ui.horizontal(|ui| {
                    if ui.small_button("✕").on_hover_text("Dismiss").clicked() {
                        dismiss = true;
                    }
                    ui.add(
                        egui::Label::new(RichText::new(error).color(palette.modified))
                            .wrap()
                            .selectable(true),
                    );
                });
            }
            if dismiss {
                store.error = None;
            }
            match store.tab {
                Tab::Installed => installed_tab(ui, host, prefs, python, node, store, palette, &mut actions),
                Tab::Available => available_tab(ui, host, index_url, store, palette, &mut actions),
            }
        });
    if let Some(id) = store.confirm_remove.clone() {
        let name = host
            .plugins
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name().to_owned())
            .unwrap_or_else(|| id.clone());
        let mut answer = None;
        let response = egui::Modal::new(Id::new("plugin-remove")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading(format!("Remove \u{201c}{name}\u{201d}?"));
            ui.add_space(6.0);
            ui.label("Its folder moves to the trash, so you can get it back from there.");
            ui.add_space(10.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 24.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if ui.button("Remove").clicked() {
                        answer = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        answer = Some(false);
                    }
                },
            );
        });
        if answer.is_none() && response.should_close() {
            answer = Some(false);
        }
        if let Some(yes) = answer {
            store.confirm_remove = None;
            if yes {
                actions.push(ManagerAction::Remove(id));
            }
        }
    }
    actions
}

#[allow(clippy::too_many_arguments)]
fn installed_tab(
    ui: &mut Ui,
    host: &PluginHost,
    prefs: &BTreeMap<String, PluginPrefs>,
    python: &mut String,
    node: &mut String,
    store: &mut Store,
    palette: &Palette,
    actions: &mut Vec<ManagerAction>,
) {
    let footer = 96.0;
    ScrollArea::vertical()
        .id_salt("plugins-installed")
        .max_height((ui.available_height() - footer).max(120.0))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            if host.plugins.is_empty() {
                ui.add_space(12.0);
                ui.label("No plugins yet.");
                ui.label(
                    RichText::new(
                        "Install the samples to see how plugins work, find plugins under Available, or put a plugin's folder in the plugins folder.",
                    )
                    .color(palette.text_dim),
                );
                ui.add_space(6.0);
                if ui.button("Install Sample Plugins").clicked() {
                    actions.push(ManagerAction::InstallSamples);
                }
            }
            for p in &host.plugins {
                let version = p.manifest.as_ref().map(|m| m.version.as_str()).unwrap_or("");
                let update = store.update_for(&p.id, version);
                let card = Card {
                    pref: prefs.get(&p.id),
                    update,
                    busy: store.busy.is_some(),
                };
                if plugin_card(ui, p, card, palette, actions) {
                    store.confirm_remove = Some(p.id.clone());
                }
            }
        });
    ui.separator();
    egui::Grid::new("plugin-runtimes")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Python");
            ui.add(
                egui::TextEdit::singleline(python)
                    .hint_text(if cfg!(windows) {
                        "python (found on PATH)"
                    } else {
                        "python3 (found on PATH)"
                    })
                    .desired_width(280.0),
            );
            ui.end_row();
            ui.label("Node");
            ui.add(
                egui::TextEdit::singleline(node)
                    .hint_text("node (found on PATH)")
                    .desired_width(280.0),
            );
            ui.end_row();
        });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Open Plugins Folder").clicked() {
            actions.push(ManagerAction::OpenPluginsFolder);
        }
        if ui
            .button("Reload Plugins")
            .on_hover_text("Stop every plugin, look for plugins again and start them")
            .clicked()
        {
            actions.push(ManagerAction::Reload);
        }
        if ui.button("Plugin Log").clicked() {
            actions.push(ManagerAction::ShowLog);
        }
        if ui.button("Plugin Guide").clicked() {
            actions.push(ManagerAction::ShowGuide);
        }
    });
}

fn available_tab(
    ui: &mut Ui,
    host: &PluginHost,
    index_url: &mut String,
    store: &mut Store,
    palette: &Palette,
    actions: &mut Vec<ManagerAction>,
) {
    ui.horizontal(|ui| {
        ui.label("Filter");
        ui.add(
            egui::TextEdit::singleline(&mut store.filter)
                .hint_text("Name, author or description")
                .desired_width(260.0),
        );
        let loading = matches!(store.index, IndexState::Loading);
        if ui
            .add_enabled(!loading, egui::Button::new("Refresh"))
            .on_hover_text("Read the plugin index again")
            .clicked()
        {
            actions.push(ManagerAction::RefreshIndex);
        }
    });
    ui.add_space(4.0);
    let footer = 34.0;
    ScrollArea::vertical()
        .id_salt("plugins-available")
        .max_height((ui.available_height() - footer).max(120.0))
        .auto_shrink([false, true])
        .show(ui, |ui| match &store.index {
            IndexState::NotLoaded | IndexState::Loading => {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Reading the plugin index…");
                });
            }
            IndexState::Failed(why) => {
                ui.add_space(12.0);
                ui.add(
                    egui::Label::new(RichText::new(why).color(palette.modified))
                        .wrap()
                        .selectable(true),
                );
                ui.add_space(6.0);
                if ui.button("Try Again").clicked() {
                    actions.push(ManagerAction::RefreshIndex);
                }
            }
            IndexState::Loaded(list) => {
                let shown: Vec<&IndexEntry> =
                    list.iter().filter(|e| e.matches(&store.filter)).collect();
                if list.is_empty() {
                    ui.label(
                        RichText::new("The plugin index lists no plugins.").color(palette.text_dim),
                    );
                } else if shown.is_empty() {
                    ui.label(RichText::new("No plugins match the filter.").color(palette.text_dim));
                }
                for e in shown {
                    let installed = host.plugins.iter().find(|p| p.id == e.id).map(|p| {
                        p.manifest
                            .as_ref()
                            .map(|m| m.version.clone())
                            .unwrap_or_default()
                    });
                    available_card(
                        ui,
                        e,
                        installed.as_deref(),
                        store.busy.is_some(),
                        palette,
                        actions,
                    );
                }
            }
        });
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Index");
        let edit = ui
            .add(
                egui::TextEdit::singleline(index_url)
                    .hint_text(DEFAULT_INDEX_URL)
                    .desired_width(ui.available_width() - 70.0),
            )
            .on_hover_text("The address of the JSON list of plugins. Press Enter to read it.");
        if edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            actions.push(ManagerAction::RefreshIndex);
        }
        if ui
            .add_enabled(
                index_url.trim() != DEFAULT_INDEX_URL,
                egui::Button::new("Default"),
            )
            .on_hover_text("Use Ettin Edit's own plugin index")
            .clicked()
        {
            *index_url = DEFAULT_INDEX_URL.to_owned();
            actions.push(ManagerAction::RefreshIndex);
        }
    });
}

fn available_card(
    ui: &mut Ui,
    e: &IndexEntry,
    installed: Option<&str>,
    busy: bool,
    palette: &Palette,
    actions: &mut Vec<ManagerAction>,
) {
    Frame::new()
        .fill(palette.tab_bg)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(6.0)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(e.display_name()).strong());
                if !e.version.is_empty() {
                    ui.label(RichText::new(&e.version).color(palette.text_dim));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(min) = e.needs_newer_app() {
                        ui.label(
                            RichText::new(format!("Needs Ettin Edit {min}"))
                                .color(palette.modified),
                        );
                        return;
                    }
                    match installed {
                        Some(v) if is_newer(&e.version, v) => {
                            if ui
                                .add_enabled(
                                    !busy,
                                    egui::Button::new(format!("Update to {}", e.version)),
                                )
                                .on_hover_text(format!(
                                    "Installed: {}",
                                    if v.is_empty() { "no version" } else { v }
                                ))
                                .clicked()
                            {
                                actions.push(ManagerAction::Install(e.id.clone()));
                            }
                        }
                        Some(_) => {
                            ui.label(RichText::new("✔ Installed").color(palette.accent));
                        }
                        None => {
                            if ui
                                .add_enabled(!busy, egui::Button::new("Install"))
                                .clicked()
                            {
                                actions.push(ManagerAction::Install(e.id.clone()));
                            }
                        }
                    }
                });
            });
            if !e.description.is_empty() {
                ui.add(
                    egui::Label::new(RichText::new(&e.description).color(palette.text_dim)).wrap(),
                );
            }
            ui.horizontal(|ui| {
                if !e.author.is_empty() {
                    ui.label(
                        RichText::new(format!("By {}", e.author))
                            .small()
                            .color(palette.text_dim),
                    );
                }
                if e.homepage.starts_with("https://") || e.homepage.starts_with("http://") {
                    ui.hyperlink_to(RichText::new("Home page").small(), &e.homepage);
                }
            });
        });
    ui.add_space(6.0);
}

/// What a plugin's card in the Installed list needs besides the plugin.
struct Card<'a> {
    pref: Option<&'a PluginPrefs>,
    /// A later version in the index.
    update: Option<&'a IndexEntry>,
    /// An install is running.
    busy: bool,
}

/// Draws an installed plugin. Returns true if Remove was clicked.
fn plugin_card(
    ui: &mut Ui,
    p: &super::Plugin,
    card: Card,
    palette: &Palette,
    actions: &mut Vec<ManagerAction>,
) -> bool {
    let mut remove = false;
    let pref = card.pref.cloned().unwrap_or_default();
    Frame::new()
        .fill(palette.tab_bg)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(6.0)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let mut enabled = !pref.disabled;
                if ui
                    .checkbox(&mut enabled, "")
                    .on_hover_text("Turn the plugin on or off")
                    .changed()
                {
                    actions.push(ManagerAction::SetEnabled(p.id.clone(), enabled));
                }
                ui.label(RichText::new(p.name()).strong());
                if let Ok(m) = &p.manifest {
                    if !m.version.is_empty() {
                        ui.label(RichText::new(&m.version).color(palette.text_dim));
                    }
                    ui.label(RichText::new(m.runtime.label()).small().color(palette.text_dim));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (text, color) = match &p.status {
                        Status::Running => ("Running", palette.accent),
                        Status::Stopped => ("Stopped", palette.text_dim),
                        Status::Disabled => ("Off", palette.text_dim),
                        Status::NeedsPermission => ("Waiting for permission", palette.modified),
                        Status::Failed(_) => ("Not running", palette.modified),
                    };
                    ui.label(RichText::new(text).color(color));
                });
            });
            if let Ok(m) = &p.manifest {
                if !m.description.is_empty() {
                    ui.label(RichText::new(&m.description).color(palette.text_dim));
                }
                if !m.author.is_empty() {
                    ui.label(
                        RichText::new(format!("By {}", m.author))
                            .small()
                            .color(palette.text_dim),
                    );
                }
            }
            if let Status::Failed(why) = &p.status {
                ui.label(RichText::new(why).color(palette.modified));
            }
            ui.horizontal(|ui| {
                let usable = p.manifest.is_ok() && !pref.disabled;
                if let Some(e) = card.update
                    && ui
                        .add_enabled(!card.busy, egui::Button::new(RichText::new(format!("Update to {}", e.version)).color(palette.accent)))
                        .on_hover_text("Download the new version from the plugin index; the old one moves to the trash")
                        .clicked()
                {
                    actions.push(ManagerAction::Install(p.id.clone()));
                }
                match &p.status {
                    Status::Running => {
                        if ui.button("Stop").clicked() {
                            actions.push(ManagerAction::Stop(p.id.clone()));
                        }
                        if ui.button("Restart").clicked() {
                            actions.push(ManagerAction::Start(p.id.clone()));
                        }
                    }
                    Status::NeedsPermission => {
                        if ui.button("Allow…").clicked() {
                            actions.push(ManagerAction::Allow(p.id.clone()));
                        }
                    }
                    _ => {
                        if ui.add_enabled(usable, egui::Button::new("Start")).clicked() {
                            actions.push(ManagerAction::Start(p.id.clone()));
                        }
                    }
                }
                if ui.button("Open Folder").clicked() {
                    actions.push(ManagerAction::OpenFolder(p.id.clone()));
                }
                if ui
                    .add_enabled(!card.busy, egui::Button::new("Remove…"))
                    .on_hover_text("Move the plugin's folder to the trash")
                    .clicked()
                {
                    remove = true;
                }
                let mut trust = pref.trust_changes;
                if ui
                    .checkbox(&mut trust, "Don't ask when it changes")
                    .on_hover_text("For a plugin you're writing: run it after each change without asking again")
                    .changed()
                {
                    actions.push(ManagerAction::SetTrust(p.id.clone(), trust));
                }
            });
        });
    ui.add_space(6.0);
    remove
}

/// The plugin log window.
pub fn log_window(ctx: &egui::Context, open: &mut bool, host: &mut PluginHost, palette: &Palette) {
    let mut clear = false;
    let mut copy = false;
    egui::Window::new("Plugin Log")
        .id(Id::new("plugin-log"))
        .open(open)
        .default_size([680.0, 320.0])
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                clear = ui.button("Clear").clicked();
                copy = ui.button("Copy All").clicked();
                ui.label(
                    RichText::new("Messages, print() and console.log() output from plugins")
                        .color(palette.text_dim),
                );
            });
            ui.separator();
            ScrollArea::both()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if host.log.is_empty() {
                        ui.label(RichText::new("Nothing yet.").color(palette.text_dim));
                    }
                    for line in &host.log {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&line.time)
                                    .monospace()
                                    .color(palette.text_dim),
                            );
                            ui.label(
                                RichText::new(&line.plugin)
                                    .monospace()
                                    .color(palette.accent),
                            );
                            let text = RichText::new(&line.text).monospace();
                            let text = if line.error {
                                text.color(palette.modified)
                            } else {
                                text
                            };
                            ui.add(egui::Label::new(text).selectable(true));
                        });
                    }
                });
        });
    if clear {
        host.log.clear();
    }
    if copy {
        let all: Vec<String> = host
            .log
            .iter()
            .map(|l| format!("{} {} {}", l.time, l.plugin, l.text))
            .collect();
        ctx.copy_text(all.join("\n"));
    }
}

pub enum Permission {
    Allow,
    Deny,
    /// Asked again next time.
    Later,
}

/// Asks whether a plugin may run.
pub fn permission(
    ctx: &egui::Context,
    host: &PluginHost,
    id: &str,
    changed: bool,
    palette: &Palette,
) -> Option<Permission> {
    let p = host.plugins.iter().find(|p| p.id == id)?;
    let mut answer = None;
    let response = egui::Modal::new(Id::new("plugin-permission")).show(ctx, |ui| {
        ui.set_width(440.0);
        ui.heading(format!("Allow \u{201c}{}\u{201d} to run?", p.name()));
        ui.add_space(6.0);
        if changed {
            ui.label(RichText::new("This plugin changed since you allowed it.").color(palette.modified));
            ui.add_space(4.0);
        }
        ui.label(
            "Plugins run as separate programs with your user account. This one could read and change your open documents and your files, and use the network. Only allow plugins you trust.",
        );
        ui.add_space(8.0);
        egui::Grid::new("plugin-permission-details")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                if let Ok(m) = &p.manifest {
                    if !m.author.is_empty() {
                        ui.label(RichText::new("Author").color(palette.text_dim));
                        ui.label(&m.author);
                        ui.end_row();
                    }
                    ui.label(RichText::new("Runs").color(palette.text_dim));
                    ui.label(format!(
                        "{} ({})",
                        m.main.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
                        m.runtime.label()
                    ));
                    ui.end_row();
                }
                ui.label(RichText::new("Folder").color(palette.text_dim));
                ui.add(egui::Label::new(p.dir.display().to_string()).wrap());
                ui.end_row();
            });
        ui.add_space(12.0);
        ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 24.0), Layout::right_to_left(Align::Center), |ui| {
            if ui.button("Allow").clicked() {
                answer = Some(Permission::Allow);
            }
            if ui
                .button("Don't Allow")
                .on_hover_text("Turn the plugin off. You can turn it on in Manage Plugins.")
                .clicked()
            {
                answer = Some(Permission::Deny);
            }
        });
    });
    if answer.is_none() && response.should_close() {
        answer = Some(Permission::Later);
    }
    answer
}

/// A plugin asking the user for a line of text.
pub struct TextPrompt {
    pub plugin: String,
    pub run: u64,
    pub id: serde_json::Value,
    pub title: String,
    pub prompt: String,
    pub value: String,
    pub focused: bool,
}

/// Shows the prompt; returns `Some(answer)` once the user is done
/// (`Some(None)` if they cancelled).
pub fn text_prompt(ctx: &egui::Context, prompt: &mut TextPrompt) -> Option<Option<String>> {
    let mut result = None;
    let response = egui::Modal::new(Id::new("plugin-prompt")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading(&prompt.title);
        ui.add_space(4.0);
        ui.label(&prompt.prompt);
        let id = Id::new("plugin-prompt-text");
        let mut output = egui::TextEdit::singleline(&mut prompt.value)
            .id(id)
            .desired_width(f32::INFINITY)
            .show(ui);
        if !prompt.focused {
            // Start with the suggested value selected, so typing replaces it.
            let all = egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(prompt.value.chars().count()),
            );
            output.state.cursor.set_char_range(Some(all));
            output.state.store(ui.ctx(), id);
            output.response.request_focus();
            prompt.focused = true;
        }
        let edit = output.response;
        if edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            result = Some(Some(prompt.value.clone()));
        }
        ui.add_space(10.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 24.0),
            Layout::right_to_left(Align::Center),
            |ui| {
                if ui.button("OK").clicked() {
                    result = Some(Some(prompt.value.clone()));
                }
                if ui.button("Cancel").clicked() {
                    result = Some(None);
                }
            },
        );
    });
    if result.is_none() && response.should_close() {
        result = Some(None);
    }
    result
}
