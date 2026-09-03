#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Stage 1 of the load tests: provisions an election event for a tenant,
bulk-creates DTMF-safe voters, runs the keys ceremony, publishes, and opens
the requested voting channel (TELEPHONE or ONLINE — see the 'setup:' section
of telephone-load-test-inputs/layers.yaml). Writes a summary.json + voters
CSV that Stage 2 consumes — run_telephone_load_test.py (driving `ivr-cli`
calls) for TELEPHONE, run_online_load_test.py (driving Playwright browsers)
for ONLINE. See
docs/docusaurus/docs/07-developers/12-ivr/telephone-load-testing-design.md
and docs/docusaurus/docs/07-developers/02-cli/02-tutorials/load-testing/online-load-testing-design.md
for the full designs, and the guide next to each for a walkthrough.

Takes no command-line arguments — every setting lives in
telephone-load-test-inputs/layers.yaml.

Requires the `step-cli` binary on PATH (cd packages/step-cli && cargo build
--release -p step-cli).
"""

from __future__ import annotations

import json
import random
import string
from pathlib import Path

import load_test_common as common


def rewrite_election_event_alias(election_event_json: Path, out_path: Path) -> tuple[str, str]:
    """Appends a random 5-char suffix to the election event's alias (every
    language, so it stays visible regardless of the admin portal's UI
    language) — makes this run's election event easy to pick out in the
    admin portal's list, especially when several load-test runs exist at
    once. The admin portal's election event list renders alias, falling back
    to name only where no alias is set — so alias, not name, is the field
    that's actually shown there. Where a language has no alias yet, seed it
    from that language's name so the suffix is visible in every language, not
    just the ones that already had one.

    Returns (election_event_alias, suffix) for logging."""
    with election_event_json.open() as f:
        data = json.load(f)

    suffix = "".join(random.choices(string.ascii_uppercase + string.digits, k=5))
    i18n = data["election_event"]["presentation"]["i18n"]
    for value in i18n.values():
        if value.get("name"):
            base = value.get("alias") or value["name"]
            value["alias"] = f"{base} - {suffix}"

    common.write_json(out_path, data)

    alias = None
    if i18n.get("en", {}).get("alias"):
        alias = i18n["en"]["alias"]
    else:
        for value in i18n.values():
            if value.get("alias"):
                alias = value["alias"]
                break
    return alias or "Unknown", suffix


def area_restricted_election_event(election_event_json: Path, voter_area_name: str | None) -> tuple[dict, str]:
    """generate-voters round-robins voters across every area in the election
    event file it's given, and different areas can have different contest
    counts — which would mean different voters need different DTMF scripts.
    Returns a copy of the election event trimmed to a single area (and only
    that area's area_contests) so every generated voter lands in the same
    area, keeping one DTMF template valid for every simulated call. This only
    affects voter generation, not the election itself — the untrimmed file is
    what gets imported. Returns (trimmed_data, resolved_area_name)."""
    with election_event_json.open() as f:
        data = json.load(f)

    if not voter_area_name:
        areas = data.get("areas") or []
        if not areas:
            common.die(f"{election_event_json} has no areas")
        voter_area_name = areas[0]["name"]

    matching_areas = [a for a in data["areas"] if a.get("name") == voter_area_name]
    if not matching_areas:
        common.die(f"setup.voter_area_name '{voter_area_name}' does not match any area in {election_event_json}")
    area_id = matching_areas[0]["id"]
    data["areas"] = matching_areas
    data["area_contests"] = [c for c in data.get("area_contests", []) if c.get("area_id") == area_id]
    return data, voter_area_name  # type: ignore[return-value]


def main() -> None:
    config = common.load_config()
    cfg = common.section(config, "setup")

    election_event_json = common.resolve_path(common.req_str(cfg, "election_event_json"))
    if not election_event_json.is_file():
        common.die(f"no such file: {election_event_json}")

    voting_channel = str(cfg.get("voting_channel") or "TELEPHONE")
    if voting_channel not in ("TELEPHONE", "ONLINE"):
        common.die("setup.voting_channel must be TELEPHONE or ONLINE")
    voting_portal_url = str(cfg.get("voting_portal_url") or "http://127.0.0.1:3000").rstrip("/")

    tenant_id = common.req_str(cfg, "tenant_id", env="SUPER_ADMIN_TENANT_ID")
    num_voters = int(cfg.get("num_voters") or 20)
    voter_pin_digits = int(cfg.get("voter_pin_digits") or 6)
    if not (1 <= voter_pin_digits <= 8):
        common.die("setup.voter_pin_digits must be between 1 and 8 (DTMF voter auth limit)")
    voter_username_start = int(cfg.get("voter_username_start") or 100)
    if voter_username_start < 0:
        common.die("setup.voter_username_start must be >= 0")
    voter_area_name = cfg.get("voter_area_name") or None
    threshold = int(cfg.get("threshold") or 2)

    endpoint_url = common.req_str(cfg, "endpoint_url", env="HASURA_ENDPOINT")
    keycloak_url = common.req_str(cfg, "keycloak_url", env="KEYCLOAK_URL")
    # Tenant-realm login: a user holding the admin-user role inside this
    # tenant's own realm — the same identity that logs into the Admin Portal
    # to manage this tenant. Used for every election-management step below
    # (import, publish, open voting).
    admin_portal_user = common.req_str(cfg, "admin_portal_user", env="ADMIN_PORTAL_TEST_USERNAME")
    admin_portal_password = common.req_str(cfg, "admin_portal_password", env="ADMIN_PORTAL_TEST_PASSWORD")
    # NOT $KEYCLOAK_CLI_CLIENT_ID: that client (admin-portal in this
    # devcontainer) gets Keycloak's default "silver" acr on direct-grant
    # login, and publish / update-event-voting-status require "gold"
    # (sequent-core's has_gold_permission checks claims.acr == "gold").
    # api-key-client is the one client configured with
    # default.acr.values: gold, matching what every CLI tutorial in
    # docs/docusaurus hardcodes for this same reason.
    keycloak_client_id = common.req_str(cfg, "keycloak_client_id")
    # NOT $KEYCLOAK_CLI_CLIENT_SECRET: that devcontainer env var is
    # admin-portal's secret, a different client. Find this one in the
    # tenant's own realm: Keycloak admin console -> Clients -> api-key-client
    # -> Credentials tab.
    keycloak_client_secret = common.req_str(cfg, "keycloak_client_secret", env="API_KEY_CLIENT_SECRET")

    trustee1_user = str(cfg.get("trustee1_user") or "trustee1")
    trustee1_password = common.req_str(cfg, "trustee1_password", env="TRUSTEE1_PASSWORD")
    trustee2_user = str(cfg.get("trustee2_user") or "trustee2")
    trustee2_password = common.req_str(cfg, "trustee2_password", env="TRUSTEE2_PASSWORD")

    out_dir = common.resolve_path(common.req_str(cfg, "out_dir"))

    step_cli_bin = common.find_step_cli()

    def configure_as(user: str, password: str) -> None:
        common.run_step(
            step_cli_bin,
            "config",
            "--tenant-id", tenant_id,
            "--endpoint-url", endpoint_url,
            "--keycloak-url", keycloak_url,
            "--keycloak-user", user,
            "--keycloak-password", password,
            "--keycloak-client-id", keycloak_client_id,
            "--keycloak-client-secret", keycloak_client_secret,
        )

    out_dir.mkdir(parents=True, exist_ok=True)
    common.log(f"Writing outputs to {out_dir}")

    common.log(f"[1/7] Authenticating as admin-portal user ({admin_portal_user})")
    configure_as(admin_portal_user, admin_portal_password)

    common.log(f"[2/7] Importing election event from {election_event_json}")
    election_event_to_import = out_dir / "election-event-to-import.json"
    election_event_alias, _suffix = rewrite_election_event_alias(election_event_json, election_event_to_import)
    out = common.run_step(step_cli_bin, "import-election", "--file-path", str(election_event_to_import), "--is-local")
    election_event_id = common.extract_id(out)
    common.log(f"    election_event_id={election_event_id}")
    common.log(f"    election_event_alias={election_event_alias}")

    common.log(f"[3/7] Generating {num_voters} voters with numeric, {voter_pin_digits}-digit DTMF-safe credentials")
    # dateOfBirth is required: this realm's IVR auth flow (checked live via
    # its {realm}/ivr-config endpoint, not assumed from a generic default)
    # resolves voters by dateOfBirth + PIN rather than by username + PIN, so
    # a voter with no dateOfBirth attribute can never authenticate over a
    # call. generate-voters writes it in the realm's expected YYYY-MM-DD form
    # already.
    trimmed_election_event, voter_area_name = area_restricted_election_event(election_event_json, voter_area_name)
    common.write_json(out_dir / "election-event.json", trimmed_election_event)
    common.log(f"    voter_area={voter_area_name}")
    external_config = {
        "election_event_json_file": "election-event.json",
        "realm_name": f"tenant-{tenant_id}-event-{election_event_id}",
        "tenant_id": tenant_id,
        "election_event_id": election_event_id,
        "area_id": "",
        "election_id": "",
        "generate_voters": {
            "csv_file_name": "voters",
            "fields": ["username", "area_name", "password", "email", "email_verified", "dateOfBirth"],
            "excluded_columns": [],
            "email_prefix": "telephone-load-test",
            "domain": "example.invalid",
            "sequence_email_number": True,
            "sequence_start_number": 0,
            "username_start_number": voter_username_start,
            "voter_password": "",
            "voter_password_policy": {"type": "random-numeric", "digits": voter_pin_digits},
            "password_salt": "",
            "hashed_password": "",
            "overseas_reference": "",
            "min_age": 18,
            "max_age": 90,
            "authorized_elections_count": 0,
            "email_verified": True,
        },
        "duplicate_votes": {"row_id_to_clone": ""},
        "generate_applications": {"applicant_data": {}, "annotations": {}},
    }
    common.write_json(out_dir / "external_config.json", external_config)
    common.run_step(step_cli_bin, "generate-voters", "--working-directory", str(out_dir), "--num-users", str(num_voters))
    voters_csv = out_dir / f"voters_{num_voters}.csv"
    if not voters_csv.is_file():
        common.die(f"expected voters CSV at {voters_csv}, not found")
    common.log(f"    voters_csv={voters_csv}")

    common.log("[4/7] Bulk-importing voters into the election event")
    common.run_step(step_cli_bin, "import-voters", "--election-event-id", election_event_id, "--file-path", str(voters_csv), "--is-local")

    common.log(f"[5/7] Starting the keys ceremony (threshold={threshold})")
    out = common.run_step(step_cli_bin, "start-key-ceremony", "--election-event-id", election_event_id, "--threshold", str(threshold))
    key_ceremony_id = common.extract_id(out)
    common.log(f"    key_ceremony_id={key_ceremony_id}")

    common.log(f"[6/7] Completing the keys ceremony as {trustee1_user}, then {trustee2_user}")
    configure_as(trustee1_user, trustee1_password)
    common.retry_step(step_cli_bin, 30, 5, "complete-key-ceremony", "--election-event-id", election_event_id, "--key-ceremony-id", key_ceremony_id)
    configure_as(trustee2_user, trustee2_password)
    common.retry_step(step_cli_bin, 30, 5, "complete-key-ceremony", "--election-event-id", election_event_id, "--key-ceremony-id", key_ceremony_id)

    common.log(f"[7/7] Publishing and opening the {voting_channel} voting channel")
    configure_as(admin_portal_user, admin_portal_password)
    common.run_step(step_cli_bin, "publish", "--election-event-id", election_event_id)
    common.run_step(
        step_cli_bin, "update-event-voting-status",
        "--election-event-id", election_event_id,
        "--voting-status", "OPEN",
        "--voting-channel", voting_channel,
    )

    realm_name = f"tenant-{tenant_id}-event-{election_event_id}"
    login_url = f"{voting_portal_url}/tenant/{tenant_id}/event/{election_event_id}/login"
    summary = {
        "tenant_id": tenant_id,
        "election_event_id": election_event_id,
        "election_event_alias": election_event_alias,
        "keycloak_realm": realm_name,
        "keycloak_url": keycloak_url,
        "hasura_url": endpoint_url,
        "voting_channel": voting_channel,
        "voting_portal_url": voting_portal_url,
        "login_url": login_url,
        "voters_csv": str(voters_csv),
        "num_voters": num_voters,
        "voter_area": voter_area_name,
    }
    summary_path = out_dir / "summary.json"
    common.write_json(summary_path, summary)

    common.log(f'Done. Election event "{election_event_alias}" ({election_event_id}) is open for {voting_channel} voting.')
    common.log(f"Summary: {summary_path}")
    common.log(f"Voters (username,password are the DTMF voter-id/PIN): {voters_csv}")


if __name__ == "__main__":
    main()
