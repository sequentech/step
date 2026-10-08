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
    errors,
    event_realm,
    graphql,
    http,
    keycloak,
    wait_task,
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
    realm = event_realm(event_id)

    status = wait_task((inserted.get("task_execution") or {}).get("id"))
    checks.check("insert task", status == "SUCCESS", status)
    realm_rep = keycloak("GET", f"/{realm}").json() or {}
    checks.check(
        "realm created",
        realm_rep.get("enabled") is True,
        {
            key: realm_rep.get(key)
            for key in ("loginTheme", "browserFlow", "registrationAllowed")
        },
    )
    clients = [
        client["clientId"]
        for client in keycloak("GET", f"/{realm}/clients?max=100").json()
    ]
    checks.check("realm clients", "voting-portal" in clients, clients)
    flows = [
        flow["alias"]
        for flow in keycloak("GET", f"/{realm}/authentication/flows").json()
        if not flow["builtIn"]
    ]
    checks.check("realm authentication flows", bool(flows), flows)
    profile = [
        attribute["name"]
        for attribute in keycloak("GET", f"/{realm}/users/profile").json()["attributes"]
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

    result = graphql(
        "mutation($e:String!){delete_election_event(election_event_id:$e){error_msg task_execution{id}}}",
        {"e": event_id},
    )
    deleted = data(result, "delete_election_event") or {}
    checks.check(
        "delete_election_event",
        bool(deleted) and not deleted.get("error_msg"),
        deleted or errors(result),
    )
    status = wait_task((deleted.get("task_execution") or {}).get("id"))
    checks.check("delete task", status == "SUCCESS", status)
    removed_status = keycloak("GET", f"/{realm}").status
    checks.check("realm removed", removed_status == 404, f"HTTP {removed_status}")
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
