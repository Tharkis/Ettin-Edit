//! The folder panel (File > Open Folder as Workspace): a folder's files as a
//! tree on the left, to open them with a click.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use egui::{RichText, ScrollArea, Ui};

use crate::theme::Palette;

/// Entries read from one folder at most; more are left out with a note.
const MAX_ENTRIES: usize = 5000;
/// Seconds between re-reading the open folders, to pick up new and removed files.
const REFRESH_EVERY: f64 = 3.0;

#[derive(Clone)]
struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

pub enum FolderAction {
    Open(PathBuf),
    FindInFolder(PathBuf),
    OpenTerminal(PathBuf),
    Close,
}

#[derive(Default)]
pub struct FolderPanel {
    root: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    children: HashMap<PathBuf, (Vec<Entry>, bool)>,
    last_refresh: f64,
    show_hidden: bool,
    /// The current document last shown in the tree.
    revealed: Option<PathBuf>,
    /// Scroll the current document into view on the next draw.
    scroll_to_active: bool,
}

fn read_folder(dir: &Path, show_hidden: bool) -> (Vec<Entry>, bool) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return (Vec::new(), false);
    };
    let mut entries: Vec<Entry> = Vec::new();
    let mut more = false;
    for e in read.flatten() {
        if entries.len() >= MAX_ENTRIES {
            more = true;
            break;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let path = e.path();
        // Follows symlinks, so a link to a folder opens like a folder.
        let is_dir = path.is_dir();
        entries.push(Entry { name, path, is_dir });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    (entries, more)
}

impl FolderPanel {
    pub fn set_root(&mut self, dir: &Path) {
        *self = Self {
            show_hidden: self.show_hidden,
            ..Default::default()
        };
        self.root = Some(dir.to_path_buf());
        self.expanded.insert(dir.to_path_buf());
    }

    fn entries(&mut self, dir: &Path) -> (Vec<Entry>, bool) {
        let show_hidden = self.show_hidden;
        self.children
            .entry(dir.to_path_buf())
            .or_insert_with(|| read_folder(dir, show_hidden))
            .clone()
    }

    /// Reads the open folders again.
    fn refresh(&mut self) {
        let show_hidden = self.show_hidden;
        self.children.retain(|dir, _| self.expanded.contains(dir));
        for (dir, list) in self.children.iter_mut() {
            *list = read_folder(dir, show_hidden);
        }
    }

    /// Draws the panel. `active` is the current document's file, which is highlighted.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        active: Option<&Path>,
        palette: &Palette,
    ) -> Option<FolderAction> {
        let Some(root) = self.root.clone() else {
            ui.label(RichText::new("No folder is open.").weak());
            return None;
        };
        // Opens the folders down to the current document when it changes.
        if let Some(file) = active
            && self.revealed.as_deref() != Some(file)
        {
            self.revealed = Some(file.to_path_buf());
            if file.starts_with(&root) {
                for dir in file.ancestors().skip(1) {
                    self.expanded.insert(dir.to_path_buf());
                    if dir == root {
                        break;
                    }
                }
                self.scroll_to_active = true;
            }
        }
        let now = ui.input(|i| i.time);
        if now - self.last_refresh > REFRESH_EVERY {
            self.last_refresh = now;
            self.refresh();
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(REFRESH_EVERY));
        let mut action = None;
        ui.horizontal(|ui| {
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| root.display().to_string());
            ui.label(RichText::new(name).strong())
                .on_hover_text(root.display().to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("×")
                    .on_hover_text("Close the folder panel")
                    .clicked()
                {
                    action = Some(FolderAction::Close);
                }
                if ui
                    .small_button("⟳")
                    .on_hover_text("Read the folder again")
                    .clicked()
                {
                    self.refresh();
                }
                if ui
                    .small_button("⊟")
                    .on_hover_text("Collapse all folders")
                    .clicked()
                {
                    self.expanded.clear();
                    self.expanded.insert(root.clone());
                }
                if ui
                    .selectable_label(self.show_hidden, RichText::new(".*").monospace())
                    .on_hover_text("Show hidden files")
                    .clicked()
                {
                    self.show_hidden = !self.show_hidden;
                    self.children.clear();
                }
            });
        });
        ui.separator();
        ScrollArea::both()
            .id_salt("folder-tree")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                self.folder(ui, &root, 0, active, palette, &mut action);
            });
        action
    }

    fn folder(
        &mut self,
        ui: &mut Ui,
        dir: &Path,
        depth: usize,
        active: Option<&Path>,
        palette: &Palette,
        action: &mut Option<FolderAction>,
    ) {
        let (entries, more) = self.entries(dir);
        for entry in entries {
            let indent = depth as f32 * 14.0;
            let open = entry.is_dir && self.expanded.contains(&entry.path);
            let is_active = active == Some(entry.path.as_path());
            let response = ui
                .horizontal(|ui| {
                    ui.add_space(indent);
                    let icon = match (entry.is_dir, open) {
                        (true, true) => "⏷",
                        (true, false) => "⏵",
                        _ => " ",
                    };
                    ui.label(RichText::new(icon).small().color(palette.text_dim));
                    let mut text = RichText::new(&entry.name);
                    if entry.is_dir {
                        text = text.color(palette.text);
                    } else if is_active {
                        text = text.color(palette.accent);
                    } else {
                        text = text.color(palette.text_dim.gamma_multiply(1.15));
                    }
                    ui.add(
                        egui::Button::selectable(is_active, text)
                            .frame_when_inactive(false)
                            .truncate(),
                    )
                })
                .inner;
            if is_active && std::mem::take(&mut self.scroll_to_active) {
                response.scroll_to_me(Some(egui::Align::Center));
            }
            if response.clicked() {
                if entry.is_dir {
                    if open {
                        self.expanded.remove(&entry.path);
                    } else {
                        self.expanded.insert(entry.path.clone());
                    }
                } else {
                    *action = Some(FolderAction::Open(entry.path.clone()));
                }
            }
            response.context_menu(|ui| {
                if !entry.is_dir && ui.button("Open").clicked() {
                    *action = Some(FolderAction::Open(entry.path.clone()));
                }
                let folder = if entry.is_dir {
                    entry.path.clone()
                } else {
                    dir.to_path_buf()
                };
                if ui.button("Find in Files Here…").clicked() {
                    *action = Some(FolderAction::FindInFolder(folder.clone()));
                }
                if ui.button("Open Terminal Here").clicked() {
                    *action = Some(FolderAction::OpenTerminal(folder));
                }
                if ui.button("Copy Path").clicked() {
                    ui.ctx().copy_text(entry.path.display().to_string());
                }
            });
            if open {
                self.folder(ui, &entry.path, depth + 1, active, palette, action);
            }
        }
        if more {
            ui.horizontal(|ui| {
                ui.add_space(depth as f32 * 14.0 + 18.0);
                ui.label(
                    RichText::new(format!("Only the first {MAX_ENTRIES} entries are shown"))
                        .small()
                        .weak(),
                );
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_come_first_and_hidden_files_are_skipped() {
        let dir = std::env::temp_dir().join(format!("ettin-folder-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("b.txt"), "").unwrap();
        std::fs::write(dir.join("A.txt"), "").unwrap();
        std::fs::write(dir.join(".hidden"), "").unwrap();
        let (entries, more) = read_folder(&dir, false);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["sub", "A.txt", "b.txt"]);
        assert!(!more);
        let (entries, _) = read_folder(&dir, true);
        assert_eq!(entries.len(), 4);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
