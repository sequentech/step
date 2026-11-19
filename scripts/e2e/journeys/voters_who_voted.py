# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Synthetic four-Post event for the list of voters who actually voted.

Every name is invented. The event has four Posts (elections):

- Post 1 (two areas): a voter who revotes, a voter whose account is disabled
  after voting, a voter who never votes, and a voter in the second area.
- Post 2: a voter and a voter who never votes.
- Post 3: a voter whose name starts with ``=``, which a spreadsheet would run,
  and a voter with a Chinese name in an area whose name has a Chinese
  character. The PDF renderer has no CJK font, so the PDF cannot show them;
  the CSV keeps them exactly.
- Post 4: one voter, who never votes: the Post's list is empty.
"""

import copy
import csv
import io
import uuid
from dataclasses import dataclass
from pathlib import Path

from scripts.e2e.journeys import fixtures


@dataclass(frozen=True)
class Post:
    key: str
    title: str
    areas: tuple  # area keys


@dataclass(frozen=True)
class Person:
    key: str
    area: str
    first_name: str
    last_name: str
    votes: int  # accepted ballots cast: 0 never votes, 2 revotes
    disabled_after_voting: bool = False


POSTS = (
    Post("p1", "Sample Post 1 · Norte", ("n1", "n2")),
    Post("p2", "Sample Post 2 · Sur", ("s1",)),
    Post("p3", "Sample Post 3 · Este", ("e1",)),
    Post("p4", "Sample Post 4 · Oeste", ("w1",)),
)
AREA_NAMES = {
    "n1": "Área Norte Uno",
    "n2": "Área Norte Dos",
    "s1": "Área Sur",
    "e1": "Área Este 東",
    "w1": "Área Oeste",
}
PEOPLE = (
    Person("revoter", "n1", "María José", "Abad", votes=2),
    Person("disabled", "n1", "Ayşe", "Ölçer", votes=1, disabled_after_voting=True),
    Person("nonvoter", "n1", "Ana", "Reyes", votes=0),
    Person("second-area", "n2", "Ma. Cristina", "Dela Peña", votes=1),
    Person("south", "s1", "Thị Hoa", "Nguyễn", votes=1),
    Person("south-nonvoter", "s1", "Pedro", "Santos", votes=0),
    Person("formula", "e1", "Juan", "=Cruz", votes=1),
    Person("cjk", "e1", "小龍", "李", votes=1),
    Person("west-nonvoter", "w1", "Luz", "Ñúñez", votes=0),
)
POST_OF_AREA = {area: post for post in POSTS for area in post.areas}


def username(tag, person):
    return f"e2e-{tag}-{person.key}"


def election_event(voting_portal_url, tag):
    """The import document: the journeys' event, made into four Posts."""
    document = fixtures.election_event(voting_portal_url, tag)
    template_election = document["elections"][0]
    template_area = document["areas"][0]
    template_contest = document["contests"][0]
    template_candidate = next(
        c for c in document["candidates"] if c["contest_id"] == template_contest["id"]
    )
    elections, areas, contests, candidates, links = [], [], [], [], []
    for post in POSTS:
        election = copy.deepcopy(template_election)
        election["id"] = str(uuid.uuid4())
        election["external_id"] = f"e2e-voted-{post.key}-{tag}"
        election["num_allowed_revotes"] = 2
        election["presentation"]["grace_period_policy"] = "no-grace-period"
        election["presentation"]["grace_period_secs"] = 0
        fixtures._name(election, post.title)
        elections.append(election)
        for area_key in post.areas:
            area = copy.deepcopy(template_area)
            area.update(id=str(uuid.uuid4()), name=AREA_NAMES[area_key])
            contest = copy.deepcopy(template_contest)
            contest.update(
                id=str(uuid.uuid4()),
                election_id=election["id"],
                min_votes=1,
                max_votes=1,
                winning_candidates_num=1,
            )
            fixtures._name(contest, f"Contest {area_key}")
            for name in ("Option One", "Option Two"):
                candidate = copy.deepcopy(template_candidate)
                candidate.update(id=str(uuid.uuid4()), contest_id=contest["id"])
                fixtures._name(candidate, name)
                candidates.append(candidate)
            areas.append(area)
            contests.append(contest)
            links.append(
                {"id": str(uuid.uuid4()), "area_id": area["id"], "contest_id": contest["id"]}
            )
    for item in [*elections, *contests, *candidates]:
        item["annotations"] = {}
    document.update(
        elections=elections,
        areas=areas,
        contests=contests,
        candidates=candidates,
        area_contests=links,
    )
    return document


def write_census(path, tag, document):
    """The voters' CSV for `step-cli step import-voters`, names included."""
    authorization = {
        post.key: election["external_id"]
        for post, election in zip(POSTS, document["elections"])
    }
    with Path(path).open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "username",
                "first_name",
                "last_name",
                "area_name",
                "email",
                "email_verified",
                "password",
                "authorized-election-ids",
            ]
        )
        for person in PEOPLE:
            name = username(tag, person)
            writer.writerow(
                [
                    name,
                    person.first_name,
                    person.last_name,
                    AREA_NAMES[person.area],
                    f"{name}@example.invalid",
                    "true",
                    name,
                    authorization[POST_OF_AREA[person.area].key],
                ]
            )


def expected_rows(tag, post_keys=None):
    """The CSV rows the list must hold: who cast an accepted ballot, once each."""
    rows = []
    for post in POSTS:
        if post_keys is not None and post.key not in post_keys:
            continue
        for person in PEOPLE:
            if POST_OF_AREA[person.area] is post and person.votes > 0:
                rows.append(
                    {
                        "election": post.title,
                        "area": AREA_NAMES[person.area],
                        "last_name": person.last_name,
                        "first_name": person.first_name,
                        "username": username(tag, person),
                    }
                )
    return rows


# Names the PDF must print as they are: the renderer's fonts cover them.
PDF_PRINTED_KEYS = ("revoter", "disabled", "second-area", "south", "formula")
# Font families with CJK glyphs. The renderer images embed none of them
# (DejaVu has no CJK), so the `cjk` voter's name and the 東 of its area print
# as boxes in the PDF, as the user docs say. If one appears, the docs (and
# this check) must change with it.
CJK_FONT_MARKERS = ("CJK", "WenQuanYi", "DroidSansFallback", "SourceHan", "Noto Sans SC",
                    "NotoSansSC", "NotoSansJP", "NotoSansKR", "NotoSansTC", "AR PL", "IPA")


def defused(cell):
    """How the CSV writes a cell a spreadsheet would run."""
    return "'" + cell if cell[:1] in ("=", "+", "-", "@", "\t", "\r") else cell


def read_csv(data):
    """The CSV's rows, after the optional configuration stamp line."""
    text = data.decode("utf-8")
    lines = text.splitlines(keepends=True)
    if lines and not lines[0].startswith("election_id,"):
        lines = lines[1:]
    return list(csv.DictReader(io.StringIO("".join(lines))))
