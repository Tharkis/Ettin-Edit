"""Helpers for the Markdown Preview plugin, kept apart from main.py so they
can be tested without Ettin Edit."""

EXTENSIONS = (".md", ".markdown", ".mdown", ".mkd", ".mdx")


def is_markdown(doc):
    """True for tabs in the Markdown language or with a Markdown file name."""
    if (doc.language or "").lower().startswith("markdown"):
        return True
    return bool(doc.path) and doc.path.lower().endswith(EXTENSIONS)


def split_front_matter(text):
    """Drops a YAML front matter block (--- ... --- at the very top).
    Returns the rest and the line (from 0) it starts on."""
    if not text.startswith("---\n"):
        return text, 0
    end = text.find("\n---", 4)
    if end < 0:
        return text, 0
    rest = text[end + 4:]
    if rest.startswith("\n"):
        rest = rest[1:]
    return rest, text[:len(text) - len(rest)].count("\n")


def strip_front_matter(text):
    """The text without its front matter block."""
    return split_front_matter(text)[0]
