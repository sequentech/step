// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Page} from "@playwright/test"
import {expect, eventPath, type Portal} from "./fixtures"

export async function review(page: Page, portal: Portal, preview = false, query = "?lang=en") {
    await page.goto(`${portal.origin}${preview ? portal.previewPath : eventPath + query}`)
    await page.getByRole("button", {name: /click to vote/i}).click()
    if (preview)
        await page.getByRole("button", {name: "I understand that my vote will not be cast"}).click()
    await page.getByRole("button", {name: "Start Voting", exact: true}).click()
    await page.getByRole("checkbox", {name: /Alice Example/}).check()
    await expect(page.getByRole("checkbox", {name: /Alice Example/})).toBeChecked()
    await expect(page.getByRole("checkbox", {name: /Bob Example/})).not.toBeChecked()
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await expect(page.getByRole("heading", {name: /^Review your ballot/})).toBeVisible()
    await expect(page.getByText("Alice Example", {exact: true})).toBeVisible()
}
