// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFile} from "node:fs/promises"
import {createServer} from "node:http"
import {once} from "node:events"
import type {AddressInfo} from "node:net"
import type {Page} from "@playwright/test"
import {test as base, expect, TENANT_ID, type AdminPortal} from "../fixtures"
import {
    ELECTION_ID,
    EVENT_ID,
    FIXED_TIME,
    TALLY_ID,
    TALLY_ROLES,
    serveResults,
    tallyWorld,
} from "./data"

const DOCUMENT_ID = "68000000-0000-4000-8000-000000000001"
const TASK_ID = "68000000-0000-4000-8000-000000000002"
const REPORT_BYTES = Buffer.from("%PDF-1.7\nSynthetic ballot image report\n%%EOF\n")
const REPORT_PATH = "/reports/ballot-images.pdf?signature=synthetic"
const VARIABLES = {
    tallySessionId: TALLY_ID,
    electionId: ELECTION_ID,
    electionEventId: EVENT_ID,
    type: "BallotImages",
}
interface ReportDownload {
    url: string
    requests: {method: string | undefined; path: string | undefined; body: string}[]
}
const test = base.extend<{report: ReportDownload}>({
    report: async ({context, portal}, use) => {
        const requests: ReportDownload["requests"] = []
        // Here a native anchor download emits URL/name but no S3Mock request,
        // then download.path() reports cancellation. An owned loopback endpoint
        // records and completes the real GET; only its exact URL is allowlisted.
        const server = createServer(async (request, response) => {
            const chunks: Buffer[] = []
            for await (const chunk of request) chunks.push(Buffer.from(chunk))
            const body = Buffer.concat(chunks).toString("utf8")
            requests.push({method: request.method, path: request.url, body})
            if (request.method !== "GET" || request.url !== REPORT_PATH || body !== "") {
                portal.violations.add(`Unexpected report request: ${request.method} ${request.url}`)
                response.writeHead(400).end("Unexpected report request")
                return
            }
            response
                .writeHead(200, {
                    "content-type": "application/pdf",
                    "content-disposition": 'attachment; filename="ballot-images.pdf"',
                    "content-length": REPORT_BYTES.length,
                })
                .end(REPORT_BYTES)
        })
        server.listen(0, "127.0.0.1")
        await once(server, "listening")
        const url = `http://127.0.0.1:${(server.address() as AddressInfo).port}${REPORT_PATH}`
        const matchesReport = (candidate: URL) => candidate.href === url
        try {
            await context.route(matchesReport, (route) => route.continue())
            await use({url, requests})
        } finally {
            try {
                await context.unroute(matchesReport)
            } finally {
                server.closeAllConnections()
                await new Promise<void>((resolve, reject) =>
                    server.close((error) => (error ? reject(error) : resolve()))
                )
            }
            if (
                requests.length !== 1 ||
                requests.some(
                    (request) =>
                        request.method !== "GET" ||
                        request.path !== REPORT_PATH ||
                        request.body !== ""
                )
            )
                test.info().expectedStatus = "passed"
            expect(
                requests,
                "exactly one completed report download, including after a rejected generation"
            ).toEqual([{method: "GET", path: REPORT_PATH, body: ""}])
        }
    },
})
test.use({roles: [...TALLY_ROLES, "report-read", "document-read"]})

async function reportWorld(portal: AdminPortal, report: ReportDownload) {
    portal.settings.ACTIVATE_MIRU_EXPORT = true
    const world = tallyWorld(portal)
    world.session.execution_status = "SUCCESS"
    world.session.is_execution_completed = true
    world.execution.status.elections_status = [
        {election_id: ELECTION_ID, status: "SUCCESS", progress: 100},
    ]
    await serveResults(portal, world)
    world.documentUrls.set(DOCUMENT_ID, report.url)
    const task = {
        id: TASK_ID,
        name: "Generate ballot images",
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        type: "GENERATE_REPORT",
        execution_status: "SUCCESS",
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: FIXED_TIME,
        logs: [],
        annotations: {},
        labels: {},
        executed_by_user: "harbour-admin",
    }
    portal.graphql.on("GenerateTemplate", () => ({
        data: {generate_template: {document_id: DOCUMENT_ID, task_execution: task}},
    }))
    portal.graphql.on("GetTaskById", () => ({
        data: {sequent_backend_tasks_execution: [task]},
    }))
    portal.graphql.on("GetDocument", () => ({
        data: {sequent_backend_document: [{name: "ballot-images.pdf", annotations: {}}]},
    }))
}

async function openElectionActions(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally", exact: true}).click()
    const row = page.getByRole("row").filter({hasText: TALLY_ID})
    await row.locator('button:has(svg[aria-label="View Tally Ceremony"])').click()
    await expect(page.getByText("Status: SUCCESS", {exact: true})).toBeVisible()
    await expect(page.getByRole("tab", {name: "Harbour election", exact: true})).toBeVisible()
    await electionActions(page).click()
}

function electionActions(page: Page) {
    return page
        .getByText("Elections.", {exact: true})
        .locator("..")
        .getByLabel("export election data")
}

async function generateAndDownload(page: Page, portal: AdminPortal, url: string) {
    const downloading = page.waitForEvent("download")
    await page.getByRole("menuitem", {name: "Generate Ballot Images", exact: true}).click()
    const download = await downloading
    expect(download.suggestedFilename()).toBe("ballot-images.pdf")
    expect(download.url()).toBe(url)
    expect(await readFile((await download.path())!)).toEqual(REPORT_BYTES)
    expect(portal.graphql.callsTo("GenerateTemplate").map(({variables}) => variables)).toEqual([
        VARIABLES,
    ])
    expect(portal.graphql.callsTo("GenerateTemplate")[0].headers["x-hasura-role"]).toBe(
        "report-read"
    )
    expect(portal.graphql.callsTo("GetDocument").map(({variables}) => variables)).toEqual([
        {id: DOCUMENT_ID, tenantId: TENANT_ID},
    ])
    const documentReads = portal.graphql
        .callsTo("FetchDocument")
        .filter(({variables}) => variables.documentId === DOCUMENT_ID)
    expect(documentReads.length).toBeGreaterThan(0)
    for (const {variables} of documentReads)
        expect(variables).toEqual({electionEventId: EVENT_ID, documentId: DOCUMENT_ID})
    await expect(
        page.getByRole("heading", {name: "Task: Generate Report SUCCESS", exact: true})
    ).toBeVisible()
}

for (const outcome of ["missing document", "GraphQL rejection"] as const) {
    test(`generates ballot images and reports ${outcome} without reusing the prior document`, async ({
        page,
        portal,
        report,
    }) => {
        await reportWorld(portal, report)
        await openElectionActions(page, portal)
        await generateAndDownload(page, portal, report.url)
        expect(report.requests).toEqual([{method: "GET", path: REPORT_PATH, body: ""}])
        portal.graphql.once("GenerateTemplate", () =>
            outcome === "missing document"
                ? {data: {generate_template: {document_id: "", task_execution: null}}}
                : {errors: [{message: "Ballot image report generation rejected"}]}
        )
        await electionActions(page).click()
        await page.getByRole("menuitem", {name: "Generate Ballot Images", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("GenerateTemplate").length).toBe(2)
        expect(portal.graphql.callsTo("GenerateTemplate").map(({variables}) => variables)).toEqual([
            VARIABLES,
            VARIABLES,
        ])
        expect(report.requests).toHaveLength(1)
        expect(portal.violations.list()).toEqual([])
        await expect(
            page.getByRole("heading", {name: /^Task: Generate Report (IN_PROGRESS|FAILED)$/})
        ).toBeVisible()
        test.fail(
            true,
            "Widget keeps its initial IN_PROGRESS state when report generation marks the task FAILED without a task ID"
        )
        await expect(page.getByText("FAILED", {exact: true})).toBeVisible({timeout: 2000})
    })
}

test("hides ballot-image generation when Miru export is disabled", async ({
    page,
    portal,
    report,
}) => {
    await reportWorld(portal, report)
    await openElectionActions(page, portal)
    await generateAndDownload(page, portal, report.url)
    portal.settings.ACTIVATE_MIRU_EXPORT = false
    await openElectionActions(page, portal)
    await expect(
        page.getByRole("menuitem", {name: "Generate Ballot Images", exact: true})
    ).toHaveCount(0)
    expect(portal.graphql.callsTo("GenerateTemplate")).toHaveLength(1)
})
