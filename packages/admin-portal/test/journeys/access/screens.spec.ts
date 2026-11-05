// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {expectRole} from "./data"

test.describe("settings administrator", () => {
    test.use({roles: ["admin-user", "tenant-write", "settings-menu", "election-type-read"]})

    test("opens settings on the election types tab", async ({page, portal}) => {
        portal.graphql.on("sequent_backend_election_type", () => ({
            data: {
                sequent_backend_election_type: [],
                sequent_backend_election_type_aggregate: {aggregate: {count: 0}},
            },
        }))
        await page.goto(`${portal.origin}/settings?lang=en`)
        await expect(page.getByText("General Configuration", {exact: true})).toBeVisible()
        await expect(page.getByRole("tab", {name: "ELECTION TYPES"})).toBeVisible()
        await expect
            .poll(() => portal.graphql.callsTo("sequent_backend_election_type").length)
            .toBeGreaterThan(0)
        expectRole(portal, "sequent_backend_election_type", "election-type-read")
    })
})

test.describe("settings reader without tenant-write", () => {
    test.use({roles: ["admin-user", "settings-menu"]})

    test("is told settings are out of reach", async ({page, portal}) => {
        await page.goto(`${portal.origin}/settings?lang=en`)
        await expect(
            page.getByText("You don't have permission to access settings.", {exact: true})
        ).toBeVisible()
        expect(portal.graphql.callsTo("sequent_backend_election_type")).toHaveLength(0)
    })
})

test("shows the messages placeholder", async ({page, portal}) => {
    await page.goto(`${portal.origin}/messages?lang=en`)
    await expect(page.getByText("Messages", {exact: true})).toBeVisible()
})

test("lists this tenant's scheduled notifications", async ({page, portal}) => {
    const notification = {
        id: "12121212-1212-4212-8212-121212121201",
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
    }
    let rows: Record<string, unknown>[] = []
    portal.graphql.on("sequent_backend_notification", () => ({
        data: {
            sequent_backend_notification: rows,
            sequent_backend_notification_aggregate: {aggregate: {count: rows.length}},
        },
    }))
    await page.goto(`${portal.origin}/sequent_backend_notification?lang=en`)
    await expect(page.getByText("No Scheduled Events yet.", {exact: true})).toBeVisible()
    rows = [notification]
    await page.reload()
    await expect(page.getByRole("columnheader", {name: "Created at"})).toBeVisible()
    await expect(page.getByText("Invalid Date")).toHaveCount(0)
    // Without an event in context the filter keeps an empty, match-all event condition.
    expect(portal.graphql.callsTo("sequent_backend_notification")[0].variables.where).toEqual({
        _and: [{election_event_id: {}}, {tenant_id: {_eq: TENANT_ID}}],
    })
})
