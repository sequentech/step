// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect, type AdminPortal} from "../fixtures"
import {EVENT_ID, EVENT_ROLES, electionEvent, listOf, mockEvent, openEvent} from "./data"

const DASHBOARD_ROLES = [...EVENT_ROLES, "admin-dashboard-view", "admin-ip-address-view"]
test.use({roles: [...DASHBOARD_ROLES, "forensic-read"]})

const IP_ROWS = [
    {
        id: "ip-1",
        ip: "203.0.113.7",
        country: "ES",
        vote_count: 12,
        pseudonyms: ["7QKM-2HXD"],
        shared: false,
        first_ballot: "2028-05-08T03:00:00Z",
        last_ballot: "2028-05-08T04:00:00Z",
    },
    {
        id: "ip-2",
        ip: "198.51.100.7",
        country: null,
        vote_count: 3,
        pseudonyms: ["9KMP-2RST"],
        shared: false,
        first_ballot: "2028-05-08T03:00:00Z",
        last_ballot: "2028-05-08T04:00:00Z",
    },
]

function mockDashboard(
    portal: AdminPortal,
    networkPolicy: "forensic-store" | "do-not-store" = "forensic-store"
) {
    mockEvent(
        portal,
        electionEvent(
            {},
            {
                ballot_identity: "pseudonym",
                network_data_policy: networkPolicy,
            }
        )
    )
    portal.graphql.on("GetElectionEventStats", () => ({
        data: {
            getElectionEventStats: {
                total_eligible_voters: 0,
                total_distinct_voters: 0,
                voters_by_channel: [],
                total_areas: 0,
                total_elections: 0,
                votes_per_day: [],
            },
            sequent_backend_election_event: [{statistics: {}}],
        },
    }))
    portal.graphql.on("ListKeysCeremony", () => ({
        data: {list_keys_ceremony: {items: [], total: {aggregate: {count: 0}}}},
    }))
    portal.graphql.on("sequent_backend_tally_session", listOf("sequent_backend_tally_session", []))
    portal.graphql.on("ForensicNetworkSummary", () => ({
        data: {forensic_network_summary: {payload: {items: IP_ROWS, total: 2}}},
    }))
}

test("lists the event's forensic ballot addresses on the dashboard", async ({page, portal}) => {
    mockDashboard(portal)
    await openEvent(page, portal)
    await expect(page.getByText("IP Addresses", {exact: true})).toBeVisible()
    const first = page.getByRole("row").filter({hasText: "203.0.113.7"})
    await expect(first).toContainText("ES")
    await expect(first).toContainText("12")
    await expect(first).toContainText("7QKM-2HXD")
    // An unknown country stays unknown; the forensic store always names an address.
    const anonymous = page.getByRole("row").filter({hasText: "198.51.100.7"})
    await expect(anonymous.getByText("—", {exact: true})).toBeVisible()
    const request = portal.graphql.callsTo("ForensicNetworkSummary")[0]
    expect(request.variables).toEqual({
        electionEventId: EVENT_ID,
        limit: 10,
        offset: 0,
        ip: null,
        country: null,
        electionId: null,
    })
    expect(request.headers["x-hasura-role"]).toBe("forensic-read")
    expect(portal.graphql.callsTo("GetCastVotesByIp")).toHaveLength(0)
})

test("does not query forensic records when the event does not store network data", async ({
    page,
    portal,
}) => {
    mockDashboard(portal, "do-not-store")
    await openEvent(page, portal)
    await expect(page.getByRole("tab", {name: "Dashboard", exact: true})).toBeVisible()
    await expect(page.getByText("IP Addresses", {exact: true})).toHaveCount(0)
    expect(portal.graphql.callsTo("ForensicNetworkSummary")).toHaveLength(0)
})

test.describe("without forensic permission", () => {
    test.use({roles: DASHBOARD_ROLES})
    test("does not query forensic records with only dashboard permissions", async ({
        page,
        portal,
    }) => {
        mockDashboard(portal)
        await openEvent(page, portal)
        await expect(page.getByRole("tab", {name: "Dashboard", exact: true})).toBeVisible()
        await expect(page.getByText("IP Addresses", {exact: true})).toHaveCount(0)
        expect(portal.graphql.callsTo("ForensicNetworkSummary")).toHaveLength(0)
    })
})
