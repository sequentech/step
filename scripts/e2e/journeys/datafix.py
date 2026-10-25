# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The Datafix API as an external voter registry calls it.

The registry authenticates with the tenant realm's `datafix-account` client
(client credentials) and names its election event by the event's
`datafix:id` annotation. The isolated stack's tenant realm has no such client,
so the journey creates it the way an administrator would: a confidential
service-account client whose secret Keycloak generates, holding the
`datafix-account` role and the claim mappers of the API client.
"""

import copy
import json

from . import fixtures
from .client import ENV, TENANT_ID, TENANT_REALM, Http

CLIENT_ID = "datafix-account"
ROLE = "datafix-account"
HARVEST_URL = ENV["HARVEST_URL"].rstrip("/")
REGISTRY_ORIGIN = "https://registry.invalid"
PASSWORD_POLICY = {"base": "password-only", "size": 6, "characters": "numeric"}
# Inbound operations never call VoterView; the address is never dialled.
VOTERVIEW_REQUEST = {
    "url": "https://voterview.invalid",
    "usr": "registry",
    "psw": "registry",
    "county_mun": "e2e",
}


def ensure_client(keycloak):
    """Create the `datafix-account` role and client once; return its secret."""
    keycloak.admin("POST", f"{TENANT_REALM}/roles", {"name": ROLE}, expect=(201, 409))
    role = keycloak.admin("GET", f"{TENANT_REALM}/roles/{ROLE}")
    if not keycloak.admin("GET", f"{TENANT_REALM}/clients?clientId={CLIENT_ID}"):
        (api_client,) = keycloak.admin(
            "GET", f"{TENANT_REALM}/clients?clientId=api-key-client"
        )
        mappers = []
        for mapper in api_client.get("protocolMappers") or []:
            mapper = copy.deepcopy(mapper)
            mapper.pop("id", None)
            mappers.append(mapper)
        keycloak.admin(
            "POST",
            f"{TENANT_REALM}/clients",
            {
                "clientId": CLIENT_ID,
                "enabled": True,
                "publicClient": False,
                "serviceAccountsEnabled": True,
                "standardFlowEnabled": False,
                "directAccessGrantsEnabled": False,
                # Harvest's token claims require `allowed-origins`, which
                # Keycloak only issues to a client with a web origin.
                "webOrigins": [REGISTRY_ORIGIN],
                "protocolMappers": mappers,
                "defaultClientScopes": api_client.get("defaultClientScopes") or [],
                "optionalClientScopes": api_client.get("optionalClientScopes") or [],
            },
            expect=(201,),
        )
    (client,) = keycloak.admin("GET", f"{TENANT_REALM}/clients?clientId={CLIENT_ID}")
    if REGISTRY_ORIGIN not in (client.get("webOrigins") or []):
        keycloak.admin(
            "PUT",
            f"{TENANT_REALM}/clients/{client['id']}",
            {**client, "webOrigins": [REGISTRY_ORIGIN]},
            expect=(204,),
        )
    account = keycloak.admin(
        "GET", f"{TENANT_REALM}/clients/{client['id']}/service-account-user"
    )
    # The claim mappers read the tenant from the user, as for every tenant user.
    attributes = dict(account.get("attributes") or {})
    attributes["tenant-id"] = [TENANT_ID]
    keycloak.admin(
        "PUT",
        f"{TENANT_REALM}/users/{account['id']}",
        {**account, "attributes": attributes},
        expect=(204,),
    )
    keycloak.admin(
        "POST",
        f"{TENANT_REALM}/users/{account['id']}/role-mappings/realm",
        [role],
        expect=(204,),
    )
    # Keycloak generates a fresh secret, as the documented procedure asks;
    # Harvest caches tokens per secret, so a new one also avoids a token
    # cached before this setup.
    return keycloak.admin(
        "POST", f"{TENANT_REALM}/clients/{client['id']}/client-secret"
    )["value"]


def event_document(voting_portal_url, tag, datafix_id):
    """An event whose areas follow the Datafix naming, configured for `datafix_id`."""
    document = fixtures.election_event(voting_portal_url, f"{tag}-datafix")
    ward = ward_of(tag)
    names = [f"{ward}-000", f"{ward}-SB1-000", f"{ward}2-000"]
    for area, name in zip(document["areas"], names):
        area["name"] = name
    document["election_event"]["annotations"] = annotations(datafix_id)
    return document


def ward_of(tag):
    return f"W{tag.upper()}"


def annotations(datafix_id):
    return {
        "datafix:id": datafix_id,
        "datafix:password_policy": json.dumps(PASSWORD_POLICY),
        "datafix:voterview_request": json.dumps(VOTERVIEW_REQUEST),
    }


class Registry:
    """The external registry calling the Datafix API for one `datafix:id`."""

    def __init__(self, datafix_id, secret):
        self.headers = {
            "tenant-id": TENANT_ID,
            "event-id": datafix_id,
            "authorization": f"{CLIENT_ID}:{secret}",
        }

    def call(self, operation, body):
        response = Http().post(
            f"{HARVEST_URL}/api/datafix/{operation}",
            json_body=body,
            headers=self.headers,
        )
        return response.status, response.json() if response.body else None


def profile_without(profile, names):
    """The realm user profile minus `names` and any unmanaged-attribute policy."""
    trimmed = copy.deepcopy(profile)
    trimmed["attributes"] = [
        attribute
        for attribute in trimmed.get("attributes") or []
        if attribute.get("name") not in names
    ]
    trimmed.pop("unmanagedAttributePolicy", None)
    return trimmed


def inbound_entries(items):
    """The operation strings of the log's inbound Datafix entries, oldest first.

    Each item is a `listElectoralLog` row; its `message` is the signed message
    as JSON, whose ExternalApiRequest body is
    `[event_id, subject, direction, api_name, operation]`.
    """
    entries = []
    for item in sorted(items, key=lambda item: item["id"]):
        if item["statement_kind"] != "ExternalApiRequest":
            continue
        message = json.loads(item["message"])
        _, subject, direction, api_name, operation = message["statement"]["body"][
            "ExternalApiRequest"
        ]
        if direction != "Inbound" or api_name != "Datafix":
            continue
        entries.append(
            {
                "operation": operation,
                "description": message["statement"]["head"]["description"],
                "user_id": message.get("user_id"),
                "username": message.get("username"),
                "area_id": message.get("area_id"),
                "subject": subject,
            }
        )
    return entries
