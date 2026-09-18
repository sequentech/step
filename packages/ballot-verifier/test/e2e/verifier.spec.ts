// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../../../e2e/fixtures"
import {login} from "../../../voting-portal/test/load/flow"

test("verifier rejects a malformed ballot @smoke", async ({page, fixture}) => {
    await page.goto(fixture.verifierUrl)
    await login(page, {username: `${fixture.usernamePrefix}3`, password: fixture.password})
    await expect(page.locator('input[type="file"]')).toBeAttached()
    await page
        .locator('input[type="file"]')
        .setInputFiles({
            name: "invalid-ballot.txt",
            mimeType: "text/plain",
            buffer: Buffer.from('{"invalid":"synthetic-fixture"}'),
        })
    await expect(page.getByRole("button", {name: /next/i})).toBeDisabled()
})
