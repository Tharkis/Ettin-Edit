# The Ettin Edit user manual

This folder is the user manual. It is written once, in Markdown, and used in
two places:

- **In the app**, under **Help > User Manual** (Shift+F1). `build.rs` reads
  `src/SUMMARY.md` and builds every page it lists into the program, and
  `src/manual.rs` shows them with a contents list, search and working links.
  It needs no internet connection.
- **On the web** at <https://tharkis.github.io/Ettin-Edit/>, built with
  [mdBook](https://rust-lang.github.io/mdBook/) by
  `.github/workflows/manual.yml` whenever the manual changes on `main`.

`Language > Language Guide` and `Plugins > Plugin Guide` open the
`languages.md` and `plugin-guide.md` pages.

## Keeping it up to date

**Every change to what Ettin Edit does updates the manual in the same pull
request**: a new command, a changed shortcut, a new setting, a renamed menu
item. `FEATURES.md` says *what* exists; the manual says *how to use it*.

`cargo test` checks the parts that are easiest to forget:

- every page in `src/` is listed in `src/SUMMARY.md` and built into the app;
- every link between pages leads to a page and to a heading that exists;
- `src/shortcuts.md` lists every built-in keyboard shortcut. Adding or
  changing a default shortcut in `src/actions.rs` fails this test until the
  table is updated.

The pull request workflow also builds the web version, so a broken
`SUMMARY.md` shows up before merging.

## Adding a page

1. Write `src/<name>.md`, starting with a `# Title`.
2. Add it to `src/SUMMARY.md`.
3. Link to it from related pages, as `[text](<name>.md)` or
   `[text](<name>.md#a-heading)`. Anchors are the heading in lower case, with
   spaces as `-` and punctuation dropped, as mdBook makes them.

## Writing style

- Write for people who use the editor, not for developers. Lead with what to
  do; explain why only when it helps.
- Name menu commands in bold with `>` between levels: **File > Open…**, and
  give the default shortcut as `<kbd>Ctrl+O</kbd>` (one `kbd` per shortcut).
- Use the same words as the app's menus and dialogs.
- Use tables for lists of commands, and `>` quotes for tips. Edda and Ottar
  may comment, sparingly.
- The in-app viewer shows headings, paragraphs, lists, tables, code blocks,
  quotes, links, `**bold**`, `*italic*` and `` `code` ``. Raw HTML blocks
  (such as the images on the welcome and tour pages) only appear on the web,
  so don't put anything the reader needs in them.

## Previewing

```sh
mdbook serve docs/manual --open    # the web version, reloading as you edit
cargo run                          # then Help > User Manual
```
