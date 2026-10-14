"""Text Statistics: a sample NixPad plugin in Python.

Shows live counts for the current tab in a side panel and adds three
commands to the Plugins menu. Read it alongside the plugin guide
(Plugins > Plugin Guide); copy the folder to start your own plugin.
"""

import re
from collections import Counter

import nixpad
from nixpad import ui

# Words, keeping "don't" and "well-known" together.
WORD = re.compile(r"[^\W_]+(?:['’-][^\W_]+)*")
# Bigger tabs are skipped, so typing stays smooth.
MAX_CHARS = 5_000_000

state = {"shown": False, "skip_short": False, "top": []}


def word_counts(text, skip_short=False):
    counts = Counter(w.lower() for w in WORD.findall(text))
    if skip_short:
        counts = Counter({w: n for w, n in counts.items() if len(w) >= 4})
    return counts


def render(doc=None):
    if not state["shown"]:
        return
    doc = doc or nixpad.active()
    if doc.length > MAX_CHARS:
        nixpad.show_panel("stats", "Text Statistics", [
            ui.label(doc.title, weak=True),
            ui.label("This tab is too big to count as you type."),
        ])
        return
    text = doc.get_text()
    words = WORD.findall(text)
    lines = text.split("\n")
    counts = word_counts(text, state["skip_short"])
    state["top"] = counts.most_common(15)
    average = sum(len(w) for w in words) / len(words) if words else 0
    rows = [
        ["Lines", len(lines)],
        ["Non-blank lines", sum(1 for line in lines if line.strip())],
        ["Words", len(words)],
        ["Different words", len(word_counts(text))],
        ["Characters", len(text)],
        ["Characters without spaces", sum(1 for c in text if not c.isspace())],
        ["Sentences", len(re.findall(r"[.!?]+(?=\s|$)", text))],
        ["Average word length", round(average, 1)],
        ["Reading time (minutes)", round(len(words) / 230, 1)],
    ]
    nixpad.show_panel("stats", "Text Statistics", [
        ui.label(doc.title, weak=True),
        ui.table("counts", ["Measure", "Value"], rows),
        ui.heading("Most common words"),
        ui.checkbox("skip_short", "Skip words shorter than 4 letters", state["skip_short"]),
        ui.table("top", ["Word", "Count"], [[w, n] for w, n in state["top"]]),
        ui.label("Click a word to find it.", weak=True),
        ui.button("export", "Word Frequencies to New Tab"),
    ])


@nixpad.command("Show or Hide Text Statistics", shortcut="Ctrl+Alt+T")
def toggle():
    state["shown"] = not state["shown"]
    if state["shown"]:
        render()
    else:
        nixpad.close_panel("stats")


@nixpad.command("Word Frequencies to New Tab")
def export():
    doc = nixpad.active()
    counts = word_counts(doc.get_text(), state["skip_short"])
    lines = ["word,count"] + ["%s,%d" % (w, n) for w, n in counts.most_common()]
    # A CSV tab: try Tools > CsvQuery on it.
    nixpad.new_document("\n".join(lines) + "\n")
    nixpad.message("%d different words" % len(counts))


@nixpad.command("Count Words in Selection")
def count_selection():
    text = nixpad.active().selected_text()
    if not text:
        nixpad.message("Select some text first.", "warning")
        return
    nixpad.message("%d words, %d characters selected" % (len(WORD.findall(text)), len(text)))


@nixpad.on("activated")
def on_activated(doc):
    render(doc)


@nixpad.on("changed")
def on_changed(doc):
    if doc.id == nixpad.active().id:
        render(doc)


@nixpad.on_panel("stats")
def on_panel(event):
    if event["event"] == "close":
        state["shown"] = False
    elif event["item"] == "skip_short":
        state["skip_short"] = event["value"]
        render()
    elif event["item"] == "export":
        export()
    elif event["item"] == "top" and event["event"] == "select":
        word = state["top"][event["value"]][0]
        doc = nixpad.active()
        match = re.search(r"(?<![\w'’-])%s(?![\w'’-])" % re.escape(word),
                          doc.get_text(), re.IGNORECASE)
        if match:
            doc.select(match.start(), match.end())


nixpad.run()
