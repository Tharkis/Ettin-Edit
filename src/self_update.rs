//! Updating Ettin Edit in place: works out how this copy was installed, picks
//! the matching file from a GitHub release, downloads it, checks its SHA-256
//! against the one GitHub lists for the file and installs it.
//!
//! - Arch and Debian packages are installed with `pkexec pacman -U` or
//!   `pkexec apt-get install`, which asks for an administrator password.
//! - The Linux `.tar.gz` and Windows `.zip` copies replace their own program
//!   file.
//! - The Windows installer runs the new `setup.exe` silently once Ettin Edit
//!   has closed; setup starts Ettin Edit again when it's done.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use sha2::{Digest, Sha256};

/// A file attached to a release.
#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    /// GitHub's SHA-256 of the file, as lowercase hex.
    pub sha256: Option<String>,
}

/// How this copy of Ettin Edit was installed.
#[derive(Clone, Debug, PartialEq)]
// Each platform only detects its own kinds.
#[allow(dead_code)]
pub enum Install {
    /// The Arch package (`pacman`), in /usr/bin.
    Arch,
    /// The Debian package (`dpkg`), in /usr/bin.
    Deb,
    /// The Linux `.tar.gz`, unpacked to a folder the user can write to.
    LinuxPortable { exe: PathBuf },
    /// The Windows installer; `all_users` when installed for everyone.
    WindowsSetup { all_users: bool },
    /// The Windows `.zip`, unpacked to a folder the user can write to.
    WindowsPortable { exe: PathBuf },
    /// Can't update itself; the reason is shown with the download link.
    Manual(String),
}

impl Install {
    /// The end of the release file's name that suits this install.
    pub fn asset_suffix(&self) -> Option<&'static str> {
        match self {
            Install::Arch => Some("-x86_64.pkg.tar.zst"),
            Install::Deb => Some("_amd64.deb"),
            Install::LinuxPortable { .. } => Some("-linux-x86_64.tar.gz"),
            Install::WindowsSetup { .. } => Some("-windows-x86_64-setup.exe"),
            Install::WindowsPortable { .. } => Some("-windows-x86_64.zip"),
            Install::Manual(_) => None,
        }
    }

    /// One line on what updating will do.
    pub fn describe(&self) -> String {
        match self {
            Install::Arch => t!("self-update-how-arch"),
            Install::Deb => t!("self-update-how-deb"),
            Install::LinuxPortable { .. } | Install::WindowsPortable { .. } => {
                t!("self-update-how-portable")
            }
            Install::WindowsSetup { .. } => t!("self-update-how-setup"),
            Install::Manual(reason) => reason.clone(),
        }
    }
}

/// The release file for `install`, if the release has one.
pub fn pick_asset<'a>(assets: &'a [Asset], install: &Install) -> Option<&'a Asset> {
    let suffix = install.asset_suffix()?;
    assets.iter().find(|a| {
        a.name.starts_with("ettin-edit")
            && a.name.ends_with(suffix)
            // The portable Windows zip, not the installer.
            && !(suffix == "-windows-x86_64.zip" && a.name.ends_with("-setup.zip"))
    })
}

/// The program file this process started from, read once at startup: once a
/// package update replaces it, Linux reports it as "(deleted)".
pub fn exe_path() -> Option<PathBuf> {
    static EXE: OnceLock<Option<PathBuf>> = OnceLock::new();
    EXE.get_or_init(|| {
        let exe = std::env::current_exe().ok()?;
        Some(exe.canonicalize().unwrap_or(exe))
    })
    .clone()
}

/// True for a program built from a source checkout (`cargo build`).
fn is_source_build(exe: &Path) -> bool {
    exe.ancestors()
        .skip_while(|p| p.file_name().is_none_or(|n| n != "target"))
        .nth(1)
        .is_some_and(|root| root.join("Cargo.toml").is_file())
}

/// True when new files can be written next to `exe`.
fn can_write_beside(exe: &Path) -> bool {
    let Some(dir) = exe.parent() else {
        return false;
    };
    let probe = dir.join(format!(".ettin-edit-write-test-{}", std::process::id()));
    let ok = std::fs::File::create(&probe).is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Works out how this copy was installed. Runs `pacman`/`dpkg-query`, so call
/// it off the UI thread.
pub fn detect() -> Install {
    let Some(exe) = exe_path() else {
        return Install::Manual(t!("self-update-no-exe"));
    };
    if !cfg!(target_arch = "x86_64") {
        return Install::Manual(t!("self-update-no-build"));
    }
    if is_source_build(&exe) {
        return Install::Manual(t!("self-update-source-build"));
    }
    detect_for(&exe)
}

#[cfg(windows)]
fn detect_for(exe: &Path) -> Install {
    let dir = exe.parent().unwrap_or(Path::new("."));
    if dir.join("unins000.exe").is_file() {
        // Per-user installs live under %LOCALAPPDATA%\Programs.
        let all_users = dirs::data_local_dir()
            .and_then(|d| d.canonicalize().ok())
            .is_none_or(|local| !exe.starts_with(local));
        return Install::WindowsSetup { all_users };
    }
    if can_write_beside(exe) {
        Install::WindowsPortable {
            exe: exe.to_owned(),
        }
    } else {
        Install::Manual(t!(
            "self-update-read-only",
            path = dir.display().to_string()
        ))
    }
}

#[cfg(not(windows))]
fn detect_for(exe: &Path) -> Install {
    let owned_by = |cmd: &str, args: &[&str]| {
        Command::new(cmd)
            .args(args)
            .arg(exe)
            .output()
            .is_ok_and(|o| o.status.success())
    };
    if exe.starts_with("/usr") {
        if owned_by("pacman", &["-Qqo"]) {
            return Install::Arch;
        }
        if owned_by("dpkg-query", &["-S"]) {
            return Install::Deb;
        }
    }
    if can_write_beside(exe) {
        Install::LinuxPortable {
            exe: exe.to_owned(),
        }
    } else {
        Install::Manual(t!(
            "self-update-read-only",
            path = exe.parent().unwrap_or(exe).display().to_string()
        ))
    }
}

/// Where downloads wait to be installed.
fn download_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ettin-edit")
        .join("updates")
}

/// Shared with the UI while a download runs.
#[derive(Default)]
pub struct Progress {
    pub done: AtomicU64,
    pub total: AtomicU64,
    pub cancel: AtomicBool,
    /// Downloaded and checked; installing now.
    pub installing: AtomicBool,
}

/// Downloads `asset` into the update folder and checks its SHA-256.
pub fn download(asset: &Asset, progress: &Progress) -> Result<PathBuf, String> {
    let expected = asset
        .sha256
        .clone()
        .ok_or_else(|| t!("self-update-no-checksum", file = asset.name.clone()))?;
    let dir = download_dir();
    // Old downloads aren't needed any more.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| t!("self-update-save-failed", error = e.to_string()))?;
    // A release file name never has a path in it, but don't trust that.
    let name = Path::new(&asset.name)
        .file_name()
        .ok_or_else(|| t!("self-update-no-checksum", file = asset.name.clone()))?;
    let path = dir.join(name);

    let mut response = crate::update::http_agent(Duration::from_secs(30 * 60))
        .get(&asset.url)
        .call()
        .map_err(|e| t!("self-update-download-failed", error = e.to_string()))?;
    progress.total.store(asset.size, Ordering::Relaxed);
    let mut reader = response.body_mut().as_reader();
    let mut file = std::fs::File::create(&path)
        .map_err(|e| t!("self-update-save-failed", error = e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        if progress.cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(t!("self-update-cancelled"));
        }
        let n = reader
            .read(&mut buf)
            .map_err(|e| t!("self-update-download-failed", error = e.to_string()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n])
            .map_err(|e| t!("self-update-save-failed", error = e.to_string()))?;
        done += n as u64;
        progress.done.store(done, Ordering::Relaxed);
    }
    file.sync_all()
        .map_err(|e| t!("self-update-save-failed", error = e.to_string()))?;
    let actual = hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(&expected) {
        let _ = std::fs::remove_file(&path);
        return Err(t!(
            "self-update-checksum-mismatch",
            file = asset.name.clone()
        ));
    }
    Ok(path)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// How an install finished.
#[derive(Debug)]
pub enum Installed {
    /// Done; restarting runs the new version.
    Restart,
    /// Runs when Ettin Edit closes (the Windows installer).
    AfterExit,
    /// No way to ask for a password here: the user runs this in a terminal.
    RunInTerminal(String),
}

/// Installs a downloaded, checked file.
pub fn install(install: &Install, file: &Path) -> Result<Installed, String> {
    let result = install_file(install, file);
    // The download isn't needed once its program is in place; the installer
    // and packages are removed by the next update.
    if matches!(
        install,
        Install::LinuxPortable { .. } | Install::WindowsPortable { .. }
    ) && result.is_ok()
    {
        let _ = std::fs::remove_dir_all(download_dir());
    }
    result
}

fn install_file(install: &Install, file: &Path) -> Result<Installed, String> {
    match install {
        Install::Arch => run_as_admin(&package_command(install, file)),
        Install::Deb => run_as_admin(&package_command(install, file)),
        Install::LinuxPortable { exe } => {
            let program = unpack_tar_gz(file, "ettin-edit")?;
            replace_exe(exe, &program)?;
            Ok(Installed::Restart)
        }
        Install::WindowsPortable { exe } => {
            let program = unpack_zip(file, "ettin-edit.exe")?;
            replace_exe(exe, &program)?;
            Ok(Installed::Restart)
        }
        Install::WindowsSetup { all_users } => {
            let mut cmd = Command::new(file);
            cmd.args(setup_args(*all_users));
            set_after_exit(Some(cmd));
            Ok(Installed::AfterExit)
        }
        Install::Manual(reason) => Err(reason.clone()),
    }
}

/// The package manager command that installs `file`, as root.
fn package_command(install: &Install, file: &Path) -> Vec<String> {
    let file = file.display().to_string();
    match install {
        Install::Arch => vec!["pacman".into(), "-U".into(), "--noconfirm".into(), file],
        _ => vec!["apt-get".into(), "install".into(), "-y".into(), file],
    }
}

fn setup_args(all_users: bool) -> Vec<&'static str> {
    vec![
        "/SILENT",
        "/SUPPRESSMSGBOXES",
        "/NORESTART",
        "/update=1",
        if all_users {
            "/ALLUSERS"
        } else {
            "/CURRENTUSER"
        },
    ]
}

/// A command line a person can paste into a terminal.
fn shell_line(args: &[String]) -> String {
    let quote = |a: &String| {
        if a.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=:+".contains(c))
        {
            a.clone()
        } else {
            format!("'{}'", a.replace('\'', r"'\''"))
        }
    };
    std::iter::once("sudo".to_owned())
        .chain(args.iter().map(quote))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs `args` as root through pkexec, which shows the desktop's password
/// prompt. Without pkexec, hands back the command to run in a terminal.
fn run_as_admin(args: &[String]) -> Result<Installed, String> {
    let output = match Command::new("pkexec").args(args).output() {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Installed::RunInTerminal(shell_line(args)));
        }
        Err(e) => return Err(t!("self-update-install-failed", error = e.to_string())),
    };
    match output.status.code() {
        Some(0) => Ok(Installed::Restart),
        // pkexec: the password prompt was dismissed, or the user isn't allowed.
        Some(126) | Some(127) => Err(t!("self-update-not-authorised")),
        _ => {
            let mut text = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            if text.is_empty() {
                text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            }
            let tail: Vec<&str> = text.lines().rev().take(6).collect();
            let text = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
            Err(t!("self-update-install-failed", error = text))
        }
    }
}

/// Pulls the file called `name` out of a `.tar.gz`, next to the archive.
fn unpack_tar_gz(archive: &Path, name: &str) -> Result<PathBuf, String> {
    let fail = |e: String| t!("self-update-unpack-failed", error = e);
    let file = std::fs::File::open(archive).map_err(|e| fail(e.to_string()))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    for entry in tar.entries().map_err(|e| fail(e.to_string()))? {
        let mut entry = entry.map_err(|e| fail(e.to_string()))?;
        let is_it = entry.header().entry_type().is_file()
            && entry
                .path()
                .ok()
                .is_some_and(|p| p.file_name().is_some_and(|n| n == name));
        if is_it {
            let out = archive.with_extension("bin");
            let mut dest = std::fs::File::create(&out).map_err(|e| fail(e.to_string()))?;
            std::io::copy(&mut entry, &mut dest).map_err(|e| fail(e.to_string()))?;
            return Ok(out);
        }
    }
    Err(fail(t!("self-update-missing-file", file = name)))
}

/// Pulls the file called `name` out of a `.zip`, next to the archive.
fn unpack_zip(archive: &Path, name: &str) -> Result<PathBuf, String> {
    let fail = |e: String| t!("self-update-unpack-failed", error = e);
    let file = std::fs::File::open(archive).map_err(|e| fail(e.to_string()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| fail(e.to_string()))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| fail(e.to_string()))?;
        let is_it = entry.is_file()
            && entry
                .enclosed_name()
                .is_some_and(|p| p.file_name().is_some_and(|n| n == name));
        if is_it {
            let out = archive.with_extension("bin");
            let mut dest = std::fs::File::create(&out).map_err(|e| fail(e.to_string()))?;
            std::io::copy(&mut entry, &mut dest).map_err(|e| fail(e.to_string()))?;
            return Ok(out);
        }
    }
    Err(fail(t!("self-update-missing-file", file = name)))
}

/// Puts `new` in place of the running program `exe`. The new file is copied
/// beside it first, so the swap is a rename on the same disk.
fn replace_exe(exe: &Path, new: &Path) -> Result<(), String> {
    let fail = |e: std::io::Error| t!("self-update-replace-failed", error = e.to_string());
    let staged = exe.with_extension("new");
    std::fs::copy(new, &staged).map_err(fail)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).map_err(fail)?;
    }
    // Windows won't overwrite a running program but will rename it; the old
    // one is deleted at the next start (see `clean_up`).
    #[cfg(windows)]
    {
        let old = old_exe(exe);
        let _ = std::fs::remove_file(&old);
        std::fs::rename(exe, &old).map_err(fail)?;
    }
    if let Err(e) = std::fs::rename(&staged, exe) {
        #[cfg(windows)]
        let _ = std::fs::rename(old_exe(exe), exe);
        let _ = std::fs::remove_file(&staged);
        return Err(fail(e));
    }
    Ok(())
}

#[cfg(windows)]
fn old_exe(exe: &Path) -> PathBuf {
    exe.with_extension("exe.old")
}

/// Removes what an earlier update left behind. Called at startup.
pub fn clean_up() {
    #[cfg(windows)]
    if let Some(exe) = exe_path() {
        let _ = std::fs::remove_file(old_exe(&exe));
    }
    if let Some(exe) = exe_path() {
        let staged = exe.with_extension("new");
        if staged.is_file() {
            let _ = std::fs::remove_file(staged);
        }
    }
}

fn after_exit() -> &'static Mutex<Option<Command>> {
    static AFTER: OnceLock<Mutex<Option<Command>>> = OnceLock::new();
    AFTER.get_or_init(|| Mutex::new(None))
}

/// Sets what to start once Ettin Edit has closed: the new version, or the
/// Windows installer.
pub fn set_after_exit(cmd: Option<Command>) {
    if let Ok(mut slot) = after_exit().lock() {
        *slot = cmd;
    }
}

/// Starts this program again once it has closed.
pub fn restart_after_exit() {
    if let Some(exe) = exe_path() {
        set_after_exit(Some(Command::new(exe)));
    }
}

pub fn has_after_exit() -> bool {
    after_exit().lock().is_ok_and(|slot| slot.is_some())
}

/// Called by `main` after the window has closed.
pub fn run_after_exit() {
    let cmd = after_exit().lock().ok().and_then(|mut slot| slot.take());
    if let Some(mut cmd) = cmd
        && let Err(e) = cmd.spawn()
    {
        eprintln!("Ettin Edit: couldn't start {:?}: {e}", cmd.get_program());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.into(),
            url: format!("https://example.com/{name}"),
            size: 1,
            sha256: Some("00".into()),
        }
    }

    #[test]
    fn picks_the_file_for_each_install() {
        let assets: Vec<Asset> = [
            "ettin-edit-1.7.0-1-x86_64.pkg.tar.zst",
            "ettin-edit-1.7.0-linux-x86_64.tar.gz",
            "ettin-edit-1.7.0-windows-x86_64-setup.exe",
            "ettin-edit-1.7.0-windows-x86_64.zip",
            "ettin-edit_1.7.0_amd64.deb",
            "hl7-viewer.zip",
        ]
        .into_iter()
        .map(asset)
        .collect();
        let name = |i: &Install| pick_asset(&assets, i).map(|a| a.name.clone());
        assert_eq!(
            name(&Install::Arch).unwrap(),
            "ettin-edit-1.7.0-1-x86_64.pkg.tar.zst"
        );
        assert_eq!(name(&Install::Deb).unwrap(), "ettin-edit_1.7.0_amd64.deb");
        let exe = PathBuf::from("x");
        assert_eq!(
            name(&Install::LinuxPortable { exe: exe.clone() }).unwrap(),
            "ettin-edit-1.7.0-linux-x86_64.tar.gz"
        );
        assert_eq!(
            name(&Install::WindowsPortable { exe }).unwrap(),
            "ettin-edit-1.7.0-windows-x86_64.zip"
        );
        assert_eq!(
            name(&Install::WindowsSetup { all_users: false }).unwrap(),
            "ettin-edit-1.7.0-windows-x86_64-setup.exe"
        );
        assert_eq!(name(&Install::Manual("no".into())), None);
    }

    #[test]
    fn spots_source_builds() {
        let dir = std::env::temp_dir().join(format!("ettin-src-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("target/release")).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "").unwrap();
        assert!(is_source_build(&dir.join("target/release/ettin-edit")));
        assert!(!is_source_build(&dir.join("bin/ettin-edit")));
        assert!(!is_source_build(Path::new("/usr/bin/ettin-edit")));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn package_commands() {
        let file =
            Path::new("/home/me/.cache/ettin-edit/updates/ettin-edit-1.7.0-1-x86_64.pkg.tar.zst");
        let arch = package_command(&Install::Arch, file);
        assert_eq!(arch[..3], ["pacman", "-U", "--noconfirm"]);
        assert_eq!(
            shell_line(&arch),
            "sudo pacman -U --noconfirm /home/me/.cache/ettin-edit/updates/ettin-edit-1.7.0-1-x86_64.pkg.tar.zst"
        );
        let deb = package_command(&Install::Deb, Path::new("/tmp/my files/e.deb"));
        assert_eq!(deb[0], "apt-get");
        assert_eq!(
            shell_line(&deb),
            "sudo apt-get install -y '/tmp/my files/e.deb'"
        );
        assert!(setup_args(true).contains(&"/ALLUSERS"));
        assert!(setup_args(false).contains(&"/CURRENTUSER"));
        assert!(setup_args(false).contains(&"/update=1"));
    }

    #[test]
    fn unpacks_and_replaces_a_portable_program() {
        let dir = std::env::temp_dir().join(format!("ettin-unpack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // A .tar.gz like the release's.
        let archive = dir.join("ettin-edit-9.0.0-linux-x86_64.tar.gz");
        {
            let gz = flate2::write::GzEncoder::new(
                std::fs::File::create(&archive).unwrap(),
                flate2::Compression::fast(),
            );
            let mut tar = tar::Builder::new(gz);
            for (path, body) in [
                ("ettin-edit-9.0.0-linux-x86_64/README.md", &b"readme"[..]),
                (
                    "ettin-edit-9.0.0-linux-x86_64/ettin-edit",
                    &b"new program"[..],
                ),
            ] {
                let mut header = tar::Header::new_gnu();
                header.set_size(body.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                tar.append_data(&mut header, path, body).unwrap();
            }
            tar.into_inner().unwrap().finish().unwrap();
        }
        let program = unpack_tar_gz(&archive, "ettin-edit").unwrap();
        assert_eq!(std::fs::read(&program).unwrap(), b"new program");
        assert!(unpack_tar_gz(&archive, "missing").is_err());

        let exe = dir.join("ettin-edit");
        std::fs::write(&exe, "old program").unwrap();
        replace_exe(&exe, &program).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"new program");
        assert!(!exe.with_extension("new").exists());

        // And a .zip like the Windows one.
        let zip_path = dir.join("e.zip");
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
            zip.start_file(
                "ettin-edit-9.0.0-windows-x86_64/ettin-edit.exe",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"windows program").unwrap();
            zip.finish().unwrap();
        }
        let program = unpack_zip(&zip_path, "ettin-edit.exe").unwrap();
        assert_eq!(std::fs::read(&program).unwrap(), b"windows program");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn hashes_as_hex() {
        assert_eq!(
            hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
