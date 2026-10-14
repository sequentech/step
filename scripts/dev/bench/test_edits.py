# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import json
import tempfile
import unittest
from pathlib import Path

from .edits import (
    EditError,
    EditSpec,
    MarkerEdit,
    insert_marker,
    load_edits,
    marker_for,
)

SOURCE = "function Header() {\n    return (\n        <Version />\n    )\n}\n"


class InsertMarkerTest(unittest.TestCase):
    def test_inserts_before_the_anchor_with_its_indentation(self):
        spec = EditSpec("Header.tsx", "<span>{marker}</span>", "<Version />")
        self.assertEqual(
            insert_marker(SOURCE, spec, "m1"),
            "function Header() {\n    return (\n        <span>m1</span>\n"
            "        <Version />\n    )\n}\n",
        )

    def test_keeps_windows_line_endings(self):
        spec = EditSpec("a.rs", "// {marker}", "b();")
        self.assertEqual(
            insert_marker("a();\r\n\tb();\r\n", spec, "m"),
            "a();\r\n\t// m\r\n\tb();\r\n",
        )

    def test_appends_without_an_anchor(self):
        spec = EditSpec("a.ts", "// {marker}")
        self.assertEqual(insert_marker("x\n", spec, "m"), "x\n// m\n")
        # A missing final newline must not glue the marker to the last line.
        self.assertEqual(insert_marker("x", spec, "m"), "x\n// m\n")

    def test_rejects_a_missing_anchor(self):
        spec = EditSpec("Header.tsx", "{marker}", "<Footer />")
        with self.assertRaisesRegex(EditError, "anchor not found"):
            insert_marker(SOURCE, spec, "m")

    def test_rejects_an_ambiguous_anchor(self):
        spec = EditSpec("Header.tsx", "{marker}", "return")
        with self.assertRaisesRegex(EditError, "matches 2 lines"):
            insert_marker(SOURCE + "return\n", spec, "m")


class EditSpecTest(unittest.TestCase):
    def test_template_needs_the_marker_field(self):
        with self.assertRaisesRegex(EditError, "must contain"):
            EditSpec("a.ts", "// no marker")

    def test_path_must_stay_inside_the_checkout(self):
        for path in ("/etc/passwd", "packages/../../x.ts"):
            with self.subTest(path=path), self.assertRaisesRegex(EditError, "relative"):
                EditSpec(path, "// {marker}")

    def test_loads_named_edits_and_rejects_unknown_fields(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "edits.json"
            path.write_text(
                json.dumps({"x": {"path": "a.ts", "template": "// {marker}"}})
            )
            self.assertEqual(load_edits(path), {"x": EditSpec("a.ts", "// {marker}")})
            path.write_text(
                json.dumps({"x": {"path": "a.ts", "template": "{marker}", "at": 1}})
            )
            with self.assertRaisesRegex(EditError, "unknown edit fields"):
                load_edits(path)


class MarkerTest(unittest.TestCase):
    def test_no_marker_contains_another(self):
        markers = [marker_for("abc123", index) for index in range(1, 25)]
        for first in markers:
            for second in markers:
                if first != second:
                    self.assertNotIn(first, second)


class MarkerEditTest(unittest.TestCase):
    def test_each_save_derives_from_the_original_and_exit_restores_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Header.tsx").write_text(SOURCE)
            spec = EditSpec("Header.tsx", "<span>{marker}</span>", "<Version />")
            with MarkerEdit(root, spec) as edit:
                edit.apply("m1")
                edit.apply("m2")
                text = (root / "Header.tsx").read_text()
                self.assertIn("<span>m2</span>", text)
                self.assertNotIn("m1", text)
            self.assertEqual((root / "Header.tsx").read_text(), SOURCE)

    def test_invalid_anchor_fails_before_any_write(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Header.tsx").write_text(SOURCE)
            with self.assertRaises(EditError):
                MarkerEdit(root, EditSpec("Header.tsx", "{marker}", "<Missing />"))
            self.assertEqual((root / "Header.tsx").read_text(), SOURCE)


if __name__ == "__main__":
    unittest.main()
