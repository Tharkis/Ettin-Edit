//! A plugin's `plugin.json`, finding plugins in the plugins folder, and the
//! fingerprint that tells whether a plugin changed since the user allowed it.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The program that runs a plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runtime {
    Python,
    Node,
}

impl Runtime {
    /// The language name shown to users.
    pub fn label(self) -> &'static str {
        match self {
            Runtime::Python => "Python",
            Runtime::Node => "JavaScript",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "python" | "python3" | "py" => Some(Runtime::Python),
            "node" | "nodejs" | "javascript" | "js" => Some(Runtime::Node),
            _ => None,
        }
    }

    fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "py" => Some(Runtime::Python),
            "js" | "cjs" | "mjs" => Some(Runtime::Node),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawManifest {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    author: Option<String>,
    main: Option<String>,
    runtime: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    /// The script to run, inside the plugin's folder.
    pub main: PathBuf,
    pub runtime: Runtime,
}

pub const MANIFEST_FILE: &str = "plugin.json";

/// A folder in the plugins folder.
#[derive(Clone, Debug)]
pub struct Discovered {
    /// The folder name, which is the plugin's id.
    pub id: String,
    pub dir: PathBuf,
    pub manifest: Result<Manifest, String>,
}

/// Every plugin folder in `root`, sorted by id. Folders starting with `.` or
/// `_` are skipped (the SDK lives in `.sdk`).
pub fn discover(root: &Path) -> Vec<Discovered> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<Discovered> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let id = e.file_name().to_str()?.to_owned();
            if id.starts_with('.') || id.starts_with('_') {
                return None;
            }
            let dir = e.path();
            Some(Discovered {
                manifest: load(&dir),
                id,
                dir,
            })
        })
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// Reads and checks `plugin.json` in a plugin folder.
pub fn load(dir: &Path) -> Result<Manifest, String> {
    let path = dir.join(MANIFEST_FILE);
    let text =
        fs::read_to_string(&path).map_err(|e| format!("Couldn't read {MANIFEST_FILE}: {e}"))?;
    parse(dir, &text)
}

fn parse(dir: &Path, text: &str) -> Result<Manifest, String> {
    let raw: RawManifest =
        serde_json::from_str(text).map_err(|e| format!("{MANIFEST_FILE} isn't valid: {e}"))?;
    let main = match &raw.main {
        Some(m) => m.clone(),
        None if dir.join("main.py").is_file() => "main.py".to_owned(),
        None if dir.join("main.js").is_file() => "main.js".to_owned(),
        None => return Err(format!("{MANIFEST_FILE} has no \"main\" script")),
    };
    let rel = Path::new(&main);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!(
            "\"main\" must be a file inside the plugin's folder: {main}"
        ));
    }
    let main_path = dir.join(rel);
    if !main_path.is_file() {
        return Err(format!("The main script {main} doesn't exist"));
    }
    let runtime = match &raw.runtime {
        Some(r) => Runtime::from_name(r)
            .ok_or_else(|| format!("Unknown runtime \"{r}\"; use \"python\" or \"node\""))?,
        None => Runtime::from_extension(rel).ok_or_else(|| {
            format!("Can't tell how to run {main}; add \"runtime\": \"python\" or \"node\"")
        })?,
    };
    let folder = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Manifest {
        name: raw.name.filter(|n| !n.trim().is_empty()).unwrap_or(folder),
        version: raw.version.unwrap_or_default(),
        description: raw.description.unwrap_or_default(),
        author: raw.author.unwrap_or_default(),
        main: main_path,
        runtime,
    })
}

/// Folders never included in the fingerprint: caches and installed packages
/// change on their own.
const SKIP_DIRS: &[&str] = &["__pycache__", "node_modules", "venv", ".venv", ".git"];
const MAX_FILES: usize = 5000;
/// File contents are hashed up to this many bytes in total; after that only
/// names and sizes count.
const MAX_HASHED_BYTES: u64 = 64 << 20;

/// A hash of every file in a plugin's folder, to notice when it changes after
/// the user allowed it to run. Not cryptographic: anyone who can write the
/// plugins folder can already run code as the user.
pub fn fingerprint(dir: &Path) -> String {
    let mut files = Vec::new();
    collect(dir, dir, &mut files);
    files.sort();
    let mut h = Fnv::new();
    let mut hashed = 0u64;
    for rel in files {
        let path = dir.join(&rel);
        h.write(rel.to_string_lossy().replace('\\', "/").as_bytes());
        h.write(&[0]);
        let len = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        h.write(&len.to_le_bytes());
        if hashed + len <= MAX_HASHED_BYTES
            && let Ok(bytes) = fs::read(&path)
        {
            hashed += len;
            h.write(&bytes);
        }
    }
    format!("{:016x}", h.0)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        if out.len() >= MAX_FILES {
            return;
        }
        let name = e.file_name();
        let name = name.to_string_lossy();
        let Ok(kind) = e.file_type() else { continue };
        let path = e.path();
        if kind.is_dir() {
            if !SKIP_DIRS.contains(&name.as_ref()) {
                collect(root, &path, out);
            }
        } else if kind.is_file()
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_path_buf());
        }
    }
}

/// 64-bit FNV-1a.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nixpad-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_manifest_and_guesses_runtime() {
        let dir = temp_dir("manifest");
        fs::write(dir.join("main.py"), "print(1)").unwrap();
        let m = parse(&dir, r#"{"name": "Hello", "version": "1.0"}"#).unwrap();
        assert_eq!(m.name, "Hello");
        assert_eq!(m.runtime, Runtime::Python);
        assert_eq!(m.main, dir.join("main.py"));

        fs::write(dir.join("tool.cjs"), "1").unwrap();
        let m = parse(&dir, r#"{"main": "tool.cjs"}"#).unwrap();
        assert_eq!(m.runtime, Runtime::Node);
        assert!(m.name.starts_with("nixpad-test"));

        assert!(parse(&dir, r#"{"main": "../x.py"}"#).is_err());
        assert!(parse(&dir, r#"{"main": "missing.py"}"#).is_err());
        assert!(parse(&dir, r#"{"main": "main.py", "runtime": "ruby"}"#).is_err());
        assert!(parse(&dir, "{oops").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discovers_folders_and_skips_hidden_ones() {
        let root = temp_dir("discover");
        for id in ["b", "a", ".sdk", "_off"] {
            fs::create_dir_all(root.join(id)).unwrap();
        }
        fs::write(root.join("a").join("main.js"), "").unwrap();
        fs::write(root.join("a").join(MANIFEST_FILE), "{}").unwrap();
        let found = discover(&root);
        let ids: Vec<&str> = found.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert!(found[0].manifest.is_ok());
        assert!(found[1].manifest.is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fingerprint_changes_with_contents() {
        let dir = temp_dir("fingerprint");
        fs::write(dir.join("main.py"), "a").unwrap();
        let first = fingerprint(&dir);
        assert_eq!(first, fingerprint(&dir));
        fs::create_dir_all(dir.join("__pycache__")).unwrap();
        fs::write(dir.join("__pycache__").join("x.pyc"), "zz").unwrap();
        assert_eq!(first, fingerprint(&dir));
        fs::write(dir.join("main.py"), "b").unwrap();
        assert_ne!(first, fingerprint(&dir));
        let _ = fs::remove_dir_all(&dir);
    }
}
