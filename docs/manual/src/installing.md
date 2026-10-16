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
it in **Open With** for text files. Software centres such as GNOME Software,
KDE Discover and Ubuntu's App Center show its licence (MIT), website, release
notes and age rating.

### Getting updates on Debian and Ubuntu

From 1.4.0 the `.deb` also adds the Ettin Edit package repository, which lives
next to this manual on GitHub Pages. After that, your software updater (or
`sudo apt update && sudo apt upgrade`) installs new versions like any other
app, and the software centre no longer says "No software repository included".
Install the 1.4.0 `.deb` by hand once if you have an older one.

The repository's settings are in `/etc/apt/sources.list.d/ettin-edit.sources`.
Delete that file to stop getting updates this way.

To add the repository without downloading the `.deb` first:

```sh
sudo curl -fsSLo /usr/share/keyrings/ettin-edit-archive-keyring.gpg \
  https://tharkis.github.io/Ettin-Edit/apt/ettin-edit-archive-keyring.gpg
sudo curl -fsSLo /etc/apt/sources.list.d/ettin-edit.sources \
  https://tharkis.github.io/Ettin-Edit/apt/ettin-edit.sources
sudo apt update && sudo apt install ettin-edit
```

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

On Debian and Ubuntu, the `.deb`'s package repository brings updates through
your software updater (see [above](#getting-updates-on-debian-and-ubuntu)).
Everywhere else, and on Debian and Ubuntu too if you like, Ettin Edit updates
itself:

**Help > Check for Updates…** (also on the toolbar's **Home** tab) asks GitHub
whether there's a newer release. If there is, it shows what changed and an
**Update Now** button. Ettin Edit downloads the file that matches how you
installed it, checks it against the SHA-256 checksum GitHub lists for it, and
installs it:

| Installed with | What Update Now does |
| --- | --- |
| **Arch / CachyOS** package | Installs the new `.pkg.tar.zst` with `pacman -U`. Your desktop asks for your password first. |
| **Debian / Ubuntu** package | Installs the new `.deb` with `apt-get install`. Your desktop asks for your password first. |
| **`.tar.gz`** or Windows **`.zip`** | Replaces the `ettin-edit` program in the folder you unpacked it to. |
| Windows **installer** | Downloads the new `setup.exe`. **Install and Restart** closes Ettin Edit, installs the update without any questions and starts Ettin Edit again. Installed for everyone, Windows asks for permission first. |

When it's done, **Restart Now** closes Ettin Edit and starts the new version.
It closes the way **File > Exit** does, so you're asked about unsaved changes
(or they're kept, if [Preferences](preferences.md) remembers them), and your
open files come back. **Later** keeps working with the old version until you
next start Ettin Edit (with the Windows installer, it installs the update when
you close Ettin Edit). Your settings, themes and plugins are kept.

The password prompt comes from `pkexec`. Without it, Ettin Edit shows the
command to run in a terminal instead, with a **Copy** button. A copy built
from source, or one in a folder you can't write to, can't update itself; the
dialog says so and offers the download page. If the download doesn't match its
checksum it isn't installed.

Downloads wait in `~/.cache/ettin-edit/updates` on Linux
(`%LOCALAPPDATA%\ettin-edit\updates` on Windows) and are removed by the next
update.

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
