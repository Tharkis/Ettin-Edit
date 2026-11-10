//! Running plugin transforms without the editor's plugin host: for recipes
//! replayed from the command line (`--recipe`) and for recipes running in
//! the background in the app. Each plugin a recipe uses is started once,
//! answers its transforms, and is stopped when the recipe is done.
//!
//! Only plugins the user allowed in Ettin Edit run; their calls about
//! documents and windows get an error or an empty answer, since a transform
//! only works on the text it is given.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use super::manifest::{self, Manifest};
use super::process::{self, Proc, ProcEvent};
use super::{FAILED, METHOD_NOT_FOUND, ReplyError, data, notification, response};
use crate::settings::PluginPrefs;

/// How long a transform may take before it is given up (seconds).
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// Runs a plugin's transform on some text: what recipe steps of type
/// `plugin` need from where they run.
pub trait PluginTransforms {
    fn run_transform(
        &self,
        plugin: &str,
        transform: &str,
        text: &str,
        options: &Map<String, Value>,
        cancel: Option<&AtomicBool>,
    ) -> Result<String, String>;
}

/// The message for a transform's error, with where in the text it is.
pub fn reply_error_text(e: &ReplyError) -> String {
    match (e.line, e.column) {
        (Some(line), Some(column)) => t!(
            "plugin-transform-error-at",
            message = e.message.clone(),
            line = line,
            column = column
        ),
        (Some(line), None) => t!(
            "plugin-transform-error-line",
            message = e.message.clone(),
            line = line
        ),
        _ => e.message.clone(),
    }
}

/// Writes the SDK modules plugins import into `<plugins>/.sdk`, when
/// they're missing or old. Returns the folder.
pub fn install_sdk(plugins: &Path) -> Result<PathBuf, String> {
    let sdk = plugins.join(".sdk");
    std::fs::create_dir_all(&sdk).map_err(|e| {
        t!(
            "make-folder-failed",
            path = sdk.display().to_string(),
            error = e.to_string()
        )
    })?;
    for (name, text) in super::SDK_FILES {
        let path = sdk.join(name);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text) {
            std::fs::write(&path, text).map_err(|e| {
                t!(
                    "write-failed",
                    path = path.display().to_string(),
                    error = e.to_string()
                )
            })?;
        }
    }
    Ok(sdk)
}

/// A started plugin, waiting for transforms to run.
struct Session {
    proc: Proc,
    rx: Receiver<(u64, ProcEvent)>,
    transforms: Vec<String>,
    ready: bool,
    closed: bool,
    next_id: u64,
    settings: Map<String, Value>,
    data_dir: Option<PathBuf>,
    options: Vec<manifest::PluginOption>,
    /// What it wrote to standard error and its log, for error messages.
    log: Vec<String>,
}

impl Drop for Session {
    fn drop(&mut self) {
        self.proc.send(notification("shutdown", json!({})));
        self.proc.close_input();
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline && self.proc.try_exit().is_none() {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Session {
    /// Handles one message from the plugin. Returns a reply to `waiting`
    /// when that is what came.
    fn handle(
        &mut self,
        event: ProcEvent,
        waiting: Option<u64>,
    ) -> Option<Result<Value, ReplyError>> {
        let line = match event {
            ProcEvent::Log(l) => {
                self.log.push(l);
                return None;
            }
            ProcEvent::Closed => {
                self.closed = true;
                return None;
            }
            ProcEvent::Message(l) => l,
        };
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            return None;
        };
        let id = msg.get("id").filter(|v| !v.is_null()).cloned();
        let Some(method) = msg.get("method").and_then(Value::as_str) else {
            if id.and_then(|v| v.as_u64()) == waiting && waiting.is_some() {
                return Some(match msg.get("error") {
                    Some(e) if !e.is_null() => Err(ReplyError::from_json(e)),
                    _ => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
                });
            }
            return None;
        };
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let text = |key: &str| params.get(key).and_then(Value::as_str).unwrap_or_default();
        let answer: Result<Value, (i64, String)> = match method {
            "transforms.register" => {
                let id = match text("id") {
                    "" => text("title"),
                    id => id,
                }
                .to_owned();
                self.transforms.push(id.clone());
                Ok(json!({ "id": id }))
            }
            "ready" => {
                self.ready = true;
                Ok(Value::Null)
            }
            "log" => {
                self.log.push(text("text").to_owned());
                Ok(Value::Null)
            }
            "error" => {
                self.log.push(text("message").to_owned());
                Ok(Value::Null)
            }
            "settings.get" => Ok(Value::Object(self.settings.clone())),
            "settings.set" => {
                if let (Some(Value::Object(values)), Some(dir)) =
                    (params.get("values"), &self.data_dir)
                {
                    for (k, v) in values {
                        self.settings.insert(k.clone(), v.clone());
                    }
                    let _ = data::save_settings(dir, &self.settings);
                    self.settings = data::load_settings(dir, &self.options);
                }
                Ok(Value::Object(self.settings.clone()))
            }
            "commands.register" => Ok(json!({ "shortcut": false })),
            "events.subscribe" | "panel.show" | "panel.close" | "status.set" | "marks.set"
            | "marks.clear" | "ui.message" | "clipboard.set" => Ok(Value::Null),
            "ui.prompt" | "ui.choose" => Ok(Value::Null),
            "app.language" => Ok(json!(crate::i18n::current())),
            m if m.starts_with("documents.")
                || m.starts_with("text.")
                || m.starts_with("selection.")
                || m.starts_with("editor.") =>
            {
                Err((FAILED, t!("plugin-no-documents-here")))
            }
            other => Err((
                METHOD_NOT_FOUND,
                format!("Ettin Edit has no method \"{other}\""),
            )),
        };
        if let Some(id) = id {
            self.proc.send(response(id, answer));
        }
        None
    }

    /// Waits for the next message, up to `until`. False when time is up,
    /// the plugin ended or the run was cancelled.
    fn next(&mut self, until: Instant, cancel: Option<&AtomicBool>) -> Option<ProcEvent> {
        loop {
            if self.closed || cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                return None;
            }
            let left = until.checked_duration_since(Instant::now())?;
            match self.rx.recv_timeout(left.min(Duration::from_millis(50))) {
                Ok((_, ev)) => return Some(ev),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.closed = true;
                    return None;
                }
            }
        }
    }

    /// The last thing the plugin logged, to explain a failure.
    fn last_error(&self) -> String {
        self.log
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_owned())
            .unwrap_or_default()
    }
}

/// Starts plugins as recipes need them. Plugins run with the same Python
/// and Node and permissions as in the editor.
pub struct Headless {
    plugins_dir: Option<PathBuf>,
    data_root: Option<PathBuf>,
    prefs: BTreeMap<String, PluginPrefs>,
    python: String,
    node: String,
    sessions: RefCell<HashMap<String, Session>>,
}

impl Headless {
    pub fn new(
        plugins_dir: Option<PathBuf>,
        data_root: Option<PathBuf>,
        prefs: BTreeMap<String, PluginPrefs>,
        python: String,
        node: String,
    ) -> Self {
        Self {
            plugins_dir,
            data_root,
            prefs,
            python,
            node,
            sessions: RefCell::new(HashMap::new()),
        }
    }

    /// With the plugins folder and settings Ettin Edit uses.
    pub fn from_settings(settings: &crate::settings::Settings) -> Self {
        Self::new(
            super::plugins_dir(),
            data::data_root(),
            settings.plugins.clone(),
            settings.python_path.clone(),
            settings.node_path.clone(),
        )
    }

    /// Checks the plugin is installed and allowed, and starts it.
    fn start(&self, id: &str) -> Result<Session, String> {
        let root = self
            .plugins_dir
            .as_ref()
            .ok_or_else(|| t!("no-plugins-folder"))?;
        let dir = root.join(id);
        if !dir.is_dir() || id.is_empty() || id.starts_with('.') {
            return Err(t!("recipe-plugin-missing", plugin = id));
        }
        let m: Manifest = manifest::load(&dir).map_err(|e| format!("{id}: {e}"))?;
        let pref = self.prefs.get(id).cloned().unwrap_or_default();
        let allowed = !pref.disabled
            && match &pref.approved {
                Some(_) if pref.trust_changes => true,
                Some(a) => *a == manifest::fingerprint(&dir),
                None => false,
            };
        if !allowed {
            return Err(t!("recipe-plugin-not-allowed", plugin = m.name.clone()));
        }
        let sdk = install_sdk(root)?;
        let custom = match m.runtime {
            manifest::Runtime::Python => &self.python,
            manifest::Runtime::Node => &self.node,
        };
        let programs = process::candidates(m.runtime, custom);
        let (tx, rx) = mpsc::channel();
        let proc = Proc::spawn(
            m.runtime,
            &programs,
            &m.main,
            &sdk,
            1,
            tx,
            std::sync::Arc::new(|| {}),
        )
        .map_err(|e| {
            let program = programs.first().map(|p| p.0.as_str()).unwrap_or("?");
            if e.kind() == std::io::ErrorKind::NotFound {
                t!(
                    "runtime-missing",
                    runtime = m.runtime.label(),
                    program = program
                )
            } else {
                t!(
                    "plugin-start-failed",
                    program = program,
                    error = e.to_string()
                )
            }
        })?;
        let data_dir = self.data_root.as_ref().map(|d| d.join(id));
        let settings = data_dir
            .as_deref()
            .map(|d| data::load_settings(d, &m.options))
            .unwrap_or_default();
        proc.send(notification(
            "initialize",
            json!({
                "plugin": id,
                "ettin": env!("CARGO_PKG_VERSION"),
                "api": super::API_VERSION,
                "language": crate::i18n::current(),
                "settings": settings,
                "dataDir": data_dir,
                "headless": true,
            }),
        ));
        Ok(Session {
            proc,
            rx,
            transforms: Vec::new(),
            ready: false,
            closed: false,
            next_id: 1,
            settings,
            data_dir,
            options: m.options,
            log: Vec::new(),
        })
    }
}

impl PluginTransforms for Headless {
    fn run_transform(
        &self,
        plugin: &str,
        transform: &str,
        text: &str,
        options: &Map<String, Value>,
        cancel: Option<&AtomicBool>,
    ) -> Result<String, String> {
        let mut sessions = self.sessions.borrow_mut();
        if !sessions.contains_key(plugin) {
            let s = self.start(plugin)?;
            sessions.insert(plugin.to_owned(), s);
        }
        let s = sessions.get_mut(plugin).expect("just started");
        let until = Instant::now() + TIMEOUT;
        let stopped = |s: &Session| {
            if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                t!("recipe-stopped")
            } else if s.closed {
                let why = s.last_error();
                if why.is_empty() {
                    t!("plugin-ended-during-transform", plugin = plugin)
                } else {
                    format!(
                        "{} {why}",
                        t!("plugin-ended-during-transform", plugin = plugin)
                    )
                }
            } else {
                t!(
                    "plugin-transform-timeout",
                    seconds = TIMEOUT.as_secs(),
                    plugin = plugin
                )
            }
        };
        // Wait until it has added the transform, or says it has added all.
        while !s.transforms.iter().any(|t| t == transform) {
            if s.ready {
                return Err(t!(
                    "recipe-plugin-no-transform",
                    plugin = plugin,
                    transform = transform
                ));
            }
            match s.next(until, cancel) {
                Some(ev) => {
                    s.handle(ev, None);
                }
                None => {
                    let e = stopped(s);
                    sessions.remove(plugin);
                    return Err(e);
                }
            }
        }
        let id = s.next_id;
        s.next_id += 1;
        s.proc.send(
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "transform.run",
                "params": { "id": transform, "text": text, "options": options },
            })
            .to_string(),
        );
        loop {
            match s.next(until, cancel) {
                Some(ev) => match s.handle(ev, Some(id)) {
                    Some(Ok(v)) => return transform_text(&v),
                    Some(Err(e)) => return Err(reply_error_text(&e)),
                    None => {}
                },
                None => {
                    let e = stopped(s);
                    sessions.remove(plugin);
                    return Err(e);
                }
            }
        }
    }
}

/// The text in a transform's answer: `{"text": "…"}` (or just the text).
pub fn transform_text(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Object(o) => match o.get("text") {
            Some(Value::String(s)) => Ok(s.clone()),
            _ => Err(t!("plugin-transform-no-text")),
        },
        _ => Err(t!("plugin-transform-no-text")),
    }
}

/// Runs real plugins through the SDKs, the way `--recipe` does.
#[cfg(test)]
mod tests {
    use super::*;

    fn have(program: &str) -> bool {
        std::process::Command::new(program)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn setup(name: &str, files: &[(&str, &str)], allow: bool) -> (PathBuf, Headless) {
        let root =
            std::env::temp_dir().join(format!("ettin-headless-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("plugins").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        for (file, text) in files {
            std::fs::write(dir.join(file), text).unwrap();
        }
        let mut prefs = BTreeMap::new();
        if allow {
            prefs.insert(
                name.to_owned(),
                PluginPrefs {
                    approved: Some(manifest::fingerprint(&dir)),
                    ..Default::default()
                },
            );
        }
        let h = Headless::new(
            Some(root.join("plugins")),
            Some(root.join("data")),
            prefs,
            String::new(),
            String::new(),
        );
        (root, h)
    }

    #[test]
    fn python_transform_runs_headless() {
        if !have("python3") {
            eprintln!("python3 isn't installed; skipped");
            return;
        }
        let main = r#"
import ettin

@ettin.transform("Shout", options={"mark": "!"})
def shout(text, options):
    return text.upper() + options["mark"]

@ettin.transform("Fail")
def fail(text, options):
    raise ettin.TransformError("bad input", line=2, column=3)

ettin.run()
"#;
        let (root, h) = setup(
            "py-shout",
            &[("main.py", main), ("plugin.json", "{}")],
            true,
        );
        let mut opts = Map::new();
        opts.insert("mark".into(), json!("?"));
        assert_eq!(
            h.run_transform("py-shout", "Shout", "hi", &opts, None),
            Ok("HI?".to_owned())
        );
        // The same process answers again.
        assert_eq!(
            h.run_transform("py-shout", "Shout", "yo", &Map::new(), None),
            Ok("YO!".to_owned())
        );
        let err = h
            .run_transform("py-shout", "Fail", "x", &Map::new(), None)
            .unwrap_err();
        assert!(
            err.contains("bad input") && err.contains('2') && err.contains('3'),
            "{err}"
        );
        let err = h
            .run_transform("py-shout", "Nope", "x", &Map::new(), None)
            .unwrap_err();
        assert!(err.contains("Nope"), "{err}");
        drop(h);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn js_transform_runs_headless() {
        if !have("node") {
            eprintln!("node isn't installed; skipped");
            return;
        }
        let main = r#"
const ettin = require('ettin');
ettin.transform('Upper Words', {}, (text) => text.replace(/\b\w/g, (c) => c.toUpperCase()));
ettin.transform('Wrap', { left: '[' }, async (text, opts) => opts.left + text + ']', { id: 'wrap' });
"#;
        let (root, h) = setup(
            "js-words",
            &[("main.js", main), ("plugin.json", "{}")],
            true,
        );
        assert_eq!(
            h.run_transform(
                "js-words",
                "Upper Words",
                "hello big world",
                &Map::new(),
                None
            ),
            Ok("Hello Big World".to_owned())
        );
        let mut opts = Map::new();
        opts.insert("left".into(), json!("<"));
        assert_eq!(
            h.run_transform("js-words", "wrap", "x", &opts, None),
            Ok("<x]".to_owned())
        );
        drop(h);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plugins_must_be_installed_and_allowed() {
        let (root, h) = setup(
            "not-allowed",
            &[("main.py", ""), ("plugin.json", "{}")],
            false,
        );
        let err = h
            .run_transform("not-allowed", "x", "", &Map::new(), None)
            .unwrap_err();
        assert!(err.contains("not-allowed"), "{err}");
        let err = h
            .run_transform("missing", "x", "", &Map::new(), None)
            .unwrap_err();
        assert!(err.contains("missing"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
