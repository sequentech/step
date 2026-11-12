#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Sets up idp-linking-authenticator for e2e_idp_linking.py.

Creates an upstream realm acting as an OIDC IdP, and in the e2e event realm: the
linked_idp_identities user-profile attribute, a first broker login flow running the authenticator
(REQUIRED, idp-claim=email) and the IdP bound to that flow. Voter V holds two upstream emails; W1
and W2 share one, which makes it ambiguous.
"""

import sys
import urllib.parse

from common import (
    KEYCLOAK_BROWSER_URL,
    KEYCLOAK_SELF_URL,
    TENANT_ID,
    Checks,
    data,
    errors,
    event_realm,
    graphql,
    keycloak,
    require_state,
    save_state,
)

UPSTREAM_REALM = "upgrade-tests-upstream-idp"
UPSTREAM_PASSWORD = "Upstream-Pass-1"
UPSTREAM_CLIENT = "event-broker"
UPSTREAM_SECRET = "upgrade-tests-broker-secret"
UPSTREAM_IDENTITIES = ("a", "b", "c", "shared")
IDP_ALIAS = "e2e-upstream"
FLOW = "e2e first broker login multivalue"
LINK_ATTRIBUTE = "linked_idp_identities"
VOTERS = {
    "e2e-link-v": ["a@e2e.test", "b@e2e.test"],
    "e2e-link-w1": ["shared@e2e.test"],
    "e2e-link-w2": ["shared@e2e.test"],
}


def created(checks: Checks, label: str, status: int, expected: int = 201) -> bool:
    return checks.check(label, status == expected, f"HTTP {status}")


def main() -> int:
    state = require_state("e2e_event_id")
    checks = Checks()
    realm = event_realm(state["e2e_event_id"])
    flow_path = urllib.parse.quote(FLOW)

    if keycloak("GET", f"/{UPSTREAM_REALM}").status == 200:
        keycloak("DELETE", f"/{UPSTREAM_REALM}")
        checks.note(f"removed {UPSTREAM_REALM} left over from an earlier run")
    created(
        checks,
        "create upstream realm",
        keycloak("POST", "", {"realm": UPSTREAM_REALM, "enabled": True}).status,
    )
    created(
        checks,
        "create broker client",
        keycloak(
            "POST",
            f"/{UPSTREAM_REALM}/clients",
            {
                "clientId": UPSTREAM_CLIENT,
                "enabled": True,
                "publicClient": False,
                "secret": UPSTREAM_SECRET,
                "standardFlowEnabled": True,
                "directAccessGrantsEnabled": False,
                "redirectUris": [
                    f"{KEYCLOAK_BROWSER_URL}/realms/{realm}/broker/{IDP_ALIAS}/endpoint"
                ],
            },
        ).status,
    )
    # email, firstName and lastName are set so the upstream realm doesn't ask to complete the profile.
    for name in UPSTREAM_IDENTITIES:
        created(
            checks,
            f"create upstream identity idp-{name}",
            keycloak(
                "POST",
                f"/{UPSTREAM_REALM}/users",
                {
                    "username": f"idp-{name}",
                    "email": f"{name}@e2e.test",
                    "emailVerified": True,
                    "firstName": "Idp",
                    "lastName": name.upper(),
                    "enabled": True,
                    "credentials": [
                        {
                            "type": "password",
                            "value": UPSTREAM_PASSWORD,
                            "temporary": False,
                        }
                    ],
                },
            ).status,
        )

    # Without a user profile entry, Keycloak drops the attribute's values.
    profile = keycloak("GET", f"/{realm}/users/profile").json()
    profile["attributes"].append(
        {
            "name": LINK_ATTRIBUTE,
            "displayName": "Linked IdP identities",
            "multivalued": True,
            "permissions": {"view": ["admin"], "edit": ["admin"]},
        }
    )
    created(
        checks,
        f"declare {LINK_ATTRIBUTE} in the user profile",
        keycloak("PUT", f"/{realm}/users/profile", profile).status,
        200,
    )

    created(
        checks,
        "create first broker login flow",
        keycloak(
            "POST",
            f"/{realm}/authentication/flows",
            {
                "alias": FLOW,
                "description": "upgrade-tests: link IdP identities by attribute",
                "providerId": "basic-flow",
                "topLevel": True,
                "builtIn": False,
            },
        ).status,
    )
    created(
        checks,
        "add idp-linking-authenticator",
        keycloak(
            "POST",
            f"/{realm}/authentication/flows/{flow_path}/executions/execution",
            {"provider": "idp-linking-authenticator"},
        ).status,
    )
    execution = keycloak(
        "GET", f"/{realm}/authentication/flows/{flow_path}/executions"
    ).json()[0]
    created(
        checks,
        "make it REQUIRED",
        keycloak(
            "PUT",
            f"/{realm}/authentication/flows/{flow_path}/executions",
            {**execution, "requirement": "REQUIRED"},
        ).status,
        204,
    )
    created(
        checks,
        "configure it (idp-claim=email)",
        keycloak(
            "POST",
            f"/{realm}/authentication/executions/{execution['id']}/config",
            {
                "alias": "upgrade-tests-idp-linking",
                "config": {"idp-claim": "email", "user-attribute": LINK_ATTRIBUTE},
            },
        ).status,
    )

    browser_base = (
        f"{KEYCLOAK_BROWSER_URL}/realms/{UPSTREAM_REALM}/protocol/openid-connect"
    )
    self_base = f"{KEYCLOAK_SELF_URL}/realms/{UPSTREAM_REALM}/protocol/openid-connect"
    created(
        checks,
        f"create IdP {IDP_ALIAS}",
        keycloak(
            "POST",
            f"/{realm}/identity-provider/instances",
            {
                "alias": IDP_ALIAS,
                "displayName": "E2E Upstream",
                "providerId": "keycloak-oidc",
                "enabled": True,
                "firstBrokerLoginFlowAlias": FLOW,
                "config": {
                    "clientId": UPSTREAM_CLIENT,
                    "clientSecret": UPSTREAM_SECRET,
                    "clientAuthMethod": "client_secret_post",
                    "authorizationUrl": f"{browser_base}/auth",
                    "tokenUrl": f"{self_base}/token",
                    "jwksUrl": f"{self_base}/certs",
                    "useJwksUrl": "true",
                    "validateSignature": "true",
                    "defaultScope": "openid email profile",
                    "syncMode": "IMPORT",
                    "pkceEnabled": "false",
                },
            },
        ).status,
    )

    # Voters are created through harvest's create_user, like the admin portal does.
    for username, linked in VOTERS.items():
        result = graphql(
            "mutation($t:String!,$e:String,$user:KeycloakUser2!){create_user(tenant_id:$t,election_event_id:$e,user:$user){id attributes}}",
            {
                "t": TENANT_ID,
                "e": state["e2e_event_id"],
                "user": {
                    "username": username,
                    "enabled": True,
                    "first_name": "E2E",
                    "last_name": username,
                    "attributes": {LINK_ATTRIBUTE: linked},
                },
            },
        )
        stored = ((data(result, "create_user") or {}).get("attributes") or {}).get(
            LINK_ATTRIBUTE
        )
        checks.check(
            f"create voter {username}", stored == linked, stored or errors(result)
        )

    save_state(
        e2e_upstream_realm=UPSTREAM_REALM,
        e2e_upstream_password=UPSTREAM_PASSWORD,
        e2e_idp_alias=IDP_ALIAS,
        e2e_link_flow=FLOW,
    )
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
