//! Themes: a syntax colour scheme plus the editor chrome colours derived from it.
//!
//! Built-in themes ship with NixPad; extra `.tmTheme` files dropped into the
//! user's `themes` folder are picked up at start-up.

use std::io::Cursor;
use std::path::Path;

use egui::{Color32, CornerRadius, Stroke, Visuals};
use syntect::highlighting::{Color as SyntectColor, Theme as SyntectTheme, ThemeSet};

pub struct ThemeDef {
    pub name: String,
    pub syntax: SyntectTheme,
    pub palette: Palette,
    /// True for themes loaded from the user's themes folder.
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
    fn derive(theme: &SyntectTheme, accent: Option<Color32>) -> Self {
        let s = &theme.settings;
        let editor_bg = s.background.map(to_egui).unwrap_or(Color32::from_rgb(30, 30, 30));
        let editor_fg = s.foreground.map(to_egui).unwrap_or(Color32::from_rgb(220, 220, 220));
        let dark = luminance(editor_bg) < 0.5;
        let (toward, away) = if dark { (Color32::BLACK, Color32::WHITE) } else { (Color32::from_rgb(214, 218, 224), Color32::BLACK) };

        let accent = accent
            .or(s.caret.map(to_egui))
            .unwrap_or(if dark { Color32::from_rgb(122, 162, 247) } else { Color32::from_rgb(26, 98, 199) });
        let selection = s
            .selection
            .map(to_egui)
            .map(|c| Color32::from_rgb(c.r(), c.g(), c.b()))
            .unwrap_or_else(|| mix(editor_bg, accent, 0.3));
        let line_highlight = s
            .line_highlight
            .map(to_egui)
            .map(|c| mix(editor_bg, Color32::from_rgb(c.r(), c.g(), c.b()), c.a() as f32 / 255.0))
            .unwrap_or_else(|| mix(editor_bg, away, 0.04));

        Self {
            dark,
            editor_bg,
            editor_fg,
            gutter_fg: mix(editor_bg, editor_fg, 0.35),
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
            modified: if dark { Color32::from_rgb(240, 163, 94) } else { Color32::from_rgb(196, 98, 0) },
        }
    }

    pub fn visuals(&self) -> Visuals {
        let mut v = if self.dark { Visuals::dark() } else { Visuals::light() };
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
        w.inactive.bg_stroke = Stroke::NONE;
        w.hovered.weak_bg_fill = mix(self.chrome_bg, self.accent, 0.18);
        w.hovered.bg_fill = mix(self.chrome_bg, self.accent, 0.18);
        w.hovered.bg_stroke = Stroke::NONE;
        w.hovered.fg_stroke = Stroke::new(1.0, self.text);
        w.active.weak_bg_fill = mix(self.chrome_bg, self.accent, 0.3);
        w.active.bg_fill = mix(self.chrome_bg, self.accent, 0.3);
        w.active.fg_stroke = Stroke::new(1.0, self.text);
        w.open.weak_bg_fill = mix(self.chrome_bg, self.accent, 0.18);
        for state in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            state.corner_radius = CornerRadius::same(5);
        }
        v
    }
}

fn load_embedded(source: &str) -> SyntectTheme {
    ThemeSet::load_from_reader(&mut Cursor::new(source)).expect("built-in theme is valid")
}

/// All themes: built-ins first, then any found in `user_dir`.
pub fn load_themes(user_dir: Option<&Path>) -> Vec<ThemeDef> {
    let mut defaults = ThemeSet::load_defaults().themes;
    let mut themes = Vec::new();
    let mut add = |name: &str, syntax: SyntectTheme, accent: Option<Color32>| {
        let palette = Palette::derive(&syntax, accent);
        themes.push(ThemeDef { name: name.to_owned(), syntax, palette, custom: false });
    };

    add("NixPad Dark", load_embedded(include_str!("../assets/themes/nixpad-dark.tmTheme")), None);
    add("NixPad Light", load_embedded(include_str!("../assets/themes/nixpad-light.tmTheme")), None);
    let bundled = [
        ("Ocean Dark", "base16-ocean.dark", Color32::from_rgb(143, 161, 179)),
        ("Ocean Light", "base16-ocean.light", Color32::from_rgb(79, 91, 102)),
        ("Eighties", "base16-eighties.dark", Color32::from_rgb(102, 153, 204)),
        ("Mocha", "base16-mocha.dark", Color32::from_rgb(138, 166, 162)),
        ("Solarized Dark", "Solarized (dark)", Color32::from_rgb(38, 139, 210)),
        ("Solarized Light", "Solarized (light)", Color32::from_rgb(38, 139, 210)),
        ("GitHub Light", "InspiredGitHub", Color32::from_rgb(9, 105, 218)),
    ];
    for (name, key, accent) in bundled {
        if let Some(t) = defaults.remove(key) {
            add(name, t, Some(accent));
        }
    }

    if let Some(dir) = user_dir
        && let Ok(entries) = std::fs::read_dir(dir) {
            let mut paths: Vec<_> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("tmTheme")))
                .collect();
            paths.sort();
            for path in paths {
                match ThemeSet::get_theme(&path) {
                    Ok(syntax) => {
                        let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                        let palette = Palette::derive(&syntax, None);
                        themes.push(ThemeDef { name, syntax, palette, custom: true });
                    }
                    Err(e) => eprintln!("nixpad: skipping theme {}: {e}", path.display()),
                }
            }
        }
    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_load_and_classify() {
        let themes = load_themes(None);
        assert!(themes.len() >= 9);
        let dark = themes.iter().find(|t| t.name == "NixPad Dark").unwrap();
        let light = themes.iter().find(|t| t.name == "NixPad Light").unwrap();
        assert!(dark.palette.dark);
        assert!(!light.palette.dark);
    }
}
