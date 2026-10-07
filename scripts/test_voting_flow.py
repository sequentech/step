# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run the local voting-flow database regression tests.

Inside the dev container, from /workspaces/step:
    devenv shell python3 scripts/test_voting_flow.py --rust-tests
"""

import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).parent / "voting_flow"))

from database import local_database
from regression import run_regressions


def main():
    """Parse options, then run the database regressions and, if asked, the Rust ones."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-tests", action="store_true")
    args = parser.parse_args()
    with local_database() as database:
        run_regressions(database)
        if args.rust_tests:
            from rust_tests import run_rust_tests

            run_rust_tests(database)


if __name__ == "__main__":
    main()
