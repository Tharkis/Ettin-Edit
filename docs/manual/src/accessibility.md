# Accessibility

Ettin Edit works with screen readers, and everything in it can be reached
from the keyboard.

## Screen readers

Ettin Edit publishes its window to the system's accessibility service
(AT-SPI on Linux, UI Automation on Windows), so screen readers such as
**Orca** and **NVDA** can read it. Buttons, tick boxes, menus, text boxes and
tabs are read by name.

Buttons that show only an icon or a symbol still have a name for the screen
reader, and the same name shows as a tooltip when you hover over them. For
example:

- the find bar's **Aa**, **W**, **\n** and **.\*** are read as **Match
  case**, **Whole word**, **Extended** (with what it means) and **Regular
  expression**, and as on or off;
- the row of icons under the list in **Transform > Recipes** is read as
  **New recipe**, **Duplicate**, **Delete…**, **Export Recipe…**, **Import
  Recipes…**, **Open Recipes Folder** and **Read the recipes folder again**;
- **B**, **I** and **U** in [Theme Colours](themes.md#edit-theme-colours) are
  read as **Bold**, **Italic** and **Underline**;
- toolbar buttons that have lost their labels in a narrow window (see
  [the toolbar](toolbar.md#when-the-window-is-narrow)) are read by their full
  names, and a collapsed group by the group's name.

On Linux, Orca reads applications through AT-SPI; turn it on in your
desktop's accessibility settings (or run `orca`), then start Ettin Edit.

## The keyboard

- <kbd>Tab</kbd> and <kbd>Shift+Tab</kbd> move between the controls of a
  window or dialog, and through the menu bar, the toolbar's tabs and its
  buttons; the one that has focus is outlined, and <kbd>Space</kbd> or
  <kbd>Enter</kbd> presses it. In the editor itself <kbd>Tab</kbd> indents,
  so click the status bar first to move through the window's controls.
- Every menu command can have a shortcut; see
  [Keyboard shortcuts](shortcuts.md) and the Shortcut Mapper.
- **View > Command Palette…** (<kbd>F1</kbd>) finds and runs any command by
  typing part of its name.
- Tapping <kbd>Alt</kbd> shows the menu bar while it is hidden.

## Seeing more clearly

- **View > Zoom In** and **Zoom Out** (<kbd>Ctrl+=</kbd> and
  <kbd>Ctrl+-</kbd>) change the text size of the editor.
- The light and dark [themes](themes.md) can be changed, down to every
  colour, in **View > Theme > Edit Theme Colours…**.

If something can't be reached or isn't read out, please
[open an issue](https://github.com/Tharkis/Ettin-Edit/issues).
