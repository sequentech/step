# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Generate a small real election import from the CLI's supported export fixture."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    """Keep one eligible area/contest and avoid committing another exported realm."""
    template = (
        ROOT
        / "packages/step-cli/scripts/telephone-load-test-inputs/election-event.json"
    )
    event = json.loads(template.read_text())
    area = event["areas"][0]
    link = next(
        item for item in event["area_contests"] if item["area_id"] == area["id"]
    )
    contest = next(
        item for item in event["contests"] if item["id"] == link["contest_id"]
    )
    election = next(
        item for item in event["elections"] if item["id"] == contest["election_id"]
    )
    event.update(
        areas=[area], area_contests=[link], contests=[contest], elections=[election]
    )
    event["candidates"] = [
        item for item in event["candidates"] if item["contest_id"] == contest["id"]
    ]
    for entity in [event["election_event"], election, contest, *event["candidates"]]:
        entity["annotations"] = {}
        for key in entity:
            if key.endswith("_document_id"):
                entity[key] = None
    area["name"] = "E2E area"
    contest["min_votes"] = 1
    contest["max_votes"] = 1
    output = ROOT / ".cache/voting-e2e/fixture.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(event, indent=2) + "\n")
    print(
        f"Generated one election, one contest, one area and {len(event['candidates'])} candidates: {output}"
    )


if __name__ == "__main__":
    main()
