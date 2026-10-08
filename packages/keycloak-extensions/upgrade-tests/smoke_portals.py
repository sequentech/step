#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Logs in to the admin portal (and opens the Voters tab) and to the voting portal with the voters
smoke_admin_client.py created, taking screenshots of every Keycloak page so the custom login
templates can be reviewed after an upgrade.
"""

import json
import re
import sys
import urllib.parse

from playwright.sync_api import Page, sync_playwright

from browser import new_context, screenshot, screenshot_dir
from common import (
    ADMIN_PORTAL_TEST_PASSWORD,
    ADMIN_PORTAL_TEST_USERNAME,
    ADMIN_PORTAL_URL,
    KEYCLOAK_URL,
    TENANT_ID,
    TENANT_REALM,
    VOTING_PORTAL_URL,
    Checks,
    event_realm,
    require_state,
)

SHOTS = screenshot_dir("smoke-portals")
SESSION_POLLING = re.compile(
    r'import \{ startSessionPolling \} from ("[^"]+");\s*startSessionPolling\(\s*("(?:[^"\\]|\\.)*")\s*\);'
)
ELECTIONS = ".election-item, .click-to-vote-button, [class*=election]"
THEME_FOLDERS_JS = """() => {
    const themes = new Set()
    for (const node of document.querySelectorAll("link[rel=stylesheet], script[src]")) {
        const match = (node.href || node.src).match(/\\/resources\\/[^/]+\\/login\\/([^/]+)\\//)
        if (match) themes.add(match[1])
    }
    return [...themes].join(",")
}"""


def keycloak_page(realm: str) -> re.Pattern:
    return re.compile(
        f"/realms/{realm}/(protocol/openid-connect/(auth|registrations)|login-actions/)"
    )


def from_origin(origin: str):
    return lambda url: url.startswith(origin)


def shot(page: Page, file: str) -> str:
    return screenshot(page, SHOTS / file)


def login_themes(page: Page) -> str:
    """Theme folders the page's assets are served from (/resources/<version>/login/<theme>/...)."""
    return page.evaluate(THEME_FOLDERS_JS)


def session_polling(page: Page, realm: str) -> str:
    """template.ftl must pass the session polling URL as a JavaScript string literal (26.8)."""
    match = SESSION_POLLING.search(page.content())
    if not match:
        raise AssertionError(
            "session polling script missing or not a JavaScript string literal"
        )
    target = urllib.parse.urlparse(
        urllib.parse.urljoin(KEYCLOAK_URL, json.loads(match.group(2)))
    )
    if not target.path.startswith(f"/realms/{realm}/login-actions/restart"):
        raise AssertionError(f"unexpected session polling URL {target.path}")
    return "session polling ok"


def assert_no_template_error(page: Page) -> None:
    body = page.locator("body").inner_text()
    if re.search(
        "FreeMarker template error|Template processing error", body, re.IGNORECASE
    ):
        raise AssertionError(f"template error rendered: {body[:200]}")


def main() -> int:
    state = require_state("smoke_password_event_id", "smoke_pin_event_id")
    checks = Checks()
    password_realm = event_realm(state["smoke_password_event_id"])
    pin_realm = event_realm(state["smoke_pin_event_id"])

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(headless=True)

        admin = new_context(browser, viewport={"width": 1440, "height": 900}).new_page()

        def admin_login_page() -> str:
            admin.goto(f"{ADMIN_PORTAL_URL}/", wait_until="domcontentloaded")
            admin.wait_for_url(keycloak_page(TENANT_REALM), timeout=90000)
            admin.locator("#kc-login").wait_for(timeout=30000)
            assert_no_template_error(admin)
            return f"{shot(admin, '01-admin-portal-login.png')} themes={login_themes(admin)} {session_polling(admin, TENANT_REALM)}"

        def admin_login() -> str:
            admin.fill("#username", ADMIN_PORTAL_TEST_USERNAME)
            admin.fill("#password", ADMIN_PORTAL_TEST_PASSWORD)
            admin.click("#kc-login")
            admin.wait_for_url(from_origin(ADMIN_PORTAL_URL), timeout=90000)
            admin.wait_for_timeout(4000)
            return shot(admin, "02-admin-portal-home.png")

        def voters_tab(event_id: str, voter: str, file: str):
            def run() -> str:
                admin.goto(
                    f"{ADMIN_PORTAL_URL}/sequent_backend_election_event/{event_id}",
                    wait_until="domcontentloaded",
                )
                admin.get_by_role("tab", name="Voters", exact=True).click(timeout=90000)
                rows = admin.locator("table tbody tr")
                rows.first.wait_for(timeout=90000)
                file = shot(admin, file)
                if rows.filter(has_text=voter).count() == 0:
                    raise AssertionError(
                        f"{voter} is not on the first page ({rows.count()} rows, see {file})"
                    )
                return f"{file} rows={rows.count()}, smoke voter listed"

            return run

        checks.step("admin: Keycloak login page (tenant realm)", admin_login_page)
        checks.step("admin: log in", admin_login)
        checks.step(
            f"admin: Voters tab (event {state['smoke_pin_event_id'][:8]})",
            voters_tab(
                state["smoke_pin_event_id"],
                state["smoke_pin_voter"],
                "03-admin-voters-tab-pin-event.png",
            ),
        )
        checks.step(
            f"admin: Voters tab (event {state['smoke_password_event_id'][:8]})",
            voters_tab(
                state["smoke_password_event_id"],
                state["smoke_password_voter"],
                "04-admin-voters-tab-password-event.png",
            ),
        )

        voting = new_context(browser).new_page()
        login_url = f"{VOTING_PORTAL_URL}/tenant/{TENANT_ID}/event/{state['smoke_password_event_id']}"

        def password_login_page() -> str:
            voting.goto(f"{login_url}/login?lang=en", wait_until="domcontentloaded")
            voting.wait_for_url(keycloak_page(password_realm), timeout=90000)
            voting.locator("#kc-login").wait_for(timeout=30000)
            assert_no_template_error(voting)
            return f"{shot(voting, '05-voting-portal-login-password.png')} themes={login_themes(voting)} {session_polling(voting, password_realm)}"

        def wrong_password() -> str:
            voting.fill("#username", state["smoke_password_voter"])
            voting.fill("#password", "definitely-wrong")
            voting.click("#kc-login")
            voting.locator(
                "#input-error, .kc-feedback-text, .alert-error, .pf-c-alert"
            ).first.wait_for(timeout=30000)
            assert_no_template_error(voting)
            return shot(voting, "06-voting-portal-login-error.png")

        def password_login() -> str:
            voting.fill("#username", state["smoke_password_voter"])
            voting.fill("#password", state["smoke_password_voter_secret"])
            voting.click("#kc-login")
            voting.wait_for_url(from_origin(VOTING_PORTAL_URL), timeout=90000)
            voting.locator(ELECTIONS).first.wait_for(timeout=90000)
            return f"{shot(voting, '07-voting-portal-after-login-password.png')} {urllib.parse.urlparse(voting.url).path}"

        checks.step(
            "voting: Keycloak login page (username + password)", password_login_page
        )
        checks.step("voting: wrong password shows the error state", wrong_password)
        checks.step("voting: log in with username + password", password_login)

        enroll = new_context(browser).new_page()

        def enrollment_page() -> str:
            enroll.goto(f"{login_url}/enroll?lang=en", wait_until="domcontentloaded")
            enroll.wait_for_url(keycloak_page(password_realm), timeout=90000)
            enroll.locator("form").first.wait_for(timeout=30000)
            assert_no_template_error(enroll)
            return f"{shot(enroll, '08-voting-portal-enrollment.png')} themes={login_themes(enroll)}"

        checks.step("voting: enrollment page (register.ftl)", enrollment_page)

        pin = new_context(browser).new_page()

        def pin_login_page() -> str:
            pin.goto(
                f"{VOTING_PORTAL_URL}/tenant/{TENANT_ID}/event/{state['smoke_pin_event_id']}/login?lang=en",
                wait_until="domcontentloaded",
            )
            pin.wait_for_url(keycloak_page(pin_realm), timeout=90000)
            pin.locator("#structured-password").wait_for(timeout=30000)
            pin.locator("#dateOfBirth").wait_for(timeout=30000)
            assert_no_template_error(pin)
            return f"{shot(pin, '09-voting-portal-login-pin.png')} themes={login_themes(pin)} {session_polling(pin, pin_realm)}"

        def pin_login() -> str:
            # Keyboard focus starts at the first PIN group; a click selects the group under the pointer.
            pin.locator("#structured-password").focus()
            pin.locator("#structured-password").press_sequentially(
                state["smoke_pin_voter_secret"]
            )
            pin.fill("#dateOfBirth", state["smoke_voter_date_of_birth"])
            shot(pin, "10-voting-portal-login-pin-filled.png")
            pin.click("#kc-login")
            pin.wait_for_url(from_origin(VOTING_PORTAL_URL), timeout=90000)
            pin.locator(ELECTIONS).first.wait_for(timeout=90000)
            return f"{shot(pin, '11-voting-portal-after-login-pin.png')} {urllib.parse.urlparse(pin.url).path}"

        checks.step(
            "voting: Keycloak login page (date of birth + structured PIN)",
            pin_login_page,
        )
        checks.step("voting: log in with date of birth + structured PIN", pin_login)
        browser.close()

    print(f"screenshots: {SHOTS}")
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
