#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Apply the public client hardening of `upsert_realm` to realms that exist.

New realms get it from `upsert_realm` (sequent-core `realm.rs`); this script
brings realms created before that change, and realm template files, to the
same settings:

- voting-portal, onsite-voting-portal, voting-portal-kiosk, results-portal and
  admin-portal: no direct access (password) or implicit grant, PKCE S256, web
  origins and post-logout redirects limited to the client's redirect URIs.
- admin-portal: redirect URIs pinned to --admin-portal-url, when given.
- account-console: redirects only to its realm's account pages.
- cli-account-admin: confidential (password grant plus client secret), no
  browser logins, and the admin-portal token claims it lacks.

Usage:
    harden_public_clients.py template FILE...        rewrite realm JSON files
    harden_public_clients.py realm [--apply] (--realm NAME... | --all-tenant-realms)
        Reads KEYCLOAK_URL, KEYCLOAK_ADMIN and KEYCLOAK_ADMIN_PASSWORD (master
        realm admin). Without --apply it only prints what it would change.
"""

import argparse
import copy
import json
import os
import secrets
import string
import sys
import urllib.parse
import urllib.request

PUBLIC_BROWSER_CLIENT_IDS = (
    "voting-portal",
    "onsite-voting-portal",
    "voting-portal-kiosk",
    "results-portal",
    "admin-portal",
)
ADMIN_PORTAL = "admin-portal"
ACCOUNT_CONSOLE = "account-console"
CLI_ACCOUNT_ADMIN = "cli-account-admin"


def generate_client_secret():
    alphabet = string.ascii_letters + string.digits
    return "".join(secrets.choice(alphabet) for _ in range(32))


def harden_public_browser_client(client):
    client["directAccessGrantsEnabled"] = False
    client["implicitFlowEnabled"] = False
    client["webOrigins"] = ["+"]
    attributes = client.setdefault("attributes", {})
    attributes["pkce.code.challenge.method"] = "S256"
    attributes["post.logout.redirect.uris"] = "+"


def make_cli_client_confidential(client, admin_portal):
    client["publicClient"] = False
    client["clientAuthenticatorType"] = "client-secret"
    if not client.get("secret"):
        client["secret"] = generate_client_secret()
    client["directAccessGrantsEnabled"] = True
    client["standardFlowEnabled"] = False
    client["implicitFlowEnabled"] = False
    client["redirectUris"] = []
    client["webOrigins"] = []
    if admin_portal is not None:
        mappers = client.setdefault("protocolMappers", [])
        names = {mapper.get("name") for mapper in mappers}
        for mapper in admin_portal.get("protocolMappers", []):
            if mapper.get("name") not in names:
                mapper = copy.deepcopy(mapper)
                mapper.pop("id", None)
                mappers.append(mapper)


def harden_clients(clients, realm, admin_portal_url=None):
    """Harden `clients` (realm representation dicts) in place, as `upsert_realm`
    does. Returns the clientIds that changed."""
    admin_portal = next(
        (client for client in clients if client.get("clientId") == ADMIN_PORTAL), None
    )
    admin_portal = copy.deepcopy(admin_portal)
    changed = []
    for client in clients:
        before = copy.deepcopy(client)
        client_id = client.get("clientId")
        if client_id == ACCOUNT_CONSOLE:
            client["redirectUris"] = [f"/realms/{realm}/account/*"]
            harden_public_browser_client(client)
        elif client_id == CLI_ACCOUNT_ADMIN:
            make_cli_client_confidential(client, admin_portal)
        elif client_id == ADMIN_PORTAL:
            if admin_portal_url:
                url = admin_portal_url.rstrip("/")
                client["rootUrl"] = url
                client["baseUrl"] = url
                client["redirectUris"] = [f"{url}/*"]
            harden_public_browser_client(client)
        elif client_id in PUBLIC_BROWSER_CLIENT_IDS:
            harden_public_browser_client(client)
        if client != before:
            changed.append(client_id)
    return changed


def harden_template(path, admin_portal_url=None):
    with open(path, encoding="utf-8") as file:
        realm = json.load(file)
    changed = harden_clients(realm.get("clients", []), realm["realm"], admin_portal_url)
    with open(path, "w", encoding="utf-8") as file:
        file.write(json.dumps(realm, indent=2, ensure_ascii=False) + "\n")
    return changed


class KeycloakAdmin:
    def __init__(self, url, username, password):
        self.url = url.rstrip("/")
        self.username = username
        self.password = password

    def token(self):
        data = urllib.parse.urlencode(
            {
                "client_id": "admin-cli",
                "grant_type": "password",
                "username": self.username,
                "password": self.password,
            }
        ).encode()
        url = f"{self.url}/realms/master/protocol/openid-connect/token"
        with urllib.request.urlopen(urllib.request.Request(url, data=data)) as response:
            return json.load(response)["access_token"]

    def call(self, method, path, body=None):
        request = urllib.request.Request(
            f"{self.url}/admin/realms{path}",
            method=method,
            data=None if body is None else json.dumps(body).encode(),
            headers={
                "Authorization": f"Bearer {self.token()}",
                "Content-Type": "application/json",
            },
        )
        with urllib.request.urlopen(request) as response:
            content = response.read()
            return json.loads(content) if content else None


def harden_realm(admin, realm, apply, admin_portal_url=None):
    clients = admin.call("GET", f"/{realm}/clients")
    originals = {client["id"]: copy.deepcopy(client) for client in clients}
    changed = harden_clients(clients, realm, admin_portal_url)
    for client in clients:
        if client.get("clientId") not in changed:
            continue
        original = originals[client["id"]]
        print(f"{realm}: {client['clientId']}: {describe(original, client)}")
        if not apply:
            continue
        new_mappers = [m for m in client.get("protocolMappers", []) if "id" not in m]
        update = {k: v for k, v in client.items() if k != "protocolMappers"}
        admin.call("PUT", f"/{realm}/clients/{client['id']}", update)
        for mapper in new_mappers:
            admin.call(
                "POST", f"/{realm}/clients/{client['id']}/protocol-mappers/models", mapper
            )


def describe(before, after):
    keys = sorted(
        key
        for key in set(before) | set(after)
        if before.get(key) != after.get(key) and key != "secret"
    )
    if before.get("secret") != after.get("secret"):
        keys.append("secret (new)")
    return ", ".join(keys)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    template = sub.add_parser("template")
    template.add_argument("files", nargs="+")
    template.add_argument("--admin-portal-url")
    realm = sub.add_parser("realm")
    realm.add_argument("--apply", action="store_true")
    realm.add_argument("--admin-portal-url")
    target = realm.add_mutually_exclusive_group(required=True)
    target.add_argument("--realm", action="append")
    target.add_argument("--all-tenant-realms", action="store_true")
    args = parser.parse_args(argv)

    if args.command == "template":
        for path in args.files:
            print(f"{path}: {', '.join(harden_template(path, args.admin_portal_url)) or 'unchanged'}")
        return 0

    admin = KeycloakAdmin(
        os.environ["KEYCLOAK_URL"],
        os.environ["KEYCLOAK_ADMIN"],
        os.environ["KEYCLOAK_ADMIN_PASSWORD"],
    )
    realms = args.realm or [
        realm["realm"]
        for realm in admin.call("GET", "")
        if realm["realm"].startswith("tenant-")
    ]
    for name in realms:
        harden_realm(admin, name, args.apply, args.admin_portal_url)
    if not args.apply:
        print("Dry run: nothing was changed. Re-run with --apply to update the realms.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
