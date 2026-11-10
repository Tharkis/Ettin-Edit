//! Running shell commands with text on their input: Transform > Filter
//! Through Command and the Command step of recipes.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

/// A finished command's standard output and error.
pub type Output = Receiver<(Vec<u8>, Vec<u8>)>;

/// The shell set in Preferences, like `bash -c`; empty uses the system's.
static SHELL: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

pub fn set_shell(shell: &str) {
    *SHELL.write().unwrap_or_else(|e| e.into_inner()) = shell.trim().to_owned();
}

/// The shell's program and the arguments before the command, from a
/// setting like `bash -c` or `"C:\Program Files\PowerShell\7\pwsh.exe" -Command`.
pub fn shell_words(setting: &str) -> Option<(String, Vec<String>)> {
    let setting = setting.trim();
    if setting.is_empty() {
        return None;
    }
    let (program, rest) = match setting.strip_prefix('"') {
        Some(quoted) => {
            let end = quoted.find('"')?;
            (quoted[..end].to_owned(), &quoted[end + 1..])
        }
        None => match setting.split_once(char::is_whitespace) {
            Some((p, rest)) => (p.to_owned(), rest),
            None => (setting.to_owned(), ""),
        },
    };
    Some((
        program,
        rest.split_whitespace().map(str::to_owned).collect(),
    ))
}

/// `command` as the shell runs it, in `dir`.
pub fn shell(command: &str, dir: Option<&std::path::Path>) -> Command {
    let custom = shell_words(&SHELL.read().unwrap_or_else(|e| e.into_inner()));
    let mut cmd = match custom {
        Some((program, args)) => {
            let mut cmd = Command::new(program);
            cmd.args(args).arg(command);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000);
            }
            cmd
        }
        None => system_shell(command),
    };
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    crate::system::on_host(cmd)
}

/// `sh -c` (`cmd /C` on Windows).
fn system_shell(command: &str) -> Command {
    #[cfg(windows)]
    let cmd = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = Command::new("cmd");
        cmd.arg("/S").arg("/C").raw_arg(format!("\"{command}\""));
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    };
    #[cfg(not(windows))]
    let cmd = {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    };
    cmd
}

/// Starts `command` with `input` on its standard input.
pub fn spawn(
    command: &str,
    dir: Option<&std::path::Path>,
    file: Option<&std::path::Path>,
    input: String,
) -> Result<(Child, Output), String> {
    let mut cmd = shell(command, dir);
    if let Some(file) = file {
        cmd.env("ETTIN_FILE", file);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().expect("piped");
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    // Writing and reading on their own threads, so a command that answers
    // before it has read everything can't block either side.
    std::thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
    });
    let errors = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        let err = errors.join().unwrap_or_default();
        let _ = tx.send((out, err));
    });
    Ok((child, rx))
}

/// A command's output as editor text: line breaks as `\n`, and no final
/// line break added to text that had none.
pub fn pipe_output(input: &str, bytes: &[u8]) -> String {
    let mut out = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
    if !input.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Why [`run`] gave no output.
#[derive(Debug, PartialEq)]
pub enum RunError {
    /// It couldn't start.
    Start(String),
    /// It ended with an error: the exit code (if any) and standard error.
    Failed(Option<i32>, String),
    /// `cancel` was set and the command was stopped.
    Stopped,
}

/// Runs `command` with `input` and waits for it, checking `cancel` while it
/// runs. Returns its output shaped like the input (see [`pipe_output`]).
pub fn run(
    command: &str,
    dir: Option<&Path>,
    file: Option<&Path>,
    input: &str,
    cancel: Option<&AtomicBool>,
) -> Result<String, RunError> {
    let (mut child, rx) = spawn(command, dir, file, input.to_owned()).map_err(RunError::Start)?;
    loop {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RunError::Stopped);
        }
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok((out, err)) => {
                let status = child.wait().ok();
                if status.is_some_and(|s| s.success()) {
                    return Ok(pipe_output(input, &out));
                }
                let stderr = String::from_utf8_lossy(&err).trim().to_owned();
                return Err(RunError::Failed(status.and_then(|s| s.code()), stderr));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::Failed(None, String::new()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_shell_setting() {
        assert_eq!(shell_words("  "), None);
        assert_eq!(
            shell_words("bash -c"),
            Some(("bash".to_owned(), vec!["-c".to_owned()]))
        );
        assert_eq!(
            shell_words(r#""C:\Program Files\pwsh.exe" -NoProfile -Command"#),
            Some((
                r"C:\Program Files\pwsh.exe".to_owned(),
                vec!["-NoProfile".to_owned(), "-Command".to_owned()]
            ))
        );
    }

    #[test]
    fn output_keeps_the_input_shape() {
        assert_eq!(pipe_output("b\na", b"a\nb\n"), "a\nb");
        assert_eq!(pipe_output("b\na\n", b"a\r\nb\r\n"), "a\nb\n");
    }

    #[cfg(unix)]
    #[test]
    fn runs_a_command() {
        let (mut child, rx) = spawn("tr a-z A-Z", None, None, "hello\n".into()).unwrap();
        let (out, _) = rx.recv().unwrap();
        assert!(child.wait().unwrap().success());
        assert_eq!(out, b"HELLO\n");
        let (mut child, rx) = spawn("echo oops >&2; exit 3", None, None, String::new()).unwrap();
        let (_, err) = rx.recv().unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(3));
        assert_eq!(err, b"oops\n");
    }

    #[cfg(unix)]
    #[test]
    fn runs_and_waits() {
        assert_eq!(run("tr a-z A-Z", None, None, "hi", None), Ok("HI".into()));
        assert_eq!(
            run("echo bad >&2; exit 2", None, None, "", None),
            Err(RunError::Failed(Some(2), "bad".into()))
        );
        let stop = AtomicBool::new(true);
        assert_eq!(
            run("sleep 5", None, None, "", Some(&stop)),
            Err(RunError::Stopped)
        );
    }
}
