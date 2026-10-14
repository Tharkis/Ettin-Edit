//! A running plugin: a child process that NixPad talks to over its standard
//! input and output, one JSON message per line. What it writes to standard
//! error goes to the plugin log.

use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use super::manifest::Runtime;

/// What a plugin's threads report back to the editor.
pub enum ProcEvent {
    /// One line of standard output: a JSON message.
    Message(String),
    /// One line of standard error.
    Log(String),
    /// Standard output closed: the plugin ended or crashed.
    Closed,
}

/// Wakes the UI when a plugin sends something.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

pub struct Proc {
    child: Child,
    to_plugin: Option<Sender<String>>,
    /// The program that runs the plugin, for the log.
    pub program: String,
}

/// The programs to try for a runtime, in order, each with its leading
/// arguments. A non-empty `custom` path replaces the defaults.
pub fn candidates(runtime: Runtime, custom: &str) -> Vec<(String, Vec<String>)> {
    let custom = custom.trim();
    if !custom.is_empty() {
        return vec![(custom.to_owned(), Vec::new())];
    }
    let plain = |p: &str| (p.to_owned(), Vec::new());
    match runtime {
        Runtime::Python if cfg!(windows) => {
            vec![
                plain("python"),
                ("py".to_owned(), vec!["-3".to_owned()]),
                plain("python3"),
            ]
        }
        Runtime::Python => vec![plain("python3"), plain("python")],
        Runtime::Node => vec![plain("node")],
    }
}

impl Proc {
    /// Starts `main` with the first program in `programs` that exists.
    /// `sdk` is added to the module search path so `import nixpad` works.
    pub fn spawn(
        runtime: Runtime,
        programs: &[(String, Vec<String>)],
        main: &Path,
        sdk: &Path,
        run: u64,
        events: Sender<(u64, ProcEvent)>,
        wake: Waker,
    ) -> io::Result<Self> {
        let dir = main.parent().unwrap_or(Path::new("."));
        let mut last_err = io::Error::new(io::ErrorKind::NotFound, "no program to run it");
        for (program, args) in programs {
            let mut cmd = Command::new(program);
            cmd.args(args);
            if runtime == Runtime::Python {
                cmd.arg("-u");
            }
            cmd.arg(main)
                .current_dir(dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env("NIXPAD_VERSION", env!("CARGO_PKG_VERSION"));
            match runtime {
                Runtime::Python => {
                    cmd.env("PYTHONPATH", prepend_path("PYTHONPATH", sdk))
                        .env("PYTHONIOENCODING", "utf-8")
                        .env("PYTHONUNBUFFERED", "1")
                        .env("PYTHONDONTWRITEBYTECODE", "1");
                }
                Runtime::Node => {
                    cmd.env("NODE_PATH", prepend_path("NODE_PATH", sdk));
                }
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                // No console window flashing up for each plugin.
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            match cmd.spawn() {
                Ok(child) => return Ok(Self::start(child, program, run, events, wake)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => last_err = e,
                Err(e) => return Err(e),
            }
        }
        Err(last_err)
    }

    fn start(
        mut child: Child,
        program: &str,
        run: u64,
        events: Sender<(u64, ProcEvent)>,
        wake: Waker,
    ) -> Self {
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (to_plugin, from_editor) = mpsc::channel::<String>();
        if let Some(stdin) = stdin {
            thread::spawn(move || write_loop(stdin, from_editor));
        }
        if let Some(out) = stdout {
            let events = events.clone();
            let wake = wake.clone();
            thread::spawn(move || {
                for line in BufReader::new(out).lines() {
                    let Ok(line) = line else { break };
                    if line.trim().is_empty() {
                        continue;
                    }
                    if events.send((run, ProcEvent::Message(line))).is_err() {
                        return;
                    }
                    wake();
                }
                let _ = events.send((run, ProcEvent::Closed));
                wake();
            });
        }
        if let Some(err) = stderr {
            thread::spawn(move || {
                for line in BufReader::new(err).lines() {
                    let Ok(line) = line else { break };
                    if events.send((run, ProcEvent::Log(line))).is_err() {
                        return;
                    }
                    wake();
                }
            });
        }
        Self {
            child,
            to_plugin: Some(to_plugin),
            program: program.to_owned(),
        }
    }

    /// Queues one message for the plugin. Never blocks the UI.
    pub fn send(&self, line: String) {
        if let Some(tx) = &self.to_plugin {
            let _ = tx.send(line);
        }
    }

    /// Closes the plugin's input, which tells it to finish.
    pub fn close_input(&mut self) {
        self.to_plugin = None;
    }

    /// The exit code once the process has ended.
    pub fn try_exit(&mut self) -> Option<Option<i32>> {
        self.child.try_wait().ok().flatten().map(|s| s.code())
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if self.try_exit().is_none() {
            self.kill();
        }
    }
}

fn write_loop(mut stdin: ChildStdin, lines: Receiver<String>) {
    for line in lines {
        if stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .and_then(|_| stdin.flush())
            .is_err()
        {
            return;
        }
    }
}

/// `dir` followed by the current value of the path variable `var`.
fn prepend_path(var: &str, dir: &Path) -> OsString {
    let mut paths = vec![dir.to_path_buf()];
    if let Some(old) = std::env::var_os(var) {
        paths.extend(std::env::split_paths(&old));
    }
    std::env::join_paths(paths).unwrap_or_else(|_| dir.as_os_str().to_owned())
}
