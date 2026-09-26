// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../fixtures"
import {EVENT_ID, EVENT_ROLES, FIXED_TIME, TENANT_ID, listOf, mockEvent} from "./data"
import {election} from "./ivr-data"

const ELECTION_ID = "50000000-0000-4000-8000-000000000001"
const PUBLICATION_ID = "50000000-0000-4000-8000-000000000002"
test.use({
    roles: [
        ...EVENT_ROLES,
        "election-publish-tab",
        "publish-read",
        "publish-write",
        "election-state-write",
        "publish-start-voting",
        "publish-pause-voting",
        "publish-stop-voting",
    ],
})

test("starts, pauses and closes only the selected election after gold login", async ({
    page,
    portal,
}) => {
    mockEvent(portal)
    let votingStatus = "NOT_STARTED"
    const current = () => ({
        ...election(ELECTION_ID, "North council"),
        status: {voting_status: votingStatus},
        presentation: {
            i18n: {en: {name: "North council"}},
            initialization_report_policy: "not-required",
        },
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: false},
    })
    portal.graphql.on(
        "sequent_backend_election",
        listOf("sequent_backend_election", () => [current()])
    )
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: [current()]}}))
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: []}}))
    portal.graphql.on(
        "sequent_backend_ballot_publication",
        listOf("sequent_backend_ballot_publication", [
            {
                id: PUBLICATION_ID,
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                election_id: ELECTION_ID,
                election_ids: [ELECTION_ID],
                is_generated: true,
                published_at: FIXED_TIME,
                created_at: "2026-01-15T11:00:00Z",
                last_updated_at: FIXED_TIME,
                annotations: {},
                labels: {},
            },
        ])
    )
    portal.graphql.on("UpdateElectionVotingStatus", ({variables}) => {
        votingStatus = String(variables.votingStatus)
        return {data: {update_election_voting_status: {election_id: ELECTION_ID}}}
    })
    await page.goto(`${portal.origin}/sequent_backend_election/${ELECTION_ID}?lang=en`)
    await expect(page.getByRole("tab", {name: "Publish", exact: true})).toHaveAttribute(
        "aria-selected",
        "true"
    )
    await expect(page.getByRole("cell", {name: PUBLICATION_ID, exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("sequent_backend_ballot_publication")[0].variables).toEqual({
        where: {
            _and: [
                {election_event_id: {_eq: EVENT_ID}},
                {election_ids: {_contains: [ELECTION_ID]}},
            ],
        },
        limit: 10,
        offset: 0,
        order_by: {created_at: "desc"},
    })
    for (const [index, action] of (["Start", "Start", "Pause", "Stop"] as const).entries()) {
        await page.getByRole("button", {name: `${action} Voting`, exact: true}).click()
        await page.getByRole("menuitem", {name: `${action} Online Voting`, exact: true}).click()
        const dialog = page.getByRole("dialog")
        await dialog
            .getByRole("button", {name: index === 0 ? "Cancel" : "Confirm", exact: true})
            .click()
        await expect(dialog).toHaveCount(0)
        await expect
            .poll(() => portal.graphql.callsTo("UpdateElectionVotingStatus").length)
            .toBe(index)
    }
    const calls = portal.graphql.callsTo("UpdateElectionVotingStatus")
    expect(calls.map(({variables}) => variables)).toEqual(
        ["OPEN", "PAUSED", "CLOSED"].map((votingStatus) => ({
            electionEventId: EVENT_ID,
            electionId: ELECTION_ID,
            votingStatus,
            votingChannel: ["ONLINE"],
        }))
    )
    expect(portal.graphql.callsTo("UpdateEventVotingStatus")).toHaveLength(0)
    expect(portal.oidc.authorizations.map(({requestedAcr}) => requestedAcr)).toEqual([
        undefined,
        "gold",
    ])
    for (const call of calls) {
        expect(call.headers["x-hasura-role"]).toBe("admin-user")
        expect(
            portal.oidc.verifyAccessToken(call.headers.authorization.replace(/^Bearer /, ""))
        ).toMatchObject({
            "acr": "gold",
            "https://hasura.io/jwt/claims": {"x-hasura-tenant-id": TENANT_ID},
        })
    }
    for (const action of ["Start", "Pause", "Stop"])
        await expect(
            page.getByRole("button", {name: `${action} Voting`, exact: true})
        ).toBeDisabled()
})
