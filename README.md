# NixPad

A modern, cross-platform text editor for Linux and Windows, inspired by Notepad++.
NixPad is an original, clean-room implementation written in Rust: it shares no code with Notepad++.

![NixPad dark theme](docs/screenshots/dark.png)

## Status

Milestone 1 is done: a tabbed editor with themes and syntax highlighting.
See [FEATURES.md](FEATURES.md) for what works today and what is still missing.

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
| `src/actions.rs` | Every command and its keyboard shortcut. Menus and shortcuts go through this list, and plugins will too |
| `src/document.rs` | Loading and saving files: encodings, BOM, line endings |
| `src/highlight.rs` | Syntax highlighting with syntect, cached per document |
| `src/theme.rs` | Built-in and custom themes, and the window colours derived from them |
| `src/settings.rs` | Settings, recent files and session, saved as JSON |
| `assets/themes/` | The NixPad Dark and Light colour schemes |

The GUI uses [egui](https://github.com/emilk/egui)/eframe, which draws everything itself
with OpenGL, so the look is identical on Linux and Windows.
