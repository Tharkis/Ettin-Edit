# Preferences and settings

**Settings > Preferences…** (<kbd>Ctrl+,</kbd>) holds every setting in one
window. The sections are on the left: **Appearance**, **Editing**,
**Languages**, **Transform**, **Files and Startup** and **Plugins**. Below
them are buttons for the [Shortcut Mapper](shortcuts.md),
[Theme Colours](themes.md#edit-theme-colours) and
[Customise Toolbar](toolbar.md#customising). Those open in front of
Preferences.

## Changes apply as you make them

There's no OK or Apply button. Each change takes effect and is saved straight
away, and closing the window keeps it. The buttons at the bottom are for
going back:

- **Undo Changes** puts every setting back as it was when you opened the
  window.
- **Reset Section** puts the settings of the section you're looking at back
  to Ettin Edit's defaults.

Theme Colours, Customise Toolbar, the Shortcut Mapper and the Recipes window
work the same way: changes are saved as you make them, and **Undo Changes**
puts things back as they were when the window opened.

## Search

Type in **Search settings** at the top to see only the settings whose name
(or a word related to it) matches, from every section at once. Searching
`tab` shows the tab width, what the Tab key types and the per-language
settings; `wrap` finds **Word wrap**. Settings that also live in a menu, like
word wrap, whitespace symbols, the menu bar and the toolbar, are all here so
search can find them. Clear the box to go back to the sections.

## Appearance

**Theme**

- **Theme**: the colours of the editor and the window. See [Themes](themes.md).
- **Follow the system's light/dark setting**: when on, Ettin Edit uses the
  **light** theme of the **Light / dark pair** while your desktop is in light
  mode and the **dark** one while it's in dark mode, and switches when the
  desktop does. Picking a theme from the **Theme** list changes the half of
  the pair that's showing. On Linux the desktop's choice is read from the
  desktop portal (GNOME, KDE and most others) or from GNOME's settings.

**Interface**

- **Interface size**: makes menus, dialogs, the toolbar and the editor bigger
  or smaller together, from 80 % to 200 %. The editor's own text size is under
  **Font**.
- **Interface language**: the language of menus, dialogs and messages: the
  same as your system's, or one you choose. **Open Translations Folder** and
  **Reload** are for adding your own translations. See
  [Translations](translations.md).
- **Toolbar**: the full ribbon, the simplified one, or hidden. See
  [The toolbar](toolbar.md).
- **Menu bar**: show or hide the menu bar (<kbd>Ctrl+M</kbd>).

**Font**

- **Editor font**: the font for the text. The list shows the fonts installed
  on your computer, monospaced ones first; tick **Show proportional fonts** to
  see the rest, and type in **Find** to look for one. **Use Built-in** goes
  back to Hack, the font Ettin Edit comes with.
- **Size**: the text size. <kbd>Ctrl+0</kbd> returns to this size after
  zooming.

Whatever font you choose, Chinese, Japanese and Korean characters are drawn
with a CJK font from your system when one is installed.

## Editing

**Show**: word wrap, line numbers, the current line highlight, whitespace,
end-of-line symbols and invisible characters, indent guides, and the
**Long-line marker at column**, with its column. These are the same switches
as in the [View menu](view.md#what-the-editor-shows).

**Typing**

- **Close brackets and quotes automatically** (**Settings > Auto-Close
  Brackets and Quotes**)
- **Close HTML and XML tags automatically** (**Tools > Close Tags
  Automatically**)
- **Smart indentation**: keep the indent on <kbd>Enter</kbd>, and indent after
  an opening bracket. Also **Settings > Smart Indentation**.
- **Suggest words from the document while typing** (**Settings > Word
  Completion**)
- **Suggest the language's keywords and functions** (**Settings > Keyword and
  Function Completion**)
- **Show a function's parameters while typing a call** (**Settings >
  Parameter Hints**)

See [Typing helpers](editing.md#typing-helpers) for what each one does.

## Languages

**Indentation**

- **Tab width**: how many columns a tab takes up, and how many spaces an
  indent is when indenting with spaces.
- **Tab key types** tabs or spaces: what <kbd>Tab</kbd> and automatic
  indentation insert. **View > Indent With Spaces** changes it for the current
  tab only.
- **Detect indentation when a file opens**: on by default. Ettin Edit looks at
  the first 1000 indented lines of a file and, when they're clearly indented
  with tabs or with a number of spaces, uses the same for that file. A Python
  file indented with 4 spaces gets 4 spaces even if your setting says tabs.

**Per-language settings**

A tab width and tabs-or-spaces for one language, which win over the settings
above. Out of the box JSON and YAML use 2 spaces, and Makefiles and Go use
tabs. **Pick a language…** and **Add** adds a row; **Default** in a row means
"use the general setting"; **×** removes the row.

The order, from weakest to strongest: the general setting, the language's
setting, what the file itself uses (when detection is on), and **View >
Indent With Spaces** for that tab.

In `settings.json` these are kept as `language_overrides`, for example
`{"json": {"tab_width": 2, "use_tabs": false}, "makefile": {"use_tabs": true}}`.

## Transform

- **Hash with no selection**: what a [hash](transform.md) does when nothing is
  selected and it hashes the whole document: **Copy the hash to the
  clipboard** (the default) or **Open the hash in a new tab**.
- **Recipe command steps**: **Ask before running** (the default) or **Run
  without asking**. See [Recipes](recipes.md#recipes-that-run-commands).
- **Shell for commands**: the program, with its options, that runs
  **Filter Through Command** and recipe command steps, like `bash -c`,
  `zsh -c` or `pwsh -Command`. Put a path with spaces in quotes. Leave it
  empty to use the system's shell (`sh -c` on Linux, `cmd /C` on Windows).
- **JSON / XML indent**: the indent **Format JSON** and **Format XML** use:
  the same as the document, 2 spaces, 4 spaces or a tab.

## Files and Startup

**Startup and exit**

- **Reopen the files that were open last time**: start where you left off.
- **Quit without asking about unsaved changes, and bring them back next
  time**: see [You won't lose your work](files.md#you-wont-lose-your-work).
  Also **Settings > Remember Unsaved Changes on Exit**.
- **Open files in the running Ettin Edit window**: one window for everything.
  Also **Settings > Single Window**.

**Files**

- **Reload files changed on disk automatically when they have no unsaved
  changes**. Also **Settings > Reload Files Changed on Disk Automatically**.
- **Check that XML is well-formed each time it's saved**. Also **Tools > Check
  Syntax When Saving**.

**New Documents**: what a new tab (<kbd>Ctrl+N</kbd>) starts with:

- **Line ending**: your system's usual one (LF on Linux, CR LF on Windows),
  or one you choose.
- **Encoding**: UTF-8 unless you choose another.
- **Language**: Plain Text, or the language you write most, so new tabs are
  coloured and complete as that language straight away.

Opened files always keep their own line ending and encoding.

**Suggestions**

- **Suggest transforms for files I open or paste**: the bar that offers
  Format for one-line JSON, Decode for a token and so on. On by default. See
  [Suggestions](suggestions.md).
- **Show the dismissed suggestions again**: shown when you have turned some
  kinds off with **Don't suggest this again**.

**Printing**

- **Print line numbers**
- **Print syntax colours (darkened to read well on paper)**

**Updates**

- **Check for a new version of Ettin Edit once a week**. Also **Help > Check
  for Updates Automatically**.

**Settings files**: **Export Settings…**, **Import Settings…** and **Open
Settings Folder**. See [Moving your settings to another computer](#moving-your-settings-to-another-computer).

## Plugins

**Programs that run plugins**: the Python 3 and Node.js programs to run
plugins with. Leave them empty and Ettin Edit finds them on the `PATH`; fill
them in if you want a particular Python (a virtual environment, say) or a
Node.js that isn't on the `PATH`. Changes apply when plugins are reloaded
(**Plugins > Reload Plugins**).

## Where settings are kept

Settings are saved in `settings.json` in the
[settings folder](folders.md#the-settings-folder) (**Settings > Open Settings
Folder**). Your keyboard shortcuts are in it too; the toolbar is in
`toolbar.json`, and themes, recipes, languages and translations each have
their own folder.

You can edit any of these files while Ettin Edit is running. When you save
one, Ettin Edit picks up the change within a second: a theme recolours the
editor, a recipe appears in the Recipes window, a language or translation is
reloaded, and changes to `settings.json` take effect. If the file has a
mistake, Ettin Edit keeps using the last version that worked and shows a
message with the file, the line and column, and an **Open** button that
opens the file at that line. (Plugins aren't watched this way; turn on
**Reload when its files change** for a plugin you're writing. See
[Plugins](plugins.md).)

## Moving your settings to another computer

**Settings > Export Settings…** saves one `.ettin-settings.zip` file with
your settings (shortcuts included), your toolbar, and your themes, recipes
and languages. Plugins aren't copied; the file lists which ones you had and
their versions.

**Settings > Import Settings…** on the other computer opens such a file and
shows what's in it before anything changes: how many settings files, themes,
recipes and languages, which files you have now that it would replace, and
any plugins it was used with that aren't installed. Choose **Import** to go
ahead. Each file it replaces is first copied to a new folder under `backups`
in the settings folder, named after the date and time, so you can get it
back.
