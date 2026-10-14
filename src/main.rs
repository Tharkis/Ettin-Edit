//! Ettin Edit: a modern, cross-platform text editor.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod actions;
mod app;
mod compare;
mod compare_ui;
mod csv_query;
mod document;
mod editor;
mod encoding;
mod file_ui;
mod folder_ui;
mod fonts;
mod function_list;
mod function_list_ui;
mod highlight;
mod instance;
mod macros;
mod menu;
mod palette_ui;
mod plugins;
mod prefs_ui;
mod search;
mod search_ui;
mod session;
mod settings;
mod shortcut_ui;
mod style_ui;
mod system;
mod text_tools_ui;
mod theme;
mod tools;
mod tools_ui;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut multi_instance = false;
    let mut files: Vec<PathBuf> = Vec::new();
    for arg in std::env::args_os().skip(1) {
        if arg == "--multi-instance" || arg == "-multiInst" {
            multi_instance = true;
        } else {
            files.push(PathBuf::from(arg));
        }
    }

    settings::migrate_old_config();

    // Hand the files to an Ettin Edit that is already running, if there is one.
    // `owner` is the window that keeps the session and backups.
    let mut owner = true;
    if let Some(dir) = settings::config_dir() {
        let single = settings::Settings::load().single_instance && !multi_instance;
        if single {
            if instance::send_to_running(&dir, &files) {
                return Ok(());
            }
        } else if instance::is_running(&dir) {
            owner = false;
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Ettin Edit")
            .with_app_id("ettin-edit")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([480.0, 320.0])
            .with_drag_and_drop(true)
            .with_icon(std::sync::Arc::new(
                eframe::icon_data::from_png_bytes(include_bytes!(
                    "../assets/icons/ettin-edit-256.png"
                ))
                .expect("bundled icon is a valid PNG"),
            )),
        ..Default::default()
    };

    eframe::run_native(
        "Ettin Edit",
        options,
        Box::new(move |cc| Ok(Box::new(app::EttinApp::new(cc, files, owner)))),
    )
}
