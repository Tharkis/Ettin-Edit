# Translations

Ettin Edit's menus, dialogs and messages can be shown in another language.
It comes with English, German (Deutsch), French (Français) and Spanish
(Español), and you can add more languages or change any translation with a
text file.

> The German, French and Spanish translations are machine-made starting
> points. They haven't been checked by native speakers yet, so some wording
> will be clumsy or wrong. Corrections are very welcome: see
> [Improving a translation](#improving-a-translation).

## Choosing the language

Open **Settings > Preferences…** (<kbd>Ctrl+,</kbd>). **Interface language**
is at the top of **Files and Startup**.

- **Same as the system** (the default) follows your system's language. On
  Linux that's the `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` or `LANG` setting; on
  Windows it's the language in your region settings. A regional language uses its base language when there's no
  translation for the region, so `de_AT` gets German. A language with no
  translation gets English.
- Or pick a language from the list.

The window changes language at once; there's nothing to restart.

## What stays in English

- The [user manual](introduction.md), including the in-app copy.
- Names of programming languages and themes, and **Plain Text**.
- Messages from the text tools' parsers, such as a JSON or XML syntax error
  and its line number, and messages from XPath, XSD, XSLT and SQL.
- Messages from plugins, unless the plugin translates them itself (see
  [Writing plugins](plugin-guide.md#translating-your-plugin)).
- Templates Ettin Edit writes for you, such as a new language file.

Right-to-left languages such as Arabic and Hebrew can't be shown correctly
yet, because the toolkit Ettin Edit is drawn with doesn't lay out
right-to-left text.

## The translations folder

**Open Translations Folder**, next to the language list in Preferences,
opens the `translations` folder in your [settings folder](folders.md),
creating it if needed. It also puts `english-source.ftl.txt` there: every
message in English, with notes, kept up to date by Ettin Edit. Copy from it;
don't edit it, because it is replaced each time.

Each file in the folder is named after its language code, such as `pt.ftl`
for Portuguese, `pt-BR.ftl` for Brazilian Portuguese, or `de.ftl` for
German. After adding or changing a file, press **Reload** next to the
language list.

### Adding a language

1. Copy `english-source.ftl.txt` to a new file named after the language,
   such as `it.ftl`.
2. Change `language-name = English` to the language's own name, such as
   `language-name = Italiano`. That's the name shown in the list.
3. Translate the text after each `=`. Leave the ids before the `=`, and
   anything in `{ $braces }`, as they are.
4. Press **Reload** in Preferences and pick the language.

You don't have to translate everything at once. Any message the file
doesn't have is shown in English.

### Improving a translation

A file for a language Ettin Edit already has is used on top of the built-in
translation, message by message. So to fix a few German messages, make a
`de.ftl` that holds only those messages:

```
save-as = Speichern als …
find-hint = Suchen
```

To share your fixes with everyone, send them to the project on
[GitHub](https://github.com/Tharkis/Ettin-Edit). The built-in files are in
`assets/i18n` in the source.

### The file format

The files use [Fluent](https://projectfluent.org/fluent/guide/), a format
made for translating software. Most lines are just `id = text`. A few
things to know:

- Text in `{ $braces }` is filled in by Ettin Edit, such as a file name or a
  count. You can move it within the sentence, but keep its name.
- A long message can continue on the next lines, indented by spaces.
- Lines starting with `#` are notes and are ignored.
- Messages that depend on a number choose between forms:

  ```
  differences-count = { $count ->
      [one] { $count } difference
     *[other] { $count } differences
  }
  ```

  The names in brackets are the language's plural forms (`zero`, `one`,
  `two`, `few`, `many` and `other`), and the one marked `*` is used when no
  other fits. Use the forms your language needs; Polish, for example, uses
  `one`, `few`, `many` and `other`.

If a file has a mistake, Ettin Edit shows a message naming the file and the
problem when it loads it, and still uses the rest of the file.
