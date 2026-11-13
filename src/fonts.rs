//! Editor fonts: the font picked in Preferences, plus fallback fonts found on
//! the system for scripts the bundled fonts don't cover (Chinese, Japanese,
//! Korean, Bangla, Devanagari, Tamil, Thai and the rest).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

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
            Ok(_) => result = Err(t!("font-unusable", path = path.display().to_string())),
            Err(e) => {
                result = Err(t!(
                    "font-read-failed",
                    path = path.display().to_string(),
                    error = e.to_string()
                ))
            }
        }
    }
    for (name, data) in script_fallbacks() {
        fonts.font_data.insert(name.clone(), data.clone());
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push(name.clone());
        }
    }
    // Toolbar icons; also a fallback so icons can sit inside ordinary labels.
    egui_phosphor::add_font_bytes_to_fonts(
        &mut fonts,
        ICON_FONT,
        egui_phosphor::Variant::Regular.font_bytes(),
    );
    ctx.set_fonts(fonts);
    result
}

/// A writing system the bundled fonts don't cover, with a letter to test a
/// font with and the font files that usually cover it.
struct Script {
    /// Also the fallback's name in egui's font list.
    name: &'static str,
    /// A letter of the script; a font that has it is taken to cover the script.
    probe: char,
    /// fontconfig language code, used when none of `files` is installed.
    #[cfg_attr(windows, allow(dead_code))]
    lang: &'static str,
    /// Start of the file name (lower case, without extension) of fonts that
    /// cover the script, best first. Noto and Lohit on Linux, the Windows
    /// fonts on Windows; only files that exist and have `probe` are used.
    files: &'static [&'static str],
}

/// Right-to-left scripts (Arabic, Hebrew) are left out: egui lays text out
/// left to right only, so their letters would show in reverse order.
const SCRIPTS: &[Script] = &[
    Script {
        name: "bengali",
        probe: 'অ',
        lang: "bn",
        files: &[
            "notosansbengali",
            "notoserifbengali",
            "lohit-bengali",
            "lohit-assamese",
            "nirmala",
            "vrinda",
            "freeserif",
            "freesans",
        ],
    },
    Script {
        name: "devanagari",
        probe: 'अ',
        lang: "hi",
        files: &[
            "notosansdevanagari",
            "notoserifdevanagari",
            "lohit-devanagari",
            "lohit-hindi",
            "lohit-marathi",
            "nirmala",
            "mangal",
            "freesans",
            "freeserif",
        ],
    },
    Script {
        name: "gurmukhi",
        probe: 'ਅ',
        lang: "pa",
        files: &[
            "notosansgurmukhi",
            "notoserifgurmukhi",
            "lohit-gurmukhi",
            "lohit-punjabi",
            "nirmala",
            "raavi",
            "freeserif",
        ],
    },
    Script {
        name: "gujarati",
        probe: 'અ',
        lang: "gu",
        files: &[
            "notosansgujarati",
            "notoserifgujarati",
            "lohit-gujarati",
            "nirmala",
            "shruti",
            "freeserif",
        ],
    },
    Script {
        name: "oriya",
        probe: 'ଅ',
        lang: "or",
        files: &[
            "notosansoriya",
            "notoserifodia",
            "notoserifor",
            "lohit-odia",
            "lohit-oriya",
            "nirmala",
            "kalinga",
        ],
    },
    Script {
        name: "tamil",
        probe: 'அ',
        lang: "ta",
        files: &[
            "notosanstamil",
            "notoseriftamil",
            "lohit-tamil",
            "nirmala",
            "latha",
            "freeserif",
        ],
    },
    Script {
        name: "telugu",
        probe: 'అ',
        lang: "te",
        files: &[
            "notosanstelugu",
            "notoseriftelugu",
            "lohit-telugu",
            "nirmala",
            "gautami",
        ],
    },
    Script {
        name: "kannada",
        probe: 'ಅ',
        lang: "kn",
        files: &[
            "notosanskannada",
            "notoserifkannada",
            "lohit-kannada",
            "nirmala",
            "tunga",
        ],
    },
    Script {
        name: "malayalam",
        probe: 'അ',
        lang: "ml",
        files: &[
            "notosansmalayalam",
            "notoserifmalayalam",
            "lohit-malayalam",
            "nirmala",
            "kartika",
            "freeserif",
        ],
    },
    Script {
        name: "sinhala",
        probe: 'අ',
        lang: "si",
        files: &[
            "notosanssinhala",
            "notoserifsinhala",
            "nirmala",
            "iskpota",
            "freeserif",
        ],
    },
    Script {
        name: "thai",
        probe: 'ก',
        lang: "th",
        files: &[
            "notosansthai",
            "notoserifthai",
            "garuda",
            "loma",
            "leelawui",
            "leelawadeeui",
            "tahoma",
            "freeserif",
        ],
    },
    Script {
        name: "lao",
        probe: 'ກ',
        lang: "lo",
        files: &[
            "notosanslao",
            "notoseriflao",
            "leelawui",
            "leelawadeeui",
            "laosansui",
        ],
    },
    Script {
        name: "khmer",
        probe: 'ក',
        lang: "km",
        files: &[
            "notosanskhmer",
            "notoserifkhmer",
            "leelawui",
            "leelawadeeui",
            "khmerui",
        ],
    },
    Script {
        name: "myanmar",
        probe: 'က',
        lang: "my",
        files: &["notosansmyanmar", "notoserifmyanmar", "padauk", "mmrtext"],
    },
    Script {
        name: "tibetan",
        probe: 'ཀ',
        lang: "bo",
        files: &[
            "notoseriftibetan",
            "notosanstibetan",
            "himalaya",
            "jomolhari",
        ],
    },
    Script {
        name: "ethiopic",
        probe: 'አ',
        lang: "am",
        files: &[
            "notosansethiopic",
            "notoserifethiopic",
            "ebrima",
            "abyssinica",
            "freeserif",
        ],
    },
    Script {
        name: "armenian",
        probe: 'Ա',
        lang: "hy",
        files: &[
            "notosansarmenian",
            "notoserifarmenian",
            "segoeui",
            "sylfaen",
            "freesans",
            "freeserif",
        ],
    },
    Script {
        name: "georgian",
        probe: 'ა',
        lang: "ka",
        files: &[
            "notosansgeorgian",
            "notoserifgeorgian",
            "segoeui",
            "sylfaen",
            "freesans",
            "freeserif",
        ],
    },
];

/// Fallback fonts for the scripts above, plus the CJK one, read once and
/// kept: they don't change while the app runs, and `install` runs again
/// whenever the editor font changes.
fn script_fallbacks() -> &'static [(String, Arc<FontData>)] {
    static FALLBACKS: OnceLock<Vec<(String, Arc<FontData>)>> = OnceLock::new();
    FALLBACKS.get_or_init(find_fallbacks)
}

fn find_fallbacks() -> Vec<(String, Arc<FontData>)> {
    let mut out = Vec::new();
    if let Some(bytes) = CANDIDATES.iter().find_map(|p| std::fs::read(p).ok()) {
        out.push((
            "cjk-fallback".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        ));
    }
    // Only file names are listed here; a font file is read only when its
    // name matches a script that still needs one.
    let mut files = Vec::new();
    for dir in font_dirs() {
        collect_files(&dir, 0, &mut files);
    }
    let by_stem = file_stems(&files);
    // A file can cover several scripts (Nirmala UI covers all the Indian
    // ones); it is loaded once and checked against the others.
    let mut loaded: Vec<(PathBuf, u32, Arc<FontData>)> = Vec::new();
    for script in SCRIPTS {
        if loaded.iter().any(|(_, _, d)| has_char(d, script.probe)) {
            continue;
        }
        let found = script
            .files
            .iter()
            .flat_map(|prefix| matching_files(&by_stem, prefix))
            .map(|p| (p, 0))
            .chain(std::iter::once(()).flat_map(|()| fontconfig_match(script)))
            .find_map(|(path, index)| {
                let bytes = std::fs::read(&path).ok()?;
                let mut data = FontData::from_owned(bytes);
                data.index = index;
                let data = Arc::new(data);
                has_char(&data, script.probe).then_some((path, index, data))
            });
        if let Some(entry) = found {
            out.push((format!("{}-fallback", script.name), entry.2.clone()));
            loaded.push(entry);
        }
    }
    out
}

/// Font files by lower-case file name without extension.
fn file_stems(files: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut stems: Vec<(String, PathBuf)> = files
        .iter()
        .filter_map(|p| Some((p.file_stem()?.to_str()?.to_lowercase(), p.clone())))
        .collect();
    stems.sort();
    stems
}

/// Files whose name starts with `prefix`, the regular face first, then a
/// variable font, then anything that isn't bold or italic.
fn matching_files(stems: &[(String, PathBuf)], prefix: &str) -> Vec<PathBuf> {
    let rank = |stem: &str| {
        let rest = &stem[prefix.len()..];
        if rest.is_empty() || rest == "-regular" || rest == "ui-regular" {
            0
        } else if rest.starts_with('[') || rest.contains("-vf") || rest.ends_with("vf") {
            1
        } else if rest.contains("bold")
            || rest.contains("italic")
            || rest.contains("oblique")
            || rest.contains("light")
            || rest.contains("thin")
            || rest.contains("black")
            || rest.contains("medium")
        {
            3
        } else {
            2
        }
    };
    let mut hits: Vec<(u8, &PathBuf)> = stems
        .iter()
        .filter(|(stem, _)| stem.starts_with(prefix))
        .map(|(stem, path)| (rank(stem), path))
        .filter(|(r, _)| *r < 3)
        .collect();
    hits.sort_by_key(|(r, _)| *r);
    hits.into_iter().map(|(_, p)| p.clone()).collect()
}

/// Fonts fontconfig says support the script's language, regular faces
/// first, for fonts that aren't in the lists above. Empty when fontconfig
/// isn't installed. `fc-list` runs once, and only if a script needs it.
/// A face from `fc-list`: file, index in the file, regular style, languages.
#[cfg(not(windows))]
type FcFace = (PathBuf, u32, bool, Vec<String>);

#[cfg(not(windows))]
fn fontconfig_match(script: &Script) -> Vec<(PathBuf, u32)> {
    static LIST: OnceLock<Vec<FcFace>> = OnceLock::new();
    let list = LIST.get_or_init(|| {
        let Ok(out) = std::process::Command::new("fc-list")
            .args(["--format", "%{file}\t%{index}\t%{style[0]}\t%{lang}\n"])
            .stderr(std::process::Stdio::null())
            .output()
        else {
            return Vec::new();
        };
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let file = parts.next().filter(|f| !f.is_empty())?;
                let index = parts.next()?.trim().parse().unwrap_or(0);
                let style = parts.next().unwrap_or("").to_ascii_lowercase();
                let langs = parts
                    .next()
                    .unwrap_or("")
                    .split('|')
                    .map(str::to_owned)
                    .collect();
                let regular = matches!(style.as_str(), "regular" | "book" | "normal" | "roman");
                Some((PathBuf::from(file), index, regular, langs))
            })
            .collect()
    });
    let mut hits: Vec<_> = list
        .iter()
        .filter(|(_, _, _, langs)| langs.iter().any(|l| l == script.lang))
        .collect();
    hits.sort_by_key(|(_, _, regular, _)| !regular);
    hits.into_iter()
        .map(|(p, i, _, _)| (p.clone(), *i))
        .collect()
}

#[cfg(windows)]
fn fontconfig_match(_script: &Script) -> Vec<(PathBuf, u32)> {
    Vec::new()
}

/// The font has an outline for `c` (colour bitmap fonts don't count, egui
/// can't draw them).
fn has_char(data: &FontData, c: char) -> bool {
    FontRef::from_index(&data.font, data.index).is_ok_and(|f| {
        (f.glyf().is_ok() || f.cff().is_ok() || f.cff2().is_ok())
            && f.charmap().map(c).is_some_and(|g| g.to_u32() != 0)
    })
}

/// The font family of the toolbar icons (Phosphor, MIT licence).
pub const ICON_FONT: &str = "phosphor";

/// The icon font at `size` points.
pub fn icon_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(ICON_FONT.into()))
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
    fn picks_the_regular_face_of_a_script_font() {
        let stems = file_stems(&[
            PathBuf::from("/f/NotoSansBengali-Bold.ttf"),
            PathBuf::from("/f/NotoSansBengali[wdth,wght].ttf"),
            PathBuf::from("/f/NotoSansBengali-Regular.ttf"),
            PathBuf::from("/f/NotoSansBengaliUI-Regular.ttf"),
        ]);
        let hits = matching_files(&stems, "notosansbengali");
        assert_eq!(hits[0], PathBuf::from("/f/NotoSansBengali-Regular.ttf"));
        assert!(!hits.contains(&PathBuf::from("/f/NotoSansBengali-Bold.ttf")));
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn fallbacks_cover_what_they_claim() {
        let start = std::time::Instant::now();
        let names: Vec<_> = script_fallbacks().iter().map(|(n, _)| n.as_str()).collect();
        eprintln!("fallbacks {names:?} found in {:?}", start.elapsed());
        for (name, data) in script_fallbacks() {
            if let Some(s) = SCRIPTS
                .iter()
                .find(|s| name == &format!("{}-fallback", s.name))
            {
                assert!(has_char(data, s.probe), "{name}");
            }
        }
    }

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
