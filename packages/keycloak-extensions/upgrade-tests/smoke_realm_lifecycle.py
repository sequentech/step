#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Realm lifecycle through the Rust Keycloak admin client: creating an election event makes windmill
import the election event realm template into Keycloak; deleting the event removes the realm.
"""

import re
import sys
import urllib.parse

from common import (
    KEYCLOAK_URL,
    TENANT_ID,
    VOTING_PORTAL_URL,
    Checks,
    data,
    delete_election_event,
    errors,
    event_realm,
    forget_state,
    graphql,
    http,
    keycloak,
    save_state,
    wait_task,
)


def inspect_realm(checks: Checks, realm: str) -> None:
    realm_rep = keycloak("GET", f"/{realm}").json() or {}
    if not checks.check(
        "realm created",
        realm_rep.get("enabled") is True,
        {
            key: realm_rep.get(key)
            for key in ("loginTheme", "browserFlow", "registrationAllowed")
        },
    ):
        return
    clients = [
        client["clientId"]
        for client in keycloak("GET", f"/{realm}/clients?max=100").json() or []
    ]
    checks.check("realm clients", "voting-portal" in clients, clients)
    flows = [
        flow["alias"]
        for flow in keycloak("GET", f"/{realm}/authentication/flows").json() or []
        if not flow["builtIn"]
    ]
    checks.check("realm authentication flows", bool(flows), flows)
    profile = [
        attribute["name"]
        for attribute in (keycloak("GET", f"/{realm}/users/profile").json() or {}).get(
            "attributes", []
        )
    ]
    checks.check("realm user profile", bool(profile), profile)

    redirect = urllib.parse.quote(f"{VOTING_PORTAL_URL}/", safe="")
    login_page = http(
        "GET",
        f"{KEYCLOAK_URL}/realms/{realm}/protocol/openid-connect/auth"
        f"?client_id=voting-portal&response_type=code&scope=openid&redirect_uri={redirect}",
    ).body
    theme = re.search(r"/login/(sequent\.[a-z-]+)/", login_page)
    checks.check(
        "login page renders with the Sequent theme",
        'id="kc-login"' in login_page and theme is not None,
        theme.group(1) if theme else "no Sequent theme resources",
    )


def main() -> int:
    checks = Checks()
    result = graphql(
        "mutation($ev:CreateElectionEventInput!){insertElectionEvent(object:$ev){id error task_execution{id}}}",
        {
            "ev": {
                "tenant_id": TENANT_ID,
                "name": "Keycloak upgrade smoke realm",
                "description": "Temporary event created by upgrade-tests/smoke_realm_lifecycle.py",
            }
        },
    )
    inserted = data(result, "insertElectionEvent") or {}
    if not checks.check(
        "insertElectionEvent",
        bool(inserted.get("id")) and not inserted.get("error"),
        inserted or errors(result),
    ):
        return checks.finish()
    event_id = inserted["id"]
    # Recorded first so smoke_cleanup.py finds the event if this script dies.
    save_state(smoke_realm_event_id=event_id)
    try:
        status = wait_task((inserted.get("task_execution") or {}).get("id"))
        if checks.check("insert task", status == "SUCCESS", status):
            inspect_realm(checks, event_realm(event_id))
    finally:
        if delete_election_event(checks, event_id):
            forget_state("smoke_realm_event_id")
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
