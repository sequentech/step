// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Locator, Page} from "@playwright/test"
import {test, expect, TENANT_ID} from "../fixtures"
import type {AdminPortal} from "../fixtures"
import {
    ALICE_ID,
    BOB_ID,
    CONTEST_ID,
    EVENT_ID,
    FIXED_TIME,
    RESULTS_ID,
    TALLY_ID,
    TALLY_ROLES,
    TRUSTEES,
    serveResults,
    tallyWorld,
} from "./data"

test.use({roles: TALLY_ROLES})

const rowAction = (row: Locator, label: string) =>
    row.locator(`button:has(svg[aria-label="${label}"])`)

async function openTallyList(page: Page, portal: AdminPortal) {
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
    portal.settings.QUERY_POLL_INTERVAL_MS = 100
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally", exact: true}).click()
    const row = page.getByRole("row").filter({hasText: TALLY_ID})
    await expect(row).toBeVisible()
    return row
}

test("a manual tally ceremony starts once trustees restore their keys and resolves a tie", async ({
    page,
    portal,
}) => {
    const world = tallyWorld(portal)
    portal.graphql.on("UpdateTallyCeremony", ({variables}) => {
        world.session.execution_status = String(variables.status)
        return {data: {update_tally_ceremony: {tally_session_id: TALLY_ID}}}
    })

    const row = await openTallyList(page, portal)
    await rowAction(row, "View Tally Ceremony").click()
    const start = page.getByRole("button", {name: "Start Tally", exact: true})
    await expect(page.getByRole("gridcell", {name: TRUSTEES[0], exact: true})).toBeVisible()
    await expect(page.getByText("0/2 trustees imported the key", {exact: true})).toBeVisible()
    await expect(page.getByRole("alert")).toContainText(
        "You cannot continue the ceremony because the tally session is not connected"
    )
    await expect(start).toBeDisabled()

    world.execution.status.trustees = TRUSTEES.map((name) => ({name, status: "KEY_RESTORED"}))
    world.session.execution_status = "CONNECTED"
    await page.clock.runFor(101)
    await expect(page.getByText("2/2 trustees imported the key", {exact: true})).toBeVisible()
    await expect(start).toBeEnabled()
    await start.click()
    const dialog = page.getByRole("dialog")
    await expect(dialog).toContainText("All required trustees have verified their key fragments.")
    expect(portal.graphql.callsTo("UpdateTallyCeremony")).toEqual([])
    await dialog.getByRole("button", {name: "Start Tally", exact: true}).click()
    await expect(page.getByText("Tally started", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("UpdateTallyCeremony").map(({variables}) => variables)).toEqual([
        {election_event_id: EVENT_ID, tally_session_id: TALLY_ID, status: "IN_PROGRESS"},
    ])

    await serveResults(portal, world)
    world.session.execution_status = "AWAITING_INPUT"
    world.resolutions = [tie()]
    portal.graphql.on("SubmitTallyResolution", () => {
        world.resolutions = [
            tie({
                status: "resolved",
                resolved_at: FIXED_TIME,
                resolved_by_user: "harbour-admin",
                resolution_data: {...tieData, resolved_by_candidate_id: ALICE_ID},
            }),
        ]
        world.session.execution_status = "IN_PROGRESS"
        return {
            data: {
                submit_tally_resolution: {
                    success: true,
                    tally_session_id: TALLY_ID,
                    resolved_count: 1,
                },
            },
        }
    })
    await page.clock.runFor(101)
    const item = page.getByText("Tie Resolution Required", {exact: true})
    await expect(item).toBeVisible()
    await expect(page.getByRole("heading", {name: "Pending resolutions (1)"})).toBeVisible()
    await expect(
        page.getByText("Tie Resolution Required: Harbour election | Mayor | Round 1")
    ).toBeVisible()
    await item.click()
    await expect(page.getByRole("alert").filter({hasText: "Tally paused"})).toHaveText(
        "Tally paused due to unresolved tie (Round 1)" +
            "Candidates tied (30 votes, 50.0%): Alice Example, Bob Example. " +
            "Manual tie-break required to continue tally."
    )
    const apply = page.getByRole("button", {name: "Apply Resolutions and Recalculate"})
    await expect(apply).toBeDisabled()
    await page.getByRole("combobox").last().click()
    await page.getByRole("option", {name: "Alice Example", exact: true}).click()
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await expect(page.getByText("Pending calculation", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("SubmitTallyResolution")).toEqual([])
    await apply.click()
    await expect(
        page.getByText("Resolutions submitted. Tally is resuming...", {exact: true})
    ).toBeVisible()
    const submitted = portal.graphql.callsTo("SubmitTallyResolution")
    expect(submitted.map(({variables}) => variables)).toEqual([
        {
            election_event_id: EVENT_ID,
            tally_session_id: TALLY_ID,
            resolutions: [{contest_id: CONTEST_ID, selected_candidate_id: ALICE_ID}],
        },
    ])
    expect(submitted[0].headers["x-hasura-role"]).toBe("tally-resolution-submit")
})

const tieData = {
    round_number: 1,
    tied_candidate_ids: [ALICE_ID, BOB_ID],
    vote_count: 30,
    method_used: "manual",
}

function tie(overrides: Record<string, unknown> = {}) {
    return {
        id: "63000000-0000-4000-8000-000000000001",
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        tally_session_id: TALLY_ID,
        contest_id: CONTEST_ID,
        area_id: null,
        resolution_type: "tie_break",
        status: "pending",
        resolution_data: tieData,
        resolved_at: null,
        resolved_by_user: null,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        ...overrides,
    }
}

const JSON_DOCUMENT_ID = "64000000-0000-4000-8000-000000000001"
const HTML_DOCUMENT_ID = "64000000-0000-4000-8000-000000000002"
const PDF_DOCUMENT_ID = "64000000-0000-4000-8000-000000000003"
const XLSX_DOCUMENT_ID = "64000000-0000-4000-8000-000000000004"
const TASK_ID = "65000000-0000-4000-8000-000000000001"

/** A completed tally whose results database and event documents are ready. */
async function completedTally(portal: AdminPortal) {
    const world = tallyWorld(portal)
    await serveResults(portal, world)
    world.session.execution_status = "SUCCESS"
    world.session.is_execution_completed = true
    world.execution.status.trustees = TRUSTEES.map((name) => ({name, status: "KEY_RESTORED"}))
    world.execution.status.elections_status = [
        {election_id: world.election.id as string, status: "SUCCESS", progress: 100},
    ]
    return world
}

function task(type: string, status = "SUCCESS") {
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

test("a completed tally shows global and per-area results and exports its documents", async ({
    page,
    portal,
}) => {
    const world = await completedTally(portal)
    world.resultsDocuments.json = JSON_DOCUMENT_ID
    world.resultsDocuments.html = HTML_DOCUMENT_ID
    const jsonUrl = portal.s3.presign(`${TENANT_ID}/${EVENT_ID}/results.json`, "json")
    world.documentUrls.set(JSON_DOCUMENT_ID, jsonUrl)
    portal.graphql.on("RenderDocumentPdf", () => ({
        data: {
            render_document_pdf: {
                document_id: PDF_DOCUMENT_ID,
                task_execution: task("RENDER_DOCUMENT_PDF", "IN_PROGRESS"),
            },
        },
    }))
    portal.graphql.on("GetTaskById", () => ({
        data: {sequent_backend_tasks_execution: [task("RENDER_DOCUMENT_PDF", "IN_PROGRESS")]},
    }))

    const row = await openTallyList(page, portal)
    await rowAction(row, "View Tally Ceremony").click()
    await expect(page.getByText("Status: SUCCESS", {exact: true})).toBeVisible()
    await expect(
        page
            .getByRole("row", {name: /Alice Example/})
            .getByRole("gridcell", {name: "37", exact: true})
    ).toBeVisible()
    await page.getByRole("tab", {name: "North precinct", exact: true}).click()
    await expect(
        page
            .getByRole("row", {name: /Alice Example/})
            .getByRole("gridcell", {name: "20", exact: true})
    ).toBeVisible()
    await expect(
        page
            .getByRole("row", {name: /Bob Example/})
            .getByRole("gridcell", {name: "10", exact: true})
    ).toBeVisible()

    await page.getByLabel("export election data").first().click()
    const download = page.waitForEvent("download")
    await page
        .getByRole("menuitem", {name: "Export in JSON format - 'Harbour event' results"})
        .click()
    const file = await download
    expect(file.suggestedFilename()).toBe("report.json")
    expect(file.url()).toBe(jsonUrl)
    expect(portal.graphql.callsTo("FetchDocument").map(({variables}) => variables)).toContainEqual({
        electionEventId: EVENT_ID,
        documentId: JSON_DOCUMENT_ID,
    })

    const pdfUrl = portal.s3.presign(`${TENANT_ID}/${EVENT_ID}/results.pdf`, "pdf")
    world.documentUrls.set(PDF_DOCUMENT_ID, pdfUrl)
    portal.graphql.on("GetDocument", () => ({
        data: {sequent_backend_document: [{name: "results.pdf", annotations: {}}]},
    }))
    await page.getByLabel("export election data").first().click()
    const pdf = page.waitForEvent("download")
    await page
        .getByRole("menuitem", {name: "Export in PDF format - 'Harbour event' results"})
        .click()
    expect((await pdf).url()).toBe(pdfUrl)
    expect(portal.graphql.callsTo("GetDocument")[0].variables).toEqual({
        id: PDF_DOCUMENT_ID,
        tenantId: TENANT_ID,
    })
    const render = portal.graphql.callsTo("RenderDocumentPdf")[0]
    expect(render.variables).toEqual({
        documentId: HTML_DOCUMENT_ID,
        tallySessionId: TALLY_ID,
        electionEventId: EVENT_ID,
    })
    expect(render.headers["x-hasura-role"]).toBe("report-read")

    const xlsxUrl = portal.s3.presign(`${TENANT_ID}/${EVENT_ID}/results.xlsx`, "xlsx")
    world.documentUrls.set(XLSX_DOCUMENT_ID, xlsxUrl)
    portal.graphql.on("GetTallySessionExecution", () => ({
        data: {sequent_backend_tally_session_execution: [world.execution]},
    }))
    portal.graphql.on("ExportTallyResults", () => ({
        data: {
            export_tally_results: {
                document_id: XLSX_DOCUMENT_ID,
                task_execution: task("EXPORT_TALLY_RESULTS_XLSX", "IN_PROGRESS"),
                error_msg: null,
            },
        },
    }))
    await page.getByLabel("export election data").first().click()
    const xlsx = page.waitForEvent("download")
    await page
        .getByRole("menuitem", {name: "Export in XLSX format - 'Harbour event' results"})
        .click()
    expect((await xlsx).url()).toBe(xlsxUrl)
    expect(portal.graphql.callsTo("GetTallySessionExecution")[0].variables).toEqual({
        tallySessionId: TALLY_ID,
        tenantId: TENANT_ID,
        resultsEventId: RESULTS_ID,
    })
    const exported = portal.graphql.callsTo("ExportTallyResults")
    expect(exported.map(({variables}) => variables)).toEqual([
        {electionEventId: EVENT_ID, tallySessionId: TALLY_ID},
    ])
    expect(exported[0].headers["x-hasura-role"]).toBe("tally-results-read")
})

/** A Hasura action error whose webhook answered with a readable reason. */
const actionError = (reason: string) => ({
    errors: [
        {
            message: "http exception when calling webhook",
            extensions: {
                code: "unexpected",
                internal: {response: {status: 400, body: JSON.stringify({message: reason})}},
            },
        },
    ],
})

test("reports the backend reason when the tally cannot start and when a tie cannot be resolved", async ({
    page,
    portal,
}) => {
    const world = tallyWorld(portal)
    world.session.execution_status = "CONNECTED"
    world.execution.status.trustees = TRUSTEES.map((name) => ({name, status: "KEY_RESTORED"}))
    portal.graphql.on("UpdateTallyCeremony", () => actionError("Trustee Bob Trustee is offline"))

    const row = await openTallyList(page, portal)
    await rowAction(row, "View Tally Ceremony").click()
    await page.getByRole("button", {name: "Start Tally", exact: true}).click()
    await page.getByRole("dialog").getByRole("button", {name: "Start Tally", exact: true}).click()
    await expect(page.getByText("Trustee Bob Trustee is offline", {exact: true})).toBeVisible()
    await expect(page.getByRole("button", {name: "Start Tally", exact: true})).toBeEnabled()
    expect(portal.graphql.callsTo("UpdateTallyCeremony")[0].variables).toMatchObject({
        status: "IN_PROGRESS",
    })

    await serveResults(portal, world)
    world.session.execution_status = "AWAITING_INPUT"
    world.resolutions = [tie()]
    portal.graphql.on("SubmitTallyResolution", () => actionError("tie already resolved"))
    await page.clock.runFor(101)
    await page.getByText("Tie Resolution Required", {exact: true}).click()
    await page.getByRole("combobox").last().click()
    await page.getByRole("option", {name: "Bob Example", exact: true}).click()
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await page.getByRole("button", {name: "Apply Resolutions and Recalculate"}).click()
    await expect(
        page.getByText("Failed to submit resolutions. Please try again.", {exact: true})
    ).toBeVisible()
    await expect(page.getByText("Pending calculation", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("SubmitTallyResolution")[0].variables).toMatchObject({
        resolutions: [{contest_id: CONTEST_ID, selected_candidate_id: BOB_ID}],
    })
})

test("cancels a started tally and recounts a completed one from the tally list", async ({
    page,
    portal,
}) => {
    const world = tallyWorld(portal)
    const completed = {
        ...world.session,
        id: "61000000-0000-4000-8000-000000000009",
        execution_status: "SUCCESS",
        is_execution_completed: true,
    }
    world.sessions = [world.session, completed]
    portal.graphql.on("UpdateTallyCeremony", ({variables}) => {
        world.session.execution_status = String(variables.status)
        return {data: {update_tally_ceremony: {tally_session_id: TALLY_ID}}}
    })
    portal.graphql.once("RecountTallySession", () => actionError("results are locked"))
    portal.graphql.on("RecountTallySession", () => ({
        data: {recount_tally_session: {tally_session_id: completed.id}},
    }))

    const row = await openTallyList(page, portal)
    await expect(row).toContainText("STARTED")
    await expect(rowAction(row, "Recount tally")).toHaveCount(0)
    await rowAction(row, "Cancel Tally Ceremony").click()
    const dialog = page.getByRole("dialog")
    await expect(dialog).toContainText(
        "You are about to cancel the tally ceremony. This action is not undoable."
    )
    await dialog.getByRole("button", {name: "Close", exact: true}).click()
    expect(portal.graphql.callsTo("UpdateTallyCeremony")).toEqual([])
    await rowAction(row, "Cancel Tally Ceremony").click()
    await dialog.getByRole("button", {name: "Cancel Tally", exact: true}).click()
    await expect(page.getByText("Tally Ceremony canceled", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("UpdateTallyCeremony").map(({variables}) => variables)).toEqual([
        {election_event_id: EVENT_ID, tally_session_id: TALLY_ID, status: "CANCELLED"},
    ])
    await expect(row).toContainText("CANCELLED")
    await expect(rowAction(row, "Cancel Tally Ceremony")).toHaveCount(0)

    const done = page.getByRole("row").filter({hasText: completed.id})
    await expect(rowAction(done, "Cancel Tally Ceremony")).toHaveCount(0)
    for (const outcome of ["Could not start recount", "Recount started"]) {
        await rowAction(done, "Recount tally").click()
        await expect(dialog).toContainText(
            "This will generate a fresh results event for the completed tally session."
        )
        await dialog.getByRole("button", {name: "Recount", exact: true}).click()
        await expect(page.getByText(outcome, {exact: true})).toBeVisible()
    }
    expect(portal.graphql.callsTo("RecountTallySession").map(({variables}) => variables)).toEqual(
        Array(2).fill({election_event_id: EVENT_ID, tally_session_id: completed.id})
    )
})

test.describe("as a trustee", () => {
    test.use({
        roles: [
            "admin-user",
            "election-event-read",
            "election-read",
            "trustee-ceremony",
            "tally-read",
            "election-event-tally-tab",
        ],
    })

    test("restores a key fragment for a started tally after a rejected upload", async ({
        page,
        portal,
    }) => {
        const world = tallyWorld(portal)
        // The OIDC user is the trustee the ceremony waits for.
        world.execution.status.trustees = [
            {name: "synthetic-admin", status: "WAITING"},
            {name: TRUSTEES[1], status: "KEY_RESTORED"},
        ]
        portal.graphql.once("RestorePrivateKey", () => ({
            data: {restore_private_key: {is_valid: false}},
        }))
        portal.graphql.on("RestorePrivateKey", () => ({
            data: {restore_private_key: {is_valid: true}},
        }))

        const row = await openTallyList(page, portal)
        expect(
            portal.graphql
                .callsTo("sequent_backend_tally_session")
                .map(({headers}) => headers["x-hasura-role"])
        ).toContain("trustee-ceremony")
        await rowAction(row, "Add Tally Key").click()
        await expect(page.getByText("Please upload you key fragment", {exact: true})).toBeVisible()
        const next = page.getByRole("button", {name: "Next", exact: true})
        await expect(next).toBeDisabled()
        const upload = (content: string) =>
            page.locator('input[type="file"]').setInputFiles({
                name: "fragment.txt",
                mimeType: "text/plain",
                buffer: Buffer.from(content),
            })
        await upload("stale-fragment")
        await expect(
            page.getByText("Invalid Encrypted Private Key Backup, please try again", {exact: true})
        ).toBeVisible()
        await expect(next).toBeDisabled()
        await upload("valid-fragment")
        await expect(page.getByText("Backup verified successfully.", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("RestorePrivateKey").map(({variables}) => variables)).toEqual(
            ["stale-fragment", "valid-fragment"].map((privateKeyBase64) => ({
                electionEventId: EVENT_ID,
                tallySessionId: TALLY_ID,
                privateKeyBase64,
            }))
        )
        await next.click()
        await expect(page.getByText("Key fragment import status", {exact: false})).toBeVisible()
        await expect(page.getByRole("gridcell", {name: TRUSTEES[1], exact: true})).toBeVisible()
        await page.getByRole("button", {name: "Back", exact: true}).click()
        await expect(row).toBeVisible()
    })
})
