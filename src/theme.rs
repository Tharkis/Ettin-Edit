//! Themes: a syntax colour scheme plus the editor chrome colours derived from it.
//!
//! The built-in themes are written to the user's `themes` folder, where they
//! can be edited; extra `.tmTheme` files dropped there are picked up too.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use egui::{Color32, CornerRadius, Stroke, Visuals};
use syntect::highlighting::{Color as SyntectColor, Theme as SyntectTheme, ThemeSet};

pub struct ThemeDef {
    pub name: String,
    /// Changes each time themes are loaded, so highlighting caches made with
    /// an older copy of the theme are dropped.
    pub key: String,
    pub syntax: SyntectTheme,
    pub palette: Palette,
    /// True for themes the user added to the themes folder.
    pub custom: bool,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub editor_bg: Color32,
    pub editor_fg: Color32,
    pub gutter_fg: Color32,
    pub gutter_active_fg: Color32,
    pub line_highlight: Color32,
    pub selection: Color32,
    pub chrome_bg: Color32,
    pub tab_bg: Color32,
    pub tab_hover_bg: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub modified: Color32,
    /// Matches of the find bar's search.
    pub find_hit: Color32,
    /// Text marked with Mark All.
    pub mark: Color32,
    /// Other occurrences of the selected word.
    pub word_hit: Color32,
    /// Background of a bracket and its partner.
    pub brace_match: Color32,
    /// An unmatched bracket.
    pub brace_bad: Color32,
    /// Indent guides, the edge line and fold lines.
    pub guide: Color32,
    /// Whitespace and end-of-line symbols.
    pub whitespace: Color32,
}

fn to_egui(c: SyntectColor) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn luminance(c: Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

impl Palette {
    /// Derives chrome colours from a syntax theme so any `.tmTheme` gives a coherent window.
    fn derive(theme: &SyntectTheme, accent: Option<Color32>, invisibles: Option<Color32>) -> Self {
        let s = &theme.settings;
        let editor_bg = s
            .background
            .map(to_egui)
            .unwrap_or(Color32::from_rgb(30, 30, 30));
        let editor_fg = s
            .foreground
            .map(to_egui)
            .unwrap_or(Color32::from_rgb(220, 220, 220));
        let dark = luminance(editor_bg) < 0.5;
        let (toward, away) = if dark {
            (Color32::BLACK, Color32::WHITE)
        } else {
            (Color32::from_rgb(214, 218, 224), Color32::BLACK)
        };

        let accent = accent.or(s.caret.map(to_egui)).unwrap_or(if dark {
            Color32::from_rgb(122, 162, 247)
        } else {
            Color32::from_rgb(26, 98, 199)
        });
        let selection = s
            .selection
            .map(to_egui)
            .map(|c| Color32::from_rgb(c.r(), c.g(), c.b()))
            .unwrap_or_else(|| mix(editor_bg, accent, 0.3));
        let line_highlight = s
            .line_highlight
            .map(to_egui)
            .map(|c| {
                mix(
                    editor_bg,
                    Color32::from_rgb(c.r(), c.g(), c.b()),
                    c.a() as f32 / 255.0,
                )
            })
            .unwrap_or_else(|| mix(editor_bg, away, 0.04));

        // Colours a theme may set; otherwise mixed from the background and text.
        let solid = |c: SyntectColor| {
            let c = to_egui(c);
            mix(
                editor_bg,
                Color32::from_rgb(c.r(), c.g(), c.b()),
                c.a() as f32 / 255.0,
            )
        };
        Self {
            dark,
            editor_bg,
            editor_fg,
            gutter_fg: s
                .gutter_foreground
                .map(solid)
                .unwrap_or_else(|| mix(editor_bg, editor_fg, 0.35)),
            gutter_active_fg: mix(editor_bg, editor_fg, 0.8),
            line_highlight,
            selection,
            chrome_bg: mix(editor_bg, toward, if dark { 0.18 } else { 0.35 }),
            tab_bg: mix(editor_bg, toward, if dark { 0.1 } else { 0.18 }),
            tab_hover_bg: mix(editor_bg, toward, if dark { 0.04 } else { 0.08 }),
            border: mix(editor_bg, away, if dark { 0.1 } else { 0.12 }),
            text: editor_fg,
            text_dim: mix(editor_bg, editor_fg, 0.6),
            accent,
            modified: if dark {
                Color32::from_rgb(240, 163, 94)
            } else {
                Color32::from_rgb(196, 98, 0)
            },
            // Search matches are drawn over the text's background, so keep them see-through.
            find_hit: s
                .find_highlight
                .map(to_egui)
                .map(|c| {
                    if c.a() == 255 {
                        c.gamma_multiply(0.45)
                    } else {
                        c
                    }
                })
                .unwrap_or(if dark {
                    Color32::from_rgba_unmultiplied(255, 196, 0, 72)
                } else {
                    Color32::from_rgba_unmultiplied(255, 170, 0, 96)
                }),
            mark: if dark {
                Color32::from_rgba_unmultiplied(64, 210, 160, 80)
            } else {
                Color32::from_rgba_unmultiplied(0, 170, 120, 72)
            },
            word_hit: Color32::from_rgba_unmultiplied(
                accent.r(),
                accent.g(),
                accent.b(),
                if dark { 52 } else { 44 },
            ),
            brace_match: Color32::from_rgba_unmultiplied(
                accent.r(),
                accent.g(),
                accent.b(),
                if dark { 90 } else { 70 },
            ),
            brace_bad: if dark {
                Color32::from_rgba_unmultiplied(240, 80, 80, 110)
            } else {
                Color32::from_rgba_unmultiplied(220, 40, 40, 80)
            },
            guide: s
                .guide
                .map(solid)
                .unwrap_or_else(|| mix(editor_bg, editor_fg, if dark { 0.22 } else { 0.25 })),
            whitespace: invisibles
                .unwrap_or_else(|| mix(editor_bg, editor_fg, if dark { 0.42 } else { 0.4 })),
        }
    }

    pub fn visuals(&self) -> Visuals {
        let mut v = if self.dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        v.panel_fill = self.chrome_bg;
        v.window_fill = self.chrome_bg;
        v.faint_bg_color = self.tab_bg;
        v.extreme_bg_color = self.editor_bg;
        v.code_bg_color = self.editor_bg;
        v.window_stroke = Stroke::new(1.0, self.border);
        v.window_corner_radius = CornerRadius::same(8);
        v.menu_corner_radius = CornerRadius::same(6);
        v.selection.bg_fill = self.selection;
        v.selection.stroke = Stroke::new(1.0, self.text);
        v.hyperlink_color = self.accent;
        v.text_cursor.stroke = Stroke::new(2.0, self.accent);
        v.override_text_color = Some(self.text);

        let w = &mut v.widgets;
        w.noninteractive.bg_fill = self.chrome_bg;
        w.noninteractive.weak_bg_fill = self.chrome_bg;
        w.noninteractive.bg_stroke = Stroke::new(1.0, self.border);
        w.noninteractive.fg_stroke = Stroke::new(1.0, self.text);
        w.inactive.weak_bg_fill = Color32::TRANSPARENT;
        w.inactive.bg_fill = self.tab_bg;
        w.inactive.fg_stroke = Stroke::new(1.0, self.text);
        // An outline keeps unticked checkboxes and radio buttons visible
        // against dialog backgrounds; menus drop it in their own style.
        w.inactive.bg_stroke = Stroke::new(1.0, mix(self.chrome_bg, self.text, 0.3));
        w.hovered.weak_bg_fill = mix(self.chrome_bg, self.accent, 0.18);
        w.hovered.bg_fill = mix(self.chrome_bg, self.accent, 0.18);
        w.hovered.bg_stroke = Stroke::NONE;
        w.hovered.fg_stroke = Stroke::new(1.0, self.text);
        w.active.weak_bg_fill = mix(self.chrome_bg, self.accent, 0.3);
        w.active.bg_fill = mix(self.chrome_bg, self.accent, 0.3);
        w.active.fg_stroke = Stroke::new(1.0, self.text);
        w.open.weak_bg_fill = mix(self.chrome_bg, self.accent, 0.18);
        for state in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            state.corner_radius = CornerRadius::same(5);
        }
        v
    }
}

fn load_embedded(source: &str) -> SyntectTheme {
    ThemeSet::load_from_reader(&mut Cursor::new(source)).expect("built-in theme is valid")
}

/// The built-in themes, in menu order. Each is also written to the user's
/// themes folder as `<name>.tmTheme`, where it can be edited.
pub const BUILTIN: &[(&str, &str)] = &[
    (
        "Ettin Dark",
        include_str!("../assets/themes/Ettin Dark.tmTheme"),
    ),
    (
        "Ettin Night",
        include_str!("../assets/themes/Ettin Night.tmTheme"),
    ),
    (
        "Ocean Dark",
        include_str!("../assets/themes/Ocean Dark.tmTheme"),
    ),
    ("Mocha", include_str!("../assets/themes/Mocha.tmTheme")),
    (
        "Solarized Dark",
        include_str!("../assets/themes/Solarized Dark.tmTheme"),
    ),
    (
        "Ettin Light",
        include_str!("../assets/themes/Ettin Light.tmTheme"),
    ),
    (
        "Ocean Light",
        include_str!("../assets/themes/Ocean Light.tmTheme"),
    ),
    (
        "Solarized Light",
        include_str!("../assets/themes/Solarized Light.tmTheme"),
    ),
    (
        "GitHub Light",
        include_str!("../assets/themes/GitHub Light.tmTheme"),
    ),
];

/// Remembers which version of each built-in theme was written to the themes
/// folder, so a copy the user never edited is updated with Ettin Edit.
const WRITTEN_FILE: &str = ".builtin-themes.json";

fn file_name(name: &str) -> String {
    format!("{name}.tmTheme")
}

/// 64-bit FNV-1a, as hex: stable across Rust versions.
fn hash(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in text.as_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Writes the built-in themes into `dir`. A missing file is written; a file
/// still exactly as Ettin Edit wrote it is updated to this version; a file
/// the user changed is kept. With `restore`, changed files are first copied
/// to `<name>.tmTheme.bak` and then replaced too.
/// Where a theme's file is in the themes folder.
pub fn theme_path(dir: &Path, name: &str) -> std::path::PathBuf {
    dir.join(file_name(name))
}

pub fn install_builtins(dir: &Path, restore: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let record_path = dir.join(WRITTEN_FILE);
    let mut written: BTreeMap<String, String> = std::fs::read_to_string(&record_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let before = written.clone();
    for (name, source) in BUILTIN {
        let file = file_name(name);
        let path = dir.join(&file);
        let ours = hash(source);
        match std::fs::read_to_string(&path) {
            Err(_) => std::fs::write(&path, source)?,
            Ok(current) => {
                let theirs = hash(&current);
                if theirs == ours {
                    // Already this version.
                } else if written.get(&file) == Some(&theirs) {
                    std::fs::write(&path, source)?;
                } else if restore {
                    std::fs::write(dir.join(format!("{file}.bak")), current)?;
                    std::fs::write(&path, source)?;
                } else {
                    continue;
                }
            }
        }
        written.insert(file, ours);
    }
    if written != before {
        let json = serde_json::to_string_pretty(&written).unwrap_or_default();
        std::fs::write(record_path, json)?;
    }
    Ok(())
}

/// The `invisibles` colour (whitespace symbols), which syntect doesn't read.
fn invisibles(source: &str, background: Option<SyntectColor>) -> Option<Color32> {
    let value = plist::Value::from_reader_xml(Cursor::new(source.as_bytes())).ok()?;
    let text = value
        .as_dictionary()?
        .get("settings")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_dictionary())
        .find(|d| !d.contains_key("scope"))?
        .get("settings")?
        .as_dictionary()?
        .get("invisibles")?
        .as_string()?;
    let c = parse_hex(text)?;
    let bg = background.map(to_egui).unwrap_or(Color32::BLACK);
    Some(mix(
        bg,
        Color32::from_rgb(c.r(), c.g(), c.b()),
        c.a() as f32 / 255.0,
    ))
}

/// `#RGB`, `#RRGGBB` or `#RRGGBBAA`.
pub fn parse_hex(text: &str) -> Option<Color32> {
    let hex = text.trim().strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    match hex.len() {
        3 => {
            let d = |i: usize| {
                u8::from_str_radix(hex.get(i..i + 1)?, 16)
                    .ok()
                    .map(|v| v * 17)
            };
            Some(Color32::from_rgb(d(0)?, d(1)?, d(2)?))
        }
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            byte(6)?,
        )),
        _ => None,
    }
}

/// Writes a colour as `#RRGGBB`, or `#RRGGBBAA` when it isn't opaque.
pub fn to_hex(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

/// Builds a theme from `.tmTheme` text, for previewing edits.
pub fn from_source(name: &str, source: &str, custom: bool) -> Result<ThemeDef, String> {
    let syntax = ThemeSet::load_from_reader(&mut Cursor::new(source)).map_err(|e| e.to_string())?;
    Ok(theme_def(name.to_owned(), syntax, custom, Some(source)))
}

fn theme_def(name: String, syntax: SyntectTheme, custom: bool, source: Option<&str>) -> ThemeDef {
    let accent = syntax.settings.accent.map(to_egui);
    let invisibles = source.and_then(|s| invisibles(s, syntax.settings.background));
    let palette = Palette::derive(&syntax, accent, invisibles);
    static LOADS: AtomicU64 = AtomicU64::new(0);
    let key = format!("{name}#{}", LOADS.fetch_add(1, Ordering::Relaxed));
    ThemeDef {
        name,
        key,
        syntax,
        palette,
        custom,
    }
}

/// All themes, built-ins first, read from `user_dir` (the themes folder)
/// so edits there take effect. A built-in theme whose file is missing or
/// broken falls back to the copy inside Ettin Edit. Returns the themes and
/// any problems to show the user.
pub fn load_themes(user_dir: Option<&Path>) -> (Vec<ThemeDef>, Vec<String>) {
    let mut errors = Vec::new();
    if let Some(dir) = user_dir
        && let Err(e) = install_builtins(dir, false)
    {
        errors.push(format!(
            "Couldn't write the built-in themes to {}: {e}",
            dir.display()
        ));
    }
    let mut themes = Vec::new();
    for (name, source) in BUILTIN {
        let from_file = user_dir.map(|d| d.join(file_name(name))).and_then(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            ThemeSet::load_from_reader(&mut Cursor::new(&text))
                .map(|t| (t, text))
                .map_err(|e| {
                    errors.push(format!(
                        "The theme file {} has a problem, so the original {name} theme is used: {e}",
                        path.display()
                    ))
                })
                .ok()
        });
        let (syntax, text) =
            from_file.unwrap_or_else(|| (load_embedded(source), (*source).to_owned()));
        themes.push(theme_def((*name).to_owned(), syntax, false, Some(&text)));
    }

    if let Some(dir) = user_dir
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("tmTheme"))
            })
            .collect();
        paths.sort();
        for path in paths {
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if BUILTIN.iter().any(|(b, _)| *b == name) {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            match ThemeSet::load_from_reader(&mut Cursor::new(&text)) {
                Ok(syntax) => themes.push(theme_def(name, syntax, true, Some(&text))),
                Err(e) => errors.push(format!("Skipped the theme {}: {e}", path.display())),
            }
        }
    }
    (themes, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_load_and_classify() {
        let (themes, errors) = load_themes(None);
        assert!(errors.is_empty());
        assert_eq!(themes.len(), BUILTIN.len());
        assert_eq!(themes[0].name, "Ettin Dark");
        let dark = themes.iter().find(|t| t.name == "Ettin Dark").unwrap();
        let light = themes.iter().find(|t| t.name == "Ettin Light").unwrap();
        assert!(dark.palette.dark);
        assert!(!light.palette.dark);
    }

    #[test]
    fn builtins_go_to_the_themes_folder() {
        let dir = std::env::temp_dir().join(format!("ettin-edit-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        install_builtins(&dir, false).unwrap();
        let dark = dir.join("Ettin Dark.tmTheme");
        assert_eq!(std::fs::read_to_string(&dark).unwrap(), BUILTIN[0].1);

        // An edit in the folder is used and kept.
        let edited = BUILTIN[0].1.replace("#2D2D2D", "#101010");
        std::fs::write(&dark, &edited).unwrap();
        install_builtins(&dir, false).unwrap();
        assert_eq!(std::fs::read_to_string(&dark).unwrap(), edited);
        let (themes, errors) = load_themes(Some(&dir));
        assert!(errors.is_empty());
        assert_eq!(
            themes[0].palette.editor_bg,
            Color32::from_rgb(0x10, 0x10, 0x10)
        );
        assert!(!themes[0].custom);

        // A copy written by an older version and never edited is updated.
        let old = BUILTIN[1].1.replace("Ettin Night", "Old Night");
        let night = dir.join("Ettin Night.tmTheme");
        std::fs::write(&night, &old).unwrap();
        let record = dir.join(WRITTEN_FILE);
        let mut written: BTreeMap<String, String> =
            serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
        written.insert("Ettin Night.tmTheme".into(), hash(&old));
        std::fs::write(&record, serde_json::to_string(&written).unwrap()).unwrap();
        install_builtins(&dir, false).unwrap();
        assert_eq!(std::fs::read_to_string(&night).unwrap(), BUILTIN[1].1);

        // Restoring puts the original back and keeps the edit as .bak.
        install_builtins(&dir, true).unwrap();
        assert_eq!(std::fs::read_to_string(&dark).unwrap(), BUILTIN[0].1);
        let bak = dir.join("Ettin Dark.tmTheme.bak");
        assert_eq!(std::fs::read_to_string(bak).unwrap(), edited);

        // A broken built-in file falls back to the original, with a message.
        std::fs::write(&dark, "not a theme").unwrap();
        let (themes, errors) = load_themes(Some(&dir));
        assert_eq!(errors.len(), 1);
        assert_eq!(themes[0].name, "Ettin Dark");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
