# Writing Ettin Edit plugins

A plugin adds commands, shortcuts and panels to Ettin Edit. You write it in
Python or JavaScript, whichever you prefer.

## Quick start

1. Choose **Plugins > Install Sample Plugins**. Ettin Edit copies three working
   plugins into your plugins folder and starts them:
   - **Text Statistics** (Python)
   - **Markdown Outline** (JavaScript)
   - **HL7 Viewer** (Python): a bigger example, with a tree panel, an
     inspector panel and its own unit tests
2. Try them from the **Plugins** menu.
3. Choose **Plugins > Open Plugins Folder**, copy one of the sample folders,
   rename it and edit it.
4. Choose **Plugins > Reload Plugins** to run your changes.

The plugins folder is `~/.config/EttinEdit/plugins` on Linux and
`%APPDATA%\EttinEdit\plugins` on Windows.

Ettin Edit used to be called NixPad. Plugins written for it that `import
nixpad` or `require('nixpad')` keep working; new plugins use `ettin`.

## What you need

- Python plugins run with the Python 3 installed on your computer
  (`python3`, or `python` / `py` on Windows).
- JavaScript plugins run with [Node.js](https://nodejs.org) 18 or newer
  (`node`).

If Ettin Edit can't find one of them, set its path in **Plugins > Manage
Plugins**. Plugins can use any package you install for that Python or
Node, such as one from pip or npm.

## A plugin's folder

Each plugin is a folder in the plugins folder. The folder name is the
plugin's id.

```
plugins/
  shout/
    plugin.json
    main.py
```

`plugin.json` describes the plugin:

```json
{
  "name": "Shout",
  "version": "1.0.0",
  "description": "Makes the selection upper case.",
  "author": "You",
  "main": "main.py"
}
```

| Field | Meaning |
|-------|---------|
| `name` | Shown in menus. Defaults to the folder name. |
| `main` | The script to run, inside the folder. Defaults to `main.py` or `main.js`. |
| `runtime` | `"python"` or `"node"`. Only needed when `main` doesn't end in `.py` or `.js`. |
| `version`, `description`, `author` | Shown in Manage Plugins. |

Folders whose names start with `.` or `_` are skipped. Rename a folder to
`_shout` to hide a plugin without deleting it.

## Permission to run

A plugin is a program that runs with your user account. It can read and
change your open documents and any of your files, and it can use the
network. Only run plugins you trust.

The first time Ettin Edit finds a plugin, it asks whether to let it run. It asks
again whenever any file in the plugin's folder changes. If you choose
**Don't Allow**, the plugin is turned off. You can turn it back on in
**Plugins > Manage Plugins**.

Ettin Edit doesn't ask about the sample plugins you install from the Plugins
menu. While you write a plugin of your own, tick **Don't ask when it
changes** for it in Manage Plugins, so reloading it doesn't ask each time.

## Sharing plugins

**Plugins > Manage Plugins** has two tabs. **Installed** lists your plugins,
with **Remove…** (the folder moves to the trash) and **Update to x.y** when
the plugin index has a newer version. **Available** lists the plugins in the
index, with a filter box and an **Install** button. **Install from File…**
installs a `.zip`, and **Install from Folder…** copies a folder that has a
`plugin.json`. A plugin you install or update always asks for permission
before it runs, even if you ticked **Don't ask when it changes**.

To share a plugin, zip it. `plugin.json` can be at the top of the zip or
inside one folder:

```
shout.zip                 shout.zip
  plugin.json               shout/
  main.py                     plugin.json
                              main.py
```

The plugin's id (its folder name) comes from the index, otherwise from the
folder in the zip, otherwise from the zip's file name. Zips with absolute
paths or `..` in them are refused.

The index is a JSON file; Ettin Edit reads
`https://raw.githubusercontent.com/Tharkis/Ettin-Edit/main/plugins/index.json`
unless you change the address at the bottom of the Available tab. To list a
plugin there, add an entry to `plugins/index.json` in the Ettin Edit
repository and open a pull request:

```json
{
  "plugins": [
    {
      "id": "shout",
      "name": "Shout",
      "version": "1.0.0",
      "description": "Makes the selection upper case.",
      "author": "You",
      "homepage": "https://example.com/shout",
      "download": "https://example.com/shout-1.0.0.zip",
      "sha256": "optional: the zip's SHA-256, checked before installing",
      "min_app_version": "optional: the oldest Ettin Edit it works with"
    }
  ]
}
```

Versions are compared number by number, so `1.10` is newer than `1.9`. A
`download` address without `https://` is relative to the index itself, so
you can host your own index next to your zips. Each Ettin Edit release
attaches `<id>.zip` for the sample plugins.

## A Python plugin

```python
import ettin

@ettin.command("Shout", shortcut="Ctrl+Alt+H")
def shout():
    doc = ettin.active()
    doc.replace_selections(doc.selected_text().upper())

@ettin.on("saved")
def saved(doc):
    ettin.log("Saved", doc.path)

ettin.run()
```

The script must end with `ettin.run()`. Calls are ordinary function calls
that return when Ettin Edit has answered.

## A JavaScript plugin

```js
const ettin = require('ettin');

ettin.command('Shout', async () => {
  const doc = await ettin.activeDocument();
  await doc.replaceSelections((await doc.getSelectedText()).toUpperCase());
}, { shortcut: 'Ctrl+Alt+H' });

ettin.on('saved', (doc) => ettin.log('Saved', doc.path));
```

Every call returns a Promise, so use `await`. Use `require`, not `import`:
Ettin Edit makes the `ettin` module available through `NODE_PATH`, and ES
modules don't read that.

## Printing and errors

Ettin Edit and the plugin talk over the plugin's standard input and output.
`print()` in Python and `console.log()` in JavaScript are sent to the
plugin log instead (**Plugins > Plugin Log**), so use them freely while
debugging.

If a command or event handler throws an error, Ettin Edit shows a short message
and writes the full traceback to the log. The plugin keeps running.

If the plugin itself stops, **Manage Plugins** shows why. Fix it, then
click **Start** or choose **Reload Plugins**.

## Text positions

Positions count characters (Unicode code points) from the start of the
document, beginning at 0. Lines end with `\n` whatever the file uses on
disk. Line and column numbers begin at 1.

In Python, string indexes already count code points, so positions match
`text[start:end]`. JavaScript strings count UTF-16 units, which differ only
for characters outside the Basic Multilingual Plane, such as emoji. Use
`Array.from(text)` to index by code point when that matters.

## API reference

Python names are shown first, JavaScript names second.

### Commands and events

| Python | JavaScript | What it does |
|--------|------------|--------------|
| `@command(title, shortcut=None, id=None)` | `command(title, fn, {shortcut, id})` | Adds a command to the Plugins menu, with an optional shortcut. |
| `@on(event)` | `on(event, fn)` | Calls the handler with a `Document` when the event happens. |
| `run()` | (not needed) | Handles commands and events until Ettin Edit stops the plugin. |

Shortcuts are written like `Ctrl+Alt+U`, `Ctrl+Shift+F9` or `F6`. They
need Ctrl or Alt unless they use a function key. A shortcut Ettin Edit or
another plugin already uses isn't added; the log says why.

Events:

| Event | When |
|-------|------|
| `opened` | A tab opened. |
| `saved` | A document was saved. |
| `closed` | A tab closed. The document can no longer be used. |
| `activated` | Another tab got focus. |
| `changed` | The text was edited. Sent after a short pause in typing. |
| `caret` | The caret or selection moved in the active tab. Sent after a short pause. |
| `shutdown` | Ettin Edit is closing or stopping the plugin. No argument. |

### Documents

| Python | JavaScript | What it does |
|--------|------------|--------------|
| `active()` | `activeDocument()` | The document in the tab that has focus. |
| `documents()` | `documents()` | Every open document, in tab order. |
| `open_file(path)` | `openFile(path)` | Opens a file, or shows it if it's already open. |
| `new_document(text="", language=None)` | `newDocument(text, language)` | Opens a new tab. `language` is a name from the Language menu, such as `"JSON"`. |

A `Document` has these properties, read when you got it (`refresh()`
updates them): `id`, `title`, `path` (`None`/`null` for a new tab),
`language`, `modified`, `read_only`/`readOnly`, `length`,
`line_count`/`lineCount`, `encoding`, `line_ending`/`lineEnding` and
`caret` (line and column of the main caret) and `caret_offset` /
`caret.offset` (its text position).

| Python | JavaScript | What it does |
|--------|------------|--------------|
| `get_text(start=None, end=None)`, `text` | `getText(start, end)` | The whole text, or part of it. |
| `set_text(text)`, `text = ...` | `setText(text)` | Replaces the whole text. |
| `replace(start, end, text)` | `replace(start, end, text)` | Replaces part of the text. |
| `insert(text)` | `insert(text)` | Types text at every caret, replacing what is selected. |
| `selections()` | `getSelections()` | Each selection: `start`, `end`, `anchor`, `head` and `text`. The main one is first. |
| `selected_text()` | `getSelectedText()` | All selected text, joined with new lines. |
| `replace_selections(text_or_list)` | `replaceSelections(textOrArray)` | Replaces each selection with the same text, or one string per selection. |
| `select(start, end=None)` | `select(start, end)` | Selects a range, or moves the caret. |
| `select_ranges([(start, end), ...])` | `selectRanges([[start, end], ...])` | Makes several selections. |
| `goto(line, column=1)` | `goTo(line, column)` | Moves the caret and scrolls to it. |
| `save()` | `save()` | Saves. Asks for a name if the tab has none. Returns whether it saved. |
| `activate()` | `activate()` | Shows the tab. |
| `close()` | `close()` | Closes the tab. Asks first if there are unsaved changes. |

Each edit is one undo step, so **Undo** reverses it. Editing a read-only
document raises an error.

### Messages, input and more

| Python | JavaScript | What it does |
|--------|------------|--------------|
| `message(text, kind="info")` | `message(text, kind)` | Shows a short message. `kind` is `info`, `warning` or `error`. |
| `prompt(text, value="", title=None)` | `prompt(text, {value, title})` | Asks for a line of text. Returns `None`/`null` if cancelled. |
| `set_clipboard(text)` | `setClipboard(text)` | Copies text. |
| `run_action(action)` | `runAction(action)` | Runs one of Ettin Edit's own commands (see below). |
| `log(*values)` | `log(...values)` | Writes a line to the plugin log. |

`run_action` takes the name of an Ettin Edit command, such as `"Duplicate"`,
`"SelectAll"`, `"Save"`, `"FindNext"` or `"ToggleWordWrap"`. Commands that
need options take an object, such as
`{"SortLines": {"kind": "IgnoreCase", "desc": false}}` or
`{"ConvertCase": "Upper"}`. The names are the ones in Ettin Edit's
`src/actions.rs`.

## Panels

A panel is a side or bottom panel that your plugin describes as a list of
items. Ettin Edit draws it. When the user clicks or changes something, your
handler is called.

```python
from ettin import ui

ettin.show_panel("todo", "To Do", [
    ui.input("task", hint="New task"),
    ui.button("add", "Add"),
    ui.table("list", ["Task", "Done"], [["Write docs", "no"]]),
])

@ettin.on_panel("todo")
def todo(event):
    if event["item"] == "add":
        ettin.message("Adding " + event["values"]["task"])
```

```js
const { ui } = ettin;
await ettin.showPanel('todo', {
  title: 'To Do',
  items: [ui.input('task', { hint: 'New task' }), ui.button('add', 'Add')],
});
ettin.onPanel('todo', (e) => {
  if (e.item === 'add') ettin.message(`Adding ${e.values.task}`);
});
```

Calling `show_panel` again with the same id updates the panel in place,
keeping where it is scrolled to; pass `scroll_to_top=True` (`scrollToTop:
true` in JavaScript) to go back to the start, for example when the panel
now shows something else.
`position` is `"right"` (the default) or `"bottom"`. `close_panel(id)` /
`closePanel(id)` closes it.

Items:

| Helper | JSON | Notes |
|--------|------|-------|
| `ui.heading(text)` | `{"type": "heading", "text"}` | |
| `ui.label(text, weak, mono)` | `{"type": "label", "text", "weak", "mono"}` | `weak` dims it; `mono` uses the code font. |
| `ui.separator()` | `{"type": "separator"}` | |
| `ui.button(id, label)` | `{"type": "button", "id", "label"}` | Sends `click`. |
| `ui.input(id, label, value, hint, multiline)` | `{"type": "input", ...}` | Sends `submit` when Enter is pressed. |
| `ui.checkbox(id, label, value)` | `{"type": "checkbox", ...}` | Sends `change` with `true` or `false`. |
| `ui.select(id, options, value, label)` | `{"type": "select", ...}` | A drop-down list. Sends `change` with the option. |
| `ui.row(*items)` | `{"type": "row", "items"}` | Puts items side by side. |
| `ui.tree(id, nodes, selected)` | `{"type": "tree", "id", "nodes", "selected"}` | Sends `select` with the node's id. `selected` (optional) is a node id to highlight; when it changes, the tree opens to it and scrolls it into view. In JavaScript: `ui.tree(id, nodes, { selected })`. |
| `ui.node(label, id, detail, children, open)` | `{"label", "id", "detail", "children", "open"}` | A tree node. `detail` is dimmed text after the label. |
| `ui.table(id, columns, rows)` | `{"type": "table", "id", "columns", "rows"}` | Click a header to sort. Sends `select` with the row's index in your `rows`. |

The handler gets: `item` (the item's id), `event` (`click`, `change`,
`submit`, `select` or `close`), `value`, and `values` (every input,
checkbox and select, by id). `close` means the user closed the panel.

Updating a panel keeps what the user typed in a field, unless you send a
different `value` for it.

## The protocol

You can write a plugin in another language by speaking the protocol
directly. Ettin Edit starts `main` and sends JSON-RPC 2.0 messages on its
standard input, one per line, UTF-8. The plugin sends requests and
notifications on its standard output, one per line. Standard error goes to
the log. Python plugins are run with `python3 -u main.py` and JavaScript
plugins with `node main.js`, in the plugin's folder.

Ettin Edit sends these notifications:

- `initialize` `{plugin, ettin, api}`. Always first. `api` is 1.
- `command` `{id}`.
- `event` `{event, doc}`.
- `panel` `{panel, item, event, value, values}`.
- `shutdown` `{}`. Finish within a second, or the plugin is stopped.

The plugin can call these methods. `doc` is a document id; it defaults to
the active tab.

| Method | Params | Result |
|--------|--------|--------|
| `commands.register` | `id, title, shortcut?` | `{shortcut, warning}` |
| `events.subscribe` | `events: [names]` | |
| `documents.list` / `documents.active` / `documents.info` | `doc?` | document info |
| `documents.open` | `path` | document info |
| `documents.new` | `text?, language?` | document info |
| `documents.activate` / `documents.close` | `doc?` | |
| `documents.save` | `doc?` | `true` if saved |
| `text.get` | `doc?, start?, end?` | string |
| `text.set` | `doc?, text` | |
| `text.replace` | `doc?, start, end, text` | |
| `text.insert` | `doc?, text` | |
| `selection.get` | `doc?` | `[{start, end, anchor, head, text}]` |
| `selection.set` | `doc?, ranges: [[start, end]]` | |
| `selection.replace` | `doc?, text` or `texts` | |
| `editor.goto` | `doc?, line, column?` | |
| `editor.runAction` | `action` | |
| `ui.message` | `text, kind?` | |
| `ui.prompt` | `prompt, value?, title?` | string or `null` |
| `clipboard.set` | `text` | |
| `panel.show` | `id, title?, items, position?` | |
| `panel.close` | `id` | |

Notifications the plugin can send: `log` `{text, level?}` and
`error` `{message}`, which also shows a short message to the user.
