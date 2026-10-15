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

pub mod manifest;
pub mod panel;
pub mod process;
pub mod shortcut;
pub mod store;
pub mod ui;

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use egui::KeyboardShortcut;
use serde_json::{Value, json};

use crate::actions::Action;
use crate::settings::PluginPrefs;
use manifest::{Discovered, Manifest};
use panel::{Item, Panel, PanelEvent, Position};
use process::{Proc, ProcEvent, Waker};

/// The version of the plugin API, sent to plugins at start.
pub const API_VERSION: u32 = 1;

pub const SDK_PYTHON: &str = include_str!("../../plugins/sdk/ettin.py");
pub const SDK_JS: &str = include_str!("../../plugins/sdk/ettin.js");
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
];

const MAX_LOG: usize = 2000;
const MAX_COMMANDS: usize = 200;
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
}

pub struct Plugin {
    pub id: String,
    pub dir: PathBuf,
    pub manifest: Result<Manifest, String>,
    pub status: Status,
    /// The program that runs it, once started.
    pub program: Option<String>,
    pub commands: Vec<Command>,
    proc: Option<Proc>,
    /// Which start of the plugin `proc` is; messages from older ones are dropped.
    run: u64,
    events: HashSet<String>,
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
    /// Errors to show the user briefly.
    pub notices: Vec<String>,
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
        self.log.push_back(LogLine {
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            plugin: plugin.to_owned(),
            text,
            error,
        });
    }

    fn sdk_dir(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(".sdk"))
    }

    /// Writes the SDK modules plugins import, when they're missing or old.
    fn install_sdk(&mut self) -> Option<PathBuf> {
        let sdk = self.sdk_dir()?;
        if let Err(e) = fs::create_dir_all(&sdk) {
            self.log(
                "Ettin Edit",
                t!(
                    "make-folder-failed",
                    path = sdk.display().to_string(),
                    error = e.to_string()
                ),
                true,
            );
            return None;
        }
        for (name, text) in [
            ("ettin.py", SDK_PYTHON),
            ("ettin.js", SDK_JS),
            ("nixpad.py", SDK_PYTHON_OLD_NAME),
            ("nixpad.js", SDK_JS_OLD_NAME),
        ] {
            let path = sdk.join(name);
            if fs::read_to_string(&path).ok().as_deref() != Some(text)
                && let Err(e) = fs::write(&path, text)
            {
                self.log(
                    "Ettin Edit",
                    t!(
                        "write-failed",
                        path = path.display().to_string(),
                        error = e.to_string()
                    ),
                    true,
                );
            }
        }
        Some(sdk)
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
        self.plugins = found
            .into_iter()
            .map(|Discovered { id, dir, manifest }| Plugin {
                id,
                dir,
                status: match &manifest {
                    Ok(_) => Status::Stopped,
                    Err(e) => Status::Failed(e.clone()),
                },
                manifest,
                program: None,
                commands: Vec::new(),
                proc: None,
                run: 0,
                events: HashSet::new(),
            })
            .collect();
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

    /// Starts a plugin without checking permission.
    fn start(&mut self, index: usize, python: &str, node: &str) {
        let Some(sdk) = self.install_sdk() else {
            return;
        };
        let run = self.next_run;
        self.next_run += 1;
        let tx = self.events_tx.clone();
        let wake = self.wake.clone();
        let p = &mut self.plugins[index];
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
                    }),
                ));
                p.program = Some(proc.program.clone());
                p.proc = Some(proc);
                p.run = run;
                p.status = Status::Running;
                p.commands.clear();
                p.events.clear();
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
        p.commands.clear();
        p.events.clear();
        p.run = 0;
        let id = p.id.clone();
        self.panels.retain(|panel| panel.plugin != id);
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
                self.notices.push(msg);
            }
            None => {}
        }
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
            let Some(index) = self.plugins.iter().position(|p| p.run == run && run != 0) else {
                continue;
            };
            let id = self.plugins[index].id.clone();
            match ev {
                ProcEvent::Log(line) => self.log(&id, line, false),
                ProcEvent::Closed => self.ended(index),
                ProcEvent::Message(line) => match parse(&line) {
                    Ok((call_id, method, params)) => {
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
                    Err(e) => self.log(&id, format!("Bad message from plugin: {e}"), true),
                },
            }
        }
        calls
    }

    /// The plugin's output closed: it ended.
    fn ended(&mut self, index: usize) {
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
        p.commands.clear();
        p.events.clear();
        p.run = 0;
        let (id, name) = (p.id.clone(), p.name().to_owned());
        self.panels.retain(|panel| panel.plugin != id);
        self.log(&id, why.clone(), true);
        if code != Some(Some(0)) {
            self.notices.push(t!("plugin-stopped-notice", name = name));
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
                let name = self
                    .plugins
                    .iter()
                    .find(|p| p.id == call.plugin)
                    .map(|p| p.name().to_owned())
                    .unwrap_or_default();
                let first = text.lines().last().unwrap_or("error").to_owned();
                self.notices
                    .push(format!("{name}: {first} (see Plugins > Plugin Log)"));
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
        let Some(index) = self.index_of(&call.plugin) else {
            return Err((FAILED, "unknown plugin".to_owned()));
        };
        let p = &mut self.plugins[index];
        let count = p.commands.len();
        match p.commands.iter_mut().find(|c| c.id == id) {
            Some(c) => {
                c.title = title;
                c.shortcut = shortcut;
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
            }),
        }
        if let Some(w) = &warning {
            let plugin = call.plugin.clone();
            self.log(&plugin, format!("Shortcut not added: {w}"), true);
        }
        Ok(json!({ "shortcut": shortcut.is_some(), "warning": warning }))
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

/// Splits a JSON-RPC message into (id, method, params).
fn parse(line: &str) -> Result<(Option<Value>, String, Value), String> {
    let v: Value = serde_json::from_str(line).map_err(|e| format!("{e}: {}", clip(line)))?;
    let method = v
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("no \"method\": {}", clip(line)))?
        .to_owned();
    let id = v.get("id").filter(|id| !id.is_null()).cloned();
    let params = v.get("params").cloned().unwrap_or(Value::Null);
    Ok((id, method, params))
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

pub fn str_param<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_requests_and_notifications() {
        let (id, method, params) =
            parse(r#"{"jsonrpc":"2.0","id":3,"method":"text.get","params":{"doc":1}}"#).unwrap();
        assert_eq!(id, Some(json!(3)));
        assert_eq!(method, "text.get");
        assert_eq!(params["doc"], 1);
        let (id, method, _) = parse(r#"{"method":"log","params":{"text":"x"}}"#).unwrap();
        assert_eq!(id, None);
        assert_eq!(method, "log");
        assert!(parse("not json").is_err());
        assert!(parse(r#"{"id":1}"#).is_err());
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
        host.plugins.push(Plugin {
            id: id.to_owned(),
            dir: PathBuf::new(),
            manifest: Err("test".into()),
            status: Status::Running,
            program: None,
            commands: Vec::new(),
            proc: None,
            run: 1,
            events: HashSet::new(),
        });
        host
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
    fn registers_commands_and_checks_shortcuts() {
        let mut host = host_with("p");
        let r = host
            .register_command(&call(
                "commands.register",
                json!({"id": "a", "title": "A", "shortcut": "Ctrl+Alt+K"}),
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
                json!({"id": "c", "title": "C", "shortcut": "ctrl+alt+k"}),
            ))
            .unwrap();
        assert_eq!(r["shortcut"], false);
        let r = host
            .register_command(&call(
                "commands.register",
                json!({"id": "a", "title": "A2", "shortcut": "Ctrl+Alt+K"}),
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
}
