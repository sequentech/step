#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Deletes the voters smoke_admin_client.py created, through harvest's delete_user action."""

import sys

from common import (
    TENANT_ID,
    Checks,
    data,
    errors,
    event_realm,
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
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
