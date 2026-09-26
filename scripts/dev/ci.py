# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""GitHub job matrices and result checking for the shared affected model."""

from __future__ import annotations

import argparse
import enum
import json
import os
from pathlib import Path

from scripts.dev.affected.changes import ChangeSet, Scope, collect
from scripts.dev.affected.model import Model, Selection, load_model
from scripts.dev.affected.report import json_report, text_report

ROOT = Path(__file__).resolve().parents[2]
RUST = (
    "electoral-log",
    "harvest",
    "strand",
    "immu-board",
    "immudb-rs",
    "sequent-core",
    "step-cli",
    "velvet",
    "wrap-map-err",
)
STORIES = (
    "ui-essentials",
    "voting-portal",
    "admin-portal",
    "results-portal",
    "ballot-verifier",
)
PORTALS = ("voting-portal", "results-portal", "ballot-verifier", "admin-portal")
PYTHON = (
    "python:scripts-dev",
    "python:scripts-coverage",
    "python:scripts-e2e",
    "python:scripts-database",
)


class Validation(enum.Enum):
    AFFECTED = "affected"
    FULL = "full"


def selection_for(
    root: Path, base: str, validation: Validation
) -> tuple[Model, Selection]:
    model = load_model(root)
    if validation is Validation.FULL:
        changes = ChangeSet(
            root, Scope.COMMITTED, [], problems=["full validation event"]
        )
    else:
        changes = collect(root, base, model.config.default_base, Scope.COMMITTED)
    return model, model.select(changes)


def matrices(selection: Selection) -> dict[str, object]:
    selected = {decision.check.id: decision.check for decision in selection.selected()}
    rust = [
        {
            "service": package,
            "extra": "--features keycloak,default_features"
            if package == "sequent-core"
            else "",
        }
        for package in RUST
        if f"cargo-test:{package}" in selected
    ]
    node = [
        {"package": check.cwd.removeprefix("packages/"), "command": check.command}
        for check in selected.values()
        if check.id.startswith("jest:")
        or check.id
        in ("vitest:workbench", "node-test:keycloak-theme-tests", "types:ui-e2e")
    ]
    python = [
        {"check": check.id, "command": check.command}
        for check in selected.values()
        if check.id in PYTHON
    ]
    docs = [
        {"check": check.id, "cwd": check.cwd, "command": check.command}
        for check in selected.values()
        if check.id in ("docs-build", "docs-graphql")
    ]
    stories = [package for package in STORIES if f"stories:{package}" in selected]
    portals = [package for package in PORTALS if f"journeys:{package}" in selected]
    builds = sorted(
        set(portals) | ({"voting-portal"} if "ballot-verifier" in portals else set())
    )
    journeys = [
        {
            "package": package,
            "shard": shard,
            "shards": 4 if package == "admin-portal" else 1,
        }
        for package in portals
        for shard in (range(1, 5) if package == "admin-portal" else (1,))
    ]
    ui_jobs = {
        "stories": bool(stories),
        "shared-ui": bool(builds),
        "build-portals": bool(builds),
        "portal-journeys": bool(journeys),
        "fixtures": bool(journeys)
        or any(
            check in selected
            for check in ("contracts:ui-test-kit", "types:ui-test-kit")
        ),
        "workbench": any(
            check in selected for check in ("smoke:workbench", "types:workbench")
        ),
    }
    jobs = {
        "run-tests": bool(rust),
        "run-windmill-tests": "cargo-test:windmill" in selected,
        "run-frontend-tests": bool(node),
        "tooling": bool(python),
        "docs": bool(docs),
        "wasm-freshness": "wasm-freshness" in selected,
        "frontend-ui": any(ui_jobs.values()),
    }
    return {
        "rust": rust,
        "node": node,
        "python": python,
        "docs": docs,
        "stories": stories,
        "builds": builds,
        "journeys": journeys,
        "jobs": jobs,
        "ui_jobs": ui_jobs,
    }


def check_results(
    expected: dict[str, bool], results: dict[str, dict[str, object]]
) -> list[str]:
    failures = []
    for job, selected in expected.items():
        result = results.get(job, {}).get("result", "missing")
        required = "success" if selected else "skipped"
        if result != required:
            failures.append(f"{job}: expected {required}, got {result}")
    return failures


def write_plan(root: Path, base: str, validation: Validation, output: Path) -> None:
    model, selection = selection_for(root, base, validation)
    plan = matrices(selection)
    output.write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    output.with_suffix(".affected.json").write_text(
        json.dumps(json_report(model, selection), indent=2) + "\n", encoding="utf-8"
    )
    encoded = json.dumps(plan, separators=(",", ":"))
    if destination := os.environ.get("GITHUB_OUTPUT"):
        with Path(destination).open("a", encoding="utf-8") as stream:
            stream.write(f"selection={encoded}\n")
    summary = (
        "## Affected feedback checks\n\n"
        "Tests and Frontend UI tests use the local affected model. Coverage, "
        "backend integration and other existing workflows retain their own gates.\n\n"
        "<details><summary>Selected and skipped checks, packages and reasons"
        "</summary>\n\n"
        f"```text\n{text_report(selection)}\n```\n\n</details>\n"
    )
    if destination := os.environ.get("GITHUB_STEP_SUMMARY"):
        with Path(destination).open("a", encoding="utf-8") as stream:
            stream.write(summary)
    print(encoded)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    plan = commands.add_parser("plan")
    plan.add_argument("--base", default="origin/ovcs")
    plan.add_argument(
        "--validation", choices=[item.value for item in Validation], default="affected"
    )
    plan.add_argument("--output", type=Path, default=Path("ci-plan.json"))
    verify = commands.add_parser("verify")
    verify.add_argument("--scope", choices=("jobs", "ui_jobs"), default="jobs")
    args = parser.parse_args()
    if args.command == "plan":
        write_plan(ROOT, args.base, Validation(args.validation), args.output)
        return 0
    plan_document = json.loads(os.environ["CI_SELECTION"])
    results = json.loads(os.environ["CI_RESULTS"])
    failures = check_results(plan_document[args.scope], results)
    if args.scope == "jobs" and results.get("plan", {}).get("result") != "success":
        failures.append("plan did not succeed")
    for failure in failures:
        print(f"::error::{failure}")
    if not failures:
        print(
            "All selected feedback checks succeeded; "
            "other feedback jobs were skipped by selection."
        )
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
