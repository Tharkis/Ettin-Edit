# Transforming text

The **Transform** menu turns text into other text: filter it through a shell
command, keep or drop lines, number them, rewrap paragraphs, rename
identifiers, convert data between formats, and work out hashes. It's also a
tab on the [toolbar](toolbar.md) and a submenu of the right-click menu.

Every command works on the selection (each one, if there are several) or on
the whole document when nothing is selected, and is one undo step. Commands
that work on lines take every line a selection touches, so you don't have to
select them exactly. If a command can't finish, a message says why and the
text is left as it was.

## Filter through a command

**Transform > Filter Through Command…** (<kbd>Ctrl+Alt+P</kbd>) sends the
text to a shell command and puts the command's output in its place. Anything
that reads standard input and writes standard output works: `sort -u`,
`jq .`, `column -t`, `awk '{print $2}'`, `python3 -c "…"` and so on.

- On Linux the command runs with `sh -c`; on Windows with `cmd /C`.
- It runs in the file's folder (your home folder for a new tab), and the
  environment variable `ETTIN_FILE` holds the file's path.
- The text goes to the command as UTF-8 with `\n` line breaks, and its output
  is read as UTF-8. When the selection doesn't end with a line break, a final
  line break in the output is dropped, so the text around it stays put.
- While the command runs, the dialog shows it with a **Stop** button. Nothing
  changes until it finishes.
- If the command fails (a non-zero exit code), the dialog shows its error
  output and the text stays as it was.
- **Recent commands** in the dialog lists the last 15 commands you ran.
- In the Flatpak, commands run on your computer outside the sandbox, so your
  own tools are found.

## Lines

**Transform > Lines**:

| Command | What it does |
| --- | --- |
| **Keep Matching Lines…** (<kbd>Ctrl+Alt+K</kbd>) | Keeps only the lines that contain the text you type, like `grep` |
| **Remove Matching Lines…** | Removes those lines instead, like `grep -v` |
| **Count Distinct Lines** | Opens a new tab with each different line once and how many times it appears, most frequent first, as `count`, a tab, then the line |
| **Add Prefix and Suffix…** | Adds text to the start and end of each line, for example `'` and `',` to turn a list into SQL values. **Skip empty lines** leaves blank lines alone |
| **Number Lines…** | Puts a number in front of each line: pick the first number, the step, the text after the number (`. ` by default), and whether numbers are aligned with spaces, padded with zeros, or neither |
| **Reflow Paragraphs…** | Rewraps paragraphs so no line is longer than the width (the long-line column from Preferences, unless you change it) |

The matching-lines dialog has **Match case** and **Regular expression**
options, and shows how many lines match as you type. Regular expressions use
the same syntax as [Find](search.md#regular-expressions).

Reflow joins the lines of each paragraph and breaks them again at spaces. A
paragraph ends at a blank line or at a list item (`-`, `*`, `+` or `1.`).
Indentation, comment markers (`//`, `#`, `>`, `--`, `;`) and list bullets are
kept, and continuation lines of a list item line up under its text. A word
longer than the width stays whole on its own line.

## Identifier case

**Transform > Identifier Case** (also at the end of **Edit > Convert Case
To**) turns each line into one name in a programming style. The words are
found wherever the text has spaces, punctuation, `_`, `-` or a change of case,
so `parse HTTP request`, `parseHttpRequest` and `PARSE_HTTP_REQUEST` all
become:

| Command | Result |
| --- | --- |
| camelCase | `parseHttpRequest` |
| PascalCase | `ParseHttpRequest` |
| snake_case | `parse_http_request` |
| kebab-case | `parse-http-request` |
| CONSTANT_CASE | `PARSE_HTTP_REQUEST` |

With nothing selected they convert the word at the caret, like the other case
commands.

## Converting data

**Transform > Convert** converts between JSON, YAML, TOML, XML and CSV, and
writes Markdown tables. Pick the format the text is in, then the one you
want: **From JSON > JSON to YAML**, **From CSV > CSV to Markdown**, and so on.

When the whole document is converted, the tab's language switches to the new
format; the file name doesn't change until you save it under another name.
Keys keep their order. JSON and XML are indented with the tab width and the
tabs-or-spaces setting from [Preferences](preferences.md).

How each format maps to the others:

- **YAML**: keys that aren't text (`1:`, `true:`) become text keys. A file
  with several documents (`---`) can't be converted in one go; select one.
- **TOML** needs a table at the top. A list becomes `[[item]]` tables. TOML
  has no null, so keys that are null are left out, and a null inside a list
  stops the conversion. Dates become text in the other formats.
- **XML to the others**: the root element becomes the one key at the top.
  Attributes become keys starting with `@`, repeated child elements become a
  list, text next to attributes or child elements goes in `#text`, and an
  empty element becomes null.
- **The others to XML** works the same way back. When the top isn't a single
  key, it's wrapped in `<root>`; keys that can't be element names have the
  odd characters replaced by `_`.
- **CSV to the others**: the first row names the columns, and each row becomes
  an object. The delimiter and quotes are detected as in
  [CsvQuery](csvquery.md); columns that are all numbers become numbers, but
  values with leading zeros, such as zip codes, stay text.
- **The others to CSV or Markdown**: a list of objects becomes one row each,
  with a column for every key that appears. A list of lists is used as it is,
  the first one being the header. An object that holds just one list is
  treated as that list. Values that are themselves lists or objects are
  written as JSON in their cell.

## Hashes

**Transform > Hash** has MD5, SHA-1, SHA-256, SHA-512 and CRC32.

- With a selection, each selection is replaced by its hash.
- With nothing selected, the hash of the whole document is copied to the
  clipboard and shown, and the text is left alone. It's worked out from the
  text in UTF-8 with the document's line endings, so for a UTF-8 file it
  matches `sha256sum` and friends.

Hashes are written in lower-case hex.

## Hex and JWT

**Transform > Hex and JWT**:

- **Text to Hex** writes the text's UTF-8 bytes as hex pairs: `Hé` becomes
  `48 C3 A9`.
- **Hex to Text** reads them back. Spaces, line breaks, commas, colons and
  `0x` or `\x` in front of bytes are all fine. If the bytes aren't UTF-8 text,
  use [Hex View](view.md#hex-view) instead.
- **Decode JWT** turns a JSON Web Token (a `Bearer ` in front is fine) into
  JSON with its `header` and `payload`, and says when it was issued and when
  it expires, in your local time. The signature is not checked, so don't
  trust what a token says because it decoded.

## Macros and plugins

Transform commands are recorded in [macros](macros.md) with the settings you
chose in their dialog, so a macro repeats exactly what you did. **Filter
Through Command** is the exception: a macro never runs shell commands. Every
Transform command is also in the [Shortcut Mapper](shortcuts.md) and the
[command palette](view.md#command-palette).
