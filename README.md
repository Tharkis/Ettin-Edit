# NixPad

A modern, cross-platform text editor for Linux and Windows, inspired by Notepad++.
NixPad is an original, clean-room implementation written in Rust: it shares no code with Notepad++.

![NixPad dark theme](docs/screenshots/dark.png)

## Status

Release 0.9 adds plugins in Python or JavaScript. A plugin can read and change text,
open and save files, add menu commands and shortcuts, react to events and show panels
with trees, tables and forms. Plugins > Install Sample Plugins adds two to try, and
Plugins > Plugin Guide explains how to write your own ([docs/plugins.md](docs/plugins.md)).
Release 0.8 added CsvQuery: SQL over the CSV or TSV file you have open, with joins
across open tabs, a sortable results grid, column statistics, and results to a new tab.
Release 0.7 added split view (two tabs side by side, or one document in both views)
and Compare, a built-in take on ComparePlus: compare two tabs, the last save, the
clipboard, the last Git commit or two selections, with word-level highlights, moved
blocks, an overview bar and patch export. Release 0.6 added the Tools menu, with built-in versions of three favourite Notepad++
plugins. JSON and JavaScript tools format, minify, validate and sort, with a live JSON
tree. MIME tools cover Base64, URL, quoted-printable, HTML entities and SAML. XML tools
pretty print, check syntax (optionally on save), run XPath queries, show the current
XPath on the status bar, and close tags as you type. Release 0.6.1 put the Edit and Tools menus in the editor's right-click menu. It all builds on 0.5's encodings,
backups and file commands, 0.4's editing tools, 0.3's search and 0.2's editor engine.
See [FEATURES.md](FEATURES.md) for what works today and what is still missing.

## Installing on Arch / CachyOS

From a checkout of this repository:

```sh
cd packaging/arch
makepkg -si
```

This builds a release binary and installs it with its icon and menu entry.

## Building

Install Rust (https://rustup.rs), then:

```sh
cargo run --release -- path/to/file.txt   # run
cargo build --release                     # binary in target/release/
cargo test                                # unit tests
```

**Linux** needs the usual desktop libraries at run time (OpenGL, `libxkbcommon-x11` on X11).
On Debian/Ubuntu: `sudo apt install libxkbcommon-x11-0 libgl1`.

**Windows** builds with the normal MSVC toolchain (`cargo build --release`), or can be
cross-compiled from Linux with `rustup target add x86_64-pc-windows-gnu` and the MinGW linker
(`cargo build --release --target x86_64-pc-windows-gnu`).

## Themes

NixPad ships with nine themes (NixPad Dark and Light are its own; the rest are classic
schemes bundled with syntect). Any TextMate `.tmTheme` file placed in the themes folder
appears under **Theme > Custom** on the next start:

- Linux: `~/.config/NixPad/themes/`
- Windows: `%APPDATA%\NixPad\themes\`

## Code layout

| File | What it does |
| --- | --- |
| `src/main.rs` | Entry point: window setup, files from the command line |
| `src/app.rs` | The window: menus, tabs, editor, status bar, dialogs |
| `src/actions.rs` | Every command and its keyboard shortcut. Menus, shortcuts and plugins go through this list |
| `src/document.rs` | Loading, saving and reloading files: encodings, line endings, changes on disk |
| `src/encoding.rs` | Encoding detection, decoding and encoding (UTF-8, UTF-16, legacy code pages) |
| `src/session.rs` | The session and backups of unsaved changes |
| `src/instance.rs` | Single-window mode: hands files to the NixPad already running |
| `src/system.rs` | Opening the file manager and a terminal, and reading the last Git commit |
| `src/file_ui.rs` | Rename, move to trash and summary dialogs, the changed-on-disk bar, the Encoding menu |
| `src/editor/` | The editor engine: `buffer.rs` (rope text and undo), `selection.rs` (carets), `layout.rs` (columns, word wrap, row index), `highlight.rs` (incremental highlighting), `find.rs` (find, replace, marks, bookmarks), `lines.rs` (line operations, case, comments), `smart.rs` (auto-close, smart indent, completion, brace matching, column editing), `fold.rs` (code folding), `view.rs` (drawing, keyboard and mouse) |
| `src/search.rs` | Search queries (plain, whole word, regex), replacement templates, and Find in Files |
| `src/search_ui.rs` | The find bar, results panel, and the Go to Line and Find in Files dialogs |
| `src/tools_ui.rs` | The Column Editor and macro dialogs |
| `src/tools/` | The Tools menu's text tools: `mime.rs` (Base64, URL, quoted-printable, HTML, SAML), `json.rs` (JSON parser, formatter and tree), `js.rs` (JavaScript formatter and minifier), `xml.rs` (pretty print, linearize, syntax check, XPath at the caret), `xpath.rs` (XPath 1.0), `csv.rs` (reading delimited text), `sql.rs` (CsvQuery's SQLite tables and queries) |
| `src/csv_query.rs`, `src/app/csv_query.rs` | The CsvQuery panel, its background query thread, and which tabs are tables |
| `src/text_tools_ui.rs` | The JSON tree and XPath panels, and the notices tools show |
| `src/menu.rs`, `src/app/menus.rs` | Menus as data, so the Edit and Tools menus are drawn in both the menu bar and the right-click menu |
| `src/app/text_tools.rs` | Runs the text tools on the current document |
| `src/app/views.rs` | Split view: which tabs are in which view, moving and cloning tabs, the divider |
| `src/compare.rs` | The compare engine: line alignment, options, moved lines, word changes, unified patches |
| `src/compare_ui.rs`, `src/app/compare.rs` | The Compare tab, and starting and refreshing comparisons |
| `src/plugins/` | The plugin host: `mod.rs` (starting plugins, messages, commands, events), `manifest.rs` (`plugin.json`, finding plugins, the permission fingerprint), `process.rs` (plugin processes), `shortcut.rs` (shortcut text), `panel.rs` (JSON panels), `ui.rs` (Manage Plugins, the log and the permission prompt) |
| `src/app/plugins.rs` | Answers plugin calls on the open documents, sends events, draws plugin panels |
| `plugins/sdk/` | The Python and JavaScript SDKs that plugins import |
| `plugins/samples/` | The sample plugins (Text Statistics in Python, Markdown Outline in JavaScript) |
| `docs/plugins.md` | The plugin guide |
| `src/macros.rs` | Macro recording and playback steps |
| `src/highlight.rs` | Language list and detection (syntect grammars) |
| `src/fonts.rs` | Fallback fonts for Chinese, Japanese and Korean text |
| `src/theme.rs` | Built-in and custom themes, and the window colours derived from them |
| `src/settings.rs` | Settings, recent files and session, saved as JSON |
| `assets/themes/` | The NixPad Dark and Light colour schemes |
| `assets/icons/` | The app icon (SVG, PNG, ICO) |
| `packaging/` | Linux `.desktop` file and the Arch `PKGBUILD` |

The GUI uses [egui](https://github.com/emilk/egui)/eframe, which draws everything itself
with OpenGL, so the look is identical on Linux and Windows.
