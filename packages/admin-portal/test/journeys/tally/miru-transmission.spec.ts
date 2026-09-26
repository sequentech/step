// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {createServer} from "node:http"
import {readFile} from "node:fs/promises"
import type {Page} from "@playwright/test"
import type {GraphQLReply} from "@sequentech/ui-test-kit/mocks/graphql"
import {test as base, expect, TENANT_ID, type AdminPortal} from "../fixtures"
import {
    AREA_ID,
    ELECTION_ID,
    EVENT_ID,
    FIXED_TIME,
    RESULTS_ID,
    SQLITE_DOCUMENT_ID,
    TALLY_ID,
    TALLY_ROLES,
    resultsDataset,
    sqlite,
    table,
    tallyWorld,
} from "./data"

const REPORT_PATH = "/miru/transmission_report.pdf"
const REPORT_BYTES = Buffer.from("%PDF-1.7 synthetic transmission report\n%%EOF\n")
const test = base.extend<{report: {url: string}}>({
    report: async ({context, portal}, use) => {
        // Chromium hands an anchor download to its network stack. This single local
        // endpoint verifies the GET and bytes while every other URL stays strictly mocked.
        const requests: {method: string | undefined; path: string | undefined; body: string}[] = []
        const server = createServer((request, response) => {
            const chunks: Buffer[] = []
            request.on("data", (chunk) => chunks.push(Buffer.from(chunk)))
            request.on("end", () => {
                const body = Buffer.concat(chunks).toString("utf8")
                requests.push({method: request.method, path: request.url, body})
                if (request.method !== "GET" || request.url !== REPORT_PATH || body) {
                    response.writeHead(400).end("Unexpected download request")
                    return
                }
                response.writeHead(200, {
                    "content-type": "application/pdf",
                    "content-disposition": 'attachment; filename="transmission_report.pdf"',
                    "content-length": REPORT_BYTES.length,
                })
                response.end(REPORT_BYTES)
            })
        })
        await new Promise<void>((resolve, reject) => {
            server.once("error", reject)
            server.listen(0, "127.0.0.1", () => {
                server.off("error", reject)
                resolve()
            })
        })
        const address = server.address()
        if (!address || typeof address === "string")
            throw new Error("Missing report listener address")
        const url = `http://127.0.0.1:${address.port}${REPORT_PATH}`
        // Depend on portal so this narrow handler follows its strict catch-all route.
        expect(portal.origin).toMatch(/^http:\/\/127\.0\.0\.1:/)
        await context.route(url, (route) => route.continue())
        try {
            await use({url})
        } finally {
            try {
                await context.unroute(url)
            } finally {
                server.closeAllConnections()
                await new Promise<void>((resolve, reject) =>
                    server.close((error) => (error ? reject(error) : resolve()))
                )
            }
            expect(requests).toEqual([{method: "GET", path: REPORT_PATH, body: ""}])
        }
    },
})

const SIGNATURE_DOCUMENT_ID = "67000000-0000-4000-8000-000000000001"
const REPORT_DOCUMENT_ID = "67000000-0000-4000-8000-000000000002"
const TASK_ID = "67000000-0000-4000-8000-000000000003"
const roles = [
    ...TALLY_ROLES,
    "miru-create",
    "miru-download",
    "miru-send",
    "miru-sign",
    "document-read",
    "document-upload",
    "area-read",
    "transmission-report-generate",
]
test.use({roles})

function task(type: string) {
    return {
        id: TASK_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: type,
        type,
        execution_status: "SUCCESS",
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: FIXED_TIME,
        logs: [],
        annotations: {},
        labels: {},
        executed_by_user: "synthetic-admin",
    }
}

async function transmissionWorld(portal: AdminPortal, signed = true, existing = true) {
    portal.settings.ACTIVATE_MIRU_EXPORT = true
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 1000
    const world = tallyWorld(portal)
    world.event.annotations = {
        "miru:sbei-users": JSON.stringify([{username: "synthetic-admin", miru_id: "SBEI-1"}]),
    }
    world.session.execution_status = "SUCCESS"
    world.session.is_execution_completed = true
    world.execution.status.elections_status = [
        {election_id: ELECTION_ID, status: "SUCCESS", progress: 100},
    ]
    const pack = {
        election_id: ELECTION_ID,
        area_id: AREA_ID,
        threshold: 1,
        servers: [
            {
                name: "Counting server",
                tag: "main",
                address: "https://counting.example.test/",
                public_key_pem: "synthetic-server-key",
            },
        ],
        documents: [
            {
                document_ids: {
                    eml: REPORT_DOCUMENT_ID,
                    xz: REPORT_DOCUMENT_ID,
                    all_servers: REPORT_DOCUMENT_ID,
                },
                transaction_id: "transaction-1",
                created_at: FIXED_TIME,
                signatures: signed
                    ? [
                          {
                              sbei_miru_id: "SBEI-1",
                              pub_key: "synthetic-public-key",
                              signature: "synthetic-signature",
                          },
                      ]
                    : [],
                servers_sent_to: [],
            },
        ],
        logs: [{created_date: FIXED_TIME, log_text: "Transmission package ready"}],
    }
    const persist = () => {
        world.session.annotations = {"miru:tally-session-data": JSON.stringify([pack])}
    }
    if (existing) persist()
    const dataset = resultsDataset(world)
    const area = {
        ...dataset.area[0],
        annotations: {"miru:area-trustee-users": JSON.stringify(["SBEI-1"])},
    }
    dataset.area = [area]
    table(portal, "sequent_backend_area", () => [area])
    const bytes = await sqlite(dataset, {
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: ELECTION_ID,
        results_event_id: RESULTS_ID,
    })
    const key = `${TENANT_ID}/${EVENT_ID}/miru-results.sqlite`
    portal.s3.putBytes("private", key, bytes, "application/vnd.sqlite3")
    world.documentUrls.set(SQLITE_DOCUMENT_ID, portal.s3.presign(key, "miru-results"))
    world.execution.results_event_id = RESULTS_ID
    world.execution.documents = {sqlite: SQLITE_DOCUMENT_ID}
    let currentTask = task("CREATE_TRANSMISSION_PACKAGE")
    portal.graphql.on("GetTaskById", () => ({
        data: {sequent_backend_tasks_execution: [currentTask]},
    }))
    portal.graphql.on("CreateTransmissionPackage", () => {
        persist()
        return {
            data: {
                create_transmission_package: {
                    error_msg: null,
                    task_execution: task("CREATE_TRANSMISSION_PACKAGE"),
                },
            },
        }
    })
    portal.graphql.on("SendTransmissionPackage", () => ({
        data: {send_transmission_package: {id: TALLY_ID}},
    }))
    portal.graphql.on("UploadSignature", () => {
        pack.documents[0].signatures = [
            {
                sbei_miru_id: "SBEI-1",
                pub_key: "synthetic-public-key",
                signature: "synthetic-signature",
            },
        ]
        persist()
        return {data: {upload_signature: {id: TALLY_ID}}}
    })
    const reportKey = `${TENANT_ID}/${EVENT_ID}/transmission-report.pdf`
    portal.s3.putBytes(
        "private",
        reportKey,
        Buffer.from("%PDF-1.7 transmission report"),
        "application/pdf"
    )
    const reportUrl = portal.s3.presign(reportKey, "transmission-report")
    world.documentUrls.set(REPORT_DOCUMENT_ID, reportUrl)
    portal.graphql.on("GetDocument", () => ({
        data: {
            sequent_backend_document: [
                {id: REPORT_DOCUMENT_ID, name: "transmission-report.pdf", annotations: {}},
            ],
        },
    }))
    portal.graphql.on("generate_transmission_report", () => {
        currentTask = task("GENERATE_TRANSMISSION_REPORT")
        return {
            data: {
                generate_transmission_report: {
                    document_id: REPORT_DOCUMENT_ID,
                    encryption_policy: "unencrypted",
                    task_execution: currentTask,
                },
            },
        }
    })
    return {world, pack, reportUrl}
}

async function openTransmission(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally", exact: true}).click()
    await page
        .getByRole("row")
        .filter({hasText: TALLY_ID})
        .locator('button:has(svg[aria-label="View Tally Ceremony"])')
        .click()
    await expect(page.getByText("Status: SUCCESS", {exact: true})).toBeVisible()
    await expect(
        page
            .getByRole("row", {name: /Alice Example/})
            .getByRole("gridcell", {name: "37", exact: true})
    ).toBeVisible()
    // Export controls are rendered in event, election, then contest order.
    await page.getByLabel("export election data", {exact: true}).nth(1).click()
    await page
        .getByRole("menuitem", {
            name: "Generate Transmission Package for Area 'North precinct'",
            exact: true,
        })
        .click()
    await expect(
        page.getByText(
            "Transmission Package for Area 'North precinct' and Election 'Harbour election'",
            {exact: true}
        )
    ).toBeVisible()
}

const transmissionVariables = {electionId: ELECTION_ID, tallySessionId: TALLY_ID, areaId: AREA_ID}
const notification = (page: Page, text: string) =>
    page.getByRole("alert", {includeHidden: true}).filter({hasText: text})

async function confirmTransmission(page: Page, portal: AdminPortal, response: GraphQLReply) {
    let respond!: () => void
    const responseReady = new Promise<void>((resolve) => (respond = resolve))
    portal.graphql.on("SendTransmissionPackage", async () => {
        await responseReady
        return response
    })
    const dialog = page.getByRole("dialog")
    try {
        await dialog.getByRole("button", {name: "Send Transmission Package", exact: true}).click()
        // The modal hides the sending button from role queries during its exit.
        // Observe a visible pending indicator before allowing the response.
        await expect(dialog).toBeHidden()
        await expect(
            page
                .getByRole("button", {name: "send transmission package", exact: true})
                .getByRole("progressbar")
        ).toBeVisible()
    } finally {
        respond()
    }
}

test("confirms transmission send and regeneration, then downloads its generated report", async ({
    page,
    portal,
    report,
}) => {
    const {world} = await transmissionWorld(portal)
    world.documentUrls.set(REPORT_DOCUMENT_ID, report.url)
    await openTransmission(page, portal)
    const sqliteKey = `${TENANT_ID}/${EVENT_ID}/miru-results.sqlite`
    expect(portal.s3.requestsFor(sqliteKey).map(({method, url}) => ({method, url}))).toEqual([
        {method: "GET", url: portal.s3.presign(sqliteKey, "miru-results")},
    ])
    expect(
        portal.graphql
            .callsTo("FetchDocument")
            .filter((call) => call.variables.documentId === SQLITE_DOCUMENT_ID)
            .map(({variables}) => variables)
    ).toEqual([{electionEventId: EVENT_ID, documentId: SQLITE_DOCUMENT_ID}])
    await expect(page.getByText("Transmission package ready", {exact: true})).toBeVisible()
    await page.getByRole("button", {name: "send transmission package", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await dialog.getByRole("button", {name: "Close", exact: true}).click()
    await expect(dialog).toHaveCount(0)
    expect(portal.graphql.callsTo("SendTransmissionPackage")).toEqual([])
    await page.getByRole("button", {name: "send transmission package", exact: true}).click()
    await confirmTransmission(page, portal, {
        data: {send_transmission_package: {id: TALLY_ID}},
    })
    await expect(notification(page, "Sending Transmission Package...")).toBeVisible()
    await expect(
        page
            .getByRole("button", {name: "send transmission package", exact: true})
            .getByRole("progressbar")
    ).toHaveCount(0)
    expect(
        portal.graphql.callsTo("SendTransmissionPackage").map(({variables}) => variables)
    ).toEqual([transmissionVariables])
    expect(portal.graphql.callsTo("SendTransmissionPackage")[0].headers["x-hasura-role"]).toBe(
        "miru-send"
    )
    await page.getByRole("button", {name: "regenerate transmission package", exact: true}).click()
    await page
        .getByRole("dialog")
        .getByRole("button", {name: "Regenerate Transmission Package", exact: true})
        .click()
    await expect(
        page
            .getByRole("heading", {name: "Task: Create Transmission Package SUCCESS", exact: true})
            .or(notification(page, "Creating Transmission Package..."))
            .first()
    ).toBeVisible()
    expect(
        portal.graphql.callsTo("CreateTransmissionPackage").map(({variables}) => variables)
    ).toEqual([{...transmissionVariables, electionEventId: EVENT_ID, force: true}])
    expect(portal.graphql.callsTo("CreateTransmissionPackage")[0].headers["x-hasura-role"]).toBe(
        "miru-create"
    )
    await page.getByLabel("export election data", {exact: true}).click()
    const download = page.waitForEvent("download")
    await page.getByRole("menuitem", {name: "Download Transmission Report", exact: true}).click()
    const file = await download
    expect(file.suggestedFilename()).toBe("transmission_report.pdf")
    expect(file.url()).toBe(report.url)
    const filePath = await file.path()
    expect(filePath).not.toBeNull()
    expect(await readFile(filePath!)).toEqual(REPORT_BYTES)
    expect(
        portal.graphql.callsTo("generate_transmission_report").map(({variables}) => variables)
    ).toEqual([
        {
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            electionId: ELECTION_ID,
            tallySessionId: TALLY_ID,
        },
    ])
    expect(portal.graphql.callsTo("generate_transmission_report")[0].headers["x-hasura-role"]).toBe(
        "transmission-report-generate"
    )
    expect(
        portal.graphql
            .callsTo("FetchDocument")
            .filter((call) => call.variables.documentId === REPORT_DOCUMENT_ID)
            .map(({variables}) => variables)
    ).toEqual([{electionEventId: EVENT_ID, documentId: REPORT_DOCUMENT_ID}])
})

test("uploads signature bytes and signs the selected area with the entered password", async ({
    page,
    portal,
}) => {
    await transmissionWorld(portal, false)
    const key = `${TENANT_ID}/${EVENT_ID}/trustee-signature.p12`
    const url = portal.s3.presign(key, "signature-upload")
    portal.s3.override(
        (request) => request.method === "PUT" && request.url === url,
        {status: 200},
        1
    )
    portal.graphql.on("GetUploadUrl", () => ({
        data: {get_upload_url: {url, document_id: SIGNATURE_DOCUMENT_ID}},
    }))
    await openTransmission(page, portal)
    await expect(
        page.getByRole("button", {name: "send transmission package", exact: true})
    ).toBeDisabled()
    const signature = Buffer.from("synthetic signature certificate")
    const uploaded = page.waitForRequest(
        (request) => request.url() === url && request.method() === "PUT"
    )
    await page.getByLabel("Drop Input File").setInputFiles({
        name: "trustee-signature.p12",
        mimeType: "application/x-pkcs12",
        buffer: signature,
    })
    const request = await uploaded
    expect(request.headers()["content-type"]).toBe("application/x-pkcs12")
    expect(request.postDataBuffer()).toEqual(signature)
    expect(portal.graphql.callsTo("GetUploadUrl").map(({variables}) => variables)).toEqual([
        {
            name: "trustee-signature.p12",
            media_type: "application/x-pkcs12",
            size: signature.length,
            is_public: false,
            election_event_id: EVENT_ID,
        },
    ])
    expect(portal.graphql.callsTo("GetUploadUrl")[0].headers["x-hasura-role"]).toBe(
        "document-upload"
    )
    const dialog = page.getByRole("dialog")
    await dialog.getByLabel("Enter your password").fill("synthetic-passphrase")
    await dialog.getByRole("button", {name: "Sign Transmission Package", exact: true}).click()
    await expect(notification(page, "Signing Successful")).toBeVisible()
    expect(portal.graphql.callsTo("UploadSignature").map(({variables}) => variables)).toEqual([
        {
            ...transmissionVariables,
            documentId: SIGNATURE_DOCUMENT_ID,
            password: "synthetic-passphrase",
        },
    ])
    expect(portal.graphql.callsTo("UploadSignature")[0].headers["x-hasura-role"]).toBe("miru-sign")
    await expect(
        page.getByRole("button", {name: "send transmission package", exact: true})
    ).toBeEnabled()
})

test("creates a transmission package for an area that has none", async ({page, portal}) => {
    await transmissionWorld(portal, true, false)
    await openTransmission(page, portal)
    expect(
        portal.graphql.callsTo("CreateTransmissionPackage").map(({variables}) => variables)
    ).toEqual([{...transmissionVariables, electionEventId: EVENT_ID, force: false}])
    expect(portal.graphql.callsTo("CreateTransmissionPackage")[0].headers["x-hasura-role"]).toBe(
        "miru-create"
    )
    await expect(
        page.getByRole("button", {name: "send transmission package", exact: true})
    ).toBeEnabled()
})

for (const failure of ["action error", "gateway error"] as const) {
    test(`ends the sending indicator after a transmission ${failure}`, async ({page, portal}) => {
        await transmissionWorld(portal)
        // Keep background tally refreshes out of the mutation's error cleanup.
        portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 3_600_000
        await openTransmission(page, portal)
        const send = page.getByRole("button", {name: "send transmission package", exact: true})
        await send.click()
        await confirmTransmission(
            page,
            portal,
            failure === "action error"
                ? {errors: [{message: "transmission rejected"}]}
                : {status: 503, body: "Gateway unavailable", contentType: "text/plain"}
        )
        await expect(notification(page, "Error sending Transmission Package")).toBeVisible()
        expect(
            portal.graphql.callsTo("SendTransmissionPackage").map(({variables}) => variables)
        ).toEqual([transmissionVariables])
        expect(portal.graphql.callsTo("SendTransmissionPackage")[0].headers["x-hasura-role"]).toBe(
            "miru-send"
        )
        await expect(send.getByRole("progressbar")).toHaveCount(0, {timeout: 2000})
    })
}

test("reports a rejected signature and closes its password dialog", async ({page, portal}) => {
    await transmissionWorld(portal, false)
    const key = `${TENANT_ID}/${EVENT_ID}/rejected-signature.p12`
    const url = portal.s3.presign(key, "rejected-signature-upload")
    portal.s3.override(
        (request) => request.method === "PUT" && request.url === url,
        {status: 200},
        1
    )
    portal.graphql.on("GetUploadUrl", () => ({
        data: {get_upload_url: {url, document_id: SIGNATURE_DOCUMENT_ID}},
    }))
    portal.graphql.on("UploadSignature", () => ({
        errors: [{message: "incorrect certificate password"}],
    }))
    await openTransmission(page, portal)
    const signature = Buffer.from("synthetic rejected certificate")
    const uploaded = page.waitForRequest(
        (request) => request.url() === url && request.method() === "PUT"
    )
    await page.getByLabel("Drop Input File").setInputFiles({
        name: "rejected-signature.p12",
        mimeType: "application/x-pkcs12",
        buffer: signature,
    })
    expect((await uploaded).postDataBuffer()).toEqual(signature)
    const dialog = page.getByRole("dialog")
    await dialog.getByLabel("Enter your password").fill("incorrect-passphrase")
    await dialog.getByRole("button", {name: "Sign Transmission Package", exact: true}).click()
    await expect(
        page.getByText("There was an error uploading signature", {exact: true})
    ).toBeVisible()
    await expect(dialog).toHaveCount(0)
    expect(portal.graphql.callsTo("UploadSignature").map(({variables}) => variables)).toEqual([
        {
            ...transmissionVariables,
            documentId: SIGNATURE_DOCUMENT_ID,
            password: "incorrect-passphrase",
        },
    ])
    await expect(
        page.getByRole("button", {name: "send transmission package", exact: true})
    ).toBeDisabled()
})

test("shows a failed task when transmission report generation is rejected", async ({
    page,
    portal,
}) => {
    await transmissionWorld(portal)
    portal.graphql.on("generate_transmission_report", () => ({
        errors: [{message: "report generation rejected"}],
    }))
    await openTransmission(page, portal)
    await page.getByLabel("export election data", {exact: true}).click()
    await page.getByRole("menuitem", {name: "Download Transmission Report", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("generate_transmission_report").length).toBe(1)
    await expect(
        page.getByRole("heading", {name: /Task: Generate Transmission Report/})
    ).toBeVisible()

    expect(
        portal.graphql.callsTo("generate_transmission_report").map(({variables}) => variables)
    ).toEqual([
        {
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            electionId: ELECTION_ID,
            tallySessionId: TALLY_ID,
        },
    ])
    expect(
        portal.graphql
            .callsTo("FetchDocument")
            .filter((call) => call.variables.documentId === REPORT_DOCUMENT_ID)
    ).toEqual([])
    await expect(
        page.getByRole("heading", {name: "Task: Generate Transmission Report FAILED", exact: true})
    ).toBeVisible({timeout: 2000})
})

test.describe("an assigned trustee without action privileges", () => {
    test.use({roles: [...TALLY_ROLES, "area-read"]})
    test("sees package status without send, regenerate, download or signature controls", async ({
        page,
        portal,
    }) => {
        await transmissionWorld(portal)
        await openTransmission(page, portal)
        await expect(
            page.getByRole("button", {
                name: "SBEI Signatures 1 out of 2 Signed, 1 minimum",
                exact: true,
            })
        ).toBeVisible()
        await expect(
            page.getByRole("button", {name: "send transmission package", exact: true})
        ).toHaveCount(0)
        await expect(
            page.getByRole("button", {name: "regenerate transmission package", exact: true})
        ).toHaveCount(0)
        await expect(page.getByLabel("export election data", {exact: true})).toHaveCount(0)
        for (const operation of [
            "CreateTransmissionPackage",
            "SendTransmissionPackage",
            "GetUploadUrl",
            "UploadSignature",
            "generate_transmission_report",
        ])
            expect(portal.graphql.callsTo(operation)).toEqual([])
        await expect(page.getByLabel("Drop Input File")).toHaveCount(0, {timeout: 2000})
    })
})
