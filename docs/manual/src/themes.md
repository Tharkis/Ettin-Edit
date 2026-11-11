# Themes and colours

A theme colours both the text and the whole window: menus, tabs, panels and
the status bar. Ettin Edit comes with nine:

| Dark | Light |
| --- | --- |
| **Ettin Dark** (the default, based on Base16 Eighties) | **Ettin Light** |
| **Ettin Night** | **Ocean Light** |
| **Ocean Dark** | **Solarized Light** |
| **Mocha** | **GitHub Light** |
| **Solarized Dark** | |

Pick one from **View > Theme**, where they're listed under **Dark** and
**Light**. The change is immediate and remembered.

To have Ettin Edit switch between a light and a dark theme with your desktop,
turn on **Follow the system's light/dark setting** in
[Preferences > Appearance](preferences.md#appearance) and pick the pair.

## Themes are files

Every theme, built-in ones included, is a TextMate `.tmTheme` file in the
themes folder:

- Linux: `~/.config/EttinEdit/themes/`
- Windows: `%APPDATA%\EttinEdit\themes\`

**View > Theme > Open Themes Folder** opens it. This means you can:

- **Change a built-in theme** by editing its file. When you save it, Ettin
  Edit reloads it within a second. If the file has a mistake, the editor keeps
  the last version that worked and a message names the file and line, with an
  **Open** button. (**View > Theme > Reload Themes** reloads them all.)
- **Add a theme** by dropping any `.tmTheme` file into the folder. Themes you
  add appear under **Custom** in **View > Theme**.
  Thousands of `.tmTheme` files exist for TextMate, Sublime Text and other
  editors.
- **Share a theme** by sending someone the file.

When Ettin Edit is updated, built-in themes you haven't edited are updated
with it; ones you have edited are left alone. **View > Theme > Restore Built-in
Themes** puts the original built-in themes back and keeps each edited copy
as `<name>.tmTheme.bak`, so nothing is lost.

## Edit Theme Colours

**View > Theme > Edit Theme Colours…** changes a theme without touching its file by
hand. Every change shows in the editor at once and is saved to the theme's
file half a second after you stop, so you can see what you're doing and
there's nothing to remember to save.

- **Theme** at the top picks the theme to edit (and switches to it).
- **Editor** colours: the **Background**, **Text**, **Caret**, **Selection**,
  **Current line**, **Line numbers**, **Indent guides**, **Whitespace
  symbols**, **Search matches**, and the **Accent** used for the current tab,
  buttons and fold markers. Click a colour to change it, or **Don't set this
  colour** to fall back to the default.
- **Syntax** colours are a list of rules. Each rule has a name, a **Scope**
  that says which text it colours, a colour, and **B**, **I** and **U** for
  bold, italic and underline. Type in **Find** to look for a rule, such as
  `string` or `comment`. **Add Rule** adds one, and the button at the end of
  a rule deletes it.
- **Undo Changes** puts the theme back exactly as it was when you opened the
  window. The window says how many settings you've changed since then.
- **Reset** puts a built-in theme back as Ettin Edit ships it. (Themes you
  added have no original to go back to, so only Undo Changes works for them.)
- **Save As New Theme…** saves your changes as a new theme with a name you
  choose and puts the original back as it was, so you can experiment on a
  built-in theme and keep the result separately.
- **Close** keeps the changes.

### Scopes

Languages label each piece of text with a *scope*, such as
`comment.line.double-slash.rust` or `string.quoted.double.json`. A rule's
scope colours every piece of text whose scope starts with it, so `string`
colours every kind of string in every language, and `string.quoted.double.json`
only colours double-quoted strings in JSON. The most specific rule wins.
Separate several scopes with commas.

The scopes most themes set are `comment`, `string`, `constant.numeric`,
`constant.language`, `keyword`, `storage`, `entity.name.function`,
`entity.name.type`, `variable`, `support.function` and `invalid`. See
[Languages](languages.md#keyword-groups) for the scopes your own languages
use.

## Fonts

The editor's font and size are in [Preferences](preferences.md), not in the
theme, so they stay the same whichever theme you pick.
