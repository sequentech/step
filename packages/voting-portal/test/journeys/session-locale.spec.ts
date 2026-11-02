// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath} from "./fixtures"

for (const {name, query, cookie, expected, title} of [
    {
        name: "forced event default",
        query: "",
        cookie: "",
        expected: "fr",
        title: "Conseil communal",
    },
    {
        name: "saved user language",
        query: "",
        cookie: "es",
        expected: "es",
        title: "Consejo comunitario",
    },
    {
        name: "explicit URL language",
        query: "?lang=en",
        cookie: "es",
        expected: "en",
        title: "Community Council",
    },
]) {
    test(`${name} selects both the portal and identity-provider locale`, async ({
        page,
        context,
        portal,
    }) => {
        Object.assign(portal.data.event.presentation.language_conf, {
            default_language_code: "fr",
            enabled_language_codes: ["en", "es", "fr"],
            language_detection_policy: "force-default",
        })
        Object.assign(portal.data.election.presentation.i18n, {
            fr: {name: "Conseil communal", description: "Choisissez votre représentant."},
            es: {name: "Consejo comunitario", description: "Elija a su representante."},
        })
        portal.publish()
        if (cookie)
            await context.addCookies([{name: "USER_LANGUAGE", value: cookie, url: portal.origin}])

        await page.goto(`${portal.origin}${eventPath}${query}`)
        await expect(page.getByRole("heading", {name: title})).toBeVisible()
        await expect(page.locator("html")).toHaveAttribute("lang", expected)
        expect(portal.oidc.authorizations).toHaveLength(1)
        expect(portal.oidc.authorizations[0].params.ui_locales).toBe(expected)
        expect(portal.graphql.callsTo("InsertCastVote")).toEqual([])
    })
}

test("refreshing a live session uses the new bearer without another login", async ({
    page,
    portal,
}) => {
    // Refresh on the configured ten-minute interval while the issued token still
    // has five minutes left; expiry/logout is covered by a separate journey.
    portal.settings.KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS = 600
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    const original = portal.oidc.lastIssued()!
    expect(portal.oidc.tokenRequests).toHaveLength(1)

    portal.now += 600000
    await page.clock.fastForward(600000)
    await expect.poll(() => portal.oidc.tokenRequests.length).toBe(2)
    const refreshed = portal.oidc.tokenRequests[1]
    expect(refreshed).toMatchObject({
        grantType: "refresh_token",
        refreshToken: original.refresh_token,
        clientId: "voting-portal",
        status: 200,
    })
    expect(refreshed.issued?.access_token).not.toBe(original.access_token)
    expect(portal.oidc.authorizations).toHaveLength(1)
    expect(portal.oidc.logouts).toEqual([])

    portal.now += 60000
    await page.clock.fastForward(60000)
    await expect
        .poll(() => portal.graphql.callsTo("GetVoterStatus").at(-1)?.headers.authorization)
        .toBe(`Bearer ${refreshed.issued?.access_token}`)
    await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
})
