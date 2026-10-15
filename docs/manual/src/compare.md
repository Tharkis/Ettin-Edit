# Compare

**Tools > Compare** shows the differences between two versions of a text,
side by side. It's Ettin Edit's take on the ComparePlus plugin.

## Starting a comparison

| Command (Tools > Compare) | Compares the current tab with… |
| --- | --- |
| **Compare with Previous Tab** (<kbd>Ctrl+Alt+D</kbd>) | the tab you were in before it. When the window is [split](view.md#split-view), this is **Compare the Two Views** and compares the two tabs you can see. |
| **Compare With** | any other open tab you pick |
| **Compare to Last Save** | the file as it is on disk, showing your unsaved changes |
| **Compare to Clipboard** | the text on the clipboard |
| **Compare to Last Commit (Git)** | the file as it was in the last Git commit. Needs `git` installed and the file in a Git repository. |
| **Compare Selections** | two pieces of text: select one, then <kbd>Ctrl</kbd>+drag to select the other, or select one in each view |

The same commands are on the editor's right-click menu under **Tools >
Compare**.

## The Compare tab

The comparison opens in a tab of its own, with the two texts side by side
and their lines lined up:

- **Added** lines, **removed** lines and **changed** lines each have their own
  colour, and the words that changed inside a changed line are highlighted.
- **Moved** blocks, lines that were cut from one place and pasted in another,
  are coloured apart from additions and removals.
- Blank filler rows keep the two sides level, and both sides scroll
  together.
- The bar at the right edge shows where all the differences are in the file;
  click or drag in it to go there.

The Compare tab is read-only, but it follows the tabs being compared: edit
one of them and the comparison updates. Double-click a line to jump to it in
its own tab and edit it there.

### The toolbar

- **First**, **↑**, **↓** and **Last** move between differences; <kbd>F7</kbd>
  and <kbd>Shift+F7</kbd> do the same from anywhere.
- *Difference 3 of 12* and a count of changed, added and removed lines, or
  **Identical** when there's nothing to show.
- **Swap Sides** puts the left text on the right.
- **Export Patch** opens the differences as a unified diff (the format `git
  diff` and `patch` use) in a new tab.
- **Options**:
  - **Ignore Spaces and Tabs**: differences in spacing don't count.
  - **Ignore Case**: `Hello` and `hello` are the same.
  - **Ignore Empty Lines**: added or removed blank lines don't count.
  - **Detect Moved Lines**: on by default.
- **×** closes the comparison (also **Tools > Compare > Close Comparison**).

**Tools > Compare > Show Comparison** brings the Compare tab back to the
front.

Very large or very different files stay responsive: the comparison has a
time limit and only draws the rows on screen.
