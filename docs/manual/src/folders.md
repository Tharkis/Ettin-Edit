# Files, folders and the command line

## The settings folder

Ettin Edit keeps everything it remembers in one folder:

- Linux: `~/.config/EttinEdit` (or `$XDG_CONFIG_HOME/EttinEdit` if you set
  `XDG_CONFIG_HOME`)
- Windows: `%APPDATA%\EttinEdit`, usually
  `C:\Users\<you>\AppData\Roaming\EttinEdit`

| In it | What it holds |
| --- | --- |
| `settings.json` | Your preferences, recent files, saved macros, changed shortcuts, and the schemas picked for XML files |
| `session.json` | The tabs that were open, with their carets, encodings and split view |
| `backup/` | Backups of unsaved changes, so they survive a crash |
| `themes/` | Every theme as a `.tmTheme` file ([Themes](themes.md)). `.builtin-themes.json` tracks which built-in themes you haven't edited. |
| `languages/` | Your own languages ([Languages](languages.md)) |
| `api/` | Your own completion lists ([Languages](languages.md#your-own-lists)) |
| `plugins/` | Your plugins, one folder each ([Using plugins](plugins.md)). `.sdk/` holds the plugin SDKs, kept up to date by Ettin Edit. |
| `instance.json` | How a second Ettin Edit finds the running one |

The **Open … Folder** commands in the Theme, Language and Plugins menus open
these folders for you, creating them if needed.

### Backing up or moving your settings

Close Ettin Edit and copy the whole folder. To start afresh, close Ettin Edit
and rename the folder; Ettin Edit makes a new one with the defaults next time
it starts.

## The command line

```
ettin-edit [--multi-instance] [file ...]
```

- Each file opens in a tab. A file that can't be opened, for example because
  it doesn't exist, is reported and skipped.
- If Ettin Edit is already running, the files open in its window (unless
  **Settings > Single Window** is off).
- `--multi-instance` (or `-multiInst`, as in Notepad++) always starts a new
  window.

On Windows, the installer can add Ettin Edit to the `PATH` so this works in
any terminal. On Linux the packages put `ettin-edit` on the `PATH`.

## Plugins' environment

Plugins run as separate programs. They get the `ETTIN_VERSION` environment
variable with Ettin Edit's version; see [Writing plugins](plugin-guide.md).
