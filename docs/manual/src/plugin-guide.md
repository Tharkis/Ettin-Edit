# Writing Ettin Edit plugins

A plugin adds transforms, commands, shortcuts and panels to Ettin Edit. You
write it in Python or JavaScript, whichever you prefer.

Most plugins are best written as a **transform**: a function that takes
text and returns new text. Ettin Edit does the rest. It runs the function on
each selection (or the whole tab) as one undo step, shows a preview, asks
for the options, lets the user send the result to a new tab or the
clipboard, and lets them use it as a [recipe](recipes.md) step, even from
the command line.

## Quick start

1. Choose **Plugins > New Plugin…**. Give it a name, pick Python or
   JavaScript and what it starts with (a transform, a command or a panel),
   and click **Create and Open**. Ettin Edit writes a working plugin to your
   plugins folder, starts it and opens its main file.
2. Try it: a transform is in **Transform > Plugin Transforms**, a command in
   the **Plugins** menu.
3. Edit the main file and save. With **Reload when its files change** ticked
   (it is, for a new plugin), Ettin Edit restarts the plugin within a second,
   so your change is ready to try.

To learn from bigger examples, choose **Plugins > Install Sample Plugins**.
Ettin Edit copies four working plugins into your plugins folder and starts
them:

- **Text Statistics** (Python)
- **Markdown Outline** (JavaScript)
- **Markdown Preview (plugin example)** (Python): how the built-in
  [Markdown Preview](view.md#markdown-preview) could be written as a plugin,
  with a `markdown` panel that updates as you type
- **HL7 Viewer** (Python): a bigger example, with a tree panel, an
  inspector panel, a transform (**Explode HL7 Segments**) and its own unit
  tests

You can also copy one of the sample folders (**Plugins > Open Plugins
Folder**), rename it and edit it; **Plugins > Reload Plugins** runs your
changes.

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
| `options` | Settings the user can change in Manage Plugins (see [Settings and options](#settings-and-options)). |

A new plugin from **Plugins > New Plugin…** also gets `ettin.pyi` (Python)
or `ettin.d.ts` (JavaScript) next to its main file: type hints for the whole
API, so editors such as VS Code or PyCharm can complete names and check
calls. The same files are in the `.sdk` folder inside the plugins folder.

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
menu, or about a plugin you make with **New Plugin…**. While you write a
plugin of your own, tick **Don't ask when it changes** for it in Manage
Plugins, so reloading it doesn't ask each time.

## Reloading as you write

Tick **Reload when its files change** on a plugin's card in **Plugins >
Manage Plugins** (a plugin made with **New Plugin…** has it ticked). Ettin
Edit then watches the plugin's folder and restarts the plugin half a second
after you stop saving files there, so a change is running within a second.
The plugin log says when it reloaded.

If the plugin fails to start, or an error stops it, Ettin Edit shows the
error with the file and line it happened on, and an **Open** button that
opens that file at that line. The full traceback is in the plugin log.

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

## Submitting a plugin to the index

The **Available** tab lists the plugins in the plugin index. While it holds
only the sample plugins, the tab says so and links here. To list your plugin:

1. Zip it as shown above and put the zip somewhere with a stable `https://`
   address, such as a release on your own GitHub repository.
2. Fork [Ettin Edit on GitHub](https://github.com/Tharkis/Ettin-Edit), add an
   entry for your plugin to `plugins/index.json` (example below), and open a
   pull request. Say in it what the plugin does and which runtime it needs.
3. Once the pull request is merged, everyone sees your plugin under
   **Available** after **Refresh**.

The index is a JSON file; Ettin Edit reads
`https://raw.githubusercontent.com/Tharkis/Ettin-Edit/main/plugins/index.json`
unless you change the address at the bottom of the Available tab. Each entry
looks like this:

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

## Transforms

A transform is a function that takes the text and its options and returns
the new text. Five lines are enough:

```python
import ettin

@ettin.transform("Reverse Lines", options={"skip_blank": False})
def reverse(text, options):
    lines = text.split("\n")
    return "\n".join(reversed(lines))

ettin.run()
```

```js
const ettin = require('ettin');
ettin.transform('Reverse Lines', { skipBlank: false },
  (text, options) => text.split('\n').reverse().join('\n'));
```

A transform appears:

- in **Transform > Plugin Transforms**, under the plugin's name;
- in the [command palette](view.md#command-palette) and the Shortcut
  Mapper, so it can have a shortcut;
- in the recipe window's **Add Step** list, so it can be a recipe step;
- on the right-click menu of tabs in the languages you list
  (`languages=["JSON"]` / `{ languages: ['JSON'] }`).

When the user runs it, Ettin Edit sends the text of each selection (or the
whole tab when nothing is selected) to the function, one call per
selection, and puts the results back as **one undo step**. A transform with
options opens a dialog first: a form for the options, a live preview of what
the transform makes of the first lines, and **Result goes to** (this tab, a
new tab or the clipboard). A transform without options runs straight away.
Recording a recipe records the transform with the options used.

`options` is a dict (an object in JavaScript) of option names and their
defaults. A true/false default gets a check box, a number a number box and
text a text box (where `\n` stands for a line break). Your function always
gets every option: the user's values, or the defaults for any they didn't
set.

When the text can't be transformed, raise `ettin.TransformError` (throw
`new ettin.TransformError(message, { line, column })` in JavaScript). Ettin
Edit shows the message and, when you give a `line` (and `column`), from 1,
counted in the text you were given, moves the caret there. Any other error
shows its message; the traceback goes to the plugin log. A
`json.JSONDecodeError` from Python's `json` module is reported with its line
and column automatically.

A transform has 10 seconds. After half a second Ettin Edit shows that it's
running, with a **Stop** button; after 10 seconds it gives up and leaves
the text as it was.

The optional `id` (the title by default) names the transform in recipes,
so keep it the same when you rename the title. A recipe that uses a
plugin's transform runs it from the command line too (`ettin-edit --recipe
NAME file`); Ettin Edit starts the plugin for it, if it is allowed to run.
If the plugin isn't installed or allowed, the recipe stops with an error
that says which plugin it needs.

## Settings and options

Each plugin has a settings store, kept in `plugin-data/<id>/settings.json`
in Ettin Edit's config folder, next to the plugins folder.
`ettin.settings` reads like a dict (a plain object in JavaScript) and is
filled in before any command, event or transform runs. In Python,
assigning saves straight away (`ettin.settings["width"] = 80`); in
JavaScript call `await ettin.setSettings({ width: 80 })`.

`ettin.data_dir()` / `ettin.dataDir()` is a folder for the plugin's own
files (caches, downloaded data). It is created when first asked for and
kept when the plugin is updated, but removed with it.

To let users change settings without editing code, list them in
`plugin.json`:

```json
{
  "name": "Wrap",
  "options": [
    { "key": "width", "title": "Wrap at column", "type": "number", "default": 80 },
    { "key": "style", "title": "Style", "type": "choice",
      "choices": ["hard", "soft"], "default": "hard" },
    { "key": "keep_indent", "title": "Keep indentation", "type": "bool", "default": true,
      "description": "Wrapped lines start at the same column." }
  ]
}
```

`type` is `string`, `bool`, `number` or `choice`. The plugin's card in
**Manage Plugins** then has an **Options…** button that opens a form for
them. Changes are saved straight away and the plugin gets a
`settings.changed` event with the new settings:

```python
@ettin.on("settings.changed")
def changed(settings):
    ettin.log("Width is now", settings["width"])
```

## Marks

A plugin can mark ranges of a document, for example to show where a
linter found problems:

```python
doc = ettin.active()
doc.mark("lint", [(10, 14), (52, 60)], style="underline-squiggle",
         color="warning", message="Unknown field")
doc.clear_marks("lint")
```

```js
await doc.mark('lint', [[10, 14], [52, 60]], { style: 'underline-squiggle', color: 'warning', message: 'Unknown field' });
await doc.clearMarks('lint');
```

`style` is `underline-squiggle` (the default), `background` or
`gutter-icon` (a small diamond beside the line). `color` is `error` (the
default), `warning`, `info` or `accent`, in the theme's colours. Hovering
over a mark shows its `message`. Marks move with the text as it is edited,
and calling `mark` again with the same id replaces that set. A plugin's
marks go away when it stops.

## Status bar items

```python
ettin.status("words", "1,204 words", tooltip="Words in this tab", command="count")
ettin.status("words", "")   # removes it
```

```js
await ettin.status('words', '1,204 words', { tooltip: 'Words in this tab', command: 'count' });
```

Shows short text on the right of the status bar. Clicking it runs the
plugin's command with that id: the `id` given to `command`, or else the
function's name in Python (`count` above) and the title in JavaScript.
A plugin can show up to four items.

## Picking from a list

```python
pick = ettin.choose(["UTF-8", "Latin-1", {"label": "UTF-16", "detail": "two bytes"}],
                    title="Encoding", placeholder="Type to filter")
```

```js
const pick = await ettin.choose(['UTF-8', 'Latin-1'], { title: 'Encoding' });
```

Shows a list the user filters by typing, like the command palette. Returns
the item picked (the string, or the dict/object) or `None`/`null` when the
user closes it.

## Printing and errors

Ettin Edit and the plugin talk over the plugin's standard input and output.
`print()` in Python and `console.log()` in JavaScript are sent to the
plugin log instead (**Plugins > Plugin Log**), so use them freely while
debugging.

If a command or event handler throws an error, Ettin Edit shows a short message
and writes the full traceback to the log. The plugin keeps running.

If the plugin itself stops, Ettin Edit shows the error with the file and
line it happened on, and an **Open** button that opens the file there.
**Manage Plugins** also shows why. Fix it, then click **Start** or choose
**Reload Plugins** (or let **Reload when its files change** do it).

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
| `@transform(title, options=None, id=None, languages=None)` | `transform(title, options, fn, {id, languages})` | Adds a transform (see [Transforms](#transforms)). |
| `@command(title, shortcut=None, id=None, icon=None)` | `command(title, fn, {shortcut, id, icon})` | Adds a command to the Plugins menu, with an optional shortcut and toolbar icon. |
| `@on(event)` | `on(event, fn)` | Calls the handler with a `Document` when the event happens. |
| `run()` | (not needed) | Handles commands and events until Ettin Edit stops the plugin. |

Shortcuts are written like `Ctrl+Alt+U`, `Ctrl+Shift+F9` or `F6`. They
need Ctrl or Alt unless they use a function key. A shortcut Ettin Edit or
another plugin already uses isn't added; the log says why.

Every command is also in the [command palette](view.md#command-palette),
under its title with *Plugin · your plugin's name* after it, so give
commands titles that say what they do on their own ("Explode JSON Lines"
rather than "Run").

Users can put any plugin command on the [toolbar](toolbar.md). The `icon`
is what its button shows there: a [Phosphor](https://phosphoricons.com/)
icon name such as `"star"` or `"chart-bar"`, or a `.png` file inside your
plugin's folder, given relative to it (`"icons/run.png"`; 32×32 pixels looks
sharp). An icon that isn't a known name or points outside the folder is left
out and the log says why; the button then shows a puzzle piece.

```python
@ettin.command("Count Words", shortcut="Ctrl+Alt+W", icon="hash")
def count_words():
    ...
```

```javascript
ettin.command('Count Words', countWords, { shortcut: 'Ctrl+Alt+W', icon: 'hash' });
```

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
| `settings.changed` | The user changed the plugin's options. Called with the new settings. |

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
| `mark(id, ranges, style, color, message)` | `mark(id, ranges, {style, color, message})` | Marks ranges (see [Marks](#marks)). |
| `clear_marks(id)` | `clearMarks(id)` | Removes the marks made with that id. |

Each edit is one undo step, so **Undo** reverses it. Editing a read-only
document raises an error.

### Messages, input and more

| Python | JavaScript | What it does |
|--------|------------|--------------|
| `message(text, kind="info")` | `message(text, kind)` | Shows a short message. `kind` is `info`, `warning` or `error`. |
| `prompt(text, value="", title=None)` | `prompt(text, {value, title})` | Asks for a line of text. Returns `None`/`null` if cancelled. |
| `choose(items, title=None, placeholder=None)` | `choose(items, {title, placeholder})` | Asks the user to pick from a list (see [Picking from a list](#picking-from-a-list)). |
| `status(id, text, tooltip=None, command=None)` | `status(id, text, {tooltip, command})` | Shows text in the status bar (see [Status bar items](#status-bar-items)). |
| `settings` | `settings`, `setSettings(values)` | The plugin's settings (see [Settings and options](#settings-and-options)). |
| `data_dir()` | `dataDir()` | A folder for the plugin's own files. |
| `set_clipboard(text)` | `setClipboard(text)` | Copies text. |
| `run_action(action)` | `runAction(action)` | Runs one of Ettin Edit's own commands (see below). |
| `ui_language()` | `uiLanguage()` | The language Ettin Edit's menus are in, like `"en"` or `"de"` (see below). |
| `log(*values)` | `log(...values)` | Writes a line to the plugin log. |

`run_action` takes the name of an Ettin Edit command, such as `"Duplicate"`,
`"SelectAll"`, `"Save"`, `"FindNext"` or `"ToggleWordWrap"`. Commands that
need options take an object, such as
`{"SortLines": {"kind": "IgnoreCase", "desc": false}}` or
`{"ConvertCase": "Upper"}`. [Transform](transform.md) commands take their
settings the same way, such as
`{"Transform": {"Convert": {"from": "Json", "to": "Yaml"}}}`,
`{"Transform": {"Hash": "Sha256"}}` or
`{"Transform": {"FilterLines": {"pattern": "ERROR", "regex": false, "match_case": true, "keep": true}}}`.
[Recipes](recipes.md) run by name: `{"RunRecipe": "Tidy Whitespace"}` works on
the selections or the whole document, and `{"RunRecipeOnAllTabs": "Tidy Whitespace"}`
on every open tab. A recipe with shell commands still asks the user first.

To find a command's id, right-click it in the command palette (F1) or in
**Settings > Shortcut Mapper** and choose **Copy Command ID**. The ids of
the built-in commands are also [listed below](#command-ids).

### Translating your plugin

Ettin Edit doesn't translate plugins, but it tells them which language its
own menus are in, so a plugin can pick its own wording. `ui_language()` /
`uiLanguage()` returns a language code such as `"en"`, `"de"` or `"pt-BR"`.
Keep your strings in the plugin, with English as the fallback:

```python
from ettin import active, command, message, ui_language, run

TEXT = {
    "en": {"title": "Count Words", "result": "{} words"},
    "de": {"title": "Wörter zählen", "result": "{} Wörter"},
}
lang = ui_language()
strings = TEXT.get(lang) or TEXT.get(lang.split("-")[0]) or TEXT["en"]

@command(strings["title"])
def count():
    message(strings["result"].format(len(active().text.split())))

run()
```

Command titles are set when the plugin starts. If the user changes the
language in Preferences, the new titles appear after **Plugins > Reload
Plugins** or the next start; messages that call `ui_language()` when they
run follow the change straight away.

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
| `ui.markdown(text, id, base, doc, line)` | `{"type": "markdown", "id", "text", "base", "doc", "line"}` | Markdown, drawn like the user manual: headings, lists, tables, quotes, code blocks with a Copy button, links. Web links open in the browser and `#heading` links scroll to the heading; any other link sends `link` with the link as `value`. `id` is optional. `base` (1.9) is the folder images with relative paths are in; without it, only the descriptions of images are shown. `doc` (1.9, a document or its id) makes the panel scroll along with that tab, and `line` is the tab's line (from 0) the text starts on, if you left some out at the top. In JavaScript: `ui.markdown(text, { id, base, doc, line })`. Needs Ettin Edit 1.5. |

The handler gets: `item` (the item's id), `event` (`click`, `change`,
`submit`, `select`, `link` or `close`), `value`, and `values` (every input,
checkbox and select, by id). `close` means the user closed the panel.

A Markdown panel that follows the text, as the **Markdown Preview (plugin
example)** sample does it, needs only a few lines:

```python
@ettin.on("changed")
def changed(doc):
    folder = os.path.dirname(doc.path) if doc.path else None
    ettin.show_panel("preview", "Preview",
                     [ui.markdown(doc.get_text(), base=folder, doc=doc)])
```

Sending the same id again updates the panel and keeps its scroll position,
so the preview doesn't jump while you type. With `doc`, the panel follows
the tab's scrolling instead, and it only moves when the tab scrolls or the
text changes, so the reader can still scroll it by hand.

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

- `initialize` `{plugin, ettin, api, language, settings, dataDir}`. Always
  first. `api` is 2 since Ettin Edit 2.3 (1 before). `language` is the
  interface language, like `en` or `de`; `settings` the plugin's settings and
  `dataDir` its folder.
- `command` `{id}`.
- `event` `{event, doc}`. For `settings.changed`, `{event, settings}`.
- `panel` `{panel, item, event, value, values}`.
- `shutdown` `{}`. Finish within a second, or the plugin is stopped.

The plugin can call these methods. `doc` is a document id; it defaults to
the active tab.

| Method | Params | Result |
|--------|--------|--------|
| `commands.register` | `id, title, shortcut?, icon?` | `{shortcut, warning}` |
| `transforms.register` | `id, title, options?, languages?` | |
| `settings.get` | | the settings object |
| `settings.set` | `values` | the settings saved |
| `marks.set` | `doc?, id, ranges: [[start, end]], style?, color?, message?` | |
| `marks.clear` | `doc?, id` | |
| `status.set` | `id, text, tooltip?, command?` | |
| `ui.choose` | `items: [{label, detail?}], title?, placeholder?` | the index picked, or `null` |
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
| `app.language` | | the interface language, like `"de"` |
| `panel.show` | `id, title?, items, position?` | |
| `panel.close` | `id` | |

Notifications the plugin can send: `log` `{text, level?}`, `error`
`{message}`, which also shows a short message to the user, and `ready`
`{}`, which says the plugin has registered its commands and transforms (a
recipe run from the command line waits for it, up to 10 seconds).

Ettin Edit sends requests (with an `id`) for transforms, and the plugin
answers each with a result or an error:

- `transform.run` `{id, text, options}`: answer `{"text": "..."}`, or an
  error whose `data` may hold `line` and `column` (from 1) in the text.

A plugin should answer any other request it doesn't know with error
`-32601`.

## Command ids

The built-in commands and the ids `run_action` / `runAction` takes. Plugin
commands are `{"PluginCommand": {"plugin": "<plugin id>", "command": "<command
id>"}}` and plugin transforms `{"PluginTransform": {"plugin": "<plugin id>",
"transform": "<transform id>"}}`. Ettin Edit checks this list against its
commands when it is built.

<!-- command-ids:start -->
| Command | ID |
|---------|----|
| File › New | `NewTab` |
| File › Open… | `Open` |
| File › Save | `Save` |
| File › Save As… | `SaveAs` |
| File › Save All | `SaveAll` |
| File › Reload from Disk | `ReloadFromDisk` |
| File › Rename… | `RenameFile` |
| File › Move to Trash… | `MoveToTrash` |
| File › Open Containing Folder | `OpenContainingFolder` |
| File › Open Terminal Here | `OpenTerminalHere` |
| File › Print… | `Print` |
| File › Export as HTML… | `ExportHtml` |
| File › Open Folder as Workspace… | `OpenFolderAsWorkspace` |
| File › Close Workspace | `CloseWorkspace` |
| File › Close | `CloseTab` |
| File › Close All | `CloseAll` |
| File › Close Others | `CloseOthers` |
| File › Clear Recent Files | `ClearRecent` |
| File › Exit | `Quit` |
| Search › Find… | `Find` |
| Search › Replace… | `Replace` |
| Search › Find Next | `FindNext` |
| Search › Find Previous | `FindPrev` |
| Search › Find Selected Next | `FindSelectedNext` |
| Search › Find Selected Previous | `FindSelectedPrev` |
| Search › Find in Files… | `FindInFiles` |
| Search › Find All in Current Document | `FindAllInDoc` |
| Search › Find All in Open Tabs | `FindAllInTabs` |
| Search › Select All Matches | `SelectAllMatches` |
| Search › Mark All | `{"MarkAll":{"bookmark":false}}` |
| Search › Clear All Marks | `ClearMarks` |
| Search › Go to Line… | `GoToLine` |
| Search › Bookmark › Toggle Bookmark | `ToggleBookmark` |
| Search › Bookmark › Next Bookmark | `NextBookmark` |
| Search › Bookmark › Previous Bookmark | `PrevBookmark` |
| Search › Bookmark › Clear All Bookmarks | `ClearBookmarks` |
| Search › Bookmark › Cut Bookmarked Lines | `CutBookmarkedLines` |
| Search › Bookmark › Copy Bookmarked Lines | `CopyBookmarkedLines` |
| Search › Bookmark › Remove Bookmarked Lines | `RemoveBookmarkedLines` |
| Search › Bookmark › Remove Unbookmarked Lines | `RemoveUnbookmarkedLines` |
| Search › Bookmark › Inverse Bookmarks | `InverseBookmarks` |
| View › Word Wrap | `ToggleWordWrap` |
| View › Line Numbers | `ToggleLineNumbers` |
| View › Highlight Current Line | `ToggleCurrentLine` |
| View › Show Symbol › Show Whitespace | `ToggleWhitespace` |
| View › Show Symbol › Show End of Line | `ToggleEol` |
| View › Show Symbol › Show Invisible Characters | `ToggleInvisible` |
| View › Show Symbol › Show All Characters | `ToggleAllChars` |
| View › Show Symbol › Indent Guides | `ToggleIndentGuides` |
| View › Long-Line Marker | `ToggleEdge` |
| View › Function List | `ToggleFunctionList` |
| View › Document Map | `ToggleDocMap` |
| View › Character Panel | `ToggleCharPanel` |
| View › Hex View | `ToggleHexView` |
| View › Markdown Preview | `ToggleMarkdownPreview` |
| View › Folder as Workspace | `ToggleFolderPanel` |
| View › Distraction-Free Mode | `ToggleDistractionFree` |
| View › Tab List… | `TabList` |
| View › Move to Other View | `MoveActiveToOtherView` |
| View › Clone to Other View | `CloneActiveToOtherView` |
| View › Focus Other View | `FocusOtherView` |
| View › Next Tab | `NextTab` |
| View › Previous Tab | `PrevTab` |
| View › Folding › Fold All | `FoldAll` |
| View › Folding › Unfold All | `UnfoldAll` |
| View › Folding › Fold Current Level | `FoldCurrent` |
| View › Folding › Unfold Current Level | `UnfoldCurrent` |
| View › Zoom In | `ZoomIn` |
| View › Zoom Out | `ZoomOut` |
| View › Reset Zoom | `ZoomReset` |
| View › Summary… | `Summary` |
| View › Command Palette… | `CommandPalette` |
| View › Toolbar › Toolbar | `ToggleToolbar` |
| View › Toolbar › Simplified Toolbar | `ToggleSimplifiedToolbar` |
| View › Toolbar › Collapse Toolbar | `CollapseToolbar` |
| View › Menu Bar | `ToggleMenuBar` |
| View › Toolbar › Customise Toolbar… | `CustomiseToolbar` |
| View › Theme › Edit Theme Colours… | `StyleConfigurator` |
| View › Theme › Open Themes Folder | `OpenThemesFolder` |
| View › Theme › Reload Themes | `ReloadThemes` |
| View › Theme › Restore Built-in Themes | `RestoreThemes` |
| Language › Define Language… | `DefineLanguage` |
| Language › New Language… | `NewLanguage` |
| Language › Open Languages Folder | `OpenLanguagesFolder` |
| Language › Open Completion Lists Folder | `OpenApiFolder` |
| Language › Reload Languages | `ReloadLanguages` |
| Language › Language Guide | `LanguageGuide` |
| Settings › Preferences… | `Preferences` |
| Settings › Shortcut Mapper… | `ShortcutMapper` |
| Settings › Export Settings… | `ExportSettings` |
| Settings › Import Settings… | `ImportSettings` |
| Settings › Open Settings Folder | `OpenSettingsFolder` |
| Settings › Auto-Close Brackets and Quotes | `ToggleAutoClose` |
| Settings › Smart Indentation | `ToggleSmartIndent` |
| Settings › Indent With Spaces | `ToggleUseSpaces` |
| Settings › Word Completion | `ToggleCompletion` |
| Settings › Keyword and Function Completion | `ToggleLanguageCompletion` |
| Settings › Parameter Hints | `ToggleParameterHints` |
| Settings › Single Window | `ToggleSingleInstance` |
| Settings › Remember Unsaved Changes on Exit | `ToggleRememberUnsaved` |
| Settings › Reload Files Changed on Disk Automatically | `ToggleAutoReload` |
| Transform › Macros › Start or Stop Recording | `ToggleMacroRecording` |
| Plugins › Manage Plugins… | `ManagePlugins` |
| Plugins › New Plugin… | `NewPlugin` |
| Plugins › Plugin Log | `ShowPluginLog` |
| Plugins › Open Plugins Folder | `OpenPluginsFolder` |
| Plugins › Reload Plugins | `ReloadPlugins` |
| Plugins › Install Sample Plugins | `InstallSamplePlugins` |
| Plugins › Plugin Guide | `PluginGuide` |
| Help › Welcome | `Welcome` |
| Help › User Manual | `UserManual` |
| Help › User Manual on the Web | `WebManual` |
| Help › Check for Updates… | `CheckForUpdates` |
| Help › Check for Updates Automatically | `ToggleUpdateCheck` |
| Help › About Ettin Edit | `About` |
| Transform › Filter Through Command… | `{"TransformDialog":"Pipe"}` |
| Transform › Lines › Keep Matching Lines… | `{"TransformDialog":{"FilterLines":{"keep":true}}}` |
| Transform › Lines › Remove Matching Lines… | `{"TransformDialog":{"FilterLines":{"keep":false}}}` |
| Transform › Lines › Count Distinct Lines | `{"Transform":"CountLines"}` |
| Transform › Lines › Sort Lines › Sort Lexicographically Ascending | `{"SortLines":{"kind":"Lexical","desc":false}}` |
| Transform › Lines › Sort Lines › Sort Lexicographically Descending | `{"SortLines":{"kind":"Lexical","desc":true}}` |
| Transform › Lines › Sort Lines › Sort Ignoring Case Ascending | `{"SortLines":{"kind":"IgnoreCase","desc":false}}` |
| Transform › Lines › Sort Lines › Sort Ignoring Case Descending | `{"SortLines":{"kind":"IgnoreCase","desc":true}}` |
| Transform › Lines › Sort Lines › Sort As Integers Ascending | `{"SortLines":{"kind":"Integer","desc":false}}` |
| Transform › Lines › Sort Lines › Sort As Integers Descending | `{"SortLines":{"kind":"Integer","desc":true}}` |
| Transform › Lines › Sort Lines › Sort As Decimals Ascending | `{"SortLines":{"kind":"Decimal","desc":false}}` |
| Transform › Lines › Sort Lines › Sort As Decimals Descending | `{"SortLines":{"kind":"Decimal","desc":true}}` |
| Transform › Lines › Sort Lines › Sort By Length Ascending | `{"SortLines":{"kind":"Length","desc":false}}` |
| Transform › Lines › Sort Lines › Sort By Length Descending | `{"SortLines":{"kind":"Length","desc":true}}` |
| Transform › Lines › Sort Lines › Sort Naturally Ascending | `{"SortLines":{"kind":"Natural","desc":false}}` |
| Transform › Lines › Sort Lines › Sort Naturally Descending | `{"SortLines":{"kind":"Natural","desc":true}}` |
| Transform › Lines › Reverse Line Order | `ReverseLines` |
| Transform › Lines › Remove Duplicate Lines | `RemoveDuplicateLines` |
| Transform › Lines › Remove Consecutive Duplicate Lines | `RemoveConsecutiveDuplicates` |
| Transform › Lines › Remove Empty Lines | `{"RemoveEmptyLines":{"blank":false}}` |
| Transform › Lines › Remove Empty Lines (Including Blank Ones) | `{"RemoveEmptyLines":{"blank":true}}` |
| Transform › Lines › Trim › Trim Leading and Trailing Space | `{"Trim":"Both"}` |
| Transform › Lines › Trim › Trim Trailing Space | `{"Trim":"Trailing"}` |
| Transform › Lines › Trim › Trim Leading Space | `{"Trim":"Leading"}` |
| Transform › Lines › EOL to Space | `EolToSpace` |
| Transform › Lines › Add Prefix and Suffix… | `{"TransformDialog":"AffixLines"}` |
| Transform › Lines › Number Lines… | `{"TransformDialog":"NumberLines"}` |
| Transform › Lines › Reflow Paragraphs… | `{"TransformDialog":"Reflow"}` |
| Transform › Lines › Tabs to Spaces | `TabsToSpaces` |
| Transform › Lines › Leading Spaces to Tabs | `{"SpacesToTabs":{"all":false}}` |
| Transform › Lines › All Spaces to Tabs | `{"SpacesToTabs":{"all":true}}` |
| Transform › Columns and Fields › Extract Matches… | `{"TransformDialog":{"Field":"Extract"}}` |
| Transform › Columns and Fields › Align Columns… | `{"TransformDialog":{"Field":"Align"}}` |
| Transform › Columns and Fields › Select Fields… | `{"TransformDialog":{"Field":"Select"}}` |
| Transform › Columns and Fields › Sort Lines by Field… | `{"TransformDialog":{"Field":"SortBy"}}` |
| Transform › Columns and Fields › Split Lines At… | `{"TransformDialog":{"Field":"Split"}}` |
| Transform › Columns and Fields › Join Lines With… | `{"TransformDialog":{"Field":"Join"}}` |
| Transform › Columns and Fields › Escape String › JSON / JavaScript | `{"Transform":{"Field":{"op":"escape_string","language":"json"}}}` |
| Transform › Columns and Fields › Escape String › C / Java / C# | `{"Transform":{"Field":{"op":"escape_string","language":"c"}}}` |
| Transform › Columns and Fields › Escape String › SQL | `{"Transform":{"Field":{"op":"escape_string","language":"sql"}}}` |
| Transform › Columns and Fields › Escape String › Regex | `{"Transform":{"Field":{"op":"escape_string","language":"regex"}}}` |
| Transform › Columns and Fields › Escape String › Shell | `{"Transform":{"Field":{"op":"escape_string","language":"shell"}}}` |
| Transform › Columns and Fields › Escape String › CSV | `{"Transform":{"Field":{"op":"escape_string","language":"csv"}}}` |
| Transform › Columns and Fields › Unescape String › JSON / JavaScript | `{"Transform":{"Field":{"op":"unescape_string","language":"json"}}}` |
| Transform › Columns and Fields › Unescape String › C / Java / C# | `{"Transform":{"Field":{"op":"unescape_string","language":"c"}}}` |
| Transform › Columns and Fields › Unescape String › SQL | `{"Transform":{"Field":{"op":"unescape_string","language":"sql"}}}` |
| Transform › Columns and Fields › Unescape String › Regex | `{"Transform":{"Field":{"op":"unescape_string","language":"regex"}}}` |
| Transform › Columns and Fields › Unescape String › Shell | `{"Transform":{"Field":{"op":"unescape_string","language":"shell"}}}` |
| Transform › Columns and Fields › Unescape String › CSV | `{"Transform":{"Field":{"op":"unescape_string","language":"csv"}}}` |
| Transform › Columns and Fields › Unix Time › Unix Time to Date (UTC) | `{"Transform":{"Field":{"op":"unix_time_to_date","utc":true}}}` |
| Transform › Columns and Fields › Unix Time › Unix Time to Date (Local Time) | `{"Transform":{"Field":{"op":"unix_time_to_date","utc":false}}}` |
| Transform › Columns and Fields › Unix Time › Date to Unix Time (Seconds) | `{"Transform":{"Field":{"op":"date_to_unix_time","millis":false}}}` |
| Transform › Columns and Fields › Unix Time › Date to Unix Time (Milliseconds) | `{"Transform":{"Field":{"op":"date_to_unix_time","millis":true}}}` |
| Transform › Convert Case To › UPPERCASE | `{"ConvertCase":"Upper"}` |
| Transform › Convert Case To › lowercase | `{"ConvertCase":"Lower"}` |
| Transform › Convert Case To › Title Case | `{"ConvertCase":"Title"}` |
| Transform › Convert Case To › Sentence case | `{"ConvertCase":"Sentence"}` |
| Transform › Convert Case To › iNVERT cASE | `{"ConvertCase":"Invert"}` |
| Transform › Convert Case To › camelCase | `{"ConvertCase":"Camel"}` |
| Transform › Convert Case To › PascalCase | `{"ConvertCase":"Pascal"}` |
| Transform › Convert Case To › snake_case | `{"ConvertCase":"Snake"}` |
| Transform › Convert Case To › kebab-case | `{"ConvertCase":"Kebab"}` |
| Transform › Convert Case To › CONSTANT_CASE | `{"ConvertCase":"Constant"}` |
| Transform › Convert to › JSON | `{"Transform":{"ConvertTo":{"to":"Json","from":null}}}` |
| Transform › Convert to › YAML | `{"Transform":{"ConvertTo":{"to":"Yaml","from":null}}}` |
| Transform › Convert to › TOML | `{"Transform":{"ConvertTo":{"to":"Toml","from":null}}}` |
| Transform › Convert to › XML | `{"Transform":{"ConvertTo":{"to":"Xml","from":null}}}` |
| Transform › Convert to › CSV | `{"Transform":{"ConvertTo":{"to":"Csv","from":null}}}` |
| Transform › Convert to › Markdown table | `{"Transform":{"ConvertTo":{"to":"Markdown","from":null}}}` |
| Transform › Encode and Hash › Base64 Encode | `{"Mime":"Base64Encode"}` |
| Transform › Encode and Hash › Base64 Encode with Line Breaks | `{"Mime":"Base64EncodeWrapped"}` |
| Transform › Encode and Hash › Base64 Encode, URL-Safe | `{"Mime":"Base64UrlEncode"}` |
| Transform › Encode and Hash › Base64 Decode | `{"Mime":"Base64Decode"}` |
| Transform › Encode and Hash › URL Encode (Part of a URL) | `{"Mime":"UrlEncodeComponent"}` |
| Transform › Encode and Hash › URL Encode (Whole URL) | `{"Mime":"UrlEncode"}` |
| Transform › Encode and Hash › URL Decode | `{"Mime":"UrlDecode"}` |
| Transform › Encode and Hash › Quoted-Printable Encode | `{"Mime":"QpEncode"}` |
| Transform › Encode and Hash › Quoted-Printable Decode | `{"Mime":"QpDecode"}` |
| Transform › Encode and Hash › HTML Encode | `{"Mime":"HtmlEncode"}` |
| Transform › Encode and Hash › HTML Encode, Including Accents and Symbols | `{"Mime":"HtmlEncodeAll"}` |
| Transform › Encode and Hash › HTML Decode | `{"Mime":"HtmlDecode"}` |
| Transform › Encode and Hash › SAML Decode | `{"Mime":"SamlDecode"}` |
| Transform › Encode and Hash › Text to Hex | `{"Transform":"HexEncode"}` |
| Transform › Encode and Hash › Hex to Text | `{"Transform":"HexDecode"}` |
| Transform › Encode and Hash › Decode JWT | `{"Transform":"JwtDecode"}` |
| Transform › Encode and Hash › Hash › MD5 | `{"Transform":{"Hash":"Md5"}}` |
| Transform › Encode and Hash › Hash › SHA-1 | `{"Transform":{"Hash":"Sha1"}}` |
| Transform › Encode and Hash › Hash › SHA-256 | `{"Transform":{"Hash":"Sha256"}}` |
| Transform › Encode and Hash › Hash › SHA-512 | `{"Transform":{"Hash":"Sha512"}}` |
| Transform › Encode and Hash › Hash › CRC32 | `{"Transform":{"Hash":"Crc32"}}` |
| Transform › JSON and XML › Format JSON | `{"Json":"Format"}` |
| Transform › JSON and XML › Minify JSON | `{"Json":"Minify"}` |
| Transform › JSON and XML › Sort JSON Keys | `{"Json":"SortKeys"}` |
| Transform › JSON and XML › Format XML | `{"Xml":"PrettyPrint"}` |
| Transform › JSON and XML › Format XML, Attributes on New Lines | `{"Xml":"PrettyPrintAttributes"}` |
| Transform › JSON and XML › Linearize | `{"Xml":"Linearize"}` |
| Transform › JSON and XML › Escape Special Characters | `{"Xml":"Escape"}` |
| Transform › JSON and XML › Unescape Special Characters | `{"Xml":"Unescape"}` |
| Tools › CsvQuery › CsvQuery Panel | `ToggleCsvQuery` |
| Tools › CsvQuery › Run Query | `RunCsvQuery` |
| Tools › CsvQuery › Copy Results | `CopyCsvResults` |
| Tools › CsvQuery › Results to New Tab | `CsvResultsToNewTab` |
| Tools › JSON Tree | `ToggleJsonTree` |
| Tools › Validate JSON | `{"Json":"Validate"}` |
| Tools › XPath Panel | `ToggleXPathPanel` |
| Tools › Copy Current XPath | `CopyXPath` |
| Tools › Check XML Syntax | `{"Xml":"CheckSyntax"}` |
| Tools › Validate XML | `{"Xml":"ValidateSchema"}` |
| Tools › Validate XML Against… | `{"Xml":"ChooseSchema"}` |
| Tools › XSL Transform… | `{"Xml":"XslTransform"}` |
| Tools › Check Syntax When Saving | `ToggleXmlCheckOnSave` |
| Tools › Close Tags Automatically | `ToggleCloseTags` |
<!-- command-ids:end -->
