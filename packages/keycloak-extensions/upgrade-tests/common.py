# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Shared settings and helpers for the Keycloak upgrade tests (standard library only).

Every setting can be overridden from the environment; the defaults match the dev container
(.devcontainer/.env). See README.md.
"""

import json
import os
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Optional

TENANT_ID = os.environ.get("TENANT_ID", "90505c8a-23a9-4cdf-a26b-4e19f6a097d5")
KEYCLOAK_URL = os.environ.get("KEYCLOAK_URL", "http://keycloak:8090")
KEYCLOAK_BROWSER_URL = os.environ.get("KEYCLOAK_BROWSER_URL", "http://localhost:8090")
KEYCLOAK_MTLS_URL = os.environ.get("KEYCLOAK_MTLS_URL", "https://127.0.0.1:8443")
# Where KEYCLOAK_MTLS_URL is served from inside the dev container.
KEYCLOAK_NGINX_HOST = os.environ.get("KEYCLOAK_NGINX_HOST", "keycloak-nginx:8443")
# How Keycloak reaches itself, for the token and JWKS calls of IdPs that broker to another realm.
KEYCLOAK_SELF_URL = os.environ.get("KEYCLOAK_SELF_URL", "http://127.0.0.1:8090")
KEYCLOAK_ADMIN = os.environ.get("KEYCLOAK_ADMIN", "admin")
KEYCLOAK_ADMIN_PASSWORD = os.environ.get("KEYCLOAK_ADMIN_PASSWORD", "admin")
ADMIN_PORTAL_TEST_USERNAME = os.environ.get("ADMIN_PORTAL_TEST_USERNAME", "admin")
ADMIN_PORTAL_TEST_PASSWORD = os.environ.get("ADMIN_PORTAL_TEST_PASSWORD", "admin")
HASURA_ENDPOINT = os.environ.get(
    "HASURA_ENDPOINT", "http://graphql-engine:8080/v1/graphql"
)
HASURA_GRAPHQL_ADMIN_SECRET = os.environ.get("HASURA_GRAPHQL_ADMIN_SECRET", "admin")
HARVEST_DOMAIN = os.environ.get("HARVEST_DOMAIN", "harvest:8400")
VOTING_PORTAL_URL = os.environ.get("VOTING_PORTAL_URL", "http://localhost:3000")
ADMIN_PORTAL_URL = os.environ.get("ADMIN_PORTAL_URL", "http://localhost:3002")
OUT = Path(
    os.environ.get(
        "KEYCLOAK_UPGRADE_TESTS_OUT",
        os.path.join(tempfile.gettempdir(), "keycloak-upgrade-tests"),
    )
)

TENANT_REALM = f"tenant-{TENANT_ID}"
STATE_FILE = OUT / "state.json"
TASK_FINAL_STATUSES = {"SUCCESS", "FAILED", "ERROR", "CANCELLED"}

OUT.mkdir(parents=True, exist_ok=True)


def event_realm(event_id: str) -> str:
    return f"tenant-{TENANT_ID}-event-{event_id}"


# --- state shared between the scripts of one run


def load_state() -> dict:
    return json.loads(STATE_FILE.read_text()) if STATE_FILE.exists() else {}


def save_state(**values: Any) -> None:
    state = load_state()
    state.update(values)
    STATE_FILE.write_text(json.dumps(state, indent=2))


def require_state(*keys: str) -> dict:
    state = load_state()
    missing = [key for key in keys if key not in state]
    if missing:
        sys.exit(
            f"{STATE_FILE} lacks {', '.join(missing)}: run the setup scripts first"
        )
    return state


# --- HTTP


@dataclass
class Response:
    status: int
    body: str
    headers: dict

    def json(self) -> Any:
        return json.loads(self.body) if self.body else None


def http(
    method: str,
    url: str,
    *,
    json_body: Any = None,
    form: Optional[dict] = None,
    headers: Optional[dict] = None,
) -> Response:
    data = None
    request_headers = dict(headers or {})
    if json_body is not None:
        data = json.dumps(json_body).encode()
        request_headers["content-type"] = "application/json"
    elif form is not None:
        data = urllib.parse.urlencode(form).encode()
    request = urllib.request.Request(
        url, data=data, method=method, headers=request_headers
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return Response(
                response.status, response.read().decode(), dict(response.headers)
            )
    except urllib.error.HTTPError as error:
        return Response(error.code, error.read().decode(), dict(error.headers))


_TOKEN_REUSE_SECONDS = 30
_tokens: dict = {}


def _token(realm: str, client_id: str, username: str, password: str) -> str:
    cached = _tokens.get((realm, username))
    if cached and cached[1] > time.monotonic():
        return cached[0]
    response = http(
        "POST",
        f"{KEYCLOAK_URL}/realms/{realm}/protocol/openid-connect/token",
        form={
            "grant_type": "password",
            "client_id": client_id,
            "username": username,
            "password": password,
            "scope": "openid",
        },
    )
    if response.status != 200:
        sys.exit(f"token request for {username}@{realm} failed: HTTP {response.status}")
    token = response.json()["access_token"]
    _tokens[(realm, username)] = (token, time.monotonic() + _TOKEN_REUSE_SECONDS)
    return token


def master_token() -> str:
    return _token("master", "admin-cli", KEYCLOAK_ADMIN, KEYCLOAK_ADMIN_PASSWORD)


def admin_token() -> str:
    """The tenant admin user, as the admin portal logs in."""
    return _token(
        TENANT_REALM,
        "admin-portal",
        ADMIN_PORTAL_TEST_USERNAME,
        ADMIN_PORTAL_TEST_PASSWORD,
    )


def keycloak(method: str, path: str, body: Any = None) -> Response:
    """Keycloak admin REST API (path under /admin/realms) as the master admin."""
    return http(
        method,
        f"{KEYCLOAK_URL}/admin/realms{path}",
        json_body=body,
        headers={"Authorization": f"Bearer {master_token()}"},
    )


def graphql(query: str, variables: Optional[dict] = None) -> dict:
    """Hasura as the tenant admin user, so actions go through harvest (and the Rust Keycloak admin
    client) exactly as the admin portal does."""
    return http(
        "POST",
        HASURA_ENDPOINT,
        json_body={"query": query, "variables": variables or {}},
        headers={"Authorization": f"Bearer {admin_token()}"},
    ).json()


def graphql_admin(query: str, variables: Optional[dict] = None) -> dict:
    """Hasura with the admin secret (direct table access)."""
    return http(
        "POST",
        HASURA_ENDPOINT,
        json_body={"query": query, "variables": variables or {}},
        headers={"x-hasura-admin-secret": HASURA_GRAPHQL_ADMIN_SECRET},
    ).json()


def user_id(realm: str, username: str) -> Optional[str]:
    users = keycloak(
        "GET", f"/{realm}/users?username={urllib.parse.quote(username)}&exact=true"
    ).json()
    return users[0]["id"] if users else None


def wait_task(task_id: Optional[str], timeout_seconds: int = 300) -> Optional[str]:
    """Waits for a windmill task and returns its final status."""
    if not task_id:
        return None
    status = None
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        result = graphql_admin(
            "query($id:uuid!,$t:uuid!){sequent_backend_tasks_execution_by_pk(id:$id,tenant_id:$t){execution_status}}",
            {"id": task_id, "t": TENANT_ID},
        )
        task = (result.get("data") or {}).get(
            "sequent_backend_tasks_execution_by_pk"
        ) or {}
        status = task.get("execution_status")
        if status in TASK_FINAL_STATUSES:
            break
        time.sleep(2)
    return status


# --- results


def data(result: dict, field: str) -> Any:
    """A GraphQL result field, or None when the request returned errors."""
    if result.get("errors"):
        return None
    return (result.get("data") or {}).get(field)


def errors(result: dict) -> str:
    return json.dumps(result.get("errors") or result)[:400]


class Checks:
    def __init__(self) -> None:
        self.failures = 0

    def check(self, label: str, ok: bool, detail: Any = "") -> bool:
        if not ok:
            self.failures += 1
        text = detail if isinstance(detail, str) else json.dumps(detail)
        print(f"{'PASS' if ok else 'FAIL'}  {label:<48} {text[:260]}", flush=True)
        return ok

    def skip(self, label: str, reason: str) -> None:
        print(f"SKIP  {label:<48} {reason}", flush=True)

    def note(self, text: str) -> None:
        print(f"NOTE  {text}", flush=True)

    def step(self, label: str, run: Callable[[], Any]) -> None:
        """Runs a check that raises on failure and returns a detail on success."""
        try:
            self.check(label, True, run() or "")
        except (
            Exception
        ) as error:  # noqa: BLE001 - every failure is reported, not raised
            self.check(
                label, False, str(error).splitlines()[0] if str(error) else repr(error)
            )

    def finish(self) -> int:
        print(f"FAILURES={self.failures}", flush=True)
        return 0 if self.failures == 0 else 1
