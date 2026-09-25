// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath} from "./fixtures"

for (const fault of ["settings", "wasm", "token"]) {
    test(`${fault} startup failure leaves an actionable error instead of an endless loader`, async ({
        page,
        portal,
    }) => {
        await page.goto(`${portal.origin}${eventPath}?lang=en`)
        await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
        const bootstrapCalls = portal.graphql.callsTo("GetVoterStatus").length
        let failures = 0
        if (fault === "token")
            portal.oidc.tokenFailure = {status: 500, body: "Synthetic identity provider failure"}
        else
            await page.route(
                fault === "settings" ? "**/global-settings.json" : "**/*.wasm",
                async (route) => {
                    failures++
                    await route.fulfill({
                        status: fault === "settings" ? 500 : 404,
                        contentType: fault === "settings" ? "text/plain" : "application/wasm",
                        body: fault === "settings" ? "Synthetic settings failure" : "",
                    })
                }
            )
        await page.reload()
        if (fault === "token")
            await expect.poll(() => portal.oidc.tokenRequests.at(-1)?.status).toBe(500)
        else await expect.poll(() => failures).toBe(1)
        await expect(
            fault === "token" ? page.getByRole("progressbar").first() : page.getByRole("status")
        ).toBeVisible()
        await expect(page.getByRole("button", {name: /click to vote/i})).toHaveCount(0)
        expect(portal.graphql.callsTo("GetVoterStatus")).toHaveLength(bootstrapCalls)
        test.fail(
            true,
            `${fault} bootstrap failure has no recovery UI; the error/retry UX needs a product decision`
        )
        await expect(page.getByRole("alert")).toBeVisible({timeout: 1000})
    })
}
