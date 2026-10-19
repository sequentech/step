# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Disposable real Keycloak/JPA storage checks, with synthetic queue and forensic boundaries.

Run from any directory after packaging the Java reactor. This owns only uniquely named containers
and its own network. HTTP checks run inside that network, with no host port or shared service access.
"""

from __future__ import annotations

import argparse
import base64
import json
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

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


def inside():
    base = "http://keycloak:8080"
    for _ in range(150):
        try:
            if request(base + "/realms/master/.well-known/openid-configuration")[0] == 200:
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
        status, admins = request(base + f"/admin/realms/{realm}/admin-events", token=token)
        assert status == 200
        updates = [
            row for row in admins
            if row["operationType"] == "UPDATE" and row["resourcePath"] == f"users/{voter}"
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
        if captured:
            break
        time.sleep(0.1)
    assert len(captured) == 1, captured
    assert captured[0]["election_event_id"] == list(EVENTS)[1], captured
    assert captured[0]["ip"] == "198.51.100.7", captured
    assert captured[0]["country"] == "PH", captured
    assert captured[0]["user_agent"] == "private-browser-sentinel", captured
    print(
        "PASS: actual LOGIN/LOGIN_ERROR and admin persistence under both policies; "
        "forensic original retained"
    )


def run(args):
    here = Path(__file__).resolve().parent
    extension = here.parents[3]
    prefix = "step-event-store-" + uuid.uuid4().hex[:10]
    names = {name: prefix + "-" + name for name in ("db", "keycloak", "forensic", "wrong")}
    created = []

    def docker(*command, input=None, check=True):
        result = subprocess.run(
            ["docker", *command], text=True, input=input, capture_output=True, check=False
        )
        if check and result.returncode:
            raise RuntimeError(f"docker {command[0]} failed: {result.stderr}\n{result.stdout}")
        return result.stdout.strip()

    def sql(statement):
        return docker(
            "exec", "-i", names["db"], "psql", "-X", "-v", "ON_ERROR_STOP=1",
            "-U", "postgres", "-d", "keycloak", "-At", input=statement,
        )

    def create(name, *arguments):
        docker(
            "create", "--name", names[name], "--network", prefix,
            "--network-alias", name, *arguments,
        )
        created.append(names[name])

    def keycloak(name, provider, imported=False):
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
        env = [arg for key, value in environment.items() for arg in ("--env", f"{key}={value}")]
        selection = (
            f"--spi-events-store--provider-default={provider}"
            if provider == "sequent-privacy" else f"--spi-events-store--provider={provider}"
        )
        command = [
            "start-dev", "--hostname-strict=false", "--proxy-headers=xforwarded", selection
        ]
        if imported:
            command.append("--import-realm")
        create(name, *env, args.image, *command)

    try:
        if not args.skip_build:
            print("Building pinned Keycloak fixture image", flush=True)
            docker("build", "--file", str(here / "Dockerfile"), "--tag", args.image, str(extension))
        docker("network", "create", "--internal", prefix)
        create(
            "db", "--env", f"POSTGRES_PASSWORD={PASSWORD}",
            "--env", "POSTGRES_DB=keycloak", "postgres:16-bookworm",
        )
        docker("start", names["db"])
        for _ in range(60):
            if "accepting connections" in docker(
                "exec", names["db"], "pg_isready", "-h", "127.0.0.1",
                "-U", "postgres", check=False,
            ):
                break
            time.sleep(0.5)
        sql("""
            CREATE SCHEMA step_queue;
            CREATE TABLE step_queue.installation(environment text);
            INSERT INTO step_queue.installation VALUES ('privacy-fixture');
            CREATE SCHEMA pgmq;
            CREATE TABLE pgmq.captured(id bigserial, queue text, payload jsonb);
            CREATE FUNCTION pgmq.send(queue_name text, body jsonb) RETURNS bigint LANGUAGE sql AS $$
              INSERT INTO pgmq.captured(queue, payload) VALUES (queue_name, body) RETURNING id;
            $$;
        """)
        create("forensic", "--entrypoint", "python3", args.mock_image, "/tmp/forensic_mock.py")
        docker("cp", str(here / "forensic_mock.py"), names["forensic"] + ":/tmp/forensic_mock.py")
        docker("cp", str(here / "event_store.py"), names["forensic"] + ":/tmp/event_store.py")
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
                    "clients": [{
                        "clientId": "fixture",
                        "publicClient": True,
                        "directAccessGrantsEnabled": True,
                    }],
                    "users": [{
                        "id": voter_id(event_id),
                        "username": "voter",
                        "enabled": True,
                        "email": "voter@example.test",
                        "firstName": "Synthetic",
                        "lastName": "Voter",
                        "credentials": [{"type": "password", "value": PASSWORD}],
                    }],
                }
                path = Path(directory) / (realm + ".json")
                path.write_text(json.dumps(representation))
                docker("cp", str(path), names["keycloak"] + ":/opt/keycloak/data/import/" + path.name)
        docker("start", names["keycloak"])
        for _ in range(150):
            if docker("inspect", "--format", "{{.State.Running}}", names["keycloak"]) != "true":
                raise AssertionError("Keycloak stopped before readiness")
            ready = docker(
                "exec", names["forensic"], "python3", "-c",
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
        print(docker("exec", names["forensic"], "python3", "/tmp/event_store.py", "--inside"), flush=True)
        # Inspect stored rows directly, not just a redacting read API.
        rows = json.loads(sql(
            "SELECT coalesce(json_agg(e), '[]') FROM event_entity e WHERE realm_id != 'master';"
        ))
        assert len(rows) == 4, rows
        assert {row["type"] for row in rows} == {"LOGIN", "LOGIN_ERROR"}
        assert all(not row["ip_address"] for row in rows), rows
        assert all("private-browser" not in str(row) and "198.51.100" not in str(row) for row in rows), rows
        audits = json.loads(sql(
            "SELECT coalesce(json_agg(e), '[]') FROM admin_event_entity e "
            "WHERE realm_id != 'master';"
        ))
        assert len(audits) == 2, audits
        assert all(not row["ip_address"] for row in audits), audits
        queue = json.loads(sql("SELECT coalesce(json_agg(payload), '[]') FROM pgmq.captured;"))
        decoded = [base64.b64decode(item["body"]).decode() for item in queue]
        assert len(decoded) >= 6, decoded
        assert all("198.51.100" not in item and "private-browser" not in item for item in decoded), decoded
        print(
            "PASS: raw JPA rows are redacted and custom authentication/admin audit queue "
            "still receives outcomes", flush=True,
        )
        docker("stop", names["keycloak"])
        keycloak("wrong", "jpa")
        docker("start", names["wrong"])
        for _ in range(120):
            if docker("inspect", "--format", "{{.State.Running}}", names["wrong"]) == "false":
                break
            time.sleep(0.5)
        else:
            raise AssertionError("Wrong event-store selection was allowed to serve authentication")
        logs = docker("logs", names["wrong"], check=False)
        assert "sequent-privacy event store must be selected" in logs, logs[-5000:]
        print("PASS: explicit JPA bypass fails startup against the existing enabled realms", flush=True)
    except Exception:
        for name in created:
            if name in (names["keycloak"], names["wrong"]):
                print(docker("logs", name, check=False)[-12000:], file=sys.stderr)
        raise
    finally:
        for name in reversed(created):
            docker("rm", "--force", "--volumes", name, check=False)
        docker("network", "rm", prefix, check=False)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--image", default="step-privacy-event-store-test:local")
    parser.add_argument("--mock-image", default="python:3.12-slim")
    parser.add_argument("--skip-build", action="store_true")
    arguments = parser.parse_args()
    inside() if arguments.inside else run(arguments)
