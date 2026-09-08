# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Create an isolated synthetic election, ceremony and patterned census via step-cli.

Requires an authenticated tenant administrator and configured automatic trustees.
The input is an exported event template; server-assigned IDs are read back before
building census eligibility. Administrator credentials never enter worker files.
"""
import argparse
import copy
import json
import os
from pathlib import Path
import re
import subprocess
import time
import uuid
import zipfile

from runner import census, import_census, read, save, validate


def fixture(template: dict, config: dict) -> dict:
    """Keep one eligible election/contest and configure exact username login."""
    event = copy.deepcopy(template)
    area = event["areas"][0]
    link = next(
        item for item in event["area_contests"] if item["area_id"] == area["id"]
    )
    contest = next(
        item for item in event["contests"] if item["id"] == link["contest_id"]
    )
    election = next(
        item for item in event["elections"] if item["id"] == contest["election_id"]
    )
    event.update(
        areas=[area], area_contests=[link], contests=[contest], elections=[election]
    )
    event["candidates"] = [
        item for item in event["candidates"] if item["contest_id"] == contest["id"]
    ]
    for item in [event["election_event"], election, contest, *event["candidates"]]:
        item["annotations"] = {}
        for key in item:
            if key.endswith("_document_id"):
                item[key] = None
    contest.update(min_votes=1, max_votes=1)
    alias = "Synthetic load " + uuid.uuid4().hex[:10]
    event["election_event"]["alias"] = alias
    event["election_event"]["presentation"].setdefault("i18n", {}).setdefault("en", {})[
        "alias"
    ] = alias
    origin = config["portal_url"].rstrip("/")
    event["election_event"]["presentation"]["logo_url"] = origin + "/favicon.svg"
    realm = event["keycloak_event_realm"]
    realm["passwordPolicy"] = (
        f"hashAlgorithm(pbkdf2-sha256) and hashIterations({config.get('hash_iterations', 27500)})"
    )
    # All synthetic accounts share a password: unique username lookup is essential.
    for authentication in realm.get("authenticatorConfig", []):
        if "matchAttributes" in authentication.get("config", {}):
            authentication["config"]["matchAttributes"] = "username"
    for messages in realm.get("localizationTexts", {}).values():
        messages["loginCustomCss"] = ""
    for client in realm.get("clients", []):
        if client["clientId"] == "voting-portal":
            client.update(
                rootUrl=origin,
                baseUrl=origin,
                redirectUris=[origin + "/*"],
                webOrigins=[origin],
            )
    return event


def provision(
    config: dict,
    template: Path,
    output: Path,
    cli: Path,
    local: bool,
    threshold: int,
    publication_preparer: Path | None = None,
) -> None:
    """Execute setup with private logs and poll the automatic ceremony to completion."""
    output.mkdir(parents=True, exist_ok=False)

    def step(*arguments: str) -> str:
        """Catch CLI application errors even when its process exits with status zero."""
        result = subprocess.run(
            [str(cli.resolve()), "step", *arguments],
            capture_output=True,
            text=True,
            check=True,
        )
        text = re.sub(r"\x1b\[[0-9;]*m", "", result.stdout + result.stderr)
        with (output / "setup.log").open("a") as log:
            log.write(text)
        if "Error!" in text or "error:" in text.lower():
            raise RuntimeError("step-cli failed; inspect private setup.log")
        return text

    def identifier(text: str) -> str:
        """Require the command's explicit ID field, not an arbitrary logged UUID."""
        matches = re.findall(r"ID:? +([A-Za-z0-9._-]+)", text)
        if not matches:
            raise RuntimeError("step-cli did not return an ID")
        return matches[-1]

    prepared = fixture(read(template), config)
    save(output / "fixture.json", prepared)
    event = identifier(
        step(
            "import-election",
            "--file-path",
            str(output / "fixture.json"),
            *(["--is-local"] if local else []),
        )
    )
    save(output / "setup-state.json", dict(election_event_id=event))
    exported = output / "export"
    exported.mkdir()
    step(
        "export-election-event",
        "--election-event-id",
        event,
        "--output-dir",
        str(exported),
    )
    with zipfile.ZipFile(exported / "election_event_export.zip") as archive:
        members = [name for name in archive.namelist() if name.endswith(".json")]
        if len(members) != 1:
            raise ValueError("Expected one exported event configuration")
        imported = json.loads(archive.read(members[0]))
    if imported["election_event"]["id"] != event:
        raise ValueError("Export scope mismatch")
    tenant = imported["election_event"]["tenant_id"]
    if tenant != config["tenant_id"]:
        raise ValueError("CLI administrator is configured for a different tenant")
    config.update(
        election_event_id=event,
        election_id=imported["elections"][0]["id"],
        realm=f"tenant-{tenant}-event-{event}",
        area_name=imported["areas"][0]["name"],
        login_url=f"{config['portal_url'].rstrip('/')}/tenant/{tenant}/event/{event}/login",
    )
    validate(config)
    save(output / "config.json", config)
    census(config, output / "census")
    import_census(config, output / "census", cli, local)
    ceremony = identifier(
        step(
            "start-key-ceremony",
            "--election-event-id",
            event,
            "--threshold",
            str(threshold),
            "--automatic",
        )
    )
    save(
        output / "setup-state.json",
        dict(election_event_id=event, key_ceremony_id=ceremony),
    )
    deadline = time.monotonic() + config.get("ceremony_timeout_seconds", 600)
    while time.monotonic() < deadline:
        step("refresh-token")
        status = step(
            "get-key-ceremony-status",
            "--election-event-id",
            event,
            "--key-ceremony-id",
            ceremony,
        )
        if re.search(r"status:\s+SUCCESS", status):
            break
        if "CANCELLED" in status:
            raise RuntimeError("Key ceremony cancelled")
        time.sleep(config.get("poll_interval_seconds", 5))
    else:
        raise RuntimeError("Key ceremony timed out; inspect setup-state.json")
    publication = identifier(step("publish", "--election-event-id", event))
    if publication_preparer:
        # Some deployments run publication-file preparation as a separate native step.
        # This helper takes the same event lock as the application publisher.
        with (output / "publication.log").open("w") as log:
            subprocess.run(
                [str(publication_preparer.resolve()), tenant, event, publication],
                stdout=log,
                stderr=log,
                check=True,
            )
    step(
        "update-event-voting-status",
        "--election-event-id",
        event,
        "--voting-status",
        "OPEN",
        "--voting-channel",
        "ONLINE",
    )
    save(
        output / "setup-state.json",
        dict(election_event_id=event, key_ceremony_id=ceremony, ready=True),
    )


def main() -> None:
    """Accept explicit paths so identical setup commands work in any deployment."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--is-local", action="store_true")
    parser.add_argument("--threshold", type=int, default=2)
    parser.add_argument("--publication-preparer", type=Path)
    args = parser.parse_args()
    os.umask(0o077)
    provision(
        read(args.config),
        args.template,
        args.output.resolve(),
        args.cli,
        args.is_local,
        args.threshold,
        args.publication_preparer,
    )


if __name__ == "__main__":
    main()
