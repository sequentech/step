#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Exercises the Rust Keycloak admin client (sequent-core) through the harvest actions the admin
portal uses, then creates one voter in each smoke event with known credentials for
smoke_portals.py. smoke_cleanup.py deletes them.
"""

import os
import sys

from common import (
    ADMIN_PORTAL_TEST_USERNAME,
    TENANT_ID,
    TENANT_REALM,
    Checks,
    data,
    errors,
    event_realm,
    graphql,
    keycloak,
    save_state,
    user_id,
    wait_task,
)

# An election event whose login form asks for username and password, and one whose form asks for
# a date of birth plus a 16-digit structured PIN (credential-input-policy=structured).
PASSWORD_EVENT_ID = os.environ.get(
    "KEYCLOAK_SMOKE_PASSWORD_EVENT_ID", "c8dafd25-392e-48d1-8e2e-87ec075ee414"
)
PASSWORD_EVENT_AREA_ID = os.environ.get(
    "KEYCLOAK_SMOKE_PASSWORD_EVENT_AREA_ID", "9239f309-4b03-4b2e-b44e-f8513761bd78"
)
PIN_EVENT_ID = os.environ.get(
    "KEYCLOAK_SMOKE_PIN_EVENT_ID", "3fce0796-1b85-4007-8ae5-a0c8d2bcc6ba"
)
PIN_EVENT_AREA_ID = os.environ.get(
    "KEYCLOAK_SMOKE_PIN_EVENT_AREA_ID", "fc7079b7-e903-494d-a30f-c822d5f452e4"
)

PASSWORD_VOTER = "upgrade-smoke-password"
PASSWORD_VOTER_SECRET = "Upgrade-Smoke-Pw42"
PIN_VOTER = "upgrade-smoke-pin"
PIN_VOTER_SECRET = "2680202610072026"
VOTER_DATE_OF_BIRTH = "1990-01-15"
EDITED_LAST_NAME = "Smoke-Edited"


def read_only_calls(checks: Checks) -> None:
    for event_id in (PASSWORD_EVENT_ID, PIN_EVENT_ID):
        short = event_id[:8]
        variables = {"t": TENANT_ID, "e": event_id}
        result = graphql(
            "query($t:String!,$e:String){get_user_profile_attributes(tenant_id:$t,election_event_id:$e){name}}",
            variables,
        )
        attributes = data(result, "get_user_profile_attributes")
        checks.check(
            f"get_user_profile_attributes ({short})",
            bool(attributes),
            [a["name"] for a in attributes or []] or errors(result),
        )
        result = graphql(
            "query($t:String!,$e:String){get_user_profile_configuration(tenant_id:$t,election_event_id:$e){__typename}}",
            variables,
        )
        checks.check(
            f"get_user_profile_configuration ({short})",
            data(result, "get_user_profile_configuration") is not None,
            errors(result),
        )
        result = graphql(
            "query($e:String!){get_realm_attributes(election_event_id:$e){attributes}}",
            {"e": event_id},
        )
        if "keycloak-realm-attributes-read" in str(result.get("errors")):
            checks.skip(
                f"get_realm_attributes ({short})",
                "admin lacks keycloak-realm-attributes-read (tenant predates it)",
            )
        else:
            realm_attributes = (data(result, "get_realm_attributes") or {}).get(
                "attributes"
            )
            checks.check(
                f"get_realm_attributes ({short})",
                realm_attributes is not None,
                (
                    f"{len(realm_attributes or {})} attributes"
                    if realm_attributes
                    else errors(result)
                ),
            )
        result = graphql(
            "query($e:String!){get_realm_password_policy(election_event_id:$e){configured minimum_length}}",
            {"e": event_id},
        )
        policy = data(result, "get_realm_password_policy")
        checks.check(
            f"get_realm_password_policy ({short})",
            policy is not None,
            policy or errors(result),
        )

    result = graphql(
        "query($t:String!){get_roles(body:{tenant_id:$t}){items{name}}}",
        {"t": TENANT_ID},
    )
    roles = [
        role["name"] for role in (data(result, "get_roles") or {}).get("items", [])
    ]
    checks.check("get_roles (tenant)", bool(roles), roles or errors(result))
    admin_id = user_id(TENANT_REALM, ADMIN_PORTAL_TEST_USERNAME)
    result = graphql(
        "query($t:String!,$u:String!){list_user_roles(tenant_id:$t,user_id:$u){name}}",
        {"t": TENANT_ID, "u": admin_id},
    )
    admin_roles = [role["name"] for role in data(result, "list_user_roles") or []]
    checks.check(
        "list_user_roles (admin)", bool(admin_roles), admin_roles or errors(result)
    )
    result = graphql(
        "mutation($t:uuid!,$u:String!){get_user(tenant_id:$t,user_id:$u){username enabled}}",
        {"t": TENANT_ID, "u": admin_id},
    )
    admin = data(result, "get_user") or {}
    checks.check(
        "get_user (admin)",
        admin.get("username") == ADMIN_PORTAL_TEST_USERNAME,
        admin or errors(result),
    )


def create_voter(
    checks: Checks, event_id: str, area_id: str, username: str, secret: str
) -> None:
    realm = event_realm(event_id)
    print(f"--- voter {username} in event {event_id}", flush=True)
    leftover = user_id(realm, username)
    if leftover:
        keycloak("DELETE", f"/{realm}/users/{leftover}")
        checks.note(f"removed {username} left over from an earlier run")

    result = graphql(
        "mutation($t:String!,$e:String,$user:KeycloakUser2!){create_user(tenant_id:$t,election_event_id:$e,user:$user){id}}",
        {
            "t": TENANT_ID,
            "e": event_id,
            "user": {
                "username": username,
                "enabled": True,
                "first_name": "Upgrade",
                "last_name": "Smoke",
                "email": f"{username}@example.com",
                "attributes": {
                    "area-id": [area_id],
                    "dateOfBirth": [VOTER_DATE_OF_BIRTH],
                },
            },
        },
    )
    created = data(result, "create_user") or {}
    if not checks.check(
        "create_user", bool(created.get("id")), created or errors(result)
    ):
        return
    voter_id = created["id"]

    # edit_user runs in windmill (EDIT_USER task): password reset plus a profile change.
    result = graphql(
        "mutation($body:EditUsersInput!){edit_user(body:$body){task_execution{id}}}",
        {
            "body": {
                "tenant_id": TENANT_ID,
                "election_event_id": event_id,
                "user_id": voter_id,
                "password": secret,
                "temporary": False,
                "last_name": EDITED_LAST_NAME,
            }
        },
    )
    task_id = ((data(result, "edit_user") or {}).get("task_execution") or {}).get("id")
    status = wait_task(task_id)
    checks.check(
        "edit_user task (password + last name)",
        status == "SUCCESS",
        status or errors(result),
    )

    result = graphql(
        "mutation($t:uuid!,$e:uuid,$id:String!){get_user(tenant_id:$t,election_event_id:$e,user_id:$id){last_name attributes}}",
        {"t": TENANT_ID, "e": event_id, "id": voter_id},
    )
    voter = data(result, "get_user") or {}
    attributes = voter.get("attributes") or {}
    summary = {
        "last_name": voter.get("last_name"),
        "area": (attributes.get("area-id") or [None])[0],
        "dateOfBirth": (attributes.get("dateOfBirth") or [None])[0],
    }
    checks.check(
        "get_user",
        summary
        == {
            "last_name": EDITED_LAST_NAME,
            "area": area_id,
            "dateOfBirth": VOTER_DATE_OF_BIRTH,
        },
        summary,
    )
    user = keycloak("GET", f"/{realm}/users/{voter_id}").json()
    credentials = [
        credential["type"]
        for credential in keycloak(
            "GET", f"/{realm}/users/{voter_id}/credentials"
        ).json()
    ]
    checks.check(
        "permanent password, no required actions",
        "password" in credentials and not user.get("requiredActions"),
        {"credentials": credentials, "requiredActions": user.get("requiredActions")},
    )
    result = graphql(
        "query($t:String!,$e:String,$id:String!){list_user_roles(tenant_id:$t,election_event_id:$e,user_id:$id){name}}",
        {"t": TENANT_ID, "e": event_id, "id": voter_id},
    )
    voter_roles = [role["name"] for role in data(result, "list_user_roles") or []]
    checks.check(
        "list_user_roles", "voter" in voter_roles, voter_roles or errors(result)
    )
    result = graphql(
        "query($t:uuid!,$e:uuid,$u:jsonb){get_users(body:{tenant_id:$t,election_event_id:$e,username:$u,limit:10}){items{username}}}",
        {"t": TENANT_ID, "e": event_id, "u": {"IsEqual": username}},
    )
    listed = [
        item["username"] for item in (data(result, "get_users") or {}).get("items", [])
    ]
    checks.check(
        "get_users (voters list, SQL)", username in listed, listed or errors(result)
    )


def main() -> int:
    checks = Checks()
    print("--- read-only calls", flush=True)
    read_only_calls(checks)
    create_voter(
        checks,
        PASSWORD_EVENT_ID,
        PASSWORD_EVENT_AREA_ID,
        PASSWORD_VOTER,
        PASSWORD_VOTER_SECRET,
    )
    create_voter(checks, PIN_EVENT_ID, PIN_EVENT_AREA_ID, PIN_VOTER, PIN_VOTER_SECRET)
    save_state(
        smoke_password_event_id=PASSWORD_EVENT_ID,
        smoke_password_voter=PASSWORD_VOTER,
        smoke_password_voter_secret=PASSWORD_VOTER_SECRET,
        smoke_pin_event_id=PIN_EVENT_ID,
        smoke_pin_voter=PIN_VOTER,
        smoke_pin_voter_secret=PIN_VOTER_SECRET,
        smoke_voter_date_of_birth=VOTER_DATE_OF_BIRTH,
    )
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
