# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Synthetic election event and census for the journeys.

The event starts from the exported fixture that `step-cli load` imports, so it
carries a real event realm; voters log in with username and password.

- The main election has areas A and B, each with its own contest, allows one
  revote and has no grace period. Its voters are authorized by the election's
  external ID.
- The grace election has area C, a 15 minute grace period, and no external
  ID, so its voters are authorized by election ID (as `step-cli step
  generate-voters` writes them).
"""

import copy
import csv
import json
import uuid
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TEMPLATE = ROOT / "packages/voting-load/fixtures/election.json"

VOTER_PASSWORD = "E2e-voter-2026!"
# num_allowed_revotes counts every vote: a first vote and one revote.
MAIN_ALLOWED_VOTES = 2
GRACE_PERIOD_SECONDS = 900


@dataclass(frozen=True)
class Area:
    key: str
    name: str
    election: str  # "main" or "grace"
    contest: str
    candidates: tuple


AREAS = (
    Area("A", "E2E area A", "main", "Contest A", ("Alice", "Bob", "Carol")),
    Area("B", "E2E area B", "main", "Contest B", ("Dave", "Erin")),
    Area("C", "E2E area C", "grace", "Contest C", ("Frank", "Grace")),
)
AREA = {area.key: area for area in AREAS}


def _name(item, name):
    """Set the display name in every language; entities keep names in presentation."""
    presentation = item.get("presentation") or {}
    i18n = presentation.setdefault("i18n", {})
    for language in i18n.values():
        language["name"] = name
        if language.get("alias") is not None:
            language["alias"] = name
    i18n.setdefault("en", {})["name"] = name
    item["presentation"] = presentation


def display_name(item):
    """Read the English display name the way `_name` wrote it."""
    return (
        ((item.get("presentation") or {}).get("i18n") or {}).get("en", {}).get("name")
    )


def election_event(voting_portal_url, tag):
    """Return the import document for a fresh event, with the fixture's translations."""
    event = json.loads(TEMPLATE.read_text())
    main = event["elections"][0]
    template_contest = event["contests"][0]
    template_area = event["areas"][0]
    template_candidate = next(
        c for c in event["candidates"] if c["contest_id"] == template_contest["id"]
    )

    _name(event["election_event"], f"Backend E2E {tag}")
    grace = copy.deepcopy(main)
    grace["id"] = str(uuid.uuid4())
    elections = {"main": main, "grace": grace}
    _name(main, "E2E main election")
    main["external_id"] = f"e2e-main-{tag}"
    main["num_allowed_revotes"] = MAIN_ALLOWED_VOTES
    main["presentation"]["grace_period_policy"] = "no-grace-period"
    main["presentation"]["grace_period_secs"] = 0
    _name(grace, "E2E grace election")
    grace["external_id"] = None
    grace["num_allowed_revotes"] = MAIN_ALLOWED_VOTES
    grace["presentation"]["grace_period_policy"] = "grace-period-without-alert"
    grace["presentation"]["grace_period_secs"] = GRACE_PERIOD_SECONDS

    areas, contests, candidates, links = [], [], [], []
    for spec in AREAS:
        area = copy.deepcopy(template_area)
        area.update(id=str(uuid.uuid4()), name=spec.name)
        contest = copy.deepcopy(template_contest)
        contest.update(
            id=str(uuid.uuid4()),
            election_id=elections[spec.election]["id"],
            min_votes=1,
            max_votes=1,
            winning_candidates_num=1,
        )
        _name(contest, spec.contest)
        for candidate_name in spec.candidates:
            candidate = copy.deepcopy(template_candidate)
            candidate.update(id=str(uuid.uuid4()), contest_id=contest["id"])
            _name(candidate, candidate_name)
            candidates.append(candidate)
        areas.append(area)
        contests.append(contest)
        links.append(
            {
                "id": str(uuid.uuid4()),
                "area_id": area["id"],
                "contest_id": contest["id"],
            }
        )

    event.update(
        elections=[main, grace],
        areas=areas,
        contests=contests,
        candidates=candidates,
        area_contests=links,
    )

    # The same simplifications `step-cli load prepare` applies to this fixture.
    for item in [event["election_event"], *event["elections"], *contests, *candidates]:
        item["annotations"] = {}
        for key in list(item):
            if key.endswith("_document_id"):
                item[key] = None
    realm = event["keycloak_event_realm"]
    for config in realm.get("authenticatorConfig") or []:
        if "matchAttributes" in config["config"]:
            config["config"]["matchAttributes"] = "username"
    for messages in (realm.get("localizationTexts") or {}).values():
        messages["loginCustomCss"] = ""
    origin = voting_portal_url.rstrip("/")
    for client in realm["clients"]:
        if client["clientId"] == "voting-portal":
            client.update(
                rootUrl=origin,
                baseUrl=origin,
                redirectUris=[f"{origin}/*"],
                webOrigins=[origin],
            )
    return event


def without_translations(document):
    """Keep one event name only; see test_5b for why publishing needs this."""
    reduced = copy.deepcopy(document)
    name = display_name(reduced["election_event"])
    reduced["election_event"]["presentation"]["i18n"] = {"en": {"name": name}}
    return reduced


def dangling_contest_link(document):
    """A copy whose last area-contest link names a contest missing from the bundle."""
    broken = copy.deepcopy(document)
    _name(
        broken["election_event"], display_name(broken["election_event"]) + " (invalid)"
    )
    broken["area_contests"][-1]["contest_id"] = str(uuid.uuid4())
    return broken


@dataclass
class Voter:
    username: str
    area: str
    tokens: dict = None


def census(tag, counts):
    """Voters per area key, e.g. {"A": 4} -> e2e-<tag>-a1 .. a4."""
    return [
        Voter(f"e2e-{tag}-{key.lower()}{index}", key)
        for key, count in counts.items()
        for index in range(1, count + 1)
    ]


def write_census(path, voters, authorization):
    """Write the voter CSV `step-cli step import-voters` takes.

    `authorization` maps an area key to its `authorized-election-ids` value.
    """
    with Path(path).open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "username",
                "area_name",
                "email",
                "email_verified",
                "password",
                "authorized-election-ids",
            ]
        )
        for voter in voters:
            writer.writerow(
                [
                    voter.username,
                    AREA[voter.area].name,
                    f"{voter.username}@example.invalid",
                    "true",
                    VOTER_PASSWORD,
                    authorization[voter.area],
                ]
            )
