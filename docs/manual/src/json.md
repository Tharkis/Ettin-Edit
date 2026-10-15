# JSON and JavaScript

**Tools > JSON and JavaScript** covers what the JSTool plugin does for
Notepad++, built in.

Like every text tool, these work on the selection, or on each selection when
there are several, or on the whole document when nothing is selected. Each
use is one undo step. When a tool finds a mistake, a notice at the bottom of
the window gives the line and column, the caret moves there, and **Go to**
takes you back to it later.

## Format and minify

| Command | Shortcut | What it does |
| --- | --- | --- |
| **Format** | <kbd>Ctrl+Alt+M</kbd> | Indents JSON or JavaScript neatly, one item per line |
| **Minify** | <kbd>Ctrl+Alt+N</kbd> | Removes all the spaces and line breaks that aren't needed |

Formatting indents with a tab or with spaces, as set for the editor in
[Preferences](preferences.md).

Both commands try strict JSON first. If the text isn't JSON (it has comments,
single quotes, trailing commas or code), they treat it as JavaScript and say
so in the notice. The JavaScript formatter understands modern syntax:
template strings, regular expression literals, arrow functions, classes,
`switch` and `case`, and comments, which it keeps.

## Checking and sorting

- **Validate JSON** checks that the text is valid JSON and points at the first
  mistake if it isn't.
- **Sort JSON Keys** puts the keys of every object in alphabetical order, all
  the way down, and formats the result.

## The JSON tree

**JSON Tree** (<kbd>Ctrl+Alt+J</kbd>) opens a panel on the right showing the
current document as a tree of objects, arrays and values.

- The tree updates as you type and opens at the caret's position.
- Click a node to jump to it in the text.
- Type in **Filter keys and values** to show only the nodes whose key or
  value contains some text, with their parents.
- The **⊞** and **⊟** buttons at the top expand and collapse everything;
  right-click a node for **Expand All Inside**.
- Right-click a node to **Copy Path** (a path such as `$.items[2].name`),
  **Copy Key** or **Copy Value**.

If the document isn't valid JSON, the panel says where the problem is, with
a **Go to Error** button. Very large documents are too big to show as a tree;
the panel says so rather than freezing.
