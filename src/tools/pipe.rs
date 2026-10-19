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

/// `command` as the system shell runs it, in `dir`.
pub fn shell(command: &str, dir: Option<&std::path::Path>) -> Command {
    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = Command::new("cmd");
        cmd.arg("/S").arg("/C").raw_arg(format!("\"{command}\""));
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    };
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    crate::system::on_host(cmd)
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
