#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Apply the public client hardening of `upsert_realm` to realms that exist.

New realms get it from `upsert_realm` (sequent-core `realm.rs`); this script
brings realms created before that change, and realm template files, to the
same settings:

- voting-portal, onsite-voting-portal, voting-portal-kiosk, results-portal and
  admin-portal: no direct access (password) or implicit grant, PKCE S256, web
  origins limited to the client's redirect URIs, no request URIs. Post-logout
  redirects are limited the same way, except on the voting, onsite and kiosk
  portals, whose Finish button logs out to the event's finish URL.
- admin-portal: redirect URIs pinned to --admin-portal-url, when given.
- account-console: redirects only to its realm's account pages.
- cli-account-admin: confidential (password grant plus client secret), no
  browser logins, no request URIs, and the admin-portal token claims it lacks.
  Keycloak keeps no secret for a public client, so the script sets it to
  KEYCLOAK_CLI_CLIENT_SECRET when that is set, and generates one otherwise. A
  stored secret is kept.

A realm is left unchanged when one of those clients would keep the redirect URI
`*`: its web origins `+` would then allow no origin and break browser logins.
Pin the client's redirect URIs first (--admin-portal-url for admin-portal).

Usage:
    harden_public_clients.py template FILE...        rewrite realm JSON files
    harden_public_clients.py realm [--apply] (--realm NAME... | --all-tenant-realms)
        Reads KEYCLOAK_URL, KEYCLOAK_ADMIN and KEYCLOAK_ADMIN_PASSWORD (master
        realm admin), and KEYCLOAK_CLI_CLIENT_SECRET if set. Without --apply it
        only prints what it would change.
"""

import argparse
import copy
import json
import os
import secrets
import string
import sys
import urllib.error
import urllib.parse
import urllib.request

PUBLIC_BROWSER_CLIENT_IDS = (
    "voting-portal",
    "onsite-voting-portal",
    "voting-portal-kiosk",
    "results-portal",
    "admin-portal",
)
# Their Finish button logs out to the event's finish URL, which can be outside
# their redirect URIs, so their post-logout redirect URIs are left as they are.
FINISH_URL_CLIENT_IDS = ("voting-portal", "onsite-voting-portal", "voting-portal-kiosk")
ADMIN_PORTAL = "admin-portal"
ACCOUNT_CONSOLE = "account-console"
CLI_ACCOUNT_ADMIN = "cli-account-admin"
HASURA_DEFAULT_ROLE_MAPPER = "x-hasura-default-role"
ANY_REDIRECT_URI = "*"
REQUEST_URIS_ATTR = "request.uris"


class UnpinnedRedirectError(Exception):
    """A hardened client still accepts any redirect URI."""


def generate_client_secret():
    alphabet = string.ascii_letters + string.digits
    return "".join(secrets.choice(alphabet) for _ in range(32))


def harden_public_browser_client(client):
    client["directAccessGrantsEnabled"] = False
    client["implicitFlowEnabled"] = False
    client["webOrigins"] = ["+"]
    attributes = client.setdefault("attributes", {})
    attributes["pkce.code.challenge.method"] = "S256"
    if client.get("clientId") not in FINISH_URL_CLIENT_IDS:
        attributes["post.logout.redirect.uris"] = "+"
    attributes.pop(REQUEST_URIS_ATTR, None)


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
    client.get("attributes", {}).pop(REQUEST_URIS_ATTR, None)
    if admin_portal is not None:
        mappers = client.setdefault("protocolMappers", [])
        by_name = {mapper.get("name"): mapper for mapper in mappers}
        for mapper in admin_portal.get("protocolMappers", []):
            name = mapper.get("name")
            if name not in by_name:
                mapper = copy.deepcopy(mapper)
                mapper.pop("id", None)
                mappers.append(mapper)
            elif name == HASURA_DEFAULT_ROLE_MAPPER:
                by_name[name]["config"] = copy.deepcopy(mapper.get("config"))


def harden_clients(clients, realm, admin_portal_url=None):
    """Harden `clients` (realm representation dicts) in place, as `upsert_realm`
    does. Returns the clientIds that changed. Raises UnpinnedRedirectError,
    before changing anything, when a hardened client would keep the redirect
    URI `*`."""
    for client in clients:
        client_id = client.get("clientId")
        pinned_here = client_id == ACCOUNT_CONSOLE or (
            client_id == ADMIN_PORTAL and admin_portal_url
        )
        if (
            client_id in PUBLIC_BROWSER_CLIENT_IDS
            and not pinned_here
            and ANY_REDIRECT_URI in client.get("redirectUris", [])
        ):
            hint = " with --admin-portal-url" if client_id == ADMIN_PORTAL else ""
            raise UnpinnedRedirectError(
                f"{realm}: {client_id} accepts any redirect URI; pin its "
                f"redirect URIs{hint} first, so that web origins '+' allow its portal"
            )
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


def mapper_changes(before, after):
    """The mappers of `after` to create and the existing ones to update."""
    existing = {mapper["id"]: mapper for mapper in before.get("protocolMappers", [])}
    mappers = after.get("protocolMappers", [])
    new = [mapper for mapper in mappers if "id" not in mapper]
    changed = [
        mapper
        for mapper in mappers
        if "id" in mapper and mapper != existing.get(mapper["id"])
    ]
    return new, changed


def stored_secret(admin, realm, client):
    """The client's stored secret, which the clients list leaves out."""
    try:
        stored = admin.call("GET", f"/{realm}/clients/{client['id']}/client-secret")
    except urllib.error.HTTPError:
        return ""
    return (stored or {}).get("value") or ""


def update_payload(before, after):
    """The PUT body that turns `before` into `after`. Keycloak keeps attributes
    an update leaves out, so removed ones are sent empty, and an unchanged
    secret isn't sent again."""
    update = {k: v for k, v in after.items() if k != "protocolMappers"}
    if update.get("secret") == before.get("secret"):
        update.pop("secret", None)
    removed = set(before.get("attributes", {})) - set(after.get("attributes", {}))
    if removed:
        update["attributes"] = {
            **after.get("attributes", {}),
            **{key: "" for key in sorted(removed)},
        }
    return update


def harden_realm(admin, realm, apply, admin_portal_url=None, cli_secret=""):
    clients = admin.call("GET", f"/{realm}/clients")
    for client in clients:
        if client.get("clientId") == CLI_ACCOUNT_ADMIN:
            client["secret"] = stored_secret(admin, realm, client)
    originals = {client["id"]: copy.deepcopy(client) for client in clients}
    for client in clients:
        if client.get("clientId") == CLI_ACCOUNT_ADMIN and not client["secret"]:
            client["secret"] = cli_secret
    changed = harden_clients(clients, realm, admin_portal_url)
    for client in clients:
        if client.get("clientId") not in changed:
            continue
        original = originals[client["id"]]
        print(f"{realm}: {client['clientId']}: {describe(original, client)}")
        if not apply:
            continue
        new_mappers, changed_mappers = mapper_changes(original, client)
        admin.call(
            "PUT", f"/{realm}/clients/{client['id']}", update_payload(original, client)
        )
        models = f"/{realm}/clients/{client['id']}/protocol-mappers/models"
        for mapper in new_mappers:
            admin.call("POST", models, mapper)
        for mapper in changed_mappers:
            admin.call("PUT", f"{models}/{mapper['id']}", mapper)


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

    failed = False
    if args.command == "template":
        for path in args.files:
            try:
                changed = harden_template(path, args.admin_portal_url)
            except UnpinnedRedirectError as error:
                print(f"{path}: not changed: {error}", file=sys.stderr)
                failed = True
                continue
            print(f"{path}: {', '.join(changed) or 'unchanged'}")
        return 1 if failed else 0

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
        try:
            harden_realm(
                admin,
                name,
                args.apply,
                args.admin_portal_url,
                os.environ.get("KEYCLOAK_CLI_CLIENT_SECRET", ""),
            )
        except UnpinnedRedirectError as error:
            print(f"not changed: {error}", file=sys.stderr)
            failed = True
    if not args.apply:
        print("Dry run: nothing was changed. Re-run with --apply to update the realms.")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
