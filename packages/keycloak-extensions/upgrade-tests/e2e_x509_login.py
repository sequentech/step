#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Certificate login scenarios: the voter clicks the certificate button on the voting portal login
page and the browser presents a client certificate to keycloak-nginx (KEYCLOAK_MTLS_URL). Needs
e2e_x509_setup.py.
"""

import re
import sys
import urllib.parse
from typing import Optional

from playwright.sync_api import Browser, BrowserContext, Page, sync_playwright

from browser import body_text, new_context, screenshot, screenshot_dir, wait_for_outcome
from common import (
    KEYCLOAK_MTLS_URL,
    OUT,
    TENANT_ID,
    VOTING_PORTAL_URL,
    Checks,
    event_realm,
    keycloak,
    require_state,
    user_id,
)

SHOTS = screenshot_dir("e2e-x509-login")
PKI = OUT / "pki"
# sequent.voting-portal/login/messages/messages_en.properties
MESSAGES = {
    "certNotProvided": "Authentication failed: No certificate was provided",
    "userNotFound": "Authentication failed: Valid certificate detected, but no matching voter or user was found",
    "accessDenied": "Authentication failed: Invalid certificate, access denied",
}


def main() -> int:
    state = require_state("e2e_event_id", "e2e_cert_voter")
    checks = Checks()
    realm = event_realm(state["e2e_event_id"])
    mtls_host = urllib.parse.urlparse(KEYCLOAK_MTLS_URL).netloc

    def set_certificate_policy(value: str) -> None:
        attributes = {
            **keycloak("GET", f"/{realm}").json()["attributes"],
            "voter-certificate-policy": value,
        }
        keycloak("PUT", f"/{realm}", {"attributes": attributes})

    def voter_links() -> list[str]:
        voter_id = user_id(realm, state["e2e_cert_voter"])
        identities = keycloak(
            "GET", f"/{realm}/users/{voter_id}/federated-identity"
        ).json()
        return [
            f"{identity['identityProvider']}:{identity['userName']}"
            for identity in identities
        ]

    def context_with(browser: Browser, certificate: Optional[str]) -> BrowserContext:
        certificates = (
            [
                {
                    "origin": KEYCLOAK_MTLS_URL,
                    "certPath": str(PKI / f"{certificate}.crt"),
                    "keyPath": str(PKI / f"{certificate}.key"),
                }
            ]
            if certificate
            else []
        )
        # keycloak-nginx serves a self-signed development certificate.
        return new_context(
            browser, ignore_https_errors=True, client_certificates=certificates
        )

    def open_login_page(page: Page) -> None:
        page.goto(
            f"{VOTING_PORTAL_URL}/tenant/{TENANT_ID}/event/{state['e2e_event_id']}/login?lang=en",
            wait_until="domcontentloaded",
        )
        page.locator("#kc-login").wait_for(timeout=90000)

    def certificate_login(
        browser: Browser, certificate: Optional[str], file: str
    ) -> dict:
        context = context_with(browser, certificate)
        page = context.new_page()
        hosts = set()
        page.on(
            "request",
            lambda request: hosts.add(urllib.parse.urlparse(request.url).netloc),
        )
        try:
            open_login_page(page)
            page.locator("#social-digital-certificates").click()
            outcome = wait_for_outcome(
                page,
                {
                    "portal": lambda p: p.url.startswith(VOTING_PORTAL_URL)
                    and not p.url.split("?")[0].endswith("/login"),
                    "error": lambda p: "AUTHENTICATION FAILED" in body_text(p).upper(),
                },
            )
            screenshot(page, SHOTS / file)
            return {
                "outcome": outcome,
                "text": body_text(page),
                "via_mtls": mtls_host in hosts,
            }
        finally:
            context.close()

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(headless=True)

        def button_follows_policy() -> str:
            def visible(file: str) -> bool:
                context = context_with(browser, None)
                try:
                    page = context.new_page()
                    open_login_page(page)
                    count = page.locator("#social-digital-certificates").count()
                    screenshot(page, SHOTS / file)
                    return count > 0
                finally:
                    context.close()

            try:
                set_certificate_policy("disabled")
                when_disabled = visible("0-policy-disabled.png")
            finally:
                # The remaining checks need the policy e2e_x509_setup.py enabled.
                set_certificate_policy("enabled")
            when_enabled = visible("0-policy-enabled.png")
            if when_disabled or not when_enabled:
                raise AssertionError(
                    f"visible when disabled={when_disabled}, when enabled={when_enabled}"
                )
            return "hidden when disabled, shown when enabled"

        def voter_logs_in() -> str:
            result = certificate_login(browser, "voter", "1-voter-certificate.png")
            linked = voter_links()
            if (
                result["outcome"] != "portal"
                or not result["via_mtls"]
                or not any(link.startswith("digital-certificates:") for link in linked)
            ):
                raise AssertionError(
                    f"{result['outcome']} via_mtls={result['via_mtls']} links={linked} | {result['text'][:120]}"
                )
            welcome = re.search(r"Welcome, \S+", result["text"])
            return f"links={linked} | {welcome.group(0) if welcome else ''}"

        def rejected(certificate: Optional[str], expected: str, file: str):
            def run() -> str:
                result = certificate_login(browser, certificate, file)
                if result["outcome"] != "error" or expected not in result["text"]:
                    raise AssertionError(
                        f"{result['outcome']} | {result['text'][:160]}"
                    )
                return expected

            return run

        checks.step(
            "0 the button follows voter-certificate-policy", button_follows_policy
        )
        checks.step("1 voter certificate logs in", voter_logs_in)
        checks.step(
            "2 no certificate",
            rejected(None, MESSAGES["certNotProvided"], "2-no-certificate.png"),
        )
        checks.step(
            "3 trusted CA, no matching voter",
            rejected("nobody", MESSAGES["userNotFound"], "3-unknown-voter.png"),
        )
        checks.step(
            "4 CA no subflow classifies",
            rejected("voter-unknown", MESSAGES["accessDenied"], "4-unknown-ca.png"),
        )
        checks.step(
            "5 rogue CA with the trusted CA's name",
            rejected("voter-rogue", MESSAGES["accessDenied"], "5-rogue-ca.png"),
        )
        browser.close()

    print(f"screenshots: {SHOTS}")
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
