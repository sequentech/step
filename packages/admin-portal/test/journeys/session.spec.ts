// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect, SECOND_TENANT_ID, TENANT_ID} from "./fixtures"

async function boot(page: import("@playwright/test").Page, origin: string) {
    await page.goto(`${origin}/?lang=en`)
    await expect(page.getByText("No Election Event yet", {exact: true})).toBeVisible()
}
test("refreshes access credentials before expiry and sends the renewed bearer on later reads", async ({
    page,
    portal,
}) => {
    await boot(page, portal.origin)
    const initial = portal.oidc.lastIssued()!
    portal.now += 841000
    await page.clock.fastForward(841000)
    await expect
        .poll(
            () =>
                portal.oidc.tokenRequests.filter((request) => request.grantType === "refresh_token")
                    .length
        )
        .toBe(1)
    const current = portal.oidc.lastIssued()!
    expect(current.access_token).not.toBe(initial.access_token)
    expect(portal.oidc.verifyAccessToken(current.access_token)).toBeTruthy()
    await page.getByText("Archived", {exact: true}).click()
    await expect
        .poll(() => portal.graphql.callsTo("election_events_tree").at(-1)?.variables.isArchived)
        .toBe(true)
    expect(portal.graphql.callsTo("election_events_tree").at(-1)?.headers.authorization).toBe(
        `Bearer ${current.access_token}`
    )
    expect(await page.evaluate(() => localStorage.getItem("token"))).toContain(current.access_token)
    expect(portal.oidc.logouts).toHaveLength(0)
})
test("confirmed logout clears the local credential and returns to provider sign-in", async ({
    page,
    portal,
}) => {
    await boot(page, portal.origin)
    await page.getByRole("button", {name: "Welcome, synthetic-admin"}).click()
    await page.getByRole("menuitem", {name: "Logout", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await expect(dialog).toBeVisible()
    await dialog.getByRole("button", {name: "OK", exact: true}).click()
    await expect.poll(() => portal.oidc.logouts.length).toBe(1)
    expect(portal.oidc.logouts[0]).toMatchObject({
        realm: `tenant-${TENANT_ID}`,
        params: {post_logout_redirect_uri: portal.origin},
    })
    await expect(page.getByRole("button", {name: "Sign in", exact: true})).toBeVisible()
    expect(await page.evaluate(() => localStorage.getItem("token"))).toBeNull()
})
test("tenant selection authenticates in the selected realm and scopes its event list", async ({
    page,
    portal,
}) => {
    await page.goto(`${portal.origin}/tenant?lang=en`)
    await page.getByRole("textbox", {name: "Tenant Name", exact: true}).fill("second")
    const discovery = page.waitForRequest((request) =>
        request
            .url()
            .endsWith(`/realms/tenant-${SECOND_TENANT_ID}/.well-known/openid-configuration`)
    )
    await page.getByRole("button", {name: "Continue", exact: true}).click()
    expect((await discovery).method()).toBe("GET")
    await expect(page.getByText("No Election Event yet", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("GetTenantBySlug")[0].variables).toEqual({slug: "second"})
    expect(portal.oidc.authorizations.at(-1)?.realm).toBe(`tenant-${SECOND_TENANT_ID}`)
    expect(await page.evaluate(() => localStorage.getItem("selected-tenant-id"))).toBe(
        SECOND_TENANT_ID
    )
    expect(portal.graphql.callsTo("sequent_backend_election_event")[0].variables).toMatchObject({
        where: {_and: [{tenant_id: {_eq: SECOND_TENANT_ID}}, {is_archived: {_eq: false}}]},
    })
    expect(
        portal.graphql
            .callsTo("election_events_tree")
            .every((call) => call.variables.tenantId === SECOND_TENANT_ID)
    ).toBe(true)
})

test("switches from an authenticated tenant through logout and a fresh realm login", async ({
    page,
    portal,
}) => {
    await boot(page, portal.origin)
    await page.goto(`${portal.origin}/tenant?lang=en`)
    const dialog = page.getByRole("dialog")
    await expect(
        dialog.getByText("Do you want to stay connected to this tenant or logout?", {exact: true})
    ).toBeVisible()
    await dialog.getByRole("button", {name: "Logout", exact: true}).click()
    await expect.poll(() => portal.oidc.logouts.length).toBe(1)
    await expect(page.getByRole("textbox", {name: "Tenant Name", exact: true})).toBeVisible()
    await page.getByRole("textbox", {name: "Tenant Name", exact: true}).fill("second")
    await page.getByRole("button", {name: "Continue", exact: true}).click()
    await expect(page.getByRole("button", {name: "Sign in", exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Sign in", exact: true}).click()
    await expect(page.getByText("No Election Event yet", {exact: true})).toBeVisible()
    expect(portal.oidc.authorizations.at(-1)?.realm).toBe(`tenant-${SECOND_TENANT_ID}`)
    await expect
        .poll(() => portal.graphql.callsTo("sequent_backend_election_event").at(-1)?.variables)
        .toMatchObject({
            where: {_and: [{tenant_id: {_eq: SECOND_TENANT_ID}}, {is_archived: {_eq: false}}]},
        })
    expect(portal.oidc.logouts[0].params.post_logout_redirect_uri).toBe(`${portal.origin}/tenant`)
})
