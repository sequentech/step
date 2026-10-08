#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Deletes the voters smoke_admin_client.py created, through harvest's delete_user action, and the
election event smoke_realm_lifecycle.py created if it did not get to delete it.
"""

import sys

from common import (
    TENANT_ID,
    Checks,
    data,
    delete_election_event,
    election_event_exists,
    errors,
    event_realm,
    forget_state,
    graphql,
    load_state,
    user_id,
)


def main() -> int:
    state = load_state()
    checks = Checks()
    for event_key, voter_key in (
        ("smoke_password_event_id", "smoke_password_voter"),
        ("smoke_pin_event_id", "smoke_pin_voter"),
    ):
        event_id, username = state.get(event_key), state.get(voter_key)
        if not event_id or not username:
            continue
        realm = event_realm(event_id)
        voter_id = user_id(realm, username)
        if not voter_id:
            checks.skip(f"delete_user ({username})", "not found")
            continue
        result = graphql(
            "mutation($t:String!,$e:String,$u:String!){delete_user(tenant_id:$t,election_event_id:$e,user_id:$u){id}}",
            {"t": TENANT_ID, "e": event_id, "u": voter_id},
        )
        checks.check(
            f"delete_user ({username})",
            data(result, "delete_user") is not None,
            errors(result) if result.get("errors") else "",
        )
        checks.check(
            f"voter gone from Keycloak ({username})",
            user_id(realm, username) is None,
            "",
        )
    event_id = state.get("smoke_realm_event_id")
    if event_id:
        if not election_event_exists(event_id):
            checks.skip("delete smoke realm event", "already deleted")
            forget_state("smoke_realm_event_id")
        elif delete_election_event(checks, event_id):
            forget_state("smoke_realm_event_id")
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
