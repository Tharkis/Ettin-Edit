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
- [x] UTF-8 with and without BOM, preserved on save; warning for files that aren't valid UTF-8

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

## Core editor features still missing

### Search
- [ ] Find / Replace bar (match case, whole word, wrap around)
- [ ] Regular expression search and replace, with capture groups
- [ ] Find in all open documents
- [ ] Find in Files (folder, filters, results panel)
- [ ] Mark all occurrences / highlight other occurrences of the selected word
- [ ] Incremental search
- [ ] Go to line (Ctrl+G)

### Editing
- [~] Multiple cursors and multi-selection (basics in 0.2; select next occurrence still to come)
- [ ] Column (block) selection and column editing
- [~] Auto-indent and smart indentation per language (keeps the previous line's indent; per-language rules still to come)
- [ ] Brace and tag matching / highlighting
- [ ] Auto-close brackets and quotes
- [ ] Toggle line and block comments per language
- [ ] Line operations: duplicate, delete, move up/down, join, split
- [ ] Sort lines (ascending/descending, case-insensitive, numeric), remove duplicate lines, remove empty lines
- [ ] Trim trailing / leading whitespace, tabs to spaces and back
- [ ] Case conversion (UPPER, lower, Title, Sentence, iNVERT)
- [ ] Word and keyword auto-completion, function parameter hints
- [ ] Macro recording and playback, saved macros
- [ ] Insert date/time
- [ ] Read-only mode
- [ ] Code folding
- [ ] Bookmarks (toggle, next/prev, copy/cut/remove bookmarked lines)

### View
- [ ] Show whitespace and end-of-line symbols
- [ ] Indent guides
- [ ] Long-line edge marker
- [ ] Split view (two documents side by side) and clone the same document into both views
- [ ] Document map (minimap)
- [ ] Function list panel
- [ ] Folder as workspace (file tree side panel)
- [ ] Distraction-free / full-screen mode
- [ ] Tab drag to reorder, pin tabs, tabs list dropdown
- [ ] Summary dialog (words, characters, lines, selection)

### Files and encodings
- [ ] Encoding conversion: ANSI/Windows-1252, ISO-8859-x, UTF-16 LE/BE, and others (open and save as)
- [ ] Encoding auto-detection beyond UTF-8
- [ ] Detect files changed on disk and offer to reload
- [ ] Periodic backup and recovery of unsaved work (including untitled tabs)
- [ ] Restore unsaved untitled tabs on restart
- [ ] Rename / delete current file, open containing folder, open terminal here
- [ ] Print and print preview
- [x] Very large file mode (incremental highlighting so 2 MB+ files keep colours), done in 0.2
- [ ] Reload from disk

### Settings and customisation
- [ ] Preferences dialog (fonts, tab width, spaces vs tabs, default encoding/EOL, new-document defaults)
- [ ] Font family picker
- [ ] Shortcut mapper (rebind any command)
- [ ] Style configurator (edit theme colours in the app)
- [ ] User-defined languages (custom keywords, comments, operators)
- [ ] More grammars (TOML, INI, TypeScript, PowerShell, Dockerfile, etc.), and loading `.sublime-syntax` files from the user folder
- [ ] Localisation / translations
- [ ] Single-instance mode (opening a file sends it to the running window)

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
- [ ] JS/JSON format (beautify) with configurable indent
- [ ] JS/JSON minify
- [ ] JSON tree viewer panel with expand/collapse and search
- [ ] Clicking a tree node jumps to it in the text; copy the node's path, key or value
- [ ] JSON validation with error position
- [ ] Sort JSON keys
- [ ] Format a selection only

### MIME Tools (encoding and decoding)
- [ ] Base64 encode / decode (selection or whole document), with optional line wrapping and padding
- [ ] Base64 URL-safe variant
- [ ] URL (percent) encode / decode, full and component modes
- [ ] Quoted-printable encode / decode
- [ ] SAML decode (Base64 + inflate) and pretty-print the resulting XML
- [ ] HTML entity encode / decode

### XML Tools
- [ ] Pretty print XML (indented) and pretty print "attributes on new lines"
- [ ] Linearize XML (remove formatting)
- [ ] Check XML syntax (well-formedness) with error line and column
- [ ] Automatic syntax check on save (optional)
- [ ] Validate against XSD or DTD (with schema picker and remembered association)
- [ ] XPath expression evaluation with results list
- [ ] Show the current XPath at the cursor in the status bar, copy it
- [ ] XSL transformation of the current document into a new tab
- [ ] Auto-close tags while typing
- [ ] Escape / unescape XML special characters
- [ ] Comment / uncomment selection, convert text to XML
- [ ] Namespace handling for XPath (prefix mapping)

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
