// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, TENANT_ID} from "../fixtures"
import type {AdminPortal} from "../fixtures"
import {
    CONTEST_ID,
    ELECTION_ID,
    EVENT_ID,
    EXECUTION_ID,
    FIXED_TIME,
    RESULTS_ID,
    TALLY_ID,
    TALLY_ROLES,
    serveResults,
    table,
    tallyWorld,
} from "./data"

const PUBLICATION_ID = "66000000-0000-4000-8000-000000000001"
const TASK_ID = "66000000-0000-4000-8000-000000000002"
test.use({roles: [...TALLY_ROLES, "publish-results-read", "publish-results-write"]})

async function publicationWorld(portal: AdminPortal) {
    const world = tallyWorld(portal)
    world.event.presentation = {
        ...(world.event.presentation as Record<string, unknown>),
        results_website: {
            status: "enabled",
            access: "authenticated",
            visibility_scope: "area_based",
        },
    }
    world.session.execution_status = "SUCCESS"
    world.session.is_execution_completed = true
    world.execution.status.elections_status = [
        {election_id: ELECTION_ID, status: "SUCCESS", progress: 100},
    ]
    await serveResults(portal, world)
    const publication = {
        id: PUBLICATION_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        tally_session_id: TALLY_ID,
        tally_session_execution_id: EXECUTION_ID,
        results_event_id: RESULTS_ID,
        version: 1,
        publication_status: "Published",
        route_scope: "event",
        route_election_id: null,
        election_ids: [ELECTION_ID],
        published_contest_ids: [CONTEST_ID],
        contest_publication_state: {},
        access: "authenticated",
        visibility_scope: "area_based",
        documents: {},
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        published_at: FIXED_TIME,
        revoked_at: null as string | null,
    }
    table(portal, "sequent_backend_tally_results_publication", () => [publication])
    portal.graphql.on("GetTaskById", () => ({
        data: {
            sequent_backend_tasks_execution: [
                {
                    id: TASK_ID,
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    type: "PUBLISH_RESULTS_WEBSITE",
                    execution_status: "SUCCESS",
                    start_at: FIXED_TIME,
                    end_at: FIXED_TIME,
                    logs: [],
                    annotations: {},
                    executed_by_user: "harbour-admin",
                },
            ],
        },
    }))
    return publication
}

async function openPublication(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally", exact: true}).click()
    const row = page.getByRole("row").filter({hasText: TALLY_ID})
    await row.locator('button:has(svg[aria-label="View Tally Ceremony"])').click()
    await expect(page.getByText("Status: SUCCESS", {exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Publish to results website", exact: true}).click()
    await expect(page.getByRole("checkbox", {name: "Harbour election - Mayor"})).toBeChecked()
}

test("publishes event results with personal visibility, then revokes the publication", async ({
    page,
    portal,
}) => {
    const publication = await publicationWorld(portal)
    portal.graphql.on("PublishResultsWebsite", () => ({
        data: {
            publishResultsWebsite: {
                publication_id: PUBLICATION_ID,
                task_execution_id: TASK_ID,
                publication_status: "Published",
                error_msg: null,
            },
        },
    }))
    portal.graphql.on("RevokeResultsPublication", () => {
        publication.publication_status = "Revoked"
        publication.revoked_at = FIXED_TIME
        return {
            data: {
                revokeResultsPublication: {
                    publication_id: PUBLICATION_ID,
                    publication_status: "Revoked",
                },
            },
        }
    })
    await openPublication(page, portal)
    const publish = page.getByRole("button", {name: "Publish selected contests", exact: true})
    const contest = page.getByRole("checkbox", {name: "Harbour election - Mayor"})
    await contest.uncheck()
    await expect(publish).toBeDisabled()
    await contest.check()
    await page.getByRole("combobox", {name: "Route", exact: true}).click()
    await page.getByRole("option", {name: "Event results", exact: true}).click()
    await expect(page.getByRole("combobox", {name: "Access", exact: true})).toBeDisabled()
    await expect(page.getByRole("combobox", {name: "Access", exact: true})).toHaveText(
        "Authenticated access"
    )
    await expect(page.getByRole("combobox", {name: "Visibility", exact: true})).toBeDisabled()
    await expect(page.getByRole("combobox", {name: "Visibility", exact: true})).toHaveText(
        "Personal visibility"
    )
    await publish.click()
    await page.getByRole("dialog").getByRole("button", {name: "Close", exact: true}).click()
    await expect(page.getByRole("dialog")).toHaveCount(0)
    expect(portal.graphql.callsTo("PublishResultsWebsite")).toEqual([])
    await publish.click()
    await page.getByRole("dialog").getByRole("button", {name: "Publish selected contests"}).click()
    await expect(page.getByText("Results publication started", {exact: true})).toBeVisible()
    const calls = portal.graphql.callsTo("PublishResultsWebsite")
    expect(calls.map(({variables}) => variables)).toEqual([
        {
            election_event_id: EVENT_ID,
            tally_session_id: TALLY_ID,
            tally_session_execution_id: EXECUTION_ID,
            results_event_id: RESULTS_ID,
            route_scope: "event",
            route_election_id: null,
            election_ids: [ELECTION_ID],
            contest_ids: [CONTEST_ID],
            access: "authenticated",
            visibility_scope: "area_based",
        },
    ])
    expect(calls[0].headers["x-hasura-role"]).toBe("publish-results-write")
    const publicationRow = page.getByRole("row").filter({hasText: "Published"})
    await expect(publicationRow.getByRole("link", {name: "Open", exact: true})).toHaveAttribute(
        "href",
        `${portal.origin}/results/${EVENT_ID}`
    )
    await publicationRow.getByRole("button", {name: "Revoke", exact: true}).click()
    await expect(page.getByText("Results publication revoked", {exact: true})).toBeVisible()
    const revoked = portal.graphql.callsTo("RevokeResultsPublication")
    expect(revoked.map(({variables}) => variables)).toEqual([
        {election_event_id: EVENT_ID, publication_id: PUBLICATION_ID},
    ])
    expect(revoked[0].headers["x-hasura-role"]).toBe("publish-results-write")
    await expect(page.getByRole("link", {name: "Open", exact: true})).toHaveCount(0)
    await expect(page.getByRole("button", {name: "Revoke", exact: true})).toHaveCount(0)
    await expect(page.getByRole("cell", {name: "Revoked", exact: true})).toBeVisible()
})

test("surfaces a rejected publication without losing the selected contest", async ({
    page,
    portal,
}) => {
    await publicationWorld(portal)
    portal.graphql.on("PublishResultsWebsite", () => ({
        data: {
            publishResultsWebsite: {
                publication_id: PUBLICATION_ID,
                task_execution_id: TASK_ID,
                publication_status: "Failed",
                error_msg: "Results are still being finalized",
            },
        },
    }))
    await openPublication(page, portal)
    await page.getByRole("button", {name: "Publish selected contests", exact: true}).click()
    await page.getByRole("dialog").getByRole("button", {name: "Publish selected contests"}).click()
    await expect(page.getByText("Results are still being finalized", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("PublishResultsWebsite").map(({variables}) => variables)).toEqual(
        [
            {
                election_event_id: EVENT_ID,
                tally_session_id: TALLY_ID,
                tally_session_execution_id: EXECUTION_ID,
                results_event_id: RESULTS_ID,
                route_scope: "election",
                route_election_id: ELECTION_ID,
                election_ids: [ELECTION_ID],
                contest_ids: [CONTEST_ID],
                access: "authenticated",
                visibility_scope: "area_based",
            },
        ]
    )
    await expect(page.getByRole("dialog")).toHaveCount(0)
    await expect(page.getByRole("checkbox", {name: "Harbour election - Mayor"})).toBeChecked()
})

test.describe("with publication read permission", () => {
    test.use({roles: [...TALLY_ROLES, "publish-results-read"]})

    test("shows published results but disables publishing and revoking", async ({page, portal}) => {
        await publicationWorld(portal)
        await openPublication(page, portal)
        await expect(
            page.getByText(
                "You need publish-results-write permission to publish or revoke results."
            )
        ).toBeVisible()
        await expect(
            page.getByRole("button", {name: "Publish selected contests", exact: true})
        ).toBeDisabled()
        await expect(page.getByRole("button", {name: "Revoke", exact: true})).toBeDisabled()
        await expect(page.getByRole("link", {name: "Open", exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("PublishResultsWebsite")).toEqual([])
        expect(portal.graphql.callsTo("RevokeResultsPublication")).toEqual([])
    })
})
