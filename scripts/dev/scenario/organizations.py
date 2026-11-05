# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Organizations that sign their protected actions, as loadable data.

Each file in ``signing-organizations/`` describes one organization: the
tenant's display name and translation overrides, the groups that sign and
their sign permissions, the Posts, the signer accounts with their titles, the
signing rules and certificate checks, and optionally its own trusted issuer
(with its test-only private key, so tests can issue certificates under it).
A file may take its rules from a preset (``"preset"``: a path from the
repository root to a JSON file with ``signing_rules`` and ``signing_checks``).

Write one organization's files with::

    python3 -m scripts.dev.scenario.organizations student-council --out /tmp/orgs

and load them as the developer guide describes (fast-feedback, "Signing
organizations").
"""

from __future__ import annotations

import argparse
import copy
import csv
import io
import json
import sys
import uuid
from pathlib import Path

from scripts.e2e.journeys import fixtures

ROOT = Path(__file__).resolve().parents[3]
DIRECTORY = Path(__file__).with_name("signing-organizations")

BUNDLE = "election-event.json"
ADMIN_USERS = "admin-users.csv"
TENANT_SETTINGS = "tenant-settings.json"
REALM_GROUPS = "realm-groups.json"
TRUSTED_ISSUER = "trusted-issuer.pem"

ADMIN_USER_COLUMNS = (
    "enabled",
    "first_name",
    "last_name",
    "username",
    "permission_labels",
    "password",
    "group_name",
    "title",
)
# The importer splits multi-valued attributes on this.
MULTI_VALUE_SEPARATOR = "|"


def _name(item: dict, name: str) -> None:
    """Sets an entity's display name in every language it has, and English."""
    presentation = item.get("presentation") or {}
    i18n = presentation.setdefault("i18n", {})
    for language in i18n.values():
        language["name"] = name
        if language.get("alias") is not None:
            language["alias"] = name
    i18n.setdefault("en", {})["name"] = name
    item["presentation"] = presentation


class OrganizationError(ValueError):
    """No organization has the requested name."""


def names() -> list[str]:
    return sorted(path.stem for path in DIRECTORY.glob("*.json"))


def load(name: str) -> dict:
    """The organization's file, with its preset's rules and checks resolved."""
    path = DIRECTORY / f"{name}.json"
    if not path.is_file():
        raise OrganizationError(
            f"unknown organization {name!r}; choose one of {', '.join(names())}"
        )
    profile = json.loads(path.read_text())
    preset_path = profile.get("preset")
    if preset_path:
        preset = json.loads((ROOT / preset_path).read_text())
        profile.setdefault("signing_rules", preset["signing_rules"])
        profile.setdefault("signing_checks", preset["signing_checks"])
    return profile


def post_label(profile: dict, post: dict) -> str:
    """The permission label of a Post: its election's label and its signers'."""
    return f"{profile['name']}-{post['key']}"


def signer_rows(profile: dict) -> list[dict]:
    """One row per signer account, in the file's order."""
    every_label = [post_label(profile, post) for post in profile["posts"]]
    rows = []
    for signers in profile["signers"]:
        if "per_post" in signers:
            for post in profile["posts"]:
                for n, title in enumerate(signers["per_post"], start=1):
                    rows.append(
                        {
                            "username": signers["username"].format(
                                post=post["key"], n=n
                            ),
                            "first_name": title,
                            "last_name": post["name"],
                            "labels": [post_label(profile, post)],
                            "group": signers["group"],
                            "title": title,
                        }
                    )
        for n, title in enumerate(signers.get("event_wide", []), start=1):
            rows.append(
                {
                    "username": signers["username"].format(n=n),
                    "first_name": title,
                    "last_name": profile["event_name"],
                    "labels": every_label,
                    "group": signers["group"],
                    "title": title,
                }
            )
    return rows


def admin_users_csv(profile: dict) -> str:
    """The tenant's users import (Users and Roles > Import)."""
    buffer = io.StringIO()
    writer = csv.writer(buffer, lineterminator="\n")
    writer.writerow(ADMIN_USER_COLUMNS)
    for row in signer_rows(profile):
        writer.writerow(
            [
                "true",
                row["first_name"],
                row["last_name"],
                row["username"],
                MULTI_VALUE_SEPARATOR.join(row["labels"]),
                profile["password"],
                row["group"],
                row["title"],
            ]
        )
    return buffer.getvalue()


def realm_groups(profile: dict) -> dict:
    """A Keycloak partial import of the signing groups for the tenant realm."""
    return {
        "ifResourceExists": "SKIP",
        "groups": [
            {
                "name": group["name"],
                "path": f"/{group['name']}",
                "realmRoles": group["permissions"],
            }
            for group in profile["groups"]
        ],
    }


def tenant_settings(profile: dict) -> dict:
    """The keys to merge into the tenant's ``settings``."""
    return {
        "display_name": profile["tenant"]["display_name"],
        "i18n": profile["tenant"]["i18n"],
    }


def election_event(profile: dict, voting_portal_url: str, tag: str) -> dict:
    """An import bundle with one election (Post), area and contest per Post.

    Built from the journeys' fixture, so it carries a working event realm.
    """
    event = fixtures.election_event(voting_portal_url, tag)
    template_election = event["elections"][0]
    template_contest = event["contests"][0]
    template_area = event["areas"][0]
    template_candidates = [
        candidate
        for candidate in event["candidates"]
        if candidate["contest_id"] == template_contest["id"]
    ]

    elections, areas, contests, candidates, links = [], [], [], [], []
    for post in profile["posts"]:
        election = copy.deepcopy(template_election)
        election.update(
            id=str(uuid.uuid4()),
            external_id=f"{profile['name']}-{post['key']}-{tag}",
            permission_label=post_label(profile, post),
        )
        _name(election, post["name"])
        area = copy.deepcopy(template_area)
        area.update(id=str(uuid.uuid4()), name=post["name"])
        contest = copy.deepcopy(template_contest)
        contest.update(id=str(uuid.uuid4()), election_id=election["id"])
        _name(contest, f"{post['name']} representative")
        for template_candidate in template_candidates:
            candidate = copy.deepcopy(template_candidate)
            candidate.update(id=str(uuid.uuid4()), contest_id=contest["id"])
            candidates.append(candidate)
        elections.append(election)
        areas.append(area)
        contests.append(contest)
        links.append(
            {
                "id": str(uuid.uuid4()),
                "area_id": area["id"],
                "contest_id": contest["id"],
            }
        )

    _name(event["election_event"], f"{profile['event_name']} {tag}")
    event.update(
        elections=elections,
        areas=areas,
        contests=contests,
        candidates=candidates,
        area_contests=links,
        signing_rules=profile["signing_rules"],
        signing_checks=profile["signing_checks"],
    )
    return event


def write(
    name: str,
    out: Path,
    voting_portal_url: str = "http://127.0.0.1:3000",
    tag: str = "demo",
) -> list[Path]:
    """Writes the organization's files under ``out/<name>/``."""
    profile = load(name)
    directory = out / name
    directory.mkdir(parents=True, exist_ok=True)
    files = {
        BUNDLE: json.dumps(election_event(profile, voting_portal_url, tag), indent=2),
        ADMIN_USERS: admin_users_csv(profile),
        TENANT_SETTINGS: json.dumps(
            tenant_settings(profile), indent=2, ensure_ascii=False
        ),
        REALM_GROUPS: json.dumps(realm_groups(profile), indent=2),
    }
    if profile.get("issuer"):
        files[TRUSTED_ISSUER] = (DIRECTORY / profile["issuer"]).read_text()
    written = []
    for file_name, content in files.items():
        path = directory / file_name
        path.write_text(content)
        written.append(path)
    return written


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("organization", choices=names())
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--voting-portal-url", default="http://127.0.0.1:3000")
    parser.add_argument("--tag", default="demo")
    args = parser.parse_args(argv)
    for path in write(args.organization, args.out, args.voting_portal_url, args.tag):
        print(path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
