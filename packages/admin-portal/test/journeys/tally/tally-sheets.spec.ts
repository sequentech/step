// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Locator, Page} from "@playwright/test"
import {test, expect, TENANT_ID} from "../fixtures"
import type {AdminPortal} from "../fixtures"
import {
    ADMIN_ID,
    AREA_ID,
    CONTEST_ID,
    ELECTION_ID,
    EVENT_ID,
    FIXED_TIME,
    recordRejections,
    registerEvent,
    registerUsers,
    table,
    type Row,
} from "./data"

const ALICE_ID = "b1000000-0000-4000-8000-000000000001"
const BOB_ID = "b1000000-0000-4000-8000-000000000002"
const SHEET_V1 = "b2000000-0000-4000-8000-000000000001"
const SHEET_V2 = "b2000000-0000-4000-8000-000000000002"
const SHEET_V3 = "b2000000-0000-4000-8000-000000000003"
const IMPORT_ID = "b3000000-0000-4000-8000-000000000001"
const scope = {tenant_id: TENANT_ID, election_event_id: EVENT_ID, election_id: ELECTION_ID}

test.use({
    roles: [
        "admin-user",
        "election-event-read",
        "election-read",
        "tally-sheet-view",
        "tally-sheet-create",
        "tally-sheet-review",
        "tally-sheet-import-view",
    ],
})

const content = (alice: number, bob: number) => ({
    area_id: AREA_ID,
    contest_id: CONTEST_ID,
    total_votes: alice + bob + 2,
    total_valid_votes: alice + bob,
    invalid_votes: {implicit_invalid: 1, explicit_invalid: 1, total_invalid: 2},
    total_blank_votes: 0,
    blank_ballots: 0,
    census: 40,
    candidate_results: {
        [ALICE_ID]: {candidate_id: ALICE_ID, total_votes: alice},
        [BOB_ID]: {candidate_id: BOB_ID, total_votes: bob},
    },
})

function sheet(id: string, version: number, status: string, overrides: Row = {}): Row {
    return {
        ...scope,
        id,
        version,
        status,
        area_id: AREA_ID,
        contest_id: CONTEST_ID,
        channel: "PAPER",
        content: content(5, 3),
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        created_by_user_id: ADMIN_ID,
        reviewed_at: status === "PENDING" ? null : FIXED_TIME,
        reviewed_by_user_id: status === "PENDING" ? null : ADMIN_ID,
        deleted_at: null,
        labels: {},
        annotations: {},
        import_id: null,
        ...overrides,
    }
}

/** An election with one ballot box (North precinct, Mayor, paper) and its sheet versions. */
function ballotBox(portal: AdminPortal) {
    registerEvent(portal)
    const election = {
        ...scope,
        id: ELECTION_ID,
        name: "Harbour election",
        alias: "Harbour election",
        presentation: {i18n: {en: {name: "Harbour election"}}},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        labels: {},
        annotations: {},
        status: {is_published: true, voting_status: "CLOSED"},
    }
    table(portal, "sequent_backend_election", () => [election])
    const contest = {
        ...scope,
        id: CONTEST_ID,
        name: "Mayor",
        presentation: {i18n: {en: {name: "Mayor"}}},
        counting_algorithm: "plurality-at-large",
        min_votes: 0,
        max_votes: 1,
        num_winners: 1,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    table(portal, "sequent_backend_contest", () => [contest])
    table(portal, "sequent_backend_candidate", () =>
        [
            [ALICE_ID, "Alice Example"],
            [BOB_ID, "Bob Example"],
        ].map(([id, name]) => ({
            ...scope,
            id,
            contest_id: CONTEST_ID,
            name,
            presentation: {i18n: {en: {name}}},
            created_at: FIXED_TIME,
            last_updated_at: FIXED_TIME,
        }))
    )
    const area = {
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        id: AREA_ID,
        name: "North precinct",
        parent_id: null,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    table(portal, "sequent_backend_area", () => [area])
    table(portal, "sequent_backend_area_contest", () => [
        {
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            id: "b4000000-0000-4000-8000-000000000001",
            area_id: AREA_ID,
            contest_id: CONTEST_ID,
            area,
        },
    ])
    portal.graphql.on("sequent_backend_contest_extended", () => ({
        data: {sequent_backend_area_contest: [{area}]},
    }))
    const state = {
        sheets: [
            sheet(SHEET_V2, 2, "PENDING", {content: content(6, 3), import_id: IMPORT_ID}),
            sheet(SHEET_V1, 1, "APPROVED"),
        ],
    }
    table(portal, "sequent_backend_tally_sheet", () => state.sheets)
    return state
}

/** Row actions are unnamed icon buttons; their tooltip label sits on an aria-hidden icon. */
const rowAction = (row: Locator, label: string) =>
    row.locator(`button:has(svg[aria-label="${label}"])`)

async function replace(input: Locator, value: string) {
    await input.fill("")
    await input.pressSequentially(value)
}

async function openTallySheets(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}/sequent_backend_election/${ELECTION_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally Sheets", exact: true}).click()
    const row = page.getByRole("row").filter({hasText: "North precinct"})
    await expect(row).toContainText("Mayor")
    return row
}

test("lists ballot boxes and saves a new version from the latest one", async ({page, portal}) => {
    const state = ballotBox(portal)
    portal.graphql.on("CreateNewTallySheet", ({variables}) => {
        const created = sheet(SHEET_V3, 3, "PENDING", {content: variables.content})
        state.sheets = [created, ...state.sheets]
        return {data: {create_new_tally_sheet: created}}
    })

    const row = await openTallySheets(page, portal)
    await expect(row).toContainText("PAPER")
    await expect(row.getByRole("cell").nth(3)).toHaveText("2")
    await expect(row.getByRole("cell").nth(4)).toHaveText("1")
    expect(
        portal.graphql.callsTo("sequent_backend_tally_sheet").map(({variables}) => variables)
    ).toContainEqual(expect.objectContaining({distinct_on: ["area_id", "contest_id", "channel"]}))

    await rowAction(row, "Add").click()
    const alice = page.getByText("Alice Example", {exact: true}).locator("..")
    await expect(alice.getByRole("textbox", {name: "Total Votes"})).toHaveValue("6")
    await replace(alice.getByRole("textbox", {name: "Total Votes"}), "7")
    await replace(page.getByRole("textbox", {name: "Total Valid Votes"}), "10")
    await page.getByRole("button", {name: "Confirm", exact: true}).click()
    // The wizard waits 400 ms for the form to store its draft before moving on.
    await page.clock.runFor(400)
    await expect(page.getByRole("button", {name: "Save", exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("CreateNewTallySheet")).toEqual([])
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await expect(page.getByText("Tally Sheet saved", {exact: true})).toBeVisible()
    const calls = portal.graphql.callsTo("CreateNewTallySheet")
    expect(calls.map(({variables}) => variables)).toEqual([
        {
            electionEventId: EVENT_ID,
            channel: "PAPER",
            contestId: CONTEST_ID,
            areaId: AREA_ID,
            content: {...content(7, 3), total_votes: 12},
        },
    ])
    expect(calls[0].headers["x-hasura-role"]).toBe("tally-sheet-create")
    await expect(row.getByRole("cell").nth(3)).toHaveText("3")
})

function importReference(portal: AdminPortal) {
    const key = `${TENANT_ID}/${EVENT_ID}/documents/north-paper.xml`
    portal.s3.putBytes("private", key, Buffer.from("<TallySheets/>"), "text/xml")
    table(portal, "sequent_backend_tally_sheet_import", () => [
        {
            ...scope,
            id: IMPORT_ID,
            status: "APPROVED",
            source_document_id: "b5000000-0000-4000-8000-000000000001",
            source_file_name: "north-paper.xml",
            source_format: "ESS_ENHANCED_XML",
            selected_channel: "PAPER",
            summary: {},
            created_at: FIXED_TIME,
            last_updated_at: FIXED_TIME,
            created_by_user_id: ADMIN_ID,
        },
    ])
    return portal.s3.presign(key, "import-source")
}

async function openVersions(page: Page, row: Locator) {
    await rowAction(row, "Versions").click()
    await expect(page.getByText("Versions for ballot box", {exact: true})).toBeVisible()
    const version = (number: string) =>
        page.getByRole("row").filter({has: page.getByRole("cell", {name: number, exact: true})})
    return {pending: version("2"), approved: version("1")}
}

test("reviews ballot box versions: shows one, approves the pending one and downloads its source", async ({
    page,
    portal,
}) => {
    const state = ballotBox(portal)
    const sourceUrl = importReference(portal)
    portal.graphql.once("FetchDocument", () => ({data: {fetchDocument: {url: sourceUrl}}}))
    portal.graphql.on("ReviewTallySheet", ({variables}) => {
        const reviewed = sheet(SHEET_V2, 2, String(variables.newStatus), {content: content(6, 3)})
        state.sheets = [reviewed, state.sheets[1]]
        return {data: {review_tally_sheet: reviewed}}
    })

    const row = await openTallySheets(page, portal)
    const versions = await openVersions(page, row)
    await expect(versions.pending).toContainText("PENDING")
    await expect(versions.pending).toContainText("Import status: APPROVED")
    await expect(versions.approved).toContainText("APPROVED")
    expect(
        portal.graphql
            .callsTo("sequent_backend_tally_sheet_import")
            .map(({variables}) => variables.where)
    ).toContainEqual({id: {_in: [IMPORT_ID]}})
    await expect(rowAction(versions.approved, "Approve")).toHaveCount(0)

    const download = page.waitForEvent("download")
    await versions.pending.locator('span[aria-label="Source file"] button').click()
    const file = await download
    expect(file.suggestedFilename()).toBe("north-paper.xml")
    expect(file.url()).toBe(sourceUrl)
    expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({
        electionEventId: EVENT_ID,
        documentId: "b5000000-0000-4000-8000-000000000001",
    })

    const laterDownloads: string[] = []
    page.on("download", (download) => laterDownloads.push(download.url()))
    portal.graphql.once("FetchDocument", () => ({errors: [{message: "source document expired"}]}))
    await versions.pending.locator('span[aria-label="Source file"] button').click()
    await expect(page.getByText("source document expired", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("FetchDocument").map(({variables}) => variables)).toEqual([
        {electionEventId: EVENT_ID, documentId: "b5000000-0000-4000-8000-000000000001"},
        {electionEventId: EVENT_ID, documentId: "b5000000-0000-4000-8000-000000000001"},
    ])
    expect(laterDownloads).toEqual([])

    await rowAction(versions.pending, "Disapprove").click()
    const dialog = page.getByRole("dialog")
    await expect(dialog).toContainText("Are you sure to disapprove this Tally Sheet?")
    await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(dialog).toHaveCount(0)
    await rowAction(versions.pending, "Approve").click()
    await expect(dialog).toContainText("Are you sure to approve this Tally Sheet?")
    expect(portal.graphql.callsTo("ReviewTallySheet")).toEqual([])
    await dialog.getByRole("button", {name: "Approve", exact: true}).click()
    await expect(page.getByText("Tally sheet reviewed", {exact: true})).toBeVisible()
    const review = portal.graphql.callsTo("ReviewTallySheet")
    expect(review.map(({variables}) => variables)).toEqual([
        {electionEventId: EVENT_ID, tallySheetId: SHEET_V2, newStatus: "APPROVED"},
    ])
    expect(review[0].headers["x-hasura-role"]).toBe("tally-sheet-review")

    await rowAction(versions.approved, "Show").click()
    await expect(page.getByText("Tally Sheet configuration.", {exact: true})).toBeVisible()
    await expect(page.getByRole("spinbutton", {name: "Census"})).toHaveValue("40")
    await expect(page.getByRole("spinbutton", {name: "Total Valid Votes"})).toHaveValue("8")
    const alice = page.getByText("Alice Example", {exact: true}).locator("..")
    await expect(alice.getByRole("spinbutton", {name: "Total Votes"})).toHaveValue("5")
    await expect(alice.getByRole("spinbutton", {name: "Total Votes"})).toBeDisabled()
    await page.getByRole("button", {name: "Back", exact: true}).click()
    await expect(row.getByRole("cell").nth(4)).toHaveText("2")
})

test("opens a version's source import on the election event's imports tab", async ({
    page,
    portal,
}) => {
    ballotBox(portal)
    importReference(portal)
    registerUsers(portal)
    table(portal, "sequent_backend_tally_sheet_import_item", () => [])
    const row = await openTallySheets(page, portal)
    const versions = await openVersions(page, row)
    await versions.pending.getByRole("button", {name: "Open import", exact: true}).click()
    await expect(page).toHaveURL(
        new RegExp(
            `/sequent_backend_election_event/${EVENT_ID}\\?.*tallySheetImportId=${IMPORT_ID}`
        ),
        {timeout: 2000}
    )
    await expect(page.getByText("Import status", {exact: false})).toHaveCount(0)
    await expect(page.getByRole("presentation").getByText(IMPORT_ID, {exact: true})).toBeVisible()
})

test("reports a failed tally sheet review", async ({page, portal}) => {
    const rejections = await recordRejections(page, "sheet already reviewed")
    ballotBox(portal)
    importReference(portal)
    portal.graphql.on("ReviewTallySheet", () => ({errors: [{message: "sheet already reviewed"}]}))
    const row = await openTallySheets(page, portal)
    const versions = await openVersions(page, row)
    await rowAction(versions.pending, "Disapprove").click()
    await page.getByRole("dialog").getByRole("button", {name: "Disapprove", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("ReviewTallySheet").length).toBe(1)
    expect(portal.graphql.callsTo("ReviewTallySheet")[0].variables).toEqual({
        electionEventId: EVENT_ID,
        tallySheetId: SHEET_V2,
        newStatus: "DISAPPROVED",
    })
    await expect(page.getByText("Error reviewing tally sheet", {exact: true})).toBeVisible({
        timeout: 3000,
    })
    expect(await rejections()).toEqual([])
})

test("requires every field before confirming a new tally sheet", async ({page, portal}) => {
    const state = ballotBox(portal)
    state.sheets = []
    await page.goto(`${portal.origin}/sequent_backend_election/${ELECTION_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally Sheets", exact: true}).click()
    await expect(page.getByText("No Tally Sheet Yet.", {exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Generate Tally Sheet"}).click()
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await page.clock.runFor(400)
    await expect(page.getByText("All fields are required", {exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Back", exact: true}).click()
    await expect(page.getByText("No Tally Sheet Yet.", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("CreateNewTallySheet")).toEqual([])
})

test.describe("with view-only tally sheet permission", () => {
    test.use({roles: ["admin-user", "election-event-read", "election-read", "tally-sheet-view"]})

    test("lists versions without add or review actions", async ({page, portal}) => {
        ballotBox(portal)
        const row = await openTallySheets(page, portal)
        await expect(rowAction(row, "Add")).toHaveCount(0)
        const versions = await openVersions(page, row)
        await expect(versions.pending).toContainText("PENDING")
        await expect(versions.pending.getByRole("cell").last()).not.toContainText("Approve")
        await expect(rowAction(versions.pending, "Approve")).toHaveCount(0)
        await expect(rowAction(versions.pending, "Show")).toHaveCount(1)
        await expect(versions.pending).not.toContainText("Import status")
    })
})
