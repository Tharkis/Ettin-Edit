# Views, panels and navigation

Everything in this chapter is in the **View** menu unless it says otherwise.

## What the editor shows

| Command | Shortcut | What it does |
| --- | --- | --- |
| **Word Wrap** | <kbd>Alt+Z</kbd> | Wrap long lines at the window's edge instead of scrolling sideways. Line numbers stay with their lines, and the top line stays in place when you resize the window, even in very large files. |
| **Line Numbers** | | The numbers in the gutter |
| **Highlight Current Line** | | A band behind the caret's line |
| **Show Symbol > Show Whitespace** | | A dot for each space and an arrow across each tab |
| **Show Symbol > Show End of Line** | | `LF`, `CRLF` or `CR` at each line end |
| **Show Symbol > Show Invisible Characters** | | A small label for each character you otherwise couldn't see (see below) |
| **Show Symbol > Show All Characters** | | Whitespace, line ends and invisible characters together; choose it again to hide them all |
| **Show Symbol > Indent Guides** | | Faint lines at each indentation level |
| **Long-Line Marker** | | A vertical line at a column, 80 unless you change it |

**Show Invisible Characters** labels characters that look like nothing, or
like an ordinary space, so they can't hide in your text:

| Label | Character |
| --- | --- |
| `NUL`, `SOH` … `US`, `DEL`, `C1` | Control characters, such as `ESC` (character 27) |
| `NBSP`, `NNBSP` | No-break space and narrow no-break space |
| `SP`, `MMSP`, `IDSP` | Other spaces (en, em, thin, hair and so on, medium mathematical, ideographic) |
| `ZWSP`, `ZWNJ`, `ZWJ`, `WJ` | Zero-width space, non-joiner, joiner and word joiner |
| `SHY` | Soft hyphen |
| `LRM`, `RLM`, `ALM`, `LRE`, `RLE`, `PDF`, `LRO`, `RLO`, `LRI`, `RLI`, `FSI`, `PDI` | Text direction marks and controls |
| `BOM` | A byte-order mark (zero-width no-break space) inside the text |
| `LS`, `PS` | Unicode line and paragraph separators |
| `VS`, `CGJ`, `TAG`, `INV`, `HF`, `MVS`, `IAC`, `DEPR` | Variation selectors and other invisible formatting characters |

Without it, control characters still show as small symbols, but the other
characters in the table look like a space or like nothing at all.

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

## Character Panel

**View > Character Panel** opens a table of characters on the right, with
each one's decimal and hex code, HTML entity and Unicode name, like
Notepad++'s ASCII Codes Insertion Panel.

- Pick a block in the list: **ASCII**, **Latin-1 Supplement**, **Greek and
  Coptic**, **Arrows**, **Box Drawing**, **Currency Symbols** and others.
- Or search: type a character (`é`), a code (`U+00E9`, `0xE9`, `233`,
  `&eacute;`) or words of its name (`e acute`, `rightwards arrow`, `smiling
  face`). The search covers every character up to the emoji.
- Double-click a row to insert the character at the caret, or at every
  caret. The second list chooses what is inserted: **the character**, **the
  HTML entity** (`&eacute;`), **the code** (`U+00E9`) or **the decimal code**
  (`233`).
- Right-click a row to copy the character, its HTML entity or its code.

## Hex View

**View > Hex View** (<kbd>Ctrl+Alt+Shift+H</kbd>) shows the current tab as
the bytes it's saved as, in its [encoding](encodings.md) and with its line
endings: the offset of each row on the left, sixteen bytes in hex in the
middle, and the same bytes as text on the right, where anything that isn't a
printable ASCII character is a dot. Choose it again to go back to the text.
Each tab remembers whether it's in hex until you close it.

The bar at the bottom shows the offset of the byte at the cursor (in hex and
decimal), its value in hex, decimal and binary, how many bytes are selected,
and the size of the file.

| Keys | What they do |
| --- | --- |
| Arrows, <kbd>PageUp</kbd>, <kbd>PageDown</kbd> | Move the cursor; with <kbd>Shift</kbd>, select bytes |
| <kbd>Home</kbd> / <kbd>End</kbd> | Start or end of the row; with <kbd>Ctrl</kbd>, of the file |
| <kbd>0</kbd>–<kbd>9</kbd>, <kbd>A</kbd>–<kbd>F</kbd> | Type a byte in hex: the first digit waits for the second, then the byte under the cursor is replaced |
| <kbd>Tab</kbd> | Switch between the hex and text columns. In the text column, typing a character replaces the bytes under the cursor with that character's bytes |
| <kbd>Delete</kbd> / <kbd>Backspace</kbd> | Remove the selected bytes, or the byte at or before the cursor |
| <kbd>Insert</kbd> | Insert a `00` byte at the cursor, to type over |
| <kbd>Ctrl+C</kbd> | Copy the selected bytes as hex, such as `48 65 6C` |
| <kbd>Ctrl+V</kbd> | Paste hex digits over the bytes at the cursor (or over the selection) |
| <kbd>Ctrl+Z</kbd> / <kbd>Ctrl+Y</kbd> | Undo and redo, shared with the text |

Every change goes into the tab's text, so **Undo** works across both views and
**Save** writes the bytes you see.

### Changing any byte

You can type any byte anywhere, even one that leaves the text invalid in the
tab's encoding. For example, in a UTF-8 file, typing over the first byte of
`é` (`C3 A9`) leaves half a character, and a lone `0D` in a file whose line
endings are `CR LF` would be saved differently. When that happens, the tab is
**edited as bytes**: the bar says so, and **Save** writes exactly the bytes
shown. Undo the change, or finish it (type the second byte of a new
character, say), and the tab goes back to being text as soon as its bytes
are valid again.

While a tab is edited as bytes it stays in Hex View. Choosing **View > Hex
View** again shows it as text if its bytes read as text in its encoding (or
in one Ettin Edit can work out from them); otherwise the bar says why it
stays. **Encoding > Reopen With** reads the file as text in the encoding you
pick, whatever its bytes (see [Encodings](encodings.md#the-encoding-menu)).

### Binary and very large files

Files that look like binary data rather than text (images, programs,
archives: anything with a `00` byte near the start that isn't UTF-16) open
straight into Hex View, edited as bytes, without being read as text first.
Files up to 64 MB are read into memory; bigger ones are read from disk as you
scroll, so even a file of several gigabytes opens at once and changing a byte
in it is instant.

Saving writes to a new file next to the old one and then puts it in place,
so the file is never half written. For files over 64 MB, Undo starts again
after saving. Unsaved changes to files up to 64 MB come back next time like
any other tab's; for bigger files, Ettin Edit asks whether to save them when
you quit. **Copy** copies at most 16 MB.

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
