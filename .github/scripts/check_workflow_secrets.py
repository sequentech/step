#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Check that release credentials are only read by jobs of the release
environment, and never by workflows that run for pull requests or comments."""

import json
import re
import sys
from pathlib import Path

import yaml

RELEASE_ENVIRONMENT = "release"
RELEASE_SECRETS = (
    "PAT",
    "DOCKERHUB_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_ACCESS_KEY_ID_GLOBALDOT",
    "AWS_SECRET_ACCESS_KEY_GLOBALDOT",
)
CONTRIBUTION_TRIGGERS = ("pull_request", "pull_request_target", "issue_comment")
LOCAL_WORKFLOW_PREFIX = "./.github/workflows/"
RELEASE_SECRET_PATTERN = re.compile(
    r"secrets\s*\.\s*(?:" + "|".join(RELEASE_SECRETS) + r")\b"
    r"|secrets\s*\["
    r"|toJSON\(\s*secrets\s*\)"
)


def triggers(workflow):
    # PyYAML reads the bare `on` key as the boolean True.
    on = workflow.get("on", workflow.get(True))
    if isinstance(on, str):
        return {on}
    if isinstance(on, (list, dict)):
        return set(on)
    return set()


def environment_name(job):
    environment = job.get("environment")
    if isinstance(environment, dict):
        return environment.get("name")
    return environment


def check_workflow(name, workflow):
    errors = []
    contribution_triggers = sorted(triggers(workflow) & set(CONTRIBUTION_TRIGGERS))
    for job_id, job in (workflow.get("jobs") or {}).items():
        where = f"{name}: job '{job_id}'"
        uses = job.get("uses")
        if job.get("secrets") == "inherit":
            errors.append(f"{where} passes every secret with 'secrets: inherit'")
        reads_release_secrets = bool(RELEASE_SECRET_PATTERN.search(json.dumps(job)))
        if not reads_release_secrets:
            continue
        if contribution_triggers:
            errors.append(
                f"{where} reads release credentials in a workflow triggered by "
                + ", ".join(contribution_triggers)
            )
        # A local reusable workflow is checked in its own file.
        if uses and uses.startswith(LOCAL_WORKFLOW_PREFIX):
            continue
        if environment_name(job) != RELEASE_ENVIRONMENT:
            errors.append(
                f"{where} reads release credentials outside the "
                f"'{RELEASE_ENVIRONMENT}' environment"
            )
    return errors


def check_directory(directory):
    errors = []
    for path in sorted(Path(directory).glob("*.y*ml")):
        with path.open(encoding="utf-8") as stream:
            workflow = yaml.safe_load(stream) or {}
        errors.extend(check_workflow(path.name, workflow))
    return errors


def main(argv):
    if len(argv) != 2:
        print(f"usage: {argv[0]} WORKFLOWS_DIRECTORY", file=sys.stderr)
        return 2
    errors = check_directory(argv[1])
    for error in errors:
        print(error, file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
