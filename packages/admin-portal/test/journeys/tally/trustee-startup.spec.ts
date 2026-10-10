// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {expect} from "@playwright/test"
import {test} from "../fixtures"

test("provides native trustee setup and custody commands", async ({page, portal}) => {
    await page.goto(`${portal.origin}/trustee?lang=en`)
    await expect(page.getByRole("heading", {name: "Wbraid trustee"})).toBeVisible()
    await expect(page.getByText(/trustee generate --trustee-config/)).toBeVisible()
    await expect(page.getByText(/trustee attest --request/)).toBeVisible()
})
