# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Provision the local browser trustee accounts on fresh or existing realms."""

from __future__ import annotations

import argparse
import copy
import json
import os
import sys
from typing import Any
from urllib.parse import quote

from scripts.dev.mode.checkout import load_checkout, running_in_container

BROWSER_TRUSTEES = ("trustee1-wbraid", "trustee2-wbraid")
REALM_FIXTURE = (
    ".devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
)


def ensure_account(
    keycloak: Any, realm: str, fixture: dict, group_id: str, password: str
) -> None:
    """Keep existing passwords, MFA, profile fields and unrelated attributes."""
    username = fixture["username"]
    query = f"{realm}/users?exact=true&username={quote(username, safe='')}"
    users = keycloak.admin("GET", query)
    if not users:
        keycloak.admin("POST", f"{realm}/users", fixture, expect=(201,))
        users = keycloak.admin("GET", query)
    if len(users) != 1:
        raise RuntimeError(f"Expected one {username} in {realm}, found {len(users)}")
    user = users[0]
    path = f"{realm}/users/{user['id']}"
    updated = copy.deepcopy(user)
    attributes = updated.setdefault("attributes", {})
    attributes.update(fixture["attributes"])
    for field in ("firstName", "lastName", "email"):
        if not updated.get(field):
            updated[field] = fixture[field]
    if updated != user:
        keycloak.admin("PUT", path, updated, expect=(204,))
    credentials = keycloak.admin("GET", f"{path}/credentials")
    if not any(credential["type"] == "password" for credential in credentials):
        keycloak.admin(
            "PUT",
            f"{path}/reset-password",
            {"type": "password", "value": password, "temporary": False},
            expect=(204,),
        )
    groups = keycloak.admin("GET", f"{path}/groups")
    if not any(group["id"] == group_id for group in groups):
        keycloak.admin("PUT", f"{path}/groups/{group_id}", expect=(204,))


def configure_dev_otp(keycloak: Any, realm: str, code: str) -> None:
    """The local dummy sender discards codes; use a known synthetic OTP.

    Enrollment resolves the first message OTP configuration in the realm,
    while login uses its own flow. Configure both, including nested flows.
    """
    configured: set[str] = set()
    flows = keycloak.admin("GET", f"{realm}/authentication/flows")
    for flow in flows:
        alias = quote(flow["alias"], safe="")
        steps = keycloak.admin(
            "GET", f"{realm}/authentication/flows/{alias}/executions"
        )
        for step in steps:
            if step.get("providerId") != "message-otp-authenticator":
                continue
            config_id = step.get("authenticationConfig")
            if not config_id or config_id in configured:
                continue
            path = f"{realm}/authentication/config/{config_id}"
            config = keycloak.admin("GET", path)
            updated = copy.deepcopy(config)
            updated.setdefault("config", {}).update(
                {"test-mode": "true", "test-mode-code": code}
            )
            if updated != config:
                keycloak.admin("PUT", path, updated, expect=(204,))
            configured.add(config_id)
    if not configured:
        raise RuntimeError(f"{realm} has no message OTP configuration")


def enroll_dev_mfa(keycloak: Any, realm: str, username: str) -> None:
    """Seed email MFA for synthetic accounts, retaining any existing MFA.

    Required-action enrollment validates a generated code even in test mode.
    The dummy sender discards it, so import the local fixture credential through
    Keycloak's user API. Message OTP stores only its enrollment marker.
    """
    user = keycloak.user(realm, username)
    path = f"{realm}/users/{user['id']}"
    credentials = keycloak.admin("GET", f"{path}/credentials")
    mfa_types = {"message-otp", "otp", "webauthn", "webauthn-passwordless"}
    if any(credential["type"] in mfa_types for credential in credentials):
        return
    updated = copy.deepcopy(user)
    updated["emailVerified"] = True
    updated["credentials"] = [
        {
            "type": "message-otp",
            "credentialData": '{"isSetup":true}',
            "secretData": '{"value":"nothing"}',
        }
    ]
    keycloak.admin("PUT", path, updated, expect=(204,))


def main() -> int:
    parser = argparse.ArgumentParser(
        prog="step-dev browser_trustees",
        description="Provision browser trustee accounts and synthetic OTP in the "
        "local dev realm. Existing passwords, MFA and native accounts are preserved.",
    )
    parser.parse_args()
    checkout = load_checkout()
    environment = {**checkout.env, **os.environ}
    if not running_in_container():
        environment["KEYCLOAK_URL"] = (
            f"http://127.0.0.1:{checkout.env.get('KC_HTTP_PORT', '8090')}"
        )
    os.environ.update(environment)
    # The existing clients read their endpoints from the environment on import.
    from scripts.e2e.journeys.client import Keycloak

    fixture = json.loads((checkout.root / REALM_FIXTURE).read_text())
    realm = fixture["realm"]
    if realm != f"tenant-{environment['SUPER_ADMIN_TENANT_ID']}":
        raise RuntimeError(
            "Browser trustee fixtures require the default development tenant"
        )
    keycloak = Keycloak()
    group = keycloak.admin("GET", f"{realm}/group-by-path/trustee")
    for index, name in enumerate(BROWSER_TRUSTEES, 1):
        user = next(user for user in fixture["users"] if user["username"] == name)
        password = environment.get(f"BROWSER_TRUSTEE{index}_PASSWORD") or name
        ensure_account(keycloak, realm, user, group["id"], password)
        enroll_dev_mfa(keycloak, realm, name)
        print(f"Ready: {name} (trustee claim: {name})")
    code = environment.get("BROWSER_TRUSTEE_OTP_CODE", "123456")
    configure_dev_otp(keycloak, realm, code)
    print(f"Local development tenant MFA test code: {code}")
    print("Open http://localhost:3002/trustee in separate browser profiles.")
    print(
        "Sign in with the local test OTP, import or generate keys, "
        "and register both keys"
    )
    print("under the matching trustee names before starting the ceremony.")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError, AssertionError) as error:
        print(f"browser_trustees: {error}", file=sys.stderr)
        sys.exit(1)
