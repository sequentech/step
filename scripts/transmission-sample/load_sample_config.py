#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Load the ER transmission sample configuration onto an existing test event.

It writes the configuration's `miru:` annotations on the event, on one of its
elections (the Post), on one area (the country), and on the contests of that
area in that election and their candidates, through Hasura. Run it before the
ballot styles are published: the tally reads the contests' and candidates'
annotations from the published ballot styles.

Contests are matched to the configuration's contests in the order of their
English names, and candidates likewise; the configuration must describe at
least as many as the event has. The SBEIs keep the configuration's
usernames unless `--sbei` names the accounts that sign (in the
configuration's order). Annotations not in the configuration are kept.

Example:

    scripts/transmission-sample/load_sample_config.py \\
        --hasura-url http://127.0.0.1:8080/v1/graphql --admin-secret admin \\
        --election-event-id <uuid> --election-id <uuid> --area-id <uuid> \\
        --sbei admin --sbei sbei2
"""

import argparse
import copy
import json
import os
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_CONFIG = HERE / "transmission-sample-config.json"
SBEI_USERS = "miru:sbei-users"


class Hasura:
    """The least GraphQL client the loader needs: `query(text, variables)`."""

    def __init__(self, url, admin_secret=None, token=None):
        self.url = url
        self.headers = {"Content-Type": "application/json"}
        if admin_secret:
            self.headers["x-hasura-admin-secret"] = admin_secret
        if token:
            self.headers["Authorization"] = f"Bearer {token}"

    def query(self, query, variables=None):
        body = json.dumps({"query": query, "variables": variables or {}}).encode()
        request = urllib.request.Request(self.url, body, self.headers, method="POST")
        with urllib.request.urlopen(request, timeout=60) as response:
            result = json.load(response)
        if result.get("errors"):
            raise RuntimeError(f"GraphQL errors: {result['errors']}")
        return result["data"]


def annotation_values(annotations):
    """Annotations as Step stores them: every value a string, JSON values encoded."""
    return {
        key: value if isinstance(value, str) else json.dumps(value)
        for key, value in annotations.items()
    }


def display_name(row):
    i18n = ((row.get("presentation") or {}).get("i18n")) or {}
    return (i18n.get("en") or {}).get("name") or ""


def ordered(rows):
    return sorted(rows, key=lambda row: (display_name(row), row["id"]))


def with_sbeis(config, usernames):
    """The configuration with its SBEIs' usernames replaced, in order."""
    config = copy.deepcopy(config)
    if not usernames:
        return config
    sbeis = config["election_event"]["annotations"][SBEI_USERS]
    if len(usernames) > len(sbeis):
        raise ValueError(
            f"{len(usernames)} SBEI accounts given; the configuration has {len(sbeis)} SBEIs"
        )
    for sbei, username in zip(sbeis, usernames):
        sbei["username"] = username
    return config


def plan(config, contests, candidates_by_contest):
    """The annotations to write on each contest and candidate row.

    `contests` are the area's contests in the Post; `candidates_by_contest`
    maps a contest id to its candidate rows. Returns `(contest_updates,
    candidate_updates)`, each a list of `(row id, annotations)`.
    """
    described = config["post"]["contests"]
    contests = ordered(contests)
    if len(contests) > len(described):
        raise ValueError(
            f"the country has {len(contests)} contests in the Post; "
            f"the configuration describes {len(described)}"
        )
    contest_updates, candidate_updates = [], []
    for contest, sample in zip(contests, described):
        contest_updates.append((contest["id"], annotation_values(sample["annotations"])))
        candidates = ordered(candidates_by_contest.get(contest["id"], []))
        if len(candidates) > len(sample["candidates"]):
            raise ValueError(
                f"contest {display_name(contest)!r} has {len(candidates)} candidates; "
                f"the configuration describes {len(sample['candidates'])}"
            )
        for candidate, sample_candidate in zip(candidates, sample["candidates"]):
            candidate_updates.append(
                (candidate["id"], annotation_values(sample_candidate["annotations"]))
            )
    return contest_updates, candidate_updates


ROWS = """
query ($event: uuid!, $election: uuid!, $area: uuid!) {
  sequent_backend_area_contest(
    where: {election_event_id: {_eq: $event}, area_id: {_eq: $area},
            contest: {election_id: {_eq: $election}}}
  ) {
    contest { id presentation candidates { id presentation } }
  }
}
"""

APPEND = """
mutation ($id: uuid!, $annotations: jsonb!) {
  update_sequent_backend_%s(where: {id: {_eq: $id}}, _append: {annotations: $annotations}) {
    affected_rows
  }
}
"""


SCOPE_ALL = "all"
# The contests' and candidates' annotations, which the published ballot
# styles carry to the tally.
SCOPE_BALLOT = "ballot"
# The event's, the Post's and the country's, which the package reads when it
# is generated.
SCOPE_PACKAGE = "package"
SCOPES = (SCOPE_ALL, SCOPE_BALLOT, SCOPE_PACKAGE)


def load(
    client, config, event_id, election_id, area_id, sbei_usernames=(), scope=SCOPE_ALL
):
    """Write the configuration's annotations; returns how many rows changed."""
    if scope not in SCOPES:
        raise ValueError(f"unknown scope {scope!r}")
    config = with_sbeis(config, list(sbei_usernames))
    updates = []
    if scope in (SCOPE_ALL, SCOPE_PACKAGE):
        updates += [
            ("election_event", event_id, annotation_values(config["election_event"]["annotations"])),
            ("election", election_id, annotation_values(config["post"]["annotations"])),
            ("area", area_id, annotation_values(config["country"]["annotations"])),
        ]
    if scope in (SCOPE_ALL, SCOPE_BALLOT):
        rows = client.query(
            ROWS, {"event": event_id, "election": election_id, "area": area_id}
        )["sequent_backend_area_contest"]
        contests = [row["contest"] for row in rows]
        if not contests:
            raise ValueError("the area has no contest in that election")
        contest_updates, candidate_updates = plan(
            config,
            contests,
            {contest["id"]: contest["candidates"] for contest in contests},
        )
        updates += [("contest", row_id, values) for row_id, values in contest_updates]
        updates += [("candidate", row_id, values) for row_id, values in candidate_updates]
    for table, row_id, values in updates:
        changed = client.query(APPEND % table, {"id": row_id, "annotations": values})[
            f"update_sequent_backend_{table}"
        ]["affected_rows"]
        if changed != 1:
            raise RuntimeError(f"{table} {row_id} was not updated")
    return len(updates)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
    parser.add_argument(
        "--hasura-url",
        default=os.environ.get("HASURA_ENDPOINT", "http://127.0.0.1:8080/v1/graphql"),
    )
    parser.add_argument(
        "--admin-secret", default=os.environ.get("HASURA_GRAPHQL_ADMIN_SECRET")
    )
    parser.add_argument("--token", help="a JWT, instead of the admin secret")
    parser.add_argument("--election-event-id", required=True)
    parser.add_argument("--election-id", required=True, help="the Post")
    parser.add_argument("--area-id", required=True, help="the country")
    parser.add_argument(
        "--sbei",
        action="append",
        default=[],
        metavar="USERNAME",
        help="an account that signs as the configuration's next SBEI",
    )
    parser.add_argument(
        "--scope",
        choices=SCOPES,
        default=SCOPE_ALL,
        help="ballot: only the contests and candidates (before publishing); "
        "package: only the event, Post and country",
    )
    args = parser.parse_args(argv)
    if not args.admin_secret and not args.token:
        parser.error("give --admin-secret or --token")
    config = json.loads(args.config.read_text())
    changed = load(
        Hasura(args.hasura_url, args.admin_secret, args.token),
        config,
        args.election_event_id,
        args.election_id,
        args.area_id,
        args.sbei,
        args.scope,
    )
    print(f"Loaded {args.config.name}: {changed} rows annotated.")
    print(
        "Switch ACTIVATE_MIRU_EXPORT on in the Admin Portal's global settings, "
        "then publish, vote, close and tally the Post."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
