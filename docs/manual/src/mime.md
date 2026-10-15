# MIME Tools

**Tools > MIME Tools** encodes and decodes text the way the MIME Tools plugin
does for Notepad++. Each command works on the selection (each selection, if
there are several) or on the whole document, and is one undo step. Select
just the part you need, such as one value in a JSON file, so the rest is left
alone.

## Base64

| Command | What it does |
| --- | --- |
| **Base64 Encode** | Encodes the text (as UTF-8) on one line |
| **Base64 Encode with Line Breaks** | The same, with a line break every 76 characters, as in email |
| **Base64 Encode, URL-Safe** | Uses `-` and `_` instead of `+` and `/`, without `=` padding, for URLs and file names |
| **Base64 Decode** | Decodes either kind, with or without padding and line breaks |

Decoding stops with a message if the result isn't text, for example if the
Base64 holds an image.

## URLs

| Command | What it does | `a b/c?d=é` becomes |
| --- | --- | --- |
| **URL Encode (Part of a URL)** | Encodes everything except letters, digits and `-_.~`, for one value in a query string or path | `a%20b%2Fc%3Fd%3D%C3%A9` |
| **URL Encode (Whole URL)** | Keeps `/`, `?`, `=`, `&`, `:` and the other characters that give a URL its structure | `a%20b/c?d=%C3%A9` |
| **URL Decode** | Turns `%XX` back into characters | |

## Quoted-printable

**Quoted-Printable Encode** and **Decode** handle the encoding used in email
bodies, where `=C3=A9` stands for `é` and a line ending in `=` continues on
the next.

## HTML

- **HTML Encode** turns `&`, `<`, `>`, `"` and `'` into entities.
- **HTML Encode, Including Accents and Symbols** also encodes every
  non-ASCII character, by name where it has one (`&eacute;`, `&euro;`) and by
  number otherwise.
- **HTML Decode** turns named and numbered entities back into characters.

## SAML

**SAML Decode** turns a SAML request or response back into readable XML.
Select the Base64 text of a POST, the value of a `SAMLRequest` or
`SAMLResponse` parameter, or a whole redirect URL that contains one. Ettin Edit
URL-decodes it if needed, decodes the Base64, inflates it if it was
compressed, and pretty prints the XML.
