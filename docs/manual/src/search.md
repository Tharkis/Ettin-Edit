# Finding and replacing

## The find bar

**Search > Find…** (<kbd>Ctrl+F</kbd>) opens the find bar above the text. If
you had text selected, it's already filled in. Ettin Edit searches as you
type, highlights every match, and shows *3 of 17* so you know where you are.

| In the find bar | What it does |
| --- | --- |
| <kbd>Enter</kbd> / <kbd>Shift+Enter</kbd> | Next / previous match |
| **↓** / **↑** | Next / previous match (also <kbd>F3</kbd> / <kbd>Shift+F3</kbd> from anywhere) |
| **Aa** | Match case |
| **W** | Whole words only |
| **.\*** | [Regular expression](#regular-expressions) |
| **…** | More: Wrap around, Find All, Select All Matches, Mark All |
| <kbd>Esc</kbd> or **×** | Close the find bar |

With **Wrap around** on (the default), searching past the last match starts
again at the top.

**Find Selected Next** (<kbd>Ctrl+F3</kbd>) and **Find Selected Previous**
(<kbd>Ctrl+Shift+F3</kbd>) search for the selected text, or the word at the
caret, without opening the find bar.

## Replacing

**Search > Replace…** (<kbd>Ctrl+H</kbd>), or the **⏵** arrow at the left of
the find bar, adds a second row:

- **Replace** replaces the current match and moves to the next. Pressing
  <kbd>Enter</kbd> in the replace box does the same.
- **Replace All** replaces every match as one undo step. It stays fast on big
  files: 282,000 replacements in a 50 MB file take about a second.
- **In selection** limits Replace All to the selected text. It's turned on
  for you when you open the bar with several lines selected.

## Regular expressions

Turn on **.\*** to search with a regular expression. Ettin Edit uses the
syntax of the Rust `regex` library, which is close to Perl's and Python's,
except that look-around (`(?=…)`, `(?<=…)`) and back-references inside the
pattern aren't supported.

| Pattern | Matches |
| --- | --- |
| `\d+` | One or more digits |
| `^\s+$` | A line holding only spaces or tabs (`^` and `$` are line starts and ends) |
| `colou?r` | `color` or `colour` |
| `(\w+)@(\w+)\.com` | An address such as `ann@example.com`, with two groups |
| `(?<year>\d{4})-(?<month>\d\d)` | A date, with named groups |
| `(?s)begin.*?end` | `begin` to the nearest `end`, across lines |

In the replacement:

| Write | To insert |
| --- | --- |
| `$1` or `\1` | The first group (`$2`, `\2` and so on for the others) |
| `${name}` | A named group |
| `$0` | The whole match |
| `\n`, `\t` | A line break, a tab |
| `$$` | A dollar sign |
| `\\` | A backslash |

For example, find `(\w+), (\w+)` and replace with `$2 $1` to turn
`Lovelace, Ada` into `Ada Lovelace`.

## Finding everything at once

These are in the **Search** menu and the find bar's **…** menu:

- **Find All in Current Document** lists every matching line in the results
  panel at the bottom.
- **Find All in Open Tabs** does the same for every tab.
- **Select All Matches** puts a caret on every match, so you can edit them
  all at once (see [multiple carets](editing.md#multiple-carets)).
- **Mark All** highlights every match in a different colour that stays after
  you close the find bar. **Mark All and Bookmark Lines** also
  [bookmarks](#bookmarks) each line with a match. **Clear All Marks** removes
  the highlights.

## Find in Files

**Search > Find in Files…** (<kbd>Ctrl+Shift+F</kbd>) searches every file in
a folder:

- **Find what**: the text or regular expression, with the same Match case,
  Whole word and Regular expression options as the find bar.
- **Folder**: where to look. **Browse…** picks one.
- **Files**: which files, as patterns separated by spaces or semicolons, such
  as `*.rs *.toml`. Start a pattern with `!` to leave those files out, for
  example `*.js !*.min.js`. Leave it empty for every file.
- **Include subfolders** and **Include hidden files and folders**.

The search runs in the background, so you can keep working; **Stop** ends it
early. Binary files are skipped.

You can also start Find in Files from the
[folder workspace](view.md#folder-as-workspace): right-click a folder and
choose **Find in Files Here…**.

## The results panel

Find All and Find in Files show their results in a panel below the text,
grouped by file, with the matching words highlighted.

- Click a line to jump to it; the file opens if it isn't open yet.
- Click a file's heading to fold its results away; **Collapse All** and
  **Expand All** do every file.
- **×** closes the panel.

## Go to Line

**Search > Go to Line…** (<kbd>Ctrl+G</kbd>), or a click on *Ln, Col* in the
status bar, asks for a line number. Type `120` to go to line 120, or
`120:15` for line 120, column 15.

## Bookmarks

Bookmarks mark lines you want to come back to. A bookmarked line shows a
marker in the gutter.

| Command (Search > Bookmark) | Shortcut |
| --- | --- |
| Toggle Bookmark (or click the gutter) | <kbd>Ctrl+F2</kbd> |
| Next Bookmark | <kbd>F2</kbd> |
| Previous Bookmark | <kbd>Shift+F2</kbd> |
| Clear All Bookmarks | |

The same submenu works on the bookmarked lines as a group: **Cut**, **Copy**
or **Remove Bookmarked Lines**, **Remove Unbookmarked Lines** (keep only the
bookmarked ones), and **Inverse Bookmarks**.

Together with **Mark All and Bookmark Lines**, that gives a quick way to
pull out every line that matches something: mark them, then **Copy
Bookmarked Lines** and paste them into a new tab.
