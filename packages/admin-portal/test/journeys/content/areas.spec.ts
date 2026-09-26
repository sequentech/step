// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {dirname, resolve} from "node:path"
import {fileURLToPath} from "node:url"
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {loadClientSchema} from "@sequentech/ui-test-kit/mocks/graphql"
import {parse, validate} from "graphql"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    answerInvalid,
    areaRow,
    catchRejections,
    contestRow,
    dropUnusedVariables,
    eventPage,
    eventRow,
    expectRole,
    iconButton,
    names,
    notification,
    openEventTab,
    table,
} from "./data"

const readerRoles = [...BASE_ROLES, "election-event-areas-tab", "area-read", "contest-read"]
const writerRoles = [
    ...readerRoles,
    "area-create",
    "area-write",
    "area-delete",
    "area-import",
    "area-upsert",
    "election-event-areas-columns",
    "election-event-areas-filters",
]
const NEW_AREA_ID = "40000000-0000-4000-8000-000000000009"
const DOCUMENT_ID = CONTENT_IDS.document

const SCHEMA = resolve(dirname(fileURLToPath(import.meta.url)), "../../../graphql.schema.json")
const MISSING_AREA_ID = 'expecting a value for non-nullable variable: "areaId"'

const DEFAULT_AREAS = [
    areaRow(),
    areaRow({id: CONTENT_IDS.secondArea, name: "South", description: "Southern district"}),
]

function areas(portal: PortalServices, initial = DEFAULT_AREAS, event = eventRow()) {
    eventPage(portal, event)
    // Pinned below: UPSERT_AREA declares $presentation without using it.
    dropUnusedVariables(portal, "UpsertArea", ["presentation"])
    const rows = table(portal, "sequent_backend_area", initial)
    const contests = [
        contestRow({presentation: names("Mayor")}),
        contestRow({id: CONTENT_IDS.secondContest, presentation: names("Council seats")}),
    ]
    table(portal, "sequent_backend_contest", contests)
    // The north area already runs the mayor contest.
    const areaContests = (areaId: unknown) =>
        areaId === IDS.area ? [{id: "ac-1", contest: contests[0]}] : []
    portal.graphql.on("get_area_with_area_contests", ({variables}) => ({
        data: {sequent_backend_area_contest: areaContests(variables.areaId)},
    }))
    portal.graphql.on("sequent_backend_area_extended", ({variables}) => ({
        data: {sequent_backend_area_contest: areaContests(variables.areaId)},
    }))
    portal.graphql.on("UpsertArea", ({variables}) => {
        const id = (variables.id as string | undefined) ?? NEW_AREA_ID
        const existing = rows.find((row) => row.id === id)
        const values = {
            name: variables.name,
            description: variables.description ?? null,
            parent_id: variables.parentId ?? null,
        }
        if (existing) Object.assign(existing, values)
        else rows.push(areaRow({id, ...values}))
        return {data: {upsert_area: {id}}}
    })
    return rows
}

async function openAreas(page: Page, portal: PortalServices) {
    await openEventTab(page, portal, "Areas")
    await expect(page.getByRole("cell", {name: "North", exact: true})).toBeVisible()
}

test.describe("area administrator", () => {
    test.use({roles: writerRoles})

    test("lists the event's areas with their contests", async ({page, portal}) => {
        areas(portal)
        await openAreas(page, portal)
        const north = page.getByRole("row").filter({hasText: "North"})
        await expect(north.getByText("Northern district", {exact: true})).toBeVisible()
        await expect(north.getByText("Mayor", {exact: true})).toBeVisible()
        await expect(page.getByRole("cell", {name: "South", exact: true})).toBeVisible()
        const list = portal.graphql.callsTo("sequent_backend_area")[0]
        expect(list.variables.where).toEqual({
            _and: [{tenant_id: {_eq: TENANT_ID}}, {election_event_id: {_eq: IDS.event}}],
        })
        expectRole(portal, "sequent_backend_area", "area-read")
        expect(
            portal.graphql
                .callsTo("get_area_with_area_contests")
                .map((call) => call.variables.areaId)
                .sort()
        ).toEqual([IDS.area, CONTENT_IDS.secondArea])
    })

    test("creates an area with contests and a parent through upsert_area", async ({
        page,
        portal,
    }) => {
        const rows = areas(portal)
        // Pinned below: the create drawer queries area contests without an area id.
        answerInvalid(
            portal,
            "sequent_backend_area_extended",
            (variables) => variables.areaId === undefined,
            MISSING_AREA_ID
        )
        await openAreas(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog")
        await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("East")
        await drawer.getByRole("textbox", {name: "Description", exact: true}).fill("Eastern ward")
        await drawer.getByRole("combobox", {name: "Area contest"}).fill("Coun")
        // Both autocompletes debounce the search by 100 ms.
        await page.clock.runFor(500)
        await page.getByRole("option", {name: "Council seats", exact: true}).click()
        await drawer.getByRole("combobox", {name: "Parent"}).fill("Nor")
        await page.clock.runFor(500)
        await page.getByRole("option", {name: "North", exact: true}).click()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Area created")).toBeVisible()
        await expect(page.getByRole("cell", {name: "East", exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("UpsertArea")[0].variables).toEqual({
            name: "East",
            description: "Eastern ward",
            tenantId: TENANT_ID,
            electionEventId: IDS.event,
            parentId: IDS.area,
            areaContestsIds: [CONTENT_IDS.secondContest],
            allow_early_voting: "no_early_voting",
        })
        expect(rows.map((row) => row.name)).toEqual(["North", "South", "East"])
        const parentSearch = portal.graphql
            .callsTo("sequent_backend_area")
            .find((call) => JSON.stringify(call.variables.where).includes("Nor"))
        expect(parentSearch?.variables.where).toEqual({
            _and: [
                {name: {_ilike: "%Nor%"}},
                {tenant_id: {_eq: TENANT_ID}},
                {election_event_id: {_eq: IDS.event}},
            ],
        })
    })

    test("edits an area keeping its contests, then deletes another after confirmation", async ({
        page,
        portal,
    }) => {
        const rows = areas(portal)
        await openAreas(page, portal)
        const north = page.getByRole("row").filter({hasText: "North"})
        await iconButton(north, "edit-area-icon").click()
        const drawer = page.getByRole("dialog")
        const description = drawer.getByRole("textbox", {name: "Description", exact: true})
        await expect(description).toHaveValue("Northern district")
        await description.fill("Northern district and islands")
        const refreshed = page.waitForResponse(
            (response) =>
                response.request().method() === "POST" &&
                response.url().endsWith("/v1/graphql") &&
                response.request().postDataJSON()?.operationName === "sequent_backend_area"
        )
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await refreshed
        await expect(notification(page, "Area updated")).toBeVisible()
        expect(portal.graphql.callsTo("UpsertArea")[0].variables).toMatchObject({
            id: IDS.area,
            name: "North",
            description: "Northern district and islands",
            tenantId: TENANT_ID,
            electionEventId: IDS.event,
            areaContestsIds: [IDS.contest],
            allow_early_voting: "no_early_voting",
        })
        await expect(
            page.getByRole("cell", {name: "Northern district and islands", exact: true})
        ).toBeVisible()
        await expect(drawer).not.toBeVisible()

        const south = page.getByRole("row").filter({hasText: "South"})
        await iconButton(south, "delete-area-icon").click()
        const confirm = page.getByRole("dialog").filter({hasText: "Warning"})
        await confirm.getByRole("button", {name: "Cancel", exact: true}).click()
        await expect(confirm).not.toBeVisible()
        expect(portal.graphql.callsTo("delete_sequent_backend_area")).toHaveLength(0)
        await iconButton(south, "delete-area-icon").click()
        await confirm.getByRole("button", {name: "Delete", exact: true}).click()
        await expect(page.getByRole("cell", {name: "South", exact: true})).not.toBeVisible()
        expect(portal.graphql.callsTo("delete_sequent_backend_area")[0].variables).toEqual({
            where: {id: {_eq: CONTENT_IDS.secondArea}},
        })
        expectRole(portal, "delete_sequent_backend_area", "area-write")
        expect(rows.map((row) => row.name)).toEqual(["North"])
    })

    for (const mode of ["import", "upsert"] as const)
        test(`${mode === "import" ? "imports" : "upserts"} areas from an uploaded CSV`, async ({
            page,
            portal,
        }) => {
            areas(portal)
            const key = "documents/areas.csv"
            const url = portal.s3.presign(key, "areas-upload")
            portal.s3.override(
                (request) => request.method === "PUT" && request.key === key,
                {status: 200},
                1
            )
            portal.graphql.on("GetUploadUrl", () => ({
                data: {get_upload_url: {url, document_id: DOCUMENT_ID}},
            }))
            const operation = mode === "import" ? "ImportAreas" : "UpsertAreas"
            portal.graphql.on(operation, () => ({
                data: {[mode === "import" ? "import_areas" : "upsert_areas"]: {id: "task"}},
            }))
            await openAreas(page, portal)
            await page
                .getByRole("button", {name: mode === "import" ? "Import" : "Upsert Areas"})
                .click()
            const drawer = page.getByRole("dialog").filter({hasText: "Import Areas"})
            const csv = Buffer.from("name,description\nEast,Eastern ward\n")
            const uploaded = page.waitForRequest(
                (request) => request.url() === url && request.method() === "PUT"
            )
            await drawer.locator('input[type="file"]').setInputFiles({
                name: "areas.csv",
                mimeType: "text/csv",
                buffer: csv,
            })
            await expect(
                notification(page, "File uploaded to server - but not imported yet")
            ).toBeVisible()
            expect(portal.graphql.callsTo("GetUploadUrl")[0].variables).toEqual({
                name: "areas.csv",
                media_type: "text/csv",
                size: csv.length,
                is_public: false,
            })
            const request = await uploaded
            expect(request.postDataBuffer()).toEqual(csv)
            expect(request.headers()["content-type"]).toBe("text/csv")
            expect(portal.s3.requestsFor(key).map((request) => request.method)).toEqual(["PUT"])
            await drawer
                .getByRole("textbox", {name: "Integrity Check (SHA-256)"})
                .fill("ab".repeat(32))
            await drawer.getByRole("button", {name: "Import", exact: true}).click()
            await expect(notification(page, "Areas Imported Successfully")).toBeVisible()
            expect(portal.graphql.callsTo(operation)[0].variables).toEqual(
                mode === "import"
                    ? {documentId: DOCUMENT_ID, electionEventId: IDS.event, sha256: "ab".repeat(32)}
                    : {documentId: DOCUMENT_ID, electionEventId: IDS.event}
            )
            expectRole(portal, operation, mode === "import" ? "area-import" : "area-upsert")
        })

    test("edits the weight and early voting of an area in a weighted, early-voting event", async ({
        page,
        portal,
    }) => {
        const event = eventRow({
            presentation: {
                ...(eventRow().presentation as Record<string, unknown>),
                weighted_voting_policy: "areas-weighted-voting",
            },
            voting_channels: {online: true, early_voting: true},
        })
        areas(
            portal,
            [
                areaRow({
                    annotations: {weight: 2},
                    presentation: {allow_early_voting: "no_early_voting"},
                }),
            ],
            event
        )
        await openAreas(page, portal)
        await expect(page.getByRole("columnheader", {name: "Weight"})).toBeVisible()
        const north = page.getByRole("row").filter({hasText: "North"})
        await expect(north.getByRole("cell", {name: "2", exact: true})).toBeVisible()
        await iconButton(north, "edit-area-icon").click()
        const drawer = page.getByRole("dialog")
        const weight = drawer.getByRole("spinbutton", {name: "Weight"})
        await expect(weight).toHaveValue("2")
        await weight.fill("3")
        const earlyVoting = drawer.getByRole("switch", {name: "Allow Early Voting"})
        await expect(earlyVoting).toBeEnabled()
        await expect(earlyVoting).not.toBeChecked()
        await earlyVoting.check()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Area updated")).toBeVisible()
        expect(portal.graphql.callsTo("UpsertArea")[0].variables).toMatchObject({
            id: IDS.area,
            annotations: {weight: 3},
            allow_early_voting: "allow_early_voting",
        })
    })

    test("notifies a failed area import", async ({page, portal}) => {
        areas(portal)
        const rejections = await catchRejections(page, "Synthetic invalid CSV")
        const url = portal.s3.presign("documents/areas.csv", "areas-upload")
        portal.s3.override((request) => request.method === "PUT", {status: 200}, 1)
        portal.graphql.on("GetUploadUrl", () => ({
            data: {get_upload_url: {url, document_id: DOCUMENT_ID}},
        }))
        portal.graphql.on("ImportAreas", () => ({
            errors: [{message: "Synthetic invalid CSV", extensions: {code: "unexpected"}}],
        }))
        await openAreas(page, portal)
        await page.getByRole("button", {name: "Import", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Import Areas"})
        await drawer.locator('input[type="file"]').setInputFiles({
            name: "areas.csv",
            mimeType: "text/csv",
            buffer: Buffer.from("name\n"),
        })
        await expect(
            notification(page, "File uploaded to server - but not imported yet")
        ).toBeVisible()
        await drawer.getByRole("button", {name: "Import", exact: true}).click()
        await page
            .getByRole("dialog")
            .filter({hasText: "Import Without Integrity Check?"})
            .getByRole("button", {name: "Yes, Import without Integrity Check"})
            .click()
        await expect.poll(() => portal.graphql.callsTo("ImportAreas").length).toBe(1)
        expect(portal.graphql.callsTo("ImportAreas")[0].variables).toEqual({
            documentId: DOCUMENT_ID,
            electionEventId: IDS.event,
            sha256: "",
        })
        test.fail(true, "handleImportAreas lets Apollo's rejection escape (Area/ListArea.tsx:192)")
        await expect(notification(page, "Error importing Areas")).toBeVisible({timeout: 3000})
        expect(await rejections()).toEqual([])
    })

    test("sends an UpsertArea document that passes GraphQL validation", async ({page, portal}) => {
        areas(portal)
        const documents = dropUnusedVariables(portal, "UpsertArea", ["presentation"])
        await openAreas(page, portal)
        await iconButton(page.getByRole("row").filter({hasText: "North"}), "edit-area-icon").click()
        const drawer = page.getByRole("dialog")
        await drawer.getByRole("textbox", {name: "Description", exact: true}).fill("Renamed")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Area updated")).toBeVisible()
        expect(documents).toHaveLength(1)
        test.fail(
            true,
            "UPSERT_AREA declares $presentation but never uses it (queries/UpsertArea.ts:11)"
        )
        expect(
            validate(loadClientSchema(SCHEMA), parse(documents[0])).map((error) => error.message)
        ).toEqual([])
    })

    test("opens the create drawer without querying contests of a missing area", async ({
        page,
        portal,
    }) => {
        areas(portal)
        const invalid = answerInvalid(
            portal,
            "sequent_backend_area_extended",
            (variables) => variables.areaId === undefined,
            MISSING_AREA_ID
        )
        await openAreas(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        await expect(
            page.getByRole("dialog").getByRole("textbox", {name: "Name", exact: true})
        ).toBeVisible()
        test.fail(
            true,
            "UpsertArea runs GET_AREAS_EXTENDED with an undefined areaId (Area/UpsertArea.tsx:51)"
        )
        expect(invalid).toEqual([])
    })
})

test.describe("area contest search", () => {
    test.use({roles: writerRoles})

    test("filters the contest choices by the typed text", async ({page, portal}) => {
        areas(portal)
        answerInvalid(
            portal,
            "sequent_backend_area_extended",
            (variables) => variables.areaId === undefined,
            MISSING_AREA_ID
        )
        await openAreas(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        await page.getByRole("dialog").getByRole("combobox", {name: "Area contest"}).fill("Coun")
        await page.clock.runFor(500)
        test.fail(
            true,
            "customBuildQuery drops the name@_ilike,alias@_ilike filter (queries/customBuildQuery.ts:65)"
        )
        expect(
            portal.graphql
                .callsTo("sequent_backend_contest")
                .some((call) => JSON.stringify(call.variables).includes("Coun"))
        ).toBe(true)
        await expect(page.getByRole("option", {name: "Council seats", exact: true})).toBeVisible()
        await expect(page.getByRole("option", {name: "Mayor", exact: true})).toHaveCount(0)
    })
})

test.describe("area reader", () => {
    test.use({roles: readerRoles})

    test("sees areas without create, import, edit or delete actions", async ({page, portal}) => {
        areas(portal)
        await openAreas(page, portal)
        for (const name of ["Add", "Import", "Upsert Areas"])
            await expect(page.getByRole("button", {name, exact: true})).not.toBeVisible()
        const north = page.getByRole("row").filter({hasText: "North"})
        await expect(iconButton(north, "edit-area-icon")).toHaveCount(0)
        await expect(iconButton(north, "delete-area-icon")).toHaveCount(0)
    })
})

test.describe("event without areas", () => {
    test.use({roles: writerRoles})

    test("offers to create the first area from the empty state", async ({page, portal}) => {
        areas(portal, [])
        answerInvalid(
            portal,
            "sequent_backend_area_extended",
            (variables) => variables.areaId === undefined,
            MISSING_AREA_ID
        )
        await openEventTab(page, portal, "Areas")
        await expect(page.getByText("No Areas yet.", {exact: true})).toBeVisible()
        await page.getByRole("button", {name: "Create Area"}).click()
        await expect(
            page.getByRole("dialog").getByRole("textbox", {name: "Name", exact: true})
        ).toBeVisible()
    })
})
