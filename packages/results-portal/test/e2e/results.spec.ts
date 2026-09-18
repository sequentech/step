// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../../../e2e/fixtures"

test("unpublished results do not expose an election result @smoke", async ({page, fixture}) => {
    const response = await page.goto(`${fixture.resultsUrl}/${fixture.eventId}`)
    expect(response?.ok()).toBe(true)
    await expect(page.getByText("Results not published yet", {exact: true})).toBeVisible()
})
