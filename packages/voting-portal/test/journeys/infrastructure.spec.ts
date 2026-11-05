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
        await expect(page.getByRole("button", {name: /click to vote/i})).toHaveCount(0)
        expect(portal.graphql.callsTo("GetVoterStatus")).toHaveLength(bootstrapCalls)
        await expect(page.getByRole("alert")).toBeVisible()
        await expect(page.getByRole("button", {name: "Try again", exact: true})).toBeVisible()
        if (fault === "token") portal.oidc.tokenFailure = undefined
        else await page.unroute(fault === "settings" ? "**/global-settings.json" : "**/*.wasm")
        await page.getByRole("button", {name: "Try again", exact: true}).click()
        await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    })
}
