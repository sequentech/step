# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Backend E2E driver, run inside the compose network by scripts/e2e/run.sh.

    python3 -m scripts.e2e.journeys bootstrap
    python3 -m scripts.e2e.journeys test [-k PATTERN]
"""

import argparse
import json
import sys
import time
import unittest

from . import bootstrap, test_journeys
from .client import OUTPUT


class JourneyResult(unittest.TextTestResult):
    """Adds per-test timings, and reports known defects as expected failures."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.records = []

    def startTest(self, test):
        self._started = time.monotonic()
        super().startTest(test)

    def _record(self, test, outcome, detail=""):
        self.records.append(
            {
                "test": test._testMethodName,
                "outcome": outcome,
                "seconds": round(time.monotonic() - self._started, 1),
                "detail": detail.strip(),
            }
        )

    def _known_defect(self, test):
        return getattr(getattr(test, test._testMethodName, None), "known_defect", None)

    def addSuccess(self, test):
        if self._known_defect(test):
            super().addUnexpectedSuccess(test)
            self._record(test, "unexpected success", "known defect no longer reproduces; remove its marker")
        else:
            super().addSuccess(test)
            self._record(test, "pass")

    def addFailure(self, test, err):
        if isinstance(err[1], test_journeys.KnownDefect):
            super().addExpectedFailure(test, err)
            self._record(test, "known defect", str(err[1]))
        else:
            super().addFailure(test, err)
            self._record(test, "fail", str(err[1]))

    def addError(self, test, err):
        super().addError(test, err)
        self._record(test, "error", f"{err[0].__name__}: {err[1]}")

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        self._record(test, "skip", reason)


def run_tests(pattern):
    loader = unittest.TestLoader()
    if pattern:
        loader.testNamePatterns = [f"*{pattern}*"]
    suite = loader.loadTestsFromTestCase(test_journeys.BackendJourneys)
    runner = unittest.TextTestRunner(stream=sys.stdout, verbosity=2, resultclass=JourneyResult)
    result = runner.run(suite)
    (OUTPUT / "journeys.json").write_text(json.dumps(result.records, indent=2))
    width = max((len(r["test"]) for r in result.records), default=0)
    print("\nJourney results:")
    for record in result.records:
        summary = record["detail"].splitlines()[0][:120] if record["detail"] else ""
        print(f"  {record['test']:<{width}}  {record['outcome']:<18} {record['seconds']:>6.1f}s  {summary}")
    return 0 if result.wasSuccessful() else 1


def main():
    parser = argparse.ArgumentParser(prog="python3 -m scripts.e2e.journeys", description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("bootstrap", help="Wait for the super tenant and prepare the administrator and trustees")
    test = commands.add_parser("test", help="Run the journeys in order")
    test.add_argument("-k", dest="pattern", help="Only run tests whose name contains PATTERN")
    arguments = parser.parse_args()
    OUTPUT.mkdir(parents=True, exist_ok=True)
    if arguments.command == "bootstrap":
        return bootstrap.main()
    return run_tests(arguments.pattern)


sys.exit(main())
