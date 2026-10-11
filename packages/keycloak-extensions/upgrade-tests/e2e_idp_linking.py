#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""idp-linking-authenticator scenarios: voters log in to the voting portal through the upstream IdP
set up by e2e_idp_linking_setup.py, and Keycloak's admin API shows what got linked.
"""

import re
import sys
import urllib.parse

from playwright.sync_api import Browser, sync_playwright

from browser import body_text, new_context, screenshot, screenshot_dir, wait_for_outcome
from common import (
    TENANT_ID,
    VOTING_PORTAL_URL,
    Checks,
    event_realm,
    keycloak,
    require_state,
    user_id,
)

SHOTS = screenshot_dir("e2e-idp-linking")
LINK_VOTER = "e2e-link-v"
AMBIGUOUS_VOTERS = ("e2e-link-w1", "e2e-link-w2")


def main() -> int:
    state = require_state("e2e_event_id", "e2e_upstream_realm", "e2e_idp_alias")
    checks = Checks()
    realm = event_realm(state["e2e_event_id"])
    alias = state["e2e_idp_alias"]
    flow_path = urllib.parse.quote(state["e2e_link_flow"])

    def links(username: str) -> list[str]:
        identities = keycloak(
            "GET", f"/{realm}/users/{user_id(realm, username)}/federated-identity"
        ).json()
        return [
            f"{identity['identityProvider']}:{identity['userName']}"
            for identity in identities
        ]

    def user_count() -> int:
        return keycloak("GET", f"/{realm}/users/count").json()

    def login_via(browser: Browser, identity: str, file: str) -> dict:
        """Logs in through the upstream IdP: "portal" when the voter got in, "rejected" when
        Keycloak refused the first broker login."""
        context = new_context(browser)
        page = context.new_page()
        try:
            page.goto(
                f"{VOTING_PORTAL_URL}/tenant/{TENANT_ID}/event/{state['e2e_event_id']}/login?lang=en",
                wait_until="domcontentloaded",
            )
            page.locator("#kc-login").wait_for(timeout=90000)
            page.locator(f"#social-{alias}").click()
            page.wait_for_url(
                re.compile(f"/realms/{state['e2e_upstream_realm']}/"), timeout=60000
            )
            page.fill("#username", identity)
            page.fill("#password", state["e2e_upstream_password"])
            page.click("#kc-login")
            outcome = wait_for_outcome(
                page,
                {
                    "portal": lambda p: p.url.startswith(VOTING_PORTAL_URL)
                    and not p.url.split("?")[0].endswith("/login"),
                    "rejected": lambda p: f"/realms/{realm}/login-actions/" in p.url
                    and p.locator(
                        "#kc-content, .kc-error-alert-text, .alert-error"
                    ).first.is_visible(),
                },
            )
            screenshot(page, SHOTS / file)
            return {
                "outcome": outcome,
                "path": urllib.parse.urlparse(page.url).path,
                "text": body_text(page),
            }
        finally:
            context.close()

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(headless=True)

        def links_to_v(identity: str, file: str, expected: list[str]):
            def run() -> str:
                result = login_via(browser, identity, file)
                linked = links(LINK_VOTER)
                if result["outcome"] != "portal" or linked != expected:
                    raise AssertionError(
                        f"{result['outcome']} links={linked} | {result['text'][:120]}"
                    )
                return f"links={linked}"

            return run

        def rejected(identity: str, file: str, check_voters: tuple[str, ...]):
            def run() -> str:
                before = user_count()
                result = login_via(browser, identity, file)
                after = user_count()
                linked = [link for voter in check_voters for link in links(voter)]
                if (
                    result["outcome"] != "rejected"
                    or before != after
                    or user_id(realm, identity)
                    or linked
                ):
                    raise AssertionError(
                        f"{result['outcome']} users {before}->{after} links={linked}"
                    )
                return f"{result['path']} | {result['text'][:100]}"

            return run

        def missing_claim() -> str:
            execution = keycloak(
                "GET", f"/{realm}/authentication/flows/{flow_path}/executions"
            ).json()[0]
            config_path = (
                f"/{realm}/authentication/config/{execution['authenticationConfig']}"
            )
            original = keycloak("GET", config_path).json()
            keycloak(
                "PUT",
                config_path,
                {**original, "config": {**original["config"], "idp-claim": "SAFE_ID"}},
            )
            try:
                return rejected("idp-b", "6-missing-claim.png", ())()
            finally:
                keycloak("PUT", config_path, original)

        checks.step(
            "1 a@ links to voter V",
            links_to_v("idp-a", "1-a-links-to-v.png", [f"{alias}:idp-a"]),
        )
        checks.step(
            "2 b@ also logs in as V (the link moves to b@)",
            links_to_v("idp-b", "2-b-links-to-v.png", [f"{alias}:idp-b"]),
        )
        checks.step(
            "3 a@ again is re-linked through the attribute",
            links_to_v("idp-a", "3-a-relinks-to-v.png", [f"{alias}:idp-a"]),
        )
        checks.step(
            "4 c@ (no voter holds it) is rejected, nobody created",
            rejected("idp-c", "4-c-no-match.png", ()),
        )
        checks.step(
            "5 shared@ (two voters hold it) is rejected, nobody linked",
            rejected("idp-shared", "5-shared-ambiguous.png", AMBIGUOUS_VOTERS),
        )
        checks.step("6 claim missing from the IdP is rejected", missing_claim)
        checks.check(
            "V still linked to a@ only",
            links(LINK_VOTER) == [f"{alias}:idp-a"],
            links(LINK_VOTER),
        )
        browser.close()

    print(f"screenshots: {SHOTS}")
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
