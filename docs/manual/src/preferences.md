# Preferences and settings

**Settings > Preferences…** (<kbd>Ctrl+,</kbd>) holds every setting in one
window. Changes take effect as you make them; there's no OK button to press.
The sections are on the left, with buttons for the
[Shortcut Mapper](shortcuts.md) and [Theme Colours](themes.md#edit-theme-colours)
below them.

The most-used switches are also in the **Settings** menu, so you can flip them
without opening Preferences.

## Editing

**Indentation**

- **Tab width**: how many columns a tab takes up, and how many spaces an
  indent is when indenting with spaces.
- **Indent with tabs** or **Indent with spaces**: what <kbd>Tab</kbd> and
  automatic indentation insert. Also **Settings > Indent With Spaces**.
- **Smart indentation**: keep the indent on <kbd>Enter</kbd>, and indent after
  an opening bracket. Also **Settings > Smart Indentation**.

**Typing**

- **Close brackets and quotes automatically** (**Settings > Auto-Close
  Brackets and Quotes**)
- **Close HTML and XML tags automatically** (**Tools > XML Tools > Close Tags
  Automatically**)
- **Suggest words from the document while typing** (**Settings > Word
  Completion**)
- **Suggest the language's keywords and functions** (**Settings > Keyword and
  Function Completion**)
- **Show a function's parameters while typing a call** (**Settings >
  Parameter Hints**)

See [Typing helpers](editing.md#typing-helpers) for what each one does.

## Display

**Font**

- **Editor font**: the font for the text. The list shows the fonts installed
  on your computer, monospaced ones first; tick **Show proportional fonts** to
  see the rest, and type in **Find** to look for one. **Use Built-in** goes
  back to Hack, the font Ettin Edit comes with.
- **Size**: the text size. <kbd>Ctrl+0</kbd> returns to this size after
  zooming.

Whatever font you choose, Chinese, Japanese and Korean characters are drawn
with a CJK font from your system when one is installed.

**Show**: word wrap, line numbers, the current line highlight, whitespace and
end-of-line symbols, indent guides, and the **Long-line marker at column**,
with its column. These are the same switches as in the
[View menu](view.md#what-the-editor-shows).

## New Documents

What a new tab (<kbd>Ctrl+N</kbd>) starts with:

- **Line ending**: your system's usual one (LF on Linux, CR LF on Windows),
  or one you choose.
- **Encoding**: UTF-8 unless you choose another.
- **Language**: Plain Text, or the language you write most, so new tabs are
  coloured and complete as that language straight away.

Opened files always keep their own line ending and encoding.

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
- **Check that XML is well-formed each time it's saved**. Also **Tools > XML
  Tools > Check Syntax When Saving**.

**Printing**

- **Print line numbers**
- **Print syntax colours (darkened to read well on paper)**

**Updates**

- **Check for a new version of Ettin Edit once a week**. Also **Help > Check
  for Updates Automatically**.

## Plugins

**Programs that run plugins**: the Python 3 and Node.js programs to run
plugins with. Leave them empty and Ettin Edit finds them on the `PATH`; fill
them in if you want a particular Python (a virtual environment, say) or a
Node.js that isn't on the `PATH`. Changes apply when plugins are reloaded
(**Plugins > Reload Plugins**).

## Where settings are kept

Settings are saved in `settings.json` in the
[settings folder](folders.md#the-settings-folder). You can copy that file to
another computer to take your settings with you. Edit it by hand only while
Ettin Edit is closed, because Ettin Edit writes it when settings change.
