"""Helpers for the Markdown Preview plugin, kept apart from main.py so they
can be tested without Ettin Edit."""

EXTENSIONS = (".md", ".markdown", ".mdown", ".mkd", ".mdx")


def is_markdown(doc):
    """True for tabs in the Markdown language or with a Markdown file name."""
    if (doc.language or "").lower().startswith("markdown"):
        return True
    return bool(doc.path) and doc.path.lower().endswith(EXTENSIONS)


def strip_front_matter(text):
    """Drops a YAML front matter block (--- ... --- at the very top)."""
    if not text.startswith("---\n"):
        return text
    end = text.find("\n---", 4)
    if end < 0:
        return text
    rest = text[end + 4:]
    return rest[1:] if rest.startswith("\n") else rest
