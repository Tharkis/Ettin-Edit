# NixPad feature tracker

What NixPad does today and what is still missing. Tick a box when a feature ships,
and add new rows as ideas come up.

Legend: `[x]` done · `[ ]` not started · `[~]` partly done (see note)

## Milestone 1: tabbed editor with themes and syntax highlighting (done)

### Tabs and files
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

### Editing and view
- [x] Line numbers gutter (stays aligned with word wrap on), toggle in View menu
- [x] Current line highlight, toggle in View menu
- [x] Word wrap (Alt+Z)
- [x] Zoom in/out/reset (Ctrl+= / Ctrl+- / Ctrl+0)
- [x] Undo/redo, clipboard, select all
- [x] Status bar: language, length, line count, Ln/Col, selection length, line ending, encoding, zoom

### Syntax highlighting
- [x] Highlighting for about 65 languages (Rust, Python, JavaScript, JSON, XML, HTML, CSS, C/C++, C#, Java, Go, PHP, SQL, YAML, Markdown, shell, and more)
- [x] Language detected from the file name, then from the first line (shebang, `<?xml`)
- [x] Language menu grouped A–Z, plus a quick picker on the status bar

### Themes
- [x] Dark mode by default, with light themes available
- [x] 9 built-in themes: NixPad Dark, NixPad Light (original), Ocean Dark/Light, Eighties, Mocha, Solarized Dark/Light, GitHub Light
- [x] Whole window (menus, tabs, status bar) follows the editor theme
- [x] Custom themes: drop any `.tmTheme` file into the themes folder (Theme > Open Themes Folder)
- [x] Theme, zoom, wrap and other view settings remembered between runs

### Platform
- [x] Linux (X11 and Wayland) and Windows builds from one codebase
- [x] Settings in `~/.config/NixPad` (Linux) or `%APPDATA%\NixPad` (Windows)

## Release 0.2: new editor engine (done)

- [x] NixPad's own editor engine on a rope buffer (replaces egui's text box)
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
- [~] Word wrap on huge files: resizing the window re-measures every line, which takes a moment on files with hundreds of thousands of lines

## Release 0.3: search and navigation (done)

### Find and replace
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

### Navigation
- [x] Go to Line (Ctrl+G, or click Ln/Col in the status bar); accepts `line` or `line:column`
- [x] Bookmarks: toggle with Ctrl+F2 or a click in the gutter, F2 / Shift+F2 to jump, Clear All
- [x] Cut, copy or remove bookmarked lines, remove unbookmarked lines, inverse bookmarks

## Release 0.4: editing power tools (done)

### Multiple cursors and column editing
- [x] Select next occurrence (Ctrl+Shift+D) and select all occurrences (Alt+F3) for multi-caret edits
- [x] Ctrl+click or Alt+click adds a caret; Esc goes back to one
- [x] Column (box) selection with Alt+drag or Alt+Shift+arrows; typing and deleting work on every row
- [x] Column Editor (Alt+C): insert text or a number sequence (start, step, repeat, leading zeros, dec/hex/oct/bin) down a column

### Line operations (Edit > Line Operations)
- [x] Duplicate (Ctrl+D), delete (Ctrl+Shift+L), move up/down (Ctrl+Shift+Up/Down)
- [x] Join (Ctrl+J), split at the long-line column (Ctrl+I), insert blank line above or below
- [x] Sort ascending or descending: lexically, ignoring case, as integers, as decimals, by length; reverse order
- [x] Remove duplicate lines, consecutive duplicates, empty lines, or blank lines
- [x] Trim trailing, leading or both; end-of-line to space; tabs to spaces; spaces to tabs (leading or all)

### Text tools
- [x] Case conversion: UPPER (Ctrl+Shift+U), lower (Ctrl+U), Title (Alt+U), Sentence (Ctrl+Alt+U), iNVERT (Ctrl+Alt+Shift+U)
- [x] Toggle line comment (Ctrl+Q) and block comment (Ctrl+Shift+Q), with the right markers for each language
- [x] Insert date and time, short (F5) or long (Shift+F5)
- [x] Read-only mode (Edit menu, or click "Read-only" in the status bar)

### Typing helpers (each can be turned off in the Settings menu)
- [x] Auto-close brackets and quotes; typing a closer steps over it; Backspace removes an empty pair; selected text gets wrapped
- [x] Smart indent: Enter indents after `{`, `(`, `[` (and after `:` in Python, YAML and Nim) and splits `{}` onto separate lines; a closing bracket outdents
- [x] Word completion from words in the document, as you type or with Ctrl+Space
- [x] Insert spaces instead of tabs (optional)
- [x] Matching bracket highlight (red when unmatched) and matching HTML/XML tag highlight

### View
- [x] Code folding by indentation: gutter arrows, fold/unfold current (Ctrl+Alt+F / Ctrl+Alt+Shift+F), fold all (Alt+0), unfold all (Alt+Shift+0), fold level 1–5 (Alt+1…5)
- [x] Indent guides
- [x] Show whitespace (spaces and tabs) and end-of-line markers (LF, CRLF, CR)
- [x] Long-line edge marker at a configurable column (default 80)

### Macros
- [x] Record (Ctrl+Shift+R) and play back (Ctrl+Shift+P) typing, keys, clipboard and edit commands
- [x] Run a macro N times or until the end of the file
- [x] Save macros by name; saved macros appear in the Macro menu and survive restarts

## Release 0.5: files, encodings and safety (done)

### Encodings
- [x] Encoding detected on open: byte-order marks, UTF-8, UTF-16 without a BOM, then a guess for legacy encodings (Windows-1252, Windows-1251, Shift_JIS and others)
- [x] Encoding menu (also on the status bar): save as UTF-8, UTF-8 BOM, UTF-16 LE/BE, or about 35 other encodings grouped by region (Windows code pages, ISO-8859-x, KOI8, Mac Roman, GBK, Big5, Shift_JIS, EUC-KR...)
- [x] Reopen With: read the file again in an encoding you pick, when the guess is wrong
- [x] Saving refuses a character the chosen encoding can't hold, and says which one, instead of losing it

### Unsaved work and files changed on disk
- [x] Unsaved changes, including new untitled tabs, are backed up every few seconds and come back after a crash
- [x] Quitting keeps unsaved changes for next time without asking (Settings > Remember Unsaved Changes on Exit; turn it off to be asked instead)
- [x] Session remembers each tab's encoding, line ending, language, caret, pin and read-only state
- [x] A bar above the editor when another program changes or deletes the open file: Reload (undoable) or Keep My Version
- [x] Optional automatic reload of changed files that have no unsaved changes (Settings menu)
- [x] File > Reload from Disk

### Window and tabs
- [x] Single window: opening a file while NixPad runs sends it to the open window (turn off in Settings, or start with `--multi-instance`)
- [x] Drag tabs to reorder them
- [x] Pin tabs: pinned tabs stay at the front, show a pin, and are skipped by Close All and Close Others

### File commands (File menu and tab right-click menu)
- [x] Rename the file on disk
- [x] Move the file to the Recycle Bin / trash
- [x] Open Containing Folder (selects the file where the file manager supports it)
- [x] Open Terminal Here (Windows Terminal or cmd on Windows; `$TERMINAL`, Konsole, GNOME Terminal and others on Linux)
- [x] Summary dialog: path, dates, encoding, characters, words, lines, size on disk, selection

### Fixes
- [x] The mouse pointer is an arrow over the gutter (fold arrows, bookmarks, line numbers), not a text cursor
- [x] Theme > Open Themes Folder works on Windows (and on Linux it uses the desktop's file manager)

## Release 0.6: JSON, JavaScript, MIME and XML tools (done)

Everything is in the new Tools menu. The plugin-feature lists further down are
ticked off too; this section covers what sits around them.

- [x] Tools work on each selection (multi-cursor too) or on the whole document, as one undo step
- [x] Errors show as a notice with line and column and a Go To button; the caret moves to the error
- [x] JSON Format and Minify fall back to JavaScript when the text isn't strict JSON, and say so
- [x] JavaScript formatter handles template strings, regex literals, comments, `switch`/`case` and modern syntax
- [x] JSON tree (Ctrl+Alt+J): updates as you type, opens at the caret, has a filter box, and right-click copies a path like `$.items[2].name`
- [x] XPath panel (Ctrl+Alt+Shift+E): full XPath 1.0 with every core function plus `ends-with`, `lower-case` and `upper-case`; node results jump to the text, other results show the value; recent expressions
- [x] Current XPath on the status bar (click to copy), kept while you are still typing unclosed tags
- [x] Auto-close tags in XML and HTML (skips HTML void elements like `<br>`); Tools > XML Tools > Close Tags Automatically
- [x] Shortcuts: Ctrl+Alt+M format JSON/JS, Ctrl+Alt+N minify, Ctrl+Alt+Shift+B pretty print XML, Ctrl+Alt+Shift+L linearize, Ctrl+Alt+Shift+M check XML, Ctrl+Alt+Shift+P copy XPath
- [ ] XPath variables and the namespace axis (rarely used; not planned yet)

## Core editor features still missing

### Editing
- [ ] Function parameter hints and language keyword completion
- [ ] Syntax-aware code folding (by grammar instead of indentation)

### View
- [ ] Split view (two documents side by side) and clone the same document into both views
- [ ] Document map (minimap)
- [ ] Function list panel
- [ ] Folder as workspace (file tree side panel)
- [ ] Distraction-free / full-screen mode
- [ ] Tabs list dropdown

### Files and encodings
- [ ] Print and print preview
- [x] Very large file mode (incremental highlighting so 2 MB+ files keep colours), done in 0.2

### Settings and customisation
- [ ] Preferences dialog (fonts, tab width, spaces vs tabs, default encoding/EOL, new-document defaults)
- [ ] Font family picker
- [ ] Shortcut mapper (rebind any command)
- [ ] Style configurator (edit theme colours in the app)
- [ ] User-defined languages (custom keywords, comments, operators)
- [ ] More grammars (TOML, INI, TypeScript, PowerShell, Dockerfile, etc.), and loading `.sublime-syntax` files from the user folder
- [ ] Localisation / translations

### Platform and distribution
- [~] Application icon and desktop integration: icon and Linux `.desktop` file done in 0.2; Windows file associations still to come
- [~] Installers / packages: Arch `PKGBUILD` done in 0.2; Windows installer, AppImage / Flatpak / .deb still to come
- [x] CI builds for Linux and Windows, done in 0.2
- [ ] Auto-update check

## Plugin system (Python and JavaScript)
- [ ] Decide the runtime model: embedded interpreters (e.g. pyo3 for Python, a JS engine such as QuickJS/Boa) or out-of-process plugins over JSON-RPC
- [ ] Plugin API: read/replace text and selection, open/save documents, show messages, add menu items and shortcuts, register commands (the editor's command list in `src/actions.rs` is built for this)
- [ ] Plugin events: document opened/saved/closed, text changed, tab switched
- [ ] Plugin panels (dock a custom view, e.g. a tree or a grid)
- [ ] Plugin folder in the config directory, enable/disable per plugin
- [ ] Plugin manager: browse, install, update, remove
- [ ] Sample plugins in both Python and JavaScript
- [ ] Sandboxing / permission prompts for plugins

## Features from Joe's favourite Notepad++ plugins

These can ship built in or as first-party plugins once the plugin system exists.

### CsvQuery (SQL over CSV)
- [ ] Parse the current document as CSV/TSV with automatic delimiter, quote and header detection
- [ ] Manual override for delimiter, quote character, header row and column types
- [ ] Load the data into an in-memory SQLite table named after the document
- [ ] SQL query box with results in a sortable grid panel
- [ ] Default query (`SELECT * FROM this`) on open
- [ ] Copy results, or write results to a new tab as CSV
- [ ] Query across several open CSV files (joins)
- [ ] Column statistics (count, distinct, min, max, empty values)

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
- [ ] Validate against XSD or DTD (with schema picker and remembered association), planned for 0.10
- [x] XPath expression evaluation with results list
- [x] Show the current XPath at the cursor in the status bar, copy it
- [ ] XSL transformation of the current document into a new tab, planned for 0.10
- [x] Auto-close tags while typing
- [x] Escape / unescape XML special characters
- [ ] Comment / uncomment selection, convert text to XML
- [~] Namespace handling for XPath: prefixes declared in the document work, and unprefixed names also match the default namespace; custom prefix mappings are still missing

### ComparePlus (diff)
- [ ] Compare two tabs side by side
- [ ] Highlight added, removed, changed and moved lines, plus character-level changes inside lines
- [ ] Next / previous / first / last difference navigation
- [ ] Synchronised scrolling between the two sides
- [ ] Compare against last saved version, clipboard, or Git HEAD
- [ ] Options: ignore whitespace, ignore case, ignore empty lines, detect moves
- [ ] Compare summary (counts of changes)
- [ ] Compare a selection against another selection
- [ ] Diff overview bar (where differences are in the file)
- [ ] Export the diff as a unified patch
