// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect, TENANT_ID} from "./fixtures"

async function boot(page: import("@playwright/test").Page, origin: string) {
    await page.goto(`${origin}/?lang=en`)
    await expect(page.getByText("No Election Event yet", {exact: true})).toBeVisible()
}

test("authenticates and requests only this tenant with the election reader role", async ({
    page,
    portal,
}) => {
    await boot(page, portal.origin)
    await expect(page.getByRole("button", {name: "Welcome, synthetic-admin"})).toBeVisible()
    await expect(page.getByRole("button", {name: "Add", exact: true})).toBeVisible()
    await expect(page.getByRole("button", {name: "Import", exact: true})).toBeVisible()
    const call = portal.graphql.callsTo("sequent_backend_election_event")[0]
    expect(call.variables).toEqual({
        where: {_and: [{tenant_id: {_eq: TENANT_ID}}, {is_archived: {_eq: false}}]},
        limit: 25,
        offset: 0,
        order_by: {created_at: "desc"},
    })
    expect(call.headers["x-hasura-role"]).toBe("election-event-read")
    expect(portal.graphql.callsTo("IntrospectionQuery")).toHaveLength(1)
    expect(portal.oidc.tokenRequests[0].grantType).toBe("authorization_code")
})

test.describe("read-only election operator", () => {
    test.use({roles: ["admin-user", "election-event-read"]})
    test("hides creation and import actions without the writer role", async ({page, portal}) => {
        await boot(page, portal.origin)
        await expect(page.getByRole("button", {name: "Add", exact: true})).toHaveCount(0)
        await expect(page.getByRole("button", {name: "Import", exact: true})).toHaveCount(0)
        expect(portal.graphql.calls.some((call) => call.query.trim().startsWith("mutation"))).toBe(
            false
        )
    })
})

test("switches the event tree between archived and active tenant scopes", async ({
    page,
    portal,
}) => {
    await boot(page, portal.origin)
    await page.getByText("Archived", {exact: true}).click()
    await expect
        .poll(() => portal.graphql.callsTo("election_events_tree").at(-1)?.variables)
        .toEqual({tenantId: TENANT_ID, isArchived: true})
    await page.getByText("Active", {exact: true}).click()
    await expect
        .poll(() => portal.graphql.callsTo("election_events_tree").at(-1)?.variables)
        .toEqual({tenantId: TENANT_ID, isArchived: false})
})

test("opens an event import without uploading or mutating before file selection", async ({
    page,
    portal,
}) => {
    await boot(page, portal.origin)
    await page.getByRole("button", {name: "Import", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await expect(dialog.getByText("Import Election Event", {exact: true})).toBeVisible()
    await expect(dialog.getByRole("button", {name: "Import", exact: true})).toBeDisabled()
    await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(dialog).toHaveCount(0)
    expect(portal.graphql.calls.some((call) => call.query.trim().startsWith("mutation"))).toBe(
        false
    )
})
