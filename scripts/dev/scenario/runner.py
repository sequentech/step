# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Bringing a scenario up, resetting it and reporting it, over a ``Backend``."""

from __future__ import annotations

import time
from collections.abc import Callable
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Protocol

from .catalog import STAGE_DESCRIPTIONS, Scenario, Stage
from .state import Ownership, State, StateStore, new_state, ownership


class ScenarioError(RuntimeError):
    """The scenario cannot proceed; the message says what to do instead."""


class Backend(Protocol):
    tenant_id: str
    project: str

    def event_annotations(self, event_id: str) -> dict[str, Any] | None:
        """The event's annotations, or None when the tenant has no such event."""

    def annotated_events(self, annotations: dict[str, str]) -> list[str]:
        """The tenant's events carrying all of these annotations."""

    def holds(self, stage: Stage, scenario: Scenario, state: State) -> bool:
        """Whether the event is in the state the stage leads to."""

    def run(self, stage: Stage, scenario: Scenario, state: State) -> None:
        """Brings the event to the stage's state, recording identifiers in ``state``."""

    def point_portals(self, state: State) -> None:
        """Lets the event's portal clients redirect to the configured portals."""

    def delete_event(self, event_id: str) -> None:
        """Deletes the event and waits until the backend no longer has it."""


Say = Callable[[str], None]


class Outcome(Enum):
    CREATED = "created"
    RESUMED = "resumed"
    REUSED = "reused"


@dataclass
class UpReport:
    state: State
    outcome: Outcome
    seconds: float
    ran: dict[str, float] = field(default_factory=dict)


def resume_index(stages: tuple[Stage, ...], holds: Callable[[Stage], bool]) -> int:
    """Where to continue: after the last stage whose state the event is in.

    Later stages can undo what earlier ones set up (closing voting after it
    opened), so a stage counts as done when any later one holds.
    """
    for index in range(len(stages) - 1, -1, -1):
        if holds(stages[index]):
            return index + 1
    return 0


def _owned_event(
    backend: Backend, store: StateStore, state: State, say: Say
) -> State | None:
    """The state to continue from, or None when its event is gone."""
    if state.event_id is None:
        # An import that did not record its event before the command stopped.
        found = backend.annotated_events(state.annotations())
        if len(found) > 1:
            raise ScenarioError(
                f"{len(found)} events carry the owner token of "
                f"{store.path(state.scenario)}: {', '.join(found)}; delete all but "
                "one in the admin portal"
            )
        if not found:
            return state
        state.event_id = found[0]
        store.save(state)
    owner = ownership(state, backend.event_annotations(state.event_id))
    if owner is Ownership.FOREIGN:
        raise ScenarioError(
            f"event {state.event_id} lacks the owner annotations recorded in "
            f"{store.path(state.scenario)}; it was not created by this scenario, so it "
            "is left alone. Remove that file to create a new event"
        )
    if owner is Ownership.MISSING:
        say(f"event {state.event_id} no longer exists; creating a new one")
        return None
    return state


def up(
    scenario: Scenario,
    backend: Backend,
    store: StateStore,
    say: Say,
    clock: Callable[[], float] = time.monotonic,
) -> UpReport:
    began = clock()
    with store.lock(scenario.name):
        state = store.load(scenario.name)
        if state is not None and state.tenant_id != backend.tenant_id:
            raise ScenarioError(
                f"{store.path(scenario.name)} belongs to tenant {state.tenant_id}, not "
                f"{backend.tenant_id}; reset it against that tenant or remove the file"
            )
        if state is not None:
            state = _owned_event(backend, store, state, say)
        if state is None:
            state = new_state(scenario.name, backend.project, backend.tenant_id)
            store.save(state)
        start = 0
        if state.event_id is not None:
            backend.point_portals(state)
            start = resume_index(
                scenario.stages, lambda stage: backend.holds(stage, scenario, state)
            )
        if start == len(scenario.stages):
            outcome = Outcome.REUSED
            final = STAGE_DESCRIPTIONS[scenario.stages[-1]]
            say(f"reusing event {state.event_id}: {final}")
        else:
            outcome = Outcome.CREATED if start == 0 else Outcome.RESUMED
            if outcome is Outcome.RESUMED:
                say(
                    f"continuing event {state.event_id} after "
                    f"{STAGE_DESCRIPTIONS[scenario.stages[start - 1]]}"
                )
        ran: dict[str, float] = {}
        for stage in scenario.stages[start:]:
            stage_began = clock()
            say(f"  {STAGE_DESCRIPTIONS[stage]} ...")
            try:
                backend.run(stage, scenario, state)
            finally:
                # Identifiers recorded before a failure let reset find the event.
                store.save(state)
            seconds = round(clock() - stage_began, 1)
            ran[stage.value] = seconds
            state.stages[stage.value] = seconds
            store.save(state)
            say(f"  {STAGE_DESCRIPTIONS[stage]} after {seconds:.1f}s")
        return UpReport(state, outcome, round(clock() - began, 1), ran)


class ResetOutcome(Enum):
    DELETED = "deleted"
    ALREADY_GONE = "already gone"
    NOTHING = "nothing to reset"


def reset(
    scenario: Scenario, backend: Backend, store: StateStore, say: Say
) -> ResetOutcome:
    with store.lock(scenario.name):
        state = store.load(scenario.name)
        if state is None:
            return ResetOutcome.NOTHING
        if state.tenant_id != backend.tenant_id:
            raise ScenarioError(
                f"{store.path(scenario.name)} belongs to tenant {state.tenant_id}, not "
                f"{backend.tenant_id}; nothing was deleted"
            )
        events = (
            [state.event_id]
            if state.event_id
            else backend.annotated_events(state.annotations())
        )
        owners = {
            event_id: ownership(state, backend.event_annotations(event_id))
            for event_id in events
        }
        foreign = [
            event for event, owner in owners.items() if owner is Ownership.FOREIGN
        ]
        if foreign:
            raise ScenarioError(
                f"event {', '.join(foreign)} lacks the owner annotations recorded in "
                f"{store.path(scenario.name)}; nothing was deleted"
            )
        outcome = ResetOutcome.ALREADY_GONE
        for event_id, owner in owners.items():
            if owner is Ownership.OWNED:
                say(f"deleting event {event_id}")
                backend.delete_event(event_id)
                outcome = ResetOutcome.DELETED
        store.remove(scenario.name)
        return outcome


class Progress(Enum):
    ABSENT = "absent"
    GONE = "gone"
    FOREIGN = "foreign"
    INCOMPLETE = "incomplete"
    READY = "ready"


@dataclass
class StatusReport:
    progress: Progress
    state: State | None = None
    # The first stage whose state does not hold yet.
    next_stage: Stage | None = None


def status(scenario: Scenario, backend: Backend, store: StateStore) -> StatusReport:
    state = store.load(scenario.name)
    if state is None:
        return StatusReport(Progress.ABSENT)
    if state.event_id is None:
        return StatusReport(Progress.INCOMPLETE, state, scenario.stages[0])
    owner = ownership(state, backend.event_annotations(state.event_id))
    if owner is Ownership.MISSING:
        return StatusReport(Progress.GONE, state)
    if owner is Ownership.FOREIGN:
        return StatusReport(Progress.FOREIGN, state)
    index = resume_index(
        scenario.stages, lambda stage: backend.holds(stage, scenario, state)
    )
    if index == len(scenario.stages):
        return StatusReport(Progress.READY, state)
    return StatusReport(Progress.INCOMPLETE, state, scenario.stages[index])
