"""Tests for the Markdown Preview plugin's helpers. Run in this folder with:

    python3 -m unittest

They need only the standard library; the editor isn't involved.
"""

import unittest

import preview as main


class Doc:
    def __init__(self, language="", path=None):
        self.language = language
        self.path = path


class PreviewTest(unittest.TestCase):
    def test_knows_markdown_tabs(self):
        self.assertTrue(main.is_markdown(Doc("Markdown")))
        self.assertTrue(main.is_markdown(Doc("Plain Text", "/notes/README.MD")))
        self.assertFalse(main.is_markdown(Doc("Python", "/x/main.py")))
        self.assertFalse(main.is_markdown(Doc("Plain Text")))

    def test_front_matter_is_dropped(self):
        self.assertEqual(main.strip_front_matter("---\ntitle: x\n---\n# Hi\n"), "# Hi\n")
        self.assertEqual(main.strip_front_matter("# Hi\n---\n"), "# Hi\n---\n")
        self.assertEqual(main.strip_front_matter("---\nno end"), "---\nno end")


if __name__ == "__main__":
    unittest.main()
