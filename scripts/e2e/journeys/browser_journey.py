# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run manual and automatic ceremonies, voting and tally with Chromium trustees."""
import json
import sys
import unittest

from .__main__ import JourneyResult
from .client import ENV, OUTPUT
from .test_journeys import BackendJourneys


def main():
    if not ENV.get("STEP_E2E_BROWSER_TRUSTEES_URL"):
        raise RuntimeError("Browser trustee controller is required")
    # The manual journey starts the browser participants and checks custody,
    # IndexedDB recovery and portable backups. Keep those same participants
    # running through the automatic ceremony and its voting/tally prerequisites.
    automatic = [
        test
        for test in unittest.TestLoader().loadTestsFromTestCase(BackendJourneys)
        if test._testMethodName < "test_8"
    ]
    suite = unittest.TestSuite(
        [
            BackendJourneys("test_b_manual_key_ceremony_and_tally"),
            *automatic,
            BackendJourneys("test_a_combined_contest_weighted_voting"),
            BackendJourneys("test_c_trustee_graphql_board_api"),
        ]
    )
    result = unittest.TextTestRunner(verbosity=2, resultclass=JourneyResult).run(suite)
    (OUTPUT / "browser-journey.json").write_text(json.dumps(result.records, indent=2))
    return (
        0
        if result.wasSuccessful() and not (result.skipped or result.expectedFailures)
        else 1
    )


if __name__ == "__main__":
    sys.exit(main())
