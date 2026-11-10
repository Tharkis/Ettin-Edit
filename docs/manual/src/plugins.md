# Using plugins

Plugins add transforms, commands, shortcuts and panels to Ettin Edit.
They're written in Python or JavaScript, so anyone who knows either can make
one: **Plugins > New Plugin…** writes a working one to start from. See
[Writing plugins](plugin-guide.md).

A plugin's **transforms** work like the built-in ones: they're in
**Transform > Plugin Transforms** (under the plugin's name), the command
palette, the recipe window's **Add Step** list, and the right-click menu of
the languages they're for. They run on each selection or the whole tab as
one undo step, show a preview and their options before they run, and can
send the result to a new tab or the clipboard. See
[Transforming text](transform.md#plugin-transforms).

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
| **Markdown Preview (plugin example)** | Python | How the built-in [Markdown Preview](view.md#markdown-preview) could be written as a plugin (see below) |
| **HL7 Viewer** | Python | Browse and edit HL7 v2 messages, and the **Explode HL7 Segments** transform (see below) |

A notice says how many were installed. Their commands appear in the
**Plugins** menu, under each plugin's name, and in the editor's right-click
menu.

### Markdown Preview (plugin example)

Since Ettin Edit 2.3 the Markdown preview is built in: **View > Markdown
Preview** (<kbd>Ctrl+Shift+M</kbd>, see [Views](view.md#markdown-preview)).
This sample does the same thing as a plugin, so you can read how a
`markdown` panel that follows the text is written. Run it from **Plugins >
Markdown Preview (plugin example) > Show or Hide Markdown Preview (plugin
example)**; it has no shortcut.

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
- **Explode HL7 Segments** (Transform > Plugin Transforms, or right-click in
  an HL7 tab) puts each field on its own line with its designation and name,
  such as `PID-5 Patient Name: Quillfeather^Maribel^June`, which is easier to
  read and to [compare](compare.md). Its options leave out the names or show
  empty fields too.

The separators are read from each `MSH` segment, so messages with unusual
delimiters work too. HL7 files are coloured by the built-in HL7 language.

## Installing plugins

**Plugins > Manage Plugins…** has two tabs:

- **Available** lists the plugins in the Ettin Edit plugin index. Type in the
  filter box to search by name, author or description, and click **Install**.
  So far the index holds only the sample plugins; while that's so, the tab
  says so and links to
  [how to submit yours](plugin-guide.md#submitting-a-plugin-to-the-index).
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

The samples you install from the Plugins menu, and plugins you make with
**New Plugin…**, don't ask.

## Managing plugins

In the **Installed** tab of **Plugins > Manage Plugins…**, each plugin shows
whether it's running, stopped or waiting for permission. Under its
description, **Adds:** lists the commands it adds and the shortcut each one
runs with now (after any change you made in the Shortcut Mapper). Type in
the **Filter** box at the top to find a plugin by its name, description or
one of its commands. Each plugin has:

- an **Enabled** tick box to turn it on or off,
- **Start**, **Stop** and **Restart**,
- **Open Folder**, to see or edit its files,
- **Options…**, when the plugin has settings you can change: a form with a
  field for each, saved as you change them,
- **Remove…**, which moves its folder (and its saved settings) to the trash,
  so you can get it back,
- **Don't ask when it changes** and **Reload when its files change**, for a
  plugin you're writing yourself.

Under the commands, **Transforms:** lists the transforms it adds. The
**New Plugin…** button at the bottom does the same as **Plugins > New
Plugin…**.

If a plugin fails to start, the reason is shown under its name.
**Plugins > Reload Plugins** stops every plugin, looks for plugins again, and
starts them.

## The plugin log

**Plugins > Plugin Log** shows what plugins print, their errors, and anything
Ettin Edit refused, such as a shortcut that clashes with one that's already
taken. Look here first when a plugin doesn't behave. **Copy All** copies the
log, for a bug report.

A plugin that crashes shows a notice and stops; it can't take Ettin Edit or
your documents down with it. When the error happened in one of the plugin's
own files, the notice says which file and line, and **Open** opens it there. Edits a plugin makes are undone in one step,
like any other edit.

## Plugin shortcuts

Plugins can give their commands shortcuts. A shortcut that's already used by
Ettin Edit or another plugin is refused, with a line in the log. You can
change or add plugin shortcuts in the [Shortcut Mapper](shortcuts.md), like
any other command's.
