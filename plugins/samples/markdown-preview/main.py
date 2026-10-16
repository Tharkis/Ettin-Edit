"""Markdown Preview: a sample Ettin Edit plugin in Python.

Shows the Markdown tab you are editing, rendered, in a side panel. The
preview follows the tab that has focus and updates a moment after you stop
typing. Images show, from files next to the Markdown file or from the web,
and the preview scrolls along with the text unless you untick "Scroll with
the text". Links to other files open them; web links open in the browser.
Read it alongside the plugin guide (Plugins > Plugin Guide).
"""

import os

import ettin
from ettin import ui

from preview import is_markdown, split_front_matter

# Bigger tabs aren't previewed, so typing stays smooth.
MAX_CHARS = 2_000_000
PANEL = "preview"

state = {"shown": False, "doc": None, "sync": True}


def render(doc=None):
    if not state["shown"]:
        return
    doc = doc or ettin.active()
    previous = state["doc"]
    state["doc"] = doc
    title = "Preview: " + doc.title
    if not is_markdown(doc):
        items = [ui.label("This tab isn't Markdown. Open or switch to a .md file to "
                          "preview it.", weak=True)]
    elif doc.length > MAX_CHARS:
        items = [ui.label("This tab is too big to preview as you type.", weak=True)]
    else:
        text, first_line = split_front_matter(doc.get_text())
        if not text.strip():
            items = [ui.label("Nothing to show yet. Start typing Markdown.", weak=True)]
        else:
            # Images with relative paths are next to the file (none until
            # it's saved). Following the tab keeps the preview level with it.
            base = os.path.dirname(doc.path) if doc.path else None
            follow = doc if state["sync"] else None
            items = [
                ui.checkbox("sync", "Scroll with the text", state["sync"]),
                ui.markdown(text, id="page", base=base, doc=follow, line=first_line),
            ]
    # A different tab starts at the top; the same tab keeps its place.
    other_tab = previous is None or previous.id != doc.id
    ettin.show_panel(PANEL, title, items, scroll_to_top=other_tab)


@ettin.on_panel(PANEL)
def panel(event):
    if event["event"] == "close":
        state["shown"] = False
    elif event["event"] == "link":
        follow(event["value"])
    elif event["item"] == "sync":
        state["sync"] = bool(event["value"])
        render()


def follow(href):
    """Opens a link to another file, relative to the previewed one."""
    doc = state["doc"]
    target = href.split("#", 1)[0]
    if href.startswith("file://"):
        target = href[len("file://"):]
    if not target:
        return
    if not os.path.isabs(target):
        if not doc or not doc.path:
            ettin.message("Save this tab first so links to other files can be found.",
                          "warning")
            return
        target = os.path.join(os.path.dirname(doc.path), target)
    if os.path.exists(target):
        ettin.open_file(os.path.normpath(target))
    else:
        ettin.message("Can't find " + target, "warning")


@ettin.command("Show or Hide Markdown Preview", shortcut="Ctrl+Shift+M",
               icon="markdown-logo")
def toggle():
    state["shown"] = not state["shown"]
    if state["shown"]:
        state["doc"] = None
        render()
    else:
        ettin.close_panel(PANEL)


@ettin.on("activated")
def activated(doc):
    render(doc)


@ettin.on("opened")
def opened(doc):
    render()


@ettin.on("changed")
def changed(doc):
    if state["shown"] and doc.id == ettin.active().id:
        render(doc)


@ettin.on("saved")
def saved(doc):
    # Saving can change the name, and so whether the tab is Markdown.
    if state["shown"] and doc.id == ettin.active().id:
        render(doc)


@ettin.on("closed")
def closed(doc):
    if state["shown"]:
        render()


ettin.run()
