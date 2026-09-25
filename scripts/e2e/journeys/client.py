# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Standard-library clients for the public interfaces the journeys drive."""

import base64
import hashlib
import html.parser
import http.cookiejar
import json
import os
import re
import secrets
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ENV = os.environ
TENANT_ID = ENV["SUPER_ADMIN_TENANT_ID"]
TENANT_REALM = f"tenant-{TENANT_ID}"
HASURA_URL = ENV["HASURA_ENDPOINT"]
KEYCLOAK_URL = ENV["KEYCLOAK_URL"].rstrip("/")
S3_URL = ENV["AWS_S3_PRIVATE_URI"].rstrip("/")
B4_URL = ENV["B4_URL"].rstrip("/")
OUTPUT = Path(ENV.get("STEP_E2E_OUTPUT_DIR", "/tmp/step-e2e"))


def event_realm(election_event_id):
    """Name the Keycloak realm windmill creates for an election event."""
    return f"tenant-{TENANT_ID}-event-{election_event_id}"


def wait_until(description, probe, timeout, interval=2.0):
    """Poll `probe` until it returns a truthy value, or fail after `timeout` seconds."""
    deadline = time.monotonic() + timeout
    last_error = None
    while True:
        try:
            value = probe()
            if value:
                return value
        except Exception as error:  # noqa: BLE001 - reported on timeout
            last_error = error
        if time.monotonic() >= deadline:
            detail = f" (last error: {last_error!r})" if last_error else ""
            raise TimeoutError(f"Timed out after {timeout}s waiting for {description}{detail}")
        time.sleep(interval)


@dataclass
class Response:
    status: int
    headers: dict
    body: bytes
    url: str

    @property
    def text(self):
        return self.body.decode("utf-8", "replace")

    def json(self):
        return json.loads(self.body)


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


class Http:
    """One cookie jar, no automatic redirects, and error statuses as values."""

    def __init__(self):
        self.jar = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(self.jar), _NoRedirect()
        )

    def request(self, method, url, *, form=None, json_body=None, data=None, headers=None, timeout=120):
        headers = dict(headers or {})
        if form is not None:
            data = urllib.parse.urlencode(form).encode()
            headers.setdefault("Content-Type", "application/x-www-form-urlencoded")
        elif json_body is not None:
            data = json.dumps(json_body).encode()
            headers.setdefault("Content-Type", "application/json")
        request = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with self.opener.open(request, timeout=timeout) as response:
                return Response(response.status, dict(response.headers), response.read(), url)
        except urllib.error.HTTPError as error:
            return Response(error.code, dict(error.headers), error.read(), url)

    def get(self, url, **kwargs):
        return self.request("GET", url, **kwargs)

    def post(self, url, **kwargs):
        return self.request("POST", url, **kwargs)


def http_get(url, **kwargs):
    return Http().get(url, **kwargs)


def jwt_claims(token):
    """Decode a JWT payload without verifying it; verification is the server's job."""
    payload = token.split(".")[1]
    return json.loads(base64.urlsafe_b64decode(payload + "=" * (-len(payload) % 4)))


class GraphQLError(AssertionError):
    def __init__(self, errors):
        super().__init__(json.dumps(errors))
        self.errors = errors


class Hasura:
    """GraphQL over Hasura, either as admin (secret) or with a bearer token."""

    def __init__(self, token=None, admin_secret=None):
        self.token = token
        self.admin_secret = admin_secret

    @classmethod
    def admin(cls):
        return cls(admin_secret=ENV["HASURA_ADMIN_SECRET"])

    def execute(self, query, variables=None):
        headers = {}
        if self.admin_secret:
            headers["x-hasura-admin-secret"] = self.admin_secret
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        response = Http().post(
            HASURA_URL, json_body={"query": query, "variables": variables or {}}, headers=headers
        )
        if response.status != 200:
            raise AssertionError(f"Hasura answered HTTP {response.status}: {response.text[:500]}")
        return response.json()

    def query(self, query, variables=None):
        """Return `data`, raising GraphQLError when Hasura reports errors."""
        result = self.execute(query, variables)
        if result.get("errors"):
            raise GraphQLError(result["errors"])
        return result["data"]


class _Forms(html.parser.HTMLParser):
    """Collect every form with its action and named inputs."""

    def __init__(self):
        super().__init__()
        self.forms = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        kind = (attrs.get("type") or ("submit" if tag == "button" else "text")).lower()
        if tag == "form":
            self.forms.append({"id": attrs.get("id"), "action": attrs.get("action"), "inputs": {}})
        elif tag in ("input", "button") and self.forms and attrs.get("name"):
            # Browsers never submit plain buttons, nor unchecked boxes.
            if kind == "button" or (tag == "button" and kind != "submit"):
                return
            if kind in ("checkbox", "radio") and "checked" not in attrs:
                return
            self.forms[-1]["inputs"][attrs["name"]] = attrs.get("value") or ""


def parse_forms(page):
    parser = _Forms()
    parser.feed(page)
    return parser.forms


class LoginError(AssertionError):
    pass


class Keycloak:
    """Keycloak's OIDC endpoints and its admin REST API (master realm admin)."""

    def __init__(self):
        self._admin_token = None
        self._admin_expiry = 0.0

    def token_url(self, realm):
        return f"{KEYCLOAK_URL}/realms/{realm}/protocol/openid-connect/token"

    def password_grant(self, realm, client_id, username, password, client_secret=None):
        form = {
            "grant_type": "password",
            "client_id": client_id,
            "username": username,
            "password": password,
            "scope": "openid",
        }
        if client_secret:
            form["client_secret"] = client_secret
        response = Http().post(self.token_url(realm), form=form)
        body = response.json()
        if response.status != 200:
            raise LoginError(f"{client_id} password grant for {username}: {body}")
        return body

    def admin_token(self):
        if time.monotonic() >= self._admin_expiry:
            body = self.password_grant("master", "admin-cli", ENV["KEYCLOAK_ADMIN"], ENV["KEYCLOAK_ADMIN_PASSWORD"])
            self._admin_token = body["access_token"]
            self._admin_expiry = time.monotonic() + body["expires_in"] - 10
        return self._admin_token

    def admin(self, method, path, json_body=None, expect=(200,)):
        response = Http().request(
            method,
            f"{KEYCLOAK_URL}/admin/realms/{path}",
            json_body=json_body,
            headers={"Authorization": f"Bearer {self.admin_token()}"},
        )
        if response.status not in expect:
            raise AssertionError(f"Keycloak admin {method} {path}: HTTP {response.status} {response.text[:300]}")
        return response.json() if response.body else None

    def user(self, realm, username):
        users = self.admin("GET", f"{realm}/users?exact=true&username={urllib.parse.quote(username)}")
        if len(users) != 1:
            raise AssertionError(f"Expected one {username} in {realm}, found {len(users)}")
        return users[0]

    def browser_login(self, realm, client_id, redirect_uri, username, password, otp=None):
        """Authorization-code login with PKCE, submitting the realm's own HTML forms."""
        session = Http()
        verifier = secrets.token_urlsafe(48)
        challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
        state, nonce = secrets.token_urlsafe(16), secrets.token_urlsafe(16)
        query = urllib.parse.urlencode(
            {
                "client_id": client_id,
                "redirect_uri": redirect_uri,
                "response_type": "code",
                "response_mode": "fragment",
                "scope": "openid",
                "state": state,
                "nonce": nonce,
                "code_challenge": challenge,
                "code_challenge_method": "S256",
            }
        )
        response = session.get(f"{KEYCLOAK_URL}/realms/{realm}/protocol/openid-connect/auth?{query}")
        for _ in range(4):
            if response.status in (302, 303):
                break
            if response.status != 200:
                raise LoginError(f"Login page for {username} answered HTTP {response.status}")
            forms = parse_forms(response.text)
            login = next((form for form in forms if form["id"] == "kc-form-login"), None)
            if login is not None:
                values = {name: value or "" for name, value in login["inputs"].items()}
                values.update(username=username, password=password)
            else:
                otp_form = next((form for form in forms if "code" in form["inputs"]), None)
                if otp_form is None or otp is None:
                    error = re.search(r'kc-feedback-text">([^<]+)<|alert-error[^>]*>\s*([^<]+)<', response.text)
                    raise LoginError(f"Unexpected login page for {username}: {error.groups() if error else forms}")
                values = {name: value or "" for name, value in otp_form["inputs"].items()}
                # The page's script copies the per-digit boxes into `code`.
                values.update({f"otp{index}": digit for index, digit in enumerate(otp, 1)}, code=otp)
                login = otp_form
            action = html.unescape(login["action"])
            response = session.post(urllib.parse.urljoin(response.url, action), form=values)
            response.url = urllib.parse.urljoin(response.url, action)
        location = response.headers.get("Location", "")
        if not location.startswith(redirect_uri.split("?")[0]):
            raise LoginError(f"Login for {username} did not return to the client: {response.status} {location[:200]}")
        fragment = urllib.parse.parse_qs(urllib.parse.urlsplit(location).fragment)
        if fragment.get("state") != [state] or "code" not in fragment:
            raise LoginError(f"OAuth callback for {username} lacks the code or state")
        token = session.post(
            self.token_url(realm),
            form={
                "grant_type": "authorization_code",
                "code": fragment["code"][0],
                "client_id": client_id,
                "redirect_uri": redirect_uri,
                "code_verifier": verifier,
            },
        )
        body = token.json()
        if token.status != 200 or "access_token" not in body:
            raise LoginError(f"Token exchange for {username} failed: {body}")
        if jwt_claims(body["id_token"]).get("nonce") != nonce:
            raise LoginError("OIDC nonce mismatch")
        return body


class StepCliError(AssertionError):
    pass


class StepCli:
    """step-cli with a private, writable copy (it keeps its session beside the binary)."""

    def __init__(self, source=None):
        source = Path(source or Path(ENV.get("STEP_E2E_BIN_DIR", "/opt/step-e2e/bin")) / "step-cli")
        self.directory = Path(tempfile.mkdtemp(prefix="step-cli-"))
        self.binary = self.directory / "step-cli"
        shutil.copy2(source, self.binary)
        self.log = OUTPUT / "step-cli.log"

    def run(self, *args, check=True, timeout=900):
        """Run a command. Legacy commands report failures only as `Error!` lines."""
        started = time.monotonic()
        process = subprocess.run(
            [str(self.binary), *args],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            env={**ENV, "NO_COLOR": "1", "HOME": str(self.directory)},
            timeout=timeout,
        )
        output = re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", process.stdout)
        with self.log.open("a") as log:
            printable = [arg if len(arg) < 200 else arg[:200] + "..." for arg in args]
            log.write(f"$ step-cli {' '.join(printable)}  # exit {process.returncode}, {time.monotonic() - started:.1f}s\n{output}\n")
        failed = process.returncode != 0 or re.search(r"^Error!", output, re.MULTILINE)
        if check and failed:
            raise StepCliError(f"step-cli {args[:2]} failed (exit {process.returncode}):\n{output[-2000:]}")
        return process.returncode, output

    def step(self, *args, **kwargs):
        return self.run("step", *args, **kwargs)[1]

    @staticmethod
    def last_id(output):
        """Return the ID printed on step-cli's final `Success! ... ID <uuid>` line."""
        ids = re.findall(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", output)
        if not ids:
            raise StepCliError(f"No ID in step-cli output:\n{output[-1000:]}")
        return ids[-1]


@dataclass
class Voter:
    username: str
    area: str
    password: str
    tokens: dict = field(default_factory=dict)
