//! The plugin manager's work: reading the plugin index, and installing,
//! updating and removing plugins from `.zip` files or folders.
//!
//! Downloads and installs run on a background thread; the window asks for
//! results each frame with [`Store::poll`].

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::manifest::{self, MANIFEST_FILE};

/// Where the list of plugins comes from unless the user changes it.
pub const DEFAULT_INDEX_URL: &str =
    "https://raw.githubusercontent.com/Tharkis/Ettin-Edit/main/plugins/index.json";

/// This Ettin Edit's version, for `min_app_version`.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_INDEX_BYTES: u64 = 4 << 20;
const MAX_DOWNLOAD_BYTES: u64 = 64 << 20;
/// Zips are refused when they'd unpack to more than this.
const MAX_EXTRACTED_BYTES: u64 = 256 << 20;
const MAX_ENTRIES: usize = 10_000;
const TIMEOUT: Duration = Duration::from_secs(60);
/// Folders not copied by Install from Folder.
const SKIP_COPY: &[&str] = &[".git", "__pycache__"];

/// One plugin in the index.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct IndexEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: String,
    /// The address of the plugin's `.zip`.
    pub download: String,
    pub sha256: Option<String>,
    pub min_app_version: Option<String>,
}

impl IndexEntry {
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            &self.id
        } else {
            &self.name
        }
    }

    /// The Ettin Edit version the plugin needs, when this one is older.
    pub fn needs_newer_app(&self) -> Option<&str> {
        self.min_app_version
            .as_deref()
            .filter(|min| compare_versions(min, APP_VERSION) == Ordering::Greater)
    }

    /// True if the filter text appears in its name, id, author or description.
    pub fn matches(&self, filter: &str) -> bool {
        let filter = filter.trim().to_lowercase();
        filter.is_empty()
            || [&self.name, &self.id, &self.author, &self.description]
                .iter()
                .any(|s| s.to_lowercase().contains(&filter))
    }
}

/// Reads an index: `{"plugins": [...]}`. Entries without a usable id or
/// download address are left out; the rest are sorted by name. Download
/// addresses may be relative to `base_url`, the index's own address.
pub fn parse_index(text: &str, base_url: &str) -> Result<Vec<IndexEntry>, String> {
    let root: Value = serde_json::from_str(text)
        .map_err(|e| format!("The plugin index isn't valid JSON: {e}"))?;
    let list = root
        .get("plugins")
        .and_then(Value::as_array)
        .ok_or("The plugin index has no \"plugins\" list")?;
    let mut seen = BTreeSet::new();
    let mut entries: Vec<IndexEntry> = list
        .iter()
        .filter_map(|v| serde_json::from_value::<IndexEntry>(v.clone()).ok())
        .filter(|e| valid_id(&e.id) && !e.download.trim().is_empty())
        .filter(|e| seen.insert(e.id.clone()))
        .map(|mut e| {
            e.download = resolve_url(base_url, e.download.trim());
            e.sha256 = e.sha256.filter(|s| !s.trim().is_empty());
            e.min_app_version = e.min_app_version.filter(|s| !s.trim().is_empty());
            e
        })
        .collect();
    entries.sort_by_key(|e| e.display_name().to_lowercase());
    Ok(entries)
}

/// `href` as a full address, relative to the folder `base` is in.
fn resolve_url(base: &str, href: &str) -> String {
    if href.contains("://") || Path::new(href).is_absolute() {
        return href.to_owned();
    }
    match base.rfind('/') {
        Some(i) if base.contains("://") => format!("{}{}", &base[..=i], href),
        _ => match Path::new(base).parent() {
            Some(dir) => dir.join(href).to_string_lossy().into_owned(),
            None => href.to_owned(),
        },
    }
}

/// Compares versions such as `1.2.10` and `1.10` part by part as numbers;
/// missing parts count as 0, a leading `v` is ignored, and a pre-release
/// (`1.0.0-beta`) comes before the release.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (an, ap) = split_version(a);
    let (bn, bp) = split_version(b);
    for i in 0..an.len().max(bn.len()) {
        let x = an.get(i).copied().unwrap_or(0);
        let y = bn.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    match (ap.is_empty(), bp.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => ap.cmp(bp),
    }
}

/// True if `available` is a later version than `installed`.
pub fn is_newer(available: &str, installed: &str) -> bool {
    compare_versions(available, installed) == Ordering::Greater
}

/// The numeric parts of a version, and its pre-release label.
fn split_version(v: &str) -> (Vec<u64>, &str) {
    let v = v.trim();
    let v = v.strip_prefix(['v', 'V']).unwrap_or(v);
    // Build metadata (`+…`) doesn't change the order.
    let v = v.split('+').next().unwrap_or("");
    let (core, pre) = v.split_once('-').unwrap_or((v, ""));
    let nums = core
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().unwrap_or(0)
        })
        .collect();
    (nums, pre)
}

/// A plugin id is a folder name: letters, digits, `-`, `_` and `.`,
/// starting with a letter or digit.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Makes a folder or file name into a plugin id, or `None` if nothing is left.
pub fn id_from_name(name: &str) -> Option<String> {
    let mut id = String::new();
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '.') {
            id.push(c.to_ascii_lowercase());
        } else if !id.ends_with('-') {
            id.push('-');
        }
    }
    let id = id
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .trim_end_matches('-');
    let id: String = id.chars().take(64).collect();
    valid_id(&id).then_some(id)
}

/// The name of a zip entry as a relative path joined with `/`, or an error
/// for a name that could write outside the folder it's unpacked into
/// (an absolute path, a drive, or `..`).
pub fn entry_path(name: &str) -> Result<String, String> {
    let unsafe_name =
        || format!("The zip has an unsafe file name, so nothing was installed: {name}");
    let unified = name.replace('\\', "/");
    if unified.starts_with('/') || unified.contains('\0') {
        return Err(unsafe_name());
    }
    let mut parts = Vec::new();
    for part in unified.split('/') {
        match part {
            "" | "." => {}
            ".." => return Err(unsafe_name()),
            // `C:` is a drive on Windows, and `:` starts a stream there.
            p if p.contains(':') => return Err(unsafe_name()),
            p => parts.push(p),
        }
    }
    Ok(parts.join("/"))
}

/// Where `plugin.json` is among a zip's entries: `""` when it's at the top,
/// or `"folder/"` when the zip holds one folder with the plugin in it.
pub fn plugin_root(names: &[String]) -> Result<String, String> {
    if names.iter().any(|n| n == MANIFEST_FILE) {
        return Ok(String::new());
    }
    let tops: BTreeSet<&str> = names
        .iter()
        .filter_map(|n| n.split('/').next())
        .filter(|t| !t.is_empty())
        .collect();
    if let [top] = tops.into_iter().collect::<Vec<_>>()[..]
        && names.iter().any(|n| *n == format!("{top}/{MANIFEST_FILE}"))
    {
        return Ok(format!("{top}/"));
    }
    Err(format!(
        "The zip has no {MANIFEST_FILE}, either at the top or inside a single folder."
    ))
}

/// Unpacks a plugin zip into `dest`, leaving out the folder around the
/// plugin if there is one. Returns that folder's name. Refuses the whole zip
/// if any entry has an unsafe name.
pub fn extract_plugin_zip(bytes: &[u8], dest: &Path) -> Result<Option<String>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("That isn't a zip file Ettin Edit can read: {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err(format!("The zip has more than {MAX_ENTRIES} files."));
    }
    let bad = |e: zip::result::ZipError| format!("The zip is damaged: {e}");
    // Check every name before writing anything.
    let mut entries = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let file = zip.by_index_raw(i).map_err(bad)?;
        let raw = file.name().map_err(bad)?.into_owned();
        let path = entry_path(&raw)?;
        if file.is_symlink() {
            return Err(format!(
                "The zip has a link, which plugins can't contain: {raw}"
            ));
        }
        if file.encrypted() {
            return Err("The zip is password-protected.".to_owned());
        }
        // macOS adds a folder of file details to zips it makes.
        if path.is_empty() || path == "__MACOSX" || path.starts_with("__MACOSX/") {
            continue;
        }
        entries.push((i, path, file.is_dir()));
    }
    let names: Vec<String> = entries.iter().map(|(_, p, _)| p.clone()).collect();
    let root = plugin_root(&names)?;
    let mut total = 0u64;
    for (i, path, is_dir) in entries {
        let Some(rel) = path.strip_prefix(&root) else {
            continue;
        };
        if rel.is_empty() {
            continue;
        }
        let target = rel
            .split('/')
            .fold(dest.to_path_buf(), |p, part| p.join(part));
        let fail = |e: std::io::Error| format!("Couldn't write {}: {e}", target.display());
        if is_dir {
            fs::create_dir_all(&target).map_err(fail)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(fail)?;
        }
        let mut file = zip.by_index(i).map_err(bad)?;
        let mut data = Vec::new();
        let left = MAX_EXTRACTED_BYTES - total;
        (&mut file)
            .take(left + 1)
            .read_to_end(&mut data)
            .map_err(|e| format!("The zip is damaged: {e}"))?;
        total += data.len() as u64;
        if total > MAX_EXTRACTED_BYTES {
            return Err("The zip unpacks to more than 256 MB.".to_owned());
        }
        fs::write(&target, data).map_err(fail)?;
    }
    Ok(root
        .strip_suffix('/')
        .filter(|r| !r.is_empty())
        .map(str::to_owned))
}

/// Copies a plugin folder, leaving out `.git` and caches.
fn copy_folder(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let name = e.file_name();
        let src = e.path();
        let dst = to.join(&name);
        let kind = e.file_type()?;
        if kind.is_dir() {
            if !SKIP_COPY.iter().any(|s| name == *s) {
                copy_folder(&src, &dst)?;
            }
        } else if kind.is_file() || (kind.is_symlink() && src.is_file()) {
            fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

/// The SHA-256 of some bytes, as lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A plugin that was installed.
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub id: String,
    pub name: String,
    pub version: String,
    /// It replaced an older copy.
    pub updated: bool,
}

/// Moves an old copy of a plugin out of the way (to the trash, outside tests).
pub type RemoveOld<'a> = &'a dyn Fn(&Path) -> Result<(), String>;

/// Moves a folder to the trash.
pub fn to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| e.to_string())
}

/// A new, hidden folder in the plugins folder to unpack into, so the final
/// move is a rename on the same drive. Hidden folders aren't loaded.
fn staging_dir(root: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(root)
        .map_err(|e| format!("Couldn't create the plugins folder {}: {e}", root.display()))?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = root.join(format!(".installing-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Installs a plugin from a zip in memory into the plugins folder `root`.
///
/// The id is `id` (from the index) if given, otherwise the folder inside the
/// zip, otherwise `name_hint` (the zip's file name). An existing copy is
/// replaced, after `remove_old` moves it away.
pub fn install_zip(
    bytes: &[u8],
    sha256: Option<&str>,
    id: Option<&str>,
    name_hint: Option<&str>,
    root: &Path,
    remove_old: RemoveOld,
) -> Result<Installed, String> {
    if let Some(expected) = sha256.map(str::trim).filter(|s| !s.is_empty()) {
        let got = sha256_hex(bytes);
        if !got.eq_ignore_ascii_case(expected) {
            return Err(format!(
                "The download doesn't match its SHA-256 checksum (expected {expected}, got {got}). It may be damaged or changed; nothing was installed."
            ));
        }
    }
    let staged = staging_dir(root)?;
    let result = extract_plugin_zip(bytes, &staged).and_then(|folder| {
        let id = choose_id(id, folder.as_deref(), name_hint)?;
        finish(&staged, root, &id, remove_old)
    });
    if result.is_err() {
        let _ = fs::remove_dir_all(&staged);
    }
    result
}

/// Installs a copy of a plugin folder (one with a `plugin.json`). Its id is
/// the folder's name.
pub fn install_folder(src: &Path, root: &Path, remove_old: RemoveOld) -> Result<Installed, String> {
    if !src.join(MANIFEST_FILE).is_file() {
        return Err(format!(
            "{} has no {MANIFEST_FILE}, so it isn't a plugin folder.",
            src.display()
        ));
    }
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let id = choose_id(None, Some(&name), None)?;
    if let (Ok(a), Ok(b)) = (src.canonicalize(), root.canonicalize())
        && (a.starts_with(&b) || b.starts_with(&a))
    {
        return Err("That folder is already in the plugins folder.".to_owned());
    }
    let staged = staging_dir(root)?;
    let result = copy_folder(src, &staged)
        .map_err(|e| format!("Couldn't copy {}: {e}", src.display()))
        .and_then(|()| finish(&staged, root, &id, remove_old));
    if result.is_err() {
        let _ = fs::remove_dir_all(&staged);
    }
    result
}

fn choose_id(id: Option<&str>, folder: Option<&str>, hint: Option<&str>) -> Result<String, String> {
    if let Some(id) = id {
        return if valid_id(id) {
            Ok(id.to_owned())
        } else {
            Err(format!("\"{id}\" can't be used as a plugin id."))
        };
    }
    folder
        .and_then(id_from_name)
        .or_else(|| hint.and_then(id_from_name))
        .ok_or_else(|| "Couldn't work out a name for the plugin's folder.".to_owned())
}

/// Checks the unpacked plugin and moves it into place as `root/id`.
fn finish(
    staged: &Path,
    root: &Path,
    id: &str,
    remove_old: RemoveOld,
) -> Result<Installed, String> {
    manifest::load(staged).map_err(|e| format!("This isn't a plugin Ettin Edit can run: {e}"))?;
    let dest = root.join(id);
    let updated = dest.exists();
    if updated {
        remove_old(&dest)
            .map_err(|e| format!("Couldn't move the installed copy of {id} to the trash: {e}"))?;
    }
    fs::rename(staged, &dest).map_err(|e| format!("Couldn't move the plugin into place: {e}"))?;
    let m = manifest::load(&dest)?;
    Ok(Installed {
        id: id.to_owned(),
        name: m.name,
        version: m.version,
        updated,
    })
}

/// Downloads `url` into memory: `http(s)://`, or a `file://` address or a
/// path for plugins on disk. Fails on any answer but success.
pub fn fetch(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let url = url.trim();
    let local = url
        .strip_prefix("file://")
        .map(PathBuf::from)
        .or_else(|| Path::new(url).is_absolute().then(|| PathBuf::from(url)));
    if let Some(path) = local {
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > limit {
            return Err("the file is too big".to_owned());
        }
        return Ok(bytes);
    }
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("the address must start with https://".to_owned());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .user_agent(format!("EttinEdit/{APP_VERSION}"))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "the server answered HTTP {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        )
        .trim_end()
        .to_owned());
    }
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

/// Downloads and reads an index.
pub fn fetch_index(url: &str) -> Result<Vec<IndexEntry>, String> {
    let bytes = fetch(url, MAX_INDEX_BYTES).map_err(|e| {
        format!(
            "The plugin index couldn't be reached ({e}). Check the address and that it's public:\n{url}"
        )
    })?;
    parse_index(&String::from_utf8_lossy(&bytes), url)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Installed,
    Available,
}

#[derive(Debug)]
pub enum IndexState {
    NotLoaded,
    Loading,
    Loaded(Vec<IndexEntry>),
    Failed(String),
}

enum Done {
    Index(String, Result<Vec<IndexEntry>, String>),
    Install(Result<Installed, String>),
}

/// The plugin manager's state: the index, a running install, and the
/// window's tab and filter.
pub struct Store {
    pub index: IndexState,
    /// The address the index was read from.
    index_url: String,
    /// What's being installed, while it is.
    pub busy: Option<String>,
    pub error: Option<String>,
    pub tab: Tab,
    pub filter: String,
    /// A plugin waiting for the user to confirm removing it.
    pub confirm_remove: Option<String>,
    tx: Sender<Done>,
    rx: Receiver<Done>,
}

impl Default for Store {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            index: IndexState::NotLoaded,
            index_url: String::new(),
            busy: None,
            error: None,
            tab: Tab::default(),
            filter: String::new(),
            confirm_remove: None,
            tx,
            rx,
        }
    }
}

impl Store {
    /// Reads the index again in the background.
    pub fn fetch_index(&mut self, url: &str, ctx: &egui::Context) {
        let url = url.trim().to_owned();
        if matches!(self.index, IndexState::Loading) && self.index_url == url {
            return;
        }
        self.index = IndexState::Loading;
        self.index_url = url.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result = fetch_index(&url);
            let _ = tx.send(Done::Index(url, result));
            ctx.request_repaint();
        });
    }

    pub fn entries(&self) -> &[IndexEntry] {
        match &self.index {
            IndexState::Loaded(list) => list,
            _ => &[],
        }
    }

    pub fn entry(&self, id: &str) -> Option<&IndexEntry> {
        self.entries().iter().find(|e| e.id == id)
    }

    /// The index entry with a later version of an installed plugin.
    pub fn update_for(&self, id: &str, installed: &str) -> Option<&IndexEntry> {
        self.entry(id)
            .filter(|e| is_newer(&e.version, installed) && e.needs_newer_app().is_none())
    }

    fn run(
        &mut self,
        what: String,
        ctx: &egui::Context,
        job: impl FnOnce() -> Result<Installed, String> + Send + 'static,
    ) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(what);
        self.error = None;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(Done::Install(job()));
            ctx.request_repaint();
        });
    }

    /// Downloads and installs a plugin from the index.
    pub fn install_entry(&mut self, entry: &IndexEntry, root: PathBuf, ctx: &egui::Context) {
        let entry = entry.clone();
        let what = format!("Installing {} {}…", entry.display_name(), entry.version);
        self.run(what, ctx, move || {
            let bytes = fetch(&entry.download, MAX_DOWNLOAD_BYTES).map_err(|e| {
                format!(
                    "Couldn't download {} ({e}):\n{}",
                    entry.display_name(),
                    entry.download
                )
            })?;
            install_zip(
                &bytes,
                entry.sha256.as_deref(),
                Some(&entry.id),
                None,
                &root,
                &to_trash,
            )
        });
    }

    /// Installs a plugin from a `.zip` file.
    pub fn install_file(&mut self, path: PathBuf, root: PathBuf, ctx: &egui::Context) {
        let what = format!(
            "Installing {}…",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        self.run(what, ctx, move || {
            let bytes = fetch(&path.to_string_lossy(), MAX_DOWNLOAD_BYTES)
                .map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
            let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
            install_zip(&bytes, None, None, stem.as_deref(), &root, &to_trash)
        });
    }

    /// Installs a copy of a plugin folder.
    pub fn install_folder(&mut self, path: PathBuf, root: PathBuf, ctx: &egui::Context) {
        let what = format!(
            "Installing {}…",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        self.run(what, ctx, move || install_folder(&path, &root, &to_trash));
    }

    /// Takes finished work: index results are kept here, and finished
    /// installs are returned for the app to load.
    pub fn poll(&mut self) -> Vec<Result<Installed, String>> {
        let mut out = Vec::new();
        while let Ok(done) = self.rx.try_recv() {
            match done {
                Done::Index(url, result) => {
                    if url != self.index_url {
                        continue;
                    }
                    self.index = match result {
                        Ok(list) => IndexState::Loaded(list),
                        Err(e) => IndexState::Failed(e),
                    };
                }
                Done::Install(result) => {
                    self.busy = None;
                    if let Err(e) = &result {
                        self.error = Some(e.clone());
                    }
                    out.push(result);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ettin-store-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_zip(files: &[(&str, &str)]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, text) in files {
            w.start_file(*name, options).unwrap();
            w.write_all(text.as_bytes()).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn remove(path: &Path) -> Result<(), String> {
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    }

    const MANIFEST: &str = r#"{"name": "Shout", "version": "1.2.0", "main": "main.py"}"#;

    #[test]
    fn parses_index() {
        let text = r#"{"plugins": [
            {"id": "zeta", "name": "Zeta", "version": "1.0", "download": "zeta.zip"},
            {"id": "alpha", "name": "Alpha", "version": "2.1.0", "description": "A",
             "author": "Me", "homepage": "https://example.com",
             "download": "https://example.com/alpha.zip", "sha256": "ab", "min_app_version": "99.0"},
            {"id": "../evil", "download": "https://example.com/x.zip"},
            {"id": "nodownload"},
            {"id": "alpha", "download": "https://example.com/dupe.zip"},
            "not an object"
        ]}"#;
        let list = parse_index(text, "https://example.com/plugins/index.json").unwrap();
        let ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["alpha", "zeta"]);
        assert_eq!(list[0].download, "https://example.com/alpha.zip");
        assert_eq!(list[0].sha256.as_deref(), Some("ab"));
        assert_eq!(list[0].needs_newer_app(), Some("99.0"));
        assert_eq!(list[1].download, "https://example.com/plugins/zeta.zip");
        assert_eq!(list[1].needs_newer_app(), None);
        assert!(list[0].matches("me"));
        assert!(!list[0].matches("zeta"));

        assert!(parse_index("{oops", "").is_err());
        assert!(parse_index(r#"{"other": []}"#, "").is_err());
    }

    #[test]
    fn the_repo_index_lists_the_samples() {
        let text = include_str!("../../plugins/index.json");
        let list = parse_index(text, DEFAULT_INDEX_URL).unwrap();
        for (folder, _) in super::super::SAMPLES {
            let e = list
                .iter()
                .find(|e| e.id == *folder)
                .expect("sample in index");
            assert!(e.download.ends_with(&format!("/{folder}.zip")));
            let manifest: Value = serde_json::from_str(
                super::super::SAMPLES
                    .iter()
                    .find(|(f, _)| f == folder)
                    .unwrap()
                    .1
                    .iter()
                    .find(|(n, _)| *n == MANIFEST_FILE)
                    .unwrap()
                    .1,
            )
            .unwrap();
            assert_eq!(manifest["version"], e.version.as_str());
            assert_eq!(manifest["name"], e.name.as_str());
        }
    }

    #[test]
    fn compares_versions_as_numbers() {
        assert_eq!(compare_versions("1.10", "1.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.2.10", "1.2.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("v2.0", "2.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.9.9", "1.0"), Ordering::Less);
        assert_eq!(compare_versions("1.0.0-beta", "1.0.0"), Ordering::Less);
        assert_eq!(
            compare_versions("1.0.0-beta.2", "1.0.0-beta.1"),
            Ordering::Greater
        );
        assert_eq!(compare_versions("1.0+build5", "1.0"), Ordering::Equal);
        assert!(is_newer("1.1", "1.0.9"));
        assert!(!is_newer("1.0", "1.0"));
        assert!(is_newer("1.0", ""));
    }

    #[test]
    fn refuses_unsafe_names() {
        assert_eq!(entry_path("a/b.py").unwrap(), "a/b.py");
        assert_eq!(entry_path("./a//b.py").unwrap(), "a/b.py");
        assert_eq!(entry_path("a\\b.py").unwrap(), "a/b.py");
        for bad in [
            "../x",
            "a/../../x",
            "/etc/passwd",
            "\\x",
            "C:/x",
            "a\\..\\x",
            "a/c:x",
        ] {
            assert!(entry_path(bad).is_err(), "{bad}");
        }

        let root = temp_dir("slip");
        let plugins = root.join("plugins");
        let zip = make_zip(&[
            ("plugin.json", MANIFEST),
            ("main.py", "print(1)"),
            ("../escaped.txt", "gotcha"),
        ]);
        let err = install_zip(&zip, None, Some("shout"), None, &plugins, &remove).unwrap_err();
        assert!(err.contains("unsafe"), "{err}");
        assert!(!root.join("escaped.txt").exists());
        assert!(!plugins.join("shout").exists());
        // The staging folder is cleaned up.
        assert_eq!(fs::read_dir(&plugins).unwrap().count(), 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_plugin_json_at_the_top_or_in_one_folder() {
        let names = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            plugin_root(&names(&["plugin.json", "main.py"])).unwrap(),
            ""
        );
        assert_eq!(
            plugin_root(&names(&["shout", "shout/plugin.json", "shout/main.py"])).unwrap(),
            "shout/"
        );
        assert!(plugin_root(&names(&["a/plugin.json", "b/main.py"])).is_err());
        assert!(plugin_root(&names(&["a/b/plugin.json"])).is_err());
        assert!(plugin_root(&names(&["main.py"])).is_err());

        let dir = temp_dir("nested");
        let zip = make_zip(&[
            ("Shout Plugin/plugin.json", MANIFEST),
            ("Shout Plugin/main.py", "print(1)"),
            ("Shout Plugin/lib/util.py", "x = 1"),
            ("__MACOSX/Shout Plugin/._main.py", "junk"),
        ]);
        let folder = extract_plugin_zip(&zip, &dir).unwrap();
        assert_eq!(folder.as_deref(), Some("Shout Plugin"));
        assert!(dir.join("plugin.json").is_file());
        assert!(dir.join("lib").join("util.py").is_file());
        assert!(!dir.join("__MACOSX").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn installs_and_updates_from_a_zip() {
        let root = temp_dir("install");
        let plugins = root.join("plugins");
        let zip = make_zip(&[("plugin.json", MANIFEST), ("main.py", "print(1)")]);

        let bad_sum = install_zip(&zip, Some("00"), None, Some("x"), &plugins, &remove);
        assert!(bad_sum.unwrap_err().contains("checksum"));

        let sum = sha256_hex(&zip);
        let got =
            install_zip(&zip, Some(&sum), None, Some("Shout 1.2"), &plugins, &remove).unwrap();
        assert_eq!(
            got,
            Installed {
                id: "shout-1.2".to_owned(),
                name: "Shout".to_owned(),
                version: "1.2.0".to_owned(),
                updated: false,
            }
        );
        assert!(plugins.join("shout-1.2").join("main.py").is_file());

        // An update replaces the old copy, after moving it away.
        let newer = make_zip(&[
            (
                "shout/plugin.json",
                r#"{"name": "Shout", "version": "1.3.0", "main": "main.py"}"#,
            ),
            ("shout/main.py", "print(2)"),
        ]);
        let removed = std::cell::RefCell::new(Vec::new());
        let remove_logged = |p: &Path| {
            removed.borrow_mut().push(p.to_path_buf());
            remove(p)
        };
        let got = install_zip(
            &newer,
            None,
            Some("shout-1.2"),
            None,
            &plugins,
            &remove_logged,
        )
        .unwrap();
        assert!(got.updated);
        assert_eq!(got.version, "1.3.0");
        assert_eq!(*removed.borrow(), [plugins.join("shout-1.2")]);
        assert_eq!(
            fs::read_to_string(plugins.join("shout-1.2").join("main.py")).unwrap(),
            "print(2)"
        );
        let discovered = manifest::discover(&plugins);
        assert_eq!(discovered.len(), 1, "the staging folder is gone or hidden");

        // Not a plugin: nothing is left behind.
        let broken = make_zip(&[("plugin.json", r#"{"main": "missing.py"}"#)]);
        assert!(install_zip(&broken, None, Some("broken"), None, &plugins, &remove).is_err());
        assert!(!plugins.join("broken").exists());
        assert_eq!(fs::read_dir(&plugins).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn installs_from_a_folder() {
        let root = temp_dir("folder");
        let src = root.join("My Tool");
        fs::create_dir_all(src.join("__pycache__")).unwrap();
        fs::write(src.join("plugin.json"), MANIFEST).unwrap();
        fs::write(src.join("main.py"), "print(1)").unwrap();
        fs::write(src.join("__pycache__").join("x.pyc"), "zz").unwrap();
        let plugins = root.join("plugins");
        let got = install_folder(&src, &plugins, &remove).unwrap();
        assert_eq!(got.id, "my-tool");
        assert!(plugins.join("my-tool").join("main.py").is_file());
        assert!(!plugins.join("my-tool").join("__pycache__").exists());
        assert!(install_folder(&plugins.join("my-tool"), &plugins, &remove).is_err());
        assert!(install_folder(&root, &plugins, &remove).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn makes_ids_from_names() {
        assert_eq!(id_from_name("Text Stats").as_deref(), Some("text-stats"));
        assert_eq!(id_from_name("--x--").as_deref(), Some("x"));
        assert_eq!(id_from_name(".hidden").as_deref(), Some("hidden"));
        assert_eq!(id_from_name("!!!"), None);
        assert!(valid_id("hl7-viewer"));
        assert!(!valid_id(".sdk"));
        assert!(!valid_id("a/b"));
    }
}
