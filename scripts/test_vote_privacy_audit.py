# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import base64
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

import vote_privacy_audit as audit


class BallotArtifactAudit(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.forbidden = {"voter_id": ["synthetic-private-voter"], "username": ["Jörg Synthetic"],
                          "ip": ["198.51.100.7"], "country": ["US"], "device": ["synthetic-device"]}
        self.config = self.write("forbidden.json", json.dumps(self.forbidden))

    def write(self, name, text):
        path = self.root / name
        path.write_text(text, encoding="utf-8")
        return path

    def test_clean_pseudonymous_artifacts_pass_without_country_substring_false_positive(self):
        for name, content in [("ballot.json", '{"pseudonym":"7QKM-2HXD","status":"valid"}'),
                              ("ballot.csv", 'pseudonym,ballot_id\n7QKM-2HXD,ABCD-EFGH\n'),
                              ("receipt.pdf.txt", 'Ballot ABCD-EFGH, pseudonym 7QKM-2HXD')]:
            self.assertEqual(audit.scan(self.write(name, content), self.forbidden), [])

    def test_decoded_json_nested_envelope_and_signed_base64_are_scanned(self):
        body = {"encrypted_ballot": json.dumps({"username": "Jörg Synthetic"}),
                "received_entry": base64.b64encode(b'\x01synthetic-private-voter\x00').decode()}
        self.assertEqual(audit.scan(self.write("proof.json", json.dumps(body)), self.forbidden),
                         ["username", "voter_id"])

    def test_csv_plain_and_extracted_pdf_text_detect_network_and_device(self):
        for extension in ["csv", "txt", "pdf.txt"]:
            path = self.write("artifact." + extension, '198.51.100.7,"US",synthetic-device')
            self.assertEqual(audit.scan(path, self.forbidden), ["country", "device", "ip"])

    def test_failure_reports_categories_without_echoing_private_values(self):
        path = self.write("ballots.csv", 'synthetic-private-voter,198.51.100.7')
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = audit.main(["--forbidden", str(self.config), str(path)])
        self.assertEqual(status, 1)
        self.assertIn("FAIL ballots.csv: ip, voter_id", output.getvalue())
        for values in self.forbidden.values():
            for value in values:
                self.assertNotIn(value, output.getvalue())

    def test_missing_artifact_and_unextracted_pdf_fail_closed_without_content(self):
        for path in [self.root / "missing.json", self.write("receipt.pdf", "synthetic-private-voter")]:
            output = io.StringIO()
            with contextlib.redirect_stderr(output):
                status = audit.main(["--forbidden", str(self.config), str(path)])
            self.assertEqual(status, 2)
            self.assertNotIn("synthetic-private-voter", output.getvalue())


if __name__ == "__main__":
    unittest.main()
