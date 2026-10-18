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
| Debian, Ubuntu, Mint, Pop!_OS, Zorin OS | `.deb` | `sudo apt install ./ettin-edit_*_amd64.deb` |
| Fedora, openSUSE | `.rpm` | `sudo dnf install ./ettin-edit-*.x86_64.rpm` (openSUSE: `sudo zypper install ./ettin-edit-*.x86_64.rpm`) |
| Any Linux, with Flatpak | `.flatpak` | Open it in your software centre, or `flatpak install --user ./ettin-edit-*-x86_64.flatpak` |
| Any Linux, nothing to install | `.AppImage` | Make it executable and run it |
| Anything else | `.tar.gz` | Extract it and run `./ettin-edit` |

The packages and the Flatpak add Ettin Edit to your application menu with its
icon, and offer it in **Open With** for text files. They carry Ettin Edit's
AppStream file, so software centres such as GNOME Software, KDE Discover and
Ubuntu's App Center show its licence (MIT), website, release notes and age
rating.

### Getting updates on Debian and Ubuntu

From 1.4.0 the `.deb` also adds the Ettin Edit package repository, which lives
next to this manual on GitHub Pages. After that, your software updater (or
`sudo apt update && sudo apt upgrade`) installs new versions like any other
app. From 1.9.1 the repository also carries the app details software centres
read, so once it's added, Ettin Edit shows up in their search with its licence,
screenshots and release notes. Install the latest `.deb` by hand once if you
have one older than 1.4.0.

The repository's settings are in `/etc/apt/sources.list.d/ettin-edit.sources`.
Delete that file to stop getting updates this way.

To add the repository without downloading the `.deb` first, which also gets
you the full software centre page before installing:

```sh
sudo curl -fsSLo /usr/share/keyrings/ettin-edit-archive-keyring.gpg \
  https://tharkis.github.io/Ettin-Edit/apt/ettin-edit-archive-keyring.gpg
sudo curl -fsSLo /etc/apt/sources.list.d/ettin-edit.sources \
  https://tharkis.github.io/Ettin-Edit/apt/ettin-edit.sources
sudo apt update && sudo apt install ettin-edit
```

### Why a downloaded .deb shows "Unknown licence"

If you open the `.deb` file itself in GNOME Software (the Software app on
Zorin OS, and on many Ubuntu-based systems), the page says "Unknown licence"
and "No software repository included". That page only reads the package's
basic fields, which have no licence field, and it only recognises Fedora-style
repositories, so no `.deb` can change it. The package does have the licence,
release notes and repository, and they show once it's installed or once the
repository is added (see above). For the full page before installing, use the
Flatpak, or add the repository first.

### Fedora and openSUSE

The `.rpm` installs Ettin Edit and adds the Ettin Edit RPM repository (for
`dnf`, and `zypper` on openSUSE) at `https://tharkis.github.io/Ettin-Edit/rpm`.
From then on your software updater, `sudo dnf upgrade` or `sudo zypper update`
installs new versions. The package and repository are signed; the first
time it updates, `dnf` (or `zypper`) asks you to trust the Ettin Edit key.
The repository's settings are in
`/etc/yum.repos.d/ettin-edit.repo` (`/etc/zypp/repos.d/ettin-edit.repo` on
openSUSE); delete that file to stop getting updates this way.

To add the repository without downloading the `.rpm` first:

```sh
sudo rpm --import https://tharkis.github.io/Ettin-Edit/rpm/RPM-GPG-KEY-ettin-edit
sudo curl -fsSLo /etc/yum.repos.d/ettin-edit.repo \
  https://tharkis.github.io/Ettin-Edit/rpm/ettin-edit.repo
sudo dnf install ettin-edit
```

### Flatpak

The `.flatpak` runs Ettin Edit on the freedesktop.org runtime, which your
system downloads from Flathub the first time (add Flathub with
`flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo`
if you haven't). Opening the file in a software centre shows the full app
page, licence included. Installing it also adds the Ettin Edit Flatpak
repository, so `flatpak update` and your software centre install new
versions. You can also install straight from the repository:

```sh
flatpak install --user https://tharkis.github.io/Ettin-Edit/flatpak/ettin-edit.flatpakref
```

The Flatpak can open any file on your computer. Git (for **Compare to Last Commit
(Git)**), **Open Terminal Here** and plugins run on your computer itself, not in
the sandbox, so they use the Python, Node.js, Git and terminal you have
installed. A Flatpak keeps its settings in
`~/.var/app/io.github.tharkis.EttinEdit/config/EttinEdit`, apart from other
installs (copy your `~/.config/EttinEdit` there to bring them along).

### AppImage

The `.AppImage` is one file that runs on almost any 64-bit Linux: make it
executable (`chmod +x ettin-edit-*.AppImage`, or tick "Allow executing" in its
properties) and run it. Tools such as Gear Lever or AppImageLauncher add it to
your application menu and can update it, and so can Ettin Edit itself (see
[Updating](#updating)). Like the `.tar.gz`, it uses your system's OpenGL and
`libxkbcommon-x11`.

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

On Debian, Ubuntu, Fedora and openSUSE, the package repository brings updates
through your software updater (see [above](#getting-updates-on-debian-and-ubuntu)),
and the Flatpak updates through `flatpak update` or your software centre.
Everywhere else, and with the packages too if you like, Ettin Edit updates
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
| **Fedora / openSUSE** package | Installs the new `.rpm` with `dnf install` (`zypper` on openSUSE). Your desktop asks for your password first. |
| **AppImage** | Puts the new `.AppImage` in place of the one you started. |
| **Flatpak** | Nothing: it says updates come through your software centre or `flatpak update`. |
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
