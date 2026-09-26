// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, TENANT_ID} from "../fixtures"
import type {AdminPortal} from "../fixtures"
import {
    ADMIN_ID,
    AREA_ID,
    CONTEST_ID,
    ELECTION_ID,
    EVENT_ID,
    FIXED_TIME,
    byId,
    registerEvent,
    registerUsers,
    sha256,
    table,
    type Row,
} from "./data"

const IMPORT_ID = "a1000000-0000-4000-8000-000000000001"
const PREVIOUS_IMPORT_ID = "a1000000-0000-4000-8000-000000000002"
const DOCUMENT_ID = "a2000000-0000-4000-8000-000000000001"
const SHEET_ID = "a3000000-0000-4000-8000-000000000001"
const UPLOAD_KEY = `${TENANT_ID}/${EVENT_ID}/uploads/harbour-paper.csv`
const CSV = [
    "area,contest,channel,candidate,votes",
    "North,MAYOR,PAPER,ALICE,41",
    "North,MAYOR,PAPER,BOB,17",
    "",
].join("\n")
const INCOMING_CSV = "candidate,votes\nALICE,41\nBOB,17\n"

const allRoles = [
    "admin-user",
    "election-event-read",
    "tally-sheet-import-view",
    "tally-sheet-import-create",
    "tally-sheet-import-review",
]
test.use({roles: allRoles})

const summary = (overrides: Row = {}) => ({
    imported_ballot_box_count: 1,
    changed_ballot_box_count: 0,
    new_ballot_box_count: 1,
    unchanged_ballot_box_count: 0,
    conflicted_ballot_box_count: 0,
    validation_error_count: 0,
    ...overrides,
})

function importRecord(overrides: Row = {}): Row {
    return {
        id: IMPORT_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        source_document_id: DOCUMENT_ID,
        source_file_name: "harbour-paper.csv",
        source_format: "CANONICAL_CSV",
        selected_channel: "PAPER",
        status: "PENDING_REVIEW",
        source_sha256: sha256(CSV),
        canonical_csv_sha256: sha256(INCOMING_CSV),
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        created_by_user_id: ADMIN_ID,
        summary: summary(),
        validation_report: [],
        labels: {},
        annotations: {},
        ...overrides,
    }
}

function importItem(overrides: Row = {}): Row {
    return {
        id: "a4000000-0000-4000-8000-000000000001",
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        import_id: IMPORT_ID,
        election_id: ELECTION_ID,
        area_id: AREA_ID,
        contest_id: CONTEST_ID,
        channel: "PAPER",
        generated_tally_sheet_id: SHEET_ID,
        baseline_approved_tally_sheet_id: null,
        baseline_approved_version: null,
        change_type: "NEW",
        status: "PENDING_REVIEW",
        previous_csv: null,
        incoming_csv: INCOMING_CSV,
        incoming_content_hash: sha256(INCOMING_CSV),
        source_refs: {
            area_name: "North",
            contest_external_id: "MAYOR",
            candidate_external_ids: ["ALICE", "BOB"],
        },
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        ...overrides,
    }
}

function preview(overrides: Row = {}): Row {
    return {
        document_id: DOCUMENT_ID,
        source_format: "CANONICAL_CSV",
        selected_channel: "PAPER",
        summary: summary(),
        items: [
            {
                channel: "PAPER",
                area_id: AREA_ID,
                area_name: "North",
                contest_id: CONTEST_ID,
                contest_name: "Mayor",
                election_id: ELECTION_ID,
                baseline_tally_sheet_id: null,
                baseline_version: null,
                previous_csv: null,
                incoming_csv: INCOMING_CSV,
                incoming_content_hash: sha256(INCOMING_CSV),
                change_type: "NEW",
            },
        ],
        validation_errors: [],
        ...overrides,
    }
}

/** An event with its import history, item rows and the upload URL service. */
function importsService(portal: AdminPortal, initial: Row[] = []) {
    registerEvent(portal)
    registerUsers(portal)
    const state = {imports: initial, items: [importItem()], duplicates: [] as Row[]}
    portal.graphql.on("sequent_backend_tally_sheet_import", ({variables}) => {
        const where = JSON.stringify(variables.where ?? {})
        const byHash = where.includes("source_sha256")
        const rows = byId(byHash ? state.duplicates : state.imports, variables.where)
        return {
            data: {
                sequent_backend_tally_sheet_import: rows,
                sequent_backend_tally_sheet_import_aggregate: {aggregate: {count: rows.length}},
            },
        }
    })
    table(portal, "sequent_backend_tally_sheet_import_item", () => state.items)
    portal.graphql.on("GetUploadUrl", () => ({
        data: {
            get_upload_url: {
                url: portal.s3.url("private", UPLOAD_KEY, {"X-Amz-Signature": "upload"}),
                document_id: DOCUMENT_ID,
            },
        },
    }))
    return state
}

async function openImportsTab(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally sheet imports", exact: true}).click()
    await expect(page.getByText(/^Import ES&S or CSV tally sheet files/)).toBeVisible()
}

async function chooseFile(page: Page) {
    const drawer = page.getByRole("presentation").filter({hasText: "Import tally sheets"})
    await drawer.locator('input[type="file"]').setInputFiles({
        name: "harbour-paper.csv",
        mimeType: "text/csv",
        buffer: Buffer.from(CSV),
    })
    return drawer
}

test("uploads a canonical CSV, previews it, saves the import and approves it", async ({
    page,
    portal,
}) => {
    const state = importsService(portal)
    portal.s3.override(
        (request) => request.method === "PUT" && request.key === UPLOAD_KEY,
        {status: 200},
        1
    )
    const uploads: {headers: Record<string, string>; body: string}[] = []
    page.on("request", (request) => {
        if (request.method() === "PUT")
            uploads.push({headers: request.headers(), body: request.postData() ?? ""})
    })
    portal.graphql.on("PreviewTallySheetImport", () => ({
        data: {preview_tally_sheet_import: {preview: preview()}},
    }))
    portal.graphql.on("CreateTallySheetImport", () => {
        state.imports = [importRecord()]
        return {data: {create_tally_sheet_import: {import: importRecord()}}}
    })
    portal.graphql.on("ReviewTallySheetImport", () => {
        state.imports = [importRecord({status: "APPROVED"})]
        state.items = [importItem({status: "APPROVED"})]
        return {data: {review_tally_sheet_import: {import: importRecord({status: "APPROVED"})}}}
    })

    await openImportsTab(page, portal)
    await expect(page.getByText("No tally sheet imports yet.", {exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Import tally sheets", exact: true}).click()
    const drawer = await chooseFile(page)
    await drawer.getByRole("combobox").first().click()
    await page.getByRole("option", {name: "Canonical CSV", exact: true}).click()
    await expect(drawer.getByRole("button", {name: "Save import", exact: true})).toBeDisabled()
    await drawer.getByRole("button", {name: "Preview", exact: true}).click()

    await expect(drawer.getByText("North / Mayor", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("GetUploadUrl").map(({variables}) => variables)).toEqual([
        {
            name: "harbour-paper.csv",
            media_type: "text/csv",
            size: Buffer.byteLength(CSV),
            is_public: false,
            election_event_id: EVENT_ID,
        },
    ])
    expect(uploads).toEqual([
        {headers: expect.objectContaining({"content-type": "text/csv"}), body: CSV},
    ])
    expect(portal.s3.requestsFor(UPLOAD_KEY)).toMatchObject([{method: "PUT", status: 200}])
    const importVariables = {
        electionEventId: EVENT_ID,
        documentId: DOCUMENT_ID,
        sha256: sha256(CSV),
        sourceFormat: "CANONICAL_CSV",
        selectedChannel: "PAPER",
    }
    expect(
        portal.graphql.callsTo("PreviewTallySheetImport").map(({variables}) => variables)
    ).toEqual([importVariables])
    await drawer.getByText("North / Mayor", {exact: true}).click()
    await expect(drawer.getByText("ALICE,41", {exact: false})).toBeVisible()
    expect(portal.graphql.callsTo("CreateTallySheetImport")).toEqual([])

    await drawer.getByRole("button", {name: "Save import", exact: true}).click()
    await expect(page.getByText("Tally sheet import created", {exact: true})).toBeVisible()
    expect(
        portal.graphql.callsTo("CreateTallySheetImport").map(({variables}) => variables)
    ).toEqual([importVariables])
    const row = page.getByRole("row").filter({hasText: "harbour-paper.csv"})
    await expect(row).toContainText("Canonical CSV")
    await expect(row).toContainText("Paper")
    await expect(row).toContainText("Pending review")
    await expect(row).toContainText("harbour-admin")

    await row.getByRole("button", {name: "Review", exact: true}).click()
    const detail = page.getByRole("presentation").filter({hasText: "Tally sheet import"})
    await expect(detail.getByText(IMPORT_ID, {exact: true})).toBeVisible()
    await expect(detail.getByText("North / MAYOR", {exact: true})).toBeVisible()
    await detail.getByText("North / MAYOR", {exact: true}).click()
    await expect(detail.getByText(`Generated tally sheet: ${SHEET_ID}`)).toBeVisible()
    await expect(detail.getByText("Source candidate IDs: ALICE, BOB")).toBeVisible()
    expect(portal.graphql.callsTo("ReviewTallySheetImport")).toEqual([])
    await detail.getByRole("button", {name: "Approve", exact: true}).click()
    await expect(page.getByText("Import approved", {exact: true})).toBeVisible()
    expect(
        portal.graphql.callsTo("ReviewTallySheetImport").map(({variables}) => variables)
    ).toEqual([{electionEventId: EVENT_ID, importId: IMPORT_ID, decision: "APPROVE"}])
    await expect(detail.getByRole("button", {name: "Approve", exact: true})).toHaveCount(0)
    await expect(detail.getByText("Approved", {exact: true}).first()).toBeVisible()
})

test("reports upload failures and keeps a preview with validation errors from being saved", async ({
    page,
    portal,
}) => {
    importsService(portal)
    portal.graphql.once("GetUploadUrl", () => ({errors: [{message: "upload quota exceeded"}]}))
    portal.s3.override((request) => request.method === "PUT", {status: 503}, 1)
    portal.s3.override((request) => request.method === "PUT", {status: 200}, 1)
    portal.graphql.on("PreviewTallySheetImport", () => ({
        data: {
            preview_tally_sheet_import: {
                preview: preview({
                    summary: summary({validation_error_count: 1}),
                    validation_errors: [
                        {
                            code: "total_votes_exceeds_census",
                            message: "total votes exceed census",
                            area_name: "North",
                            params: {totalVotes: "58", census: "50"},
                        },
                    ],
                }),
            },
        },
    }))

    await openImportsTab(page, portal)
    await page.getByRole("button", {name: "Import tally sheets", exact: true}).click()
    const drawer = page.getByRole("presentation").filter({hasText: "Import tally sheets"})
    await expect(drawer.getByRole("button", {name: "Preview", exact: true})).toBeDisabled()
    await chooseFile(page)
    const previewButton = drawer.getByRole("button", {name: "Preview", exact: true})
    await previewButton.click()
    await expect(page.getByText("upload quota exceeded", {exact: true})).toBeVisible()
    expect(portal.s3.requestsFor(UPLOAD_KEY)).toEqual([])

    await previewButton.click()
    await expect(page.getByText("Could not upload import file", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("PreviewTallySheetImport")).toEqual([])

    await previewButton.click()
    await expect(
        drawer.getByText("Total votes (58) must not be greater than census (50)", {exact: true})
    ).toBeVisible()
    expect(portal.graphql.callsTo("PreviewTallySheetImport")[0].variables).toMatchObject({
        sourceFormat: "ESS_ENHANCED_XML",
        selectedChannel: "PAPER",
    })
    await expect(drawer.getByRole("button", {name: "Save import", exact: true})).toBeDisabled()
    await drawer.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(drawer).toHaveCount(0)
    expect(portal.graphql.callsTo("CreateTallySheetImport")).toEqual([])
})

test("warns when the source hash was imported before and opens the earlier import", async ({
    page,
    portal,
}) => {
    const earlier = importRecord({id: PREVIOUS_IMPORT_ID, status: "APPROVED"})
    const state = importsService(portal, [earlier])
    state.duplicates = [earlier]
    portal.s3.override((request) => request.method === "PUT", {status: 200}, 1)
    portal.graphql.on("PreviewTallySheetImport", () => ({
        data: {
            preview_tally_sheet_import: {
                preview: preview({
                    selected_channel: "POSTAL",
                    summary: summary({new_ballot_box_count: 0, unchanged_ballot_box_count: 1}),
                }),
            },
        },
    }))

    await openImportsTab(page, portal)
    await expect(page.getByRole("row").filter({hasText: "harbour-paper.csv"})).toContainText(
        "Approved"
    )
    await page.getByRole("button", {name: "Import tally sheets", exact: true}).click()
    const drawer = await chooseFile(page)
    await drawer.getByRole("combobox").nth(1).click()
    await page.getByRole("option", {name: "Postal", exact: true}).click()
    await drawer.getByRole("button", {name: "Preview", exact: true}).click()
    await expect(
        drawer.getByText(
            "This source file hash already appears in a previous tally sheet import.",
            {exact: true}
        )
    ).toBeVisible()
    expect(portal.graphql.callsTo("PreviewTallySheetImport")[0].variables).toMatchObject({
        sourceFormat: "ESS_ENHANCED_XML",
        selectedChannel: "POSTAL",
    })
    const lookup = portal.graphql
        .callsTo("sequent_backend_tally_sheet_import")
        .find((call) => JSON.stringify(call.variables).includes("source_sha256"))
    expect(lookup?.variables).toMatchObject({
        where: {
            _and: [
                {tenant_id: {_eq: TENANT_ID}},
                {election_event_id: {_eq: EVENT_ID}},
                {source_sha256: {_ilike: `%${sha256(CSV)}%`}},
            ],
        },
        limit: 1,
    })
    await drawer.getByRole("button", {name: "Open existing", exact: true}).click()
    const detail = page.getByRole("presentation").filter({hasText: "Tally sheet import"})
    await expect(detail.getByText(PREVIOUS_IMPORT_ID, {exact: true})).toBeVisible()
    await expect(detail.getByRole("button", {name: "Approve", exact: true})).toHaveCount(0)
    expect(portal.graphql.callsTo("CreateTallySheetImport")).toEqual([])
})

test("opens an import from a link, surfaces a stale baseline conflict and disapproves it", async ({
    page,
    portal,
}) => {
    const state = importsService(portal, [
        importRecord({
            status: "CONFLICTED",
            summary: summary({conflicted_ballot_box_count: 1}),
            validation_report: [
                {code: "stale_baseline", message: "North MAYOR changed since preview"},
            ],
        }),
    ])
    state.items = [
        importItem({
            change_type: "CHANGED",
            status: "CONFLICTED",
            previous_csv: "candidate,votes\nALICE,40\nBOB,17\n",
        }),
    ]
    const decisions: string[] = []
    portal.graphql.on("ReviewTallySheetImport", ({variables}) => {
        decisions.push(String(variables.decision))
        const status = variables.decision === "APPROVE" ? "CONFLICTED" : "DISAPPROVED"
        state.imports = [importRecord({status})]
        return {data: {review_tally_sheet_import: {import: importRecord({status})}}}
    })

    await page.goto(
        `${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en&tallySheetImportId=${IMPORT_ID}`
    )
    const detail = page.getByRole("presentation").filter({hasText: "Tally sheet import"})
    await expect(detail.getByText(IMPORT_ID, {exact: true})).toBeVisible()
    expect(
        portal.graphql
            .callsTo("sequent_backend_tally_sheet_import")
            .some(
                ({variables}) => JSON.stringify(variables.where) === `{"id":{"_eq":"${IMPORT_ID}"}}`
            )
    ).toBe(true)
    await expect(detail.getByText("North MAYOR changed since preview", {exact: true})).toBeVisible()
    await expect(detail.getByText("Created by: harbour-admin")).toBeVisible()
    await detail.getByText("North / MAYOR", {exact: true}).click()
    await expect(detail.getByText("ALICE,40", {exact: true})).toBeVisible()
    await expect(detail.getByText("ALICE,41", {exact: true})).toBeVisible()

    await detail.getByRole("button", {name: "Approve", exact: true}).click()
    await expect(page.getByText("Import has stale baseline conflicts", {exact: true})).toBeVisible()
    await detail.getByRole("button", {name: "Disapprove", exact: true}).click()
    await expect(page.getByText("Import disapproved", {exact: true})).toBeVisible()
    await expect(detail.getByRole("button", {name: "Disapprove", exact: true})).toHaveCount(0)
    expect(decisions).toEqual(["APPROVE", "DISAPPROVE"])
    expect(
        portal.graphql.callsTo("ReviewTallySheetImport").map(({variables}) => variables.importId)
    ).toEqual([IMPORT_ID, IMPORT_ID])
    await detail.getByRole("button", {name: "Close", exact: true}).click()
    await expect(detail).toHaveCount(0)
    await expect(page.getByRole("tab", {name: "Tally sheet imports"})).toHaveAttribute(
        "aria-selected",
        "true"
    )
})

/** A stored source file and a FetchDocument reply that presigns it once. */
function sourceDocument(portal: AdminPortal) {
    const key = `${TENANT_ID}/${EVENT_ID}/documents/${DOCUMENT_ID}`
    portal.s3.putBytes("private", key, Buffer.from(CSV), "text/csv")
    const url = portal.s3.presign(key, "source")
    portal.graphql.once("FetchDocument", () => ({data: {fetchDocument: {url}}}))
    return url
}

test("downloads an import's source file from a presigned URL", async ({page, portal}) => {
    importsService(portal, [importRecord()])
    const url = sourceDocument(portal)

    await openImportsTab(page, portal)
    const download = page.waitForEvent("download")
    await page
        .getByRole("row")
        .filter({hasText: "harbour-paper.csv"})
        .getByRole("button", {name: "Source", exact: true})
        .click()
    // Chromium cancels downloads answered by a route, so the presigned URL is the contract.
    const file = await download
    expect(file.suggestedFilename()).toBe("harbour-paper.csv")
    expect(file.url()).toBe(url)
    expect(portal.graphql.callsTo("FetchDocument").map(({variables}) => variables)).toEqual([
        {electionEventId: EVENT_ID, documentId: DOCUMENT_ID},
    ])
})

test("reports a failed source URL request instead of downloading the previous URL", async ({
    page,
    portal,
}) => {
    importsService(portal, [importRecord()])
    sourceDocument(portal)
    portal.graphql.once("FetchDocument", () => ({errors: [{message: "document expired"}]}))

    await openImportsTab(page, portal)
    const source = page
        .getByRole("row")
        .filter({hasText: "harbour-paper.csv"})
        .getByRole("button", {name: "Source", exact: true})
    const first = page.waitForEvent("download")
    await source.click()
    await first
    const downloads: string[] = []
    page.on("download", (download) => downloads.push(download.url()))
    await source.click()
    await expect.poll(() => portal.graphql.callsTo("FetchDocument").length).toBe(2)
    expect(portal.graphql.callsTo("FetchDocument").map(({variables}) => variables)).toEqual([
        {electionEventId: EVENT_ID, documentId: DOCUMENT_ID},
        {electionEventId: EVENT_ID, documentId: DOCUMENT_ID},
    ])
    test.fail(true, "a FetchDocument error keeps the previous result, so the stale URL downloads")
    await expect(
        page.getByText(/^(document expired|Could not create source download URL)$/)
    ).toBeVisible({timeout: 5000})
    expect(downloads).toEqual([])
})

test("names the import format and channel selectors after their labels", async ({page, portal}) => {
    importsService(portal)
    await openImportsTab(page, portal)
    await page.getByRole("button", {name: "Import tally sheets", exact: true}).click()
    const drawer = page.getByRole("presentation").filter({hasText: "Import tally sheets"})
    await expect(drawer.getByRole("combobox").first()).toHaveText("ES&S Enhanced XML")
    test.fail(
        true,
        "the Format and Channel selects have no labelId, so their comboboxes are unnamed"
    )
    await expect(drawer.getByRole("combobox", {name: "Format"})).toHaveText("ES&S Enhanced XML", {
        timeout: 2000,
    })
    await expect(drawer.getByRole("combobox", {name: "Channel"})).toHaveText("Paper")
})

test.describe("with view-only import permission", () => {
    test.use({roles: ["admin-user", "election-event-read", "tally-sheet-import-view"]})

    test("lists imports without offering import or review actions", async ({page, portal}) => {
        importsService(portal, [importRecord()])
        await openImportsTab(page, portal)
        const row = page.getByRole("row").filter({hasText: "harbour-paper.csv"})
        await expect(row).toContainText("Pending review")
        await expect(page.getByRole("button", {name: "Import tally sheets"})).toHaveCount(0)
        await row.getByRole("button", {name: "Review", exact: true}).click()
        const detail = page.getByRole("presentation").filter({hasText: "Tally sheet import"})
        await expect(detail.getByText(IMPORT_ID, {exact: true})).toBeVisible()
        await expect(detail.getByRole("button", {name: "Close", exact: true})).toBeVisible()
        await expect(detail.getByRole("button", {name: "Approve"})).toHaveCount(0)
        await expect(detail.getByRole("button", {name: "Disapprove"})).toHaveCount(0)
    })
})
