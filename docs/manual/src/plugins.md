# Using plugins

Plugins add commands, shortcuts and panels to Ettin Edit. They're written in
Python or JavaScript, so anyone who knows either can make one; see
[Writing plugins](plugin-guide.md).

Many things that need a plugin in other editors are built into Ettin Edit
already: the [JSON](json.md), [XML](xml.md) and [MIME](mime.md) tools,
[Compare](compare.md) and [CsvQuery](csvquery.md).

## What you need

- **Python plugins** need Python 3 installed. Most Linux systems have it; on
  Windows, install it from [python.org](https://www.python.org/downloads/) or
  the Microsoft Store.
- **JavaScript plugins** need [Node.js](https://nodejs.org) 18 or newer.

Ettin Edit finds them on the `PATH`. If it can't, or you want a different
one, set the program in **Plugins > Manage Plugins** or
[Preferences > Plugins](preferences.md#plugins).

## Trying the samples

**Plugins > Install Sample Plugins** installs and starts four plugins:

| Plugin | Language | What it does |
| --- | --- | --- |
| **Text Statistics** | Python | Live counts for the current tab in a side panel (<kbd>Ctrl+Alt+T</kbd>), word frequencies as CSV in a new tab, and a word count of the selection |
| **Markdown Outline** | JavaScript | A clickable outline of a Markdown file's headings, **Insert UUID**, and **Wrap Lines…** to a width you choose |
| **Markdown Preview** | Python | A live preview of the Markdown tab you're editing (see below) |
| **HL7 Viewer** | Python | Browse and edit HL7 v2 messages (see below) |

Their commands appear in the **Plugins** menu, under each plugin's name, and
in the editor's right-click menu.

### Markdown Preview

Open a Markdown file and choose **Plugins > Markdown Preview > Show or Hide
Markdown Preview** (<kbd>Ctrl+Shift+M</kbd>). A panel on the right shows the
file rendered: headings, lists, tables, quotes, code blocks, links,
**bold**, *italic* and ~~strikethrough~~. It updates a moment after you stop
typing, keeps its place as you edit, and follows you when you switch tabs.

- Click a web link to open it in your browser, a link to a heading
  (`#install`) to scroll there, or a link to another file (`notes.md`) to
  open it in a tab.
- Images show: PNG, JPEG, GIF, BMP, WebP and SVG, from files next to the
  Markdown file (`![Logo](img/logo.png)`) or from the web. An image that
  can't be found or read shows its description instead. Images in an
  unsaved tab only load from the web, as there's no folder to look in yet.
- The preview scrolls along with the text: scroll the editor and the preview
  shows the same place. Untick **Scroll with the text** at the top of the
  preview to scroll it on its own.
- YAML front matter (the `---` block some sites put at the top) isn't shown.
- Raw HTML is left out.
- Tabs that aren't Markdown (by language, or a `.md` / `.markdown` name) show
  a note instead.

### HL7 Viewer

HL7 v2 is the message format hospitals use to pass patient, order and result
data between systems. Open a message (or choose **Plugins > HL7 Viewer > Open
Sample HL7 Message**), then **Show or Hide HL7 Viewer**
(<kbd>Ctrl+Alt+H</kbd>):

- A tree on the right shows the message's segments, fields, repetitions,
  components and subcomponents. It follows the caret, and clicking a node
  selects that part of the text.
- An inspector shows the selected piece's designation (such as `PID-5.1`),
  its name, data type, whether it's required or repeatable, its length and
  its table, from the HL7 v2.5.1 definitions. Change the value there to edit
  the message.
- **Copy HL7 Field Designation** copies the designation of the piece at the
  caret.

The separators are read from each `MSH` segment, so messages with unusual
delimiters work too. HL7 files are coloured by the built-in HL7 language.

## Installing plugins

**Plugins > Manage Plugins…** has two tabs:

- **Available** lists the plugins in the Ettin Edit plugin index. Type in the
  filter box to search by name, author or description, and click **Install**.
- **Installed** lists your plugins, and offers **Update to x.y** when the
  index has a newer version.

You can also install a plugin someone sent you: **Install from File…** takes
a `.zip`, and **Install from Folder…** copies a folder that has a
`plugin.json` in it. Or copy the folder into the plugins folder yourself
(**Plugins > Open Plugins Folder**) and choose **Plugins > Reload Plugins**.

## Permission to run

A plugin is a program that runs with your user account: it can read and
change your documents and files and use the network, like any program you
install. So the first time Ettin Edit finds a plugin, and again whenever its
files change, it asks whether to let it run. Choose **Allow** only for
plugins you trust. **Don't Allow** turns the plugin off; you can turn it on
later in Manage Plugins.

The samples you install from the Plugins menu don't ask.

## Managing plugins

In the **Installed** tab of **Plugins > Manage Plugins…**, each plugin shows
whether it's running, stopped or waiting for permission, and has:

- a switch to turn it on or off,
- **Start**, **Stop** and **Restart**,
- **Open Folder**, to see or edit its files,
- **Remove…**, which moves its folder to the trash (so you can get it back),
- **Don't ask when it changes**, for a plugin you're writing yourself.

If a plugin fails to start, the reason is shown under its name.
**Plugins > Reload Plugins** stops every plugin, looks for plugins again, and
starts them.

## The plugin log

**Plugins > Plugin Log** shows what plugins print, their errors, and anything
Ettin Edit refused, such as a shortcut that clashes with one that's already
taken. Look here first when a plugin doesn't behave. **Copy All** copies the
log, for a bug report.

A plugin that crashes shows a notice and stops; it can't take Ettin Edit or
your documents down with it. Edits a plugin makes are undone in one step,
like any other edit.

## Plugin shortcuts

Plugins can give their commands shortcuts. A shortcut that's already used by
Ettin Edit or another plugin is refused, with a line in the log. You can
change or add plugin shortcuts in the [Shortcut Mapper](shortcuts.md), like
any other command's.
