# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The OTP capacity bench's pure parts: nothing here starts a process."""

from __future__ import annotations

import contextlib
import io
import unittest
from pathlib import Path

from . import messaging_otp
from .__main__ import parser


class MessagingOtpTest(unittest.TestCase):
    def test_help_prints(self):
        with contextlib.redirect_stdout(io.StringIO()) as output:
            with self.assertRaises(SystemExit) as exit_:
                parser().parse_args(["messaging-otp", "--help"])
        self.assertEqual(exit_.exception.code, 0)
        self.assertIn("saturates the same account", output.getvalue())

    def test_ids_written_into_sql_must_be_uuids(self):
        tenant = "0b1c2d3e-0000-4000-8000-000000000001"
        account = "0b1c2d3e-0000-4000-8000-000000000002"
        checkout = str(Path(__file__).resolve().parents[3])
        common = ["messaging-otp", "--label", "t", "--checkout", checkout]
        parsed = parser().parse_args(
            [*common, "--tenant-id", tenant, "--account-id", account]
        )
        self.assertEqual((parsed.tenant_id, parsed.account_id), (tenant, account))
        for bad in ["x'; DROP TABLE sequent_backend.message; --", "not-a-uuid"]:
            with contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    parser().parse_args(
                        [*common, "--tenant-id", bad, "--account-id", account]
                    )

    def test_percentiles_are_nearest_rank(self):
        values = [float(n) for n in range(1, 101)]
        self.assertEqual(messaging_otp.percentile(values, 0.50), 50.0)
        self.assertEqual(messaging_otp.percentile(values, 0.95), 95.0)
        self.assertIsNone(messaging_otp.percentile([], 0.5))

    def test_a_window_counts_codes_past_their_budget(self):
        codes = [
            messaging_otp.Code(0.2, 200, "ACCEPTED", None),
            messaging_otp.Code(10.0, 200, "FAILED", messaging_otp.CAPACITY_BUSY),
            messaging_otp.Code(30.0, None, None, None, "TimeoutError"),
        ]
        summary = messaging_otp.window_summary(codes)
        self.assertEqual(summary["answered"], 2)
        self.assertEqual(summary["capacity_busy"], 1)
        self.assertEqual(
            summary["states"], {"ACCEPTED": 1, "FAILED": 1, "http-None": 1}
        )

    def test_environment_references_resolve_process_values_first(self):
        resolved = messaging_otp.interpolated(
            {"HASURA_DB__HOST": "${HASURA_PG_HOST}", "HASURA_PG_HOST": "postgres"},
            {"HASURA_PG_HOST": "localhost"},
        )
        self.assertEqual(resolved["HASURA_DB__HOST"], "localhost")

    def test_a_code_request_is_what_keycloak_sends(self):
        options = messaging_otp.OtpOptions(
            label="t",
            checkout=Path("."),
            tenant_id="00000000-0000-4000-8000-000000000001",
            account_id="00000000-0000-4000-8000-000000000002",
            election_event_id=None,
            messages_per_second=50,
            otp_reserved_per_second=10,
            otp_rate=5.0,
            duration=1.0,
            warmup=0.0,
            baseline=False,
            bulk_workers=1,
            provider_latency_ms=0,
            profile="dev",
            build=False,
            keep_rows=False,
            token_command=None,
            timeout=1.0,
            database=messaging_otp.Database("postgres", "postgres"),
            output_dir=Path("."),
            run_id="run1",
        )
        request = messaging_otp.code_request(options, "VIBER", 7)
        self.assertEqual(request["purpose"], "OTP")
        self.assertEqual(request["logical_key"], "bench-otp:run1:7")
        self.assertEqual(request["destination"], "+639180000007")
        self.assertEqual(request["content"]["code"], "000007")
        self.assertIsNotNone(request["expires_at"])


if __name__ == "__main__":
    unittest.main()
