#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Run Step's public/private contracts against an explicitly owned local testkit.

The caller starts/stops Vitrine's integration_server. Its owner-only capability
file contains test credentials; this script never prints or copies them. Only
the synthetic public policy/ballot artifacts are exported for the browser test.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
from urllib.parse import urlparse


def local_url(value):
    assert urlparse(value).hostname in ("127.0.0.1", "localhost", "::1"), "use an owned local fixture"
    return value


def deployment(root, facts):
    template = (root / ".devcontainer/vitrine/deployment.toml").read_text()
    boundary = "# The two witnesses"
    assert boundary in template, "review the development template before updating this helper"
    text = template.split(boundary, 1)[0]
    text = text.replace("period = 10", "period = 1").replace("max_gap = 60", "max_gap = 10")
    quote = json.dumps
    for witness in facts["witnesses"]:
        local_url(witness["url"])
        text += "\n[[witnesses]]\n" + "".join(f"{key} = {quote(witness[key])}\n" for key in ("name", "vkey", "url", "party"))
    text += '\n[[groups]]\nname = "witnesses"\nthreshold = "all"\nmembers = ' + quote([w["name"] for w in facts["witnesses"]]) + "\n"
    text += '\n[[mirrors]]\nurl = ' + quote(facts["public_url"].rstrip("/") + "/{path}/") + '\nparty = "test-observers"\n'
    for hub in facts["hubs"]:
        local_url(hub["url"])
        text += "\n[[hubs]]\n" + "".join(f"{key} = {quote(hub[key])}\n" for key in ("name", "vkey", "url", "operator"))
        if hub["bootstrap"]:
            text += 'role = "bootstrap"\n'
        text += 'quorum = "all-witnesses"\nwitnesses = [\n'
        for witness in hub["witnesses"]:
            local_url(witness["url"])
            text += "  {" + ", ".join(f"{key} = {quote(witness[key])}" for key in ("name", "vkey", "url", "party")) + "},\n"
        text += ']\ngroups = [{name = "all-witnesses", threshold = "all", members = ' + quote([w["name"] for w in hub["witnesses"]]) + "}]\n"
    for monitor in facts["monitors"]:
        local_url(monitor["url"])
        text += "\n[[monitors]]\n" + "".join(f"{key} = {quote(monitor[key])}\n" for key in ("name", "vkey", "url", "party"))
    return text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capabilities", type=Path)
    parser.add_argument("test_binary", type=Path)
    parser.add_argument("output", type=Path, help="new output directory; existing output is refused")
    args = parser.parse_args()
    assert args.capabilities.stat().st_mode & 0o077 == 0, "capabilities must be owner-only"
    facts = json.loads(args.capabilities.read_text())
    assert facts["status"] == "development simulation", "never run against a production deployment"
    local_url(facts["database_url"])
    local_url(facts["application_database_url"])
    local_url(facts["public_url"])
    local_url(facts["internal_url"])
    for admin in facts["witness_admins"]:
        local_url(admin["url"])
    root = Path(__file__).resolve().parent.parent
    output = args.output.resolve()
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    config = output / "deployment.toml"
    with config.open("x") as file:
        file.write(deployment(root, facts))
    config.chmod(0o600)
    env = os.environ.copy()
    env.update({
        "WINDMILL_VITRINE_TEST_INTERNAL_URL": facts["internal_url"],
        "WINDMILL_VITRINE_TEST_ADMIN_TOKEN_FILE": facts["admin_token_file"],
        "WINDMILL_VITRINE_TEST_DATABASE_URL": facts["application_database_url"],
        "WINDMILL_VITRINE_TEST_WITNESS_ADMINS": ",".join(w["url"] + "|" + w["token_file"] for w in facts["witness_admins"]),
        "WINDMILL_VITRINE_TEST_CONFIG_FILE": str(config),
        "ELECTORAL_LOG_PG_VITRINE_QUEUE_ROLE": facts["queue_role"],
        "WINDMILL_VITRINE_TEST_ARTIFACT_DIR": str(output / "public"),
    })
    scenarios = {
        "public": "a_public_event_logs_its_ballots_and_statements_on_the_sidecar",
        "private": "a_private_event_provisions_both_blinded_keys_and_exposes_authenticated_policy_history",
    }
    for label, test in scenarios.items():
        with (output / f"{label}.txt").open("x") as log:
            result = subprocess.run([str(args.test_binary.resolve()), test, "--exact", "--ignored", "--nocapture"], cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
        print(f"{label} native contract exit code: {result.returncode}", flush=True)
        if result.returncode:
            raise SystemExit(result.returncode)
    print(f"Public browser inputs: {output / 'public'}", flush=True)


if __name__ == "__main__":
    main()
