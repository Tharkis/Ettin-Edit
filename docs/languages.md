# Languages in Ettin Edit

Ettin Edit colours code with Sublime Text–style grammars. It ships with the
usual set (C, C++, C#, Java, Python, Rust, Go, JavaScript, HTML, CSS, SQL,
Markdown, JSON, YAML, XML, shell scripts and more), plus its own grammars for:

- TOML
- INI and other key/value files: `.ini`, `.cfg`, `.conf`, `.properties`,
  `.desktop`, `.reg`, `.inf`, `.editorconfig`, `.gitconfig` and systemd units
  (`.service`, `.socket`, `.timer` and the rest)
- TypeScript (`.ts`, `.mts`, `.cts`) and TSX
- PowerShell
- Dockerfile (also `Containerfile` and `Dockerfile.*`)
- HL7 v2 messages (the separators are read from each `MSH` segment)
- Kotlin, Swift, Dart, Zig
- CMake, Nginx, `.env` files
- Protocol Buffers, GraphQL, Terraform / HCL

Pick a language for the current file from the **Language** menu. Ettin Edit
chooses one when a file opens, by its name, its extension and finally its
first line (a shebang such as `#!/usr/bin/env python3`).

You can add your own languages, or replace a built-in one, in two ways:

- write a short `.lang.json` file (no regular expressions needed), or
- drop in a `.sublime-syntax` grammar.

## The languages folder

Put language files in the `languages` folder inside the Ettin Edit settings
folder:

- Linux: `~/.config/EttinEdit/languages`
- Windows: `%APPDATA%\EttinEdit\languages`

Create the folder if it is not there. Ettin Edit reads it when it starts, so
restart Ettin Edit after adding or changing a file.

- A file that cannot be loaded is skipped. Ettin Edit tells you which file it
  was and why when it starts; the other files still load.
- A language with the same name as a built-in one replaces it. For example, a
  file with `"name": "Python"` takes over `.py` files.
- If two of your files use the same name, the one that comes later in
  alphabetical order wins.

## User-defined languages (`.lang.json`)

A `.lang.json` file describes a language by its comments, strings and
keywords. Ettin Edit turns it into a grammar when it loads it. Save the file
as `<anything>.lang.json`, for example `mylang.lang.json`.

Here is a complete example. Keys that start with `_` are notes and are
ignored, so you can keep them as comments:

```json
{
  "_about": "An Ettin Edit user-defined language. Keep it in the languages folder as <something>.lang.json, then restart Ettin Edit. Keys that start with _ are notes and are ignored.",

  "name": "My Language",
  "extensions": ["mylang"],

  "_first_line": "Optional regular expression tried on the first line of files whose extension is unknown, for example a shebang.",
  "first_line": "^#!.*\\bmylang\\b",

  "_case_sensitive": "false makes keywords match in any case (IF, If, if).",
  "case_sensitive": true,

  "line_comment": ["//", "#"],
  "block_comment": [["/*", "*/"]],

  "_strings": "Each entry is [open, close] or [open, close, escape]. Strings end at the end of the line unless multiline_strings is true.",
  "strings": [["\"", "\"", "\\"], ["'", "'", "\\"]],
  "multiline_strings": false,

  "_keywords": "Groups: keyword, type, constant, builtin, modifier, operator, variable, or a scope name such as support.class.",
  "keywords": {
    "keyword": ["if", "then", "else", "elif", "end", "while", "for", "in", "return", "break"],
    "type": ["int", "float", "string", "bool"],
    "constant": ["true", "false", "nil"],
    "builtin": ["print", "len", "open"]
  },

  "operators": ["+", "-", "*", "/", "%", "=", "==", "!=", "<", "<=", ">", ">=", "&&", "||", "!"],
  "numbers": true,

  "_folding": "Bracket pairs that open and close a block. They are coloured as brackets today and are kept for grammar-aware folding.",
  "folding": [["{", "}"]]
}
```

Only `name` is required. Everything else is optional.

| Key | Meaning | Default |
| --- | --- | --- |
| `name` | The name shown in the **Language** menu. | (required) |
| `extensions` | File extensions without the dot, such as `["mylang", "ml2"]`. A single string also works. | none |
| `first_line` | A regular expression tried on the first line of a file whose extension is not known, such as a shebang. | none |
| `case_sensitive` | `false` makes keywords match in any case (`IF`, `If`, `if`). | `true` |
| `line_comment` | Markers that start a comment running to the end of the line. A single string also works. | none |
| `block_comment` | `[open, close]` pairs, such as `[["/*", "*/"]]`. | none |
| `strings` | `[open, close]` or `[open, close, escape]` entries. When the escape is the same as the close (`["'", "'", "'"]`), a doubled quote stays inside the string, as in SQL. | none |
| `multiline_strings` | `true` lets strings run over several lines. Otherwise a string ends at the end of its line, so a missing quote only spoils one line. | `false` |
| `keywords` | Groups of words, coloured by group (see below). Words match whole words only. | none |
| `operators` | Symbols to colour as operators, such as `"+"` or `"=="`. | none |
| `numbers` | Colour numbers (`42`, `3.14`, `1e-9`, `0xFF`). | `true` |
| `folding` | Bracket pairs that open and close a block, such as `[["{", "}"]]`. They are coloured as brackets today. | none |

A key Ettin Edit does not recognise is reported as an error, so a typo such as
`"line_coment"` does not go unnoticed.

### Keyword groups

The group name decides the colour, using the same scopes as the built-in
grammars, so every theme already knows how to colour them:

| Group | Used for | Scope |
| --- | --- | --- |
| `keyword` | control words: `if`, `while`, `return` | `keyword.control` |
| `type` | type names: `int`, `string` | `storage.type` |
| `constant` | `true`, `false`, `null` | `constant.language` |
| `builtin` (or `function`) | built-in functions | `support.function` |
| `modifier` | `public`, `static`, `const` | `storage.modifier` |
| `operator` | word operators: `and`, `or`, `not` | `keyword.operator.word` |
| `variable` | `self`, `this` | `variable.language` |

Plural names (`keywords`, `types` and so on) work too. For anything else, use a
scope name with a dot, such as `"support.class"` or `"entity.name.tag"`.

## Sublime Text grammars (`.sublime-syntax`)

For full control, write a grammar in the
[sublime-syntax format](https://www.sublimetext.com/docs/syntax.html) and save
it in the languages folder with the `.sublime-syntax` extension. Ettin Edit
loads it as it is.

- The grammar's `name` is what the **Language** menu shows. Without one, the
  file name is used.
- `file_extensions` and `first_line_match` decide which files it opens.
- The grammar needs a `main` context, and every context it refers to must
  exist. A grammar that fails these checks is skipped with a message, rather
  than leaving files uncoloured.
- Ettin Edit uses the [syntect](https://github.com/trishume/syntect) engine,
  which supports nearly all of the format. Grammars that need the newest
  Sublime Text features (such as `extends` or `branch_point`) may not load.

Only use grammars whose licence lets you use them.

## Colours

Grammars only label the text (a keyword, a string, a comment). The theme
decides the colours, so a new language follows whichever theme you pick from
the **Theme** menu.
