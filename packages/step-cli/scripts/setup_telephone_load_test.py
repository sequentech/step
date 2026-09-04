#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Stage 1 of the load tests: provisions the same election event into one or
more tenants, bulk-creates DTMF-safe voters, runs the keys ceremony,
publishes, and opens the requested voting channel (TELEPHONE or ONLINE — see
the 'setup:' section of telephone-load-test-inputs/layers.yaml) — in every
target tenant. Writes one summary.json + voters CSV per tenant that Stage 2
consumes — run_telephone_load_test.py (driving `ivr-cli` calls) for
TELEPHONE, run_online_load_test.py (driving Playwright browsers) for ONLINE.
See docs/docusaurus/docs/07-developers/12-ivr/telephone-load-testing-design.md
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
import re
import string
import time
from pathlib import Path

import load_test_common as common

_CEREMONY_STATUS_RE = re.compile(r"Keys Ceremony status:\s*(\S+)")
_TRUSTEE_LINE_RE = re.compile(r"Trustee: name=(\S+) public_key=(\S+)")


def parse_trustees(list_trustees_output: str) -> list[tuple[str, str]]:
    return _TRUSTEE_LINE_RE.findall(list_trustees_output)


def rewrite_election_event_alias(election_event_json: Path, out_path: Path, suffix: str) -> str:
    """Appends the given suffix to the election event's alias (every
    language, so it stays visible regardless of the admin portal's UI
    language) — makes this run's election event easy to pick out in the
    admin portal's list, especially when several load-test runs (or several
    tenants from the same run) exist at once. The admin portal's election
    event list renders alias, falling back to name only where no alias is
    set — so alias, not name, is the field that's actually shown there.
    Where a language has no alias yet, seed it from that language's name so
    the suffix is visible in every language, not just the ones that already
    had one. The same suffix across every tenant in one run makes it easy to
    spot that they're all "the same event".

    Returns the resolved election_event_alias for logging."""
    with election_event_json.open() as f:
        data = json.load(f)

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
    return alias or "Unknown"


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


def wait_for_automatic_ceremony(step_cli_bin: str, election_event_id: str, key_ceremony_id: str, attempts: int = 120, delay: float = 10) -> None:
    """Polls get-key-ceremony-status until execution_status is SUCCESS. Each
    trustee's braid service posts its DKG round to the board on its own
    schedule (same as a manual ceremony); windmill auto-derives SUCCESS once
    every trustee's public key is on the board, so there's nothing for this
    script to submit — only to wait for. Refreshes the session's JWT every
    attempt, since a real DKG round can outlast one access token's lifetime."""
    for attempt in range(1, attempts + 1):
        try:
            common.run_step(step_cli_bin, "refresh-token")
            out = common.run_step(
                step_cli_bin, "get-key-ceremony-status",
                "--election-event-id", election_event_id,
                "--key-ceremony-id", key_ceremony_id,
            )
            match = _CEREMONY_STATUS_RE.search(out)
            status = match.group(1) if match else None
        except common.StepCliError:
            status = None
        if status == "SUCCESS":
            return
        if status == "CANCELLED":
            common.die(f"key ceremony {key_ceremony_id} was cancelled")
        if attempt >= attempts:
            common.die(f"key ceremony {key_ceremony_id} did not reach SUCCESS after {attempts} attempts (last status: {status})")
        common.log(f"    ceremony status: {status or 'unknown'} — waiting {delay}s (attempt {attempt}/{attempts})")
        time.sleep(delay)


def provision_tenant(
    *,
    step_cli_bin: str,
    tenant_id: str,
    tenant_out_dir: Path,
    configure_as,
    admin_portal_user: str,
    admin_portal_password: str,
    election_event_json: Path,
    election_event_alias_suffix: str,
    voter_area_name: str | None,
    num_voters: int,
    voter_pin_digits: int,
    voter_username_start: int,
    threshold: int,
    ceremony_policy: str,
    trustee1_user: str | None,
    trustee1_password: str | None,
    trustee2_user: str | None,
    trustee2_password: str | None,
    voting_channel: str,
    voting_portal_url: str,
    keycloak_url: str,
    endpoint_url: str,
) -> dict:
    """Imports the election event into tenant_id, generates and imports
    voters, runs the keys ceremony, publishes, and opens voting. Assumes the
    caller has already configure_as'd this tenant's admin_portal_user (so
    this function can be called back-to-back for several tenants without
    re-authenticating on entry — it does re-authenticate as needed for the
    trustee/admin steps in between)."""
    tenant_out_dir.mkdir(parents=True, exist_ok=True)
    common.log(f"Writing outputs to {tenant_out_dir}")

    common.log(f"[1/7] Importing election event from {election_event_json}")
    election_event_to_import = tenant_out_dir / "election-event-to-import.json"
    election_event_alias = rewrite_election_event_alias(election_event_json, election_event_to_import, election_event_alias_suffix)

    out = common.run_step(step_cli_bin, "import-election", "--file-path", str(election_event_to_import), "--is-local")
    election_event_id = common.extract_id(out)
    common.log(f"    election_event_id={election_event_id}")
    common.log(f"    election_event_alias={election_event_alias}")

    common.log(f"[2/7] Generating {num_voters} voters with numeric, {voter_pin_digits}-digit DTMF-safe credentials")
    # dateOfBirth is required: this realm's IVR auth flow (checked live via
    # its {realm}/ivr-config endpoint, not assumed from a generic default)
    # resolves voters by dateOfBirth + PIN rather than by username + PIN, so
    # a voter with no dateOfBirth attribute can never authenticate over a
    # call. generate-voters writes it in the realm's expected YYYY-MM-DD form
    # already.
    trimmed_election_event, resolved_voter_area = area_restricted_election_event(election_event_json, voter_area_name)
    common.write_json(tenant_out_dir / "election-event.json", trimmed_election_event)
    common.log(f"    voter_area={resolved_voter_area}")
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
    common.write_json(tenant_out_dir / "external_config.json", external_config)
    common.run_step(step_cli_bin, "generate-voters", "--working-directory", str(tenant_out_dir), "--num-users", str(num_voters))
    voters_csv = tenant_out_dir / f"voters_{num_voters}.csv"
    if not voters_csv.is_file():
        common.die(f"expected voters CSV at {voters_csv}, not found")
    common.log(f"    voters_csv={voters_csv}")

    common.log("[3/7] Bulk-importing voters into the election event")
    common.run_step(step_cli_bin, "import-voters", "--election-event-id", election_event_id, "--file-path", str(voters_csv), "--is-local")

    common.log(f"[4/7] Starting the keys ceremony (threshold={threshold}, policy={ceremony_policy})")
    start_args = [step_cli_bin, "start-key-ceremony", "--election-event-id", election_event_id, "--threshold", str(threshold)]
    if ceremony_policy == "AUTOMATIC":
        start_args.append("--automatic")
    out = common.run_step(*start_args)
    key_ceremony_id = common.extract_id(out)
    common.log(f"    key_ceremony_id={key_ceremony_id}")

    if ceremony_policy == "AUTOMATIC":
        common.log("[5/7] Waiting for the automatic keys ceremony to complete")
        wait_for_automatic_ceremony(step_cli_bin, election_event_id, key_ceremony_id)
    else:
        common.log(f"[5/7] Completing the keys ceremony as {trustee1_user}, then {trustee2_user}")
        configure_as(trustee1_user, trustee1_password, tenant_id)
        common.retry_step(step_cli_bin, 30, 5, "complete-key-ceremony", "--election-event-id", election_event_id, "--key-ceremony-id", key_ceremony_id)
        configure_as(trustee2_user, trustee2_password, tenant_id)
        common.retry_step(step_cli_bin, 30, 5, "complete-key-ceremony", "--election-event-id", election_event_id, "--key-ceremony-id", key_ceremony_id)
        configure_as(admin_portal_user, admin_portal_password, tenant_id)

    common.log(f"[6/7] Publishing and opening the {voting_channel} voting channel")
    common.run_step(step_cli_bin, "publish", "--election-event-id", election_event_id)
    common.run_step(
        step_cli_bin, "update-event-voting-status",
        "--election-event-id", election_event_id,
        "--voting-status", "OPEN",
        "--voting-channel", voting_channel,
    )

    common.log("[7/7] Writing summary")
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
        "voter_area": resolved_voter_area,
    }
    summary_path = tenant_out_dir / "summary.json"
    common.write_json(summary_path, summary)
    common.log(f'Done. Election event "{election_event_alias}" ({election_event_id}) is open for {voting_channel} voting in tenant {tenant_id}.')
    return summary


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

    # tenant_id is always required — it's the identity used to log in and,
    # when new_tenants > 0, to create the new tenants (see AskUserQuestion
    # decision: reusing admin_portal_user/tenant_id for tenant creation
    # rather than a separate super-admin identity). Whether it's ALSO one of
    # the tenants that receives the election event depends on new_tenants
    # below.
    tenant_id_explicit = bool(cfg.get("tenant_id"))
    tenant_id = common.req_str(cfg, "tenant_id", env="SUPER_ADMIN_TENANT_ID")

    # 0 when tenant_id was explicitly set in layers.yaml (use exactly that
    # tenant, today's behavior); 1 when it wasn't (replicates the old
    # "no tenant_id -> creates a new tenant" behavior). Set explicitly to
    # provision N new tenants regardless of whether tenant_id is given.
    new_tenants = cfg.get("new_tenants")
    new_tenants = int(new_tenants) if new_tenants is not None else (0 if tenant_id_explicit else 1)
    if new_tenants < 0:
        common.die("setup.new_tenants must be >= 0")

    num_voters = int(cfg.get("num_voters") or 20)
    voter_pin_digits = int(cfg.get("voter_pin_digits") or 6)
    if not (1 <= voter_pin_digits <= 8):
        common.die("setup.voter_pin_digits must be between 1 and 8 (DTMF voter auth limit)")
    voter_username_start = int(cfg.get("voter_username_start") or 100)
    if voter_username_start < 0:
        common.die("setup.voter_username_start must be >= 0")
    voter_area_name = cfg.get("voter_area_name") or None
    threshold = int(cfg.get("threshold") or 2)

    # AUTOMATIC: each trustee's braid service does its DKG round on its own
    # (same as MANUAL — this doesn't change how the ceremony's cryptography
    # runs), but the ceremony's completion is derived automatically once the
    # public key lands on the board, instead of requiring a human/CLI to log
    # in as each trustee and confirm via complete-key-ceremony. Matches the
    # Admin Portal's "automatic ceremony" checkbox.
    ceremony_policy = str(cfg.get("ceremony_policy") or "AUTOMATIC").upper()
    if ceremony_policy not in ("AUTOMATIC", "MANUAL"):
        common.die("setup.ceremony_policy must be AUTOMATIC or MANUAL")

    endpoint_url = common.req_str(cfg, "endpoint_url", env="HASURA_ENDPOINT")
    keycloak_url = common.req_str(cfg, "keycloak_url", env="KEYCLOAK_URL")
    # Tenant-realm login: a user holding the admin-user role inside a
    # tenant's own realm — the same identity that logs into the Admin Portal
    # to manage that tenant. Used for every election-management step below
    # (import, publish, open voting) — and, when new_tenants > 0, assumed to
    # also work unmodified against each freshly created tenant (same
    # username/password, since they're seeded from the same tenant
    # template).
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
    # -> Credentials tab. Only valid for tenant_id itself — a freshly
    # created tenant gets its own, different, randomly generated secret,
    # looked up live below via keycloak_admin_user/password.
    keycloak_client_secret = common.req_str(cfg, "keycloak_client_secret", env="API_KEY_CLIENT_SECRET")

    # Only needed when new_tenants > 0: looking up a freshly created
    # tenant's api-key-client secret requires a Keycloak *platform*
    # master-realm admin token (distinct from admin_portal_user, which is
    # scoped to one tenant's realm) — see lookup_client_secret in
    # load_test_common.py.
    keycloak_admin_user = keycloak_admin_password = None
    if new_tenants > 0:
        keycloak_admin_user = common.req_str(cfg, "keycloak_admin_user", env="KEYCLOAK_ADMIN")
        keycloak_admin_password = common.req_str(cfg, "keycloak_admin_password", env="KEYCLOAK_ADMIN_PASSWORD")

    # Only needed for ceremony_policy: MANUAL — an AUTOMATIC ceremony never
    # calls complete-key-ceremony, so no trustee Keycloak login is required.
    trustee1_user = trustee1_password = trustee2_user = trustee2_password = None
    if ceremony_policy == "MANUAL":
        trustee1_user = str(cfg.get("trustee1_user") or "trustee1")
        trustee1_password = common.req_str(cfg, "trustee1_password", env="TRUSTEE1_PASSWORD")
        trustee2_user = str(cfg.get("trustee2_user") or "trustee2")
        trustee2_password = common.req_str(cfg, "trustee2_password", env="TRUSTEE2_PASSWORD")

    out_dir = common.resolve_path(common.req_str(cfg, "out_dir"))

    step_cli_bin = common.find_step_cli()

    # tenant_id -> that tenant's api-key-client secret. Seeded with the
    # bootstrap tenant's already-known secret; new tenants get theirs added
    # as they're created.
    client_secrets = {tenant_id: keycloak_client_secret}

    def configure_as(user: str, password: str, target_tenant_id: str) -> None:
        common.run_step(
            step_cli_bin,
            "config",
            "--tenant-id", target_tenant_id,
            "--endpoint-url", endpoint_url,
            "--keycloak-url", keycloak_url,
            "--keycloak-user", user,
            "--keycloak-password", password,
            "--keycloak-client-id", keycloak_client_id,
            "--keycloak-client-secret", client_secrets[target_tenant_id],
        )

    common.log(f"Authenticating as admin-portal user ({admin_portal_user}) against tenant {tenant_id}")
    configure_as(admin_portal_user, admin_portal_password, tenant_id)

    if new_tenants > 0:
        common.log(f"Creating {new_tenants} new tenant(s), cloned from tenant {tenant_id}")

        common.log(f"Exporting tenant {tenant_id}'s Keycloak/roles config")
        out = common.run_step(step_cli_bin, "export-tenant-config", "--tenant-id", tenant_id)
        export_document_id = common.extract_id(out)
        common.log(f"    document_id={export_document_id}")
        # Documents are tenant-owned records — a document exported under
        # tenant_id's ownership isn't visible to import-tenant-config once
        # it's targeting a different tenant. Download the export once here
        # (still authenticated as tenant_id, the owner) and re-upload it
        # per new tenant below, once authenticated as that tenant, so each
        # import references a document that tenant actually owns.
        export_zip_path = out_dir / "tenant-config-export.zip"
        common.run_step(step_cli_bin, "download-document", "--document-id", export_document_id, "--output", str(export_zip_path))

        common.log(f"Reading tenant {tenant_id}'s registered trustees to replicate")
        out = common.run_step(step_cli_bin, "list-trustees")
        trustees_to_seed = parse_trustees(out)
        if not trustees_to_seed:
            common.die(f"tenant {tenant_id} has no registered trustees to copy into new tenants")
        common.log(f"    {len(trustees_to_seed)} trustee(s): {', '.join(name for name, _ in trustees_to_seed)}")

        # Stay authenticated as tenant_id (the bootstrap/super-admin identity)
        # for every create-tenant call below — harvest's insertTenant action
        # requires the caller to BE the super-admin tenant, so switching to a
        # just-created tenant mid-loop (as configure_as would) breaks
        # authorization for the next create-tenant call. Each new tenant is
        # configure_as'd only briefly, to seed its trustees, then switched
        # straight back to tenant_id before the loop continues.
        target_tenant_ids = []
        for i in range(new_tenants):
            slug = "loadtest-" + "".join(random.choices(string.ascii_lowercase + string.digits, k=8))
            out = common.run_step(step_cli_bin, "create-tenant", "--slug", slug)
            new_tenant_id = common.extract_id(out)
            common.log(f"    created tenant {i + 1}/{new_tenants}: tenant_id={new_tenant_id} (slug={slug})")

            client_secrets[new_tenant_id] = common.lookup_client_secret(
                keycloak_url, keycloak_admin_user, keycloak_admin_password, new_tenant_id, keycloak_client_id
            )
            # import-tenant-config's task_execution row is tagged with the
            # TARGET tenant (new_tenant_id), not the caller's own session
            # tenant — polling for its completion only works from a session
            # already authenticated as new_tenant_id, so switch before
            # calling it (not after, as the earlier trustee-seeding-only
            # version of this loop did).
            configure_as(admin_portal_user, admin_portal_password, new_tenant_id)

            common.log(f"    importing tenant {tenant_id}'s Keycloak/roles config into {new_tenant_id}")
            # First authenticated call against a freshly created tenant: its
            # realm's JWKS may not have propagated to every backend instance
            # yet, which manifests as a JWT signature verification failure
            # despite a perfectly valid token — retry rather than treat that
            # as fatal.
            out = common.retry_step(step_cli_bin, 10, 3, "upload-document", "--file-path", str(export_zip_path))
            tenant_scoped_document_id = common.extract_id(out)
            common.run_step(
                step_cli_bin, "import-tenant-config",
                "--tenant-id", new_tenant_id, "--document-id", tenant_scoped_document_id,
            )
            # The import may have regenerated api-key-client's secret again
            # (same per-realm randomization as tenant creation) or replaced
            # the client outright — refresh the secret and re-authenticate
            # rather than assume the pre-import session is still valid.
            client_secrets[new_tenant_id] = common.lookup_client_secret(
                keycloak_url, keycloak_admin_user, keycloak_admin_password, new_tenant_id, keycloak_client_id
            )
            configure_as(admin_portal_user, admin_portal_password, new_tenant_id)

            common.log(f"    seeding {len(trustees_to_seed)} trustee(s) into {new_tenant_id}")
            for name, public_key in trustees_to_seed:
                common.run_step(step_cli_bin, "create-trustee", "--name", name, "--public-key", public_key)
            configure_as(admin_portal_user, admin_portal_password, tenant_id)

            target_tenant_ids.append(new_tenant_id)
    else:
        target_tenant_ids = [tenant_id]

    out_dir.mkdir(parents=True, exist_ok=True)
    # Shared across every tenant provisioned below, so the same election
    # event is easy to recognize by alias in each tenant's admin portal.
    alias_suffix = "".join(random.choices(string.ascii_uppercase + string.digits, k=5))

    summaries = []
    for i, target_tenant_id in enumerate(target_tenant_ids):
        common.log(f"=== Provisioning tenant {i + 1}/{len(target_tenant_ids)}: {target_tenant_id} ===")
        configure_as(admin_portal_user, admin_portal_password, target_tenant_id)
        summary = provision_tenant(
            step_cli_bin=step_cli_bin,
            tenant_id=target_tenant_id,
            tenant_out_dir=out_dir / f"tenant-{target_tenant_id}",
            configure_as=configure_as,
            admin_portal_user=admin_portal_user,
            admin_portal_password=admin_portal_password,
            election_event_json=election_event_json,
            election_event_alias_suffix=alias_suffix,
            voter_area_name=voter_area_name,
            num_voters=num_voters,
            voter_pin_digits=voter_pin_digits,
            voter_username_start=voter_username_start,
            threshold=threshold,
            ceremony_policy=ceremony_policy,
            trustee1_user=trustee1_user,
            trustee1_password=trustee1_password,
            trustee2_user=trustee2_user,
            trustee2_password=trustee2_password,
            voting_channel=voting_channel,
            voting_portal_url=voting_portal_url,
            keycloak_url=keycloak_url,
            endpoint_url=endpoint_url,
        )
        summaries.append(summary)

    tenants_index_path = out_dir / "tenants.json"
    common.write_json(tenants_index_path, {
        "tenant_ids": target_tenant_ids,
        "tenants": [{"tenant_id": s["tenant_id"], "dir": f"tenant-{s['tenant_id']}"} for s in summaries],
    })

    common.log(f"Done. Provisioned {len(target_tenant_ids)} tenant(s): {', '.join(target_tenant_ids)}")
    common.log(f"Tenant index: {tenants_index_path}")


if __name__ == "__main__":
    main()
