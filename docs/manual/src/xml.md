# XML Tools

**Tools > XML Tools** covers what the XML Tools plugin does for Notepad++,
built in. Like the other text tools, the formatting commands work on the
selection or on the whole document, as one undo step.

## Formatting

| Command | Shortcut | What it does |
| --- | --- | --- |
| **Pretty Print** | <kbd>Ctrl+Alt+Shift+B</kbd> | Indents the XML, one element per line |
| **Pretty Print, Attributes on New Lines** | | The same, with each attribute on a line of its own |
| **Linearize** | <kbd>Ctrl+Alt+Shift+L</kbd> | Removes the indentation and line breaks between elements |

Text inside elements, comments, CDATA sections and processing instructions
is kept as it is.

## Checking XML

- **Check Syntax** (<kbd>Ctrl+Alt+Shift+M</kbd>) checks that the document is
  well-formed: every tag closed, attributes quoted, one root element. A
  mistake is shown with its line and column, and the caret moves there.
- **Check Syntax When Saving** runs that check each time you save an XML
  file, and warns you (the file is still saved). Also in
  [Preferences](preferences.md) under **Files and Startup**.

## Validating against a schema

**Validate** checks the document against an XSD schema or a DTD:

- the XSD named in the root element's `xsi:schemaLocation` or
  `xsi:noNamespaceSchemaLocation`, or
- the DTD in the document's `<!DOCTYPE>`, or
- the schema you last used for that file.

If there isn't one, or to use another, choose **Validate Against…** and pick
the `.xsd` or `.dtd` file. Ettin Edit remembers your choice for that file.

Problems are listed in the [results panel](search.md#the-results-panel);
click one to jump to its line. A valid document gets a short notice saying
so.

Not supported: `xs:key`, `xs:keyref` and `xs:unique`, `xs:redefine`, and
schemas that have to be downloaded from the internet (keep a copy next to
your file instead).

## XSL Transform

**XSL Transform…** asks for an XSLT 1.0 stylesheet and runs it over the
document. The output opens in a new tab, coloured as HTML or XML when that's
what it is. Messages from `xsl:message` are shown in a notice.

Not supported: `xsl:import`, `xsl:include` and the `document()` function.

## XPath

**XPath Panel** (<kbd>Ctrl+Alt+Shift+E</kbd>) opens a panel below the text.
Type an XPath 1.0 expression and press <kbd>Enter</kbd> or **Evaluate**:

- If the result is a set of nodes, each is listed; click one to jump to it
  in the text. **Copy Results** copies them all.
- Otherwise the panel shows the value, such as a number from `count(//item)`
  or the text from `string(//title)`.
- **Recent** brings back earlier expressions.

Every XPath 1.0 function is available, plus `ends-with()`, `lower-case()` and
`upper-case()` from XPath 2.0. Namespace prefixes declared in the document
work in expressions, and a name without a prefix also matches elements in the
default namespace, so `//book` finds `<book>` even in an XHTML or Atom file.

### The current XPath

In an XML file the status bar shows where the caret is, such as
`/catalog/book[3]/title`. Click it, or use **Copy Current XPath**
(<kbd>Ctrl+Alt+Shift+P</kbd>), to copy it.

## Escaping

- **Escape Special Characters** turns `<`, `>`, `&`, `"` and `'` into
  `&lt;`, `&gt;`, `&amp;`, `&quot;` and `&apos;`.
- **Unescape Special Characters** turns them back.

## Closing tags

**Close Tags Automatically** adds the closing tag as you type an opening one
in XML and HTML. Empty HTML elements such as `<br>` and `<img>` don't get
one.
