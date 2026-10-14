//! Editor fonts: the font picked in Preferences, plus fallback fonts for
//! scripts the bundled fonts don't cover (Chinese, Japanese, Korean).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};
use skrifa::raw::{FileRef, TableProvider};
use skrifa::{FontRef, MetadataProvider, attribute::Style, string::StringId};

#[cfg(windows)]
const CANDIDATES: &[&str] = &[
    r"C:\Windows\Fonts\msyh.ttc",
    r"C:\Windows\Fonts\YuGothM.ttc",
    r"C:\Windows\Fonts\meiryo.ttc",
    r"C:\Windows\Fonts\malgun.ttf",
    r"C:\Windows\Fonts\simsun.ttc",
];

#[cfg(not(windows))]
const CANDIDATES: &[&str] = &[
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansMonoCJK-Regular.ttc",
    "/usr/share/fonts/adobe-source-han-sans/SourceHanSans.ttc",
    "/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
];

/// The editor font chosen in Preferences.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EditorFont {
    /// Shown in Preferences; empty means the built-in font.
    pub family: String,
    pub path: Option<PathBuf>,
    /// The font's index inside a `.ttc` collection.
    pub index: u32,
}

/// Sets up the fonts: the chosen editor font first (when there is one), then
/// egui's own, then a CJK fallback found on the system. Returns an error
/// message when the chosen font can't be read.
pub fn install(ctx: &egui::Context, editor: &EditorFont) -> Result<(), String> {
    let mut fonts = FontDefinitions::default();
    let mut result = Ok(());
    if let Some(path) = &editor.path {
        match std::fs::read(path) {
            Ok(bytes) if FontRef::from_index(&bytes, editor.index).is_ok() => {
                let mut data = FontData::from_owned(bytes);
                data.index = editor.index;
                fonts
                    .font_data
                    .insert("editor-font".to_owned(), Arc::new(data));
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .insert(0, "editor-font".to_owned());
            }
            Ok(_) => {
                result = Err(format!(
                    "{} isn't a font Ettin Edit can use.",
                    path.display()
                ))
            }
            Err(e) => {
                result = Err(format!(
                    "Couldn't read the editor font {}: {e}",
                    path.display()
                ))
            }
        }
    }
    if let Some(bytes) = CANDIDATES.iter().find_map(|p| std::fs::read(p).ok()) {
        fonts.font_data.insert(
            "cjk-fallback".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("cjk-fallback".to_owned());
        }
    }
    ctx.set_fonts(fonts);
    result
}

/// A regular-style font installed on the system.
#[derive(Clone, Debug)]
pub struct SystemFont {
    pub family: String,
    pub path: PathBuf,
    pub index: u32,
    /// Every character is the same width.
    pub monospace: bool,
}

impl SystemFont {
    pub fn to_editor_font(&self) -> EditorFont {
        EditorFont {
            family: self.family.clone(),
            path: Some(self.path.clone()),
            index: self.index,
        }
    }
}

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
        dirs.push(PathBuf::from(windir).join("Fonts"));
        if let Some(local) = dirs::data_local_dir() {
            dirs.push(local.join("Microsoft").join("Windows").join("Fonts"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(data) = dirs::data_dir() {
            dirs.push(data.join("fonts"));
        }
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".fonts"));
        }
    }
    dirs
}

fn collect_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < 6 {
                collect_files(&path, depth + 1, out);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
        {
            out.push(path);
        }
    }
}

fn family_name(font: &FontRef) -> Option<String> {
    [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]
        .into_iter()
        .find_map(|id| font.localized_strings(id).english_or_first())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
}

/// The font says it's fixed pitch, or a narrow and a wide letter have the same advance.
fn is_monospace(font: &FontRef) -> bool {
    if font.post().is_ok_and(|p| p.is_fixed_pitch() != 0) {
        return true;
    }
    let charmap = font.charmap();
    let metrics = font.glyph_metrics(
        skrifa::instance::Size::unscaled(),
        skrifa::instance::LocationRef::default(),
    );
    let width = |c: char| charmap.map(c).and_then(|g| metrics.advance_width(g));
    match (width('i'), width('M'), width('.')) {
        (Some(a), Some(b), Some(c)) => a > 0.0 && a == b && b == c,
        _ => false,
    }
}

/// Upright, normal-weight faces in a file (a `.ttc` can hold several).
fn regular_faces(path: &Path) -> Vec<SystemFont> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let fonts: Vec<(u32, FontRef)> = match FileRef::new(&bytes) {
        Ok(FileRef::Font(f)) => vec![(0, f)],
        Ok(FileRef::Collection(c)) => (0..c.len())
            .filter_map(|i| c.get(i).ok().map(|f| (i, f)))
            .collect(),
        Err(_) => Vec::new(),
    };
    fonts
        .into_iter()
        // Colour bitmap fonts (emoji) have no outlines egui can draw.
        .filter(|(_, f)| f.glyf().is_ok() || f.cff().is_ok() || f.cff2().is_ok())
        .filter(|(_, f)| {
            let a = f.attributes();
            a.style == Style::Normal && (350.0..=450.0).contains(&a.weight.value())
        })
        .filter_map(|(index, f)| {
            Some(SystemFont {
                family: family_name(&f)?,
                path: path.to_path_buf(),
                index,
                monospace: is_monospace(&f),
            })
        })
        .collect()
}

/// Fonts installed on the system, one per family, sorted by name. Reads every
/// font file, so call it off the UI thread.
pub fn system_fonts() -> Vec<SystemFont> {
    let mut files = Vec::new();
    for dir in font_dirs() {
        collect_files(&dir, 0, &mut files);
    }
    let mut by_family: BTreeMap<String, SystemFont> = BTreeMap::new();
    for path in files {
        for face in regular_faces(&path) {
            // Prefer a plain .ttf/.otf over a face inside a collection.
            let key = face.family.to_lowercase();
            match by_family.get(&key) {
                Some(existing) if existing.index == 0 || face.index != 0 => {}
                _ => {
                    by_family.insert(key, face);
                }
            }
        }
    }
    by_family.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_system_fonts_without_panicking() {
        let fonts = system_fonts();
        for f in &fonts {
            assert!(!f.family.is_empty());
        }
        // DejaVu is on most Linux systems; check monospace detection with it.
        if let Some(f) = fonts.iter().find(|f| f.family == "DejaVu Sans Mono") {
            assert!(f.monospace);
        }
        if let Some(f) = fonts.iter().find(|f| f.family == "DejaVu Sans") {
            assert!(!f.monospace);
        }
    }
}
