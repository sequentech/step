# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Select a release build scope without granting credentials to the planner."""

import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scope", choices=("all", "authoring"))
    args = parser.parse_args()
    rows = json.loads(Path(__file__).with_name("build_images.json").read_text())
    if args.scope == "authoring":
        rows = [row for row in rows if row["service"] in (
            "election-architect", "delivery-orchestrator"
        )]
    print(json.dumps({"include": rows}, separators=(",", ":")))


if __name__ == "__main__":
    main()
