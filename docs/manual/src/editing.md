# Editing text

## The basics

Ettin Edit edits text the way you'd expect, plus a few extras:

| Keys | What they do |
| --- | --- |
| <kbd>Ctrl+Z</kbd> / <kbd>Ctrl+Y</kbd> | Undo and redo. Typing is undone a burst at a time, and the tab knows when you're back at the saved version. |
| <kbd>Ctrl+X</kbd> / <kbd>Ctrl+C</kbd> / <kbd>Ctrl+V</kbd> | Cut, copy and paste |
| <kbd>Ctrl+A</kbd> | Select everything |
| <kbd>Home</kbd> | Go to the first non-blank character of the line; press again for the very start |
| <kbd>Ctrl+Left</kbd> / <kbd>Ctrl+Right</kbd> | Move a word at a time |
| <kbd>Ctrl+Backspace</kbd> / <kbd>Ctrl+Delete</kbd> | Delete a word at a time |
| <kbd>Tab</kbd> / <kbd>Shift+Tab</kbd> on selected lines | Indent or outdent them |
| Double-click / triple-click | Select a word / a line. Keep the button down and drag to select by words or lines. |

<kbd>Enter</kbd> keeps the current line's indentation. Whether <kbd>Tab</kbd>
inserts a tab or spaces, and how wide a tab is, are set in
[Preferences](preferences.md) or **Settings > Indent With Spaces**.

## Multiple carets

Several carets let you make the same change in several places at once.
Everything you type, delete or paste happens at each of them.

| To | Do this |
| --- | --- |
| Add a caret | <kbd>Ctrl</kbd>+click or <kbd>Alt</kbd>+click |
| Add a selection | <kbd>Ctrl</kbd>+drag |
| Add a caret on the line above or below | <kbd>Ctrl+Alt+Up</kbd> / <kbd>Ctrl+Alt+Down</kbd> |
| Select the next place the selected text appears | <kbd>Ctrl+Shift+D</kbd> (**Edit > Multi-Select > Select Next Occurrence**) |
| Select every place it appears | <kbd>Alt+F3</kbd> (**Select All Occurrences**) |
| Put a caret on every search match | **Search > Select All Matches** |
| Go back to one caret | <kbd>Esc</kbd> |

Pasting several lines onto the same number of carets puts one line at each
caret, which is handy for filling in a column.

The status bar shows how many carets there are.

## Column (box) selection

Hold <kbd>Alt</kbd> and drag, or use <kbd>Alt+Shift</kbd> with the arrow
keys, to select a rectangle. Typing then replaces or inserts on every row,
and <kbd>Backspace</kbd> and <kbd>Delete</kbd> work on every row too.

### Column Mode

**Edit > Column Mode** (<kbd>Alt+Shift+C</kbd>) saves holding
<kbd>Alt</kbd>: while it's on, a plain drag and <kbd>Shift</kbd> with the
arrow keys select a rectangle, and the status bar says **Column mode**. A
plain click still puts one caret where you click, and <kbd>Shift</kbd>+click
still extends a normal selection. Click **Column mode** in the status bar, or
use the command again, to turn it off. It's off each time Ettin Edit starts.

### Column Editor

**Edit > Column Editor…** (<kbd>Alt+C</kbd>) inserts the same thing at every
caret, or at the caret's column on each line below it:

- **Text to insert**: the same text on every line.
- **Numbers to insert**: a sequence, with an **Initial number**, how much to
  **Increase by**, how many times to **Repeat** each number, **Leading
  zeros**, and **Dec**, **Hex**, **Oct** or **Bin**.

For example, start 1, increase by 1, on ten lines gives 1 to 10; with leading
zeros it gives 01 to 10.

## Line operations

**Edit > Line Operations**. The commands in this table work on the lines the
selection touches, or on the caret's line:

| Command | Shortcut |
| --- | --- |
| Duplicate | <kbd>Ctrl+D</kbd> |
| Delete Lines | <kbd>Ctrl+Shift+L</kbd> |
| Move Up / Move Down | <kbd>Ctrl+Shift+Up</kbd> / <kbd>Ctrl+Shift+Down</kbd> |
| Join Lines | <kbd>Ctrl+J</kbd> |
| Split Lines at Long-Line Column | <kbd>Ctrl+I</kbd> |
| Insert Blank Line Above / Below | <kbd>Ctrl+Alt+Enter</kbd> / <kbd>Ctrl+Alt+Shift+Enter</kbd> |

Sorting, reversing and removing lines work on the selected lines, or on the
whole document when nothing is selected. Everything is one undo step.

**Sorting**, ascending or descending:

- **Lexicographically**: in character order, so `B` comes before `a`.
- **Ignoring Case**: `a` and `A` together.
- **As Integers** and **As Decimals**: by the number at the start of each
  line, so `9` comes before `10`. Lines without a number go first.
- **By Length**: shortest first.
- **Reverse Line Order** flips the lines without sorting.

**Removing lines**:

- **Remove Duplicate Lines** keeps the first copy of each line.
- **Remove Consecutive Duplicate Lines** only removes repeats that follow
  each other, like the Unix `uniq`.
- **Remove Empty Lines** removes lines with nothing on them;
  **Including Blank Ones** also removes lines that only hold spaces or tabs.

## Spaces and tabs

**Edit > Blank Operations** work on the selected lines, or the whole document:

- **Trim Trailing Space**, **Trim Leading Space**, or both.
- **EOL to Space** joins the lines with a space between them.
- **Tabs to Spaces**, **Leading Spaces to Tabs** and **All Spaces to Tabs**,
  using the tab width from Preferences.

## Case

**Edit > Convert Case To**:

| Command | Shortcut | Example |
| --- | --- | --- |
| UPPERCASE | <kbd>Ctrl+Shift+U</kbd> | `HELLO WORLD` |
| lowercase | <kbd>Ctrl+U</kbd> | `hello world` |
| Title Case | <kbd>Alt+U</kbd> | `Hello World` |
| Sentence case | <kbd>Ctrl+Alt+U</kbd> | `Hello world` |
| iNVERT cASE | <kbd>Ctrl+Alt+Shift+U</kbd> | `hELLO wORLD` |

Below them are the identifier cases (camelCase, PascalCase, snake_case,
kebab-case and CONSTANT_CASE), which turn each line into one name; see
[Identifier case](transform.md#identifier-case).

## Comments

**Edit > Comment > Toggle Line Comment** (<kbd>Ctrl+Q</kbd>) comments out the
selected lines, or uncomments them if they're all comments.
**Toggle Block Comment** (<kbd>Ctrl+Shift+Q</kbd>) wraps the selection in a
block comment. Ettin Edit uses the right markers for the tab's language:
`//` and `/* */` in Rust, `#` in Python, `<!-- -->` in HTML, and so on.

## Date and time

**Edit > Insert > Date and Time** puts the current date and time at the
caret, short (<kbd>F5</kbd>) or long (<kbd>Shift+F5</kbd>).

## Typing helpers

Each of these can be turned off in the **Settings** menu or in
[Preferences](preferences.md).

- **Auto-Close Brackets and Quotes**: typing `(`, `[`, `{`, `"` or `'` adds
  the closing one. Typing the closer steps over it, <kbd>Backspace</kbd>
  removes an empty pair, and typing an opener with text selected wraps the
  selection.
- **Close Tags Automatically** (XML and HTML, in **Tools > XML Tools**):
  typing `>` after `<p` adds `</p>`. Empty HTML elements such as `<br>` are
  left alone.
- **Smart Indentation**: <kbd>Enter</kbd> after `{`, `(` or `[` (and after
  `:` in Python and YAML) indents the new line, and between `{}` it puts the
  closing bracket on a line of its own. Typing a closing bracket outdents.
- **Word Completion**: as you type, a list offers words already in the
  document. <kbd>Ctrl+Space</kbd> opens it at any time.
- **Keyword and Function Completion**: the list also offers the language's
  keywords and functions, each with its signature.
- **Parameter Hints**: typing `(` after a function's name shows its
  parameters above the caret, with the one you're on in bold. It follows you
  from argument to argument; <kbd>Esc</kbd> hides it and
  <kbd>Ctrl+Shift+Space</kbd> brings it back.

In the completion list, use the arrow keys and <kbd>Enter</kbd> or
<kbd>Tab</kbd> to pick a word, or <kbd>Esc</kbd> to close it.

Completion knows Python, JavaScript and TypeScript, Rust, C, C++, C#, Java,
Go, PHP, Lua, Ruby, Kotlin, Swift, SQL, PowerShell, CSS, HTML and shell
scripts, plus the functions your document defines. You can add your own
lists; see [Languages](languages.md#completion-and-parameter-hints).

## Matching brackets and tags

Putting the caret next to a bracket highlights its partner; an unmatched
bracket turns red. In HTML and XML the matching opening or closing tag is
highlighted too. Other places where the selected word appears are lightly
highlighted, which makes typos easy to spot.
