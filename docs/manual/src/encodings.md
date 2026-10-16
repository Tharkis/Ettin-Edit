# Encodings and line endings

A text file is just bytes; its *encoding* says which bytes mean which
characters. Today almost everything is UTF-8, but older files, and files
from some Windows programs, use other encodings. Ettin Edit reads and writes
all the common ones.

## How Ettin Edit picks an encoding

When a file opens, Ettin Edit looks for, in order:

1. a byte-order mark (BOM) at the start, which names UTF-8 or UTF-16 for
   certain;
2. text that is valid UTF-8;
3. UTF-16 without a BOM;
4. failing those, the most likely legacy encoding, such as Windows-1252,
   Windows-1251 or Shift_JIS.

The encoding is shown on the status bar, and the file is saved in the same
one, with or without a BOM just as it was. If some bytes don't make sense in
the encoding it chose, Ettin Edit says so and shows them as �. Saving would
replace them with �, so if the text looks wrong, use **Reopen With** (below)
first.

Binary files (with a `00` byte near the start that isn't UTF-16) aren't read
as text at all: they open in [Hex View](view.md#binary-and-very-large-files).

## The Encoding menu

Open it from the menu bar or by clicking the encoding on the status bar.

**Save as** (the top of the menu) changes the encoding the tab will be saved
in. The text you see doesn't change; the next save writes it in the new
encoding. The choices are:

- **UTF-8**, **UTF-8 with BOM**, **UTF-16 LE** and **UTF-16 BE**;
- about 35 older encodings, grouped by region: Western European
  (Windows-1252, ISO-8859-1, ISO-8859-15, Mac Roman…), Central European,
  Cyrillic (including KOI8-R and KOI8-U), Greek, Turkish, Baltic, Nordic,
  Hebrew, Arabic, Vietnamese, Thai, Romanian, Chinese (GBK, GB18030, Big5),
  Japanese (Shift_JIS, EUC-JP, ISO-2022-JP) and Korean (EUC-KR).

If the text contains a character the new encoding can't hold, such as `€`
in ISO-8859-1, saving stops and tells you which character and where. Nothing
is lost.

**Reopen With** reads the file from disk again in the encoding you pick. Use
it when Ettin Edit guessed wrong and the text looks garbled, for example
`Ã©` where there should be `é`. It's only available for saved files, and you
can undo it.

New tabs use the encoding set in **Preferences > New Documents**, UTF-8
unless you change it.

## Line endings

Lines end with `LF` on Linux and macOS, and `CR LF` on Windows; very old Mac
files use `CR`. Ettin Edit detects the ending a file uses and keeps it when
saving, so editing a Windows file on Linux doesn't change every line.

Click the line ending on the status bar, *Unix (LF)*, *Windows (CR LF)* or
*Macintosh (CR)*, to convert the whole document to another. **View > Show
Symbol > Show End of Line** shows each line's ending.

New tabs use the line ending set in **Preferences > New Documents**: your
system's usual one unless you change it.

## Chinese, Japanese and Korean text

Ettin Edit's own fonts don't include Chinese, Japanese or Korean characters,
so it uses a CJK font installed on your system for them, such as Noto Sans
CJK on Linux or Microsoft YaHei on Windows. If those characters show as
boxes, install a CJK font.
