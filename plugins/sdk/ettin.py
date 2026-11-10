"""Ettin Edit plugin API for Python.

A plugin imports this module, adds its commands and event handlers, then
calls ``ettin.run()``::

    import ettin

    @ettin.command("Shout", shortcut="Ctrl+Alt+S")
    def shout():
        doc = ettin.active()
        doc.replace_selections(doc.selected_text().upper())

    ettin.run()

Ettin Edit starts the plugin as its own process and talks to it over standard
input and output, so ``print()`` goes to the plugin log instead (Plugins >
Plugin Log). Text positions count characters (Unicode code points) from the
start of the document, and lines end with ``\\n``.

A transform is a function from text to text. Ettin Edit runs it on each
selection (or the whole tab) as one undo step, previews it and records it
into recipes::

    @ettin.transform("Shout", options={"mark": "!"})
    def shout(text, options):
        return text.upper() + options["mark"]

The full guide is in Ettin Edit under Plugins > Plugin Guide.
"""

import json
import os
import sys
import traceback
from collections import deque
from collections.abc import MutableMapping

__all__ = [
    "EttinError", "TransformError", "Document", "Selection", "ui",
    "command", "transform", "on", "on_panel", "run",
    "active", "documents", "open_file", "new_document",
    "message", "prompt", "choose", "status", "set_clipboard", "run_action", "log",
    "show_panel", "close_panel", "call", "notify", "info",
    "settings", "data_dir",
]

# The protocol uses the real standard output; print() goes to the log.
_in = sys.stdin.buffer
_out = sys.stdout.buffer
sys.stdout = sys.stderr

_next_id = 0
_queue = deque()
_commands = {}
_transforms = {}
_handlers = {}
_panel_handlers = {}
#: What Ettin Edit sent at start: plugin id, Ettin Edit version, API version
#: and interface language.
info = {}


class EttinError(Exception):
    """A call Ettin Edit couldn't do, such as editing a read-only document."""


class TransformError(Exception):
    """Raise it from a transform to say why the text can't be transformed.
    ``line`` and ``column`` (from 1) point at the problem in the text; Ettin
    Edit shows them with the message."""

    def __init__(self, message, line=None, column=None):
        super().__init__(message)
        self.line = line
        self.column = column


def _send(msg):
    msg["jsonrpc"] = "2.0"
    _out.write(json.dumps(msg, ensure_ascii=False).encode("utf-8") + b"\n")
    _out.flush()


def _read():
    line = _in.readline()
    if not line:
        # Ettin Edit closed.
        raise SystemExit(0)
    return json.loads(line.decode("utf-8"))


def _clean(params):
    return {k: v for k, v in params.items() if v is not None}


def _ensure_init():
    """Reads Ettin Edit's first message, if it hasn't been read yet, so
    ``info`` and ``settings`` are filled in at import time too."""
    while not info:
        msg = _read()
        if msg.get("method") == "initialize":
            _dispatch(msg)
        else:
            _queue.append(msg)


def call(method, **params):
    """Calls an Ettin Edit API method and returns its result. Raises EttinError."""
    global _next_id
    _next_id += 1
    my_id = _next_id
    _send({"id": my_id, "method": method, "params": _clean(params)})
    while True:
        msg = _read()
        if "method" in msg:
            # A command or event that arrived meanwhile; handled after this.
            _queue.append(msg)
        elif msg.get("id") == my_id:
            if "error" in msg:
                raise EttinError(msg["error"].get("message", "error"))
            return msg.get("result")


def notify(method, **params):
    """Sends a message that has no answer."""
    _send({"method": method, "params": _clean(params)})


# ----------------------------------------------------------------------
# Documents


class Selection:
    """One selection: ``start`` and ``end`` (start <= end), and its text.
    ``anchor`` is where it began and ``head`` where the caret is."""

    __slots__ = ("start", "end", "anchor", "head", "text")

    def __init__(self, d):
        self.start = d["start"]
        self.end = d["end"]
        self.anchor = d.get("anchor", self.start)
        self.head = d.get("head", self.end)
        self.text = d.get("text", "")

    def __repr__(self):
        return "Selection(%d, %d, %r)" % (self.start, self.end, self.text)


class Document:
    """An open tab. Its properties are a snapshot from when you got it;
    ``refresh()`` updates them. Methods always act on the current text."""

    def __init__(self, info):
        self._info = info
        self.id = info["id"]

    def __repr__(self):
        return "Document(%d, %r)" % (self.id, self.title)

    def __eq__(self, other):
        return isinstance(other, Document) and other.id == self.id

    def __hash__(self):
        return hash(self.id)

    title = property(lambda self: self._info.get("title"), doc="Tab title")
    path = property(lambda self: self._info.get("path"), doc="File path, or None")
    language = property(lambda self: self._info.get("language"), doc="Highlighting language")
    modified = property(lambda self: self._info.get("modified"), doc="Has unsaved changes")
    read_only = property(lambda self: self._info.get("readOnly"))
    length = property(lambda self: self._info.get("length"), doc="Length in characters")
    line_count = property(lambda self: self._info.get("lineCount"))
    encoding = property(lambda self: self._info.get("encoding"))
    line_ending = property(lambda self: self._info.get("lineEnding"))

    @property
    def caret(self):
        """(line, column) of the main caret, both from 1."""
        c = self._info.get("caret") or {}
        return (c.get("line", 1), c.get("column", 1))

    @property
    def caret_offset(self):
        """Where the main caret is, as a text position."""
        return (self._info.get("caret") or {}).get("offset", 0)

    def refresh(self):
        self._info = call("documents.info", doc=self.id)
        return self

    @property
    def text(self):
        return self.get_text()

    @text.setter
    def text(self, value):
        self.set_text(value)

    def get_text(self, start=None, end=None):
        """The whole text, or the characters from start to end."""
        return call("text.get", doc=self.id, start=start, end=end)

    def set_text(self, text):
        """Replaces the whole text (one undo step)."""
        call("text.set", doc=self.id, text=text)

    def replace(self, start, end, text):
        """Replaces the characters from start to end (one undo step)."""
        call("text.replace", doc=self.id, start=start, end=end, text=text)

    def insert(self, text):
        """Types text at every caret, replacing what is selected."""
        call("text.insert", doc=self.id, text=text)

    def selections(self):
        """Every selection (or caret), the main one first."""
        return [Selection(s) for s in call("selection.get", doc=self.id)]

    def selected_text(self):
        """The text of all selections, joined with new lines."""
        return "\n".join(s.text for s in self.selections() if s.text)

    def replace_selections(self, text):
        """Replaces each selection: with the same text, or with one string
        per selection when given a list (one undo step)."""
        if isinstance(text, str):
            call("selection.replace", doc=self.id, text=text)
        else:
            call("selection.replace", doc=self.id, texts=list(text))

    def select(self, start, end=None):
        """Selects from start to end, or puts the caret at start."""
        self.select_ranges([(start, start if end is None else end)])

    def select_ranges(self, ranges):
        """Sets several selections: a list of (start, end) pairs."""
        call("selection.set", doc=self.id, ranges=[list(r) for r in ranges])

    def goto(self, line, column=1):
        """Moves the caret to a line and column (both from 1) and shows it."""
        call("editor.goto", doc=self.id, line=line, column=column)

    def save(self):
        """Saves the file; asks for a name if it has none. True if saved."""
        return call("documents.save", doc=self.id)

    def activate(self):
        """Shows this tab."""
        call("documents.activate", doc=self.id)

    def close(self):
        """Closes the tab (asks first if it has unsaved changes)."""
        call("documents.close", doc=self.id)

    def mark(self, id, ranges, style="underline-squiggle", color="error", message=None):
        """Marks text: ranges is a list of (start, end) pairs. style is
        "underline-squiggle", "background" or "gutter-icon"; color is
        "error", "warning", "info" or "accent". message shows when the
        mouse is over the mark. Marking with the same id again replaces
        those marks. Marks move with edits."""
        call("marks.set", doc=self.id, id=id, ranges=[list(r) for r in ranges],
             style=style, color=color, message=message)

    def clear_marks(self, id):
        """Removes the marks made with this id."""
        call("marks.clear", doc=self.id, id=id)


def active():
    """The document in the tab that has focus."""
    return Document(call("documents.active"))


def documents():
    """Every open document, in tab order."""
    return [Document(d) for d in call("documents.list")]


def open_file(path):
    """Opens a file in a tab (or shows it if it's open) and returns it."""
    return Document(call("documents.open", path=str(path)))


def new_document(text="", language=None):
    """Opens a new tab with this text, e.g. language="JSON"."""
    return Document(call("documents.new", text=text, language=language))


# ----------------------------------------------------------------------
# Messages and other editor features


def message(text, kind="info"):
    """Shows a short message; kind is "info", "warning" or "error"."""
    call("ui.message", text=str(text), kind=kind)


def prompt(text, value="", title=None):
    """Asks the user for a line of text. Returns None if they cancel."""
    return call("ui.prompt", prompt=str(text), value=value, title=title)


def set_clipboard(text):
    call("clipboard.set", text=str(text))


def run_action(action):
    """Runs one of Ettin Edit's own commands, like "Duplicate", "SelectAll" or
    {"SortLines": {"kind": "IgnoreCase", "desc": False}}."""
    call("editor.runAction", action=action)


def ui_language():
    """The language Ettin Edit's menus are in now, like "en" or "de". Use it to
    pick the language of your own commands and messages."""
    return call("app.language")


def log(*args):
    """Writes a line to the plugin log."""
    notify("log", text=" ".join(str(a) for a in args))


def choose(items, title=None, placeholder=None):
    """Shows a list to pick from by typing, like the command palette.
    items are strings, or dicts with "label" (and optionally "detail").
    Returns the chosen item, or None if the user pressed Escape."""
    items = list(items)
    index = call("ui.choose", items=[_choice(i) for i in items], title=title,
                 placeholder=placeholder)
    return None if index is None else items[index]


def _choice(item):
    if isinstance(item, dict):
        return {"label": str(item.get("label", "")), "detail": str(item.get("detail", ""))}
    return {"label": str(item), "detail": ""}


def status(id, text, tooltip=None, command=None):
    """Shows text on the status bar (or changes it). An empty text removes
    it. command is the id of one of your commands, run when it's clicked."""
    call("status.set", id=id, text=str(text or ""), tooltip=tooltip, command=command)


class Settings(MutableMapping):
    """Your plugin's settings: the options in plugin.json (changed in Manage
    Plugins > Options) and anything you store. Ettin Edit keeps them in its
    plugin-data folder, so they survive updates. Changing a key saves it."""

    def __init__(self):
        self._values = {}

    def _load(self):
        _ensure_init()
        return self._values

    def __getitem__(self, key):
        return self._load()[key]

    def __setitem__(self, key, value):
        self._load()
        self._values = call("settings.set", values={key: value}) or {}

    def __delitem__(self, key):
        raise TypeError("settings can't be removed; set a value instead")

    def __iter__(self):
        return iter(dict(self._load()))

    def __len__(self):
        return len(self._load())

    def __repr__(self):
        return "Settings(%r)" % (self._load(),)

    def update_all(self, values):
        """Saves several settings at once."""
        self._load()
        self._values = call("settings.set", values=dict(values)) or {}


#: The plugin's settings (see Settings).
settings = Settings()


def data_dir():
    """A folder for your plugin's own files, kept when it's updated."""
    _ensure_init()
    path = info.get("dataDir")
    if not path:
        raise EttinError("Ettin Edit has no folder for plugin data")
    os.makedirs(path, exist_ok=True)
    return path


# ----------------------------------------------------------------------
# Commands, events and panels


def command(title, shortcut=None, id=None, icon=None):
    """Decorator: adds a command to the Plugins menu, with an optional
    shortcut like "Ctrl+Alt+U". icon is shown when the user puts the command
    on the toolbar: a Phosphor icon name like "star", or a .png file in the
    plugin's folder. The function takes no arguments."""

    def decorate(fn):
        cid = id or fn.__name__
        _commands[cid] = fn
        # A shortcut that can't be added is reported in the plugin log.
        call("commands.register", id=cid, title=title, shortcut=shortcut, icon=icon)
        return fn

    return decorate


def transform(title, options=None, id=None, languages=None):
    """Decorator: adds a transform, a function ``fn(text, options)`` that
    returns the new text. Ettin Edit runs it on each selection (or the
    whole tab) as one undo step, previews it, and records it into recipes.
    options is a dict of option names and defaults; Ettin Edit asks for
    them before running it from a menu. languages (like ["JSON"]) puts it
    on the right-click menu of tabs in those languages. Raise
    TransformError to say why some text can't be transformed."""

    def decorate(fn):
        tid = id or title
        _transforms[tid] = (fn, dict(options or {}))
        langs = [languages] if isinstance(languages, str) else languages
        call("transforms.register", id=tid, title=title, options=dict(options or {}),
             languages=list(langs) if langs else None)
        return fn

    return decorate


EVENTS = ("opened", "saved", "closed", "activated", "changed", "caret", "shutdown",
          "settings.changed")


def on(event):
    """Decorator: calls the function with a Document when the event happens.
    Events: opened, saved, closed, activated (another tab got focus),
    changed (the text was edited; sent after a short pause in typing),
    caret (the caret or selection moved in the active tab; sent after a
    short pause), shutdown (no argument; Ettin Edit is closing or stopping the plugin)
    and settings.changed (called with the new settings dict, after the user
    changed your options)."""
    if event not in EVENTS:
        raise ValueError("unknown event %r; use one of %s" % (event, ", ".join(EVENTS)))

    def decorate(fn):
        _handlers.setdefault(event, []).append(fn)
        if event not in ("shutdown", "settings.changed"):
            call("events.subscribe", events=[event])
        return fn

    return decorate


def show_panel(id, title, items, position="right", scroll_to_top=False):
    """Shows a panel (or updates it, if it's open). items are made with the
    helpers in ``ui``. position is "right" or "bottom". scroll_to_top
    scrolls an open panel back to its start."""
    call("panel.show", id=id, title=title, items=items, position=position,
         scrollToTop=scroll_to_top)


def close_panel(id):
    call("panel.close", id=id)


def on_panel(panel_id):
    """Decorator: called with a dict when the user uses the panel:
    item (the item's id), event ("click", "change", "submit", "select",
    "link" or "close"), value, and values (every field's value, by id)."""

    def decorate(fn):
        _panel_handlers[panel_id] = fn
        return fn

    return decorate


class ui:
    """Builds panel items."""

    @staticmethod
    def heading(text):
        return {"type": "heading", "text": str(text)}

    @staticmethod
    def label(text, weak=False, mono=False):
        return {"type": "label", "text": str(text), "weak": weak, "mono": mono}

    @staticmethod
    def separator():
        return {"type": "separator"}

    @staticmethod
    def button(id, label):
        return {"type": "button", "id": id, "label": label}

    @staticmethod
    def input(id, label="", value="", hint="", multiline=False):
        return {"type": "input", "id": id, "label": label, "value": value,
                "hint": hint, "multiline": multiline}

    @staticmethod
    def checkbox(id, label, value=False):
        return {"type": "checkbox", "id": id, "label": label, "value": value}

    @staticmethod
    def select(id, options, value="", label=""):
        return {"type": "select", "id": id, "options": list(options),
                "value": value, "label": label}

    @staticmethod
    def row(*items):
        return {"type": "row", "items": list(items)}

    @staticmethod
    def tree(id, nodes, selected=None):
        """``selected`` is a node id to highlight, open and scroll to."""
        item = {"type": "tree", "id": id, "nodes": list(nodes)}
        if selected is not None:
            item["selected"] = selected
        return item

    @staticmethod
    def node(label, id="", detail="", children=None, open=None):
        n = {"label": str(label), "id": str(id), "detail": str(detail),
             "children": list(children or [])}
        if open is not None:
            n["open"] = open
        return n

    @staticmethod
    def markdown(text, id="", base=None, doc=None, line=0):
        """``base`` is the folder images with relative paths are in. With
        ``doc`` (a Document or its id), the panel scrolls along with that
        tab; ``line`` is the tab's line (from 0) the text starts on."""
        item = {"type": "markdown", "id": id, "text": str(text)}
        if base:
            item["base"] = str(base)
        if doc is not None:
            item["doc"] = doc.id if isinstance(doc, Document) else int(doc)
            item["line"] = int(line)
        return item

    @staticmethod
    def table(id, columns, rows):
        return {"type": "table", "id": id, "columns": list(columns),
                "rows": [list(r) for r in rows]}


def _report():
    text = traceback.format_exc()
    sys.stderr.write(text)
    sys.stderr.flush()
    try:
        notify("error", message=text.strip())
    except Exception:
        pass


def _run_transform(msg):
    """Answers a transform.run request: {"text": ...} or an error."""
    params = msg.get("params") or {}
    found = _transforms.get(params.get("id"))
    try:
        if found is None:
            raise TransformError("this plugin has no transform %r" % params.get("id"))
        fn, defaults = found
        # Options missing from an older recipe get their defaults.
        options = dict(defaults)
        options.update(params.get("options") or {})
        out = fn(params.get("text", ""), options)
        if not isinstance(out, str):
            raise TransformError("the transform returned %s, not text" % type(out).__name__)
        _send({"id": msg["id"], "result": {"text": out}})
    except Exception as e:
        data = {}
        line = getattr(e, "line", None) or getattr(e, "lineno", None)
        column = getattr(e, "column", None) or getattr(e, "colno", None)
        if isinstance(line, int):
            data["line"] = line
        if isinstance(column, int):
            data["column"] = column
        if not isinstance(e, TransformError):
            sys.stderr.write(traceback.format_exc())
            sys.stderr.flush()
        message = getattr(e, "msg", None) if isinstance(line, int) else None
        _send({"id": msg["id"], "error": {
            "code": -32000,
            "message": str(message or e) or type(e).__name__,
            "data": data,
        }})


def _dispatch(msg):
    """Handles one message from Ettin Edit. Returns False to stop."""
    method = msg.get("method")
    params = msg.get("params") or {}
    if method == "initialize":
        info.update(params)
        settings._values = dict(params.get("settings") or {})
    elif method == "transform.run" and "id" in msg:
        _run_transform(msg)
    elif method == "command":
        fn = _commands.get(params.get("id"))
        if fn:
            try:
                fn()
            except Exception:
                _report()
    elif method == "event":
        event = params.get("event")
        if event == "settings.changed":
            settings._values = dict(params.get("settings") or {})
            arg = dict(settings._values)
        else:
            doc = params.get("doc")
            arg = Document(doc) if doc else None
        for fn in _handlers.get(event, []):
            try:
                fn(arg)
            except Exception:
                _report()
    elif method == "panel":
        fn = _panel_handlers.get(params.get("panel"))
        if fn:
            try:
                fn(params)
            except Exception:
                _report()
    elif "id" in msg and method:
        # A request this version doesn't know.
        _send({"id": msg["id"], "error": {"code": -32601, "message": "unknown method %s" % method}})
    elif method == "shutdown":
        for fn in _handlers.get("shutdown", []):
            try:
                fn()
            except Exception:
                _report()
        return False
    return True


def run():
    """Handles commands and events until Ettin Edit stops the plugin."""
    try:
        # Every command and transform is added by now.
        notify("ready")
        while True:
            while _queue:
                if not _dispatch(_queue.popleft()):
                    return
            if not _dispatch(_read()):
                return
    except (SystemExit, KeyboardInterrupt, BrokenPipeError):
        return
