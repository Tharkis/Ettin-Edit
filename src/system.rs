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
