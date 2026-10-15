# Ettin Edit feature tracker

What Ettin Edit does today and what is still missing. Tick a box when a feature ships,
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
- [x] 9 built-in themes: Ettin Dark (the default, based on Base16 Eighties), Ettin Night, Ettin Light (original), Ocean Dark/Light, Mocha, Solarized Dark/Light, GitHub Light
- [x] Whole window (menus, tabs, status bar) follows the editor theme
- [x] Custom themes: drop any `.tmTheme` file into the themes folder (Theme > Open Themes Folder)
- [x] The built-in themes are files in the themes folder too, so they can be edited (Theme > Reload Themes, Theme > Restore Built-in Themes)
- [x] Theme, zoom, wrap and other view settings remembered between runs

### Platform
- [x] Linux (X11 and Wayland) and Windows builds from one codebase
- [x] Settings in `~/.config/EttinEdit` (Linux) or `%APPDATA%\EttinEdit` (Windows)

## Release 0.2: new editor engine (done)

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
- [x] Ctrl+click or Alt+click adds a caret, Ctrl+drag adds a selection (0.7); Esc goes back to one
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
- [x] Code folding by indentation: boxed −/+ markers in the gutter with a line showing where each region starts and ends, fold/unfold current (Ctrl+Alt+F / Ctrl+Alt+Shift+F), fold all (Alt+0), unfold all (Alt+Shift+0), fold level 1–5 (Alt+1…5)
- [x] Indent guides
- [x] Show whitespace (dots for spaces, an arrow across the full width of each tab) and end-of-line markers (LF, CRLF, CR)
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
- [x] Single window: opening a file while Ettin Edit runs sends it to the open window (turn off in Settings, or start with `--multi-instance`)
- [x] Drag tabs to reorder them
- [x] Pin tabs: pinned tabs stay at the front, show a pin, and are skipped by Close All and Close Others

### File commands (File menu and tab right-click menu)
- [x] Rename the file on disk
- [x] Move the file to the Recycle Bin / trash
- [x] Open Containing Folder (selects the file where the file manager supports it)
- [x] Open Terminal Here (Windows Terminal or cmd on Windows; `$TERMINAL`, Konsole, GNOME Terminal and others on Linux)
- [x] Summary dialog: path, dates, encoding, characters, words, lines, size on disk, selection

### Fixes
- [x] The mouse pointer is an arrow over the gutter (fold markers, bookmarks, line numbers), not a text cursor
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

## Release 0.6.1: right-click menu (done)

- [x] The editor's right-click menu has the whole Edit menu (Undo through Read-Only, with every submenu) and the Tools menu
- [x] Undo, Redo, Cut and Copy are greyed out when there is nothing to undo, redo or copy (in the Edit menu too)

## Release 0.7: split view and Compare (done)

### Split view (View menu and tab right-click menu)
- [x] Move a tab to the other view, or clone it so the same document shows in both views with separate carets and scrolling
- [x] Each view has its own tab bar and active tab; F8 switches views; drag the divider to resize
- [x] Closing a clone only closes that view's copy; the split ends when one view is empty
- [x] The split layout is saved with the session

### Compare (Tools > Compare, and the right-click menu)
- [x] Opens as a Compare tab: both sides aligned row by row, read-only, and updated as you edit the compared tabs
- [x] Double-click a line to jump to it in its tab for editing
- [x] Toolbar: first, previous, next and last difference, the change summary, Swap Sides, Export Patch, and the options
- [x] Shortcuts: Ctrl+Alt+D compares the two views (or this tab with the one used before it), F7 / Shift+F7 next and previous difference
- [x] Compare With any open tab, the last save, the clipboard, the last Git commit, or two selections (Ctrl+drag a second selection, or select in each view)
- [x] Moved blocks are coloured apart from added and removed lines
- [x] Very large or very different files stay responsive (the diff has a time limit and the view only draws the visible rows)

## Release 0.8: CsvQuery (done)

Tools > CsvQuery, or Ctrl+Alt+Q, opens a panel under the editor. The plugin
list further down is ticked off too; this section covers what sits around it.

- [x] The current tab is the table `this`; every other tab is a table named after its file (`orders 2024.csv` is `orders_2024`), loaded when a query names it
- [x] Delimiters: comma, semicolon, tab, pipe and space; decimal commas (`3,50`) count as numbers in semicolon files
- [x] Values with leading zeros (zip codes, ids) stay text; empty fields are NULL
- [x] Queries run in the background and re-run as you edit the tab or switch tabs; Ctrl+Enter runs the SQL box
- [x] Results: click a column header to sort, right-click a cell to copy it or its row, first 100,000 rows shown
- [x] Columns view: each column's type (changeable), filled, empty and distinct counts, min, max and average
- [x] Recent queries menu, and a Tables menu that inserts table names
- [x] UPDATE, DELETE and CREATE work on the query copy; the file itself never changes

## Release 0.9: plugin system (done)

Plugins are folders in the plugins folder (`~/.config/EttinEdit/plugins/` on Linux,
`%APPDATA%\EttinEdit\plugins\` on Windows) with a `plugin.json` and a Python or
JavaScript script. See [docs/plugins.md](docs/plugins.md), also under Plugins > Plugin Guide.

- [x] Plugins menu: each running plugin's commands with their shortcuts, Manage Plugins, Plugin Log, Open Plugins Folder, Reload Plugins, Install Sample Plugins and Plugin Guide; the right-click menu has the plugin commands too
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

## Release 0.10: Ettin Edit (done)

- [x] Renamed to Ettin Edit everywhere: window, menus, docs, packages (`ettin-edit`), desktop entry and a new icon
- [x] Settings, session, backups, themes and plugins are copied from the old NixPad folder on first start; plugins that `import nixpad` keep working
- [x] Ettin Dark (based on Base16 Eighties) is the default dark theme, with JSON keys in a different colour from every kind of value
- [x] The built-in themes are `.tmTheme` files in the themes folder, so they can be edited; Theme > Reload Themes and Theme > Restore Built-in Themes
- [x] Show Whitespace draws each tab as an arrow across its full width; whitespace and end-of-line symbols are easier to see in light and dark themes
- [x] Fold markers are − and + in rounded boxes, with a line from where each region starts to where it ends
- [x] HL7 Viewer plugin (Plugins > Install Sample Plugins): HL7 v2 messages as a tree of segments, fields, repetitions, components and subcomponents; the tree follows the caret; an inspector shows each piece's designation (e.g. `ORC-4.1`), name, data type, optionality, repeatability, length and table from an HL7 v2.5.1 dictionary, and edits its value in place
- [x] Plugin API: a `caret` event, trees that select and reveal a node, and panels that scroll back to the top

## Release 1.0: customisation, panels, languages and installers (done)

### Settings and customisation
- [x] Settings > Preferences (Ctrl+,): editor font and size, tab width, tabs or spaces, typing helpers, what's shown, new-document line ending, encoding and language, startup and file options, printing, update checks, plugin programs
- [x] Font picker listing the installed fonts (monospaced first, proportional on request); a CJK fallback still covers Chinese, Japanese and Korean
- [x] Shortcut Mapper (Settings menu): every command, saved macro and plugin command with its shortcuts; add, change or remove any of them, with conflicts shown; Reset All
- [x] Theme colour editor (Theme > Edit Theme Colours…): window and syntax colours with live preview, Save, Save As a new theme, Discard
- [x] Command palette (F1): every command, searchable by any words, with its shortcut

### Panels and views
- [x] Function List (View menu): functions, classes, methods, headings, sections and keys for about 40 languages, nested, with a filter; follows the caret and jumps on click
- [x] Document Map (View menu): the whole document in miniature with the visible part marked; click or drag to scroll
- [x] Folder as Workspace (File > Open Folder as Workspace…): a file tree on the left that opens files with a click, follows the current file, refreshes by itself, shows hidden files on request, and has Find in Files Here and Open Terminal Here
- [x] Distraction-free mode (F11): full screen, no menus, tabs or panels, and the text in a centred column
- [x] Tab List (the ⏷ button at the end of the tabs, or View > Tab List…): search the open tabs by name or folder

### Languages
- [x] 17 more grammars: TOML, INI (and cfg, conf, properties, systemd units, .editorconfig, .gitconfig), TypeScript, TSX, PowerShell, Dockerfile, HL7, Kotlin, Swift, Dart, Zig, CMake, Nginx, `.env`, Protobuf, GraphQL and Terraform/HCL
- [x] User-defined languages: short `.lang.json` files (keywords, comments, strings, operators, numbers, folding), or any `.sublime-syntax` grammar, in the languages folder; Language > New Language…, Open Languages Folder, Reload Languages and Language Guide ([docs/languages.md](docs/languages.md))

### XML Tools
- [x] Validate: against the XSD named by `xsi:schemaLocation` / `noNamespaceSchemaLocation`, the DOCTYPE's DTD, or the schema last used for that file; Validate Against… picks one. Problems are listed in the results panel and jump to their line
- [x] XSL Transform…: XSLT 1.0 into a new tab (HTML or XML output gets its language), with `xsl:message` texts shown

### Files, printing and updates
- [x] File > Print… (Ctrl+P): opens the document in the browser's print preview, with syntax colours darkened for paper (a dark theme prints with Ettin Light's colours) and line numbers; both can be turned off in Preferences
- [x] File > Export as HTML…: the document as a web page in the current theme's colours
- [x] Help > Check for Updates…, and a weekly automatic check (Help menu or Preferences) that only speaks up when there's a newer release
- [x] Saved macros keep their shortcuts when another macro is deleted

### Installers
- [x] Windows installer (`setup.exe`): per-user without admin rights, or for everyone; Start menu, optional desktop shortcut, "Edit with Ettin Edit" in Explorer's right-click menu, "Open with" for 33 text and code file types, optional PATH entry, and an uninstaller
- [x] Debian / Ubuntu `.deb` package, next to the Arch package and the Linux `.tar.gz`

## Core editor features still missing

### Editing
- [ ] Function parameter hints and language keyword completion
- [ ] Syntax-aware code folding (by grammar instead of indentation)

### View
- [x] Split view (two documents side by side) and clone the same document into both views (0.7)
- [x] Document map (minimap) (1.0)
- [x] Function list panel (1.0)
- [x] Folder as workspace (file tree side panel) (1.0)
- [x] Distraction-free / full-screen mode (1.0)
- [x] Tabs list dropdown (1.0)

### Files and encodings
- [x] Print and print preview, through the browser (1.0)
- [x] Very large file mode (incremental highlighting so 2 MB+ files keep colours), done in 0.2

### Settings and customisation
- [x] Preferences dialog (fonts, tab width, spaces vs tabs, default encoding/EOL, new-document defaults) (1.0)
- [x] Font family picker (1.0)
- [x] Shortcut mapper (rebind any command) (1.0)
- [x] Style configurator (edit theme colours in the app) (1.0)
- [x] User-defined languages (custom keywords, comments, operators): `.lang.json` files in the `languages` folder; see [docs/languages.md](docs/languages.md)
- [x] More grammars (TOML, INI, TypeScript, PowerShell, Dockerfile, etc.), and loading `.sublime-syntax` files from the user folder: also TSX, HL7, Kotlin, Swift, Dart, Zig, CMake, Nginx, `.env`, Protobuf, GraphQL and Terraform/HCL; user grammars replace built-ins of the same name
- [ ] Localisation / translations

### Platform and distribution
- [x] Application icon and desktop integration: icon and Linux `.desktop` file in 0.2; Windows "Open with", right-click menu and Start menu entries from the installer in 1.0
- [x] App icon in the top left corner of the window (click it for About), redrawn from the Edda and Ottar mascot and used for the window, taskbar, Windows `.exe` and installer, and Linux icon theme sizes
- [~] Installers / packages: Arch package (0.2), Windows installer and `.deb` (1.0); AppImage and Flatpak not yet
- [x] CI builds for Linux and Windows, done in 0.2
- [x] Update check, by hand or weekly (1.0)

## Plugin system (Python and JavaScript)
- [x] Runtime model: out-of-process plugins over JSON-RPC (one JSON message per line on standard input and output), so a crashing plugin can't take the editor down and any installed Python 3 or Node.js works
- [x] Plugin API: read/replace text and selection, open/save documents, show messages, add menu items and shortcuts, register commands (the editor's command list in `src/actions.rs` is built for this)
- [x] Plugin events: document opened/saved/closed, text changed, tab switched
- [x] Plugin panels (dock a custom view, e.g. a tree or a grid)
- [x] Plugin folder in the config directory, enable/disable per plugin
- [x] Plugin manager: browse, install, update, remove (Plugins > Manage Plugins: Installed and Available tabs from a plugin index, Install from File or Folder, updates when the index has a newer version, Remove to the trash; new and updated plugins ask for permission)
- [x] Sample plugins in both Python and JavaScript
- [x] Sandboxing / permission prompts for plugins (a permission prompt the first time a plugin runs and again when its files change; plugins are not sandboxed)

## Features from Joe's favourite Notepad++ plugins

These can ship built in or as first-party plugins once the plugin system exists.

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
- [ ] Comment / uncomment selection, convert text to XML
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
