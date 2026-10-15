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

Pick one from the **Theme** menu, where they're listed under **Dark** and
**Light**. The change is immediate and remembered.

## Themes are files

Every theme, built-in ones included, is a TextMate `.tmTheme` file in the
themes folder:

- Linux: `~/.config/EttinEdit/themes/`
- Windows: `%APPDATA%\EttinEdit\themes\`

**Theme > Open Themes Folder** opens it. This means you can:

- **Change a built-in theme** by editing its file, then choosing **Theme >
  Reload Themes**.
- **Add a theme** by dropping any `.tmTheme` file into the folder and
  reloading. Themes you add appear under **Custom** in the Theme menu.
  Thousands of `.tmTheme` files exist for TextMate, Sublime Text and other
  editors.
- **Share a theme** by sending someone the file.

When Ettin Edit is updated, built-in themes you haven't edited are updated
with it; ones you have edited are left alone. **Theme > Restore Built-in
Themes** puts the original built-in themes back and keeps each edited copy
as `<name>.tmTheme.bak`, so nothing is lost.

## Edit Theme Colours

**Theme > Edit Theme Colours…** changes a theme without touching its file by
hand. Every change shows in the editor at once, so you can see what you're
doing.

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
- **Save** writes the changes to the theme's file. **Save As New Theme…**
  saves them as a new theme with a name you choose, leaving the original
  alone. **Discard Changes** goes back to the theme as it was saved, and
  closing the window without saving does the same.

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
