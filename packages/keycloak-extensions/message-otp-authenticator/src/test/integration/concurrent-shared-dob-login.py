#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Concurrent voting-portal logins of voters who share a date of birth (meta#13460).

Runs against the running dev environment; see readme.md next to this file.

Exit status: 0 all logins succeeded, 1 concurrent logins were rejected (the
meta#13460 failure), 2 the environment or baseline logins are not usable.

Reference URLs for the event it uses (browser, via the localhost port forwards):
  Admin portal:  http://localhost:3002/sequent_backend_election_event/951a8a8a-7877-483e-aaf1-02d2417cb636
  Voting portal: http://localhost:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-951a8a8a-7877-483e-aaf1-02d2417cb636/protocol/openid-connect/auth?client_id=voting-portal&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Ftenant%2F90505c8a-23a9-4cdf-a26b-4e19f6a097d5%2Fevent%2F951a8a8a-7877-483e-aaf1-02d2417cb636%2Flogin&state=d20b778f-4fc1-4e7c-ba4f-2df8da8ab563&response_mode=fragment&response_type=code&scope=openid&nonce=715dd054-518a-42cd-bb5b-08c284eeeb91&code_challenge=pNua2wWb10y5Flet0st9JVJTC2YIY6H_aBa9K57xWl4&code_challenge_method=S256
"""

import argparse
import base64
from dataclasses import dataclass
from typing import Optional
import datetime
import hashlib
import html
import http.cookiejar
import json
import os
import re
import secrets
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

TENANT_ID = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
ELECTION_EVENT_ID = "951a8a8a-7877-483e-aaf1-02d2417cb636"
DATAFIX_EVENT_ID = "0014"
REALM = f"tenant-{TENANT_ID}-event-{ELECTION_EVENT_ID}"
CLIENT_ID = "voting-portal"
REDIRECT_URI = (
    f"http://localhost:3000/tenant/{TENANT_ID}/event/{ELECTION_EVENT_ID}/login"
)
# Where Keycloak sends a voter whose credentials it accepted but who still has a
# required action (e.g. choosing an MFA method); rejected credentials re-render
# the login form instead.
REQUIRED_ACTION_PATH = "/login-actions/required-action"

ADMIN_PORTAL_EVENT_URL = (
    f"http://localhost:3002/sequent_backend_election_event/{ELECTION_EVENT_ID}"
)

# Datafix names areas WARD[-SCHOOLBOARD]-000: the poll is always 000.
DATAFIX_POLL = "000"
DEFAULT_AREA = "07-P-000"
AREA_NOT_FOUND = "area-not-found"

# Voters are added with ids from here up, above the imported voter ids, and stay
# in the event after the run.
FIRST_VOTER_ID = 1000000001
# Keycloak's user search matches this as a username prefix.
VOTER_ID_SEARCH_PREFIX = "1000000"

KEYCLOAK_URL = os.environ.get("KEYCLOAK_URL", "http://keycloak:8090").rstrip("/")
KEYCLOAK_ADMIN_USER = os.environ.get("KEYCLOAK_ADMIN_USER", "admin")
KEYCLOAK_ADMIN_PASSWORD = os.environ.get("KEYCLOAK_ADMIN_PASSWORD", "admin")
HARVEST_URL = os.environ.get("HARVEST_URL", "http://harvest:8400").rstrip("/")

SEARCH_BIRTHDATES_FROM = "1921-01-01"
MAX_BIRTHDATE_SEARCH_DAYS = 366
TIMEOUT_SECONDS = 60
EXIT_OK, EXIT_REJECTED, EXIT_SETUP = 0, 1, 2


@dataclass
class LoginResult:
    ok: bool
    detail: str
    started: float
    finished: float

    @property
    def millis(self):
        return int((self.finished - self.started) * 1000)


@dataclass(frozen=True)
class Area:
    name: str
    ward: str
    schoolboard: Optional[str]


def parse_area(name):
    """Splits a Datafix area name, WARD[-SCHOOLBOARD]-000, into its request fields."""
    parts = name.upper().split("-")
    if len(parts) not in (2, 3) or parts[-1] != DATAFIX_POLL or not all(parts):
        raise argparse.ArgumentTypeError(
            f"{name!r} is not a Datafix area name; use WARD-SCHOOLBOARD-{DATAFIX_POLL} "
            f"or WARD-{DATAFIX_POLL}, e.g. {DEFAULT_AREA}"
        )
    return Area("-".join(parts), parts[0], parts[1] if len(parts) == 3 else None)


class SetupError(Exception):
    pass


class HttpError(SetupError):
    def __init__(self, message, body):
        super().__init__(message)
        self.body = body

    def json_field(self, name):
        try:
            return json.loads(self.body).get(name)
        except (ValueError, AttributeError):
            return None


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def request_json(url, data=None, headers=None, method=None):
    request = urllib.request.Request(
        url, data=data, headers=headers or {}, method=method
    )
    try:
        with urllib.request.urlopen(request, timeout=TIMEOUT_SECONDS) as response:
            body = response.read()
    except urllib.error.HTTPError as err:
        detail = err.read().decode(errors="replace").strip()
        raise HttpError(f"{method or 'GET'} {url} -> HTTP {err.code}: {detail}", detail)
    except urllib.error.URLError as err:
        raise SetupError(f"cannot reach {url}: {err.reason}")
    return json.loads(body) if body else None


class KeycloakAdmin:
    def call(self, path, **query):
        # Master realm admin tokens are short-lived, so take a fresh one per call.
        token = request_json(
            f"{KEYCLOAK_URL}/realms/master/protocol/openid-connect/token",
            data=urllib.parse.urlencode(
                {
                    "client_id": "admin-cli",
                    "grant_type": "password",
                    "username": KEYCLOAK_ADMIN_USER,
                    "password": KEYCLOAK_ADMIN_PASSWORD,
                }
            ).encode(),
        )
        url = f"{KEYCLOAK_URL}/admin/realms/{REALM}/{path}?"
        return request_json(
            url + urllib.parse.urlencode(query),
            headers={"Authorization": f"Bearer {token['access_token']}"},
        )

    def usernames_with_birthdate(self, birthdate):
        users = self.call(
            "users",
            q=f"dateOfBirth:{birthdate}",
            exact="true",
            max=100,
            briefRepresentation="true",
        )
        return {user["username"] for user in users}

    def next_free_voter_id(self):
        """One above the highest numeric username at or over FIRST_VOTER_ID."""
        users = self.call(
            "users",
            search=VOTER_ID_SEARCH_PREFIX,
            max=100000,
            briefRepresentation="true",
        )
        taken = [
            int(user["username"])
            for user in users
            if user["username"].isdigit() and int(user["username"]) >= FIRST_VOTER_ID
        ]
        return max(taken, default=FIRST_VOTER_ID - 1) + 1


def datafix(path, body):
    password = os.environ.get("DATAFIX_PASSWORD")
    if not password:
        raise SetupError("set DATAFIX_PASSWORD to the tenant's datafix-account password")
    return request_json(
        f"{HARVEST_URL}/api/datafix/{path}",
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "tenant-id": TENANT_ID,
            "event-id": DATAFIX_EVENT_ID,
            "authorization": f"datafix-account:{password}",
        },
        method="POST",
    )


def add_voter(voter_id, birthdate, area):
    try:
        datafix(
            "add-voter",
            {
                "voter_id": voter_id,
                "ward": area.ward,
                "schoolboard": area.schoolboard,
                "poll": DATAFIX_POLL,
                "birthdate": birthdate,
            },
        )
    except HttpError as err:
        if err.json_field("error_code") != AREA_NOT_FOUND:
            raise
        raise SetupError(
            f"area {area.name} does not exist in election event {ELECTION_EVENT_ID}, "
            "so Datafix cannot add voters to it. No voters were added.\n"
            f"  Add an area named exactly {area.name} in the admin portal, under the "
            "election event's Areas tab:\n"
            f"    {ADMIN_PORTAL_EVENT_URL}\n"
            f"  or run again with an existing area, e.g. --area {DEFAULT_AREA}."
        ) from None


def replace_pin(voter_id):
    return datafix("replace-pin", {"voter_id": voter_id})["pin"]


class LoginSession:
    """One browser-like login attempt: loads the login form, then submits it."""

    def __init__(self):
        self.opener = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()),
            NoRedirect,
        )
        verifier = secrets.token_urlsafe(48)
        challenge = (
            base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest())
            .rstrip(b"=")
            .decode()
        )
        query = urllib.parse.urlencode(
            {
                "client_id": CLIENT_ID,
                "redirect_uri": REDIRECT_URI,
                "state": secrets.token_hex(8),
                "response_mode": "fragment",
                "response_type": "code",
                "scope": "openid",
                "nonce": secrets.token_hex(8),
                "code_challenge": challenge,
                "code_challenge_method": "S256",
            }
        )
        url = f"{KEYCLOAK_URL}/realms/{REALM}/protocol/openid-connect/auth?{query}"
        try:
            with self.opener.open(url, timeout=TIMEOUT_SECONDS) as response:
                page = response.read().decode(errors="replace")
        except (urllib.error.HTTPError, urllib.error.URLError) as err:
            raise SetupError(f"cannot load the voting-portal login form: {err}")
        match = re.search(r'<form id="kc-form-login"[^>]*action="([^"]+)"', page)
        if not match:
            raise SetupError("the voting-portal login page has no kc-form-login form")
        # The action carries the realm's frontend URL (localhost); post to the
        # Keycloak we loaded the form from so the session cookies are sent.
        action = urllib.parse.urlsplit(html.unescape(match.group(1)))
        base = urllib.parse.urlsplit(KEYCLOAK_URL)
        self.action = action._replace(scheme=base.scheme, netloc=base.netloc).geturl()

    def submit(self, birthdate, pin):
        form = urllib.parse.urlencode(
            {"dateOfBirth": birthdate, "password": pin, "credentialId": ""}
        ).encode()
        started = time.monotonic()
        try:
            with self.opener.open(self.action, data=form, timeout=TIMEOUT_SECONDS) as response:
                page = response.read().decode(errors="replace")
        except urllib.error.HTTPError as err:
            finished = time.monotonic()
            location = err.headers.get("Location", "")
            if err.code in (302, 303) and location.startswith(REDIRECT_URI):
                if "code=" in location:
                    return LoginResult(True, "logged in", started, finished)
                error = urllib.parse.parse_qs(
                    urllib.parse.urlsplit(location).fragment
                ).get("error", ["unknown"])[0]
                return LoginResult(False, f"redirected with error={error}", started, finished)
            target = urllib.parse.urlsplit(location)
            if err.code in (302, 303) and target.path.endswith(REQUIRED_ACTION_PATH):
                action = urllib.parse.parse_qs(target.query).get("execution", ["?"])[0]
                return LoginResult(
                    True,
                    f"credentials accepted, continued to required action {action}",
                    started,
                    finished,
                )
            return LoginResult(
                False, f"HTTP {err.code} {location}".rstrip(), started, finished
            )
        except urllib.error.URLError as err:
            return LoginResult(False, f"request failed: {err.reason}", started, time.monotonic())
        finished = time.monotonic()
        alert = re.search(r'class="pf-c-alert__title kc-feedback-text">([^<]*)<', page)
        message = html.unescape(alert.group(1).strip()) if alert else None
        if 'id="kc-form-login"' in page and 'name="dateOfBirth"' in page:
            return LoginResult(
                False, f'"{message or "login form shown again"}"', started, finished
            )
        title = re.search(r"<title>([^<]*)</title>", page)
        page_name = html.unescape(title.group(1).strip()) if title else "another page"
        return LoginResult(
            True, f"credentials accepted, continued to {page_name}", started, finished
        )


def concurrent_round(voter_ids, birthdate, pins):
    sessions = {voter_id: LoginSession() for voter_id in voter_ids}
    barrier = threading.Barrier(len(voter_ids))
    results = {}

    def login(voter_id):
        barrier.wait()
        results[voter_id] = sessions[voter_id].submit(birthdate, pins[voter_id])

    threads = [threading.Thread(target=login, args=(voter_id,)) for voter_id in voter_ids]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    return results


def overlap_millis(results):
    """How long every request of a round was in flight at the same time."""
    latest_start = max(result.started for result in results.values())
    earliest_end = min(result.finished for result in results.values())
    return int((earliest_end - latest_start) * 1000)


def step(ok, message):
    print(f"  [{'OK' if ok else 'FAIL'}] {message}", flush=True)


def unused_birthdate(admin):
    """First date from SEARCH_BIRTHDATES_FROM that no voter in the realm has."""
    day = datetime.date.fromisoformat(SEARCH_BIRTHDATES_FROM)
    for _ in range(MAX_BIRTHDATE_SEARCH_DAYS):
        if not admin.usernames_with_birthdate(day.isoformat()):
            return day.isoformat()
        day += datetime.timedelta(days=1)
    raise SetupError(
        f"no unused date of birth in the {MAX_BIRTHDATE_SEARCH_DAYS} days from "
        f"{SEARCH_BIRTHDATES_FROM}"
    )


def prepare(admin, count, area, added):
    """Adds `count` voters sharing a fresh date of birth; returns it and their PINs."""
    print("\nSetup")
    birthdate = unused_birthdate(admin)
    step(True, f"no voter has date of birth {birthdate} yet")
    first_id = admin.next_free_voter_id()
    for voter_id in map(str, range(first_id, first_id + count)):
        add_voter(voter_id, birthdate, area)
        added.append(voter_id)
        step(True, f"{voter_id}: added to area {area.name}, date of birth {birthdate}")
    sharing = admin.usernames_with_birthdate(birthdate)
    if sharing != set(added):
        raise SetupError(
            f"Keycloak shows {sorted(sharing)} with date of birth {birthdate}, "
            f"expected {added}"
        )
    step(True, f"Keycloak lists exactly these {count} voters with date of birth {birthdate}")
    pins = {voter_id: replace_pin(voter_id) for voter_id in added}
    step(True, f"PINs issued for {len(pins)} voters")
    return birthdate, pins


def run(admin, count, area, rounds, added):
    """Returns the exit status; raises SetupError when the test cannot run."""
    birthdate, pins = prepare(admin, count, area, added)

    print("\nBaseline: each voter logs in on their own")
    for voter_id in added:
        result = LoginSession().submit(birthdate, pins[voter_id])
        step(result.ok, f"{voter_id}: {result.detail} ({result.millis} ms)")
        if not result.ok:
            raise SetupError(
                f"voter {voter_id} cannot log in even without concurrency, so the "
                "concurrent rounds would not tell us anything; check the realm's login "
                "flow and the voter's account"
            )

    print(f"\nConcurrent: all {count} voters log in at once, {rounds} round(s)")
    rejected = []
    overlapped = 0
    for round_number in range(1, rounds + 1):
        results = concurrent_round(added, birthdate, pins)
        overlap = overlap_millis(results)
        overlapped += overlap > 0
        succeeded = sum(result.ok for result in results.values())
        print(
            f"  round {round_number}: {succeeded}/{count} logged in, all requests "
            + (f"in flight together for {overlap} ms" if overlap > 0 else "did NOT overlap")
        )
        for voter_id in added:
            result = results[voter_id]
            step(result.ok, f"{voter_id}: {result.detail} ({result.millis} ms)")
            if not result.ok:
                rejected.append((round_number, voter_id, result.detail))

    attempts = rounds * count
    print()
    if rejected:
        print("=" * 78)
        print(
            f"RESULT: FAIL - {len(rejected)} of {attempts} concurrent logins were rejected"
        )
        print("=" * 78)
        print(
            "Every voter submitted their own correct date of birth and PIN, and each of\n"
            "them logged in fine on their own in the baseline. They were only rejected\n"
            "while logging in at the same time as other voters sharing their date of\n"
            "birth, which is the meta#13460 bug: Keycloak's brute-force protector holds\n"
            "every account a login request asks it about until that request ends, and\n"
            "the Multi-Attribute + Password Form asked about every voter sharing the\n"
            "submitted date of birth.\n"
            "\n"
            "Expected on a Keycloak built without the fix in\n"
            "MultiAttributeCredentialResolver (e.g. plain release/10.0); on a build with\n"
            "the fix this is a regression.\n"
        )
        for round_number, voter_id, detail in rejected:
            print(f"  round {round_number}, voter {voter_id}: {detail}")
        return EXIT_REJECTED
    if not overlapped:
        raise SetupError(
            "all logins succeeded, but no round had every request in flight at once, "
            "so the test is inconclusive; run it again with more --rounds"
        )
    print("=" * 78)
    print(
        f"RESULT: PASS - all {attempts} concurrent logins succeeded "
        f"({overlapped}/{rounds} rounds fully overlapped)"
    )
    print("=" * 78)
    return EXIT_OK


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--voters", type=int, default=5, help="voters to add (at least 2)")
    parser.add_argument(
        "--area",
        type=parse_area,
        default=DEFAULT_AREA,
        help=f"existing area to add the voters to (default: {DEFAULT_AREA})",
    )
    parser.add_argument("--rounds", type=int, default=5)
    args = parser.parse_args()
    if args.voters < 2:
        parser.error("--voters must be at least 2 for their logins to collide")

    print("Concurrent shared date-of-birth login test (meta#13460)")
    print(f"  Keycloak: {KEYCLOAK_URL}  realm: {REALM}")
    print(f"  Harvest:  {HARVEST_URL}  datafix event: {DATAFIX_EVENT_ID}")

    added = []
    try:
        status = run(KeycloakAdmin(), args.voters, args.area, args.rounds, added)
    except SetupError as err:
        print(f"\nSETUP ERROR: {err}")
        status = EXIT_SETUP
    if added:
        print(
            f"\nThe added voters {added[0]}-{added[-1]} remain in the event; "
            "the next run adds new ones."
        )
    return status


if __name__ == "__main__":
    sys.exit(main())
