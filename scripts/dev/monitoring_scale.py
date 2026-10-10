# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""``step-dev monitoring_scale``: seed an existing event with synthetic voters.

Fills an election event of the checkout's running stack with N synthetic
voters carrying the attributes the monitoring projection reads, and
optionally with cast votes and enrollment applications, so the monitoring
snapshot and dashboards can be measured at scale. ``--cleanup`` removes
exactly what a seed created and nothing else.

- Voters go through the platform's own importer (``step-cli step
  import-voters``), which COPYs a CSV straight into the event realm's Keycloak
  tables. Their usernames start with ``mon-scale-`` and their emails end with
  ``@mon-scale.invalid``; cleanup deletes only accounts matching both.
- Votes and applications are inserted through Hasura with the admin secret,
  each carrying the ``mon-scale-seed`` annotation that cleanup filters on.
  The votes have no ballot content: a tally of the event would fail on them,
  so they are opt-in (``--with-votes``) and meant for a throwaway event.

The stack's clients are imported only when a command talks to it; the pure
planning functions work without any settings, which the tests rely on.
"""

from __future__ import annotations

import argparse
import base64
import csv
import hashlib
import os
import random
import shutil
import sys
import time
import urllib.parse
import uuid
from collections.abc import Iterator, Mapping, Sequence
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import UTC, date, datetime, timedelta
from pathlib import Path
from typing import Any

PREFIX = "mon-scale-"
EMAIL_DOMAIN = "@mon-scale.invalid"
# The annotation key every inserted vote and application carries.
MARKER = "mon-scale-seed"
AREA_ATTRIBUTE = "area-id"
# Above this many voters a seed needs --force.
MAX_VOTERS = 50_000
DEFAULT_SEED = 13624
# Keycloak's pbkdf2-sha256 parameters, as the importer writes them.
PBKDF2_ITERATIONS = 27_500
PBKDF2_KEY_BYTES = 32
USER_PAGE = 1_000
ACCEPTED = "ACCEPTED"
REJECTED = "REJECTED"
PENDING = "PENDING"
APPLICATION_STATUSES = ((ACCEPTED, 0.6), (REJECTED, 0.25), (PENDING, 0.15))
REJECTION_REASONS = (
    "insufficient-information",
    "no-matching-voter",
    "voter-already-approved",
    "other",
)
# Age band shares of the comelec preset's bands (18-24, 25-39, 40-59, 60+).
AGE_BANDS = (((18, 24), 0.15), ((25, 39), 0.35), ((40, 59), 0.35), ((60, 90), 0.15))
SEXES = (("M", 0.49), ("F", 0.49), ("", 0.02))
STATUSES = (("Land-based", 0.78), ("Seafarer", 0.2), ("", 0.02))
VERIFIED = "VERIFIED"
VERIFIED_SHARE = 0.4
UNKNOWN_COUNTRY_SHARE = 0.01
# Countries with a second post, "Country/Consulate".
CONSULATES = 15
COUNTRIES = (
    "Saudi Arabia", "United Arab Emirates", "United States", "Canada", "Qatar",
    "Kuwait", "Japan", "Hong Kong", "Singapore", "Italy", "United Kingdom",
    "Australia", "Taiwan", "Malaysia", "South Korea", "Oman", "Bahrain",
    "Germany", "Spain", "France", "New Zealand", "Israel", "Lebanon", "Jordan",
    "Macau", "China", "Brunei", "Greece", "Cyprus", "Switzerland", "Austria",
    "Netherlands", "Belgium", "Ireland", "Norway", "Sweden", "Denmark",
    "Finland", "Poland", "Czech Republic", "Hungary", "Portugal", "Romania",
    "Turkey", "Russia", "Ukraine", "Egypt", "Libya", "Nigeria", "Kenya",
    "South Africa", "Ethiopia", "Morocco", "Algeria", "Tunisia", "Sudan",
    "Iraq", "Iran", "Pakistan", "India", "Bangladesh", "Sri Lanka", "Nepal",
    "Thailand", "Vietnam", "Cambodia", "Laos", "Myanmar", "Indonesia",
    "Timor-Leste", "Papua New Guinea", "Fiji", "Palau", "Guam",
    "Marshall Islands", "Micronesia", "Solomon Islands", "Vanuatu", "Samoa",
    "Tonga", "Mexico", "Brazil", "Argentina", "Chile", "Peru", "Colombia",
    "Venezuela", "Panama", "Costa Rica", "Cuba", "Dominican Republic",
    "Jamaica", "Trinidad and Tobago", "Bahamas", "Barbados", "Ecuador",
    "Bolivia", "Uruguay", "Paraguay", "Guatemala", "Honduras", "El Salvador",
    "Nicaragua", "Belize", "Haiti", "Iceland", "Luxembourg", "Malta",
    "Slovakia", "Slovenia", "Croatia", "Serbia", "Bulgaria", "Lithuania",
    "Latvia", "Estonia", "Belarus", "Moldova", "Georgia", "Armenia",
    "Azerbaijan", "Kazakhstan", "Uzbekistan", "Kyrgyzstan", "Tajikistan",
    "Turkmenistan", "Mongolia", "Afghanistan", "Maldives", "Bhutan",
    "Mauritius", "Seychelles", "Madagascar", "Mozambique", "Tanzania",
    "Uganda", "Rwanda", "Ghana", "Ivory Coast", "Senegal", "Cameroon",
    "Angola", "Zambia", "Zimbabwe", "Botswana", "Namibia", "Gabon",
    "Equatorial Guinea", "Djibouti", "Somalia", "Yemen", "Syria",
    "Bosnia and Herzegovina", "Albania", "North Macedonia", "Montenegro",
)  # fmt: skip


class SeedError(RuntimeError):
    """The seed or cleanup cannot go on; nothing further is written."""


@dataclass(frozen=True)
class Area:
    id: str
    name: str
    # The elections the area votes in, through its contests.
    elections: tuple[str, ...]


@dataclass(frozen=True)
class Voter:
    username: str
    area: Area
    country: str
    sex: str
    date_of_birth: str
    status: str
    verified: bool

    @property
    def email(self) -> str:
        return f"{self.username}{EMAIL_DOMAIN}"


@dataclass(frozen=True)
class Account:
    """A seeded voter as the event realm holds it."""

    user_id: str
    username: str
    area_id: str | None


@dataclass(frozen=True)
class Credential:
    """One password hash every seeded voter shares, computed once."""

    salt: str
    hashed: str
    iterations: int


# Pure planning


def check_voter_count(count: int, force: bool) -> None:
    if count < 1:
        raise SeedError("--voters must be at least 1")
    if count > MAX_VOTERS and not force:
        raise SeedError(
            f"{count} voters is above the sanity limit of {MAX_VOTERS}; pass --force "
            "to seed that many"
        )


def check_uuid(name: str, value: str) -> None:
    try:
        uuid.UUID(value)
    except ValueError as error:
        raise SeedError(f"{name} must be a UUID, got {value!r}") from error


def check_share(name: str, value: float) -> None:
    if not 0.0 <= value <= 1.0:
        raise SeedError(f"{name} must be between 0 and 1, got {value}")


def batches(items: Sequence[Any], size: int) -> Iterator[Sequence[Any]]:
    """Consecutive slices of at most ``size`` items."""
    if size < 1:
        raise SeedError(f"batch size must be at least 1, got {size}")
    for start in range(0, len(items), size):
        yield items[start : start + size]


def batch_count(total: int, size: int) -> int:
    return -(-total // size) if total else 0


def usable_areas(
    areas: Sequence[Mapping[str, Any]],
    area_contests: Sequence[Mapping[str, Any]],
    contests: Sequence[Mapping[str, Any]],
) -> tuple[list[Area], list[str]]:
    """The areas voters can be put in, and warnings about the others.

    The importer finds an area by name, so an area whose name another area
    shares is left out. Areas that vote in no election are used only when no
    area does: the projection counts no voter of such an area.
    """
    warnings = []
    names: dict[str, int] = {}
    for area in areas:
        names[area["name"]] = names.get(area["name"], 0) + 1
    duplicated = sorted(name for name, count in names.items() if count > 1)
    if duplicated:
        warnings.append(
            "left out areas sharing a name, which the importer cannot tell apart: "
            + ", ".join(duplicated)
        )
    election_of = {contest["id"]: contest.get("election_id") for contest in contests}
    elections: dict[str, set[str]] = {}
    for link in area_contests:
        election = election_of.get(link.get("contest_id"))
        if link.get("area_id") and election:
            elections.setdefault(link["area_id"], set()).add(election)
    candidates = sorted(
        (
            Area(area["id"], area["name"], tuple(sorted(elections.get(area["id"], ()))))
            for area in areas
            if names[area["name"]] == 1 and area["name"]
        ),
        key=lambda area: area.id,
    )
    voting = [area for area in candidates if area.elections]
    if voting:
        silent = len(candidates) - len(voting)
        if silent:
            warnings.append(f"left out {silent} areas that vote in no election")
        return voting, warnings
    if candidates:
        warnings.append(
            "no area votes in an election (no area_contest rows): the projection "
            "counts none of these voters and no votes can be cast"
        )
    return candidates, warnings


def _weighted(rng: random.Random, choices: Sequence[tuple[Any, float]]) -> Any:
    values, weights = zip(*choices, strict=True)
    return rng.choices(values, weights)[0]


def posts() -> list[tuple[str, float]]:
    """Country/Post values with a long-tailed share, as a census has them."""
    weighted = []
    for rank, country in enumerate(COUNTRIES):
        weight = 1.0 / (rank + 1) ** 0.9
        if rank < CONSULATES:
            weighted.append((f"{country}/Embassy", weight * 0.7))
            weighted.append((f"{country}/Consulate General", weight * 0.3))
        else:
            weighted.append((f"{country}/Embassy", weight))
    return weighted


def birth_date(rng: random.Random, on: date) -> str:
    low, high = _weighted(rng, AGE_BANDS)
    age = rng.randint(low, high)
    # Born in the year up to `age` years before `on`, so as to be `age` then.
    try:
        latest = on.replace(year=on.year - age)
    except ValueError:  # 29 February
        latest = on.replace(year=on.year - age, day=28)
    return (latest - timedelta(days=rng.randint(0, 364))).isoformat()


def generate_voters(
    count: int, areas: Sequence[Area], rng: random.Random, on: date
) -> list[Voter]:
    """``count`` synthetic voters spread unevenly over ``areas``."""
    if not areas:
        raise SeedError("the event has no area to put voters in")
    area_weights = [rng.uniform(0.5, 2.0) for _ in areas]
    country_choices = posts()
    width = max(6, len(str(count)))
    voters = []
    for index in range(count):
        country = (
            ""
            if rng.random() < UNKNOWN_COUNTRY_SHARE
            else _weighted(rng, country_choices)
        )
        voters.append(
            Voter(
                username=f"{PREFIX}{index:0{width}d}",
                area=rng.choices(areas, area_weights)[0],
                country=country,
                sex=_weighted(rng, SEXES),
                date_of_birth=birth_date(rng, on),
                status=_weighted(rng, STATUSES),
                verified=rng.random() < VERIFIED_SHARE,
            )
        )
    return voters


CENSUS_COLUMNS = (
    "username",
    "email",
    "email_verified",
    "group_name",
    "password_salt",
    "hashed_password",
    "num_of_iterations",
    "area_name",
    "country",
    "sex",
    "dateOfBirth",
    "landBasedOrSeafarer",
    "sequent.read-only.id-card-number-validated",
)


def credential(password: str, salt: bytes) -> Credential:
    """The pbkdf2-sha256 hash Keycloak checks, given to the importer ready-made.

    The importer would otherwise hash each row's password itself, which is
    most of its time at scale.
    """
    hashed = hashlib.pbkdf2_hmac(
        "sha256", password.encode(), salt, PBKDF2_ITERATIONS, PBKDF2_KEY_BYTES
    )
    return Credential(
        base64.b64encode(salt).decode(),
        base64.b64encode(hashed).decode(),
        PBKDF2_ITERATIONS,
    )


def census_rows(
    voters: Sequence[Voter], group: str, secret: Credential
) -> Iterator[list[str]]:
    """The importer's CSV rows, header first. An empty cell sets no attribute."""
    yield list(CENSUS_COLUMNS)
    for voter in voters:
        yield [
            voter.username,
            voter.email,
            "true",
            group,
            secret.salt,
            secret.hashed,
            str(secret.iterations),
            voter.area.name,
            voter.country,
            voter.sex,
            voter.date_of_birth,
            voter.status,
            VERIFIED if voter.verified else "",
        ]


def write_census(
    path: Path, voters: Sequence[Voter], group: str, secret: Credential
) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        csv.writer(handle).writerows(census_rows(voters, group, secret))


def is_seeded_user(user: Mapping[str, Any]) -> bool:
    """Whether a Keycloak user is one a seed created: both marks must match."""
    username = str(user.get("username") or "")
    email = str(user.get("email") or "")
    return username.startswith(PREFIX) and email.endswith(EMAIL_DOMAIN)


def seeded_account(user: Mapping[str, Any]) -> Account:
    areas = (user.get("attributes") or {}).get(AREA_ATTRIBUTE) or []
    return Account(user["id"], user["username"], areas[0] if areas else None)


def marked_rows(tenant_id: str, event_id: str) -> dict[str, Any]:
    """The Hasura filter for the votes and applications a seed inserted."""
    return {
        "tenant_id": {"_eq": tenant_id},
        "election_event_id": {"_eq": event_id},
        "annotations": {"_has_key": MARKER},
    }


def allows_revote(num_allowed_revotes: int | None) -> bool:
    """Whether a second valid vote passes the cast_vote revote trigger.

    The trigger reads a missing limit as 1 vote and 0 as unlimited.
    """
    allowed = 1 if num_allowed_revotes is None else num_allowed_revotes
    return allowed == 0 or allowed >= 2


def _stamp(moment: datetime) -> str:
    return moment.astimezone(UTC).isoformat()


def vote_time(rng: random.Random, start: datetime, days: int) -> datetime:
    """A moment in ``days`` days from ``start``, busier later and by day."""
    day = rng.choices(range(days), [1.0 + index for index in range(days)])[0]
    # Mostly daytime, peaking in the afternoon.
    return start + timedelta(days=day + rng.betavariate(2.5, 2.0))


@dataclass(frozen=True)
class VotePlan:
    turnout: float
    revote_share: float
    days: int
    now: datetime


def plan_votes(
    accounts: Sequence[Account],
    areas: Mapping[str, Area],
    revote_limits: Mapping[str, int | None],
    plan: VotePlan,
    rng: random.Random,
    tenant_id: str,
    event_id: str,
    tag: str,
) -> list[dict[str, Any]]:
    """Valid votes of about ``turnout`` of the voters, in each election of
    their area, and a later revote where the election allows one."""
    if plan.days < 1:
        raise SeedError("--vote-days must be at least 1")
    start = plan.now - timedelta(days=plan.days)
    latest = plan.now - timedelta(seconds=1)
    votes = []

    def vote(account: Account, area: Area, election: str, at: datetime) -> None:
        stamp = _stamp(min(at, latest))
        votes.append(
            {
                "tenant_id": tenant_id,
                "election_event_id": event_id,
                "election_id": election,
                "area_id": area.id,
                "voter_id_string": account.user_id,
                "status": "valid",
                "created_at": stamp,
                "last_updated_at": stamp,
                "annotations": {MARKER: tag},
            }
        )

    for account in accounts:
        area = areas.get(account.area_id or "")
        if area is None or not area.elections or rng.random() >= plan.turnout:
            continue
        first = vote_time(rng, start, plan.days)
        revotes = rng.random() < plan.revote_share
        for offset, election in enumerate(area.elections):
            at = first + timedelta(seconds=30 * offset)
            vote(account, area, election, at)
            if revotes and allows_revote(revote_limits.get(election)):
                vote(account, area, election, at + timedelta(hours=rng.uniform(1, 48)))
    return votes


def plan_applications(
    accounts: Sequence[Account],
    share: float,
    days: int,
    now: datetime,
    rng: random.Random,
    tenant_id: str,
    event_id: str,
    tag: str,
) -> list[dict[str, Any]]:
    """Enrollment applications of about ``share`` of the voters: accepted,
    rejected with a reason, or still pending."""
    start = now - timedelta(days=days + 7)
    applications = []
    for account in accounts:
        if account.area_id is None or rng.random() >= share:
            continue
        status = _weighted(rng, APPLICATION_STATUSES)
        created = start + timedelta(days=rng.uniform(0, days + 6))
        updated = created
        annotations: dict[str, Any] = {MARKER: tag}
        if status != PENDING:
            updated = min(created + timedelta(hours=rng.uniform(1, 72)), now)
        if status == REJECTED:
            annotations["rejection_reason"] = rng.choice(REJECTION_REASONS)
        applications.append(
            {
                "tenant_id": tenant_id,
                "election_event_id": event_id,
                "area_id": account.area_id,
                "applicant_id": account.user_id,
                "status": status,
                "verification_type": "MANUAL",
                "applicant_data": {"username": account.username},
                "annotations": annotations,
                "created_at": _stamp(created),
                "updated_at": _stamp(updated),
            }
        )
    return applications


@dataclass(frozen=True)
class Options:
    event_id: str
    tenant_id: str
    voters: int
    with_votes: bool
    with_applications: bool
    turnout: float
    revote_share: float
    vote_days: int
    application_share: float
    import_batch: int
    db_batch: int
    seed: int


def describe_plan(options: Options, realm: str, areas: Sequence[Area]) -> list[str]:
    """What a seed would do, for --dry-run and before a real run."""
    elections = sum(len(area.elections) for area in areas) / max(len(areas), 1)
    votes = round(options.voters * options.turnout * elections)
    lines = [
        f"realm        {realm}",
        f"areas        {len(areas)} used, {elections:.1f} elections each on average",
        f"voters       {options.voters} ({PREFIX}*, *{EMAIL_DOMAIN}), imported in "
        f"{batch_count(options.voters, options.import_batch)} file(s) of up to "
        f"{options.import_batch} with step-cli import-voters",
    ]
    if options.with_votes:
        lines.append(
            f"votes        ~{votes} valid first votes ({options.turnout:.0%} turnout) "
            f"over {options.vote_days} days, +~{options.revote_share:.0%} revotes "
            "where the election allows them; Hasura inserts of up to "
            f"{options.db_batch}, annotated {MARKER}"
        )
    else:
        lines.append("votes        none (--with-votes adds them)")
    if options.with_applications:
        applications = round(options.voters * options.application_share)
        lines.append(
            f"applications ~{applications} ({options.application_share:.0%} of "
            "voters: accepted, rejected with a reason, pending), annotated "
            f"{MARKER}"
        )
    else:
        lines.append("applications none (--with-applications adds them)")
    return lines


# The stack


EVENT = """query ($tenant: uuid!, $event: uuid!) {
  sequent_backend_election_event(
    where: {tenant_id: {_eq: $tenant}, id: {_eq: $event}}
  ) { id }
  sequent_backend_area(
    where: {tenant_id: {_eq: $tenant}, election_event_id: {_eq: $event}}
  ) { id name }
  sequent_backend_area_contest(
    where: {tenant_id: {_eq: $tenant}, election_event_id: {_eq: $event}}
  ) { area_id contest_id }
  sequent_backend_contest(
    where: {tenant_id: {_eq: $tenant}, election_event_id: {_eq: $event}}
  ) { id election_id }
  sequent_backend_election(
    where: {tenant_id: {_eq: $tenant}, election_event_id: {_eq: $event}}
  ) { id num_allowed_revotes }
  sequent_backend_tally_session_aggregate(
    where: {tenant_id: {_eq: $tenant}, election_event_id: {_eq: $event}}
  ) { aggregate { count } }
}"""
MARKED = """query ($votes: sequent_backend_cast_vote_bool_exp!,
                   $applications: sequent_backend_applications_bool_exp!) {
  sequent_backend_cast_vote_aggregate(where: $votes) { aggregate { count } }
  sequent_backend_applications_aggregate(where: $applications) {
    aggregate { count }
  }
}"""
INSERT_VOTES = """mutation ($objects: [sequent_backend_cast_vote_insert_input!]!) {
  insert_sequent_backend_cast_vote(objects: $objects) { affected_rows }
}"""
INSERT_APPLICATIONS = """mutation (
  $objects: [sequent_backend_applications_insert_input!]!
) {
  insert_sequent_backend_applications(objects: $objects) { affected_rows }
}"""
DELETE_MARKED = """mutation ($votes: sequent_backend_cast_vote_bool_exp!,
                      $applications: sequent_backend_applications_bool_exp!) {
  delete_sequent_backend_cast_vote(where: $votes) { affected_rows }
  delete_sequent_backend_applications(where: $applications) { affected_rows }
}"""


def say(message: str = "") -> None:
    print(message, flush=True)


@contextmanager
def phase(name: str) -> Iterator[None]:
    began = time.monotonic()
    say(f"{name} ...")
    yield
    say(f"{name}: {time.monotonic() - began:.1f}s")


@dataclass
class Event:
    realm: str
    areas: list[Area]
    revote_limits: dict[str, int | None]
    tallied: bool


class Stack:
    """The checkout's running stack, through the backend journeys' clients."""

    def __init__(self, tenant_id: str, event_id: str) -> None:
        from scripts.dev.scenario import cli

        self.context = cli._context(cli.OutputFormat.TEXT)
        self.backend = cli._backend(self.context, None)
        # Imported after _backend has set the environment they read.
        from scripts.e2e.journeys import bootstrap, client

        self.bootstrap = bootstrap
        self.client = client
        if tenant_id != client.TENANT_ID:
            raise SeedError(
                f"tenant {tenant_id} is not this stack's super tenant "
                f"{client.TENANT_ID}, the only one its administrator can import into"
            )
        self.tenant_id = tenant_id
        self.event_id = event_id
        self.realm = client.event_realm(event_id)
        self.hasura = client.Hasura.admin()
        self.keycloak = client.Keycloak()
        self.voter_group = self.context.environment["KEYCLOAK_VOTER_GROUP_NAME"]
        self.cli: Any = None

    def event_data(self) -> dict[str, Any]:
        """The event's rows; fails unless the event and its realm exist."""
        data = self.hasura.query(
            EVENT, {"tenant": self.tenant_id, "event": self.event_id}
        )
        if not data["sequent_backend_election_event"]:
            raise SeedError(
                f"election event {self.event_id} does not exist in tenant "
                f"{self.tenant_id}"
            )
        response = self.client.Http().get(
            f"{self.client.KEYCLOAK_URL}/admin/realms/{self.realm}",
            headers={"Authorization": f"Bearer {self.keycloak.admin_token()}"},
        )
        if response.status != 200:
            raise SeedError(
                f"the event realm {self.realm} is missing ({response.status})"
            )
        return data

    def check_voter_group(self) -> None:
        """The importer silently skips a group the realm lacks; the projection
        then counts none of the voters."""
        query = urllib.parse.urlencode({"search": self.voter_group})
        groups = self.keycloak.admin("GET", f"{self.realm}/groups?{query}") or []
        if not any(group.get("name") == self.voter_group for group in groups):
            raise SeedError(
                f"the realm {self.realm} has no group {self.voter_group!r} "
                "(KEYCLOAK_VOTER_GROUP_NAME), which the projection counts voters by"
            )

    def event(self) -> Event:
        data = self.event_data()
        areas, warnings = usable_areas(
            data["sequent_backend_area"],
            data["sequent_backend_area_contest"],
            data["sequent_backend_contest"],
        )
        for warning in warnings:
            say(f"warning: {warning}")
        if not areas:
            raise SeedError(f"event {self.event_id} has no area to put voters in")
        return Event(
            self.realm,
            areas,
            {
                row["id"]: row.get("num_allowed_revotes")
                for row in data["sequent_backend_election"]
            },
            data["sequent_backend_tally_session_aggregate"]["aggregate"]["count"] > 0,
        )

    def seeded_users(self) -> list[dict[str, Any]]:
        """Every account a seed created in the realm, with its attributes."""
        users = []
        first = 0
        while True:
            query = urllib.parse.urlencode(
                {
                    "username": PREFIX,
                    "first": first,
                    "max": USER_PAGE,
                    "briefRepresentation": "false",
                }
            )
            page = self.keycloak.admin("GET", f"{self.realm}/users?{query}")
            if not page:
                return users
            first += len(page)
            users.extend(user for user in page if is_seeded_user(user))

    def marked(self) -> tuple[int, int]:
        where = marked_rows(self.tenant_id, self.event_id)
        data = self.hasura.query(MARKED, {"votes": where, "applications": where})
        return (
            data["sequent_backend_cast_vote_aggregate"]["aggregate"]["count"],
            data["sequent_backend_applications_aggregate"]["aggregate"]["count"],
        )

    def open_cli(self, explicit: str | None) -> None:
        from scripts.dev.scenario.settings import find_step_cli

        binary = find_step_cli(
            explicit, self.context.checkout.root, os.environ.get("PATH")
        )
        say("waiting for the backend services")
        self.backend.check_services()
        self.backend.authenticate()
        self.cli = self.client.StepCli(binary)

    def import_voters(self, path: Path) -> None:
        # A fresh login per file, as the scenarios run step-cli.
        self.bootstrap.configure_step_cli(self.cli)
        self.cli.step(
            "import-voters",
            "--election-event-id",
            self.event_id,
            "--file-path",
            str(path),
            "--is-local",
        )

    def insert(self, mutation: str, rows: Sequence[dict[str, Any]], size: int) -> int:
        written = 0
        for batch in batches(rows, size):
            data = self.hasura.query(mutation, {"objects": list(batch)})
            written += next(iter(data.values()))["affected_rows"]
        return written

    def delete_marked(self) -> tuple[int, int]:
        where = marked_rows(self.tenant_id, self.event_id)
        data = self.hasura.query(DELETE_MARKED, {"votes": where, "applications": where})
        return (
            data["delete_sequent_backend_cast_vote"]["affected_rows"],
            data["delete_sequent_backend_applications"]["affected_rows"],
        )

    def delete_users(self, users: Sequence[Mapping[str, Any]], workers: int) -> int:
        self.keycloak.admin_token()

        def delete(user: Mapping[str, Any]) -> None:
            if not is_seeded_user(user):
                raise SeedError(f"refusing to delete {user.get('username')}")
            self.keycloak.admin(
                "DELETE", f"{self.realm}/users/{user['id']}", expect=(204, 404)
            )

        with ThreadPoolExecutor(max_workers=workers) as pool:
            return len(list(pool.map(delete, users)))

    def close(self) -> None:
        if self.cli is not None:
            shutil.rmtree(self.cli.directory, ignore_errors=True)


# Commands


def next_steps(event_id: str) -> None:
    say(
        "next: the monitoring snapshot sees the voters on its next full pass over "
        f"the realm. For event {event_id} to be refreshed at all its dashboards "
        "must be configured (monitoring_event.dashboard_mode = CONFIGURED); beat "
        "then sends a pass every 30s and a full pass runs every "
        "MONITORING_VOTER_FULL_PASS_SECONDS (300s by default) or when the "
        "event's monitoring settings change. Votes and applications are picked up "
        "on every pass once the voters are projected."
    )


def offline_areas(count: int) -> list[Area]:
    return [
        Area(str(uuid.UUID(int=index + 1)), f"Area {index + 1}", ("election",))
        for index in range(count)
    ]


def command_seed(arguments: argparse.Namespace, options: Options) -> int:
    check_voter_count(options.voters, arguments.force)
    for name, value in (
        ("--turnout", options.turnout),
        ("--revote-share", options.revote_share),
        ("--application-share", options.application_share),
    ):
        check_share(name, value)
    if options.import_batch < 1 or options.db_batch < 1:
        raise SeedError("batch sizes must be at least 1")
    if arguments.offline:
        say("offline plan: the stack is not contacted")
        realm = f"tenant-{options.tenant_id}-event-{options.event_id}"
        for line in describe_plan(options, realm, offline_areas(arguments.areas)):
            say(f"  {line}")
        return 0
    stack = Stack(options.tenant_id, options.event_id)
    try:
        return _seed(stack, arguments, options)
    finally:
        stack.close()


def _seed(stack: Stack, arguments: argparse.Namespace, options: Options) -> int:
    began = time.monotonic()
    with phase("checks"):
        event = stack.event()
        stack.check_voter_group()
        existing = stack.seeded_users()
        votes, applications = stack.marked()
    say(f"event {options.event_id}")
    for line in describe_plan(options, event.realm, event.areas):
        say(f"  {line}")
    say(
        f"  already seeded: {len(existing)} voters, {votes} votes, "
        f"{applications} applications"
    )
    if existing and len(existing) != options.voters:
        raise SeedError(
            f"the realm already holds {len(existing)} seeded voters, not "
            f"{options.voters}; run --cleanup first"
        )
    if options.with_votes and votes:
        raise SeedError(f"the event already holds {votes} seeded votes; run --cleanup")
    if options.with_applications and applications:
        raise SeedError(
            f"the event already holds {applications} seeded applications; run "
            "--cleanup"
        )
    if options.with_votes and event.tallied and not arguments.force:
        raise SeedError(
            "the event already has a tally session; the seeded votes have no ballot "
            "content, so pass --force only for an event whose tally does not matter"
        )
    if arguments.dry_run:
        say("dry run: nothing written")
        return 0
    rng = random.Random(options.seed)
    if not existing:
        voters = generate_voters(
            options.voters, event.areas, rng, datetime.now(UTC).date()
        )
        secret = credential(arguments.password, rng.randbytes(16))
        stack.open_cli(arguments.step_cli)
        with phase(f"import {len(voters)} voters"):
            chunks = list(batches(voters, options.import_batch))
            for number, chunk in enumerate(chunks, 1):
                path = stack.cli.directory / f"{PREFIX}{number}.csv"
                write_census(path, chunk, stack.voter_group, secret)
                with phase(f"  file {number}/{len(chunks)} ({len(chunk)} voters)"):
                    stack.import_voters(path)
    else:
        say(f"voters already seeded ({len(existing)}); not importing")
    with phase("read back the seeded voters"):
        accounts = [seeded_account(user) for user in stack.seeded_users()]
    if len(accounts) != options.voters:
        raise SeedError(
            f"the realm holds {len(accounts)} seeded voters after the import, "
            f"expected {options.voters}"
        )
    tag = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    now = datetime.now(UTC)
    areas = {area.id: area for area in event.areas}
    if options.with_votes:
        plan = VotePlan(options.turnout, options.revote_share, options.vote_days, now)
        rows = plan_votes(
            accounts,
            areas,
            event.revote_limits,
            plan,
            rng,
            options.tenant_id,
            options.event_id,
            tag,
        )
        with phase(f"insert {len(rows)} votes"):
            stack.insert(INSERT_VOTES, rows, options.db_batch)
    if options.with_applications:
        rows = plan_applications(
            accounts,
            options.application_share,
            options.vote_days,
            now,
            rng,
            options.tenant_id,
            options.event_id,
            tag,
        )
        with phase(f"insert {len(rows)} applications"):
            stack.insert(INSERT_APPLICATIONS, rows, options.db_batch)
    say(f"seeded in {time.monotonic() - began:.1f}s")
    next_steps(options.event_id)
    return 0


def command_cleanup(arguments: argparse.Namespace, options: Options) -> int:
    if arguments.offline:
        raise SeedError("--cleanup needs the stack; --offline only plans a seed")
    stack = Stack(options.tenant_id, options.event_id)
    try:
        with phase("find what the seed created"):
            stack.event_data()
            users = stack.seeded_users()
            votes, applications = stack.marked()
        say(
            f"seeded: {len(users)} voters in {stack.realm}, {votes} votes, "
            f"{applications} applications annotated {MARKER}"
        )
        if arguments.dry_run:
            say("dry run: nothing deleted")
            return 0
        with phase("delete the seeded votes and applications"):
            deleted = stack.delete_marked()
        say(f"  deleted {deleted[0]} votes, {deleted[1]} applications")
        with phase(f"delete {len(users)} seeded voters"):
            stack.delete_users(users, arguments.workers)
        say(
            "the next full monitoring pass removes the voters from the projection"
        )
        return 0
    finally:
        stack.close()


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="step-dev monitoring_scale",
        description="Seed an existing election event of the checkout's running "
        "stack with synthetic voters (and optionally votes and applications) for "
        "monitoring at scale, or remove exactly what a seed created. Run it in "
        "the devcontainer, on a throwaway event.",
    )
    parser.add_argument("--event", required=True, help="election event id")
    parser.add_argument(
        "--tenant",
        help="tenant id (default: the stack's SUPER_ADMIN_TENANT_ID, the only one "
        "supported)",
    )
    parser.add_argument("--voters", type=int, default=1000, help="voters to create")
    parser.add_argument(
        "--cleanup", action="store_true", help="delete what seeds created instead"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="read the event and print the plan; write nothing",
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="with --dry-run: print the plan without contacting the stack",
    )
    parser.add_argument(
        "--areas",
        type=int,
        default=20,
        help="areas the --offline plan assumes (default 20)",
    )
    parser.add_argument(
        "--with-votes",
        action="store_true",
        help="also insert valid cast votes. They carry no ballot content, so a "
        "tally of the event fails afterwards: use a throwaway event",
    )
    parser.add_argument(
        "--with-applications",
        action="store_true",
        help="also insert enrollment applications (accepted, rejected, pending)",
    )
    parser.add_argument("--turnout", type=float, default=0.6)
    parser.add_argument(
        "--revote-share",
        type=float,
        default=0.1,
        help="share of voting voters who vote again where the election allows it",
    )
    parser.add_argument(
        "--vote-days", type=int, default=5, help="days the votes spread over"
    )
    parser.add_argument("--application-share", type=float, default=0.3)
    parser.add_argument(
        "--import-batch",
        type=int,
        default=10_000,
        help="voters per imported file (default 10000)",
    )
    parser.add_argument(
        "--db-batch", type=int, default=1_000, help="rows per Hasura insert"
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=8,
        help="parallel Keycloak deletions in --cleanup",
    )
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED, help="random seed")
    parser.add_argument(
        "--password",
        default="Mon-scale-2026!",
        help="the synthetic voters' password",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help=f"allow more than {MAX_VOTERS} voters, or votes in a tallied event",
    )
    parser.add_argument("--step-cli", help="the step-cli binary")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    arguments = _parser().parse_args(argv)
    try:
        if arguments.offline and not arguments.dry_run:
            raise SeedError("--offline only goes with --dry-run")
        check_uuid("--event", arguments.event)
        tenant = arguments.tenant or os.environ.get("SUPER_ADMIN_TENANT_ID")
        if not tenant and arguments.offline:
            tenant = str(uuid.UUID(int=0))
        if not tenant:
            from scripts.dev.mode.checkout import load_checkout

            tenant = load_checkout().env.get("SUPER_ADMIN_TENANT_ID", "")
        check_uuid("--tenant", tenant)
        options = Options(
            event_id=arguments.event,
            tenant_id=tenant,
            voters=arguments.voters,
            with_votes=arguments.with_votes,
            with_applications=arguments.with_applications,
            turnout=arguments.turnout,
            revote_share=arguments.revote_share,
            vote_days=arguments.vote_days,
            application_share=arguments.application_share,
            import_batch=arguments.import_batch,
            db_batch=arguments.db_batch,
            seed=arguments.seed,
        )
        if arguments.cleanup:
            return command_cleanup(arguments, options)
        return command_seed(arguments, options)
    except KeyboardInterrupt:
        print("step-dev monitoring_scale: interrupted", file=sys.stderr)
        return 130
    except (RuntimeError, ValueError, OSError, AssertionError) as error:
        # SeedError and the checkout's errors are RuntimeErrors or
        # ValueErrors; the journeys' clients report failed requests and
        # step-cli commands as AssertionErrors.
        print(f"step-dev monitoring_scale: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
