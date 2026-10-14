"""A small HL7 version 2 parser that remembers where everything is.

parse(text) splits text into segments, fields, repetitions, components and
subcomponents. Every piece is a Node that records its exact start and end
in the text (Python string indexes, so code points), so text[node.start:
node.end] == node.raw always holds.

Pieces are found by a path: (segment index, field, repetition, component,
subcomponent), all numbers from 1 except the segment index, which counts
segments from 0. A path can stop at any level, and the repetition is
always there below a field, even when the field doesn't repeat:
(4, 7, 1, 13) is the 13th component of PV1-7 when segment 4 is PV1.

Nothing here knows about the editor; the plugin glue is in main.py.
"""

import bisect
import re

HEADERS = ("MSH", "FHS", "BHS")
"""Segments that define the delimiters, in their first two fields."""
ENVELOPE = ("FHS", "BHS", "BTS", "FTS")
"""File and batch segments, which belong to no message."""

DEFAULT_ENCODING = "^~\\&"
_LINE = re.compile(r"[^\r\n]+")
# Leading and trailing characters that are never part of a segment:
# blanks and MLLP framing (vertical tab, file separator).
_JUNK = " \t\x0b\x1c\x00\f"


class Delimiters:
    """The separators a message uses. A character that is None isn't used."""

    __slots__ = ("field", "component", "repetition", "escape", "subcomponent", "truncation")

    def __init__(self, field="|", encoding=DEFAULT_ENCODING):
        self.field = field
        enc = encoding or DEFAULT_ENCODING

        def char(i):
            return enc[i] if len(enc) > i else None

        self.component = char(0)
        self.repetition = char(1)
        self.escape = char(2)
        self.subcomponent = char(3)
        self.truncation = char(4)

    @property
    def encoding(self):
        return "".join(c for c in (self.component, self.repetition, self.escape,
                                   self.subcomponent, self.truncation) if c)

    def __repr__(self):
        return "Delimiters(%r, %r)" % (self.field, self.encoding)


class Node:
    """One piece of the text: a segment, field, repetition, component or
    subcomponent. children holds the next level down, in order."""

    __slots__ = ("kind", "path", "start", "end", "raw", "children", "atomic")

    def __init__(self, kind, path, start, end, raw, atomic=False):
        self.kind = kind
        self.path = path
        self.start = start
        self.end = end
        self.raw = raw
        self.children = []
        # MSH-1 and MSH-2 hold the delimiters themselves: never split them.
        self.atomic = atomic

    def child(self, n):
        """The nth child, from 1; None if there isn't one."""
        return self.children[n - 1] if 1 <= n <= len(self.children) else None

    def __repr__(self):
        return "<%s %s %d-%d %r>" % (self.kind, path_id(self.path), self.start, self.end, self.raw)


class Segment(Node):
    """A segment. Its fields are split off the text the first time they're
    needed, so a big file parses quickly."""

    __slots__ = ("name", "delims", "message", "line", "_text", "_fields", "_header")

    def __init__(self, text, index, start, end, delims, header):
        self.kind = "segment"
        self.path = (index,)
        self.start = start
        self.end = end
        self.raw = text[start:end]
        self.atomic = False
        self.delims = delims
        self.message = None
        self.line = 1
        self._text = text
        self._fields = None
        self._header = header
        if header:
            self.name = self.raw[:3]
        else:
            cut = self.raw.find(delims.field)
            self.name = self.raw if cut < 0 else self.raw[:cut]

    @property
    def children(self):
        if self._fields is None:
            self._fields = _split_fields(self)
        return self._fields

    def field(self, n):
        return self.child(n)

    def value(self, field, rep=1, comp=None, sub=None):
        """A raw value by position, or "" when it isn't there."""
        node = self.field(field)
        for n in (rep, comp, sub):
            if node is None or n is None:
                break
            node = node.child(n)
        return node.raw if node is not None else ""


class Message:
    """Segments from one MSH to the next. header is the MSH segment, or
    None for segments that come before any MSH."""

    __slots__ = ("index", "header", "segments")

    def __init__(self, index, header):
        self.index = index
        self.header = header
        self.segments = []

    def _msh(self, field, comp=None):
        if self.header is None or self.header.name != "MSH":
            return ""
        return self.header.value(field, 1, comp) if comp else self.header.value(field)

    @property
    def type(self):
        """MSH-9 without the message structure, like "ORM^O01"."""
        h = self.header
        if h is None or h.name != "MSH":
            return ""
        code = self._msh(9, 1)
        event = self._msh(9, 2)
        sep = h.delims.component or "^"
        return code + sep + event if event else code

    @property
    def structure(self):
        return self._msh(9, 3)

    @property
    def version(self):
        return self._msh(12, 1)

    @property
    def control_id(self):
        return self._msh(10)

    @property
    def start(self):
        return self.segments[0].start if self.segments else 0

    @property
    def end(self):
        return self.segments[-1].end if self.segments else 0


class Parsed:
    """The result of parse(): segments in order, and the messages."""

    def __init__(self, text):
        self.text = text
        self.segments = []
        self.messages = []
        # Where each segment starts, for finding one by offset.
        self._starts = []
        # Where each line starts, for line and column numbers.
        self._lines = [0] + [m.end() for m in re.finditer(r"\r\n|\r|\n", text)]

    @property
    def grouped(self):
        """True when the tree should group segments by message: more than
        one message, or file and batch segments around them."""
        return len(self.messages) > 1 or any(s.name in ENVELOPE for s in self.segments)

    def node(self, path):
        """The node at a path, or None."""
        if not path or not 0 <= path[0] < len(self.segments):
            return None
        node = self.segments[path[0]]
        for n in path[1:]:
            node = node.child(n)
            if node is None:
                return None
        return node

    def segment_at(self, offset):
        """The segment that contains offset (its end counts), or None."""
        i = bisect.bisect_right(self._starts, offset) - 1
        if i >= 0 and offset <= self.segments[i].end:
            return self.segments[i]
        return None

    def locate(self, offset):
        """The path of the deepest piece at offset, or None. Where two
        pieces touch (MSH-1 and MSH-2), the later one wins."""
        seg = self.segment_at(offset)
        if seg is None:
            return None
        node = seg
        while node.children:
            starts = [c.start for c in node.children]
            i = bisect.bisect_right(starts, offset) - 1
            if i < 0 or offset > node.children[i].end:
                break
            node = node.children[i]
        return node.path

    def visible(self, path):
        """The nearest path at or above `path` that the outline shows."""
        while len(path) > 1 and not self.shown(path):
            path = path[:-1]
        return path

    def shown(self, path):
        """Whether the outline shows the piece at path: every segment, every
        non-empty field, and below that every non-empty piece that has
        siblings (a field that doesn't repeat shows its components directly,
        and a value without separators shows nothing below it)."""
        node = self.node(path)
        if node is None:
            return False
        if len(path) == 1:
            return True
        if not node.raw:
            return False
        if len(path) == 2:
            return True
        return len(self.node(path[:-1]).children) > 1

    def outline_children(self, node):
        """The pieces the outline shows under a node."""
        if node.kind == "segment":
            return [f for f in node.children if f.raw]
        while not node.atomic and len(node.children) == 1:
            node = node.children[0]
        if node.atomic:
            return []
        return [c for c in node.children if c.raw]

    def line_col(self, offset):
        """(line, column), both from 1."""
        i = bisect.bisect_right(self._lines, offset) - 1
        return i + 1, offset - self._lines[i] + 1

    def message_of(self, segment):
        return self.messages[segment.message] if segment.message is not None else None


def _spans(text, start, end, sep):
    """(start, end) of each piece of text[start:end] split on sep."""
    if not sep:
        return [(start, end)]
    out = []
    i = start
    while True:
        j = text.find(sep, i, end)
        if j < 0:
            out.append((i, end))
            return out
        out.append((i, j))
        i = j + 1


def _header_delims(text, start, end):
    """Delimiters from an MSH, FHS or BHS segment at text[start:end]."""
    fs = text[start + 3]
    enc_end = text.find(fs, start + 4, end)
    if enc_end < 0:
        enc_end = end
    return Delimiters(fs, text[start + 4:enc_end])


def _field(text, path, start, end, d, atomic=False):
    f = Node("field", path, start, end, text[start:end], atomic)
    for r, (rs, re_) in enumerate(_spans(text, start, end, None if atomic else d.repetition), 1):
        rep = Node("repetition", path + (r,), rs, re_, text[rs:re_], atomic)
        f.children.append(rep)
        for c, (cs, ce) in enumerate(_spans(text, rs, re_, None if atomic else d.component), 1):
            comp = Node("component", rep.path + (c,), cs, ce, text[cs:ce], atomic)
            rep.children.append(comp)
            for s, (ss, se) in enumerate(_spans(text, cs, ce, None if atomic else d.subcomponent), 1):
                comp.children.append(
                    Node("subcomponent", comp.path + (s,), ss, se, text[ss:se], atomic))
    return f


def _split_fields(seg):
    text, d, index = seg._text, seg.delims, seg.path[0]
    parts = _spans(text, seg.start, seg.end, d.field)
    if seg._header:
        # Field 1 is the field separator itself, field 2 the encoding
        # characters; the rest follow the usual numbering, shifted by one.
        fields = [_field(text, (index, 1), seg.start + 3, seg.start + 4, d, True)]
        for n, (fs, fe) in enumerate(parts[1:], 2):
            fields.append(_field(text, (index, n), fs, fe, d, n == 2))
        return fields
    return [_field(text, (index, n), fs, fe, d) for n, (fs, fe) in enumerate(parts[1:], 1)]


def parse(text):
    """Parses HL7 v2 text: segments end with CR, LF or CR LF, and each MSH
    starts a new message. Never fails; odd lines become odd segments."""
    doc = Parsed(text)
    delims = Delimiters()
    message = None
    for m in _LINE.finditer(text):
        start, end = m.span()
        while start < end and text[start] in _JUNK:
            start += 1
        while end > start and text[end - 1] in _JUNK:
            end -= 1
        if start == end:
            continue
        index = len(doc.segments)
        head = text[start:start + 3]
        is_header = head in HEADERS and end - start > 3
        if is_header:
            delims = _header_delims(text, start, end)
        seg = Segment(text, index, start, end, delims, is_header)
        seg.line = doc.line_col(start)[0]
        if seg.name == "MSH":
            message = Message(len(doc.messages), seg)
            doc.messages.append(message)
        elif seg.name in ENVELOPE:
            message = None
        elif message is None:
            message = Message(len(doc.messages), None)
            doc.messages.append(message)
        if seg.name in ENVELOPE:
            seg.message = None
        else:
            seg.message = message.index
            message.segments.append(seg)
        doc.segments.append(seg)
        doc._starts.append(start)
    return doc


# ----------------------------------------------------------------------
# Escape sequences

_FORMATTING = {".br": "\n", "H": "", "N": "", ".fi": "", ".nf": "", ".ce": "\n"}


def unescape(value, delims):
    """The value with HL7 escape sequences turned into the characters they
    stand for: \\F\\ \\S\\ \\T\\ \\R\\ \\E\\ \\P\\, \\Xhh..\\ (hex bytes) and
    the formatting commands \\.br\\, \\.sp n\\, \\.sk n\\ and friends.
    Sequences it doesn't know, such as \\Zxx\\, are kept as they are."""
    esc = delims.escape
    if not esc or esc not in value:
        return value
    out = []
    i = 0
    n = len(value)
    while i < n:
        c = value[i]
        if c != esc:
            out.append(c)
            i += 1
            continue
        j = value.find(esc, i + 1)
        if j < 0:
            out.append(value[i:])
            break
        meaning = _escape(value[i + 1:j], delims)
        out.append(value[i:j + 1] if meaning is None else meaning)
        i = j + 1
    return "".join(out)


def _escape(seq, d):
    simple = {
        "F": d.field, "S": d.component, "T": d.subcomponent,
        "R": d.repetition, "E": d.escape, "P": d.truncation or "#",
    }
    if seq in simple:
        return simple[seq] or ""
    if seq in _FORMATTING:
        return _FORMATTING[seq]
    if seq.startswith("X") and len(seq) > 1:
        hexa = seq[1:]
        if len(hexa) % 2 or not re.fullmatch(r"[0-9A-Fa-f]+", hexa):
            return None
        data = bytes.fromhex(hexa)
        try:
            return data.decode("utf-8")
        except UnicodeDecodeError:
            return data.decode("latin-1")
    word, _, arg = seq.partition(" ")
    if word in (".sp", ".sk", ".in", ".ti"):
        try:
            count = int(arg) if arg.strip() else 1
        except ValueError:
            return None
        if word == ".sp":
            return "\n" * max(count, 1)
        if word == ".sk":
            return " " * max(count, 0)
        return ""
    if seq[:1] in ("C", "M") and len(seq) > 1:
        # A character set switch: nothing to show.
        return ""
    return None


# ----------------------------------------------------------------------
# Names and labels


def path_id(path):
    """(4, 7, 1, 13) -> "4.7.1.13"."""
    return ".".join(str(n) for n in path)


def parse_id(text):
    """"4.7.1.13" -> (4, 7, 1, 13); None if it isn't a path."""
    try:
        path = tuple(int(p) for p in text.split("."))
    except (ValueError, AttributeError):
        return None
    return path if path and all(n >= 0 for n in path) and all(n >= 1 for n in path[1:]) else None


def repeats(doc, path):
    """Whether the field on this path has more than one repetition."""
    f = doc.node(path[:2]) if len(path) >= 2 else None
    return f is not None and len(f.children) > 1


def designation(doc, path, style="."):
    """A name for a piece, like "PV1-7.1" or "PID-3[2].4.1". style "-"
    gives the outline's form, "PV1-7-1". The repetition is only written
    when the field repeats."""
    seg = doc.segments[path[0]]
    text = seg.name
    if len(path) >= 2:
        text += "-%d" % path[1]
        if len(path) >= 3 and repeats(doc, path):
            text += "[%d]" % path[2]
        for n in path[3:]:
            text += "%s%d" % (style, n)
    return text


def shorten(text, limit=120):
    """One line of at most `limit` characters, ending in … when cut."""
    text = text.replace("\r", " ").replace("\n", " ")
    return text if len(text) <= limit else text[:limit - 1] + "…"


def looks_like_hl7(text, path=None):
    """Whether a document is probably HL7 v2: it starts with MSH|, FHS| or
    BHS| (after blanks or MLLP framing), or its file name ends in .hl7."""
    if path and str(path).lower().endswith(".hl7"):
        return True
    head = text.lstrip(_JUNK + "\r\n")[:4]
    return head in ("MSH|", "FHS|", "BHS|")
