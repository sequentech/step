// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Locator, Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    EVENT_SCOPE,
    type Row,
    eventPage,
    expectRole,
    notification,
    openEventTab,
    table,
} from "./data"

const readerRoles = [...BASE_ROLES, "election-event-data-tab", "support-material-read"]
const writerRoles = [...readerRoles, "support-material-write"]
const NEW_MATERIAL_ID = "f0000000-0000-4000-8000-000000000009"
const UPLOADED_DOCUMENT_ID = "a0000000-0000-4000-8000-000000000005"

function materialRow(overrides: Row = {}): Row {
    return {
        id: CONTENT_IDS.supportMaterial,
        ...EVENT_SCOPE,
        kind: "application/pdf",
        data: {title_i18n: {en: "Voting guide"}, subtitle_i18n: {en: "How to vote"}},
        document_id: CONTENT_IDS.document,
        is_hidden: false,
        annotations: {},
        labels: {},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        ...overrides,
    }
}

function materials(portal: PortalServices, initial = [materialRow()]) {
    eventPage(portal)
    table(portal, "sequent_backend_election", [])
    // The event data tab also shows the voter password policy.
    portal.graphql.on("GetRealmPasswordPolicy", () => ({
        data: {
            get_realm_password_policy: {
                configured: false,
                minimum_length: 8,
                maximum_length: 64,
                include_uppercase: false,
                include_lowercase: false,
                include_digits: false,
                include_special_characters: false,
            },
        },
    }))
    table(portal, "sequent_backend_document", [
        {
            id: CONTENT_IDS.document,
            ...EVENT_SCOPE,
            name: "guide.pdf",
            media_type: "application/pdf",
            size: 1024,
            is_public: true,
            annotations: {},
            labels: {},
            created_at: FIXED_TIME,
            last_updated_at: FIXED_TIME,
        },
    ])
    return table(portal, "sequent_backend_support_material", initial, () => NEW_MATERIAL_ID)
}

function upload(portal: PortalServices) {
    const key = "documents/booklet.pdf"
    const url = portal.s3.presign(key, "material-upload")
    portal.s3.override(
        (request) => request.method === "PUT" && request.key === key,
        {status: 200},
        1
    )
    portal.graphql.on("GetUploadUrl", () => ({
        data: {get_upload_url: {url, document_id: UPLOADED_DOCUMENT_ID}},
    }))
    return key
}

async function openMaterials(page: Page, portal: PortalServices) {
    await openEventTab(page, portal, "Data")
    await page.getByRole("button", {name: "Support Materials"}).click()
}

/** Row actions are unlabelled icon buttons: edit comes first, delete second. */
const rowButtons = (page: Page, title: string): Locator =>
    page.getByRole("row").filter({hasText: title}).getByRole("button")

test.describe("support material editor", () => {
    test.use({roles: writerRoles})

    test("lists the event's support materials", async ({page, portal}) => {
        materials(portal)
        await openMaterials(page, portal)
        const row = page.getByRole("row").filter({hasText: "Voting guide"})
        await expect(row.getByRole("cell", {name: "How to vote", exact: true})).toBeVisible()
        expect(
            portal.graphql
                .callsTo("sequent_backend_support_material")
                .filter((call) => call.variables.limit)
                .at(-1)?.variables.where
        ).toEqual({
            _and: [{tenant_id: {_eq: TENANT_ID}}, {election_event_id: {_eq: IDS.event}}],
        })
        expectRole(portal, "sequent_backend_support_material", "support-material-read")
    })

    test("uploads a new support material with its titles", async ({page, portal}) => {
        const rows = materials(portal)
        const key = upload(portal)
        await openMaterials(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Enter support material data."})
        await drawer.getByRole("textbox", {name: "Title", exact: true}).fill("Candidates booklet")
        await drawer.getByRole("textbox", {name: "Subtitle", exact: true}).fill("All lists")
        await drawer.getByRole("switch", {name: "Is Hidden"}).check()
        const pdf = Buffer.from("%PDF-1.7 booklet")
        const uploaded = page.waitForRequest(
            (request) =>
                request.url() === portal.s3.presign(key, "material-upload") &&
                request.method() === "PUT"
        )
        await drawer.locator('input[type="file"]').setInputFiles({
            name: "booklet.pdf",
            mimeType: "application/pdf",
            buffer: pdf,
        })
        await expect(notification(page, "File loaded")).toBeVisible()
        expect(portal.graphql.callsTo("GetUploadUrl")[0].variables).toEqual({
            name: "booklet.pdf",
            media_type: "application/pdf",
            size: pdf.length,
            is_public: true,
            election_event_id: IDS.event,
        })
        const request = await uploaded
        expect(request.postDataBuffer()).toEqual(pdf)
        expect(request.headers()["content-type"]).toBe("application/pdf")
        expect(portal.s3.requestsFor(key)).toMatchObject([{method: "PUT", status: 200}])
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Support material created")).toBeVisible()
        expect(
            portal.graphql.callsTo("insert_sequent_backend_support_material")[0].variables
        ).toEqual({
            objects: {
                ...EVENT_SCOPE,
                is_hidden: true,
                labels: {},
                annotations: {},
                data: {title_i18n: {en: "Candidates booklet"}, subtitle_i18n: {en: "All lists"}},
                kind: "application/pdf",
                document_id: UPLOADED_DOCUMENT_ID,
            },
        })
        expectRole(portal, "insert_sequent_backend_support_material", "support-material-write")
        expect(rows.map((row) => row.id)).toEqual([CONTENT_IDS.supportMaterial, NEW_MATERIAL_ID])
    })

    test("replaces an existing support document before saving its new title", async ({
        page,
        portal,
    }) => {
        materials(portal)
        const key = upload(portal)
        await openMaterials(page, portal)
        await rowButtons(page, "Voting guide").nth(0).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Enter support material data."})
        await drawer.getByRole("textbox", {name: "Title", exact: true}).fill("Replacement guide")
        const pdf = Buffer.from("%PDF-1.7 replacement")
        const uploaded = page.waitForRequest(
            (request) =>
                request.url() === portal.s3.presign(key, "material-upload") &&
                request.method() === "PUT"
        )
        await drawer
            .locator('input[type="file"]')
            .setInputFiles({name: "booklet.pdf", mimeType: "application/pdf", buffer: pdf})
        const request = await uploaded
        expect(request.postDataBuffer()).toEqual(pdf)
        expect(request.headers()["content-type"]).toBe("application/pdf")
        await expect(notification(page, "File loaded")).toBeVisible()
        expect(portal.graphql.callsTo("GetUploadUrl").map(({variables}) => variables)).toEqual([
            {
                name: "booklet.pdf",
                media_type: "application/pdf",
                size: pdf.length,
                is_public: true,
                election_event_id: IDS.event,
            },
        ])
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Support material updated")).toBeVisible()
        expect(
            portal.graphql
                .callsTo("update_sequent_backend_support_material")
                .map(({variables}) => variables)
        ).toEqual([
            {
                where: {id: {_eq: CONTENT_IDS.supportMaterial}},
                _set: {
                    data: {
                        title_i18n: {en: "Replacement guide"},
                        subtitle_i18n: {en: "How to vote"},
                    },
                    document_id: UPLOADED_DOCUMENT_ID,
                },
            },
        ])
        await expect(page.getByRole("cell", {name: "Replacement guide", exact: true})).toBeVisible()
    })

    test("reports a rejected support material update without changing the saved title", async ({
        page,
        portal,
    }) => {
        materials(portal)
        portal.graphql.on("update_sequent_backend_support_material", () => ({
            errors: [{message: "support update rejected"}],
        }))
        await openMaterials(page, portal)
        await rowButtons(page, "Voting guide").nth(0).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Enter support material data."})
        await drawer.getByRole("textbox", {name: "Title", exact: true}).fill("Rejected title")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect
            .poll(() => portal.graphql.callsTo("update_sequent_backend_support_material").length)
            .toBe(1)
        expect(
            portal.graphql
                .callsTo("update_sequent_backend_support_material")
                .map(({variables}) => variables)
        ).toEqual([
            {
                where: {id: {_eq: CONTENT_IDS.supportMaterial}},
                _set: {
                    data: {title_i18n: {en: "Rejected title"}, subtitle_i18n: {en: "How to vote"}},
                },
            },
        ])
        await expect(page.getByRole("cell", {name: "Voting guide", exact: true})).toBeVisible()
        test.fail(
            true,
            "EditSupportMaterial passes an untranslated error key to react-admin notify"
        )
        await expect(notification(page, "Error updating support material")).toBeVisible({
            timeout: 2000,
        })
    })

    test("refuses to create a material without a title and a document", async ({page, portal}) => {
        materials(portal)
        await openMaterials(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Enter support material data."})
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "The form is not valid.")).toBeVisible()
        expect(portal.graphql.callsTo("insert_sequent_backend_support_material")).toHaveLength(0)
    })

    test("explains which support material fields are missing", async ({page, portal}) => {
        materials(portal)
        await openMaterials(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Enter support material data."})
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        test.fail(
            true,
            "formValidator keys its errors to data and document_id, which no input renders (SupportMaterials/CreateSupportMaterial.tsx:164)"
        )
        await expect(drawer.getByText("Title is required")).toBeVisible({timeout: 3000})
    })

    test("edits a material's title and shows its public URL, then deletes it", async ({
        page,
        portal,
    }) => {
        const rows = materials(portal)
        await openMaterials(page, portal)
        await rowButtons(page, "Voting guide").nth(0).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Enter support material data."})
        const title = drawer.getByRole("textbox", {name: "Title", exact: true})
        await expect(title).toHaveValue("Voting guide")
        await expect(drawer.getByRole("textbox", {name: "Public URL"})).toHaveValue(
            `${portal.s3.publicBucketUrl()}tenant-${TENANT_ID}/document-${CONTENT_IDS.document}/guide.pdf`
        )
        await title.fill("Voting guide 2026")
        const refreshed = page.waitForResponse(
            (response) =>
                response.request().method() === "POST" &&
                response.url().endsWith("/v1/graphql") &&
                response.request().postDataJSON()?.operationName ===
                    "sequent_backend_support_material"
        )
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await refreshed
        await expect(notification(page, "Support material updated")).toBeVisible()
        await expect(drawer).not.toBeVisible()
        const update = portal.graphql.callsTo("update_sequent_backend_support_material")[0]
        expect(update.variables.where).toEqual({id: {_eq: CONTENT_IDS.supportMaterial}})
        // Only the changed column is sent.
        expect(update.variables._set).toEqual({
            data: {title_i18n: {en: "Voting guide 2026"}, subtitle_i18n: {en: "How to vote"}},
        })
        await expect(page.getByRole("cell", {name: "Voting guide 2026"})).toBeVisible()

        await rowButtons(page, "Voting guide 2026").nth(1).click()
        await page
            .getByRole("dialog")
            .filter({hasText: "Warning"})
            .getByRole("button", {name: "Delete", exact: true})
            .click()
        await expect(page.getByText("No support material yet")).toBeVisible()
        expect(
            portal.graphql.callsTo("delete_sequent_backend_support_material")[0].variables
        ).toEqual({
            where: {id: {_eq: CONTENT_IDS.supportMaterial}},
        })
        expect(rows).toHaveLength(0)
    })
})

test.describe("support material create drawer", () => {
    test.use({roles: writerRoles})

    test("opens a single create drawer", async ({page, portal}) => {
        materials(portal)
        await openMaterials(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        await expect(
            page.getByRole("dialog").filter({hasText: "Enter support material data."})
        ).toBeVisible()
        test.fail(
            true,
            "ListActions and ListSupportMaterials each render a create drawer bound to openCreate (SupportMaterials/ListSuportMaterial.tsx:163,226)"
        )
        await expect(page.getByRole("dialog", {includeHidden: true})).toHaveCount(1, {
            timeout: 1000,
        })
    })
})

test.describe("support material reader", () => {
    test.use({roles: readerRoles})

    test("sees materials without add, edit or delete actions", async ({page, portal}) => {
        materials(portal)
        await openMaterials(page, portal)
        await expect(page.getByRole("cell", {name: "Voting guide"})).toBeVisible()
        await expect(page.getByRole("button", {name: "Add", exact: true})).not.toBeVisible()
        await expect(rowButtons(page, "Voting guide")).toHaveCount(0)
    })
})
