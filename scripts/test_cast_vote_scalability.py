# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run local voting-flow regression tests and an optional SQL-path benchmark.

Inside the dev container, from /workspaces/step:
    devenv shell python3 scripts/test_cast_vote_scalability.py --benchmark
"""

import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).parent / "voting_flow"))

from database import RESULTS, local_database
from regression import run_regressions


def main():
    """Parse benchmark options, run database regressions, and write any requested results."""
    from benchmark import SCENARIOS

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--benchmark", action="store_true")
    parser.add_argument("--rust-tests", action="store_true")
    parser.add_argument("--output", type=Path, default=RESULTS / "voting-flow.json")
    parser.add_argument(
        "--scenario",
        action="append",
        choices=[scenario.name for scenario in SCENARIOS],
        help="Select a benchmark scenario; repeat to select several (default: all)",
    )
    args = parser.parse_args()
    if args.scenario and not args.benchmark:
        parser.error("--scenario requires --benchmark")
    with local_database() as database:
        run_regressions(database)
        if args.rust_tests:
            from rust_tests import run_rust_tests

            run_rust_tests(database)
        if args.benchmark:
            from benchmark import run_benchmark

            run_benchmark(database, args.output, args.scenario)


if __name__ == "__main__":
    main()
