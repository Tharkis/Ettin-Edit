// Ettin Edit plugin API for JavaScript (Node).
//
// A plugin requires this module and adds its commands and event handlers:
//
//     const ettin = require('ettin');
//
//     ettin.command('Shout', async () => {
//       const doc = await ettin.activeDocument();
//       await doc.replaceSelections((await doc.getSelectedText()).toUpperCase());
//     }, { shortcut: 'Ctrl+Alt+S' });
//
// Every call to Ettin Edit returns a Promise. Ettin Edit starts the plugin as its own
// process and talks to it over standard input and output, so console.log()
// goes to the plugin log instead (Plugins > Plugin Log). Text positions count
// characters (Unicode code points) from the start of the document, and lines
// end with "\n".
//
// A transform is a function from text to text. Ettin Edit runs it on each
// selection (or the whole tab) as one undo step, previews it and records it
// into recipes:
//
//     ettin.transform('Upper Words', {}, (text) =>
//       text.replace(/\b\w/g, (c) => c.toUpperCase()));
//
// The full guide is in Ettin Edit under Plugins > Plugin Guide.

'use strict';

const readline = require('readline');

// The protocol uses the real standard output; console output goes to the log.
const writeOut = process.stdout.write.bind(process.stdout);
for (const name of ['log', 'info', 'debug']) {
  console[name] = (...args) => console.error(...args);
}

let nextId = 0;
const pending = new Map();
const commands = new Map();
const transforms = new Map();
const handlers = new Map();
const panelHandlers = new Map();
/** What Ettin Edit sent at start: plugin id, Ettin Edit version, API version. */
const info = {};
/** The plugin's settings: the options in plugin.json and anything stored with
 *  setSettings(). Filled in before any command, event or transform runs. */
const settings = {};

class EttinError extends Error {}

/** Throw it from a transform to say why the text can't be transformed;
 *  line and column (from 1) point at the problem in the text. */
class TransformError extends Error {
  constructor(message, { line, column } = {}) {
    super(message);
    this.line = line;
    this.column = column;
  }
}

function send(msg) {
  msg.jsonrpc = '2.0';
  writeOut(JSON.stringify(msg) + '\n');
}

function clean(params) {
  const out = {};
  for (const [k, v] of Object.entries(params || {})) {
    if (v !== undefined && v !== null) out[k] = v;
  }
  return out;
}

/** Calls an Ettin Edit API method. Resolves with its result. */
function call(method, params = {}) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    send({ id, method, params: clean(params) });
  });
}

/** Sends a message that has no answer. */
function notify(method, params = {}) {
  send({ method, params: clean(params) });
}

// ----------------------------------------------------------------------
// Documents

/** An open tab. Its properties are a snapshot from when you got it;
 *  refresh() updates them. Methods always act on the current text. */
class Document {
  constructor(info) {
    this._info = info;
    this.id = info.id;
  }
  get title() { return this._info.title; }
  get path() { return this._info.path; }
  get language() { return this._info.language; }
  get modified() { return this._info.modified; }
  get readOnly() { return this._info.readOnly; }
  get length() { return this._info.length; }
  get lineCount() { return this._info.lineCount; }
  get encoding() { return this._info.encoding; }
  get lineEnding() { return this._info.lineEnding; }
  /** {line, column, offset, anchor} of the main caret; line and column from 1. */
  get caret() { return this._info.caret; }

  async refresh() {
    this._info = await call('documents.info', { doc: this.id });
    return this;
  }
  /** The whole text, or the characters from start to end. */
  getText(start, end) { return call('text.get', { doc: this.id, start, end }); }
  /** Replaces the whole text (one undo step). */
  setText(text) { return call('text.set', { doc: this.id, text }); }
  /** Replaces the characters from start to end (one undo step). */
  replace(start, end, text) { return call('text.replace', { doc: this.id, start, end, text }); }
  /** Types text at every caret, replacing what is selected. */
  insert(text) { return call('text.insert', { doc: this.id, text }); }
  /** Every selection: {start, end, anchor, head, text}, the main one first. */
  getSelections() { return call('selection.get', { doc: this.id }); }
  /** The text of all selections, joined with new lines. */
  async getSelectedText() {
    const sels = await this.getSelections();
    return sels.filter((s) => s.text).map((s) => s.text).join('\n');
  }
  /** Replaces each selection with the same text, or with one string per
   *  selection when given an array (one undo step). */
  replaceSelections(text) {
    return Array.isArray(text)
      ? call('selection.replace', { doc: this.id, texts: text })
      : call('selection.replace', { doc: this.id, text });
  }
  /** Selects from start to end, or puts the caret at start. */
  select(start, end) { return this.selectRanges([[start, end === undefined ? start : end]]); }
  /** Sets several selections: an array of [start, end] pairs. */
  selectRanges(ranges) { return call('selection.set', { doc: this.id, ranges }); }
  /** Moves the caret to a line and column (both from 1) and shows it. */
  goTo(line, column = 1) { return call('editor.goto', { doc: this.id, line, column }); }
  /** Saves the file; asks for a name if it has none. Resolves true if saved. */
  save() { return call('documents.save', { doc: this.id }); }
  /** Shows this tab. */
  activate() { return call('documents.activate', { doc: this.id }); }
  /** Closes the tab (asks first if it has unsaved changes). */
  close() { return call('documents.close', { doc: this.id }); }
  /** Marks text: ranges is an array of [start, end] pairs. style:
   *  'underline-squiggle', 'background' or 'gutter-icon'; color: 'error',
   *  'warning', 'info' or 'accent'; message shows when the mouse is over it.
   *  Marking with the same id again replaces those marks. Marks move with edits. */
  mark(id, ranges, { style = 'underline-squiggle', color = 'error', message } = {}) {
    return call('marks.set', { doc: this.id, id, ranges, style, color, message });
  }
  /** Removes the marks made with this id. */
  clearMarks(id) { return call('marks.clear', { doc: this.id, id }); }
}

/** The document in the tab that has focus. */
async function activeDocument() { return new Document(await call('documents.active')); }
/** Every open document, in tab order. */
async function documents() { return (await call('documents.list')).map((d) => new Document(d)); }
/** Opens a file in a tab (or shows it if it's open). */
async function openFile(path) { return new Document(await call('documents.open', { path: String(path) })); }
/** Opens a new tab with this text, e.g. newDocument(text, 'JSON'). */
async function newDocument(text = '', language) {
  return new Document(await call('documents.new', { text, language }));
}

// ----------------------------------------------------------------------
// Messages and other editor features

/** Shows a short message; kind is 'info', 'warning' or 'error'. */
function message(text, kind = 'info') { return call('ui.message', { text: String(text), kind }); }
/** Asks the user for a line of text. Resolves null if they cancel. */
function prompt(text, { value = '', title } = {}) {
  return call('ui.prompt', { prompt: String(text), value, title });
}
function setClipboard(text) { return call('clipboard.set', { text: String(text) }); }
/** The language Ettin Edit's menus are in now, like 'en' or 'de'. */
function uiLanguage() { return call('app.language'); }
/** Runs one of Ettin Edit's own commands, like 'Duplicate', 'SelectAll' or
 *  {SortLines: {kind: 'IgnoreCase', desc: false}}. */
function runAction(action) { return call('editor.runAction', { action }); }
/** Writes a line to the plugin log. */
function log(...args) { notify('log', { text: args.map(String).join(' ') }); }

/** Shows a list to pick from by typing. items are strings or {label, detail}.
 *  Resolves the chosen item, or null if the user pressed Escape. */
async function choose(items, { title, placeholder } = {}) {
  const list = Array.from(items);
  const shown = list.map((i) => (typeof i === 'object' && i !== null
    ? { label: String(i.label ?? ''), detail: String(i.detail ?? '') }
    : { label: String(i), detail: '' }));
  const index = await call('ui.choose', { items: shown, title, placeholder });
  return index === null || index === undefined ? null : list[index];
}

/** Shows text on the status bar (or changes it); empty text removes it.
 *  command is the id of one of your commands, run when it's clicked. */
function status(id, text, { tooltip, command } = {}) {
  return call('status.set', { id, text: String(text ?? ''), tooltip, command });
}

/** Saves settings: an object of keys and values. Resolves all settings. */
async function setSettings(values) {
  const all = await call('settings.set', { values });
  for (const k of Object.keys(settings)) delete settings[k];
  Object.assign(settings, all || {});
  return settings;
}

/** A folder for your plugin's own files, kept when it's updated. */
function dataDir() {
  if (!info.dataDir) throw new EttinError('Ettin Edit has no folder for plugin data');
  require('fs').mkdirSync(info.dataDir, { recursive: true });
  return info.dataDir;
}

// ----------------------------------------------------------------------
// Commands, events and panels

/** Adds a command to the Plugins menu. options: {shortcut: 'Ctrl+Alt+U', id,
 *  icon}. icon is shown when the user puts the command on the toolbar: a
 *  Phosphor icon name like 'star', or a .png file in the plugin's folder. */
async function command(title, fn, options = {}) {
  const id = options.id || title;
  commands.set(id, fn);
  // A shortcut that can't be added is reported in the plugin log.
  return call('commands.register', { id, title, shortcut: options.shortcut, icon: options.icon });
}

/** Adds a transform: fn(text, options) returns (or resolves) the new text.
 *  Ettin Edit runs it on each selection (or the whole tab) as one undo step,
 *  previews it and records it into recipes. options is an object of option
 *  names and defaults; Ettin Edit asks for them before running it from a
 *  menu. extra: {id, languages}; languages (like ['JSON']) puts it on the
 *  right-click menu of tabs in those languages. */
function transform(title, options, fn, extra = {}) {
  if (typeof options === 'function') {
    [options, fn, extra] = [{}, options, fn || {}];
  }
  const id = extra.id || title;
  transforms.set(id, { fn, defaults: { ...(options || {}) } });
  const languages = typeof extra.languages === 'string' ? [extra.languages] : extra.languages;
  return call('transforms.register', { id, title, options: options || {}, languages });
}

const EVENTS = ['opened', 'saved', 'closed', 'activated', 'changed', 'caret', 'shutdown',
  'settings.changed'];

/** Calls fn(doc) when the event happens: opened, saved, closed, activated
 *  (another tab got focus), changed (the text was edited; sent after a short
 *  pause in typing), caret (the caret or selection moved in the active tab;
 *  sent after a short pause) or shutdown (no argument; the plugin is stopping). */
function on(event, fn) {
  if (!EVENTS.includes(event)) {
    throw new Error(`unknown event "${event}"; use one of ${EVENTS.join(', ')}`);
  }
  if (!handlers.has(event)) handlers.set(event, []);
  handlers.get(event).push(fn);
  return event === 'shutdown' || event === 'settings.changed'
    ? Promise.resolve()
    : call('events.subscribe', { events: [event] });
}

/** Shows a panel (or updates it, if it's open). position: 'right' or 'bottom'. */
/** Shows or updates a panel. scrollToTop scrolls an open panel back to its start. */
function showPanel(id, { title, items = [], position = 'right', scrollToTop = false } = {}) {
  return call('panel.show', { id, title, items, position, scrollToTop });
}
function closePanel(id) { return call('panel.close', { id }); }
/** Calls fn(event) when the user uses the panel: {item, event, value, values},
 *  where event is 'click', 'change', 'submit', 'select', 'link' or 'close'. */
function onPanel(panelId, fn) { panelHandlers.set(panelId, fn); }

/** Builds panel items. */
const ui = {
  heading: (text) => ({ type: 'heading', text: String(text) }),
  label: (text, { weak = false, mono = false } = {}) => ({ type: 'label', text: String(text), weak, mono }),
  separator: () => ({ type: 'separator' }),
  button: (id, label) => ({ type: 'button', id, label }),
  input: (id, { label = '', value = '', hint = '', multiline = false } = {}) =>
    ({ type: 'input', id, label, value, hint, multiline }),
  checkbox: (id, label, value = false) => ({ type: 'checkbox', id, label, value }),
  select: (id, options, { value = '', label = '' } = {}) => ({ type: 'select', id, options, value, label }),
  row: (...items) => ({ type: 'row', items }),
  tree: (id, nodes, { selected } = {}) =>
    selected === undefined ? { type: 'tree', id, nodes } : { type: 'tree', id, nodes, selected },
  node: (label, { id = '', detail = '', children = [], open } = {}) => {
    const n = { label: String(label), id: String(id), detail: String(detail), children };
    if (open !== undefined) n.open = open;
    return n;
  },
  table: (id, columns, rows) => ({ type: 'table', id, columns, rows }),
  // `base`: the folder images with relative paths are in. `doc` (a document
  // or its id): scroll along with that tab; `line`: its line (from 0) the
  // text starts on.
  markdown: (text, { id = '', base, doc, line = 0 } = {}) => {
    const item = { type: 'markdown', id, text: String(text) };
    if (base) item.base = String(base);
    if (doc !== undefined && doc !== null) {
      item.doc = typeof doc === 'object' ? doc.id : Number(doc);
      item.line = Number(line);
    }
    return item;
  },
};

// ----------------------------------------------------------------------
// Messages from Ettin Edit

function report(err) {
  const text = (err && err.stack) || String(err);
  console.error(text);
  try { notify('error', { message: text }); } catch (_) { /* closing */ }
}

async function runHandler(fn, ...args) {
  try {
    await fn(...args);
  } catch (err) {
    report(err);
  }
}

async function shutdown() {
  for (const fn of handlers.get('shutdown') || []) await runHandler(fn);
  process.exit(0);
}

async function runTransform(msg) {
  const params = msg.params || {};
  const found = transforms.get(params.id);
  try {
    if (!found) throw new TransformError(`this plugin has no transform "${params.id}"`);
    // Options missing from an older recipe get their defaults.
    const out = await found.fn(params.text ?? '', { ...found.defaults, ...(params.options || {}) });
    if (typeof out !== 'string') throw new TransformError(`the transform returned ${typeof out}, not text`);
    send({ id: msg.id, result: { text: out } });
  } catch (err) {
    if (!(err instanceof TransformError)) console.error((err && err.stack) || String(err));
    const data = {};
    if (Number.isInteger(err && err.line)) data.line = err.line;
    if (Number.isInteger(err && err.column)) data.column = err.column;
    send({ id: msg.id, error: { code: -32000, message: (err && err.message) || String(err), data } });
  }
}

function dispatch(msg) {
  const params = msg.params || {};
  if (msg.method === 'transform.run' && msg.id !== undefined) {
    runTransform(msg);
    return;
  }
  switch (msg.method) {
    case 'initialize':
      Object.assign(info, params);
      Object.assign(settings, params.settings || {});
      break;
    case 'command': {
      const fn = commands.get(params.id);
      if (fn) runHandler(fn);
      break;
    }
    case 'event': {
      let arg = params.doc ? new Document(params.doc) : null;
      if (params.event === 'settings.changed') {
        for (const k of Object.keys(settings)) delete settings[k];
        Object.assign(settings, params.settings || {});
        arg = { ...settings };
      }
      for (const fn of handlers.get(params.event) || []) runHandler(fn, arg);
      break;
    }
    case 'panel': {
      const fn = panelHandlers.get(params.panel);
      if (fn) runHandler(fn, params);
      break;
    }
    case 'shutdown':
      shutdown();
      break;
    default:
      if (msg.id !== undefined) {
        send({ id: msg.id, error: { code: -32601, message: `unknown method ${msg.method}` } });
      }
      break;
  }
}

const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
input.on('line', (line) => {
  if (!line.trim()) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (err) {
    console.error('ettin: bad message from Ettin Edit:', line);
    return;
  }
  if (msg.method) {
    dispatch(msg);
  } else if (pending.has(msg.id)) {
    const { resolve, reject } = pending.get(msg.id);
    pending.delete(msg.id);
    if (msg.error) reject(new EttinError(msg.error.message || 'error'));
    else resolve(msg.result);
  }
});
// Ettin Edit closed.
input.on('close', () => shutdown());
process.on('unhandledRejection', report);
// Runs once the plugin's own script has finished loading: every command and
// transform is added by then.
setImmediate(() => notify('ready'));

/** Kept for symmetry with Python; a Node plugin keeps running on its own. */
function run() {}

module.exports = {
  EttinError, TransformError, Document, ui, info, settings,
  command, transform, on, onPanel, run,
  activeDocument, documents, openFile, newDocument,
  message, prompt, choose, status, setClipboard, runAction, uiLanguage, log,
  showPanel, closePanel, call, notify, setSettings, dataDir,
};
