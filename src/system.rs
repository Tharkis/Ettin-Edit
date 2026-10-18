//! Handing things off to the desktop: the file manager and a terminal.

use std::path::Path;
use std::process::Command;

/// True when Ettin Edit runs inside a Flatpak sandbox.
pub fn in_flatpak() -> bool {
    static FLATPAK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FLATPAK.get_or_init(|| cfg!(target_os = "linux") && Path::new("/.flatpak-info").is_file())
}

/// `cmd` as it should run: unchanged, except inside a Flatpak, where it runs
/// on the computer itself through `flatpak-spawn --host`, since the sandbox
/// has no terminal, git or Node.js. Its folder and environment go along.
/// Standard input and output are set on the result, not on `cmd`.
pub fn on_host(cmd: Command) -> Command {
    if !in_flatpak() {
        return cmd;
    }
    host_command(&cmd)
}

fn host_command(cmd: &Command) -> Command {
    let mut host = Command::new("flatpak-spawn");
    // --watch-bus ends the program when Ettin Edit ends.
    host.args(["--host", "--watch-bus"]);
    if let Some(dir) = cmd.get_current_dir() {
        let mut arg = std::ffi::OsString::from("--directory=");
        arg.push(dir);
        host.arg(arg);
    }
    for (key, value) in cmd.get_envs() {
        if let Some(value) = value {
            let mut arg = std::ffi::OsString::from("--env=");
            arg.push(key);
            arg.push("=");
            arg.push(value);
            host.arg(arg);
        }
    }
    host.arg(cmd.get_program()).args(cmd.get_args());
    host
}

#[cfg(windows)]
fn hidden(mut cmd: Command) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Opens a web page in the default browser.
pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    let result = hidden({
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", url]);
        cmd
    })
    .spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(url).spawn();
    result
        .map(|_| ())
        .map_err(|e| t!("open-url-failed", url = url, error = e.to_string()))
}

/// Opens a folder in the system file manager.
pub fn open_folder(dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let result = Command::new("explorer").arg(dir).spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(dir).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(dir).spawn();
    result.map(|_| ()).map_err(|e| {
        t!(
            "open-url-failed",
            url = dir.display().to_string(),
            error = e.to_string()
        )
    })
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
            .map_err(|e| t!("explorer-failed", error = e.to_string()))
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
            None => Err(t!("no-parent-folder", path = file.display().to_string())),
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
            .map_err(|e| t!("terminal-failed", error = e.to_string()))
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal"])
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| t!("terminal-failed", error = e.to_string()))
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
            let mut cmd = Command::new(term);
            cmd.current_dir(dir);
            if on_host(cmd).spawn().is_ok() {
                return Ok(());
            }
        }
        Err(t!("no-terminal"))
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
    #[cfg(not(windows))]
    let mut cmd = on_host(cmd);
    let out = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            t!("git-missing")
        } else {
            t!("git-failed", error = e.to_string())
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
        .ok_or_else(|| t!("no-parent-folder", path = file.display().to_string()))?;
    let root = git(dir, &["rev-parse", "--show-toplevel"])
        .map_err(|_| t!("not-in-git", path = file.display().to_string()))?;
    let root = std::path::PathBuf::from(String::from_utf8_lossy(&root).trim());
    let root = root.canonicalize().unwrap_or(root);
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let rel = file.strip_prefix(&root).map_err(|_| {
        t!(
            "not-in-repository",
            path = file.display().to_string(),
            root = root.display().to_string()
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
            t!("git-not-committed")
        } else if e.contains("invalid object name") || e.contains("bad revision") {
            t!("git-no-commits")
        } else {
            e
        }
    })?;
    Ok(String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
}

#[cfg(test)]
mod host_tests {
    #[test]
    fn host_command_carries_folder_and_environment() {
        let mut cmd = std::process::Command::new("node");
        cmd.arg("main.js")
            .current_dir("/home/me/plugins/x")
            .env("NODE_PATH", "/home/me/sdk");
        let host = super::host_command(&cmd);
        assert_eq!(host.get_program(), "flatpak-spawn");
        let args: Vec<_> = host.get_args().map(|a| a.to_string_lossy()).collect();
        assert_eq!(
            args,
            [
                "--host",
                "--watch-bus",
                "--directory=/home/me/plugins/x",
                "--env=NODE_PATH=/home/me/sdk",
                "node",
                "main.js"
            ]
        );
    }
}
