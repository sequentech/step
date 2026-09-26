# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Push-to-result timings of GitHub Actions runs, read-only through ``gh api``.

Each push of a pull request is one sample. The push time is the creation of the
earliest workflow run for that head commit (GitHub creates runs within seconds
of the event); jobs are taken from each run's first attempt.

Only jobs concluding success or failure count as results, and workflows that do
not validate the change itself are ignored (licensing, CLA, line counts, review
bots, dependency audit, static-analysis upload, documentation preview):

  first_check        the earliest such job, formatting, lint and tooling checks
                     included
  first_actionable   the earliest such job that builds or tests product code,
                     i.e. one whose workflow and name do not mark it as a lint,
                     format, selection, result summary or CI-tooling self-check
  all_done           the last job of the push, once every run has completed
"""

from __future__ import annotations

import json
import re
import statistics
import subprocess
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from enum import Enum
from pathlib import Path
from typing import Any

from .common import SampleTimer, start_run
from .results import CacheState, SampleRole

SCENARIO = "ci"
PUSH_EVENTS = frozenset({"pull_request", "pull_request_target", "push", "merge_group"})
RESULT_CONCLUSIONS = frozenset({"success", "failure"})
STATIC_CHECK = re.compile(
    r"lint|prettif|format|\bfmt\b|clippy|tooling|\bPrebuild development tools\b|"
    r"(?:Required feedback|Selected frontend) checks\b|"
    r"Select affected feedback checks\b|\bdocs-(?:build|graphql)\b",
    re.I,
)
DEFAULT_EXCLUDED_WORKFLOWS = (
    r"reuse",
    r"\bcla\b",
    r"\blocs?\b",
    r"lines of code",
    r"copilot",
    r"coderabbit",
    r"sonar",
    r"dependency audit",
    r"documentation preview",
    r"label",
)


class StepKind(Enum):
    SETUP = "setup"
    BUILD = "build"
    TEST = "test"
    TEARDOWN = "teardown"
    OTHER = "other"


# First match wins; a step's name is all the API tells about its work.
STEP_PATTERNS: tuple[tuple[StepKind, re.Pattern[str]], ...] = (
    (StepKind.TEARDOWN, re.compile(r"^(post |complete job|stop containers)", re.I)),
    (
        StepKind.SETUP,
        re.compile(
            r"^(set ?up|initiali[sz]e|install|restore|download|log ?in|prepare)|"
            r"check ?out|cache|toolchain|dependencies|pull ",
            re.I,
        ),
    ),
    (StepKind.BUILD, re.compile(r"build|compile|wasm-pack|bundle|\bpack\b", re.I)),
    (
        StepKind.TEST,
        re.compile(
            r"test|jest|vitest|playwright|journey|coverage|e2e|lint|clippy|fmt|"
            r"format|prettier|eslint|typecheck|type check|check|audit|scan|verify|"
            r"validate|contract",
            re.I,
        ),
    ),
)


def classify_step(name: str) -> StepKind:
    for kind, pattern in STEP_PATTERNS:
        if pattern.search(name):
            return kind
    return StepKind.OTHER


def parse_time(value: str | None) -> datetime | None:
    if not value:
        return None
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def seconds_between(start: datetime | None, end: datetime | None) -> float | None:
    if start is None or end is None:
        return None
    return (end - start).total_seconds()


def completion(job: Job) -> datetime:
    return job.completed or datetime.max.replace(tzinfo=UTC)


def job_name(job: Job) -> str:
    return f"{job.workflow} / {job.name}"


def is_excluded(workflow: str, patterns: Sequence[str]) -> bool:
    return any(re.search(pattern, workflow, re.I) for pattern in patterns)


@dataclass
class Job:
    workflow: str
    name: str
    status: str
    conclusion: str | None
    created: datetime | None
    started: datetime | None
    completed: datetime | None
    steps: list[dict[str, Any]]
    runner: str | None

    def breakdown(self) -> dict[str, float]:
        totals = {kind.value: 0.0 for kind in StepKind}
        for step in self.steps:
            duration = seconds_between(
                parse_time(step.get("started_at")), parse_time(step.get("completed_at"))
            )
            if duration is not None:
                totals[classify_step(str(step.get("name", ""))).value] += duration
        return totals

    def to_dict(self, push: datetime) -> dict[str, Any]:
        return {
            "workflow": self.workflow,
            "name": self.name,
            "status": self.status,
            "conclusion": self.conclusion,
            "runner": self.runner,
            "queued_seconds": seconds_between(self.created, self.started),
            "duration_seconds": seconds_between(self.started, self.completed),
            "push_to_start_seconds": seconds_between(push, self.started),
            "push_to_completion_seconds": seconds_between(push, self.completed),
            "steps_seconds": self.breakdown(),
        }


def push_metrics(
    runs: Sequence[dict[str, Any]], jobs: Sequence[Job], excluded: Sequence[str]
) -> dict[str, Any]:
    """Timings of one push from its workflow runs and their jobs."""
    push = min(
        created
        for created in (parse_time(run.get("created_at")) for run in runs)
        if created
    )
    complete = all(run.get("status") == "completed" for run in runs) and all(
        job.status == "completed" for job in jobs
    )
    results = [
        job
        for job in jobs
        if job.status == "completed"
        and job.conclusion in RESULT_CONCLUSIONS
        and job.completed is not None
        and not is_excluded(job.workflow, excluded)
    ]
    actionable = [
        job
        for job in results
        if not STATIC_CHECK.search(f"{job.workflow} / {job.name}")
    ]
    first_check = min(results, key=completion) if results else None
    first = min(actionable, key=completion) if actionable else None
    finished = [job.completed for job in jobs if job.completed is not None]
    started = [job.started for job in jobs if job.started is not None]
    queues = [
        value
        for value in (seconds_between(job.created, job.started) for job in jobs)
        if value is not None
    ]
    phases: dict[str, float] = {}
    if first_check is not None:
        phases["first_check"] = (completion(first_check) - push).total_seconds()
    if first is not None:
        phases["first_actionable"] = (completion(first) - push).total_seconds()
    if started:
        phases["first_job_started"] = (min(started) - push).total_seconds()
    if complete and finished:
        phases["all_done"] = (max(finished) - push).total_seconds()
    if queues:
        phases["median_queue"] = statistics.median(queues)
        phases["max_queue"] = max(queues)
    return {
        "push": push.isoformat(),
        "complete": complete,
        "first_check_job": None if first_check is None else job_name(first_check),
        "first_actionable_job": None if first is None else job_name(first),
        "phases": phases,
        "jobs": [job.to_dict(push) for job in jobs],
    }


def gh_api(path: str) -> Any:
    completed = subprocess.run(
        ["gh", "api", "--method", "GET", path],
        capture_output=True,
        text=True,
        check=False,
        timeout=120,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"gh api {path} failed: {completed.stderr.strip()}")
    return json.loads(completed.stdout)


def gh_pages(path: str, key: str) -> Iterable[dict[str, Any]]:
    page = 1
    while True:
        separator = "&" if "?" in path else "?"
        document = gh_api(f"{path}{separator}per_page=100&page={page}")
        items = document.get(key, [])
        yield from items
        if len(items) < 100:
            return
        page += 1


def pull_request(repository: str, number: int) -> dict[str, Any]:
    return dict(gh_api(f"repos/{repository}/pulls/{number}"))


def pushes(repository: str, branch: str, limit: int) -> list[list[dict[str, Any]]]:
    """The branch's workflow runs grouped by head commit, newest push first."""
    groups: dict[str, list[dict[str, Any]]] = {}
    for run in gh_pages(
        f"repos/{repository}/actions/runs?branch={branch}", "workflow_runs"
    ):
        if run.get("event") in PUSH_EVENTS:
            groups.setdefault(run["head_sha"], []).append(run)
    ordered = sorted(
        groups.values(),
        key=lambda runs: min(run["created_at"] for run in runs),
        reverse=True,
    )
    return ordered[:limit]


def run_jobs(repository: str, run: dict[str, Any]) -> list[Job]:
    """Jobs of the first attempt: a later re-run does not time the push."""
    jobs = []
    path = f"repos/{repository}/actions/runs/{run['id']}/attempts/1/jobs"
    for job in gh_pages(path, "jobs"):
        jobs.append(
            Job(
                workflow=str(run.get("name")),
                name=str(job.get("name")),
                status=str(job.get("status")),
                conclusion=job.get("conclusion"),
                created=parse_time(job.get("created_at")),
                started=parse_time(job.get("started_at")),
                completed=parse_time(job.get("completed_at")),
                steps=list(job.get("steps") or []),
                runner=job.get("runner_name"),
            )
        )
    return jobs


def run_ci(
    *,
    repository: str,
    numbers: Sequence[int],
    label: str,
    pushes_per_pr: int,
    excluded: Sequence[str],
    output_dir: Path,
) -> Path:
    run = start_run(
        scenario=SCENARIO,
        target="pull-requests",
        label=label,
        cache=CacheState.UNCONTROLLED,
        cache_detail="hosted GitHub Actions as found; runner caches not controlled",
        checkout=None,
        output_dir=output_dir,
        services=[],
        commands=[f"gh api repos/{repository}/actions/runs?branch=<head branch>"],
        parameters={
            "repository": repository,
            "pull_requests": list(numbers),
            "pushes_per_pr": pushes_per_pr,
            "excluded_workflows": list(excluded),
            "push_events": sorted(PUSH_EVENTS),
        },
        extra_tools={"gh": ("gh", "--version")},
    )
    index = 0
    for number in numbers:
        request = pull_request(repository, number)
        branch = request["head"]["ref"]
        for runs in pushes(repository, branch, pushes_per_pr):
            index += 1
            timer = SampleTimer(index, SampleRole.MEASURED)
            jobs = [
                job
                for workflow_run in runs
                for job in run_jobs(repository, workflow_run)
            ]
            metrics = push_metrics(runs, jobs, excluded)
            first = metrics["phases"].get("first_actionable")
            detail = {
                "pull_request": number,
                "base": request["base"]["ref"],
                "branch": branch,
                "head_sha": runs[0]["head_sha"],
                "runs": [
                    {
                        "id": workflow_run["id"],
                        "workflow": workflow_run.get("name"),
                        "event": workflow_run.get("event"),
                        "status": workflow_run.get("status"),
                        "conclusion": workflow_run.get("conclusion"),
                        "attempt": workflow_run.get("run_attempt"),
                    }
                    for workflow_run in runs
                ],
                **metrics,
            }
            run.add(
                timer.finish(
                    ok=first is not None,
                    seconds=first,
                    phases=metrics["phases"],
                    detail=detail,
                    error=None if first is not None else "no actionable job completed",
                )
            )
    return run.finish()
