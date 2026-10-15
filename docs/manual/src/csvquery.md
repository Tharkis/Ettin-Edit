# CsvQuery

CsvQuery runs SQL queries on the CSV file you have open, like the CsvQuery
plugin for Notepad++. Open a `.csv`, `.tsv` or other delimited file and choose
**Tools > CsvQuery > CsvQuery Panel** (<kbd>Ctrl+Alt+Q</kbd>). A panel opens
below the text, and runs `SELECT * FROM this` straight away.

> **Ottar says:** your spreadsheet would like a word. It can wait.

## Tables

- The current tab is always the table **`this`**.
- Every other open tab is a table named after its file, with anything that
  isn't a letter or digit turned into `_`: `orders 2024.csv` is
  `orders_2024`. A tab is only read when a query names it.
- The **Tables** menu in the panel inserts a table's name into the query.

So you can join files:

```sql
SELECT c.name, SUM(o.total) AS spent
FROM this AS o
JOIN customers AS c ON c.id = o.customer_id
GROUP BY c.name
ORDER BY spent DESC
```

## Writing and running queries

Type SQL in the box at the top of the panel and press <kbd>Ctrl+Enter</kbd>,
click **Run**, or use **Tools > CsvQuery > Run Query**. Queries use
[SQLite's SQL](https://www.sqlite.org/lang_select.html), so `WHERE`, `GROUP
BY`, `ORDER BY`, `LIMIT`, joins, sub-queries, window functions and SQLite's
functions all work.

Queries run in the background, so a big file never freezes the editor. The
query runs again by itself when you edit the tab or switch to another one.
**Recent** brings back earlier queries.

`UPDATE`, `DELETE`, `INSERT` and `CREATE TABLE` work too, on a copy of the
data held for the query; **the file itself never changes**. To keep the
result, put it in a new tab (below).

## How the file is read

Ettin Edit works out how the file is laid out, and the bar above the query
box lets you correct it:

- **Delimiter**: comma, semicolon, tab, pipe or space.
- **Quote character**: `"`, `'` or none.
- **Header row**: whether the first row names the columns. Without one, the
  columns are called `col1`, `col2` and so on.
- **Reset** goes back to what was detected.

Each column is stored as integers, decimals or text, whichever is the
narrowest that fits every value. Values with leading zeros, such as zip codes
or ids like `007`, stay text so the zeros aren't lost. Empty fields are
`NULL`. In semicolon-separated files, decimal commas such as `3,50` count as
numbers.

## Results

The **Results** view shows the first 100,000 rows, with how long the query
took.

- Click a column heading to sort by it; click again to reverse.
- Right-click a cell to **Copy Value** or **Copy Row**.
- **Copy** (or **Tools > CsvQuery > Copy Results**) copies every row as
  tab-separated text, which pastes cleanly into a spreadsheet.
- **Open in New Tab** (**Results to New Tab**) writes all the result rows to
  a new tab as CSV.

## Columns

The **Columns** view describes each column of `this`: its type, how many
values are filled, empty and distinct, and the minimum, maximum and average.
Change a column's **Type** there if Ettin Edit picked the wrong one, for
example to sort a column of numbers stored as text.
