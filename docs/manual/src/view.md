# Views, panels and navigation

Everything in this chapter is in the **View** menu unless it says otherwise.

## What the editor shows

| Command | Shortcut | What it does |
| --- | --- | --- |
| **Word Wrap** | <kbd>Alt+Z</kbd> | Wrap long lines at the window's edge instead of scrolling sideways. Line numbers stay with their lines. |
| **Line Numbers** | | The numbers in the gutter |
| **Highlight Current Line** | | A band behind the caret's line |
| **Show Symbol > Show Whitespace** | | A dot for each space and an arrow across each tab |
| **Show Symbol > Show End of Line** | | `LF`, `CRLF` or `CR` at each line end |
| **Show Symbol > Indent Guides** | | Faint lines at each indentation level |
| **Long-Line Marker** | | A vertical line at a column, 80 unless you change it |

Change the long-line column at the bottom of the **Settings** menu or in
[Preferences](preferences.md). **Edit > Line Operations > Split Lines at
Long-Line Column** (<kbd>Ctrl+I</kbd>) rewraps text to that width.

## Zoom

**Zoom In** (<kbd>Ctrl+=</kbd> or <kbd>Ctrl++</kbd>), **Zoom Out**
(<kbd>Ctrl+-</kbd>) and **Reset Zoom** (<kbd>Ctrl+0</kbd>) change the size of
the text. The status bar shows the zoom level. To change the size Reset Zoom
returns to, set the font size in [Preferences](preferences.md).

## Code folding

Folding hides a block of code, such as a function or an `if`, behind its first
line. Blocks follow the language's grammar: brackets, HTML and XML tags,
keyword blocks such as `if` … `end`, Markdown headings, comment blocks and
`#region` markers. Python, YAML and plain text fold by indentation. The full
list is in [Languages](languages.md#code-folding).

- Click **−** in the gutter to fold a block and **+** to unfold it. A line in
  the gutter shows where each block starts and ends.
- The block's closing line stays visible, so `} else {` folds each half.

| Command (View > Folding) | Shortcut |
| --- | --- |
| Fold All | <kbd>Alt+0</kbd> |
| Unfold All | <kbd>Alt+Shift+0</kbd> |
| Fold Current Level (the block the caret is in) | <kbd>Ctrl+Alt+F</kbd> |
| Unfold Current Level | <kbd>Ctrl+Alt+Shift+F</kbd> |
| Fold Level 1 to 5 (every block at that depth) | <kbd>Alt+1</kbd> … <kbd>Alt+5</kbd> |

## Split view

Split view shows two tabs side by side, or one tab twice.

- **Move to Other View** moves the current tab to the other side, splitting
  the window if it isn't split yet.
- **Clone to Other View** shows the same document on both sides. Each side
  has its own caret and scroll position, and an edit on one side appears on
  the other at once. Closing one copy leaves the other open.
- Each side has its own tab bar and current tab. Click a side, or press
  <kbd>F8</kbd> (**Focus Other View**), to work in it.
- Drag the divider to change the sizes.
- When one side has no tabs left, the split ends.

Both commands are also on the tab's right-click menu. The split is saved with
the session. [Compare](compare.md) uses the two views too: with the window
split, <kbd>Ctrl+Alt+D</kbd> compares the two tabs you see.

## Function List

**View > Function List** opens a panel on the right listing the functions,
classes, methods and other parts of the document, nested the way they are in
the code. It understands about 40 languages, including Markdown headings,
INI sections and the keys of JSON and YAML files.

- Click an entry to jump to it.
- Type in **Filter** to find one by name.
- **Show where the caret is** keeps the entry you're in selected as you move
  around.

## Document Map

**View > Document Map** shows the whole document in miniature on the right,
with the part on screen marked. Click or drag in it to scroll. It's a quick
way to find your place in a long file.

## Folder as Workspace

**File > Open Folder as Workspace…** opens a folder as a tree on the left,
for working on a project. **View > Folder as Workspace** shows or hides the
panel, and **File > Close Workspace** closes the folder.

- Click a file to open it. The tree follows the current tab, and notices
  files being added or removed by itself.
- The panel's buttons collapse every folder, read the folder again, and
  **Show hidden files**.
- Right-click an entry for **Open**, **Copy Path**, **Find in Files Here…**
  and **Open Terminal Here**.

The workspace is remembered next time you start Ettin Edit.

## Distraction-free mode

**View > Distraction-Free Mode** (<kbd>F11</kbd>) goes full screen and hides
the menus, tabs, panels and status bar, leaving the text in a centred column.
Press <kbd>F11</kbd> again to come back. Shortcuts and the command palette
(<kbd>F1</kbd>) still work.

## Tab list

**View > Tab List…**, or the **⏷** button at the end of the tab bar, lists
every open tab. Type part of a file or folder name to narrow the list, then
press <kbd>Enter</kbd> or click a tab to switch to it.

## Command palette

**View > Command Palette…** (<kbd>F1</kbd>) lists every command in every menu,
your saved macros and your plugins' commands, each with its shortcut. Type a
few letters of any words in its name, in any order, then press
<kbd>Enter</kbd>.
