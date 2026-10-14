//! NixPad: a modern, cross-platform text editor.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod actions;
mod app;
mod document;
mod highlight;
mod settings;
mod theme;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let files: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NixPad")
            .with_app_id("nixpad")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([480.0, 320.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "NixPad",
        options,
        Box::new(move |cc| Ok(Box::new(app::NixPadApp::new(cc, files)))),
    )
}
