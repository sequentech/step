// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../fixtures"
import {EVENT_ID, EVENT_ROLES, listOf, mockEvent, openEvent} from "./data"

test.use({roles: [...EVENT_ROLES, "admin-dashboard-view", "admin-ip-address-view"]})

const IP_ROWS = [
    {
        id: "ip-1",
        ip: "203.0.113.7",
        country: "ES",
        election_presentation: {alias: "Council seats"},
        vote_count: 12,
        voters_id: ["voter-1"],
    },
    {
        id: "ip-2",
        ip: null,
        country: null,
        election_presentation: {alias: "Mayor"},
        vote_count: 3,
        voters_id: ["voter-2"],
    },
]

test("lists the event's votes per IP address on the dashboard", async ({page, portal}) => {
    mockEvent(portal)
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
    portal.graphql.on("GetCastVotesByIp", () => ({
        data: {get_top_votes_by_ip: {items: IP_ROWS, total: {aggregate: {count: 2}}}},
    }))
    await openEvent(page, portal)
    await expect(page.getByText("IP Addresses", {exact: true})).toBeVisible()
    const first = page.getByRole("row").filter({hasText: "203.0.113.7"})
    await expect(first).toContainText("ES")
    await expect(first).toContainText("12")
    await expect(first).toContainText("Council seats")
    // Rows without an address or country fall back to a dash.
    const anonymous = page.getByRole("row").filter({hasText: "Mayor"})
    await expect(anonymous.getByRole("cell", {name: "-", exact: true})).toHaveCount(2)
    expect(portal.graphql.callsTo("GetCastVotesByIp")[0].variables).toEqual({
        election_event_id: EVENT_ID,
        limit: 10,
        offset: 0,
        ip: null,
        country: null,
        election_id: null,
    })
})
