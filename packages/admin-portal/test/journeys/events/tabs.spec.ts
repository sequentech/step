// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {
    electionEvent,
    EVENT_ID,
    EVENT_ROLES,
    EVENT_URL,
    listOf,
    mockEvent,
    openEvent,
    TENANT_ID,
} from "./data"

const TAB_ROLES = [
    "election-event-data-tab",
    "election-event-voters-tab",
    "election-event-areas-tab",
    "election-event-keys-tab",
    "admin-ceremony",
    "election-event-tally-tab",
    "tally-read",
    "tally-sheet-import-view",
    "election-event-publish-tab",
    "election-event-tasks-tab",
    "election-event-logs-tab",
    "election-event-scheduled-tab",
    "election-event-reports-tab",
    "election-event-approvals-tab",
    "election-event-ivr-tab",
    "election-event-cas-tab",
]
const ALL_TABS = [
    "Data",
    "IVR",
    "Localization",
    "Voters",
    "Areas",
    "Keys",
    "Certificates",
    "Tally",
    "Tally sheet imports",
    "Publish",
    "Tasks",
    "Logs",
    "Scheduled Events",
    "Reports",
    "Approvals",
]

function tabs(page: Page) {
    return page.getByRole("tablist").first().getByRole("tab")
}

async function firstCall(portal: AdminPortal, operation: string) {
    await expect.poll(() => portal.graphql.callsTo(operation).length).toBeGreaterThan(0)
    return portal.graphql.callsTo(operation)[0].variables
}

function operationalLists(portal: AdminPortal) {
    for (const resource of [
        "sequent_backend_tasks_execution",
        "sequent_backend_scheduled_event",
        "sequent_backend_applications",
        "sequent_backend_area",
    ])
        portal.graphql.on(resource, listOf(resource, []))
    portal.graphql.on("listElectoralLog", () => ({
        data: {listElectoralLog: {items: [], total: {aggregate: {count: 0}}}},
    }))
    portal.graphql.on("getUserProfileAttributes", () => ({
        data: {get_user_profile_attributes: []},
    }))
}

test.describe("with every tab permission", () => {
    test.use({roles: [...EVENT_ROLES, ...TAB_ROLES]})

    test("shows every tab for a telephone event with voter certificates", async ({
        page,
        portal,
    }) => {
        mockEvent(
            portal,
            electionEvent(
                {voting_channels: {online: true, telephone: true}},
                {voter_certificate_policy: "enabled"}
            )
        )
        await openEvent(page, portal)
        await expect(tabs(page)).toHaveText(ALL_TABS)
        await expect(page.getByRole("tab", {name: "Data", exact: true})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(page.getByText("Election event configuration.", {exact: true})).toBeVisible()
    })

    test("hides the channel- and policy-dependent tabs by default", async ({page, portal}) => {
        mockEvent(portal)
        await openEvent(page, portal)
        await expect(tabs(page)).toHaveText(
            ALL_TABS.filter((tab) => !["IVR", "Certificates"].includes(tab))
        )
    })

    test("keeps only the audit tabs of a locked-down event", async ({page, portal}) => {
        operationalLists(portal)
        mockEvent(
            portal,
            electionEvent(
                {voting_channels: {online: true, telephone: true}},
                {locked_down: "locked-down", voter_certificate_policy: "enabled"}
            )
        )
        await openEvent(page, portal)
        await expect(tabs(page)).toHaveText(["IVR", "Voters", "Certificates", "Logs", "Reports"])
    })
})

test.describe("operational tabs", () => {
    test.use({
        roles: [
            ...EVENT_ROLES,
            "election-event-areas-tab",
            "election-event-tasks-tab",
            "election-event-logs-tab",
            "election-event-scheduled-tab",
            "election-event-approvals-tab",
            "tasks-read",
            "logs-read",
        ],
    })

    test("scopes each tab's list to the event", async ({page, portal}) => {
        operationalLists(portal)
        mockEvent(portal)
        await openEvent(page, portal)
        await expect(tabs(page)).toHaveText([
            "Areas",
            "Tasks",
            "Logs",
            "Scheduled Events",
            "Approvals",
        ])
        await expect(page.getByText("No Areas yet.", {exact: true})).toBeVisible()
        await page.getByRole("tab", {name: "Tasks", exact: true}).click()
        await expect(page.getByText("Tasks Execution", {exact: true})).toBeVisible()
        expect(await firstCall(portal, "sequent_backend_tasks_execution")).toEqual({
            where: {_and: [{election_event_id: {_eq: EVENT_ID}}]},
            limit: 10,
            offset: 0,
            order_by: {start_at: "desc"},
        })
        await page.getByRole("tab", {name: "Logs", exact: true}).click()
        await expect(page.getByText("No Electoral logs yet.", {exact: true})).toBeVisible()
        expect(await firstCall(portal, "listElectoralLog")).toEqual({
            election_event_id: EVENT_ID,
            limit: 10,
            offset: 0,
            order_by: {id: "desc"},
            filter: {},
        })
        await page.getByRole("tab", {name: "Scheduled Events", exact: true}).click()
        await expect(page.getByText("Scheduled Events", {exact: true}).last()).toBeVisible()
        expect(await firstCall(portal, "sequent_backend_scheduled_event")).toEqual({
            where: {
                _and: [
                    {election_event_id: {_eq: EVENT_ID}},
                    {tenant_id: {_eq: TENANT_ID}},
                    {archived_at: {_is_null: true}},
                    {_or: [{event_payload: {_contains: {election_id: null}}}]},
                ],
            },
            limit: 10,
            offset: 0,
            order_by: [{id: "asc"}, {id: "asc"}],
        })
        await page.getByRole("tab", {name: "Approvals", exact: true}).click()
        await expect(page.getByText(/No Sequent backend applications/)).toBeVisible()
        expect(await firstCall(portal, "sequent_backend_applications")).toEqual({
            where: {
                _and: [{status: {_ilike: "%pending%"}}, {election_event_id: {_eq: EVENT_ID}}],
            },
            limit: 10,
            offset: 0,
            order_by: {created_at: "desc"},
        })
    })

    test("opens the tab named by the tabIndex query and cleans the URL", async ({page, portal}) => {
        operationalLists(portal)
        mockEvent(portal)
        await page.goto(`${portal.origin}${EVENT_URL}?tabIndex=2&lang=en`)
        await expect(page.getByRole("tab", {name: "Logs", exact: true})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect.poll(() => new URL(page.url()).searchParams.has("tabIndex")).toBe(false)
        expect(new URL(page.url()).pathname).toBe(EVENT_URL)
    })
})

test("redirects the event list to the tenant's newest active event", async ({page, portal}) => {
    mockEvent(portal)
    await page.goto(`${portal.origin}/sequent_backend_election_event?lang=en`)
    await expect(page).toHaveURL(`${portal.origin}${EVENT_URL}`)
    expect(portal.graphql.callsTo("sequent_backend_election_event")[0].variables).toEqual({
        where: {_and: [{tenant_id: {_eq: TENANT_ID}}, {is_archived: {_eq: false}}]},
        limit: 25,
        offset: 0,
        order_by: {created_at: "desc"},
    })
})

test.describe("without write permission", () => {
    test.use({roles: ["admin-user", "election-event-read", "election-read"]})

    test("offers no create or import on an empty event list", async ({page, portal}) => {
        await page.goto(`${portal.origin}/sequent_backend_election_event?lang=en`)
        await expect(page.getByText("No Election Event yet", {exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Add", exact: true})).toHaveCount(0)
        await expect(page.getByRole("button", {name: "Import", exact: true})).toHaveCount(0)
    })
})

test.describe("ceremony, tally and publication tabs", () => {
    test.use({
        roles: [
            ...EVENT_ROLES,
            "election-event-keys-tab",
            "admin-ceremony",
            "election-event-tally-tab",
            "tally-read",
            "tally-sheet-import-view",
            "election-event-publish-tab",
            "publish-read",
            "election-event-reports-tab",
            "report-read",
        ],
    })

    test("renders each tab's empty state for a new event", async ({page, portal}) => {
        mockEvent(portal)
        portal.graphql.on("ListKeysCeremony", () => ({
            data: {list_keys_ceremony: {items: [], total: {aggregate: {count: 0}}}},
        }))
        portal.graphql.on("TrusteeNames", () => ({data: {sequent_backend_trustee: []}}))
        for (const resource of [
            "sequent_backend_keys_ceremony",
            "sequent_backend_tally_session",
            "sequent_backend_tally_session_execution",
            "sequent_backend_tally_sheet_import",
            "sequent_backend_ballot_publication",
            "sequent_backend_template",
            "sequent_backend_report",
        ])
            portal.graphql.on(resource, listOf(resource, []))
        await openEvent(page, portal)
        await expect(tabs(page)).toHaveText([
            "Keys",
            "Tally",
            "Tally sheet imports",
            "Publish",
            "Reports",
        ])
        const emptyStates = {
            "Keys": "No Key Ceremony yet.",
            "Tally": "No Tally yet.",
            "Tally sheet imports": "No tally sheet imports yet.",
            "Publish": "No Publication Yet.",
            "Reports": "No Reports yet.",
        }
        for (const [tab, empty] of Object.entries(emptyStates)) {
            await page.getByRole("tab", {name: tab, exact: true}).click()
            await expect(page.getByText(empty, {exact: true})).toBeVisible()
        }
        expect(await firstCall(portal, "ListKeysCeremony")).toEqual({electionEventId: EVENT_ID})
    })

    test("opens tally sheet imports from a shared tabId link", async ({page, portal}) => {
        mockEvent(portal)
        portal.graphql.on("ListKeysCeremony", () => ({
            data: {list_keys_ceremony: {items: [], total: {aggregate: {count: 0}}}},
        }))
        portal.graphql.on("TrusteeNames", () => ({data: {sequent_backend_trustee: []}}))
        for (const resource of [
            "sequent_backend_keys_ceremony",
            "sequent_backend_tally_sheet_import",
        ])
            portal.graphql.on(resource, listOf(resource, []))
        await page.goto(`${portal.origin}${EVENT_URL}?tabId=tally-sheet-imports&lang=en`)
        await expect(
            page.getByRole("tab", {name: "Tally sheet imports", exact: true})
        ).toHaveAttribute("aria-selected", "true")
        await expect(page.getByText("No tally sheet imports yet.", {exact: true})).toBeVisible()
        await expect.poll(() => new URL(page.url()).searchParams.has("tabId")).toBe(false)
    })
})
