# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Generate and import distinct synthetic census voters before encryption/load.

Use an authenticated step-cli and an external_config.json referencing the exported
server-assigned election/area IDs. This stage does not publish elections or cast.
"""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import subprocess
import uuid

from capture import save


def main():
    """Persist the generated census and private import logs as a preparation artifact."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--count", type=int, required=True)
    parser.add_argument("--username-start", type=int, required=True)
    args = parser.parse_args()
    if args.count <= 0 or args.username_start < 0:
        parser.error("count must be positive and username-start nonnegative")
    os.umask(0o077)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    config = json.loads(args.config.read_text())
    event = Path(config["election_event_json_file"])
    if not event.is_absolute():
        event = (args.config.parent / event).resolve()
    if not event.is_file():
        raise ValueError("Missing exported election configuration")
    config["election_event_json_file"] = str(event)
    config["generate_voters"]["username_start_number"] = args.username_start
    save(output / "external_config.json", config)
    cli = str(args.cli.resolve())
    with (output / "generate.log").open("w") as log:
        subprocess.run(
            [
                cli,
                "step",
                "generate-voters",
                "--working-directory",
                str(output),
                "--num-users",
                str(args.count),
            ],
            stdout=log,
            stderr=log,
            check=True,
        )
    path = output / f"voters_{args.count}.csv"
    with path.open() as stream:
        reader = csv.DictReader(stream)
        fields = reader.fieldnames
        rows = list(reader)
    if len(rows) != args.count or len({r["username"] for r in rows}) != args.count:
        raise ValueError("Generated census is incomplete or duplicated")
    prefix = uuid.uuid4().hex
    for row in rows:
        if "email" in row:
            row["email"] = f"load-{prefix}-{row['username']}@example.invalid"
    with path.open("w") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)
    with (output / "import.log").open("w") as log:
        subprocess.run(
            [
                cli,
                "step",
                "import-voters",
                "--election-event-id",
                config["election_event_id"],
                "--file-path",
                str(path),
                "--is-local",
            ],
            stdout=log,
            stderr=log,
            check=True,
        )
    # The CLI can print an application error while exiting zero.
    if "Voters imported successfully" not in (output / "import.log").read_text():
        raise RuntimeError("Census import did not confirm success; inspect private log")
    save(
        output / "census.json",
        dict(
            count=len(rows),
            election_event_id=config["election_event_id"],
            csv_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            imported=True,
        ),
    )
    print(f"Generated and imported {len(rows)} distinct synthetic voters")


if __name__ == "__main__":
    main()
