"""HL7 Viewer: browse and edit HL7 version 2 messages. A sample Ettin Edit
plugin in Python.

The right panel shows the message as a tree: segments, then fields,
repetitions, components and subcomponents (empty ones are hidden). The
bottom panel inspects the piece that is clicked in the tree or under the
caret: its name, data type, optionality and so on from an HL7 v2.5.1
dictionary, and a box to change its value.

The parser is in hl7.py and the dictionary in hl7_defs.py; neither knows
about the editor, so they can be tested on their own (test_hl7.py).
"""

import os

import ettin
from ettin import ui

import hl7
import hl7_defs as defs

# Bigger tabs are skipped, so typing stays smooth.
MAX_CHARS = 3_000_000
# The tree lists at most this many segments.
MAX_TREE_SEGMENTS = 500
# A tree with more nodes than this doesn't follow the caret: sending it
# again on every move would be slow. The inspector still follows.
MAX_SYNCED_NODES = 4000
TREE = "hl7-tree"
INSPECTOR = "hl7-inspector"

state = {
    "shown": False,        # The user turned the viewer on.
    "inspector": True,     # The user hasn't closed the inspector.
    "doc": None,           # Id of the document that was parsed.
    "title": "",           # Its tab title.
    "parsed": None,        # hl7.Parsed for it.
    "nodes": [],           # Its tree, built once per parse.
    "header": [],          # Panel items above the tree.
    "selected": None,      # Id of the inspected node: a path id, or "m<n>".
    "tree_sent": None,     # The selection the tree panel shows.
    "pin": None,           # Offsets of a range selected from the panels.
    "rows": [],            # Node id for each row of the inspector's parts table.
    "sync": True,          # The tree is small enough to follow the caret.
}


# ----------------------------------------------------------------------
# Looking things up


def caret_offset(doc):
    """The main caret's offset, from the document info the event carried
    (its caret has line, column and offset)."""
    offset = getattr(doc, "caret_offset", None)
    if offset is None:
        offset = (getattr(doc, "_info", {}).get("caret") or {}).get("offset")
    if offset is None:
        offset = doc.selections()[0].head
    return offset


def field_type(seg, fdef):
    """A field's data type. OBX-5 takes its type from OBX-2."""
    if seg.name == "OBX" and fdef.number == 5:
        value_type = seg.value(2).strip()
        if value_type and defs.datatype(value_type).known:
            return value_type
    return fdef.type


def describe(parsed, path):
    """What the dictionary says about the piece at path, as a dict:
    name, type, field (FieldDef), field_type, table."""
    seg = parsed.segments[path[0]]
    if len(path) == 1:
        sdef = defs.segment(seg.name)
        return {"name": sdef.description, "type": "", "field": None, "field_type": "", "table": None}
    fdef = defs.field(seg.name, path[1])
    ftype = field_type(seg, fdef)
    info = {"name": fdef.name, "type": ftype, "field": fdef, "field_type": ftype,
            "table": fdef.table}
    type_code = ftype
    for n in path[3:]:
        cdef = defs.component(type_code, n)
        info.update(name=cdef.name, type=cdef.type, table=cdef.table)
        type_code = cdef.type
    return info


def message_summary(msg):
    parts = [msg.type or "(no MSH-9)"]
    if msg.version:
        parts.append("HL7 v" + msg.version)
    return " · ".join(parts)


# ----------------------------------------------------------------------
# The tree


def build_tree(parsed):
    """The panel's tree nodes, the items to show above it, and how many
    nodes there are."""
    segments = parsed.segments[:MAX_TREE_SEGMENTS]
    count = [0]

    def piece(node, seg):
        count[0] += 1
        path = node.path
        children = [piece(c, seg) for c in parsed.outline_children(node)]
        label = "%s: %s" % (hl7.designation(parsed, path, "-"), hl7.shorten(node.raw, 100))
        return ui.node(label, hl7.path_id(path), describe(parsed, path)["name"], children)

    def segment_node(seg):
        children = [piece(f, seg) for f in parsed.outline_children(seg)]
        return ui.node(hl7.shorten(seg.raw, 120), hl7.path_id(seg.path),
                       defs.segment(seg.name).description, children)

    if parsed.grouped:
        nodes = []
        groups = {}
        for seg in segments:
            if seg.message is None:
                nodes.append(segment_node(seg))
                continue
            group = groups.get(seg.message)
            if group is None:
                msg = parsed.messages[seg.message]
                detail = "%d segments" % len(msg.segments)
                if msg.control_id:
                    detail = "control ID %s · %s" % (msg.control_id, detail)
                group = ui.node("Message %d: %s" % (msg.index + 1, msg.type or "(no MSH)"),
                                "m%d" % msg.index, detail, [],
                                True if len(parsed.messages) == 1 else None)
                groups[seg.message] = group
                nodes.append(group)
            group["children"].append(segment_node(seg))
    else:
        nodes = [segment_node(seg) for seg in segments]

    header = []
    messages = [m for m in parsed.messages if m.header is not None]
    if len(messages) == 1:
        header.append(ui.label(message_summary(messages[0])))
    elif messages:
        types = sorted({m.type for m in messages if m.type})
        versions = sorted({m.version for m in messages if m.version})
        text = "%d messages" % len(messages)
        if types:
            text += " · " + ", ".join(types[:4]) + (" …" if len(types) > 4 else "")
        if versions:
            text += " · HL7 v" + ", ".join(versions)
        header.append(ui.label(text))
    else:
        header.append(ui.label("No MSH segment found", weak=True))
    segs = "%d segments" % len(parsed.segments)
    if len(parsed.segments) > MAX_TREE_SEGMENTS:
        segs += "; the tree lists the first %d" % MAX_TREE_SEGMENTS
    if count[0] > MAX_SYNCED_NODES:
        segs += "; the tree doesn't follow the caret in a file this big"
    header.append(ui.label(segs + ". Names follow HL7 v%s." % defs.VERSION, weak=True))
    return nodes, header, count[0]


def tree_selection():
    """The tree node to highlight for the current selection, if listed."""
    sel = state["selected"]
    parsed = state["parsed"]
    if sel is None or parsed is None:
        return None
    if sel.startswith("m"):
        return sel
    path = hl7.parse_id(sel)
    return sel if path and path[0] < MAX_TREE_SEGMENTS else None


def show_tree():
    selected = tree_selection()
    state["tree_sent"] = selected
    ettin.show_panel(TREE, "HL7 Viewer", [ui.label(state["title"], weak=True)] + state["header"] + [
        ui.tree("tree", state["nodes"], selected=selected),
    ])


def show_message(doc_title, text):
    """The tree panel with only a message in it; no inspector."""
    state.update(doc=None, title=doc_title, parsed=None, nodes=[], header=[], selected=None,
                 tree_sent=None)
    ettin.show_panel(TREE, "HL7 Viewer", [
        ui.label(doc_title, weak=True),
        ui.label(text),
    ])
    ettin.close_panel(INSPECTOR)


# ----------------------------------------------------------------------
# The inspector


def props_table(rows):
    return ui.table("props", ["Property", "Value"], [[k, v] for k, v in rows if v not in (None, "")])


def inspect_message(parsed, index):
    msg = parsed.messages[index]
    h = msg.header
    items = [ui.heading("Message %d: %s" % (index + 1, msg.type or "(no MSH)"))]
    rows = [
        ("Message type", msg.type),
        ("Message structure", msg.structure),
        ("Version", msg.version),
        ("Control ID", msg.control_id),
        ("Sent", h.value(7) if h else ""),
        ("From", " / ".join(v for v in (h.value(3), h.value(4)) if v) if h else ""),
        ("To", " / ".join(v for v in (h.value(5), h.value(6)) if v) if h else ""),
        ("Segments", str(len(msg.segments))),
        ("Lines", "%d to %d" % (msg.segments[0].line, msg.segments[-1].line) if msg.segments else ""),
    ]
    items.append(props_table(rows))
    state["rows"] = [hl7.path_id(s.path) for s in msg.segments]
    items.append(ui.heading("Segments"))
    items.append(ui.table("parts", ["#", "Segment", "Description"], [
        [i + 1, s.name, defs.segment(s.name).description] for i, s in enumerate(msg.segments)
    ]))
    return items


def inspect_segment(parsed, seg):
    sdef = defs.segment(seg.name)
    msg = parsed.message_of(seg)
    same = [s for s in (msg.segments if msg else parsed.segments) if s.name == seg.name]
    rows = [
        ("Segment", seg.name),
        ("Description", sdef.description),
        ("Occurrence", "%d of %d in the message" % (same.index(seg) + 1, len(same)) if len(same) > 1 else ""),
        ("Message", "%d: %s" % (msg.index + 1, msg.type) if msg and parsed.grouped else ""),
        ("Line", str(seg.line)),
        ("Fields", "%d sent, %d defined" % (len(seg.children), len(sdef.fields))),
    ]
    items = [ui.heading("%s — %s" % (seg.name, sdef.description)), props_table(rows)]
    count = max(len(seg.children), len(sdef.fields))
    table, ids = [], []
    for n in range(1, count + 1):
        fdef = defs.field(seg.name, n)
        f = seg.field(n)
        table.append(["%s-%d" % (seg.name, n), fdef.name, field_type(seg, fdef),
                      hl7.shorten(f.raw, 80) if f else ""])
        ids.append(hl7.path_id(f.path) if f is not None and f.raw else None)
    if table:
        items.append(ui.heading("Fields"))
        items.append(ui.table("parts", ["Field", "Name", "Type", "Value"], table))
    state["rows"] = ids
    return items


def inspect_piece(parsed, path):
    node = parsed.node(path)
    seg = parsed.segments[path[0]]
    info = describe(parsed, path)
    fdef = info["field"]
    name = hl7.designation(parsed, path)
    field_name = hl7.designation(parsed, path[:2])
    deeper = len(path) > 3
    rows = [("Designation", name), ("Description", info["name"])]
    if deeper:
        rows.append(("Part of", "%s %s (%s)" % (field_name, fdef.name, info["field_type"] or "?")))
    if len(path) > 4:
        comp = describe(parsed, path[:4])
        rows.append(("Component", "%s %s (%s)" % (
            hl7.designation(parsed, path[:4]), comp["name"], comp["type"] or "?")))
    rows.append(("Data type", defs.type_label(info["type"])))
    suffix = " — of %s" % field_name if deeper else ""
    rows.append(("Optionality", defs.optionality_text(fdef.optionality) + suffix))
    rows.append(("Repeatable", defs.repeat_text(fdef.repeat) + suffix))
    if len(path) >= 3 and hl7.repeats(parsed, path):
        rows.append(("Repetition", "%d of %d" % (path[2], len(parsed.node(path[:2]).children))))
    if fdef.length:
        rows.append(("Max length", "%d%s" % (fdef.length, suffix)))
    if info["table"]:
        rows.append(("Table", "HL7 %s" % info["table"]))
    if fdef.note:
        rows.append(("Note", fdef.note))
    line, col = parsed.line_col(node.start)
    rows.append(("Position", "Line %d, column %d (%d characters)" % (line, col, node.end - node.start)))
    rows.append(("Raw value", node.raw if node.raw else "(empty)"))
    value = node.raw if node.atomic else hl7.unescape(node.raw, seg.delims)
    if value != node.raw:
        rows.append(("Value", value.replace("\n", " ⏎ ")))
    items = [ui.heading("%s — %s" % (name, info["name"])), props_table(rows)]
    if value != node.raw and (len(value) > 60 or "\n" in value):
        # Too long for its table cell: the whole text, with its line breaks.
        items.append(ui.label(value, mono=True))

    # The parts below this one: repetitions, or components of a composite type.
    state["rows"] = []
    if node.atomic:
        return items
    if len(path) == 2 and len(node.children) > 1:
        table = []
        for rep in node.children:
            table.append([rep.path[2], hl7.shorten(rep.raw, 100)])
            state["rows"].append(hl7.path_id(rep.path) if rep.raw else None)
        items.append(ui.heading("Repetitions"))
        items.append(ui.table("parts", ["#", "Value"], table))
        return items
    # The components of a field or repetition, the subcomponents of a component.
    if len(path) == 2:
        parts = node.children[0].children
    elif len(path) in (3, 4):
        parts = node.children
    else:
        return items
    type_code = info["type"]
    tdef = defs.datatype(type_code)
    if not tdef.components and len(parts) < 2:
        return items
    count = max(len(tdef.components), len(parts))
    table = []
    for n in range(1, count + 1):
        cdef = defs.component(type_code, n)
        part = parts[n - 1] if n <= len(parts) else None
        table.append([n, cdef.name, cdef.type, hl7.shorten(part.raw, 80) if part else ""])
        visible = part is not None and part.raw and parsed.shown(part.path)
        state["rows"].append(hl7.path_id(part.path) if visible else None)
    kind = "Subcomponents" if len(path) == 4 else "Components"
    items.append(ui.heading("%s (%s)" % (kind, type_code or "unknown type")))
    items.append(ui.table("parts", ["#", "Name", "Type", "Value"], table))
    return items


def show_inspector():
    if not state["inspector"]:
        return
    parsed = state["parsed"]
    sel = state["selected"]
    if parsed is None or sel is None:
        ettin.show_panel(INSPECTOR, "HL7 Inspector", [
            ui.label("Click a node in the HL7 Viewer, or move the caret in the message.", weak=True),
        ], position="bottom")
        return
    if sel.startswith("m"):
        items = inspect_message(parsed, int(sel[1:]))
    else:
        path = hl7.parse_id(sel)
        if len(path) == 1:
            items = inspect_segment(parsed, parsed.segments[path[0]])
        else:
            items = inspect_piece(parsed, path)
        node = parsed.node(path)
        label = "Raw %s" % ("segment" if len(path) == 1 else "value")
        items += [
            ui.separator(),
            ui.input("value", label + " (press Enter or Apply to change it in the document)",
                     value=node.raw, hint="empty"),
            ui.row(ui.button("apply", "Apply"), ui.button("copy", "Copy Designation")),
        ]
    shown = state.get("inspected")
    state["inspected"] = sel
    ettin.show_panel(INSPECTOR, "HL7 Inspector", items, position="bottom",
                     scroll_to_top=shown != sel)


# ----------------------------------------------------------------------
# Keeping up with the editor


def refresh(doc=None, force=False, keep=None):
    """Parses the active document again if its text changed, and updates
    the panels. The inspector shows the piece at the caret, or the path
    `keep` (or the nearest piece above it) when given."""
    if not state["shown"]:
        return
    doc = doc or ettin.active()
    if doc.length is not None and doc.length > MAX_CHARS:
        show_message(doc.title, "This tab is too big for the HL7 Viewer (over %d MB)."
                     % (MAX_CHARS // 1_000_000))
        return
    head = doc.get_text(0, min(doc.length or 0, 512)) if doc.length else ""
    if not hl7.looks_like_hl7(head, doc.path):
        show_message(doc.title, "This tab doesn't look like an HL7 v2 message. "
                     "The viewer opens messages that start with MSH|, FHS| or BHS|, "
                     "and files ending in .hl7.")
        return
    text = doc.get_text()
    parsed = state["parsed"]
    if not force and parsed is not None and state["doc"] == doc.id and parsed.text == text:
        return
    parsed = hl7.parse(text)
    nodes, header, count = build_tree(parsed)
    state.update(doc=doc.id, title=doc.title, parsed=parsed, nodes=nodes, header=header, pin=None,
                 sync=count <= MAX_SYNCED_NODES)
    if keep:
        while keep and not parsed.shown(keep):
            keep = keep[:-1]
        path = keep
    else:
        path = parsed.locate(caret_offset(doc))
        path = parsed.visible(path) if path else None
    state["selected"] = hl7.path_id(path) if path else None
    show_tree()
    show_inspector()


def select_in_editor(doc, sel):
    """Selects a node's text in the editor."""
    parsed = state["parsed"]
    if sel.startswith("m"):
        msg = parsed.messages[int(sel[1:])]
        start, end = msg.start, msg.end
    else:
        node = parsed.node(hl7.parse_id(sel))
        start, end = node.start, node.end
    # The caret lands at the end of the selection: don't let the caret
    # event that follows pick a deeper node than this one.
    state["pin"] = (start, end)
    doc.select(start, end)


def select(sel, doc=None):
    """Inspects a node. With doc (a click in a panel), also selects its text
    in the editor. The tree highlights it if it doesn't already."""
    state["selected"] = sel
    if doc is not None:
        select_in_editor(doc, sel)
    show_inspector()
    if state["tree_sent"] != tree_selection() and (doc is not None or state["sync"]):
        show_tree()


def current_doc():
    """The active document if it's the one parsed (parsed again first if
    its text changed), else None."""
    if state["parsed"] is None:
        return None
    doc = ettin.active()
    if doc.id != state["doc"]:
        return None
    if doc.length != len(state["parsed"].text):
        refresh(doc)
    return doc if state["parsed"] is not None else None


def valid(sel):
    """Whether a node id names something in the current parse."""
    parsed = state["parsed"]
    if parsed is None or not sel:
        return False
    if sel.startswith("m"):
        return sel[1:].isdigit() and int(sel[1:]) < len(parsed.messages)
    path = hl7.parse_id(sel)
    return path is not None and parsed.node(path) is not None


def apply_edit(new):
    doc = current_doc()
    sel = state["selected"]
    if doc is None or sel is None or sel.startswith("m"):
        return
    path = hl7.parse_id(sel)
    node = state["parsed"].node(path)
    if "\r" in new or "\n" in new:
        ettin.message("A value can't contain a line break: it would start a new segment.", "warning")
        return
    if new == node.raw:
        return
    if doc.read_only:
        ettin.message("This tab is read-only.", "warning")
        return
    if doc.get_text(node.start, node.end) != node.raw:
        ettin.message("The text changed meanwhile; check the value and try again.", "warning")
        refresh(doc, force=True)
        return
    name = hl7.designation(state["parsed"], path)
    doc.replace(node.start, node.end, new)
    # Keep inspecting the same place, and select its new text.
    refresh(doc.refresh(), force=True, keep=path)
    if state["selected"]:
        select_in_editor(doc, state["selected"])
    ettin.message("Changed %s." % name)


def designation_for_caret():
    """The designation of the inspected node, or of the piece at the caret."""
    if state["shown"] and current_doc() is not None and state["selected"]:
        sel = state["selected"]
        if sel.startswith("m"):
            return None
        return hl7.designation(state["parsed"], hl7.parse_id(sel))
    doc = ettin.active()
    parsed = hl7.parse(doc.get_text())
    path = parsed.locate(caret_offset(doc))
    return hl7.designation(parsed, parsed.visible(path)) if path else None


@ettin.command("Show or Hide HL7 Viewer", shortcut="Ctrl+Alt+H", icon="heartbeat")
def toggle():
    state["shown"] = not state["shown"]
    if state["shown"]:
        state["inspector"] = True
        refresh(force=True)
    else:
        state.update(doc=None, parsed=None, selected=None, tree_sent=None)
        ettin.close_panel(TREE)
        ettin.close_panel(INSPECTOR)


@ettin.command("Open Sample HL7 Message")
def open_sample():
    # A copy in a new tab, so editing it leaves the plugin's folder alone.
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sample.hl7")
    with open(path, encoding="utf-8", newline="") as f:
        ettin.new_document(f.read())
    if not state["shown"]:
        toggle()


@ettin.command("Copy HL7 Field Designation")
def copy_designation():
    name = designation_for_caret()
    if not name:
        ettin.message("Put the caret in an HL7 segment first.", "warning")
        return
    ettin.set_clipboard(name)
    ettin.message("Copied %s" % name)


@ettin.on("activated")
def on_activated(doc):
    refresh(doc)


@ettin.on("changed")
def on_changed(doc):
    if state["shown"] and doc.id == ettin.active().id:
        refresh(doc)


@ettin.on("closed")
def on_closed(doc):
    if doc.id == state["doc"]:
        state.update(doc=None, parsed=None, selected=None)


@ettin.on("caret")
def on_caret(doc):
    parsed = state["parsed"]
    if not state["shown"] or parsed is None or doc.id != state["doc"]:
        return
    if doc.length != len(parsed.text):
        # An edit that the "changed" event hasn't brought yet.
        refresh(doc)
        return
    offset = caret_offset(doc)
    pin = state["pin"]
    if pin and offset in pin:
        return
    state["pin"] = None
    path = parsed.locate(offset)
    if path is None:
        return
    sel = hl7.path_id(parsed.visible(path))
    if sel != state["selected"]:
        select(sel)


@ettin.on_panel(TREE)
def on_tree(event):
    if event["event"] == "close":
        state["shown"] = False
        state.update(doc=None, parsed=None, selected=None, tree_sent=None)
        ettin.close_panel(INSPECTOR)
    elif event["item"] == "tree" and event["event"] == "select" and state["parsed"]:
        doc = current_doc()
        sel = event["value"]
        if doc is None or not valid(sel):
            refresh(force=True)
            return
        state["inspector"] = True
        # The tree already highlights the click.
        state["tree_sent"] = sel
        select(sel, doc)


@ettin.on_panel(INSPECTOR)
def on_inspector(event):
    if event["event"] == "close":
        # Comes back on the next click in the tree.
        state["inspector"] = False
        return
    item = event["item"]
    if item == "apply" or (item == "value" and event["event"] == "submit"):
        apply_edit(event["values"].get("value", ""))
    elif item == "copy":
        copy_designation()
    elif item == "parts" and event["event"] == "select":
        rows = state["rows"]
        index = event["value"]
        doc = current_doc()
        if doc is not None and 0 <= index < len(rows) and rows[index] and valid(rows[index]):
            select(rows[index], doc)


ettin.run()
