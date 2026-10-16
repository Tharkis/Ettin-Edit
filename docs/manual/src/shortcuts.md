# Keyboard shortcuts

Every command in Ettin Edit can have a keyboard shortcut, and you can change
any of them. This page lists the defaults; the menus and the
[command palette](view.md#command-palette) always show the ones in use.

## Changing shortcuts

**Settings > Shortcut Mapper…** lists every command, every saved
[macro](macros.md) and every plugin command, with their shortcuts.

1. Type in **Find** to narrow the list, by the command's name or by a
   shortcut such as `ctrl+d`. Tick **Only commands with a shortcut** to hide
   the rest.
2. Select a command.
3. Click **Set…** (or **Change…**), then press the new keys. <kbd>Esc</kbd>
   cancels.
4. If another command already uses those keys, Ettin Edit tells you which one.
   **Move It Here** takes the shortcut from the other command; **Cancel**
   leaves things as they were.

A command can have more than one shortcut: **Add Another…** adds one,
**Remove** removes the selected one, and **Reset to Default** puts the
command's original shortcut back. **Reset All Shortcuts** resets every
command.

Rules for shortcuts:

- They need <kbd>Ctrl</kbd> or <kbd>Alt</kbd>, unless they use a function key
  (<kbd>F1</kbd> to <kbd>F12</kbd>), so plain typing is never taken over.
- <kbd>Ctrl+C</kbd>, <kbd>Ctrl+X</kbd> and <kbd>Ctrl+V</kbd> are kept for
  copy, cut and paste.
- Undo, redo, cut, copy, paste, select all and completion are handled by the
  editor itself and can't be changed.

Your shortcuts are saved in `settings.json` in the
[settings folder](folders.md), so they move with your settings.

## Default shortcuts

On macOS-style keyboards, <kbd>Ctrl</kbd> is still <kbd>Ctrl</kbd>; Ettin Edit
runs on Linux and Windows only.

### File

| Command | Shortcut |
| --- | --- |
| New | <kbd>Ctrl+N</kbd> |
| Open… | <kbd>Ctrl+O</kbd> |
| Save | <kbd>Ctrl+S</kbd> |
| Save As… | <kbd>Ctrl+Alt+S</kbd> |
| Save All | <kbd>Ctrl+Shift+S</kbd> |
| Print… | <kbd>Ctrl+P</kbd> |
| Close | <kbd>Ctrl+W</kbd> |
| Close All | <kbd>Ctrl+Shift+W</kbd> |

### Edit

| Command | Shortcut |
| --- | --- |
| Undo | <kbd>Ctrl+Z</kbd> |
| Redo | <kbd>Ctrl+Y</kbd> |
| Cut | <kbd>Ctrl+X</kbd> |
| Copy | <kbd>Ctrl+C</kbd> |
| Paste | <kbd>Ctrl+V</kbd> |
| Select All | <kbd>Ctrl+A</kbd> |
| Line Operations > Duplicate | <kbd>Ctrl+D</kbd> |
| Line Operations > Delete Lines | <kbd>Ctrl+Shift+L</kbd> |
| Line Operations > Move Up | <kbd>Ctrl+Shift+Up</kbd> |
| Line Operations > Move Down | <kbd>Ctrl+Shift+Down</kbd> |
| Line Operations > Join Lines | <kbd>Ctrl+J</kbd> |
| Line Operations > Split Lines at Long-Line Column | <kbd>Ctrl+I</kbd> |
| Line Operations > Insert Blank Line Above | <kbd>Ctrl+Alt+Enter</kbd> |
| Line Operations > Insert Blank Line Below | <kbd>Ctrl+Alt+Shift+Enter</kbd> |
| Convert Case To > UPPERCASE | <kbd>Ctrl+Shift+U</kbd> |
| Convert Case To > lowercase | <kbd>Ctrl+U</kbd> |
| Convert Case To > Title Case | <kbd>Alt+U</kbd> |
| Convert Case To > Sentence case | <kbd>Ctrl+Alt+U</kbd> |
| Convert Case To > iNVERT cASE | <kbd>Ctrl+Alt+Shift+U</kbd> |
| Comment > Toggle Line Comment | <kbd>Ctrl+Q</kbd> |
| Comment > Toggle Block Comment | <kbd>Ctrl+Shift+Q</kbd> |
| Insert > Date and Time (Short) | <kbd>F5</kbd> |
| Insert > Date and Time (Long) | <kbd>Shift+F5</kbd> |
| Multi-Select > Select Next Occurrence | <kbd>Ctrl+Shift+D</kbd> |
| Multi-Select > Select All Occurrences | <kbd>Alt+F3</kbd> |
| Auto-Completion > Complete Word | <kbd>Ctrl+Space</kbd> |
| Auto-Completion > Show Parameter Hint | <kbd>Ctrl+Shift+Space</kbd> |
| Column Editor… | <kbd>Alt+C</kbd> |
| Column Mode | <kbd>Alt+Shift+C</kbd> |

### Search

| Command | Shortcut |
| --- | --- |
| Find… | <kbd>Ctrl+F</kbd> |
| Replace… | <kbd>Ctrl+H</kbd> |
| Find Next | <kbd>F3</kbd> |
| Find Previous | <kbd>Shift+F3</kbd> |
| Find Selected Next | <kbd>Ctrl+F3</kbd> |
| Find Selected Previous | <kbd>Ctrl+Shift+F3</kbd> |
| Find in Files… | <kbd>Ctrl+Shift+F</kbd> |
| Go to Line… | <kbd>Ctrl+G</kbd> |
| Bookmark > Toggle Bookmark | <kbd>Ctrl+F2</kbd> |
| Bookmark > Next Bookmark | <kbd>F2</kbd> |
| Bookmark > Previous Bookmark | <kbd>Shift+F2</kbd> |

### View

| Command | Shortcut |
| --- | --- |
| Word Wrap | <kbd>Alt+Z</kbd> |
| Distraction-Free Mode | <kbd>F11</kbd> |
| Command Palette… | <kbd>F1</kbd> |
| Collapse Toolbar | <kbd>Ctrl+F1</kbd> |
| Next Tab | <kbd>Ctrl+Tab</kbd> or <kbd>Ctrl+PageDown</kbd> |
| Previous Tab | <kbd>Ctrl+Shift+Tab</kbd> or <kbd>Ctrl+PageUp</kbd> |
| Focus Other View | <kbd>F8</kbd> |
| Hex View | <kbd>Ctrl+Alt+Shift+H</kbd> |
| Folding > Fold All | <kbd>Alt+0</kbd> |
| Folding > Unfold All | <kbd>Alt+Shift+0</kbd> |
| Folding > Fold Current Level | <kbd>Ctrl+Alt+F</kbd> |
| Folding > Unfold Current Level | <kbd>Ctrl+Alt+Shift+F</kbd> |
| Folding > Fold Level 1 … 5 | <kbd>Alt+1</kbd>, <kbd>Alt+2</kbd>, <kbd>Alt+3</kbd>, <kbd>Alt+4</kbd>, <kbd>Alt+5</kbd> |
| Zoom In | <kbd>Ctrl++</kbd> or <kbd>Ctrl+=</kbd> |
| Zoom Out | <kbd>Ctrl+-</kbd> |
| Reset Zoom | <kbd>Ctrl+0</kbd> |

### Settings

| Command | Shortcut |
| --- | --- |
| Preferences… | <kbd>Ctrl+,</kbd> |

### Tools

| Command | Shortcut |
| --- | --- |
| Compare > Compare with Previous Tab / the Two Views | <kbd>Ctrl+Alt+D</kbd> |
| Compare > Next Difference | <kbd>F7</kbd> |
| Compare > Previous Difference | <kbd>Shift+F7</kbd> |
| CsvQuery > CsvQuery Panel | <kbd>Ctrl+Alt+Q</kbd> |
| CsvQuery > Run Query (in the panel) | <kbd>Ctrl+Enter</kbd> |
| JSON and JavaScript > Format | <kbd>Ctrl+Alt+M</kbd> |
| JSON and JavaScript > Minify | <kbd>Ctrl+Alt+N</kbd> |
| JSON and JavaScript > JSON Tree | <kbd>Ctrl+Alt+J</kbd> |
| XML Tools > Pretty Print | <kbd>Ctrl+Alt+Shift+B</kbd> |
| XML Tools > Linearize | <kbd>Ctrl+Alt+Shift+L</kbd> |
| XML Tools > Check Syntax | <kbd>Ctrl+Alt+Shift+M</kbd> |
| XML Tools > XPath Panel | <kbd>Ctrl+Alt+Shift+E</kbd> |
| XML Tools > Copy Current XPath | <kbd>Ctrl+Alt+Shift+P</kbd> |

### Macro

| Command | Shortcut |
| --- | --- |
| Start or Stop Recording | <kbd>Ctrl+Shift+R</kbd> |
| Playback | <kbd>Ctrl+Shift+P</kbd> |

### Help

| Command | Shortcut |
| --- | --- |
| User Manual | <kbd>Shift+F1</kbd> |

### In the editor

These aren't commands, so they're not in the Shortcut Mapper:

| Keys | What they do |
| --- | --- |
| <kbd>Ctrl</kbd>+click, <kbd>Alt</kbd>+click | Add a caret |
| <kbd>Ctrl</kbd>+drag | Add a selection |
| <kbd>Alt</kbd>+drag, <kbd>Alt+Shift+arrows</kbd> | Column selection (a plain drag and <kbd>Shift+arrows</kbd> in [Column Mode](editing.md#column-mode)) |
| <kbd>Ctrl+Alt+Up</kbd>, <kbd>Ctrl+Alt+Down</kbd> | Add a caret above or below |
| <kbd>Esc</kbd> | Back to one caret; close the completion list, a parameter hint or the find bar |
| <kbd>Home</kbd> | First non-blank character, then the start of the line |
| <kbd>Ctrl+Left</kbd>, <kbd>Ctrl+Right</kbd> | Move by words |
| <kbd>Ctrl+Backspace</kbd>, <kbd>Ctrl+Delete</kbd> | Delete by words |
| <kbd>Tab</kbd>, <kbd>Shift+Tab</kbd> | Indent or outdent the selected lines |
| Middle-click on a tab | Close it |
