# Tables

A table tab shows data as rows and columns, like a spreadsheet, so you can
reshape an export from one system into what another system wants: pick and
rename columns, read dates the way they were written, filter and sort rows,
and write the result as CSV, an Excel workbook, JSON or Markdown.

Every change you make is a **step**. The steps are kept in order on the
right, can be switched off, changed or removed at any time, and can be saved
as a [recipe](recipes.md) that does the same to next week's export, in the
app or from the command line.

<p><img class="screenshot" src="images/table-tab.png" alt="A table tab: a grid of patient rows with typed column headers, a Filters chip reading Ward is North, and a Steps panel on the right with Source, Born is Date and Rows where Ward is North"></p>

## Opening data as a table

- **Excel and OpenDocument workbooks** (`.xlsx`, `.xlsm`, `.xls`, `.xlsb`,
  `.ods`) open straight into a table tab, from **File > Open…**, the
  command line or by dropping them on the window. The workbook itself is
  never changed; use **Export** to write the result.
- **CSV, TSV and JSON**: open the file as usual and press
  <kbd>Ctrl+Alt+T</kbd> (**Table > Show as Table**), or use **File > Open as
  Table…** to do both at once. When a file looks like CSV, the
  [suggestion bar](suggestions.md) offers **Show as Table** too.
- Any text tab can be shown as a table. <kbd>Ctrl+Alt+T</kbd> again (or
  **Show as Text**) goes back to the text; the table and its steps stay, and
  follow the text when you edit it.

Ettin Edit guesses how to read the data: the delimiter and whether the
first row holds the column names for delimited text, the header row for a
workbook, and the list of rows for JSON. The **Source** step at the top of
the Steps panel says what it chose; click it (or **Table > Import
Options…**) to change it, with a live preview:

| Data | Options |
| --- | --- |
| Delimited text | Delimiter (comma, tab, semicolon, pipe or any other character), quote character or none, first row has the column names |
| Workbook | Sheet, and the row with the column names (rows above it, such as a report title, are skipped; 0 means there are no names) |
| JSON | Where the list of rows is, like `data.items` (empty for a file that is a list). Nested objects become columns with dotted names such as `address.city`; lists stay as JSON text |

**Detect column types** is on by default.

## Column types

Each column has a type, shown as a small badge in its header:

| Badge | Type | Values look like |
| --- | --- | --- |
| `abc` | Text | anything |
| `123` | Whole number | `42`, `-7` |
| `1.2` | Decimal | `3.25` |
| `date` | Date | `2026-11-03` |
| `time` | Date and time | `2026-11-03 08:12:00` |
| `y/n` | True/false | `true`, `false` |

Detection is careful: a column of codes such as `000123` stays text, so the
leading zeros survive. A value that doesn't fit its column's type is kept as
it is and drawn in red, and the status bar counts them, so a stray `n/a` in
a column of weights is easy to find.

**Change Type…** (in a column's menu, or **Table > Columns**) sets a
column's type. For dates written another way, give a **date pattern**:

<p><img class="screenshot" src="images/table-change-type.png" alt="The Change Type dialog: column Born, type Date, date pattern DD/MM/YYYY, with a preview where the dates now read 1984-03-02 and so on"></p>

| Token | Means |
| --- | --- |
| `YYYY` / `YY` | year, four or two digits (`YY` 00–49 is 20xx, 50–99 is 19xx) |
| `MM` / `M` | month number |
| `MMM` / `MMMM` | month name, short or long (`Nov`, `November`) |
| `DD` / `D` | day of the month |
| `HH` / `H` | hour, 0–23 |
| `hh` / `h` | hour, 1–12, with `A` |
| `mm` | minutes |
| `ss` | seconds |
| `SSS` | fractions of a second (read and dropped) |
| `A` | AM or PM |

Anything else must appear as written: `DD/MM/YYYY` reads `02/03/1984`,
`YYYYMMDD` reads `19840302`, `D MMM YYYY h:mm A` reads `3 Nov 2026 8:12 PM`.
Numbers can have fewer digits when a separator follows, so `D/M/YYYY`
reads `2/3/1984` too. Dates are stored as `YYYY-MM-DD` and date-times as
`YYYY-MM-DD HH:MM:SS`, so they sort properly and export cleanly.

For numbers, **Decimal comma** reads `1.234,5` as 1234.5. Thousands
separators are dropped either way.

## Changing the table

Click a column's name for its menu, or right-click it. The same commands are
in the **Table** menu, on the ribbon's **Table** tab (which appears while a
table is showing), in the [command palette](tour.md) and in the right-click
menu of a cell.

| Group | Commands |
| --- | --- |
| Columns | Rename, Change Type, Duplicate, Move Left and Right, Remove, Choose Columns (tick the ones to keep and drag them into order) |
| Values | Replace Values (in one column or all; whole cell, match case, regular expression), and Text: Trim spaces, Collapse spaces, UPPER CASE, lower case, Title Case |
| Rows | Sort Ascending, Sort Descending, Sort… (several columns), Filter Rows…, Remove Duplicates (rows that match on the columns you pick; the first is kept), Remove Empty Rows, Remove Row |

Most commands open a small dialog with a preview of the result before you
press **OK**. Empty cells always sort last.

**Filter Rows…** keeps the rows where a column *is*, *is not*, *contains*,
*starts with*, *is empty*, *is greater than* and so on, or *is one of* a
list of values you tick (with how often each appears). Right-click a cell
for **Keep only "North"** or **Hide "North"** in one click. Filters show as
chips above the grid; **×** on a chip removes that filter.

### Editing cells

Double-click a cell, or select it and press <kbd>Enter</kbd> or
<kbd>F2</kbd>, to type a new value; <kbd>Delete</kbd> clears it. A cell edit
is a step too, but it picks its row by number, so it is marked **by row
number**: on next week's data, row 12 may be someone else. Prefer **Replace
Values** or a filter in steps you want to replay.

### Finding

The **Find in table** box above the grid (<kbd>Ctrl+F</kbd>) highlights
matching cells; <kbd>Enter</kbd> jumps to the next one. **Column
Statistics** shows how many cells are filled, empty and distinct, how many
don't fit the type, the smallest and largest values and the most common
ones.

## The Steps panel

The first step, **Source**, reads the data; each step after it changes the
table. In the panel you can:

- **Click** a step to see the table as it was after that step. A banner
  says so; **Show the result** goes back to the end. A new step is added
  after the one you're looking at.
- **Untick** a step to switch it off and see the table without it.
- Move it up or down, change it (pencil, or double-click) or delete it.

Renaming a column renames it in the later steps too. If a step can't run any
more, for example because an earlier change removed its column, it says why
in red and the steps after it are skipped; the table shows what came before.

<kbd>Ctrl+Z</kbd> and <kbd>Ctrl+Y</kbd> undo and redo changes to the steps.

## Exporting

**Export** writes the table as (<kbd>Ctrl+S</kbd> opens it too, in a workbook
tab or a table tab with changes):

| Format | Options |
| --- | --- |
| CSV / delimited text | Delimiter, quote values when needed, always or never, write the column names, Windows line endings |
| Excel workbook (.xlsx) | Sheet name. Numbers, dates and true/false are written as real Excel values, with a bold header row and fitted column widths |
| JSON | A list of objects; indented or not, numbers and true/false as JSON values, empty cells as `null` |
| JSON Lines | One object per line |
| Markdown table | |

The result goes to a **new tab**, a **file** or the **clipboard** (a
workbook can only go to a file). A preview shows the first rows. Saving never
writes over the workbook or file you opened: a table tab with changes asks
where to export instead, and closing it asks first, like any unsaved tab.

## Saving the steps as a recipe

**Save as Recipe…** saves how the data is read, every step and the export as
a recipe with one **Table** step. Run it on the next file from the
[Recipes window](recipes.md), or from the command line, where `--out`
writes the result to a file (needed for an Excel workbook):

```
ettin-edit --recipe "Ward report" export.csv > report.json
ettin-edit --recipe "Ward report" export.xlsx --out report.xlsx
```

A recipe made from a workbook reads workbooks, and one made from CSV reads
CSV: the Source step is part of the recipe.

To change a saved table recipe, open some data as a table, make the steps
again, and save under the same name: the recipe's steps are replaced.

Open tables and their steps are kept with the [session](files.md#you-wont-lose-your-work),
so they are still there after a restart.
