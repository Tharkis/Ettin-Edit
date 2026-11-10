//! Plugins written in Python or JavaScript.
//!
//! Each plugin is a folder in the plugins folder with a `plugin.json` and a
//! script. Ettin Edit runs the script with the installed Python or Node as a
//! separate process and talks to it with JSON-RPC 2.0 messages, one per line,
//! over its standard input and output. A plugin that crashes or hangs can't
//! take the editor down.
//!
//! The host here starts and stops plugins and handles the calls that are
//! about plugins themselves (registering commands, panels, logging). Calls
//! about documents are handed to the app (see `app/plugins.rs`).

pub mod data;
pub mod headless;
pub mod manifest;
pub mod new_plugin;
pub mod panel;
pub mod process;
pub mod shortcut;
pub mod store;
pub mod ui;

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use egui::KeyboardShortcut;
use serde_json::{Map, Value, json};

use crate::actions::Action;
use crate::settings::PluginPrefs;
use manifest::{Discovered, Manifest};
use panel::{Item, Panel, PanelEvent, Position};
use process::{Proc, ProcEvent, Waker};

/// The version of the plugin API, sent to plugins at start. 2 (Ettin Edit
/// 2.3) added transforms, settings, marks, status items and `choose`.
pub const API_VERSION: u32 = 2;

pub const SDK_PYTHON: &str = include_str!("../../plugins/sdk/ettin.py");
pub const SDK_JS: &str = include_str!("../../plugins/sdk/ettin.js");
/// Type stubs for editors, copied next to new plugins.
pub const SDK_PYTHON_STUBS: &str = include_str!("../../plugins/sdk/ettin.pyi");
pub const SDK_JS_TYPES: &str = include_str!("../../plugins/sdk/ettin.d.ts");
/// The files written to the plugins folder's `.sdk` folder, which is on the
/// plugins' module path.
pub const SDK_FILES: [(&str, &str); 6] = [
    ("ettin.py", SDK_PYTHON),
    ("ettin.js", SDK_JS),
    ("ettin.pyi", SDK_PYTHON_STUBS),
    ("ettin.d.ts", SDK_JS_TYPES),
    ("nixpad.py", SDK_PYTHON_OLD_NAME),
    ("nixpad.js", SDK_JS_OLD_NAME),
];
/// Plugins written before the rename import `nixpad`; these point it at `ettin`.
const SDK_PYTHON_OLD_NAME: &str = "import sys\nimport ettin\n\nsys.modules[__name__] = ettin\n";
const SDK_JS_OLD_NAME: &str = "module.exports = require('./ettin');\n";

/// The sample plugins: (folder, [(file, contents)]).
pub const SAMPLES: &[(&str, &[(&str, &str)])] = &[
    (
        "text-stats",
        &[
            (
                "plugin.json",
                include_str!("../../plugins/samples/text-stats/plugin.json"),
            ),
            (
                "main.py",
                include_str!("../../plugins/samples/text-stats/main.py"),
            ),
        ],
    ),
    (
        "markdown-outline",
        &[
            (
                "plugin.json",
                include_str!("../../plugins/samples/markdown-outline/plugin.json"),
            ),
            (
                "main.js",
                include_str!("../../plugins/samples/markdown-outline/main.js"),
            ),
        ],
    ),
    (
        "hl7-viewer",
        &[
            (
                "plugin.json",
                include_str!("../../plugins/samples/hl7-viewer/plugin.json"),
            ),
            (
                "main.py",
                include_str!("../../plugins/samples/hl7-viewer/main.py"),
            ),
            (
                "hl7.py",
                include_str!("../../plugins/samples/hl7-viewer/hl7.py"),
            ),
            (
                "hl7_defs.py",
                include_str!("../../plugins/samples/hl7-viewer/hl7_defs.py"),
            ),
            (
                "test_hl7.py",
                include_str!("../../plugins/samples/hl7-viewer/test_hl7.py"),
            ),
            (
                "sample.hl7",
                include_str!("../../plugins/samples/hl7-viewer/sample.hl7"),
            ),
        ],
    ),
    (
        "markdown-preview",
        &[
            (
                "plugin.json",
                include_str!("../../plugins/samples/markdown-preview/plugin.json"),
            ),
            (
                "main.py",
                include_str!("../../plugins/samples/markdown-preview/main.py"),
            ),
            (
                "preview.py",
                include_str!("../../plugins/samples/markdown-preview/preview.py"),
            ),
            (
                "test_preview.py",
                include_str!("../../plugins/samples/markdown-preview/test_preview.py"),
            ),
        ],
    ),
];

const MAX_LOG: usize = 2000;
const MAX_COMMANDS: usize = 200;
const MAX_TRANSFORMS: usize = 200;
const MAX_STATUS_ITEMS: usize = 4;
/// Seconds to wait for a plugin's last error output before reporting why
/// it stopped.
const FAILURE_DELAY: f64 = 0.3;
/// How often plugins that reload on change are checked (seconds), and how
/// long their files must stay unchanged before the reload.
const WATCH_EVERY: f64 = 0.2;
const WATCH_SETTLE: f64 = 0.5;
/// How long a stopping plugin gets to finish before it is killed (seconds).
const STOP_GRACE: f64 = 1.5;

/// Where plugins live: `~/.config/EttinEdit/plugins` on Linux.
pub fn plugins_dir() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("plugins"))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Running,
    Stopped,
    /// Turned off by the user.
    Disabled,
    /// New or changed since the user allowed it; waiting for them to answer.
    NeedsPermission,
    /// It couldn't start, or it ended on its own.
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub shortcut: Option<KeyboardShortcut>,
    /// A Phosphor icon name or a PNG file in the plugin's folder, for the toolbar.
    pub icon: Option<String>,
}

/// A transform a plugin added: a function from text to text that Ettin Edit
/// runs on each selection, previews and records into recipes.
#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub id: String,
    pub title: String,
    /// Its options and their defaults; Ettin Edit asks for them before it
    /// runs from a menu.
    pub options: Map<String, Value>,
    /// Languages it suits (lower case); it is on the right-click menu of
    /// tabs in these. Empty: every tab, from the Transform menu only.
    pub languages: Vec<String>,
}

impl Transform {
    /// True when the right-click menu of a tab in `language` offers it.
    pub fn suits(&self, language: &str) -> bool {
        let language = language.to_lowercase();
        self.languages.contains(&language)
    }
}

/// A running plugin's transform, as menus and recipes show it.
#[derive(Clone, Debug, PartialEq)]
pub struct KnownTransform {
    pub plugin: String,
    pub plugin_name: String,
    pub id: String,
    pub title: String,
    pub options: Map<String, Value>,
}

static KNOWN: std::sync::RwLock<Vec<KnownTransform>> = std::sync::RwLock::new(Vec::new());

/// The transforms of the running plugins, for the Transform menu and the
/// recipe window (see [`crate::transforms`]).
pub fn known_transforms() -> Vec<KnownTransform> {
    KNOWN.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The title of a plugin transform and its plugin's name, if it's running.
pub fn transform_title(plugin: &str, transform: &str) -> Option<(String, String)> {
    KNOWN
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|t| t.plugin == plugin && t.id == transform)
        .map(|t| (t.title.clone(), t.plugin_name.clone()))
}

/// Text a plugin shows on the status bar.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusItem {
    pub plugin: String,
    pub id: String,
    pub text: String,
    pub tooltip: Option<String>,
    /// One of the plugin's commands, run when the item is clicked.
    pub command: Option<String>,
}

/// A message for the user about a plugin. `open` is a file and line (from
/// 1) to offer to open, such as where an error happened.
#[derive(Clone, Debug, PartialEq)]
pub struct HostNotice {
    pub text: String,
    pub open: Option<(PathBuf, usize)>,
}

impl From<String> for HostNotice {
    fn from(text: String) -> Self {
        Self { text, open: None }
    }
}

/// How many answers are kept waiting to be taken.
const MAX_REPLIES: usize = 256;

/// An answer to a request Ettin Edit sent a plugin.
#[derive(Clone, Debug)]
pub struct Reply {
    pub plugin: String,
    pub id: u64,
    pub result: Result<Value, ReplyError>,
}

/// A plugin's error answer. `line` and `column` (from 1) say where in the
/// text the problem is, when the plugin knows.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplyError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl ReplyError {
    fn from_json(error: &Value) -> Self {
        let data = error.get("data").unwrap_or(&Value::Null);
        let number = |key: &str| {
            data.get(key)
                .or_else(|| error.get(key))
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .filter(|n| *n > 0)
        };
        Self {
            message: error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("error")
                .to_owned(),
            line: number("line"),
            column: number("column"),
        }
    }
}

pub struct Plugin {
    pub id: String,
    pub dir: PathBuf,
    pub manifest: Result<Manifest, String>,
    pub status: Status,
    /// The program that runs it, once started.
    pub program: Option<String>,
    pub commands: Vec<Command>,
    pub transforms: Vec<Transform>,
    /// It said it has added its commands and transforms.
    pub ready: bool,
    /// Its settings: saved values with the defaults of its options.
    pub settings: Map<String, Value>,
    proc: Option<Proc>,
    /// Which start of the plugin `proc` is; messages from older ones are dropped.
    run: u64,
    /// The start before `run` was reset, so its last error output still
    /// reaches the log.
    last_run: u64,
    events: HashSet<String>,
    /// While it reloads on change: the files' stamp, and when they last
    /// changed if a reload is waiting.
    watch: Option<(u64, Option<f64>)>,
    /// The log's line count when it last started.
    log_from: u64,
}

impl Plugin {
    pub fn name(&self) -> &str {
        self.manifest
            .as_ref()
            .map(|m| m.name.as_str())
            .unwrap_or(&self.id)
    }

    pub fn is_running(&self) -> bool {
        self.status == Status::Running
    }

    pub fn options(&self) -> &[manifest::PluginOption] {
        self.manifest.as_ref().map_or(&[], |m| m.options.as_slice())
    }

    pub fn transform(&self, id: &str) -> Option<&Transform> {
        self.transforms.iter().find(|t| t.id == id)
    }

    fn new(Discovered { id, dir, manifest }: Discovered) -> Self {
        Plugin {
            id,
            dir,
            status: match &manifest {
                Ok(_) => Status::Stopped,
                Err(e) => Status::Failed(e.clone()),
            },
            manifest,
            program: None,
            commands: Vec::new(),
            transforms: Vec::new(),
            ready: false,
            settings: Map::new(),
            proc: None,
            run: 0,
            last_run: 0,
            events: HashSet::new(),
            watch: None,
            log_from: 0,
        }
    }

    /// Forgets what the running plugin added.
    fn clear(&mut self) {
        self.commands.clear();
        self.transforms.clear();
        self.events.clear();
        self.ready = false;
    }
}

/// A call from a plugin that the app answers.
pub struct Call {
    pub plugin: String,
    pub run: u64,
    /// `None` for a notification, which gets no answer.
    pub id: Option<Value>,
    pub method: String,
    pub params: Value,
}

pub struct LogLine {
    /// Counts up from the first line, so a plugin's lines since it started
    /// can be found.
    pub seq: u64,
    pub time: String,
    pub plugin: String,
    pub text: String,
    pub error: bool,
}

/// An error answer: JSON-RPC code and message.
pub type CallError = (i64, String);

pub const INVALID_PARAMS: i64 = -32602;
pub const METHOD_NOT_FOUND: i64 = -32601;
/// The call was fine but couldn't be done (no such document, read-only...).
pub const FAILED: i64 = -32000;

pub struct PluginHost {
    pub dir: Option<PathBuf>,
    pub plugins: Vec<Plugin>,
    pub panels: Vec<Panel>,
    pub log: VecDeque<LogLine>,
    /// Plugins waiting for the user to allow them to run.
    pub prompts: VecDeque<String>,
    /// Messages to show the user briefly.
    pub notices: Vec<HostNotice>,
    /// Where plugins' settings and files are kept.
    pub data_dir: Option<PathBuf>,
    /// Answers to requests sent with [`PluginHost::request`], for the app.
    pub replies: Vec<Reply>,
    next_request: u64,
    /// Plugins' status bar items, in the order they were added.
    pub status: Vec<StatusItem>,
    /// Plugins that stopped on their own: (id, run, when to report it).
    failures: Vec<(String, u64, f64)>,
    last_watch: f64,
    log_seq: u64,
    events_tx: Sender<(u64, ProcEvent)>,
    events_rx: Receiver<(u64, ProcEvent)>,
    /// Events taken from the channel while waiting, not handled yet.
    stash: VecDeque<(u64, ProcEvent)>,
    /// Plugins that were asked to stop, with when to kill them.
    ending: Vec<(Proc, f64)>,
    next_run: u64,
    wake: Waker,
    pub scanned: bool,
}

impl PluginHost {
    pub fn new(wake: Waker) -> Self {
        let (events_tx, events_rx) = mpsc::channel();
        Self {
            dir: plugins_dir(),
            plugins: Vec::new(),
            panels: Vec::new(),
            log: VecDeque::new(),
            prompts: VecDeque::new(),
            notices: Vec::new(),
            data_dir: data::data_root(),
            replies: Vec::new(),
            next_request: 1,
            status: Vec::new(),
            failures: Vec::new(),
            last_watch: f64::NEG_INFINITY,
            log_seq: 0,
            events_tx,
            events_rx,
            stash: VecDeque::new(),
            ending: Vec::new(),
            next_run: 1,
            wake,
            scanned: false,
        }
    }

    pub fn log(&mut self, plugin: &str, text: impl Into<String>, error: bool) {
        let text = text.into();
        if self.log.len() >= MAX_LOG {
            self.log.pop_front();
        }
        self.log_seq += 1;
        self.log.push_back(LogLine {
            seq: self.log_seq,
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            plugin: plugin.to_owned(),
            text,
            error,
        });
    }

    /// Writes the SDK modules plugins import, when they're missing or old.
    fn install_sdk(&mut self) -> Option<PathBuf> {
        let dir = self.dir.clone()?;
        match headless::install_sdk(&dir) {
            Ok(sdk) => Some(sdk),
            Err(e) => {
                self.log("Ettin Edit", e, true);
                None
            }
        }
    }

    /// Stops every plugin, finds plugins again and starts the ones that are
    /// enabled and allowed.
    pub fn reload(
        &mut self,
        prefs: &BTreeMap<String, PluginPrefs>,
        python: &str,
        node: &str,
        now: f64,
    ) {
        for i in 0..self.plugins.len() {
            self.stop(i, now);
        }
        self.prompts.clear();
        let found = match &self.dir {
            Some(dir) => {
                let _ = fs::create_dir_all(dir);
                manifest::discover(dir)
            }
            None => Vec::new(),
        };
        self.plugins = found.into_iter().map(Plugin::new).collect();
        self.scanned = true;
        for i in 0..self.plugins.len() {
            self.start_if_allowed(i, prefs, python, node);
        }
    }

    /// Starts a plugin if it's enabled and the user allowed this version of
    /// it; otherwise asks for permission.
    pub fn start_if_allowed(
        &mut self,
        index: usize,
        prefs: &BTreeMap<String, PluginPrefs>,
        python: &str,
        node: &str,
    ) {
        let Some(p) = self.plugins.get_mut(index) else {
            return;
        };
        if p.manifest.is_err() || p.is_running() {
            return;
        }
        let pref = prefs.get(&p.id).cloned().unwrap_or_default();
        if pref.disabled {
            p.status = Status::Disabled;
            return;
        }
        let fingerprint = manifest::fingerprint(&p.dir);
        let allowed = match &pref.approved {
            Some(_) if pref.trust_changes => true,
            Some(a) => *a == fingerprint,
            None => false,
        };
        if !allowed {
            p.status = Status::NeedsPermission;
            if !self.prompts.contains(&p.id) {
                self.prompts.push_back(p.id.clone());
            }
            return;
        }
        self.start(index, python, node);
    }

    /// Where plugin `id` keeps its settings and files.
    pub fn plugin_data_dir(&self, id: &str) -> Option<PathBuf> {
        self.data_dir.as_ref().map(|d| d.join(id))
    }

    /// Adds a plugin just written to the plugins folder, without restarting
    /// the others. Returns its index.
    pub fn add_new(&mut self, id: &str) -> Option<usize> {
        if let Some(i) = self.index_of(id) {
            return Some(i);
        }
        let found = manifest::discover(self.dir.as_ref()?)
            .into_iter()
            .find(|d| d.id == id)?;
        self.plugins.push(Plugin::new(found));
        Some(self.plugins.len() - 1)
    }

    /// Reads a plugin's `plugin.json` again, for a reload after its files
    /// changed. Stops it first.
    pub fn refresh(&mut self, index: usize, now: f64) {
        self.stop(index, now);
        let Some(p) = self.plugins.get_mut(index) else {
            return;
        };
        p.manifest = manifest::load(&p.dir);
        p.status = match &p.manifest {
            Ok(_) => Status::Stopped,
            Err(e) => Status::Failed(e.clone()),
        };
    }

    /// Starts a plugin without checking permission.
    fn start(&mut self, index: usize, python: &str, node: &str) {
        let Some(sdk) = self.install_sdk() else {
            return;
        };
        let run = self.next_run;
        self.next_run += 1;
        let tx = self.events_tx.clone();
        let wake = self.wake.clone();
        let data_dir = self.plugin_data_dir(&self.plugins[index].id);
        let p = &mut self.plugins[index];
        p.settings = data_dir
            .as_deref()
            .map(|d| data::load_settings(d, p.options()))
            .unwrap_or_default();
        let Ok(m) = &p.manifest else { return };
        let custom = match m.runtime {
            manifest::Runtime::Python => python,
            manifest::Runtime::Node => node,
        };
        let programs = process::candidates(m.runtime, custom);
        let result = Proc::spawn(m.runtime, &programs, &m.main, &sdk, run, tx, wake);
        let id = p.id.clone();
        match result {
            Ok(proc) => {
                proc.send(notification(
                    "initialize",
                    json!({
                        "plugin": id,
                        "ettin": env!("CARGO_PKG_VERSION"),
                        "api": API_VERSION,
                        "language": crate::i18n::current(),
                        "settings": p.settings,
                        "dataDir": data_dir,
                    }),
                ));
                p.program = Some(proc.program.clone());
                p.proc = Some(proc);
                p.run = run;
                p.last_run = run;
                p.log_from = self.log_seq;
                p.status = Status::Running;
                p.commands.clear();
                p.transforms.clear();
                p.events.clear();
                p.ready = false;
                let msg = t!(
                    "plugin-started",
                    program = p.program.clone().unwrap_or_else(|| "?".to_owned()),
                    script = m
                        .main
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                );
                self.log(&id, msg, false);
            }
            Err(e) => {
                let program = programs.first().map(|p| p.0.as_str()).unwrap_or("?");
                let why = if e.kind() == std::io::ErrorKind::NotFound {
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
                };
                p.status = Status::Failed(why.clone());
                self.log(&id, why, true);
            }
        }
    }

    /// Asks a plugin to finish, and kills it if it doesn't soon.
    pub fn stop(&mut self, index: usize, now: f64) {
        let Some(p) = self.plugins.get_mut(index) else {
            return;
        };
        if let Some(mut proc) = p.proc.take() {
            proc.send(notification("shutdown", json!({})));
            proc.close_input();
            self.ending.push((proc, now + STOP_GRACE));
        }
        if p.is_running() {
            p.status = Status::Stopped;
        }
        p.clear();
        p.run = 0;
        let id = p.id.clone();
        self.panels.retain(|panel| panel.plugin != id);
        self.status.retain(|s| s.plugin != id);
        self.failures.retain(|(f, ..)| *f != id);
    }

    /// Stops everything, for when Ettin Edit closes.
    pub fn shutdown(&mut self) {
        for i in 0..self.plugins.len() {
            self.stop(i, 0.0);
        }
        // Give them a moment to save their state, then kill the rest.
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            self.ending
                .retain_mut(|(proc, _)| proc.try_exit().is_none());
            if self.ending.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        self.ending.clear();
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.plugins.iter().position(|p| p.id == id)
    }

    /// The running plugin `id`, if `run` is its current start.
    fn running(&self, id: &str, run: u64) -> Option<&Plugin> {
        self.plugins
            .iter()
            .find(|p| p.id == id && p.run == run && p.proc.is_some())
    }

    /// True while plugin `id` runs.
    pub fn is_running_plugin(&self, id: &str) -> bool {
        self.plugins.iter().any(|p| p.id == id && p.is_running())
    }

    pub fn is_current(&self, id: &str, run: u64) -> bool {
        self.running(id, run).is_some()
    }

    fn send(&self, id: &str, line: String) {
        if let Some(proc) = self
            .plugins
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.proc.as_ref())
        {
            proc.send(line);
        }
    }

    /// Answers a call.
    pub fn respond(&self, plugin: &str, run: u64, id: Value, result: Result<Value, CallError>) {
        if let Some(proc) = self.running(plugin, run).and_then(|p| p.proc.as_ref()) {
            proc.send(response(id, result));
        }
    }

    /// Runs one of a plugin's commands.
    pub fn run_command(&mut self, plugin: &str, command: &str) {
        match self.plugins.iter().find(|p| p.id == plugin) {
            Some(p) if p.is_running() => {
                if let Some(proc) = &p.proc {
                    proc.send(notification("command", json!({ "id": command })));
                }
            }
            Some(p) => {
                let msg = format!("{} isn't running, so its command can't run.", p.name());
                self.notices.push(msg.into());
            }
            None => {}
        }
    }

    /// Sends a request the plugin answers; its answer arrives in
    /// [`PluginHost::replies`] with the id returned. `None` when the plugin
    /// isn't running.
    pub fn request(&mut self, plugin: &str, method: &str, params: Value) -> Option<u64> {
        let id = self.next_request;
        let proc = self
            .plugins
            .iter()
            .find(|p| p.id == plugin && p.is_running())?
            .proc
            .as_ref()?;
        self.next_request += 1;
        proc.send(
            json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string(),
        );
        Some(id)
    }

    /// Takes `plugin`'s answer to request `id`, once it has come.
    pub fn take_reply(&mut self, plugin: &str, id: u64) -> Option<Reply> {
        let i = self
            .replies
            .iter()
            .position(|r| r.id == id && r.plugin == plugin)?;
        Some(self.replies.remove(i))
    }

    /// The transform `transform` of plugin `plugin`, if it's running.
    pub fn transform(&self, plugin: &str, transform: &str) -> Option<(&Plugin, &Transform)> {
        let p = self
            .plugins
            .iter()
            .find(|p| p.id == plugin && p.is_running())?;
        Some((p, p.transform(transform)?))
    }

    /// Changes a plugin's settings (from Manage Plugins > Options), saves
    /// them and tells the plugin.
    pub fn set_settings(&mut self, plugin: &str, values: Map<String, Value>) -> Result<(), String> {
        let dir = self
            .plugin_data_dir(plugin)
            .ok_or_else(|| t!("no-plugins-folder"))?;
        let index = self
            .index_of(plugin)
            .ok_or_else(|| "unknown plugin".to_owned())?;
        data::save_settings(&dir, &values)?;
        let p = &mut self.plugins[index];
        p.settings = data::load_settings(&dir, p.options());
        if let Some(proc) = &p.proc {
            proc.send(notification(
                "event",
                json!({ "event": "settings.changed", "settings": p.settings }),
            ));
        }
        Ok(())
    }

    /// The settings plugin `id` has now, read from disk when it isn't running.
    pub fn settings_of(&self, id: &str) -> Map<String, Value> {
        let Some(p) = self.plugins.iter().find(|p| p.id == id) else {
            return Map::new();
        };
        if p.is_running() {
            return p.settings.clone();
        }
        self.plugin_data_dir(id)
            .map(|d| data::load_settings(&d, p.options()))
            .unwrap_or_default()
    }

    /// Plugins that reload when their files change: returns the ones whose
    /// files changed and then stayed the same for a moment, to restart.
    pub fn changed_plugins(&mut self, watched: &[String], now: f64) -> Vec<String> {
        if now - self.last_watch < WATCH_EVERY {
            return Vec::new();
        }
        self.last_watch = now;
        let mut due = Vec::new();
        for p in &mut self.plugins {
            if !watched.contains(&p.id) {
                p.watch = None;
                continue;
            }
            let stamp = manifest::stamp(&p.dir);
            match &mut p.watch {
                None => p.watch = Some((stamp, None)),
                Some((old, changed)) => {
                    if *old != stamp {
                        *old = stamp;
                        *changed = Some(now);
                    } else if changed.is_some_and(|t| now - t >= WATCH_SETTLE) {
                        *changed = None;
                        due.push(p.id.clone());
                    }
                }
            }
        }
        due
    }

    /// Publishes the running plugins' transforms for menus and recipes (see
    /// [`known_transforms`]). Cheap when nothing changed.
    pub fn publish_transforms(&self) {
        let list: Vec<KnownTransform> = self
            .plugins
            .iter()
            .filter(|p| p.is_running())
            .flat_map(|p| {
                p.transforms.iter().map(|t| KnownTransform {
                    plugin: p.id.clone(),
                    plugin_name: p.name().to_owned(),
                    id: t.id.clone(),
                    title: t.title.clone(),
                    options: t.options.clone(),
                })
            })
            .collect();
        if *KNOWN.read().unwrap_or_else(|e| e.into_inner()) != list {
            *KNOWN.write().unwrap_or_else(|e| e.into_inner()) = list;
        }
    }

    /// True while a watched plugin's files changed and its reload waits.
    pub fn reload_waiting(&self) -> bool {
        self.plugins
            .iter()
            .any(|p| p.watch.is_some_and(|(_, c)| c.is_some()))
    }

    /// True if any running plugin wants `event`.
    pub fn wants(&self, event: &str) -> bool {
        self.plugins
            .iter()
            .any(|p| p.is_running() && (p.events.contains(event) || p.events.contains("*")))
    }

    /// Sends an event to the plugins that asked for it.
    pub fn event(&self, event: &str, data: Value) {
        let mut line = None;
        for p in &self.plugins {
            if p.is_running()
                && (p.events.contains(event) || p.events.contains("*"))
                && let Some(proc) = &p.proc
            {
                let line = line.get_or_insert_with(|| {
                    let mut data = data.clone();
                    data["event"] = json!(event);
                    notification("event", data)
                });
                proc.send(line.clone());
            }
        }
    }

    /// Plugin shortcuts, for the keyboard handler.
    pub fn shortcuts(&self) -> Vec<(KeyboardShortcut, Action)> {
        self.plugins
            .iter()
            .filter(|p| p.is_running())
            .flat_map(|p| {
                p.commands.iter().filter_map(|c| {
                    Some((
                        c.shortcut?,
                        Action::PluginCommand {
                            plugin: p.id.clone(),
                            command: c.id.clone(),
                        },
                    ))
                })
            })
            .collect()
    }

    /// Takes one waiting message from a plugin, up to `timeout`. Returns
    /// false if nothing came.
    pub fn wait(&mut self, timeout: Duration) -> bool {
        match self.events_rx.recv_timeout(timeout) {
            Ok(ev) => {
                self.stash.push_back(ev);
                true
            }
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => false,
        }
    }

    /// Handles what plugins sent; returns the calls the app must answer.
    pub fn poll(&mut self, now: f64) -> Vec<Call> {
        self.ending.retain_mut(|(proc, deadline)| {
            if proc.try_exit().is_some() {
                return false;
            }
            if now >= *deadline {
                proc.kill();
                return false;
            }
            true
        });
        let mut events: Vec<(u64, ProcEvent)> = self.stash.drain(..).collect();
        events.extend(self.events_rx.try_iter());
        let mut calls = Vec::new();
        for (run, ev) in events {
            if let ProcEvent::Log(line) = &ev {
                // Error output can come after the plugin ended.
                if let Some(p) = self
                    .plugins
                    .iter()
                    .find(|p| run != 0 && (p.run == run || p.last_run == run))
                {
                    let id = p.id.clone();
                    self.log(&id, line.clone(), false);
                }
                continue;
            }
            let Some(index) = self.plugins.iter().position(|p| p.run == run && run != 0) else {
                continue;
            };
            let id = self.plugins[index].id.clone();
            match ev {
                ProcEvent::Log(_) => {}
                ProcEvent::Closed => self.ended(index, now),
                ProcEvent::Message(line) => match parse(&line) {
                    Ok(Incoming::Call(call_id, method, params)) => {
                        let call = Call {
                            plugin: id,
                            run,
                            id: call_id,
                            method,
                            params,
                        };
                        if let Some(call) = self.handle(call) {
                            calls.push(call);
                        }
                    }
                    Ok(Incoming::Reply(reply_id, result)) => {
                        // Answers nobody takes (they came too late) don't pile up.
                        if self.replies.len() >= MAX_REPLIES {
                            self.replies.remove(0);
                        }
                        self.replies.push(Reply {
                            plugin: id,
                            id: reply_id,
                            result,
                        })
                    }
                    Err(e) => self.log(&id, format!("Bad message from plugin: {e}"), true),
                },
            }
        }
        self.report_failures(now);
        calls
    }

    /// Tells the user about plugins that stopped on their own, once their
    /// last error output has had time to arrive: where the error is, when
    /// the log shows it.
    fn report_failures(&mut self, now: f64) {
        let due: Vec<(String, u64, f64)> = self
            .failures
            .iter()
            .filter(|(.., t)| now >= *t)
            .cloned()
            .collect();
        if due.is_empty() {
            return;
        }
        self.failures.retain(|(.., t)| now < *t);
        for (id, _, _) in due {
            let Some(p) = self.plugins.iter().find(|p| p.id == id) else {
                continue;
            };
            let name = p.name().to_owned();
            let lines: Vec<&str> = self
                .log
                .iter()
                .filter(|l| l.plugin == id && l.seq > p.log_from)
                .map(|l| l.text.as_str())
                .collect();
            let notice = error_notice(&name, &lines, &p.dir)
                .unwrap_or_else(|| t!("plugin-stopped-notice", name = name).into());
            self.notices.push(notice);
        }
    }

    /// The plugin's output closed: it ended.
    fn ended(&mut self, index: usize, now: f64) {
        let p = &mut self.plugins[index];
        let Some(mut proc) = p.proc.take() else {
            return;
        };
        // Give the process a moment to exit, so the code is known.
        let mut code = None;
        for _ in 0..20 {
            if let Some(c) = proc.try_exit() {
                code = Some(c);
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let why = match code {
            Some(Some(0)) => t!("plugin-exited"),
            Some(Some(c)) => t!("plugin-exit-code", code = c),
            _ => t!("plugin-died"),
        };
        p.status = Status::Failed(why.clone());
        p.clear();
        let run = p.run;
        p.run = 0;
        let id = p.id.clone();
        self.panels.retain(|panel| panel.plugin != id);
        self.status.retain(|s| s.plugin != id);
        self.log(&id, why.clone(), true);
        if code != Some(Some(0)) {
            self.failures.push((id, run, now + FAILURE_DELAY));
        }
        if code.is_none() {
            self.ending.push((proc, f64::NEG_INFINITY));
        }
    }

    /// Handles calls about plugins themselves. Returns the call when the app
    /// has to answer it.
    fn handle(&mut self, call: Call) -> Option<Call> {
        let result = match call.method.as_str() {
            "commands.register" => self.register_command(&call),
            "transforms.register" => self.register_transform(&call),
            "ready" => {
                if let Some(i) = self.index_of(&call.plugin) {
                    self.plugins[i].ready = true;
                }
                Ok(Value::Null)
            }
            "settings.get" => Ok(Value::Object(self.settings_of(&call.plugin))),
            "settings.set" => self.plugin_sets_settings(&call),
            "status.set" => self.set_status(&call),
            "events.subscribe" => self.subscribe(&call),
            "panel.show" => self.show_panel(&call),
            "panel.close" => {
                let id = str_param(&call.params, "id").unwrap_or_default();
                self.panels
                    .retain(|p| !(p.plugin == call.plugin && p.id == id));
                Ok(Value::Null)
            }
            "log" => {
                let text = str_param(&call.params, "text").unwrap_or_default();
                let error = str_param(&call.params, "level") == Some("error");
                self.log(&call.plugin, text, error);
                Ok(Value::Null)
            }
            "error" => {
                let text = str_param(&call.params, "message").unwrap_or_default();
                self.log(&call.plugin, text, true);
                let (name, dir) = self
                    .plugins
                    .iter()
                    .find(|p| p.id == call.plugin)
                    .map(|p| (p.name().to_owned(), p.dir.clone()))
                    .unwrap_or_default();
                let lines: Vec<&str> = text.lines().collect();
                let notice = error_notice(&name, &lines, &dir).unwrap_or_else(|| {
                    let first = text.lines().last().unwrap_or("error").to_owned();
                    format!("{name}: {first} (see Plugins > Plugin Log)").into()
                });
                self.notices.push(notice);
                Ok(Value::Null)
            }
            _ => return Some(call),
        };
        if let Some(id) = call.id {
            self.respond(&call.plugin, call.run, id, result);
        }
        None
    }

    fn register_command(&mut self, call: &Call) -> Result<Value, CallError> {
        let id = str_param(&call.params, "id")
            .filter(|s| !s.is_empty())
            .ok_or((INVALID_PARAMS, "\"id\" is required".to_owned()))?
            .to_owned();
        let title = str_param(&call.params, "title")
            .filter(|s| !s.is_empty())
            .ok_or((INVALID_PARAMS, "\"title\" is required".to_owned()))?
            .to_owned();
        let mut warning = None;
        let shortcut = match str_param(&call.params, "shortcut").filter(|s| !s.trim().is_empty()) {
            None => None,
            Some(text) => match shortcut::parse(text) {
                Ok(s) => match self.shortcut_owner(&s, &call.plugin, &id) {
                    Some(owner) => {
                        warning = Some(format!("{text} is already used by {owner}"));
                        None
                    }
                    None => Some(s),
                },
                Err(e) => {
                    warning = Some(e);
                    None
                }
            },
        };
        let icon = match str_param(&call.params, "icon").map(str::trim) {
            None | Some("") => None,
            Some(text) => match check_icon(text) {
                Ok(()) => Some(text.to_owned()),
                Err(e) => {
                    let plugin = call.plugin.clone();
                    self.log(&plugin, format!("Icon not used: {e}"), true);
                    None
                }
            },
        };
        let Some(index) = self.index_of(&call.plugin) else {
            return Err((FAILED, "unknown plugin".to_owned()));
        };
        let p = &mut self.plugins[index];
        let count = p.commands.len();
        match p.commands.iter_mut().find(|c| c.id == id) {
            Some(c) => {
                c.title = title;
                c.shortcut = shortcut;
                c.icon = icon;
            }
            None if count >= MAX_COMMANDS => {
                return Err((
                    FAILED,
                    format!("A plugin can add at most {MAX_COMMANDS} commands"),
                ));
            }
            None => p.commands.push(Command {
                id,
                title,
                shortcut,
                icon,
            }),
        }
        if let Some(w) = &warning {
            let plugin = call.plugin.clone();
            self.log(&plugin, format!("Shortcut not added: {w}"), true);
        }
        Ok(json!({ "shortcut": shortcut.is_some(), "warning": warning }))
    }

    fn register_transform(&mut self, call: &Call) -> Result<Value, CallError> {
        let title = str_param(&call.params, "title")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or((INVALID_PARAMS, "\"title\" is required".to_owned()))?
            .to_owned();
        let id = str_param(&call.params, "id")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&title)
            .to_owned();
        let options = match call.params.get("options") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(o)) => o.clone(),
            Some(_) => {
                return Err((
                    INVALID_PARAMS,
                    "\"options\" must be a dict of option names and defaults".to_owned(),
                ));
            }
        };
        let languages = match call.params.get("languages") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::String(s)) => vec![s.to_lowercase()],
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| v.as_str().map(str::to_lowercase))
                .collect::<Option<Vec<_>>>()
                .ok_or((
                    INVALID_PARAMS,
                    "\"languages\" must be a list of language names".to_owned(),
                ))?,
            Some(_) => {
                return Err((
                    INVALID_PARAMS,
                    "\"languages\" must be a list of language names".to_owned(),
                ));
            }
        };
        let index = self
            .index_of(&call.plugin)
            .ok_or((FAILED, "unknown plugin".to_owned()))?;
        let p = &mut self.plugins[index];
        let transform = Transform {
            id: id.clone(),
            title,
            options,
            languages,
        };
        let count = p.transforms.len();
        match p.transforms.iter_mut().find(|t| t.id == id) {
            Some(t) => *t = transform,
            None if count >= MAX_TRANSFORMS => {
                return Err((
                    FAILED,
                    format!("A plugin can add at most {MAX_TRANSFORMS} transforms"),
                ));
            }
            None => p.transforms.push(transform),
        }
        Ok(json!({ "id": id }))
    }

    fn plugin_sets_settings(&mut self, call: &Call) -> Result<Value, CallError> {
        let Some(Value::Object(values)) = call.params.get("values") else {
            return Err((INVALID_PARAMS, "\"values\" must be a dict".to_owned()));
        };
        let dir = self
            .plugin_data_dir(&call.plugin)
            .ok_or((FAILED, t!("no-plugins-folder")))?;
        let index = self
            .index_of(&call.plugin)
            .ok_or((FAILED, "unknown plugin".to_owned()))?;
        let p = &mut self.plugins[index];
        let mut all = p.settings.clone();
        for (k, v) in values {
            all.insert(k.clone(), v.clone());
        }
        data::save_settings(&dir, &all).map_err(|e| (FAILED, e))?;
        p.settings = data::load_settings(&dir, p.options());
        Ok(Value::Object(p.settings.clone()))
    }

    fn set_status(&mut self, call: &Call) -> Result<Value, CallError> {
        let id = str_param(&call.params, "id")
            .filter(|s| !s.is_empty())
            .ok_or((INVALID_PARAMS, "\"id\" is required".to_owned()))?
            .to_owned();
        let text = str_param(&call.params, "text").unwrap_or_default().trim();
        let at = self
            .status
            .iter()
            .position(|s| s.plugin == call.plugin && s.id == id);
        if text.is_empty() {
            if let Some(i) = at {
                self.status.remove(i);
            }
            return Ok(Value::Null);
        }
        let item = StatusItem {
            plugin: call.plugin.clone(),
            id,
            text: text.chars().take(80).collect(),
            tooltip: str_param(&call.params, "tooltip")
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            command: str_param(&call.params, "command")
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
        };
        match at {
            Some(i) => self.status[i] = item,
            None => {
                let mine = self
                    .status
                    .iter()
                    .filter(|s| s.plugin == call.plugin)
                    .count();
                if mine >= MAX_STATUS_ITEMS {
                    return Err((
                        FAILED,
                        format!("A plugin can show at most {MAX_STATUS_ITEMS} status items"),
                    ));
                }
                self.status.push(item);
            }
        }
        Ok(Value::Null)
    }

    /// Who already uses a shortcut: Ettin Edit or another plugin command.
    fn shortcut_owner(&self, s: &KeyboardShortcut, plugin: &str, command: &str) -> Option<String> {
        if crate::actions::all_shortcuts()
            .iter()
            .any(|(b, _)| shortcut::same(b, s))
        {
            return Some("Ettin Edit".to_owned());
        }
        self.plugins.iter().find_map(|p| {
            p.commands
                .iter()
                .find(|c| {
                    c.shortcut.is_some_and(|cs| shortcut::same(&cs, s))
                        && !(p.id == plugin && c.id == command)
                })
                .map(|c| format!("{} ({})", c.title, p.name()))
        })
    }

    fn subscribe(&mut self, call: &Call) -> Result<Value, CallError> {
        let events: Vec<String> = call
            .params
            .get("events")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .ok_or((
                INVALID_PARAMS,
                "\"events\" must be a list of names".to_owned(),
            ))?;
        let index = self
            .index_of(&call.plugin)
            .ok_or((FAILED, "unknown plugin".to_owned()))?;
        self.plugins[index].events.extend(events);
        Ok(Value::Null)
    }

    fn show_panel(&mut self, call: &Call) -> Result<Value, CallError> {
        let id = str_param(&call.params, "id")
            .filter(|s| !s.is_empty())
            .ok_or((INVALID_PARAMS, "\"id\" is required".to_owned()))?
            .to_owned();
        let items: Vec<Item> = match call.params.get("items") {
            Some(v) => serde_json::from_value(v.clone())
                .map_err(|e| (INVALID_PARAMS, format!("Bad panel items: {e}")))?,
            None => Vec::new(),
        };
        let position: Option<Position> = match call.params.get("position") {
            Some(v) if !v.is_null() => Some(serde_json::from_value(v.clone()).map_err(|_| {
                (
                    INVALID_PARAMS,
                    "\"position\" is \"right\" or \"bottom\"".to_owned(),
                )
            })?),
            _ => None,
        };
        let title = str_param(&call.params, "title").map(str::to_owned);
        let index = match self
            .panels
            .iter()
            .position(|p| p.plugin == call.plugin && p.id == id)
        {
            Some(i) => i,
            None => {
                let mut panel = Panel::new(&call.plugin, &id);
                panel.title = id.clone();
                self.panels.push(panel);
                self.panels.len() - 1
            }
        };
        let panel = &mut self.panels[index];
        if let Some(t) = title {
            panel.title = t;
        }
        if let Some(pos) = position {
            panel.position = pos;
        }
        panel.set_items(items);
        if call.params.get("scrollToTop").and_then(Value::as_bool) == Some(true) {
            panel.scroll_to_top();
        }
        Ok(Value::Null)
    }

    /// Tells a plugin what the user did in one of its panels.
    pub fn panel_event(&mut self, index: usize, event: PanelEvent) {
        let Some(panel) = self.panels.get(index) else {
            return;
        };
        let (plugin, id) = (panel.plugin.clone(), panel.id.clone());
        let msg = notification(
            "panel",
            json!({
                "panel": id,
                "item": event.item,
                "event": event.kind,
                "value": event.value,
                "values": panel.values(),
            }),
        );
        self.send(&plugin, msg);
        if event.kind == "close" {
            self.panels.remove(index);
        }
    }
}

/// A message from a plugin: a call (or notification) or an answer to one
/// of Ettin Edit's requests.
enum Incoming {
    /// (id, method, params); no id for a notification.
    Call(Option<Value>, String, Value),
    Reply(u64, Result<Value, ReplyError>),
}

/// Reads a JSON-RPC message.
fn parse(line: &str) -> Result<Incoming, String> {
    let v: Value = serde_json::from_str(line).map_err(|e| format!("{e}: {}", clip(line)))?;
    let id = v.get("id").filter(|id| !id.is_null()).cloned();
    let Some(method) = v.get("method").and_then(Value::as_str) else {
        let id = id
            .and_then(|id| id.as_u64())
            .ok_or_else(|| format!("no \"method\": {}", clip(line)))?;
        return Ok(Incoming::Reply(
            id,
            match v.get("error") {
                Some(e) if !e.is_null() => Err(ReplyError::from_json(e)),
                _ => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
            },
        ));
    };
    let params = v.get("params").cloned().unwrap_or(Value::Null);
    Ok(Incoming::Call(id, method.to_owned(), params))
}

/// Where an error in a plugin's own files happened, from its traceback
/// (Python) or stack (Node): the file and line, and the error message.
pub fn error_location(lines: &[&str], dir: &Path) -> Option<(PathBuf, usize, String)> {
    let inside = |path: &str| -> Option<PathBuf> {
        let path = PathBuf::from(path);
        let full = if path.is_absolute() {
            path
        } else {
            dir.join(path)
        };
        let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        canon(&full).starts_with(canon(dir)).then_some(full)
    };
    // Python: File "/plugins/x/main.py", line 12, in <module>; the last
    // one in the plugin's files is nearest the error.
    let mut found = None;
    for l in lines {
        let Some(rest) = l.trim_start().strip_prefix("File \"") else {
            continue;
        };
        let Some((path, after)) = rest.split_once("\", line ") else {
            continue;
        };
        let number: String = after.chars().take_while(char::is_ascii_digit).collect();
        if let (Some(path), Ok(n)) = (inside(path), number.parse::<usize>()) {
            found = Some((path, n));
        }
    }
    let message = |python: bool| -> String {
        let looks_like_error = |l: &&&str| {
            let l = l.trim();
            l.split_once(':').is_some_and(|(name, _)| {
                !name.contains(' ') && (name.ends_with("Error") || name.ends_with("Exception"))
            })
        };
        let pick = if python {
            lines.iter().rev().find(looks_like_error)
        } else {
            lines.iter().find(looks_like_error)
        };
        pick.or_else(|| lines.iter().rev().find(|l| !l.trim().is_empty()))
            .map(|l| l.trim().to_owned())
            .unwrap_or_default()
    };
    if let Some((path, n)) = found {
        return Some((path, n, message(true)));
    }
    // Node: /plugins/x/main.js:12 or (/plugins/x/main.js:12:5); the first
    // one in the plugin's files is where it was thrown.
    for l in lines {
        for (start, _) in l.match_indices(['/', '\\']).chain(
            l.char_indices()
                .filter(|(_, c)| c.is_ascii_alphabetic())
                .filter(|(i, _)| l[*i..].get(1..3) == Some(":\\"))
                .map(|(i, _)| (i, "")),
        ) {
            let tail = &l[start..];
            // The path ends at ":<digits>".
            let mut search = 0;
            while let Some(colon) = tail[search..].find(':').map(|c| c + search) {
                let digits: String = tail[colon + 1..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                if !digits.is_empty()
                    && let Some(path) = inside(&tail[..colon])
                    && path.is_file()
                {
                    return Some((path, digits.parse().ok()?, message(false)));
                }
                search = colon + 1;
            }
        }
    }
    None
}

/// "Plugin X stopped with an error: main.py line 12: NameError…", with the
/// file and line to open, when `lines` (its log) show where.
fn error_notice(name: &str, lines: &[&str], dir: &Path) -> Option<HostNotice> {
    let (path, line, error) = error_location(lines, dir)?;
    let file = path
        .strip_prefix(dir)
        .unwrap_or(&path)
        .display()
        .to_string();
    Some(HostNotice {
        text: t!(
            "plugin-error-at",
            name = name,
            file = file,
            line = line,
            error = error
        ),
        open: Some((path, line)),
    })
}

fn clip(line: &str) -> String {
    match line.char_indices().nth(200) {
        Some((i, _)) => format!("{}…", &line[..i]),
        None => line.to_owned(),
    }
}

pub fn notification(method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string()
}

pub fn response(id: Value, result: Result<Value, CallError>) -> String {
    match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message },
        }),
    }
    .to_string()
}

/// A command icon is a Phosphor icon name, or a PNG file inside the plugin's
/// folder given relative to it.
fn check_icon(icon: &str) -> Result<(), String> {
    if !icon.to_ascii_lowercase().ends_with(".png") {
        return match crate::ribbon::glyph(icon) {
            Some(_) => Ok(()),
            None => Err(format!(
                "\"{icon}\" is not a Phosphor icon name or a .png file"
            )),
        };
    }
    let path = std::path::Path::new(icon);
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "\"{icon}\" must be a path inside the plugin's folder"
        ));
    }
    Ok(())
}

pub fn str_param<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_requests_and_notifications() {
        let Ok(Incoming::Call(id, method, params)) =
            parse(r#"{"jsonrpc":"2.0","id":3,"method":"text.get","params":{"doc":1}}"#)
        else {
            panic!("not a call")
        };
        assert_eq!(id, Some(json!(3)));
        assert_eq!(method, "text.get");
        assert_eq!(params["doc"], 1);
        let Ok(Incoming::Call(id, method, _)) = parse(r#"{"method":"log","params":{"text":"x"}}"#)
        else {
            panic!("not a call")
        };
        assert_eq!(id, None);
        assert_eq!(method, "log");
        assert!(parse("not json").is_err());
        assert!(parse(r#"{"id":"x"}"#).is_err());
        let Ok(Incoming::Reply(1, Ok(v))) = parse(r#"{"id":1,"result":{"text":"hi"}}"#) else {
            panic!("not a reply")
        };
        assert_eq!(v["text"], "hi");
        let Ok(Incoming::Reply(2, Err(e))) = parse(
            r#"{"id":2,"error":{"code":-32000,"message":"bad","data":{"line":3,"column":4}}}"#,
        ) else {
            panic!("not an error reply")
        };
        assert_eq!(
            (e.message.as_str(), e.line, e.column),
            ("bad", Some(3), Some(4))
        );
    }

    #[test]
    fn builds_responses() {
        let ok: Value = serde_json::from_str(&response(json!(1), Ok(json!("hi")))).unwrap();
        assert_eq!(ok["result"], "hi");
        let err: Value =
            serde_json::from_str(&response(json!(2), Err((FAILED, "nope".into())))).unwrap();
        assert_eq!(err["error"]["code"], FAILED);
        assert_eq!(err["error"]["message"], "nope");
    }

    fn host_with(id: &str) -> PluginHost {
        let mut host = PluginHost::new(std::sync::Arc::new(|| {}));
        host.dir = None;
        host.data_dir = None;
        let mut p = Plugin::new(Discovered {
            id: id.to_owned(),
            dir: PathBuf::new(),
            manifest: Err("test".into()),
        });
        p.status = Status::Running;
        p.run = 1;
        host.plugins.push(p);
        host
    }

    #[test]
    fn registers_transforms() {
        let mut host = host_with("p");
        host.register_transform(&call(
            "transforms.register",
            json!({"title": "Explode HL7 Segments", "id": "explode",
                   "options": {"separator": "\n"}, "languages": ["HL7"]}),
        ))
        .unwrap();
        host.register_transform(&call("transforms.register", json!({"title": "Shout"})))
            .unwrap();
        let p = &host.plugins[0];
        assert_eq!(p.transforms.len(), 2);
        let t = p.transform("explode").unwrap();
        assert_eq!(t.options["separator"], "\n");
        assert!(t.suits("hl7") && t.suits("HL7") && !t.suits("JSON"));
        // Without an id, the title is the id.
        assert!(p.transform("Shout").is_some());
        assert!(host.transform("p", "explode").is_some());
        // Registering again replaces it.
        host.register_transform(&call(
            "transforms.register",
            json!({"title": "Explode", "id": "explode"}),
        ))
        .unwrap();
        assert_eq!(host.plugins[0].transforms.len(), 2);
        assert_eq!(host.plugins[0].transforms[0].title, "Explode");
        assert!(
            host.register_transform(&call("transforms.register", json!({"options": 3})))
                .is_err()
        );
        assert!(
            host.register_transform(&call(
                "transforms.register",
                json!({"title": "x", "options": [1]})
            ))
            .is_err()
        );
    }

    #[test]
    fn status_items() {
        let mut host = host_with("p");
        host.set_status(&call(
            "status.set",
            json!({"id": "field", "text": "PID-5.1 Family name", "tooltip": "t", "command": "c"}),
        ))
        .unwrap();
        assert_eq!(host.status.len(), 1);
        assert_eq!(host.status[0].command.as_deref(), Some("c"));
        host.set_status(&call("status.set", json!({"id": "field", "text": "PID-3"})))
            .unwrap();
        assert_eq!(host.status.len(), 1);
        assert_eq!(host.status[0].text, "PID-3");
        assert_eq!(host.status[0].tooltip, None);
        for i in 0..3 {
            host.set_status(&call(
                "status.set",
                json!({"id": i.to_string(), "text": "x"}),
            ))
            .unwrap();
        }
        assert!(
            host.set_status(&call("status.set", json!({"id": "5", "text": "x"})))
                .is_err()
        );
        host.set_status(&call("status.set", json!({"id": "field", "text": ""})))
            .unwrap();
        assert_eq!(host.status.len(), 3);
        host.stop(0, 0.0);
        assert!(host.status.is_empty());
    }

    #[test]
    fn finds_where_an_error_happened() {
        let dir = std::env::temp_dir().join(format!("ettin-errloc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("main.py"), "x").unwrap();
        fs::write(dir.join("main.js"), "x").unwrap();
        let main_py = dir.join("main.py").display().to_string();
        let python = [
            "Traceback (most recent call last):".to_owned(),
            format!("  File \"{main_py}\", line 12, in <module>"),
            "    boom()".to_owned(),
            "  File \"/usr/lib/python3/x.py\", line 3, in boom".to_owned(),
            "NameError: name 'boom' is not defined".to_owned(),
        ];
        let lines: Vec<&str> = python.iter().map(String::as_str).collect();
        let (path, line, error) = error_location(&lines, &dir).unwrap();
        assert_eq!(path, dir.join("main.py"));
        assert_eq!(line, 12);
        assert_eq!(error, "NameError: name 'boom' is not defined");
        let main_js = dir.join("main.js").display().to_string();
        let node = [
            format!("{main_js}:7"),
            "  oops(".to_owned(),
            "  ^".to_owned(),
            String::new(),
            "ReferenceError: oops is not defined".to_owned(),
            format!("    at Object.<anonymous> ({main_js}:7:3)"),
        ];
        let lines: Vec<&str> = node.iter().map(String::as_str).collect();
        let (path, line, error) = error_location(&lines, &dir).unwrap();
        assert_eq!(path, dir.join("main.js"));
        assert_eq!(line, 7);
        assert_eq!(error, "ReferenceError: oops is not defined");
        let n = error_notice("Mine", &lines, &dir).unwrap();
        assert!(
            n.text.contains("main.js") && n.text.contains('7'),
            "{}",
            n.text
        );
        assert_eq!(n.open, Some((dir.join("main.js"), 7)));
        assert!(error_location(&["something else"], &dir).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    fn call(method: &str, params: Value) -> Call {
        Call {
            plugin: "p".into(),
            run: 1,
            id: None,
            method: method.into(),
            params,
        }
    }

    #[test]
    fn command_icons_are_names_or_png_files_inside_the_plugin() {
        let mut host = host_with("p");
        for (id, icon, kept) in [
            ("a", "star", true),
            ("b", "icons/run.png", true),
            ("c", "no-such-icon", false),
            ("d", "../other/run.png", false),
            ("e", "/tmp/run.png", false),
        ] {
            host.register_command(&call(
                "commands.register",
                json!({"id": id, "title": id, "icon": icon}),
            ))
            .unwrap();
            let c = host.plugins[0]
                .commands
                .iter()
                .find(|c| c.id == id)
                .unwrap();
            assert_eq!(c.icon.is_some(), kept, "{icon}");
        }
    }

    #[test]
    fn registers_commands_and_checks_shortcuts() {
        let mut host = host_with("p");
        let r = host
            .register_command(&call(
                "commands.register",
                json!({"id": "a", "title": "A", "shortcut": "Ctrl+Alt+Y"}),
            ))
            .unwrap();
        assert_eq!(r["shortcut"], true);
        // Ettin Edit's own shortcut.
        let r = host
            .register_command(&call(
                "commands.register",
                json!({"id": "b", "title": "B", "shortcut": "Ctrl+S"}),
            ))
            .unwrap();
        assert_eq!(r["shortcut"], false);
        assert!(r["warning"].as_str().unwrap().contains("Ettin Edit"));
        // Taken by another command; registering "a" again keeps its own.
        let r = host
            .register_command(&call(
                "commands.register",
                json!({"id": "c", "title": "C", "shortcut": "ctrl+alt+y"}),
            ))
            .unwrap();
        assert_eq!(r["shortcut"], false);
        let r = host
            .register_command(&call(
                "commands.register",
                json!({"id": "a", "title": "A2", "shortcut": "Ctrl+Alt+Y"}),
            ))
            .unwrap();
        assert_eq!(r["shortcut"], true);
        assert_eq!(host.plugins[0].commands.len(), 3);
        assert_eq!(host.plugins[0].commands[0].title, "A2");
        assert_eq!(host.shortcuts().len(), 1);
        assert!(
            host.register_command(&call("commands.register", json!({"title": "x"})))
                .is_err()
        );
    }

    #[test]
    fn panels_and_events() {
        let mut host = host_with("p");
        assert!(!host.wants("saved"));
        host.subscribe(&call("events.subscribe", json!({"events": ["saved"]})))
            .unwrap();
        assert!(host.wants("saved"));
        assert!(!host.wants("changed"));
        host.show_panel(&call(
            "panel.show",
            json!({"id": "main", "title": "Stats", "position": "bottom", "items": [{"type": "label", "text": "x"}]}),
        ))
        .unwrap();
        assert_eq!(host.panels.len(), 1);
        assert_eq!(host.panels[0].position, Position::Bottom);
        // Showing again updates the same panel.
        host.show_panel(&call("panel.show", json!({"id": "main", "items": []})))
            .unwrap();
        assert_eq!(host.panels.len(), 1);
        assert_eq!(host.panels[0].title, "Stats");
        assert!(
            host.show_panel(&call(
                "panel.show",
                json!({"id": "x", "items": [{"type": "bad"}]})
            ))
            .is_err()
        );
        host.panel_event(
            0,
            PanelEvent {
                item: String::new(),
                kind: "close",
                value: Value::Null,
            },
        );
        assert!(host.panels.is_empty());
    }
}

/// Runs real plugins with the installed Python and Node, through the SDKs.
#[cfg(test)]
mod process_tests {
    use super::*;
    use std::time::Instant;

    fn have(program: &str) -> bool {
        std::process::Command::new(program)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    /// Polls until `done` or a timeout, answering `text.get` with "hello".
    /// The timeout is generous: interpreters can start slowly on busy CI runners.
    fn pump(host: &mut PluginHost, mut done: impl FnMut(&PluginHost) -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(60) {
            host.wait(Duration::from_millis(50));
            for call in host.poll(0.0) {
                let result = match call.method.as_str() {
                    "text.get" => Ok(json!("hello")),
                    other => Err((METHOD_NOT_FOUND, other.to_owned())),
                };
                if let Some(id) = call.id {
                    host.respond(&call.plugin, call.run, id, result);
                }
            }
            if done(host) {
                return true;
            }
        }
        false
    }

    fn run_plugin(folder: &str, main: &str, script: &str) {
        let root =
            std::env::temp_dir().join(format!("ettin-plugins-{}-{folder}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join(folder);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(main), script).unwrap();
        fs::write(dir.join("plugin.json"), r#"{"name": "Test"}"#).unwrap();

        let mut host = PluginHost::new(std::sync::Arc::new(|| {}));
        host.dir = Some(root.clone());
        // Not allowed yet: it waits for permission.
        let mut prefs = BTreeMap::new();
        host.reload(&prefs, "", "", 0.0);
        assert_eq!(host.plugins[0].status, Status::NeedsPermission);
        assert_eq!(host.prompts.len(), 1);
        prefs.insert(
            folder.to_owned(),
            PluginPrefs {
                approved: Some(manifest::fingerprint(&dir)),
                ..Default::default()
            },
        );
        host.reload(&prefs, "", "", 0.0);
        assert_eq!(
            host.plugins[0].status,
            Status::Running,
            "{:?}",
            host.log.iter().map(|l| &l.text).collect::<Vec<_>>()
        );

        assert!(
            pump(&mut host, |h| !h.plugins[0].commands.is_empty()
                && h.wants("saved")),
            "command or event not registered"
        );
        let cmd = &host.plugins[0].commands[0];
        assert_eq!(cmd.title, "Echo");
        assert!(cmd.shortcut.is_some());
        assert!(host.wants("saved"));

        host.run_command(folder, &host.plugins[0].commands[0].id.clone());
        // print() / console.log() go to the log, not the protocol. They come
        // through stderr, which is read separately, so either may arrive first.
        assert!(
            pump(&mut host, |h| h.log.iter().any(|l| l.text == "got hello")
                && h.log.iter().any(|l| l.text == "printed")),
            "no answer: {:?}",
            host.log.iter().map(|l| &l.text).collect::<Vec<_>>()
        );

        host.stop(0, 0.0);
        assert_eq!(host.plugins[0].status, Status::Stopped);
        host.shutdown();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn python_plugin_round_trip() {
        if !have("python3") {
            eprintln!("python3 isn't installed; skipped");
            return;
        }
        run_plugin(
            "py-test",
            "main.py",
            r#"
import ettin

@ettin.command("Echo", shortcut="Ctrl+Alt+F11")
def echo():
    print("printed")
    ettin.log("got", ettin.call("text.get"))

@ettin.on("saved")
def saved(doc):
    pass

ettin.run()
"#,
        );
    }

    #[test]
    fn node_plugin_round_trip() {
        if !have("node") {
            eprintln!("node isn't installed; skipped");
            return;
        }
        run_plugin(
            "js-test",
            "main.js",
            r#"
const ettin = require('ettin');
ettin.command('Echo', async () => {
  console.log('printed');
  ettin.log('got', await ettin.call('text.get'));
}, { shortcut: 'Ctrl+Alt+F11' });
ettin.on('saved', () => {});
"#,
        );
    }

    /// Settings with an options list, a status item, a mark, choose(), a
    /// transform and settings.changed, through the Python SDK.
    #[test]
    fn python_plugin_api_2() {
        if !have("python3") {
            eprintln!("python3 isn't installed; skipped");
            return;
        }
        let root = std::env::temp_dir().join(format!("ettin-api2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join("plugins").join("api2");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("plugin.json"),
            r#"{"name": "API 2", "options": [
                {"key": "width", "title": "Width", "type": "number", "default": 80},
                {"key": "mode", "title": "Mode", "type": "choice", "choices": ["a", "b"], "default": "a"}
            ]}"#,
        )
        .unwrap();
        fs::write(
            dir.join("main.py"),
            r#"
import ettin

@ettin.transform("Wrap", options={"mark": "!"})
def wrap(text, options):
    if text == "bad":
        raise ettin.TransformError("no good", line=2, column=3)
    return "[" + text + "]" + options["mark"]

@ettin.command("Try")
def try_it():
    ettin.log("width", ettin.settings["width"], ettin.settings["mode"])
    ettin.status("count", "3 words", tooltip="Words", command="try_it")
    ettin.active().mark("lint", [(0, 2), (4, 5)], style="background", color="warning",
                        message="look")
    ettin.log("picked", ettin.choose(["x", {"label": "y", "detail": "why"}], title="Pick"))
    ettin.settings["seen"] = True

@ettin.on("settings.changed")
def changed(settings):
    ettin.log("changed", settings["width"])

ettin.run()
"#,
        )
        .unwrap();
        let mut host = PluginHost::new(std::sync::Arc::new(|| {}));
        host.dir = Some(root.join("plugins"));
        host.data_dir = Some(root.join("plugin-data"));
        let mut prefs = BTreeMap::new();
        prefs.insert(
            "api2".to_owned(),
            PluginPrefs {
                approved: Some(manifest::fingerprint(&dir)),
                ..Default::default()
            },
        );
        host.reload(&prefs, "", "", 0.0);
        assert!(
            pump(&mut host, |h| h.plugins[0].ready),
            "{:?} {:?} {}",
            host.log.iter().map(|l| &l.text).collect::<Vec<_>>(),
            host.plugins[0].status,
            host.plugins[0].transforms.len()
        );
        assert_eq!(host.plugins[0].transforms[0].title, "Wrap");
        assert_eq!(host.plugins[0].settings["width"], 80);

        // The command: settings, status, a mark and choose().
        host.run_command("api2", "try_it");
        let mut marks = None;
        let start = Instant::now();
        let log = |h: &PluginHost| h.log.iter().map(|l| l.text.clone()).collect::<Vec<_>>();
        while start.elapsed() < Duration::from_secs(60)
            && !log(&host).iter().any(|l| l.starts_with("picked"))
        {
            host.wait(Duration::from_millis(50));
            for call in host.poll(0.0) {
                let result = match call.method.as_str() {
                    "marks.set" => {
                        marks = Some(call.params.clone());
                        Ok(Value::Null)
                    }
                    "ui.choose" => {
                        assert_eq!(call.params["title"], "Pick");
                        assert_eq!(call.params["items"][1]["detail"], "why");
                        Ok(json!(1))
                    }
                    "documents.active" | "documents.info" => Ok(json!({"id": 1, "title": "t"})),
                    other => Err((METHOD_NOT_FOUND, other.to_owned())),
                };
                if let Some(id) = call.id {
                    host.respond(&call.plugin, call.run, id, result);
                }
            }
        }
        let lines = log(&host);
        assert!(lines.iter().any(|l| l == "width 80 a"), "{lines:?}");
        assert!(
            lines
                .iter()
                .any(|l| l.contains("picked") && l.contains("'y'")),
            "{lines:?}"
        );
        let marks = marks.expect("marks.set was called");
        assert_eq!(marks["id"], "lint");
        assert_eq!(marks["style"], "background");
        assert_eq!(marks["color"], "warning");
        assert_eq!(marks["ranges"], json!([[0, 2], [4, 5]]));
        assert_eq!(host.status.len(), 1);
        assert_eq!(host.status[0].text, "3 words");
        assert_eq!(host.status[0].command.as_deref(), Some("try_it"));
        assert!(pump(&mut host, |h| h.settings_of("api2").get("seen")
            == Some(&json!(true))));
        let saved = fs::read_to_string(root.join("plugin-data/api2/settings.json")).unwrap();
        assert!(saved.contains("\"seen\""), "{saved}");

        // The Options form saves, and the plugin hears about it.
        let mut values = host.settings_of("api2");
        values.insert("width".into(), json!(100));
        host.set_settings("api2", values).unwrap();
        assert!(pump(&mut host, |h| h
            .log
            .iter()
            .any(|l| l.text == "changed 100")));

        // The transform, its options and its errors.
        let id = host
            .request(
                "api2",
                "transform.run",
                json!({"id": "Wrap", "text": "hi", "options": {}}),
            )
            .unwrap();
        assert!(pump(&mut host, |h| h.replies.iter().any(|r| r.id == id)));
        let reply = host.take_reply("api2", id).unwrap();
        assert_eq!(
            headless::transform_text(&reply.result.unwrap()).unwrap(),
            "[hi]!"
        );
        let id = host
            .request(
                "api2",
                "transform.run",
                json!({"id": "Wrap", "text": "bad", "options": {}}),
            )
            .unwrap();
        assert!(pump(&mut host, |h| h.replies.iter().any(|r| r.id == id)));
        let err = host.take_reply("api2", id).unwrap().result.unwrap_err();
        assert_eq!(
            (err.message.as_str(), err.line, err.column),
            ("no good", Some(2), Some(3))
        );

        host.shutdown();
        let _ = fs::remove_dir_all(&root);
    }
}
