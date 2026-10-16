# Files and tabs

## Opening files

- **File > Open…** (<kbd>Ctrl+O</kbd>) opens one or more files; select
  several in the dialog to open them all.
- **File > Open Recent** lists the last 12 files you had open.
  **Clear Recent Files** at the bottom empties the list.
- Drag files from your file manager onto the window.
- Pass them on the command line: `ettin-edit notes.txt todo.md`
  (see [the command line](folders.md#the-command-line)).
- On Windows, right-click a file and choose **Edit with Ettin Edit**, if you
  turned that on when installing.
- In the [folder workspace](view.md#folder-as-workspace), click a file.

A file opened while the only tab is an untouched new one replaces that tab,
so you don't collect empty tabs.

Ettin Edit works out each file's [encoding and line endings](encodings.md)
and its [language](languages.md) as it opens it, and keeps them when you
save. Very large files are fine: a 50 MB log opens in about a second, and word wrap
keeps up when you resize the window.

### One window or several

By default there is one Ettin Edit window. Opening a file from your file
manager or a terminal while Ettin Edit is running sends it to the open window
instead of starting a second copy. To get separate windows, turn off
**Settings > Single Window**, or start Ettin Edit with `--multi-instance`.

## New files

**File > New** (<kbd>Ctrl+N</kbd>) or the **+** at the end of the tabs opens
an empty tab named *new 1*, *new 2* and so on. Its line ending, encoding and
language come from **Preferences > New Documents**.

## Saving

| Command | Shortcut | What it does |
| --- | --- | --- |
| **File > Save** | <kbd>Ctrl+S</kbd> | Saves the current tab. A new tab asks for a name. |
| **File > Save As…** | <kbd>Ctrl+Alt+S</kbd> | Saves under a new name; the tab then shows that file. |
| **File > Save All** | <kbd>Ctrl+Shift+S</kbd> | Saves every tab with changes. |

The file keeps its encoding, line endings and byte-order mark. If the text
holds a character the file's encoding can't store, Ettin Edit refuses to
save and tells you which character it is, rather than quietly losing it.
Pick another encoding from the [Encoding menu](encodings.md) and save again.

XML files can be checked for mistakes every time they're saved; see
[XML Tools](xml.md#checking-xml).

## Tabs

- Click a tab to switch to it, or use <kbd>Ctrl+Tab</kbd> /
  <kbd>Ctrl+Shift+Tab</kbd> (also <kbd>Ctrl+PageDown</kbd> /
  <kbd>Ctrl+PageUp</kbd>) for the next and previous tab.
- Drag a tab to move it along the tab bar.
- Close a tab with its **×**, a middle-click, or <kbd>Ctrl+W</kbd>.
  **File > Close All** (<kbd>Ctrl+Shift+W</kbd>) and **File > Close Others**
  close the rest.
- **Pin Tab** (right-click a tab) keeps a tab at the front with a pin on it.
  Close All and Close Others leave pinned tabs open, and the **×** on a
  pinned tab unpins it instead of closing it.
- Closing a tab with unsaved changes asks whether to save them.

### The tab's right-click menu

| Command | What it does |
| --- | --- |
| **Close**, **Close Others** | Close this tab, or every other one |
| **Pin Tab** / **Unpin Tab** | See above |
| **Move to Other View**, **Clone to Other View** | See [split view](view.md#split-view) |
| **Reload from Disk** | Read the file again, replacing what's in the tab (Undo brings your version back) |
| **Rename…** | Rename the file on disk; the tab follows it |
| **Move to Trash…** | Move the file to the Recycle Bin or trash and close the tab |
| **Open Containing Folder** | Show the file in your file manager, selected where possible |
| **Open Terminal Here** | Open a terminal in the file's folder |
| **Copy Full Path**, **Copy File Name** | Copy them to the clipboard |

The file commands are also in the **File** menu, for the current tab. **Open
Terminal Here** uses Windows Terminal or `cmd` on Windows; on Linux it tries
`$TERMINAL`, then common terminals such as Konsole and GNOME Terminal.

## You won't lose your work

Ettin Edit backs up unsaved changes every few seconds, including tabs that
were never saved. If Ettin Edit, your computer or the power goes away, your
tabs and their changes are back next time you start it.

When you quit, Ettin Edit remembers the open tabs, unsaved changes included,
without asking, and reopens everything next time: each tab's caret, encoding,
line ending, language, pin, read-only state and the split view. If you'd
rather be asked to save each file when quitting, turn off **Settings >
Remember Unsaved Changes on Exit**. To start with no tabs from last time, turn
off **Reopen the files that were open last time** in
[Preferences](preferences.md).

## When another program changes the file

If a file you have open is changed by another program, a bar appears above
the text:

- **Reload** reads the new version. If you had unsaved changes, Undo brings
  them back.
- **Keep My Version** leaves your text alone; saving will overwrite the other
  program's changes.

If the file is deleted or moved, the bar offers **Keep Open**, which keeps
the text in the tab so you can save it again, or **Close**.

To reload changed files without being asked, turn on **Settings > Reload
Files Changed on Disk Automatically**. It only reloads files that have no
unsaved changes in Ettin Edit.

## Read-only tabs

**Edit > Read-Only** stops a tab from being changed by accident. The status
bar shows **Read-only**; click it, or choose the command again, to allow
editing.

## Summary

**View > Summary…** shows the file's full path, when it was created and
changed, its encoding, and how many characters, words and lines it has, plus
its size on disk and the same counts for the selection.
