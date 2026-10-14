//! Handing things off to the desktop: the file manager and a terminal.

use std::path::Path;
use std::process::Command;

#[cfg(windows)]
fn hidden(mut cmd: Command) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Opens a folder in the system file manager.
pub fn open_folder(dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let result = Command::new("explorer").arg(dir).spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(dir).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(dir).spawn();
    result
        .map(|_| ())
        .map_err(|e| format!("Couldn't open {}: {e}", dir.display()))
}

/// Opens the folder holding `file`, with the file selected where the
/// platform supports it.
pub fn reveal_file(file: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        // Explorer wants "/select," and the path as one argument.
        use std::os::windows::process::CommandExt;
        let mut arg = std::ffi::OsString::from("/select,\"");
        arg.push(file.as_os_str());
        arg.push("\"");
        Command::new("explorer")
            .raw_arg(arg)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Couldn't open Explorer: {e}"))
    }
    #[cfg(not(windows))]
    {
        // The freedesktop file manager interface selects the file (Dolphin,
        // Nautilus, Nemo, Thunar...); fall back to just opening the folder.
        let uri = file_uri(file);
        let selected = Command::new("dbus-send")
            .args([
                "--session",
                "--print-reply",
                "--dest=org.freedesktop.FileManager1",
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1.ShowItems",
            ])
            .arg(format!("array:string:{uri}"))
            .arg("string:")
            .output()
            .is_ok_and(|o| o.status.success());
        if selected {
            return Ok(());
        }
        match file.parent() {
            Some(dir) => open_folder(dir),
            None => Err(format!("{} has no folder", file.display())),
        }
    }
}

/// Opens a terminal window in `dir`.
pub fn open_terminal(dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        // `start` opens the default terminal (Windows Terminal on Windows 11).
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "cmd"]).current_dir(dir);
        hidden(cmd)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Couldn't open a terminal: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal"])
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Couldn't open a terminal: {e}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let mut candidates: Vec<String> = Vec::new();
        if let Ok(t) = std::env::var("TERMINAL")
            && !t.trim().is_empty()
        {
            candidates.push(t);
        }
        candidates.extend(
            [
                "x-terminal-emulator",
                "konsole",
                "gnome-terminal",
                "kgx",
                "alacritty",
                "kitty",
                "foot",
                "wezterm",
                "xfce4-terminal",
                "tilix",
                "xterm",
            ]
            .map(String::from),
        );
        for term in &candidates {
            if Command::new(term).current_dir(dir).spawn().is_ok() {
                return Ok(());
            }
        }
        Err("Couldn't find a terminal. Set the TERMINAL environment variable to your terminal's command.".into())
    }
}

/// A `file://` URI with everything but unreserved characters percent-encoded.
#[cfg(not(windows))]
fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(all(test, not(windows)))]
mod tests {
    #[test]
    fn file_uri_escapes() {
        assert_eq!(
            super::file_uri(std::path::Path::new("/home/joe/My Notes/ä.txt")),
            "file:///home/joe/My%20Notes/%C3%A4.txt"
        );
    }
}

/// Runs git in `dir` and returns what it printed, or its error message.
fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).args(args);
    #[cfg(windows)]
    let mut cmd = hidden(cmd);
    let out = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "Git isn't installed (or isn't on the PATH).".to_owned()
        } else {
            format!("Couldn't run git: {e}")
        }
    })?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }
}

/// The text of `file` in the last commit (Git HEAD) of the repository it is in.
pub fn git_head_text(file: &Path) -> Result<String, String> {
    let dir = file
        .parent()
        .ok_or_else(|| format!("{} has no folder", file.display()))?;
    let root = git(dir, &["rev-parse", "--show-toplevel"])
        .map_err(|_| format!("{} isn't in a Git repository.", file.display()))?;
    let root = std::path::PathBuf::from(String::from_utf8_lossy(&root).trim());
    let root = root.canonicalize().unwrap_or(root);
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let rel = file.strip_prefix(&root).map_err(|_| {
        format!(
            "{} isn't in the repository at {}.",
            file.display(),
            root.display()
        )
    })?;
    // Git wants forward slashes, also on Windows.
    let rel: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let spec = format!("HEAD:{}", rel.join("/"));
    let bytes = git(&root, &["show", &spec]).map_err(|e| {
        if e.contains("exists on disk, but not in") || e.contains("does not exist in") {
            "This file isn't in the last commit yet.".to_owned()
        } else if e.contains("invalid object name") || e.contains("bad revision") {
            "The repository has no commits yet.".to_owned()
        } else {
            e
        }
    })?;
    Ok(String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
}
