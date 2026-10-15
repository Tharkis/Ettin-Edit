# Macros

A macro records what you do in the editor and plays it back, so a repetitive
edit only has to be done once.

## Recording and playing back

1. Put the caret where the first edit starts.
2. Choose **Macro > Start Recording** (<kbd>Ctrl+Shift+R</kbd>). The status
   bar shows **● Recording macro**.
3. Make the edit: type, move with the keyboard, use the clipboard, and use
   commands such as Find Next, line operations or case conversion.
4. Choose **Macro > Stop Recording** (<kbd>Ctrl+Shift+R</kbd> again).
5. Choose **Macro > Playback** (<kbd>Ctrl+Shift+P</kbd>) to do it all again
   from wherever the caret is.

The trick is to finish the recording where the next run should start, for
example at the beginning of the next line. Use keys rather than the mouse
while recording; mouse clicks aren't recorded.

> **Example:** to wrap every line of a list in quotes and add a comma, put
> the caret at the start of the first line and start recording. Type `"`,
> press <kbd>End</kbd>, type `",`, then press <kbd>Down</kbd> and
> <kbd>Home</kbd>. Stop recording. Now each playback does the next line, and
> [Run a Macro Multiple Times](#running-a-macro-many-times) does the rest.

## Running a macro many times

**Macro > Run a Macro Multiple Times…** plays the current macro a number of
times you choose, or with **Run until the end of the file** until the caret
reaches the last line.

## Saving macros

The current macro is forgotten when you record a new one or quit. To keep it,
choose **Macro > Save Current Macro…** and give it a name. Saved macros:

- appear at the bottom of the **Macro** menu, ready to play,
- are in the [command palette](view.md#command-palette),
- can have a keyboard shortcut of their own: find the macro in the
  [Shortcut Mapper](shortcuts.md#changing-shortcuts) and click **Set…**,
- are kept between sessions.

**Macro > Delete Saved Macro** removes one.
