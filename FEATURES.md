# Ettin Edit Features

This document lists the features of Ettin Edit, the release each one arrived in, and the
work that remains. It is updated with every release.

| Status | Meaning |
| --- | --- |
| `[x]` | Available |
| `[~]` | Partially available (see the note) |
| `[ ]` | Planned, not yet available |

## Contents

- [Release history](#release-history)
- [Plugin system](#plugin-system)
- [Notepad++ plugin equivalents](#notepad-plugin-equivalents)
- [Known bugs](#known-bugs)
- [Roadmap](#roadmap)

## Release history

### 1.9: Markdown Preview and Hex View finished

Requested on 2026-10-16: the parts of the 1.5 Markdown Preview and Hex View that were still missing.

- [x] Markdown Preview shows images: PNG, JPEG, GIF, BMP, WebP and SVG (with its text), from files next to the Markdown file or from the web, no wider than the panel; an image that can't be loaded shows its description, and an image inside a link follows the link ([Using plugins](docs/manual/src/plugins.md#markdown-preview))
- [x] Markdown Preview scrolls along with the text: scroll the editor and the preview shows the same place, down to the end; untick **Scroll with the text** to scroll it on its own
- [x] Plugins: `ui.markdown(text, base=..., doc=..., line=...)` loads images from a folder and follows a tab's scrolling ([Writing plugins](docs/manual/src/plugin-guide.md#panels)); Markdown Preview sample 1.1.0
- [x] Hex View changes any byte in any encoding: an edit that leaves the text invalid (half a UTF-8 character, a lone CR in a CR LF file) turns the tab into bytes, saved exactly as shown, and it becomes text again as soon as its bytes are valid ([Changing any byte](docs/manual/src/view.md#changing-any-byte))
- [x] Binary files (a `00` byte near the start that isn't UTF-16) open straight into Hex View without being read as text; files over 64 MB are read from disk as you scroll, so multi-gigabyte files open at once ([Binary and very large files](docs/manual/src/view.md#binary-and-very-large-files))
- [x] Byte edits are kept in a piece table with their own undo; saving goes through a temporary file, so the file is never half written
- [x] Unsaved byte edits come back after a restart like text ones (files up to 64 MB; for bigger ones Ettin Edit asks to save when you quit)
- [x] New text in all four interface languages

### 1.8: Word wrap on very large files

Reported on 2026-10-16.

- [x] Resizing the window with word wrap on no longer stalls on files with hundreds of thousands of lines. Lines that fit the new width are never read again; long lines are re-wrapped visible ones first, the rest a few at a time in the background while the window stays responsive (a resize step on a 500,000-line file went from about 0.65 s to under 10 ms)
- [x] The line at the top of the view stays there when the window is resized or word wrap is turned on or off
- [x] A document shown in both views of a split at different widths keeps the wrapping for both, instead of re-wrapping the whole file every frame

### 1.7: Updates from inside Ettin Edit

Requested on 2026-10-16.

- [x] **Update Now** in Check for Updates: Ettin Edit downloads the release file that matches how it was installed (Arch package, `.deb`, `.tar.gz`, Windows installer or `.zip`), checks its SHA-256 against the checksum GitHub lists, installs it and offers **Restart Now** ([Updating](docs/manual/src/installing.md#updating))
- [x] Packages install through `pkexec pacman -U` or `pkexec apt-get install`, which shows the desktop's password prompt; without pkexec the dialog shows the command to run in a terminal
- [x] The `.tar.gz` and `.zip` copies replace their own program; the Windows installer runs silently as Ettin Edit closes and starts the new version
- [x] Restarting closes like File > Exit (unsaved work is asked about or kept) and reopens your files
- [x] A Help group on the toolbar's Home tab (User Manual, User Manual on the Web, Check for Updates, About), so Help can be reached with the menu bar hidden. Customised toolbars get the group added once ([The toolbar](docs/manual/src/toolbar.md#where-the-layout-is-kept))
- [x] New text in all four interface languages

### 1.6.1: Finding line breaks

Reported on 2026-10-16.

- [x] `\r` and `\r\n` find line breaks in regular expressions, as they already did in Extended mode, and `\r?\n` still finds one line break ([Finding and replacing](docs/manual/src/search.md#line-breaks))
- [x] Matches on a line break are highlighted (they used to have no width, so a search for one seemed to find nothing)
- [x] `\r` in a regular-expression replacement inserts a line break instead of a stray carriage return
- [x] Replacing with an empty box deletes the matches, in normal, Extended and regular-expression modes (now covered by tests)

### 1.6: A tidier window

Requested on 2026-10-16.

- [x] Hide the menu bar: View > Menu Bar, <kbd>Ctrl+M</kbd>, or the toolbar's right-click menu. While it's hidden, a tap of Alt shows it until you run a command, press Escape or click elsewhere, so you can't lose the menus ([A tour of the window](docs/manual/src/tour.md#the-menu-bar))
- [x] The toolbar and the menu bar can each be hidden, and both choices are remembered
- [x] The app icon is no longer in the menu bar (Help > About Ettin Edit shows the version)
- [x] New text in all four interface languages

### 1.5: Hex View, Character Panel, Markdown Preview and more

The six features requested on 2026-10-16.

- [x] Markdown Preview, a new sample plugin (Python): a side panel showing the Markdown tab rendered, updating a moment after you stop typing and keeping its place; it follows the active tab, opens links to other files, scrolls to `#heading` links and skips YAML front matter (<kbd>Ctrl+Shift+M</kbd>; [Using plugins](docs/manual/src/plugins.md#markdown-preview)). Images showed as their description until [1.9](#19-markdown-preview-and-hex-view-finished); raw HTML is left out
- [x] Plugins can show Markdown in a panel: a new `markdown` panel item (`ui.markdown(text)`), drawn like the user manual, with a `link` event for links that aren't web pages ([Writing plugins](docs/manual/src/plugin-guide.md#panels))
- [x] Hex View (View > Hex View, <kbd>Ctrl+Alt+Shift+H</kbd>): a tab's bytes as saved (in its encoding and line endings) with offsets, hex and text columns; type hex digits or characters to change bytes, Delete, Backspace and Insert, copy and paste bytes as hex, undo shared with the text. Edits that would leave invalid text in the tab's encoding were refused until [1.9](#19-markdown-preview-and-hex-view-finished) ([Hex View](docs/manual/src/view.md#hex-view))
- [x] Column Mode (Edit > Column Mode, <kbd>Alt+Shift+C</kbd>): a plain drag and Shift+arrows select a rectangle without holding Alt; the status bar shows it's on ([Column Mode](docs/manual/src/editing.md#column-mode))
- [x] Extended search mode (the `\n` button in the find bar, and Find in Files): `\n`, `\r`, `\t`, `\0`, `\\`, `\xNN`, `\uNNNN`, `\u{…}`, `\dNNN`, `\oNNN` and `\bNNNNNNNN` in Find and Replace mean those characters ([Extended mode](docs/manual/src/search.md#extended-mode-n-t-xnn))
- [x] View > Show Symbol > Show Invisible Characters: labels such as `NBSP`, `ZWSP`, `BOM`, `RLO` and `ESC` for control characters, unusual spaces, zero-width and direction characters; and **Show All Characters**, which turns whitespace, line ends and invisible characters on or off together (also in Preferences and on the toolbar's View tab)
- [x] Character Panel (View > Character Panel): characters by Unicode block (ASCII, Latin-1, Greek, arrows, box drawing, currency and more) with decimal and hex codes, HTML entities and Unicode names; search by character, code or name; double-click to insert the character, its HTML entity or its code; right-click to copy ([Character Panel](docs/manual/src/view.md#character-panel))
- [x] New toolbar buttons for Column Mode, Show All Characters, Character Panel and Hex View, and all new text in the four interface languages

### 1.4: Package information and updates through APT (in progress)

- [x] Fixed: the `.deb` had no licence, release notes, age rating or update source, so software centres showed "Unknown License" and "No Software Repository Included" and never updated it ([Installing and updating](docs/manual/src/installing.md#getting-updates-on-debian-and-ubuntu))
- [x] AppStream metadata (`io.github.tharkis.EttinEdit.metainfo.xml`) in the `.deb`, the Arch package and the `.tar.gz`: MIT licence, website, bug tracker, manual, screenshots, release notes and an "all ages" rating
- [x] The `.deb`'s copyright file is in the machine-readable Debian format
- [x] A signed APT repository at `https://tharkis.github.io/Ettin-Edit/apt`, rebuilt after every release from the latest `.deb` files; the `.deb` adds it, so the software updater installs new versions

### 1.3: The toolbar

- [x] A tabbed toolbar under the menu bar, like the Office ribbon: Home, Edit, View and Tools tabs, each with groups of large and small buttons for common commands ([The toolbar](docs/manual/src/toolbar.md))
- [x] Buttons show their command's menu path and shortcut on hover, are outlined while a setting they switch is on, and grey out when the command can't run
- [x] Settings > Customise Toolbar… (also from the toolbar's right-click menu, View and Preferences): put any menu command on it, including themes, languages, encodings, saved macros and plugin commands, by dragging or with Add
- [x] Add, rename, reorder, hide and remove tabs and groups; move buttons by dragging or with the arrow buttons; change a button's label, size and icon (1,500 Phosphor icons, MIT licence); Reset to Defaults
- [x] Right-click a button to remove it; collapse the toolbar to its tab names (double-click a tab, the arrow button, View > Collapse Toolbar or Ctrl+F1), and click a tab to show its buttons over the text; View > Toolbar hides it
- [x] The layout is kept in `toolbar.json` in the settings folder; there's no file until you change something, so you get new default buttons in later versions
- [x] Plugins can give commands a toolbar icon: a Phosphor icon name or a PNG in the plugin's folder (`icon` in `@command` / `command`); the sample plugins have icons
- [x] Toolbar text in all four interface languages

### 1.2: Translations and the user manual

#### Translations
- [x] Every menu, dialog, message, tooltip and Preferences page can be shown in another language, built on [Fluent](https://projectfluent.org/)
- [x] Built-in English, German, French and Spanish; the German, French and Spanish texts are machine-made starting points that still need a native speaker's review
- [x] Settings > Preferences > Files and Startup > Interface language: follows the system language by default (`de_AT` finds German), or pick one; the window switches at once, with no restart
- [x] Your own translations: `.ftl` files in the `translations` folder (Open Translations Folder in Preferences) add a language or change single messages of a built-in one; `english-source.ftl.txt` there is the English to copy from, and anything a file leaves out stays English ([Translations](docs/manual/src/translations.md))
- [x] Plugins can ask which language the menus are in (`ui_language()` / `uiLanguage()`) and translate their own text
- [x] Tests check that every message the app uses exists in English, every English message is used, and the built-in translations have every message with the same `{ $values }`
- [~] Stays in English: the user manual, language and theme names, parser messages from the text tools (JSON, XML, XPath, XSD, XSLT, SQL), plugin messages and generated templates; right-to-left languages can't be laid out yet

#### User manual
- [x] A user manual covering every part of the editor and how to customise it: 23 pages from installing to troubleshooting
- [x] Built into the app and readable offline: Help > User Manual (<kbd>Shift</kbd>+<kbd>F1</kbd>), with contents, search, back and forward, links between pages and a Copy button on examples
- [x] The same manual on the web at https://tharkis.github.io/Ettin-Edit/, published by GitHub Pages whenever it changes on `main`
- [x] Language > Language Guide and Plugins > Plugin Guide open their pages of the manual
- [x] Tests check that every page is in the contents, every link and heading anchor works, and every default shortcut is listed

### 1.1: Syntax-aware folding, keyword completion and parameter hints

#### Code folding
- [x] Folding follows the language's grammar instead of indentation: multi-line `{ }`, `[ ]` and `( )` (not inside strings or comments), HTML/XML/JSX tags (empty HTML elements and `<tag/>` don't open one), the code inside `<script>` and `<style>`, keyword blocks in Ruby, Lua, shell scripts, Pascal and CMake, LaTeX environments, INI/TOML sections, Markdown headings and fenced code, multi-line comments and runs of line comments, and `#region` / `#endregion` markers
- [x] `folding` pairs in `.lang.json` languages, brackets or words (`["if", "end"]`), drive folding
- [x] The closing line stays visible, so `} else {` folds each half; Python, YAML, plain text and files without blocks still fold by indentation
- [x] The scan runs in the background and resumes from the first edited line, so typing in big files stays smooth

#### Completion and parameter hints
- [x] The completion list offers the language's keywords and functions next to words from the document, each marked as a keyword or with its signature
- [x] Parameter hints: typing `(` after a function shows its signatures above the caret, with the current parameter in bold; it follows the caret between arguments and nested calls, Esc closes it, Ctrl+Shift+Space shows it again (Edit > Auto-Completion)
- [x] Signatures of the functions the document defines, read from their definitions (parameters spread over several lines too), for the ~25 languages the function list understands
- [x] Built-in keyword and function lists for Python, JavaScript/TypeScript, Rust, C, C++, C#, Java, Go, PHP, Lua, Ruby, Kotlin, Swift, SQL, PowerShell, CSS, HTML and shell scripts; other languages complete the keywords their grammar colours
- [x] Your own lists: `.json` files in the `api` folder (Language > Open Completion Lists Folder) add keywords and signatures for any language ([Languages](docs/manual/src/languages.md#completion-and-parameter-hints))
- [x] Settings > Keyword and Function Completion and Settings > Parameter Hints, also in Preferences

#### Other
- [x] Application icon in the top-left corner of the window (click it for About), plus 32 to 128 px sizes in the Linux icon theme

### 0.1: Tabbed editor, themes and syntax highlighting

#### Tabs and files
- [x] Multiple documents in tabs, with a `+` button and Ctrl+N for new tabs
- [x] Open (multi-select) with Ctrl+O, Save Ctrl+S, Save As Ctrl+Alt+S, Save All Ctrl+Shift+S
- [x] Close tab (Ctrl+W, middle-click, × button), Close All, Close Others
- [x] Unsaved-changes dot on the tab and `*` in the window title
- [x] "Save changes?" prompt when closing a modified tab or quitting
- [x] Next/previous tab with Ctrl+Tab / Ctrl+Shift+Tab and Ctrl+PgDn / Ctrl+PgUp
- [x] Tab right-click menu: Close, Close Others, Copy Full Path, Copy File Name
- [x] Opening a file replaces an untouched "new" tab
- [x] Files passed on the command line open in tabs
- [x] Drag and drop files onto the window to open them
- [x] Recent files list (File > Open Recent)
- [x] Session restore: reopens the files that were open last time
- [x] Line endings detected (LF, CRLF, CR), preserved on save, switchable from the status bar
- [x] UTF-8 with and without BOM, preserved on save; warning for files that aren't valid UTF-8 (other encodings added in 0.5)

#### Editing and view
- [x] Line numbers gutter (stays aligned with word wrap on), toggle in View menu
- [x] Current line highlight, toggle in View menu
- [x] Word wrap (Alt+Z)
- [x] Zoom in/out/reset (Ctrl+= / Ctrl+- / Ctrl+0)
- [x] Undo/redo, clipboard, select all
- [x] Status bar: language, length, line count, Ln/Col, selection length, line ending, encoding, zoom

#### Syntax highlighting
- [x] Highlighting for about 65 languages (Rust, Python, JavaScript, JSON, XML, HTML, CSS, C/C++, C#, Java, Go, PHP, SQL, YAML, Markdown, shell, and more)
- [x] Language detected from the file name, then from the first line (shebang, `<?xml`)
- [x] Language menu grouped A–Z, plus a quick picker on the status bar

#### Themes
- [x] Dark mode by default, with light themes available
- [x] 9 built-in themes: Ettin Dark (the default, based on Base16 Eighties), Ettin Night, Ettin Light (original), Ocean Dark/Light, Mocha, Solarized Dark/Light, GitHub Light
- [x] Whole window (menus, tabs, status bar) follows the editor theme
- [x] Custom themes: drop any `.tmTheme` file into the themes folder (Theme > Open Themes Folder)
- [x] The built-in themes are files in the themes folder too, so they can be edited (Theme > Reload Themes, Theme > Restore Built-in Themes)
- [x] Theme, zoom, wrap and other view settings remembered between runs

#### Platform
- [x] Linux (X11 and Wayland) and Windows builds from one codebase
- [x] Settings in `~/.config/EttinEdit` (Linux) or `%APPDATA%\EttinEdit` (Windows)

### 0.2: Editor engine

- [x] Ettin Edit's own editor engine on a rope buffer (replaces egui's text box)
- [x] Undo/redo history with typing merged into steps, and the saved point tracked through undo
- [x] Multiple carets: Alt+click to add, Ctrl+Alt+Up/Down to add above or below, Esc to go back to one
- [x] Typing, deleting, pasting and Enter work at every caret; pasting N lines onto N carets puts one line at each
- [x] Double-click selects a word, triple-click a line; drag extends by char, word or line
- [x] Enter keeps the current line's indent; Tab / Shift+Tab indent and outdent selected lines
- [x] Smart Home (first non-blank, then column 0), Ctrl+arrows by word, Ctrl+Backspace/Delete by word
- [x] Right-click menu: Undo, Redo, Cut, Copy, Paste, Select All
- [x] Large files keep their colours: incremental highlighting parses only what is on screen (a 50 MB, 940,000-line file opens in about a second)
- [x] Very long lines (minified files) stay responsive: lines over 20 KB are shown unstyled
- [x] Chinese, Japanese and Korean text uses a system CJK font when one is installed
- [x] Application icon, Linux `.desktop` file, and an Arch/CachyOS `PKGBUILD`
- [x] CI builds and tests for Linux and Windows on every pull request

### 0.3: Search and navigation

#### Find and replace
- [x] Find bar (Ctrl+F) and Replace (Ctrl+H) with match case, whole word, wrap around and regular expressions
- [x] Regex replace with capture groups (`$1`, `${name}` or `\1`) and `\n` / `\t` in the replacement
- [x] Search as you type, with every match highlighted and an "N of M" count
- [x] Find Next / Previous (F3 / Shift+F3), Find Selected Next / Previous (Ctrl+F3 / Ctrl+Shift+F3)
- [x] Replace one, Replace All (one undo step), Replace All in selection
- [x] Select All Matches: puts a caret on every match for multi-caret editing
- [x] Find All in Current Document and Find All in Open Tabs, listed in a results panel
- [x] Find in Files (Ctrl+Shift+F): folder, file filters (`*.rs *.toml`, `!name` to skip), subfolders, hidden files; runs in the background with a Stop button; skips binary files
- [x] Results panel: hits grouped by file with the matches highlighted; click a line to jump there; collapse and expand groups
- [x] Mark All (optionally bookmarking the matching lines) and Clear All Marks
- [x] Other occurrences of the selected word are highlighted
- [x] Big files: search, highlighting and Replace All stay fast on a 50 MB file (282,000 replacements in about a second)

#### Navigation
- [x] Go to Line (Ctrl+G, or click Ln/Col in the status bar); accepts `line` or `line:column`
- [x] Bookmarks: toggle with Ctrl+F2 or a click in the gutter, F2 / Shift+F2 to jump, Clear All
- [x] Cut, copy or remove bookmarked lines, remove unbookmarked lines, inverse bookmarks

### 0.4: Editing tools

#### Multiple cursors and column editing
- [x] Select next occurrence (Ctrl+Shift+D) and select all occurrences (Alt+F3) for multi-caret edits
- [x] Ctrl+click or Alt+click adds a caret, Ctrl+drag adds a selection (0.7); Esc goes back to one
- [x] Column (box) selection with Alt+drag or Alt+Shift+arrows; typing and deleting work on every row
- [x] Column Editor (Alt+C): insert text or a number sequence (start, step, repeat, leading zeros, dec/hex/oct/bin) down a column

#### Line operations (Edit > Line Operations)
- [x] Duplicate (Ctrl+D), delete (Ctrl+Shift+L), move up/down (Ctrl+Shift+Up/Down)
- [x] Join (Ctrl+J), split at the long-line column (Ctrl+I), insert blank line above or below
- [x] Sort ascending or descending: lexically, ignoring case, as integers, as decimals, by length; reverse order
- [x] Remove duplicate lines, consecutive duplicates, empty lines, or blank lines
- [x] Trim trailing, leading or both; end-of-line to space; tabs to spaces; spaces to tabs (leading or all)

#### Text tools
- [x] Case conversion: UPPER (Ctrl+Shift+U), lower (Ctrl+U), Title (Alt+U), Sentence (Ctrl+Alt+U), iNVERT (Ctrl+Alt+Shift+U)
- [x] Toggle line comment (Ctrl+Q) and block comment (Ctrl+Shift+Q), with the right markers for each language
- [x] Insert date and time, short (F5) or long (Shift+F5)
- [x] Read-only mode (Edit menu, or click "Read-only" in the status bar)

#### Typing helpers (each can be turned off in the Settings menu)
- [x] Auto-close brackets and quotes; typing a closer steps over it; Backspace removes an empty pair; selected text gets wrapped
- [x] Smart indent: Enter indents after `{`, `(`, `[` (and after `:` in Python, YAML and Nim) and splits `{}` onto separate lines; a closing bracket outdents
- [x] Word completion from words in the document, as you type or with Ctrl+Space
- [x] Insert spaces instead of tabs (optional)
- [x] Matching bracket highlight (red when unmatched) and matching HTML/XML tag highlight

#### View
- [x] Code folding by indentation: boxed −/+ markers in the gutter with a line showing where each region starts and ends, fold/unfold current (Ctrl+Alt+F / Ctrl+Alt+Shift+F), fold all (Alt+0), unfold all (Alt+Shift+0), fold level 1–5 (Alt+1…5)
- [x] Indent guides
- [x] Show whitespace (dots for spaces, an arrow across the full width of each tab) and end-of-line markers (LF, CRLF, CR)
- [x] Long-line edge marker at a configurable column (default 80)

#### Macros
- [x] Record (Ctrl+Shift+R) and play back (Ctrl+Shift+P) typing, keys, clipboard and edit commands
- [x] Run a macro N times or until the end of the file
- [x] Save macros by name; saved macros appear in the Macro menu and survive restarts

### 0.5: Files, encodings and data safety

#### Encodings
- [x] Encoding detected on open: byte-order marks, UTF-8, UTF-16 without a BOM, then a guess for legacy encodings (Windows-1252, Windows-1251, Shift_JIS and others)
- [x] Encoding menu (also on the status bar): save as UTF-8, UTF-8 BOM, UTF-16 LE/BE, or about 35 other encodings grouped by region (Windows code pages, ISO-8859-x, KOI8, Mac Roman, GBK, Big5, Shift_JIS, EUC-KR...)
- [x] Reopen With: read the file again in an encoding you pick, when the guess is wrong
- [x] Saving refuses a character the chosen encoding can't hold, and says which one, instead of losing it

#### Unsaved work and files changed on disk
- [x] Unsaved changes, including new untitled tabs, are backed up every few seconds and come back after a crash
- [x] Quitting keeps unsaved changes for next time without asking (Settings > Remember Unsaved Changes on Exit; turn it off to be asked instead)
- [x] Session remembers each tab's encoding, line ending, language, caret, pin and read-only state
- [x] A bar above the editor when another program changes or deletes the open file: Reload (undoable) or Keep My Version
- [x] Optional automatic reload of changed files that have no unsaved changes (Settings menu)
- [x] File > Reload from Disk

#### Window and tabs
- [x] Single window: opening a file while Ettin Edit runs sends it to the open window (turn off in Settings, or start with `--multi-instance`)
- [x] Drag tabs to reorder them
- [x] Pin tabs: pinned tabs stay at the front, show a pin, and are skipped by Close All and Close Others

#### File commands (File menu and tab right-click menu)
- [x] Rename the file on disk
- [x] Move the file to the Recycle Bin / trash
- [x] Open Containing Folder (selects the file where the file manager supports it)
- [x] Open Terminal Here (Windows Terminal or cmd on Windows; `$TERMINAL`, Konsole, GNOME Terminal and others on Linux)
- [x] Summary dialog: path, dates, encoding, characters, words, lines, size on disk, selection

#### Fixes
- [x] The mouse pointer is an arrow over the gutter (fold markers, bookmarks, line numbers), not a text cursor
- [x] Theme > Open Themes Folder works on Windows (and on Linux it uses the desktop's file manager)

### 0.6: JSON, JavaScript, MIME and XML tools

All of these tools are in the Tools menu. The individual plugin features are listed under
[Notepad++ plugin equivalents](#notepad-plugin-equivalents); this section covers the
surrounding behaviour.

- [x] Tools work on each selection (multi-cursor too) or on the whole document, as one undo step
- [x] Errors show as a notice with line and column and a Go To button; the caret moves to the error
- [x] JSON Format and Minify fall back to JavaScript when the text isn't strict JSON, and say so
- [x] JavaScript formatter handles template strings, regex literals, comments, `switch`/`case` and modern syntax
- [x] JSON tree (Ctrl+Alt+J): updates as you type, opens at the caret, has a filter box, and right-click copies a path like `$.items[2].name`
- [x] XPath panel (Ctrl+Alt+Shift+E): full XPath 1.0 with every core function plus `ends-with`, `lower-case` and `upper-case`; node results jump to the text, other results show the value; recent expressions
- [x] Current XPath on the status bar (click to copy), kept while you are still typing unclosed tags
- [x] Auto-close tags in XML and HTML (skips HTML void elements like `<br>`); Tools > XML Tools > Close Tags Automatically
- [x] Shortcuts: Ctrl+Alt+M format JSON/JS, Ctrl+Alt+N minify, Ctrl+Alt+Shift+B pretty print XML, Ctrl+Alt+Shift+L linearize, Ctrl+Alt+Shift+M check XML, Ctrl+Alt+Shift+P copy XPath

### 0.6.1: Context menu

- [x] The editor's right-click menu has the whole Edit menu (Undo through Read-Only, with every submenu) and the Tools menu
- [x] Undo, Redo, Cut and Copy are greyed out when there is nothing to undo, redo or copy (in the Edit menu too)

### 0.7: Split view and Compare

#### Split view (View menu and tab right-click menu)
- [x] Move a tab to the other view, or clone it so the same document shows in both views with separate carets and scrolling
- [x] Each view has its own tab bar and active tab; F8 switches views; drag the divider to resize
- [x] Closing a clone only closes that view's copy; the split ends when one view is empty
- [x] The split layout is saved with the session

#### Compare (Tools > Compare, and the right-click menu)
- [x] Opens as a Compare tab: both sides aligned row by row, read-only, and updated as you edit the compared tabs
- [x] Double-click a line to jump to it in its tab for editing
- [x] Toolbar: first, previous, next and last difference, the change summary, Swap Sides, Export Patch, and the options
- [x] Shortcuts: Ctrl+Alt+D compares the two views (or this tab with the one used before it), F7 / Shift+F7 next and previous difference
- [x] Compare With any open tab, the last save, the clipboard, the last Git commit, or two selections (Ctrl+drag a second selection, or select in each view)
- [x] Moved blocks are coloured apart from added and removed lines
- [x] Very large or very different files stay responsive (the diff has a time limit and the view only draws the visible rows)

### 0.8: CsvQuery

Tools > CsvQuery (Ctrl+Alt+Q) opens a query panel below the editor. The individual CsvQuery
features are listed under [Notepad++ plugin equivalents](#notepad-plugin-equivalents).

- [x] The current tab is the table `this`; every other tab is a table named after its file (`orders 2024.csv` is `orders_2024`), loaded when a query names it
- [x] Delimiters: comma, semicolon, tab, pipe and space; decimal commas (`3,50`) count as numbers in semicolon files
- [x] Values with leading zeros (zip codes, ids) stay text; empty fields are NULL
- [x] Queries run in the background and re-run as you edit the tab or switch tabs; Ctrl+Enter runs the SQL box
- [x] Results: click a column header to sort, right-click a cell to copy it or its row, first 100,000 rows shown
- [x] Columns view: each column's type (changeable), filled, empty and distinct counts, min, max and average
- [x] Recent queries menu, and a Tables menu that inserts table names
- [x] UPDATE, DELETE and CREATE work on the query copy; the file itself never changes

### 0.9: Plugin system

Plugins are folders in the plugins folder (`~/.config/EttinEdit/plugins/` on Linux,
`%APPDATA%\EttinEdit\plugins\` on Windows) with a `plugin.json` and a Python or
JavaScript script. See [Writing plugins](docs/manual/src/plugin-guide.md), also under Plugins > Plugin Guide.

- [x] Plugins menu: each running plugin's commands with their shortcuts, Manage Plugins, Plugin Log, Open Plugins Folder, Reload Plugins, Install Sample Plugins and Plugin Guide; the right-click menu has the plugin commands too
- [x] Plugin commands can go on the toolbar, with an icon the plugin chooses (1.3)
- [x] Plugins run as their own processes; Ettin Edit finds `python3`/`python` (`python`, `py -3` on Windows) and `node`, or the paths set in Manage Plugins
- [x] A small SDK for each language (`import ettin-edit` / `require("ettin-edit")`) that Ettin Edit keeps up to date
- [x] API: documents (list, open, new, save, close, activate, info), text (get, set, replace ranges, insert), selections, go to line, run any built-in command, messages, a text prompt, the clipboard and the plugin log
- [x] Events: opened, saved, closed, activated (tab switched), changed (after typing pauses) and shutdown
- [x] Panels on the right or bottom described in JSON: headings, labels, buttons, inputs, checkboxes, dropdowns, rows, trees and sortable tables, with clicks and edits sent back to the plugin
- [x] Plugin shortcuts that clash with a built-in one or another plugin's are refused, with a line in the log
- [x] A permission prompt the first time a plugin runs and again when its files change, unless you tick "Don't ask when it changes"
- [x] Manage Plugins: enable or disable, start, stop, restart, open each plugin's folder, see why one failed
- [x] Plugin Log: what plugins print, their errors, and what Ettin Edit refused
- [x] A plugin that crashes or fails to start shows a notice and never takes the editor down; a plugin's undoable edits undo in one step
- [x] Sample plugins: Text Statistics (Python: counts and word frequencies in a panel, frequencies to a CSV tab) and Markdown Outline (JavaScript: a clickable heading tree, Insert UUID, Wrap Lines)

### 0.10: Ettin Edit rename and HL7 Viewer

- [x] Renamed to Ettin Edit everywhere: window, menus, docs, packages (`ettin-edit`), desktop entry and a new icon
- [x] Settings, session, backups, themes and plugins are copied from the old NixPad folder on first start; plugins that `import nixpad` keep working
- [x] Ettin Dark (based on Base16 Eighties) is the default dark theme, with JSON keys in a different colour from every kind of value
- [x] The built-in themes are `.tmTheme` files in the themes folder, so they can be edited; Theme > Reload Themes and Theme > Restore Built-in Themes
- [x] Show Whitespace draws each tab as an arrow across its full width; whitespace and end-of-line symbols are easier to see in light and dark themes
- [x] Fold markers are − and + in rounded boxes, with a line from where each region starts to where it ends
- [x] HL7 Viewer plugin (Plugins > Install Sample Plugins): HL7 v2 messages as a tree of segments, fields, repetitions, components and subcomponents; the tree follows the caret; an inspector shows each piece's designation (e.g. `ORC-4.1`), name, data type, optionality, repeatability, length and table from an HL7 v2.5.1 dictionary, and edits its value in place
- [x] Plugin API: a `caret` event, trees that select and reveal a node, and panels that scroll back to the top

### 1.0: Customisation, panels, languages and installers

#### Settings and customisation
- [x] Settings > Preferences (Ctrl+,): editor font and size, tab width, tabs or spaces, typing helpers, what's shown, new-document line ending, encoding and language, startup and file options, printing, update checks, plugin programs
- [x] Font picker listing the installed fonts (monospaced first, proportional on request); a CJK fallback still covers Chinese, Japanese and Korean
- [x] Shortcut Mapper (Settings menu): every command, saved macro and plugin command with its shortcuts; add, change or remove any of them, with conflicts shown; Reset All
- [x] Theme colour editor (Theme > Edit Theme Colours…): window and syntax colours with live preview, Save, Save As a new theme, Discard
- [x] Command palette (F1): every command, searchable by any words, with its shortcut

#### Panels and views
- [x] Function List (View menu): functions, classes, methods, headings, sections and keys for about 40 languages, nested, with a filter; follows the caret and jumps on click
- [x] Document Map (View menu): the whole document in miniature with the visible part marked; click or drag to scroll
- [x] Folder as Workspace (File > Open Folder as Workspace…): a file tree on the left that opens files with a click, follows the current file, refreshes by itself, shows hidden files on request, and has Find in Files Here and Open Terminal Here
- [x] Distraction-free mode (F11): full screen, no menus, tabs or panels, and the text in a centred column
- [x] Tab List (the ⏷ button at the end of the tabs, or View > Tab List…): search the open tabs by name or folder

#### Languages
- [x] 17 more grammars: TOML, INI (and cfg, conf, properties, systemd units, .editorconfig, .gitconfig), TypeScript, TSX, PowerShell, Dockerfile, HL7, Kotlin, Swift, Dart, Zig, CMake, Nginx, `.env`, Protobuf, GraphQL and Terraform/HCL
- [x] User-defined languages: short `.lang.json` files (keywords, comments, strings, operators, numbers, folding), or any `.sublime-syntax` grammar, in the languages folder; Language > New Language…, Open Languages Folder, Reload Languages and Language Guide ([Languages](docs/manual/src/languages.md))

#### XML Tools
- [x] Validate: against the XSD named by `xsi:schemaLocation` / `noNamespaceSchemaLocation`, the DOCTYPE's DTD, or the schema last used for that file; Validate Against… picks one. Problems are listed in the results panel and jump to their line
- [x] XSL Transform…: XSLT 1.0 into a new tab (HTML or XML output gets its language), with `xsl:message` texts shown

#### Files, printing and updates
- [x] File > Print… (Ctrl+P): opens the document in the browser's print preview, with syntax colours darkened for paper (a dark theme prints with Ettin Light's colours) and line numbers; both can be turned off in Preferences
- [x] File > Export as HTML…: the document as a web page in the current theme's colours
- [x] Help > Check for Updates…, and a weekly automatic check (Help menu or Preferences) that only speaks up when there's a newer release
- [x] Update Now downloads, checks and installs the new release, then restarts (1.7)
- [x] Saved macros keep their shortcuts when another macro is deleted

#### Installers
- [x] Windows installer (`setup.exe`): per-user without admin rights, or for everyone; Start menu, optional desktop shortcut, "Edit with Ettin Edit" in Explorer's right-click menu, "Open with" for 33 text and code file types, optional PATH entry, and an uninstaller
- [x] Debian / Ubuntu `.deb` package, next to the Arch package and the Linux `.tar.gz`

## Plugin system

Plugins are written in Python or JavaScript. See [Writing plugins](docs/manual/src/plugin-guide.md) for the
full API.

- [x] Runtime model: out-of-process plugins over JSON-RPC (one JSON message per line on standard input and output), so a crashing plugin can't take the editor down and any installed Python 3 or Node.js works
- [x] Plugin API: read/replace text and selection, open/save documents, show messages, add menu items and shortcuts, register commands, read the interface language
- [x] Plugin events: document opened/saved/closed, text changed, tab switched
- [x] Plugin panels (dock a custom view, e.g. a tree or a grid)
- [x] Plugin folder in the config directory, enable/disable per plugin
- [x] Plugin manager: browse, install, update, remove (Plugins > Manage Plugins: Installed and Available tabs from a plugin index, Install from File or Folder, updates when the index has a newer version, Remove to the trash; new and updated plugins ask for permission)
- [x] Sample plugins in both Python and JavaScript (Text Statistics, Markdown Outline, Markdown Preview, HL7 Viewer)
- [x] Permission prompt the first time a plugin runs and again when its files change (plugins are not sandboxed)

## Notepad++ plugin equivalents

Ettin Edit builds in the functionality of several popular Notepad++ plugins. These features
are original implementations, available from the Tools menu without installing anything.

### CsvQuery (SQL over CSV)
- [x] Parse the current document as CSV/TSV with automatic delimiter, quote and header detection
- [x] Manual override for delimiter, quote character, header row and column types
- [x] Load the data into an in-memory SQLite table named after the document
- [x] SQL query box with results in a sortable grid panel
- [x] Default query (`SELECT * FROM this`) on open
- [x] Copy results, or write results to a new tab as CSV
- [x] Query across several open CSV files (joins)
- [x] Column statistics (count, distinct, min, max, empty values)

### JSTool (JavaScript and JSON)
- [x] JS/JSON format (beautify), using the tab's indent setting
- [x] JS/JSON minify
- [x] JSON tree viewer panel with expand/collapse and search
- [x] Clicking a tree node jumps to it in the text; copy the node's path, key or value
- [x] JSON validation with error position
- [x] Sort JSON keys
- [x] Format a selection only

### MIME Tools (encoding and decoding)
- [x] Base64 encode / decode (selection or whole document), with optional line wrapping
- [x] Base64 URL-safe variant
- [x] URL (percent) encode / decode, full and component modes
- [x] Quoted-printable encode / decode
- [x] SAML decode (Base64 + inflate) and pretty-print the resulting XML
- [x] HTML entity encode / decode

### XML Tools
- [x] Pretty print XML (indented) and pretty print "attributes on new lines"
- [x] Linearize XML (remove formatting)
- [x] Check XML syntax (well-formedness) with error line and column
- [x] Automatic syntax check on save (optional)
- [x] Validate against XSD or DTD (with schema picker and remembered association) (1.0). Not supported: `xs:key`/`keyref`/`unique`, `xs:redefine`, and fetching schemas over the network
- [x] XPath expression evaluation with results list
- [x] Show the current XPath at the cursor in the status bar, copy it
- [x] XSL transformation of the current document into a new tab (XSLT 1.0) (1.0). Not supported: `xsl:import`, `xsl:include`, `document()`
- [x] Auto-close tags while typing
- [x] Escape / unescape XML special characters
- [ ] Comment / uncomment selection, convert text to XML (planned)
- [~] Namespace handling for XPath: prefixes declared in the document work, and unprefixed names also match the default namespace; custom prefix mappings are still missing

### ComparePlus (diff)
- [x] Compare two tabs side by side
- [x] Highlight added, removed, changed and moved lines, plus character-level changes inside lines
- [x] Next / previous / first / last difference navigation
- [x] Synchronised scrolling between the two sides
- [x] Compare against last saved version, clipboard, or Git HEAD
- [x] Options: ignore whitespace, ignore case, ignore empty lines, detect moves
- [x] Compare summary (counts of changes)
- [x] Compare a selection against another selection
- [x] Diff overview bar (where differences are in the file)
- [x] Export the diff as a unified patch

## Known bugs

Bugs reported but not yet in a release, with the release that fixes them.

| Bug | Fixed in |
| --- | --- |
| Word wrap on very large files: resizing the window re-measures every line, which takes a moment on files with hundreds of thousands of lines | 1.8 |
| Searching for `\r` or `\r\n` finds nothing with regular expressions, and line-break matches aren't highlighted in Extended mode | 1.6.1 |
| The `.deb` doesn't tell software centres its licence or repository, so they show "Unknown License" and "No Software Repository Included" and the software updater can't update it | 1.4 |

## Roadmap

Features that are planned or only partly done.

### Planned (not yet scheduled)
The six features requested on 2026-10-16 are in [1.5](#15-hex-view-character-panel-markdown-preview-and-more), and what was left of them is in [1.9](#19-markdown-preview-and-hex-view-finished). Nothing else is planned yet.

### XML
- [ ] Comment / uncomment selection, convert text to XML
- [~] XPath namespaces: prefixes declared in the document work, and unprefixed names also match the default namespace; custom prefix mappings are still missing
- [ ] XPath variables and the namespace axis

### Platform and distribution
- [x] Translations (1.2): English, German, French and Spanish, plus your own `.ftl` files
- [~] Packages: Arch package (0.2), Windows installer and `.deb` (1.0), APT repository and AppStream metadata (1.4); AppImage and Flatpak are not yet available
