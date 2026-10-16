# Troubleshooting and FAQ

## Ettin Edit won't start on Linux

Run `ettin-edit` in a terminal to see the error. The usual cause is a missing
desktop library when using the `.tar.gz`: install OpenGL and, on X11,
`libxkbcommon-x11` (`sudo apt install libxkbcommon-x11-0 libgl1` on Debian
and Ubuntu). The `.deb` and Arch packages install these for you.

## The software centre says "No software repository included" or "Unknown licence"

That's a `.deb` from before 1.4.0, which had no package information for
software centres. Install the latest `.deb` once: it shows the licence and
release notes, and adds the Ettin Edit repository so updates arrive through
your software updater. See [Getting updates on Debian and
Ubuntu](installing.md#getting-updates-on-debian-and-ubuntu).

## A file opened in the running window instead of a new one

That's the **Single Window** setting. Turn it off in the **Settings** menu, or
start Ettin Edit with `--multi-instance`, to get separate windows. See
[One window or several](files.md#one-window-or-several).

## The text looks garbled: `Ã©` instead of `é`

Ettin Edit guessed the wrong encoding. Choose **Encoding > Reopen With** and
pick the right one; for files from older Windows programs it's usually
**Windows-1252**. See [Encodings](encodings.md).

## Saving says a character can't be saved

The file's encoding can't hold that character (for example `€` in
ISO-8859-1). Choose **Encoding > UTF-8** and save again, or remove the
character. Nothing is lost while you decide.

## Chinese, Japanese or Korean text shows as boxes

Install a CJK font, such as Noto Sans CJK (`noto-fonts-cjk` on Arch,
`fonts-noto-cjk` on Debian and Ubuntu), and restart Ettin Edit.

## A shortcut doesn't work

- Open the [command palette](view.md#command-palette) (<kbd>F1</kbd>) and
  find the command: it shows the shortcut actually in use.
- Check the [Shortcut Mapper](shortcuts.md#changing-shortcuts) for a clash
  with another command or a plugin.
- Your desktop may be using the keys itself. GNOME, KDE and Windows all
  reserve a few <kbd>Ctrl+Alt</kbd> combinations. Pick another shortcut for
  the command.

## A plugin doesn't run

1. Open **Plugins > Manage Plugins…** and look under the plugin's name for
   the reason.
2. Check **Plugins > Plugin Log** for errors.
3. Make sure Python 3 or Node.js is installed, or set its path in
   [Preferences > Plugins](preferences.md#plugins).
4. If you chose **Don't Allow** earlier, turn the plugin back on in Manage
   Plugins.

## The update check says it couldn't reach GitHub

Ettin Edit needs to reach `api.github.com` to check for updates. Behind a
proxy or firewall that blocks it, check the
[releases page](https://github.com/Tharkis/Ettin-Edit/releases) in your
browser instead, and turn off **Help > Check for Updates Automatically**.

## Word wrap is slow to resize on a huge file

Resizing the window with word wrap on measures every line again, which takes
a moment in files with hundreds of thousands of lines. Turn word wrap off
(<kbd>Alt+Z</kbd>) for files that size.

## I closed a file without saving

If **Remember Unsaved Changes on Exit** is on (the default), quitting never
throws changes away. Closing a single tab does ask first, though, and
**Don't Save** really doesn't save. If the file was saved at some point,
your file manager's trash or your backups may still have an older copy.

## Where do I report a bug or ask for a feature?

On the [issues page](https://github.com/Tharkis/Ettin-Edit/issues). Say which
version you have (**Help > About Ettin Edit**) and which system you use, and
attach the plugin log if a plugin is involved.

## Is Ettin Edit Notepad++?

No. It's inspired by Notepad++ and shares none of its code; every line was
written for Ettin Edit. It's free and open source under the MIT licence.

## Who gets the spoon?

Still being decided. Please don't open an issue about it.
