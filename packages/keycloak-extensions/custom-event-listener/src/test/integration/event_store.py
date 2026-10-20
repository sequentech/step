# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Disposable Keycloak event/session storage and authentication checks with synthetic data.

Run from any directory after packaging the Java reactor. This owns only uniquely named containers
and its own network. HTTP checks run inside that network, with no host port or shared service access.
"""

from __future__ import annotations

import argparse
import base64
import html
import json
import re
import subprocess
import sys
import tempfile
import time
import uuid
from http.cookiejar import CookieJar
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode, urlparse, parse_qs
from urllib.request import (
    Request,
    urlopen,
    build_opener,
    HTTPCookieProcessor,
    HTTPRedirectHandler,
)

TENANT = "11111111-1111-4111-8111-111111111111"
EVENTS = {
    "22222222-2222-4222-8222-222222222222": "do-not-store",
    "33333333-3333-4333-8333-333333333333": "forensic-store",
}
PASSWORD = "synthetic-test-password"


def voter_id(event_id):
    return str(uuid.uuid5(uuid.NAMESPACE_URL, event_id))


def request(url, *, method="GET", body=None, token=None, form=None, headers=None):
    headers = dict(headers or {})
    data = None
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    if form is not None:
        data = urlencode(form).encode()
        headers["Content-Type"] = "application/x-www-form-urlencoded"
    if token:
        headers["Authorization"] = "Bearer " + token
    try:
        query = Request(url, data=data, method=method, headers=headers)
        with urlopen(query, timeout=10) as answer:
            raw = answer.read()
            return answer.status, json.loads(raw) if raw else None
    except HTTPError as error:
        return error.code, error.read().decode()


def browser_sessions(base, realm):
    """Exercise code login, cookie SSO and forced reauthentication against real Keycloak."""

    class NoRedirect(HTTPRedirectHandler):
        def redirect_request(self, request, response, code, message, headers, url):
            return None

    opener = build_opener(HTTPCookieProcessor(CookieJar()), NoRedirect())
    redirect = "http://forensic:8081/callback"
    tokens = None
    for prompt in ("login", "none", "login"):
        parameters = {
            "client_id": "fixture",
            "response_type": "code",
            "scope": "openid",
            "redirect_uri": redirect,
            "prompt": prompt,
        }
        if tokens is not None and prompt == "login":
            parameters["max_age"] = "0"
        headers = {
            "X-Forwarded-For": "198.51.100.7",
            "User-Agent": "private-browser-sentinel",
            "CF-IPCountry": "PH",
        }
        try:
            response = opener.open(
                Request(
                    base
                    + f"/realms/{realm}/protocol/openid-connect/auth?"
                    + urlencode(parameters),
                    headers=headers,
                ),
                timeout=10,
            )
        except HTTPError as answer:
            response = answer
        if prompt == "login":
            assert response.status == 200, response.status
            page = response.read().decode()
            form = next(
                tag
                for tag in re.findall(r"<form\b[^>]*>", page)
                if 'id="kc-form-login"' in tag
            )
            action = html.unescape(re.search(r'action="([^"]+)"', form).group(1))
            try:
                response = opener.open(
                    Request(
                        action,
                        data=urlencode(
                            {
                                "username": "voter",
                                "password": PASSWORD,
                                "credentialId": "",
                            }
                        ).encode(),
                        headers=headers,
                    ),
                    timeout=10,
                )
            except HTTPError as answer:
                response = answer
        assert response.status == 302, response.status
        code = parse_qs(urlparse(response.headers["Location"]).query)["code"][0]
        status, tokens = request(
            base + f"/realms/{realm}/protocol/openid-connect/token",
            method="POST",
            form={
                "client_id": "fixture",
                "grant_type": "authorization_code",
                "code": code,
                "redirect_uri": redirect,
            },
        )
        assert status == 200, (status, tokens)
    return tokens["refresh_token"]


def inside(restart=False):
    base = "http://keycloak:8080"
    for _ in range(150):
        try:
            if (
                request(base + "/realms/master/.well-known/openid-configuration")[0]
                == 200
            ):
                break
        except (URLError, TimeoutError):
            pass
        time.sleep(1)
    else:
        raise AssertionError("Keycloak did not become ready")
    status, tokens = request(
        base + "/realms/master/protocol/openid-connect/token",
        method="POST",
        form={
            "client_id": "admin-cli",
            "grant_type": "password",
            "username": "admin",
            "password": PASSWORD,
        },
    )
    assert status == 200, (status, tokens)
    token = tokens["access_token"]
    status, info = request(base + "/admin/serverinfo", token=token)
    assert status == 200
    assert "sequent-privacy" in info["providers"]["eventsStore"]["providers"]
    if restart:
        saved = json.loads(Path("/tmp/session_refresh_tokens.json").read_text())
        for event_id, refresh_tokens in saved.items():
            realm = f"tenant-{TENANT}-event-{event_id}"
            status, sessions = request(
                base + f"/admin/realms/{realm}/users/{voter_id(event_id)}/sessions",
                token=token,
            )
            assert status == 200 and sessions, (status, sessions)
            assert all(not row.get("ipAddress") for row in sessions), sessions
            for refresh in refresh_tokens:
                status, answer = request(
                    base + f"/realms/{realm}/protocol/openid-connect/token",
                    method="POST",
                    form={
                        "client_id": "fixture",
                        "grant_type": "refresh_token",
                        "refresh_token": refresh,
                    },
                )
                assert status == 200, (status, answer)
                status, result = request(
                    base + f"/realms/{realm}/protocol/openid-connect/logout",
                    method="POST",
                    form={
                        "client_id": "fixture",
                        "refresh_token": answer["refresh_token"],
                    },
                )
                assert status == 204, (status, result)
                status, _ = request(
                    base + f"/realms/{realm}/protocol/openid-connect/token",
                    method="POST",
                    form={
                        "client_id": "fixture",
                        "grant_type": "refresh_token",
                        "refresh_token": answer["refresh_token"],
                    },
                )
                assert status == 400, status
        print(
            "PASS: persisted online/offline sessions refresh after restart and logout revokes them"
        )
        return
    refresh_tokens = {}
    expected_forensic = 0
    request("http://forensic:8081/policies", method="POST", body=EVENTS)
    for event_id, policy in EVENTS.items():
        realm = f"tenant-{TENANT}-event-{event_id}"
        voter = voter_id(event_id)
        status, settings = request(base + "/admin/realms/" + realm, token=token)
        assert status == 200
        assert settings["eventsEnabled"] is True
        assert settings["adminEventsEnabled"] is True
        assert settings["adminEventsDetailsEnabled"] is True
        assert settings["eventsExpiration"] == 3600
        assert settings["eventsListeners"] == ["custom-event-listener"]
        for password, expected in ((PASSWORD, 200), ("wrong-password", 400)):
            status, answer = request(
                base + f"/realms/{realm}/protocol/openid-connect/token",
                method="POST",
                form={
                    "client_id": "fixture",
                    "grant_type": "password",
                    "username": "voter",
                    "password": password,
                },
                headers={
                    "User-Agent": "private-browser-sentinel",
                    "CF-IPCountry": "PH",
                    "X-Forwarded-For": "198.51.100.7",
                },
            )
            assert status == expected, (status, answer)
            if expected == 200:
                normal = answer
        status, sessions = request(
            base + f"/admin/realms/{realm}/users/{voter}/sessions", token=token
        )
        assert status == 200 and len(sessions) == 1, (status, sessions)
        assert all(not session.get("ipAddress") for session in sessions), sessions
        refresh_tokens[event_id] = []
        for scope in (None, "offline_access"):
            if scope:
                status, normal = request(
                    base + f"/realms/{realm}/protocol/openid-connect/token",
                    method="POST",
                    form={
                        "client_id": "fixture",
                        "grant_type": "password",
                        "username": "voter",
                        "password": PASSWORD,
                        "scope": scope,
                    },
                    headers={"X-Forwarded-For": "198.51.100.7"},
                )
                assert status == 200, (status, normal)
            status, refreshed = request(
                base + f"/realms/{realm}/protocol/openid-connect/token",
                method="POST",
                form={
                    "client_id": "fixture",
                    "grant_type": "refresh_token",
                    "refresh_token": normal["refresh_token"],
                },
            )
            assert status == 200, (status, refreshed)
            refresh_tokens[event_id].append(refreshed["refresh_token"])
        status, clients = request(
            base + f"/admin/realms/{realm}/clients?clientId=fixture", token=token
        )
        assert status == 200 and len(clients) == 1
        status, offline = request(
            base
            + f"/admin/realms/{realm}/users/{voter}/offline-sessions/{clients[0]['id']}",
            token=token,
        )
        assert status == 200 and len(offline) == 1, (status, offline)
        assert all(not row.get("ipAddress") for row in offline), offline
        refresh_tokens[event_id].append(browser_sessions(base, realm))
        status, sessions = request(
            base + f"/admin/realms/{realm}/users/{voter}/sessions", token=token
        )
        assert status == 200 and sessions
        assert all(not row.get("ipAddress") for row in sessions), sessions
        # A real admin update must remain auditable, including normal representation fields.
        status, answer = request(
            base + f"/admin/realms/{realm}/users/{voter}",
            token=token,
            method="PUT",
            body={
                "firstName": "Audited change",
                "attributes": {
                    "user_agent": ["private-browser-sentinel"],
                    "ip": ["198.51.100.8"],
                    "area": ["north"],
                },
            },
            headers={"X-Forwarded-For": "198.51.100.9"},
        )
        assert status == 204, (status, answer)
        status, events = request(base + f"/admin/realms/{realm}/events", token=token)
        assert status == 200
        for kind in ("LOGIN", "LOGIN_ERROR"):
            rows = [row for row in events if row["type"] == kind]
            assert rows, (policy, kind, events)
            assert all(not row.get("ipAddress") for row in rows), rows
            assert all(row["userId"] == voter for row in rows), rows
            if policy == "forensic-store" and kind == "LOGIN":
                expected_forensic = len(rows)
        status, admins = request(
            base + f"/admin/realms/{realm}/admin-events", token=token
        )
        assert status == 200
        updates = [
            row
            for row in admins
            if row["operationType"] == "UPDATE"
            and row["resourcePath"] == f"users/{voter}"
        ]
        assert len(updates) == 1, admins
        saved = updates[0]
        assert not saved["authDetails"].get("ipAddress"), saved
        representation = json.loads(saved["representation"])
        assert representation["firstName"] == "Audited change", representation
        assert representation["attributes"]["area"] == ["north"], representation
        assert "private-browser" not in saved["representation"]
        assert "198.51.100" not in saved["representation"]
    for _ in range(50):
        captured = request("http://forensic:8081/captured")[1]
        if len(captured) == expected_forensic:
            break
        time.sleep(0.1)
    assert len(captured) == expected_forensic and expected_forensic >= 2, captured
    assert captured[0]["election_event_id"] == list(EVENTS)[1], captured
    assert all(row["ip"] == "198.51.100.7" for row in captured), captured
    assert any(
        row["country"] == "PH" and row["user_agent"] == "private-browser-sentinel"
        for row in captured
    ), captured
    assert all(
        row["election_event_id"] == list(EVENTS)[1] for row in captured
    ), captured
    Path("/tmp/session_refresh_tokens.json").write_text(json.dumps(refresh_tokens))
    status, admins = request(
        base + "/admin/realms/master/users?username=admin", token=token
    )
    assert status == 200 and len(admins) == 1
    status, admin_sessions = request(
        base + f"/admin/realms/master/users/{admins[0]['id']}/sessions", token=token
    )
    assert status == 200 and any(
        row.get("ipAddress") for row in admin_sessions
    ), admin_sessions
    print(
        "PASS: actual LOGIN/LOGIN_ERROR and admin persistence under both policies; "
        "forensic original retained"
    )


def run(args):
    here = Path(__file__).resolve().parent
    extension = here.parents[3]
    prefix = "step-event-store-" + uuid.uuid4().hex[:10]
    names = {
        name: prefix + "-" + name
        for name in ("db", "keycloak", "forensic", "wrong", "wrong-session")
    }
    created = []

    def docker(*command, input=None, check=True):
        result = subprocess.run(
            ["docker", *command],
            text=True,
            input=input,
            capture_output=True,
            check=False,
        )
        if check and result.returncode:
            raise RuntimeError(
                f"docker {command[0]} failed: {result.stderr}\n{result.stdout}"
            )
        return result.stdout.strip()

    def sql(statement):
        return docker(
            "exec",
            "-i",
            names["db"],
            "psql",
            "-X",
            "-v",
            "ON_ERROR_STOP=1",
            "-U",
            "postgres",
            "-d",
            "keycloak",
            "-At",
            input=statement,
        )

    def create(name, *arguments):
        docker(
            "create",
            "--name",
            names[name],
            "--network",
            prefix,
            "--network-alias",
            name,
            *arguments,
        )
        created.append(names[name])

    def keycloak(name, provider, imported=False, session_provider="sequent-privacy"):
        environment = {
            "KC_DB": "postgres",
            "KC_DB_URL_HOST": "db",
            "KC_DB_URL_DATABASE": "keycloak",
            "KC_DB_USERNAME": "postgres",
            "KC_DB_PASSWORD": PASSWORD,
            "KC_BOOTSTRAP_ADMIN_USERNAME": "admin",
            "KC_BOOTSTRAP_ADMIN_PASSWORD": PASSWORD,
            "QUEUE_DB__HOST": "db",
            "QUEUE_DB__DBNAME": "keycloak",
            "QUEUE_DB__USER": "postgres",
            "QUEUE_DB__PASSWORD": PASSWORD,
            "ENV_SLUG": "privacy-fixture",
            "HARVEST_DOMAIN": "http://forensic:8081",
            "KEYCLOAK_URL": "http://forensic:8081",
            "KEYCLOAK_CLIENT_ID": "fixture-service",
            "KEYCLOAK_CLIENT_SECRET": "synthetic-secret",
        }
        env = [
            arg
            for key, value in environment.items()
            for arg in ("--env", f"{key}={value}")
        ]
        selection = (
            f"--spi-events-store--provider-default={provider}"
            if provider == "sequent-privacy"
            else f"--spi-events-store--provider={provider}"
        )
        command = [
            "start-dev",
            "--hostname-strict=false",
            "--proxy-headers=xforwarded",
            selection,
            (
                f"--spi-user-sessions--provider-default={session_provider}"
                if session_provider == "sequent-privacy"
                else f"--spi-user-sessions--provider={session_provider}"
            ),
        ]
        if imported:
            command.append("--import-realm")
        create(name, *env, args.image, *command)

    try:
        if not args.skip_build:
            print("Building pinned Keycloak fixture image", flush=True)
            docker(
                "build",
                "--file",
                str(here / "Dockerfile"),
                "--tag",
                args.image,
                str(extension),
            )
        docker("network", "create", "--internal", prefix)
        create(
            "db",
            "--env",
            f"POSTGRES_PASSWORD={PASSWORD}",
            "--env",
            "POSTGRES_DB=keycloak",
            "postgres:16-bookworm",
        )
        docker("start", names["db"])
        for _ in range(60):
            if "accepting connections" in docker(
                "exec",
                names["db"],
                "pg_isready",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
                check=False,
            ):
                break
            time.sleep(0.5)
        sql(
            """
            CREATE SCHEMA step_queue;
            CREATE TABLE step_queue.installation(environment text);
            INSERT INTO step_queue.installation VALUES ('privacy-fixture');
            CREATE SCHEMA pgmq;
            CREATE TABLE pgmq.captured(id bigserial, queue text, payload jsonb);
            CREATE FUNCTION pgmq.send(queue_name text, body jsonb) RETURNS bigint LANGUAGE sql AS $$
              INSERT INTO pgmq.captured(queue, payload) VALUES (queue_name, body) RETURNING id;
            $$;
        """
        )
        create(
            "forensic",
            "--entrypoint",
            "python3",
            args.mock_image,
            "/tmp/forensic_mock.py",
        )
        docker(
            "cp",
            str(here / "forensic_mock.py"),
            names["forensic"] + ":/tmp/forensic_mock.py",
        )
        docker(
            "cp",
            str(here / "event_store.py"),
            names["forensic"] + ":/tmp/event_store.py",
        )
        docker("start", names["forensic"])
        keycloak("keycloak", "sequent-privacy", imported=True)
        with tempfile.TemporaryDirectory(prefix=prefix) as directory:
            for event_id in EVENTS:
                realm = f"tenant-{TENANT}-event-{event_id}"
                representation = {
                    "realm": realm,
                    "enabled": True,
                    "sslRequired": "none",
                    "eventsEnabled": True,
                    "eventsListeners": ["custom-event-listener"],
                    "enabledEventTypes": ["LOGIN", "LOGIN_ERROR"],
                    "eventsExpiration": 3600,
                    "adminEventsEnabled": True,
                    "adminEventsDetailsEnabled": True,
                    "clients": [
                        {
                            "clientId": "fixture",
                            "publicClient": True,
                            "directAccessGrantsEnabled": True,
                            "optionalClientScopes": ["offline_access"],
                            "redirectUris": ["http://forensic:8081/callback"],
                            "standardFlowEnabled": True,
                        }
                    ],
                    "users": [
                        {
                            "id": voter_id(event_id),
                            "username": "voter",
                            "enabled": True,
                            "email": "voter@example.test",
                            "firstName": "Synthetic",
                            "lastName": "Voter",
                            "realmRoles": ["offline_access"],
                            "credentials": [{"type": "password", "value": PASSWORD}],
                        }
                    ],
                }
                path = Path(directory) / (realm + ".json")
                path.write_text(json.dumps(representation))
                docker(
                    "cp",
                    str(path),
                    names["keycloak"] + ":/opt/keycloak/data/import/" + path.name,
                )
        docker("start", names["keycloak"])
        for _ in range(150):
            if (
                docker("inspect", "--format", "{{.State.Running}}", names["keycloak"])
                != "true"
            ):
                raise AssertionError("Keycloak stopped before readiness")
            ready = docker(
                "exec",
                names["forensic"],
                "python3",
                "-c",
                "from urllib.request import urlopen; "
                "urlopen('http://keycloak:8080/realms/master/"
                ".well-known/openid-configuration', timeout=1); print('ready')",
                check=False,
            )
            if ready == "ready":
                break
            time.sleep(0.5)
        else:
            raise AssertionError("Keycloak did not become ready")
        print(
            docker(
                "exec", names["forensic"], "python3", "/tmp/event_store.py", "--inside"
            ),
            flush=True,
        )
        # Inspect stored rows directly, not just a redacting read API.
        rows = json.loads(
            sql(
                "SELECT coalesce(json_agg(e), '[]') FROM event_entity e WHERE realm_id != 'master';"
            )
        )
        assert len(rows) >= 6, rows
        assert {row["type"] for row in rows} == {"LOGIN", "LOGIN_ERROR"}
        assert all(not row["ip_address"] for row in rows), rows
        assert all(
            "private-browser" not in str(row) and "198.51.100" not in str(row)
            for row in rows
        ), rows
        audits = json.loads(
            sql(
                "SELECT coalesce(json_agg(e), '[]') FROM admin_event_entity e "
                "WHERE realm_id != 'master';"
            )
        )
        assert len(audits) == 2, audits
        assert all(not row["ip_address"] for row in audits), audits
        queue = json.loads(
            sql("SELECT coalesce(json_agg(payload), '[]') FROM pgmq.captured;")
        )
        decoded = [base64.b64decode(item["body"]).decode() for item in queue]
        assert len(decoded) >= 6, decoded
        assert all(
            "198.51.100" not in item and "private-browser" not in item
            for item in decoded
        ), decoded
        print(
            "PASS: raw JPA rows are redacted and custom authentication/admin audit queue "
            "still receives outcomes",
            flush=True,
        )
        docker("stop", names["keycloak"])
        retained = sql(
            "SELECT count(*) FROM offline_user_session "
            "WHERE data::text LIKE '%198.51.100.7%';"
        )
        assert retained == "0", (
            "Ordinary sessions retained the voter address: " + retained
        )
        client_retained = sql(
            "SELECT count(*) FROM offline_client_session WHERE data::text LIKE '%198.51.100.7%' OR data::text LIKE '%private-browser-sentinel%';"
        )
        assert client_retained == "0", (
            "Client sessions retained network data: " + client_retained
        )
        sql(
            "CREATE TABLE session_remediation_seed AS SELECT s.* FROM offline_user_session s JOIN realm r ON r.id=s.realm_id WHERE r.name LIKE 'tenant-%-event-%';"
        )
        docker("start", names["keycloak"])
        print(
            docker(
                "exec",
                names["forensic"],
                "python3",
                "/tmp/event_store.py",
                "--inside",
                "--restart",
            ),
            flush=True,
        )
        docker("stop", names["keycloak"])
        sql(
            "INSERT INTO offline_user_session SELECT * FROM session_remediation_seed ON CONFLICT DO NOTHING; "
            "UPDATE offline_user_session SET data=jsonb_set(data::jsonb,'{ipAddress}','\"198.51.100.7\"'::jsonb)::text WHERE realm_id IN (SELECT id FROM realm WHERE name LIKE 'tenant-%-event-%');"
        )
        first_event = next(iter(EVENTS))
        first_realm = f"tenant-{TENANT}-event-{first_event}"
        sql(
            "INSERT INTO offline_user_session SELECT (jsonb_populate_record(NULL::offline_user_session, "
            "to_jsonb(seed) || jsonb_build_object('user_session_id','bounded-purge-' || n))).* "
            "FROM (SELECT s.* FROM session_remediation_seed s JOIN realm r ON r.id=s.realm_id "
            f"WHERE r.name='{first_realm}' LIMIT 1) seed CROSS JOIN generate_series(1,5001) g(n);"
        )
        cleanup = (here.parents[2] / "scripts/purge_event_sessions.sql").read_text()
        master_before = sql(
            "SELECT count(*) FROM offline_user_session s JOIN realm r ON r.id=s.realm_id WHERE r.name='master';"
        )
        for invalid in (
            "",
            f"{chr(92)}set sessions_quiesced yes\n{chr(92)}set realm_name master\n",
            f"{chr(92)}set sessions_quiesced yes\n{chr(92)}set realm_name tenant-{TENANT}-event-{TENANT}\n",
        ):
            try:
                sql(invalid + cleanup)
            except RuntimeError:
                pass
            else:
                raise AssertionError(
                    "Remediation accepted an unacknowledged or non-event scope"
                )
        for event_id in EVENTS:
            realm = f"tenant-{TENANT}-event-{event_id}"
            invocation = (
                f"{chr(92)}set sessions_quiesced yes\n{chr(92)}set realm_name {realm}\n"
                + cleanup
            )
            batches = []
            for _ in range(5):
                result = sql(invocation)
                assert result.strip().endswith("COMMIT"), result
                counts = [int(line) for line in result.splitlines() if line.isdigit()]
                assert len(counts) == 1 and counts[0] <= 5000, result
                batches.extend(counts)
                if counts[0] == 0:
                    break
            assert batches[0] > 0 and batches[-1] == 0, batches
            if event_id == first_event:
                assert batches[0] == 5000 and sum(batches) >= 5001, batches
        remaining = sql(
            "SELECT count(*) FROM offline_user_session s JOIN realm r ON r.id=s.realm_id WHERE r.name LIKE 'tenant-%-event-%';"
        )
        assert remaining == "0", remaining
        assert master_before != "0"
        assert (
            sql(
                "SELECT count(*) FROM offline_user_session s JOIN realm r ON r.id=s.realm_id WHERE r.name='master';"
            )
            == master_before
        )
        print(
            "PASS: bounded quiesced event-session remediation is idempotent and preserves master sessions",
            flush=True,
        )
        keycloak("wrong", "jpa")
        docker("start", names["wrong"])
        for _ in range(120):
            if (
                docker("inspect", "--format", "{{.State.Running}}", names["wrong"])
                == "false"
            ):
                break
            time.sleep(0.5)
        else:
            raise AssertionError(
                "Wrong event-store selection was allowed to serve authentication"
            )
        logs = docker("logs", names["wrong"], check=False)
        assert "sequent-privacy event store must be selected" in logs, logs[-5000:]
        print(
            "PASS: explicit JPA bypass fails startup against the existing enabled realms",
            flush=True,
        )
        keycloak("wrong-session", "sequent-privacy", session_provider="infinispan")
        docker("start", names["wrong-session"])
        for _ in range(120):
            if (
                docker(
                    "inspect", "--format", "{{.State.Running}}", names["wrong-session"]
                )
                == "false"
            ):
                break
            time.sleep(0.5)
        else:
            raise AssertionError(
                "Unsafe session provider was allowed to serve authentication"
            )
        logs = docker("logs", names["wrong-session"], check=False)
        assert "sequent-privacy user session provider must be selected" in logs, logs[
            -5000:
        ]
        print(
            "PASS: exclusive session bypass fails startup even when the privacy factory is unloaded",
            flush=True,
        )
    except Exception:
        for name in created:
            if name in (names["keycloak"], names["wrong"], names["wrong-session"]):
                print(docker("logs", name, check=False)[-12000:], file=sys.stderr)
        raise
    finally:
        for name in reversed(created):
            docker("rm", "--force", "--volumes", name, check=False)
        docker("network", "rm", prefix, check=False)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--restart", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--image", default="step-privacy-event-store-test:local")
    parser.add_argument("--mock-image", default="python:3.12-slim")
    parser.add_argument("--skip-build", action="store_true")
    arguments = parser.parse_args()
    inside(arguments.restart) if arguments.inside else run(arguments)
