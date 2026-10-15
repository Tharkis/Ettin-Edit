# Installing and updating

Ettin Edit runs on 64-bit Windows and 64-bit Linux (X11 or Wayland). Every release is on the
[releases page](https://github.com/Tharkis/Ettin-Edit/releases/latest).

## Windows

Download one of these:

- **`ettin-edit-<version>-setup.exe`**, the installer. Run it and pick:
  - **Install for me only**, which needs no administrator rights, or
    **Install for all users**.
  - A desktop shortcut (optional).
  - **Edit with Ettin Edit** in the right-click menu of every file in Explorer.
  - **Open with** entries for 33 common text and code file types.
  - Adding Ettin Edit to the `PATH`, so `ettin-edit file.txt` works in a
    terminal (optional).

  Ettin Edit then appears in the Start menu. Remove it from **Settings > Apps**
  like any other program.
- **The `.zip`**, if you'd rather not install anything. Extract it anywhere
  and run `ettin-edit.exe`. Your settings still go to
  `%APPDATA%\EttinEdit` (see [Files, folders and the command
  line](folders.md)).

## Linux

| Your system | Download | Install with |
| --- | --- | --- |
| Arch, CachyOS, Manjaro, EndeavourOS | `.pkg.tar.zst` | `sudo pacman -U ./ettin-edit-*.pkg.tar.zst` |
| Debian, Ubuntu, Mint, Pop!_OS | `.deb` | `sudo apt install ./ettin-edit_*_amd64.deb` |
| Anything else | `.tar.gz` | Extract it and run `./ettin-edit` |

The packages add Ettin Edit to your application menu with its icon, and offer
it in **Open With** for text files.

The `.tar.gz` needs the usual desktop libraries, which almost every desktop
already has: OpenGL and, on X11, `libxkbcommon-x11`. On Debian or Ubuntu:
`sudo apt install libxkbcommon-x11-0 libgl1`.

### Building the Arch package yourself

From a copy of the source code:

```sh
cd packaging/arch
makepkg -si
```

## Building from source

Install Rust from <https://rustup.rs>, then in the source folder:

```sh
cargo build --release      # the program is target/release/ettin-edit
cargo run --release -- notes.txt
```

## Updating

**Help > Check for Updates…** asks GitHub whether there's a newer release. If
there is, it shows what changed and an **Open Download Page** button; download
and install the new version over the old one. Your settings, themes, plugins
and open files are kept.

Ettin Edit also checks by itself once a week and only speaks up when there's
something new. Turn that off with **Help > Check for Updates Automatically**,
or in [Preferences](preferences.md) under **Files and Startup**. The check
sends nothing about you or your files; it only reads the public list of
releases.

## Coming from NixPad

Ettin Edit used to be called NixPad. The first time Ettin Edit starts, it
copies your NixPad settings, session, backups, themes and plugins to its own
folder, and plugins written for NixPad keep working. The old folder is left
alone, so you can delete it when you're happy.
