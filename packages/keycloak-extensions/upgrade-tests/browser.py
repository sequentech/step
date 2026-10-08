# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Playwright helpers for the browser checks (needs the virtualenv from README.md)."""

import time
from pathlib import Path
from typing import Callable

from playwright.sync_api import Browser, BrowserContext, Error, Page

from common import OUT


def new_context(browser: Browser, **options) -> BrowserContext:
    defaults = {
        "viewport": {"width": 1280, "height": 900},
        "locale": "en-US",
        # Headless Chromium otherwise reports Etc/Unknown, which the backend rejects.
        "timezone_id": "Europe/Madrid",
    }
    return browser.new_context(**{**defaults, **options})


def screenshot_dir(name: str) -> Path:
    directory = OUT / "screenshots" / name
    directory.mkdir(parents=True, exist_ok=True)
    return directory


def screenshot(page: Page, path: Path) -> str:
    try:
        page.wait_for_load_state("networkidle", timeout=15000)
    except Error:
        pass
    page.screenshot(path=str(path), full_page=True)
    return path.name


def body_text(page: Page) -> str:
    try:
        return " ".join(page.locator("body").inner_text(timeout=2000).split())
    except Error:
        return ""


def wait_for_outcome(
    page: Page, outcomes: dict[str, Callable[[Page], bool]], timeout_seconds: int = 60
) -> str:
    """Polls until one of the named conditions holds; returns its name, or "timeout"."""
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        for name, holds in outcomes.items():
            try:
                if holds(page):
                    return name
            except Error:
                pass
        page.wait_for_timeout(250)
    return "timeout"
