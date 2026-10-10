// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, TENANT_ID} from "../fixtures"
import type {AdminPortal} from "../fixtures"
import {AREA_ID, EVENT_ID, FIXED_TIME, registerEvent, type Row} from "./data"

const PUBLICATION_ID = "d1000000-0000-4000-8000-000000000001"
const PREVIEW_DOCUMENT_ID = "d2000000-0000-4000-8000-000000000001"
const EXPORT_DOCUMENT_ID = "d2000000-0000-4000-8000-000000000002"
const TASK_ID = "d3000000-0000-4000-8000-000000000001"

test.use({
    roles: [
        "admin-user",
        "election-event-read",
        "election-read",
        "election-event-publish-tab",
        "election-event-publish-view",
        "election-event-publish-preview",
        "publish-read",
        "publish-write",
        "publish-export",
    ],
})

function task(type: string, status = "SUCCESS"): Row {
    return {
        id: TASK_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: type,
        type,
        execution_status: status,
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: FIXED_TIME,
        logs: [],
        annotations: {},
        labels: {},
        executed_by_user: "harbour-admin",
    }
}

/** An event with one generated, unpublished ballot publication for North precinct. */
function generatedPublication(portal: AdminPortal) {
    registerEvent(portal, {
        status: {voting_status: "NOT_STARTED"},
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: false},
        elections: [],
        elections_aggregate: {aggregate: {count: 0}, nodes: []},
    })
    const publication = {
        id: PUBLICATION_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: null,
        is_generated: true,
        published_at: null,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
    }
    portal.graphql.on("sequent_backend_ballot_publication", () => ({
        data: {
            sequent_backend_ballot_publication: [publication],
            sequent_backend_ballot_publication_aggregate: {aggregate: {count: 1}},
        },
    }))
    portal.graphql.on("GetBallotPublicationChange", () => ({
        data: {
            get_ballot_publication_changes: {
                previous: null,
                current: {
                    ballot_publication_id: PUBLICATION_ID,
                    ballot_styles: [
                        {id: "north-style", area_id: AREA_ID, ballot_eml: "North ballot"},
                    ],
                },
            },
        },
    }))
    portal.graphql.on("sequent_backend_area", () => ({
        data: {
            sequent_backend_area: [
                {id: AREA_ID, name: "North precinct"},
                {id: "e1000000-0000-4000-8000-000000000002", name: "Unpublished area"},
            ],
        },
    }))
}

async function openPublication(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    const row = page.getByRole("row").filter({hasText: PUBLICATION_ID})
    // The list's row actions are unnamed icon buttons, told apart by their icon class.
    await row.locator("button:has(svg.publish-visibility-icon)").click()
    await expect(page.getByText("North ballot").first()).toBeVisible()
}

/** window.open would start a second portal; the requested URLs are the contract. */
async function recordOpenedWindows(page: Page) {
    await page.addInitScript(() => {
        const opened: string[] = []
        Object.assign(window, {openedWindows: opened})
        window.open = (url?: string | URL) => {
            opened.push(String(url))
            return null
        }
    })
    return () => page.evaluate(() => (window as unknown as {openedWindows: string[]}).openedWindows)
}

test("previews a generated publication for one of its areas and copies the preview link", async ({
    page,
    portal,
    context,
}) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"])
    const opened = await recordOpenedWindows(page)
    generatedPublication(portal)
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
    portal.settings.QUERY_POLL_INTERVAL_MS = 100
    portal.graphql.on("PrepareBallotPublicationPreview", () => ({
        data: {
            prepare_ballot_publication_preview: {
                error_msg: null,
                document_id: PREVIEW_DOCUMENT_ID,
                task_execution: task("PREPARE_PUBLICATION_PREVIEW", "IN_PROGRESS"),
            },
        },
    }))
    portal.graphql.on("GetTaskById", () => ({
        data: {sequent_backend_tasks_execution: [task("PREPARE_PUBLICATION_PREVIEW")]},
    }))

    await openPublication(page, portal)
    await page.getByRole("button", {name: "Preview", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await dialog.getByRole("combobox").click()
    await expect(page.getByRole("option")).toHaveText(["North precinct"])
    await page.getByRole("option", {name: "North precinct", exact: true}).click()
    expect(portal.graphql.callsTo("PrepareBallotPublicationPreview")).toEqual([])
    await dialog.getByRole("button", {name: "Preview", exact: true}).click()
    await expect(page.getByText("Success opening preview", {exact: true})).toBeVisible()
    const previewUrl = `${portal.origin}/voting/preview/${TENANT_ID}/${PREVIEW_DOCUMENT_ID}/${AREA_ID}/${PUBLICATION_ID}`
    expect(await opened()).toEqual([previewUrl])
    expect(
        portal.graphql.callsTo("PrepareBallotPublicationPreview").map(({variables}) => variables)
    ).toEqual([{electionEventId: EVENT_ID, ballotPublicationId: PUBLICATION_ID}])
    expect(portal.graphql.callsTo("GetTaskById")[0].variables).toEqual({task_id: TASK_ID})

    // The finished task's widget covers the toolbar until it is closed; its close button
    // is the last, unnamed icon button of the widget.
    await page.locator(".widget-stack .status-icons button").last().click()
    await expect(page.locator(".widget-stack .status-icons")).toHaveCount(0)
    await page.getByRole("button", {name: "Preview", exact: true}).click()
    await dialog.getByRole("combobox").click()
    await page.getByRole("option", {name: "North precinct", exact: true}).click()
    await dialog.getByRole("button", {name: "Copy link"}).click()
    await expect(page.getByText("Success copying preview link", {exact: true})).toBeVisible()
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(previewUrl)
    expect(await opened()).toEqual([previewUrl])
})

test("exports a publication from its read-only view and reports a failed preview", async ({
    page,
    portal,
}) => {
    const opened = await recordOpenedWindows(page)
    generatedPublication(portal)
    portal.graphql.on("PrepareBallotPublicationPreview", () => ({
        errors: [{message: "ballot styles are missing"}],
    }))
    portal.graphql.on("ExportBallotPublication", () => ({
        data: {
            export_ballot_publication: {
                document_id: EXPORT_DOCUMENT_ID,
                task_execution: task("EXPORT_BALLOT_PUBLICATION"),
            },
        },
    }))
    portal.graphql.on("GetTaskById", () => ({
        data: {sequent_backend_tasks_execution: [task("EXPORT_BALLOT_PUBLICATION")]},
    }))
    portal.graphql.on("GetDocument", () => ({
        data: {sequent_backend_document: [{name: "publication.zip", annotations: {}}]},
    }))
    const exportUrl = portal.s3.presign(`${TENANT_ID}/${EVENT_ID}/publication.zip`, "export")
    portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url: exportUrl}}}))

    await openPublication(page, portal)
    await page.getByRole("button", {name: "Preview", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await dialog.getByRole("combobox").click()
    await page.getByRole("option", {name: "North precinct", exact: true}).click()
    await dialog.getByRole("button", {name: "Preview", exact: true}).click()
    await expect(page.getByText("Error previewing publication", {exact: true})).toBeVisible()
    expect(await opened()).toEqual([])
    await page.keyboard.press("Escape")
    await expect(dialog).toHaveCount(0)

    await page.getByRole("button", {name: "Export", exact: true}).click()
    await expect(dialog).toBeVisible()
    expect(portal.graphql.callsTo("ExportBallotPublication")).toEqual([])
    const download = page.waitForEvent("download")
    await dialog.getByRole("button", {name: "Export", exact: true}).click()
    expect((await download).url()).toBe(exportUrl)
    const exported = portal.graphql.callsTo("ExportBallotPublication")
    expect(exported.map(({variables}) => variables)).toEqual([
        {
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            electionId: null,
            ballotPublicationId: PUBLICATION_ID,
        },
    ])
    expect(exported[0].headers["x-hasura-role"]).toBe("publish-write")
    expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({
        documentId: EXPORT_DOCUMENT_ID,
    })
})
