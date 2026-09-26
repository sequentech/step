# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The named scenarios, the stages that build them and the links they print."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from scripts.e2e.journeys import fixtures


class Stage(Enum):
    """A state of the scenario's event, checked against the backend."""

    EVENT = "event"
    VOTERS = "voters"
    KEYS = "keys"
    BALLOTS = "ballots"
    ONLINE_OPEN = "online-open"
    KIOSK_OPEN = "kiosk-open"
    VOTES = "votes"
    ONLINE_CLOSED = "online-closed"
    TALLY = "tally"
    RESULTS = "results"


STAGE_DESCRIPTIONS = {
    Stage.EVENT: "synthetic election event imported",
    Stage.VOTERS: "synthetic voters imported",
    Stage.KEYS: "automatic keys ceremony completed",
    Stage.BALLOTS: "ballot styles published",
    Stage.ONLINE_OPEN: "online voting open",
    Stage.KIOSK_OPEN: "kiosk voting open",
    Stage.VOTES: "synthetic votes cast",
    Stage.ONLINE_CLOSED: "online voting closed",
    Stage.TALLY: "electoral results tallied",
    Stage.RESULTS: "results published",
}

# Stages the braid trustee services take part in.
TRUSTEE_STAGES = frozenset({Stage.KEYS, Stage.TALLY})


class Link(Enum):
    VOTING = "voting"
    KIOSK = "kiosk"
    VERIFIER = "verifier"
    RESULTS = "results"
    ADMIN = "admin"


# Voters per area of the journeys' fixture (scripts/e2e/journeys/fixtures.py).
CENSUS = {"A": 4, "B": 2, "C": 2}


@dataclass(frozen=True)
class Vote:
    area: str
    # Position of the voter in the area's census, from 1.
    voter: int
    candidate: str


@dataclass(frozen=True)
class Scenario:
    name: str
    summary: str
    stages: tuple[Stage, ...]
    links: tuple[Link, ...]
    votes: tuple[Vote, ...] = ()

    @property
    def needs_trustees(self) -> bool:
        return any(stage in TRUSTEE_STAGES for stage in self.stages)


SCENARIOS = (
    Scenario(
        name="kiosk-voter",
        summary="kiosk voting open and voters ready to sign in at the kiosk URL",
        stages=(
            Stage.EVENT,
            Stage.VOTERS,
            Stage.KEYS,
            Stage.BALLOTS,
            Stage.KIOSK_OPEN,
        ),
        links=(Link.KIOSK, Link.VERIFIER, Link.ADMIN),
    ),
    Scenario(
        name="completed-ceremony",
        summary="keys ceremony completed, ballots published and online voting open",
        stages=(
            Stage.EVENT,
            Stage.VOTERS,
            Stage.KEYS,
            Stage.BALLOTS,
            Stage.ONLINE_OPEN,
        ),
        links=(Link.VOTING, Link.VERIFIER, Link.ADMIN),
    ),
    Scenario(
        name="published-results",
        summary="votes cast, online voting closed, results tallied and published",
        stages=(
            Stage.EVENT,
            Stage.VOTERS,
            Stage.KEYS,
            Stage.BALLOTS,
            Stage.ONLINE_OPEN,
            Stage.VOTES,
            Stage.ONLINE_CLOSED,
            Stage.TALLY,
            Stage.RESULTS,
        ),
        links=(Link.RESULTS, Link.ADMIN),
        votes=(
            Vote("A", 1, "Alice"),
            Vote("A", 2, "Alice"),
            Vote("A", 3, "Bob"),
            Vote("B", 1, "Erin"),
            Vote("C", 1, "Frank"),
        ),
    ),
)


class CatalogError(ValueError):
    """No scenario has the requested name."""


def scenario_names() -> list[str]:
    return [scenario.name for scenario in SCENARIOS]


def find_scenario(name: str) -> Scenario:
    for scenario in SCENARIOS:
        if scenario.name == name:
            return scenario
    raise CatalogError(
        f"unknown scenario {name!r}; choose one of {', '.join(scenario_names())}"
    )


def census(scenario: Scenario) -> list[fixtures.Voter]:
    """The scenario's synthetic voters; each scenario has its own event realm."""
    return fixtures.census(scenario.name, CENSUS)


def census_usernames(scenario: Scenario) -> dict[str, list[str]]:
    usernames: dict[str, list[str]] = {}
    for voter in census(scenario):
        usernames.setdefault(voter.area, []).append(voter.username)
    return usernames


def voter_username(scenario: Scenario, vote: Vote) -> str:
    return census_usernames(scenario)[vote.area][vote.voter - 1]
