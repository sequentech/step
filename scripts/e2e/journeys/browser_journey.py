# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run the independent manual journey with real Chromium trustees."""
import json
import sys
import unittest

from .__main__ import JourneyResult
from .client import ENV, OUTPUT
from .test_journeys import BackendJourneys


def main():
    if not ENV.get("STEP_E2E_BROWSER_TRUSTEES_URL"):
        raise RuntimeError("Browser trustee controller is required")
    suite = unittest.TestSuite(
        [BackendJourneys("test_b_manual_key_ceremony_and_tally")]
    )
    result = unittest.TextTestRunner(verbosity=2, resultclass=JourneyResult).run(suite)
    (OUTPUT / "browser-journey.json").write_text(json.dumps(result.records, indent=2))
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
