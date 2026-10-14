# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""What a scenario created, recorded per Compose project under ``.cache``.

The state file and a pair of annotations on the election event together prove
that a scenario created the event; ``reset`` deletes nothing else.
"""

from __future__ import annotations

import contextlib
import fcntl
import json
import os
import secrets
from collections.abc import Iterator
from dataclasses import dataclass, field
from datetime import UTC, datetime
from enum import Enum
from pathlib import Path
from typing import Any

STATE_DIRECTORY = Path(".cache/scenarios")
STATE_VERSION = 1
SCENARIO_ANNOTATION = "step-dev:scenario"
OWNER_ANNOTATION = "step-dev:owner"
# Event import replaces every UUID in the bundle, so the owner token is not one.
OWNER_TOKEN_BYTES = 16


class StateError(RuntimeError):
    """A state file is unreadable or another command holds it."""


def now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


@dataclass
class State:
    scenario: str
    project: str
    tenant_id: str
    owner: str
    event_id: str | None = None
    # Identifiers of the event's elections, areas, contests and candidates.
    ids: dict[str, Any] = field(default_factory=dict)
    # Synthetic voter usernames by area.
    voters: dict[str, list[str]] = field(default_factory=dict)
    # Seconds each stage took the last time it ran.
    stages: dict[str, float] = field(default_factory=dict)
    created_at: str = field(default_factory=now)
    updated_at: str = field(default_factory=now)

    def annotations(self) -> dict[str, str]:
        return {SCENARIO_ANNOTATION: self.scenario, OWNER_ANNOTATION: self.owner}

    def to_document(self) -> dict[str, Any]:
        return {
            "version": STATE_VERSION,
            "scenario": self.scenario,
            "project": self.project,
            "tenantId": self.tenant_id,
            "owner": self.owner,
            "eventId": self.event_id,
            "ids": self.ids,
            "voters": self.voters,
            "stages": self.stages,
            "createdAt": self.created_at,
            "updatedAt": self.updated_at,
        }

    @classmethod
    def from_document(cls, document: Any) -> State:
        if not isinstance(document, dict) or document.get("version") != STATE_VERSION:
            raise StateError(f"not a version {STATE_VERSION} scenario state")
        try:
            return cls(
                scenario=document["scenario"],
                project=document["project"],
                tenant_id=document["tenantId"],
                owner=document["owner"],
                event_id=document.get("eventId"),
                ids=document.get("ids") or {},
                voters=document.get("voters") or {},
                stages=document.get("stages") or {},
                created_at=document.get("createdAt") or "",
                updated_at=document.get("updatedAt") or "",
            )
        except KeyError as error:
            raise StateError(f"scenario state lacks {error}") from error


def new_state(scenario: str, project: str, tenant_id: str) -> State:
    return State(scenario, project, tenant_id, secrets.token_hex(OWNER_TOKEN_BYTES))


class Ownership(Enum):
    OWNED = "owned"
    MISSING = "missing"
    FOREIGN = "foreign"


def ownership(state: State, annotations: dict[str, Any] | None) -> Ownership:
    """Whether the event whose annotations these are was created for ``state``.

    ``annotations`` is None when the event does not exist.
    """
    if annotations is None:
        return Ownership.MISSING
    if all(annotations.get(key) == value for key, value in state.annotations().items()):
        return Ownership.OWNED
    return Ownership.FOREIGN


@dataclass(frozen=True)
class StateStore:
    """The state files of one Compose project's scenarios."""

    directory: Path

    @classmethod
    def for_project(cls, root: Path, project: str) -> StateStore:
        return cls(root / STATE_DIRECTORY / project)

    def path(self, name: str) -> Path:
        return self.directory / f"{name}.json"

    def load(self, name: str) -> State | None:
        path = self.path(name)
        try:
            text = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            return None
        try:
            state = State.from_document(json.loads(text))
        except (json.JSONDecodeError, StateError) as error:
            raise StateError(f"{path}: {error}") from error
        if state.scenario != name:
            raise StateError(f"{path} describes scenario {state.scenario!r}")
        return state

    def save(self, state: State) -> None:
        state.updated_at = now()
        self.directory.mkdir(parents=True, exist_ok=True)
        path = self.path(state.scenario)
        temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
        temporary.write_text(
            json.dumps(state.to_document(), indent=2) + "\n", encoding="utf-8"
        )
        temporary.replace(path)

    def remove(self, name: str) -> None:
        self.path(name).unlink(missing_ok=True)

    @contextlib.contextmanager
    def lock(self, name: str) -> Iterator[None]:
        """Holds the scenario for one command; a second one fails instead of racing."""
        self.directory.mkdir(parents=True, exist_ok=True)
        with (self.directory / f"{name}.lock").open("a") as handle:
            try:
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise StateError(
                    f"another step-dev scenario command is working on {name}"
                ) from error
            yield
