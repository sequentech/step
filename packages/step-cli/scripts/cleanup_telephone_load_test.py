#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Stage 3 of the telephone (IVR/DTMF) load test: deletes every election
event Stage 1 (setup_telephone_load_test.py) provisioned, then deletes every
tenant Stage 1 auto-created (setup.new_tenants) along with them.

Reads Stage 1's tenants.json (under setup.out_dir) and each
tenant-<id>/summary.json, re-authenticates step-cli against each tenant in
turn (a session is scoped to one tenant at a time), and calls
delete-election-event for that tenant's election_event_id — the same
sequence documented in the guide's "Clean up" step, automated.

Once every tenant's election event is gone, tenants are deleted too via
delete-tenant (a tenant can't be deleted while it still has election events
— see windmill's delete_tenant task). The bootstrap tenant (setup.tenant_id)
is never deleted, even if it ends up with no election events left — it's the
persistent identity Stage 1 authenticates as, meant to be reused across
runs.

Unlike the other load-test scripts, this one DOES take command-line flags —
they scope how destructive a run is, which doesn't belong in layers.yaml
(a deployment target, not a per-invocation choice):

  --events-only       delete election events only; leave every tenant realm
                       in place (including ones setup.new_tenants created).
  --new-tenants-only  delete election events as usual, but only delete
                       tenants tenants.json marks as source: "new" (created
                       by setup.new_tenants this run) — tenants reused via
                       setup.use_existing_tenants, and any tenants.json
                       entry with no source (e.g. hand-written), are left in
                       place.

With neither flag: delete every election event and every non-bootstrap
tenant, matching the default behavior this script always had.

Reads the same layers.yaml `setup:` section Stage 1 used (endpoint_url /
keycloak_url / admin credentials / out_dir must match what provisioned
these tenants).
"""

from __future__ import annotations

import argparse
import json

import load_test_common as common


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    scope = parser.add_mutually_exclusive_group()
    scope.add_argument(
        "--events-only",
        action="store_true",
        help="Delete election events only; leave every tenant realm in place.",
    )
    scope.add_argument(
        "--new-tenants-only",
        action="store_true",
        help='Delete election events as usual, but only delete tenants.json entries marked source: "new".',
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    config = common.load_config()
    cfg = common.section(config, "setup")

    endpoint_url = common.req_str(cfg, "endpoint_url", env="HASURA_ENDPOINT")
    keycloak_url = common.req_str(cfg, "keycloak_url", env="KEYCLOAK_URL")
    admin_portal_user = common.req_str(cfg, "admin_portal_user", env="ADMIN_PORTAL_TEST_USERNAME")
    admin_portal_password = common.req_str(cfg, "admin_portal_password", env="ADMIN_PORTAL_TEST_PASSWORD")
    keycloak_client_id = common.req_str(cfg, "keycloak_client_id")
    bootstrap_tenant_id = common.req_str(cfg, "tenant_id", env="SUPER_ADMIN_TENANT_ID")
    bootstrap_client_secret = common.req_str(cfg, "keycloak_client_secret", env="API_KEY_CLIENT_SECRET")

    out_dir = common.resolve_path(common.req_str(cfg, "out_dir"))
    tenants_json = out_dir / "tenants.json"
    if not tenants_json.is_file():
        common.die(f"no such file: {tenants_json} (run setup_telephone_load_test.py first)")
    with tenants_json.open() as f:
        tenants = json.load(f)["tenants"]

    targets = []
    for t in tenants:
        summary_path = out_dir / t["dir"] / "summary.json"
        with summary_path.open() as f:
            summary = json.load(f)
        targets.append((t["tenant_id"], summary["election_event_id"], t.get("source")))

    non_bootstrap_tenant_ids = [tenant_id for tenant_id, _, _ in targets if tenant_id != bootstrap_tenant_id]
    if args.events_only:
        tenant_ids_to_delete = []
    elif args.new_tenants_only:
        # source is missing on tenants.json entries written before this
        # field existed (or hand-written ones) — treat those as NOT new, the
        # same conservative default as the bootstrap tenant, rather than
        # risk deleting a tenant nothing actually confirms this run created.
        tenant_ids_to_delete = [
            tenant_id for tenant_id, _, source in targets if tenant_id != bootstrap_tenant_id and source == "new"
        ]
    else:
        tenant_ids_to_delete = non_bootstrap_tenant_ids

    # Only the bootstrap tenant's client secret is known up front (from
    # layers.yaml); any other tenant got its own, randomly generated one,
    # which needs looking up the same way setup_telephone_load_test.py did
    # (whether it created that tenant or just reused it via
    # use_existing_tenants — either way its secret isn't known ahead of
    # time). Needed for every non-bootstrap tenant in targets regardless of
    # scope flags, since deleting its election event still requires
    # authenticating into it first.
    keycloak_admin_user = keycloak_admin_password = None
    if non_bootstrap_tenant_ids:
        keycloak_admin_user = common.req_str(cfg, "keycloak_admin_user", env="KEYCLOAK_ADMIN")
        keycloak_admin_password = common.req_str(cfg, "keycloak_admin_password", env="KEYCLOAK_ADMIN_PASSWORD")

    step_cli_bin = common.find_step_cli()

    def configure_as(tenant_id: str, client_secret: str) -> None:
        common.log(f"Authenticating as admin-portal user ({admin_portal_user}) against tenant {tenant_id}")
        common.run_step(
            step_cli_bin,
            "config",
            "--tenant-id", tenant_id,
            "--endpoint-url", endpoint_url,
            "--keycloak-url", keycloak_url,
            "--keycloak-user", admin_portal_user,
            "--keycloak-password", admin_portal_password,
            "--keycloak-client-id", keycloak_client_id,
            "--keycloak-client-secret", client_secret,
        )

    common.log(f"Deleting {len(targets)} election event(s) across {len({t for t, _, _ in targets})} tenant(s)")
    for tenant_id, election_event_id, _ in targets:
        client_secret = (
            bootstrap_client_secret
            if tenant_id == bootstrap_tenant_id
            else common.lookup_client_secret(
                keycloak_url, keycloak_admin_user, keycloak_admin_password, tenant_id, keycloak_client_id
            )
        )
        configure_as(tenant_id, client_secret)

        common.log(f"Deleting election event {election_event_id} (tenant {tenant_id})")
        common.run_step(step_cli_bin, "delete-election-event", "--election-event-id", election_event_id)

    if args.events_only:
        common.log("--events-only: leaving every tenant realm in place.")
    elif tenant_ids_to_delete:
        common.log(f"Deleting {len(tenant_ids_to_delete)} tenant(s)")
        # delete-tenant is a super-admin-only action (same authorization
        # model as create-tenant) — authenticate as the bootstrap tenant for
        # every call below, passing the target tenant as --tenant-id rather
        # than switching sessions into it.
        configure_as(bootstrap_tenant_id, bootstrap_client_secret)
        for tenant_id in tenant_ids_to_delete:
            common.log(f"Deleting tenant {tenant_id}")
            common.run_step(step_cli_bin, "delete-tenant", "--tenant-id", tenant_id)
    elif args.new_tenants_only:
        common.log("--new-tenants-only: no tenants.json entry is marked source \"new\" — nothing to delete.")

    common.log("Done.")


if __name__ == "__main__":
    main()
