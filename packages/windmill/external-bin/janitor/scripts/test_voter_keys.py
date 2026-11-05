# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""CSV contracts use mapper keys, while aliases remain display/lookup values."""
import contextlib
import csv
import importlib.util
import io
import json
from pathlib import Path
import os
import sys
import tempfile
from types import SimpleNamespace, ModuleType
import unittest
from unittest.mock import patch

# CSV generation never accesses PostgreSQL. Fail if that unrelated optional
# boundary is accidentally reached; keep the pinned Faker generator real.
with patch.dict(sys.modules, {"psycopg2": ModuleType("psycopg2")}):
    import load_tool

VALIDATOR_PATH = Path(__file__).resolve().parents[1] / "validate_voters.py"
spec = importlib.util.spec_from_file_location("validate_voters", VALIDATOR_PATH)
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)

EVENT = {
    "areas": [{"id": "a", "name": "Eligible"}, {"id": "b", "name": "None"}],
    "elections": [
        {"id": "db-1", "external_id": "external-1", "alias": "Embassy - One"},
        {"id": "db-2", "external_id": None, "alias": "Embassy - Two"},
        {"id": "db-3", "external_id": "", "alias": "Embassy - Three"},
        {"id": "db-4", "alias": "Embassy - Four"},
    ],
    "contests": [{"id": f"c{i}", "election_id": f"db-{i}"} for i in range(1, 5)],
    "area_contests": [{"area_id": "a", "contest_id": f"c{i}"} for i in [1, 2, 1, 3, 4]],
}

class VoterKeys(unittest.TestCase):
    def test_generator_writes_external_keys_and_id_fallbacks_not_aliases(self):
        with tempfile.TemporaryDirectory() as directory, contextlib.redirect_stdout(io.StringIO()):
            root = Path(directory)
            (root / "event.json").write_text(json.dumps(EVENT))
            (root / "config.json").write_text(json.dumps({"election_event_json_file": "event.json"}))
            load_tool.run_generate_voters(SimpleNamespace(working_directory=directory, num_users=2))
            with (root / "generated_users.csv").open() as stream:
                rows = list(csv.DictReader(stream))
            self.assertEqual([r["authorized-election-ids"] for r in rows],
                             ["external-1|db-2|db-3|db-4", "Unknown"])
            self.assertEqual(rows[0]["country"], "Embassy/Unknown")

    def test_validator_accepts_mapper_keys_and_rejects_display_alias_or_wrong_db_key(self):
        for key, accepted in [("external-1|db-2|db-3|db-4", True),
                              ("Embassy - One|Embassy - Two|Embassy - Three|Embassy - Four", False),
                              ("db-1|db-2|db-3|db-4", False)]:
            with self.subTest(key=key), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "event.json").write_text(json.dumps(EVENT))
                with (root / "voters.csv").open("w") as stream:
                    writer = csv.writer(stream)
                    writer.writerow(["area_name", "authorized-election-ids", "country", "dateOfBirth"])
                    writer.writerow(["Eligible", key, "Embassy/Unknown", "2000-01-01"])
                    writer.writerow(["None", "Unknown", "Unknown/Unknown", "2000-01-01"])
                before = Path.cwd()
                try:
                    os.chdir(root)
                    output = io.StringIO()
                    with patch("sys.argv", ["validate_voters", "event.json", "voters.csv"]), contextlib.redirect_stdout(output):
                        validator.main()
                    if accepted:
                        self.assertIn("No errors found", output.getvalue())
                        self.assertFalse((root / "error_log.txt").exists())
                    else:
                        self.assertIn("Mismatch authorized-election-ids", (root / "error_log.txt").read_text())
                finally:
                    os.chdir(before)

if __name__ == "__main__":
    unittest.main()
