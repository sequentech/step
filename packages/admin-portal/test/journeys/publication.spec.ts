// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "./fixtures"

const PUBLICATION_ID = "88888888-8888-4888-8888-888888888888"
const TASK_ID = "99999999-9999-4999-8999-999999999999"
const roles = [
    "admin-user",
    "election-event-read",
    "election-read",
    "election-event-publish-tab",
    "publish-read",
    "publish-write",
    "publish-create",
    "publish-changes",
    "election-state-write",
    "publish-start-voting",
    "publish-pause-voting",
    "publish-stop-voting",
]
test.use({roles})

function publicationWorkflow(portal: PortalServices) {
    let generated = false
    let started = false
    let published = false
    let votingStatus = "NOT_STARTED"
    const event = () => ({
        id: IDS.event,
        tenant_id: TENANT_ID,
        name: "Council publication",
        description: "Synthetic publication lifecycle",
        encryption_protocol: "RSA256",
        is_archived: false,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        elections: [],
        elections_aggregate: {aggregate: {count: 0}, nodes: []},
        presentation: {
            i18n: {en: {name: "Council publication"}},
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
            initialization_report_policy: "not-required",
        },
        status: {voting_status: votingStatus},
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: false},
    })
    const publication = () => ({
        id: PUBLICATION_ID,
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        election_id: null,
        is_generated: generated,
        published_at: published ? FIXED_TIME : null,
        created_at: "2026-01-15T11:00:00Z",
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
    })
    const task = () => ({
        id: TASK_ID,
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        execution_status: generated ? "SUCCESS" : "IN_PROGRESS",
        type: "GENERATE_BALLOT_PUBLICATION",
        start_at: FIXED_TIME,
        end_at: generated ? FIXED_TIME : null,
        logs: [],
        annotations: {},
        executed_by_user: IDS.voter,
    })
    portal.settings.QUERY_POLL_INTERVAL_MS = 100
    portal.graphql.on("sequent_backend_election_event", () => ({
        data: {
            sequent_backend_election_event: [event()],
            sequent_backend_election_event_aggregate: {aggregate: {count: 1}},
        },
    }))
    portal.graphql.on("election_events_tree", () => ({
        data: {sequent_backend_election_event: [event()]},
    }))
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: []}}))
    portal.graphql.on("sequent_backend_ballot_publication", () => ({
        data: {
            sequent_backend_ballot_publication: started ? [publication()] : [],
            sequent_backend_ballot_publication_aggregate: {aggregate: {count: started ? 1 : 0}},
        },
    }))
    portal.graphql.on("GenerateBallotPublication", () => {
        started = true
        return {
            data: {
                generate_ballot_publication: {
                    ballot_publication_id: PUBLICATION_ID,
                    task_execution: task(),
                },
            },
        }
    })
    portal.graphql.on("GetTaskById", () => ({data: {sequent_backend_tasks_execution: [task()]}}))
    portal.graphql.on("GetBallotPublicationChange", () => ({
        data: {
            get_ballot_publication_changes: {
                previous: null,
                current: {
                    ballot_publication_id: PUBLICATION_ID,
                    ballot_styles: [{id: "council-style", ballot_eml: "Council ballot revision"}],
                },
            },
        },
    }))
    portal.graphql.on("PublishBallot", () => {
        published = true
        return {data: {publish_ballot: {ballot_publication_id: PUBLICATION_ID}}}
    })
    portal.graphql.on("UpdateEventVotingStatus", ({variables}) => {
        votingStatus = String(variables.votingStatus)
        return {data: {update_event_voting_status: {election_event_id: IDS.event}}}
    })
    return {
        completeGeneration: () => {
            generated = true
        },
    }
}

async function channelDialog(page: Page, action: "Start" | "Pause" | "Stop") {
    await page.getByRole("button", {name: `${action} Voting`, exact: true}).click()
    await page.getByRole("menuitem", {name: `${action} Online Voting`, exact: true}).click()
    const dialog = page.getByRole("dialog")
    await expect(dialog).toBeVisible()
    return dialog
}

test("generates and publishes after gold login, then starts, pauses and closes online voting", async ({
    page,
    portal,
}) => {
    const workflow = publicationWorkflow(portal)
    await page.goto(`${portal.origin}/sequent_backend_election_event/${IDS.event}?lang=en`)
    await expect(page.getByRole("tab", {name: "Publish", exact: true})).toHaveAttribute(
        "aria-selected",
        "true"
    )
    await expect(page.getByText("No Publication Yet.", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("sequent_backend_ballot_publication")[0].variables).toEqual({
        where: {
            _and: [{election_event_id: {_eq: IDS.event}}, {election_id: {_is_null: true}}],
        },
        limit: 10,
        offset: 0,
        order_by: {created_at: "desc"},
    })
    expect(portal.graphql.callsTo("GenerateBallotPublication")).toHaveLength(0)
    await page.getByRole("button", {name: "Generate Publication", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
    expect(portal.graphql.callsTo("GetBallotPublicationChange")).toHaveLength(0)
    expect(portal.graphql.callsTo("PublishBallot")).toHaveLength(0)
    expect(portal.oidc.authorizations.map(({requestedAcr}) => requestedAcr)).toEqual([
        undefined,
        "gold",
    ])
    expect(portal.oidc.tokenRequests.map(({pkce}) => pkce)).toEqual(["valid", "valid"])
    const generation = portal.graphql.callsTo("GenerateBallotPublication")
    expect(generation).toHaveLength(1)
    expect(generation[0].variables).toEqual({electionEventId: IDS.event})
    expect(portal.graphql.callsTo("GetTaskById")[0].variables).toEqual({task_id: TASK_ID})
    workflow.completeGeneration()
    await page.clock.runFor(250)
    await expect(page.getByRole("region", {name: "Changes to Publish"})).toContainText(
        "Council ballot revision"
    )
    expect(portal.graphql.callsTo("GetBallotPublicationChange")[0].variables).toEqual({
        electionEventId: IDS.event,
        ballotPublicationId: PUBLICATION_ID,
        limit: 50,
    })
    await page.getByRole("button", {name: "Publish Changes", exact: true}).last().click()
    await expect(page.getByRole("cell", {name: PUBLICATION_ID, exact: true})).toBeVisible()
    await expect(page.getByRole("cell", {name: FIXED_TIME, exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("PublishBallot").map(({variables}) => variables)).toEqual([
        {electionEventId: IDS.event, ballotPublicationId: PUBLICATION_ID},
    ])

    const canceled = await channelDialog(page, "Start")
    await canceled.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(canceled).toHaveCount(0)
    expect(portal.graphql.callsTo("UpdateEventVotingStatus")).toHaveLength(0)
    for (const [index, action] of (["Start", "Pause", "Stop"] as const).entries()) {
        const dialog = await channelDialog(page, action)
        expect(portal.graphql.callsTo("UpdateEventVotingStatus")).toHaveLength(index)
        await dialog.getByRole("button", {name: "Confirm", exact: true}).click()
        await expect(dialog).toHaveCount(0)
        await expect
            .poll(() => portal.graphql.callsTo("UpdateEventVotingStatus").length)
            .toBe(index + 1)
    }
    expect(
        portal.graphql.callsTo("UpdateEventVotingStatus").map(({variables}) => variables)
    ).toEqual(
        ["OPEN", "PAUSED", "CLOSED"].map((votingStatus) => ({
            electionEventId: IDS.event,
            votingStatus,
            votingChannel: ["ONLINE"],
        }))
    )
    for (const action of ["Start", "Pause", "Stop"])
        await expect(
            page.getByRole("button", {name: `${action} Voting`, exact: true})
        ).toBeDisabled()
    for (const operation of [
        "GenerateBallotPublication",
        "GetBallotPublicationChange",
        "PublishBallot",
        "UpdateEventVotingStatus",
    ])
        for (const call of portal.graphql.callsTo(operation)) {
            expect(call.headers["x-hasura-role"]).toBe("admin-user")
            expect(
                portal.oidc.verifyAccessToken(call.headers.authorization.replace(/^Bearer /, ""))
            ).toMatchObject({acr: "gold", auth_time: Date.parse(FIXED_TIME) / 1000})
        }
    for (const call of portal.graphql.callsTo("sequent_backend_ballot_publication")) {
        expect(call.headers["x-hasura-role"]).toBe("publish-read")
        expect(
            portal.oidc.verifyAccessToken(call.headers.authorization.replace(/^Bearer /, ""))
        ).toMatchObject({"https://hasura.io/jwt/claims": {"x-hasura-tenant-id": TENANT_ID}})
    }
})
