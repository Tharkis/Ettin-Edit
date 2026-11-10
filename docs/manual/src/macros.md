# Macros

A macro records what you do in the editor and plays it back, so a repetitive
edit only has to be done once. Macros are in **Transform > Macros**, next to
[recipes](recipes.md); the top of each submenu says how they differ:

- **Macros replay keystrokes and edits**, from wherever the caret is.
- **Recipes replay transforms** on a selection, a document or every tab, and
  can run from the command line.

## Recording and playing back

1. Put the caret where the first edit starts.
2. Choose **Transform > Macros > Start Recording** (<kbd>Ctrl+Shift+R</kbd>). The status
   bar shows **● Recording macro**.
3. Make the edit: type, move with the keyboard, use the clipboard, and use
   commands such as Find Next, line operations or case conversion.
4. Choose **Transform > Macros > Stop Recording** (<kbd>Ctrl+Shift+R</kbd> again).
5. Choose **Transform > Macros > Playback** (<kbd>Ctrl+Shift+P</kbd>) to do it all again
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

**Transform > Macros > Run a Macro Multiple Times…** plays the current macro a number of
times you choose, or with **Run until the end of the file** until the caret
reaches the last line.

## Saving macros

The current macro is forgotten when you record a new one or quit. To keep it,
choose **Transform > Macros > Save Current Macro…** and give it a name. Saved macros:

- appear at the bottom of **Transform > Macros**, ready to play,
- are in the [command palette](view.md#command-palette),
- can have a keyboard shortcut of their own: find the macro in the
  [Shortcut Mapper](shortcuts.md#changing-shortcuts) and click **Set…**,
- are kept between sessions.

**Transform > Macros > Delete Saved Macro** removes one.

## Turning a macro into a recipe

When everything you recorded is a transform (sorting, trimming, case
changes, Replace All, anything in the Transform menu), **Transform > Macros >
Save Macro as Recipe…** turns the macro into a [recipe](recipes.md) with the
same steps and opens it in the Recipes window, ready for a name. Replace All
keeps the find bar's text and options as they were when you recorded it.

If the macro has anything else in it, such as typing or moving the caret, the
item is greyed out and its tooltip names the first step that can't be a
recipe step, for example *Can't become a recipe: typing “xy” isn't a
transform.*

## Macros or recipes?

A macro replays keys and commands from wherever the caret is, which suits
edits that move through the text. When the job is turning a whole selection or
document into something else (replace this, keep those lines, sort, convert
to JSON), a [recipe](recipes.md) is usually the better fit: it shows a preview,
can be edited step by step, runs on all open tabs or from the command line, and
is a file you can share.
