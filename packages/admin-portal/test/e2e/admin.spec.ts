// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../../../e2e/fixtures"
import {login} from "../../../voting-portal/test/load/flow"

test("administrator can inspect the freshly provisioned election @smoke @probe", async ({
    page,
    fixture,
}) => {
    await page.goto(fixture.adminUrl)
    await login(page, {username: fixture.adminUsername, password: fixture.adminPassword})
    await expect(page).toHaveURL(/sequent_backend_election_event/)
    await expect(page.getByRole("main")).toBeVisible()
    await expect(page.locator(`a[href*="${fixture.eventId}"]`).first()).toBeVisible()
})
