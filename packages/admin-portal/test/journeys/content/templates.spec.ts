// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Locator, Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {BASE_ROLES, CONTENT_IDS, type Row, expectRole, notification, table} from "./data"

const readerRoles = [...BASE_ROLES, "communication-template-read", "templates-menu"]
const writerRoles = [...readerRoles, "communication-template-write", "tenant-write", "report-read"]
const RECEIPT_TEMPLATE_ID = "b0000000-0000-4000-8000-000000000002"
const EXPORT_DOCUMENT_ID = "a0000000-0000-4000-8000-000000000006"
const IMPORT_DOCUMENT_ID = "a0000000-0000-4000-8000-000000000007"
const TASK = {
    id: "d0000000-0000-4000-8000-000000000006",
    name: "Templates",
    execution_status: "IN_PROGRESS",
    created_at: FIXED_TIME,
    start_at: FIXED_TIME,
    end_at: null,
    logs: [],
    annotations: {},
    labels: {},
    executed_by_user: "synthetic-admin",
    tenant_id: TENANT_ID,
    election_event_id: "",
    type: "EXPORT_TEMPLATES",
}

function templateRow(overrides: Row = {}): Row {
    return {
        id: CONTENT_IDS.template,
        tenant_id: TENANT_ID,
        alias: "welcome",
        type: "CREDENTIALS",
        communication_method: "EMAIL",
        template: {
            alias: "welcome",
            name: "Welcome letter",
            selected_methods: {SMS: true},
            sms: {message: "Your credentials are ready"},
        },
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        created_by: "synthetic-admin",
        annotations: {},
        labels: {},
        ...overrides,
    }
}

function templates(portal: PortalServices, initial = [templateRow()]) {
    const rows = table(portal, "sequent_backend_template", initial)
    portal.graphql.on("GetUserTemplate", () => ({
        data: {
            get_user_template: {
                template_hbs: "<p>{{receipt}}</p>",
                extra_config: JSON.stringify({
                    communication_templates: {sms_config: {message: "Default receipt SMS"}},
                    pdf_options: {format: "A4"},
                    report_options: {max_items_per_report: 10},
                }),
            },
        },
    }))
    portal.graphql.on("InsertTemplate", ({variables}) => {
        const object = variables.object as Row
        const row = templateRow({...object, id: RECEIPT_TEMPLATE_ID})
        rows.push(row)
        return {data: {insert_sequent_backend_template: {returning: [row], affected_rows: 1}}}
    })
    portal.graphql.on("UpdateTemplate", ({variables}) => {
        Object.assign(rows.find((row) => row.id === variables.id) ?? {}, variables.set)
        return {data: {update_sequent_backend_template_by_pk: {id: variables.id}}}
    })
    return rows
}

async function openTemplates(page: Page, portal: PortalServices) {
    await page.goto(`${portal.origin}/sequent_backend_template?lang=en`)
    await expect(page.getByText("List of templates", {exact: true})).toBeVisible()
}

/** Row actions are unlabelled icon buttons: edit comes first, delete second. */
const rowButtons = (page: Page, name: string): Locator =>
    page.getByRole("row").filter({hasText: name}).getByRole("button")

test.describe("template administrator", () => {
    test.use({roles: writerRoles})

    test("lists the tenant's templates", async ({page, portal}) => {
        templates(portal)
        await openTemplates(page, portal)
        const row = page.getByRole("row").filter({hasText: "Welcome letter"})
        await expect(row.getByRole("cell", {name: "welcome", exact: true})).toBeVisible()
        await expect(row.getByRole("cell", {name: "CREDENTIALS", exact: true})).toBeVisible()
        expectRole(portal, "sequent_backend_template", "communication-template-read")
    })

    test("creates a template from the default of its type", async ({page, portal}) => {
        const rows = templates(portal)
        await openTemplates(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Create a Template"})
        await drawer.getByRole("textbox", {name: "Template Alias"}).fill("receipt")
        await drawer.getByRole("textbox", {name: "Template Name"}).fill("Ballot receipt")
        await drawer.getByRole("combobox", {name: "Type"}).click()
        await page.getByRole("option", {name: "Ballot Receipt", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("GetUserTemplate").length).toBe(1)
        expect(portal.graphql.callsTo("GetUserTemplate")[0].variables).toEqual({
            template_type: "ballot_receipt",
        })
        expectRole(portal, "GetUserTemplate", "report-read")
        await drawer.getByRole("switch", {name: "SMS"}).check()
        await drawer.getByRole("button", {name: "SMS Message"}).click()
        await expect(drawer.getByRole("textbox", {name: "SMS Message"})).toHaveValue(
            "Default receipt SMS"
        )
        await drawer.getByRole("textbox", {name: "SMS Message"}).fill("Your receipt is ready")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Template created")).toBeVisible()
        expect(portal.graphql.callsTo("InsertTemplate")[0].variables).toEqual({
            object: {
                alias: "receipt",
                tenant_id: TENANT_ID,
                type: "BALLOT_RECEIPT",
                communication_method: "EMAIL",
                template: {
                    alias: "receipt",
                    name: "Ballot receipt",
                    selected_methods: {DOCUMENT: false, EMAIL: false, SMS: true},
                    sms: {message: "Your receipt is ready"},
                    email: "",
                    document: "<p>{{receipt}}</p>",
                    pdf_options: {format: "A4"},
                    report_options: {max_items_per_report: 10},
                },
            },
        })
        expect(rows).toHaveLength(2)
    })

    test("renames a template and deletes another after confirmation", async ({page, portal}) => {
        const rows = templates(portal, [
            templateRow(),
            templateRow({
                id: RECEIPT_TEMPLATE_ID,
                alias: "receipt",
                type: "BALLOT_RECEIPT",
                template: {alias: "receipt", name: "Ballot receipt"},
            }),
        ])
        await openTemplates(page, portal)
        await rowButtons(page, "Welcome letter").nth(0).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Edit a Template"})
        await expect(drawer.getByRole("textbox", {name: "Template Alias"})).toHaveValue("welcome")
        await drawer.getByRole("textbox", {name: "Template Name"}).fill("Welcome letter v2")
        const refreshed = page.waitForResponse(
            (response) =>
                response.request().method() === "POST" &&
                response.url().endsWith("/v1/graphql") &&
                response.request().postDataJSON()?.operationName === "sequent_backend_template"
        )
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await refreshed
        await expect(notification(page, "Template updated")).toBeVisible()
        await expect(drawer).not.toBeVisible()
        await expect(page.getByRole("cell", {name: "Welcome letter v2", exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("UpdateTemplate")[0].variables).toEqual({
            id: CONTENT_IDS.template,
            tenantId: TENANT_ID,
            set: {
                alias: "welcome",
                annotations: {},
                communication_method: "EMAIL",
                created_at: FIXED_TIME,
                created_by: "synthetic-admin",
                labels: {},
                template: {
                    alias: "welcome",
                    name: "Welcome letter v2",
                    selected_methods: {DOCUMENT: false, EMAIL: false, SMS: true},
                    sms: {message: "Your credentials are ready"},
                    email: "",
                    pdf_options: "",
                    report_options: {},
                },
                tenant_id: TENANT_ID,
                type: "CREDENTIALS",
                updated_at: FIXED_TIME,
            },
        })
        expect(portal.graphql.callsTo("GetUserTemplate")).toHaveLength(0)

        // The refreshed list must have mounted before opening its row-local dialog.
        await rowButtons(page, "Ballot receipt").nth(1).click()
        await page
            .getByRole("dialog")
            .filter({hasText: "Warning"})
            .getByRole("button", {name: "Delete", exact: true})
            .click()
        await expect(page.getByRole("cell", {name: "Ballot receipt"})).not.toBeVisible()
        expect(portal.graphql.callsTo("delete_sequent_backend_template")[0].variables).toEqual({
            where: {id: {_eq: RECEIPT_TEMPLATE_ID}},
        })
        expectRole(portal, "delete_sequent_backend_template", "communication-template-write")
        expect(rows.map((row) => row.id)).toEqual([CONTENT_IDS.template])
    })

    test("exports the tenant's templates as a CSV download", async ({page, portal}) => {
        templates(portal)
        const key = `tenant/${TENANT_ID}/documents/${EXPORT_DOCUMENT_ID}`
        portal.s3.putBytes("private", key, Buffer.from("alias,name\n"), "text/csv")
        const url = portal.s3.presign(key, "templates-export")
        portal.graphql.on("ExportTemplate", () => ({
            data: {
                export_template: {
                    error_msg: null,
                    document_id: EXPORT_DOCUMENT_ID,
                    task_execution: TASK,
                },
            },
        }))
        portal.graphql.on("GetDocument", () => ({
            data: {sequent_backend_document: [{name: "templates.csv", annotations: {}}]},
        }))
        portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url}}}))
        portal.graphql.on("GetTaskById", () => ({
            data: {sequent_backend_tasks_execution: []},
        }))
        await openTemplates(page, portal)
        await page.getByRole("button", {name: "Export", exact: true}).click()
        const download = page.waitForEvent("download")
        await page.getByRole("dialog").getByRole("button", {name: "Export", exact: true}).click()
        expect((await download).suggestedFilename()).toBe("templates-export.csv")
        expect((await download).url()).toBe(url)
        expect(portal.graphql.callsTo("ExportTemplate")[0].variables).toEqual({
            tenantId: TENANT_ID,
        })
    })

    test("imports templates from an uploaded CSV", async ({page, portal}) => {
        templates(portal)
        const key = "documents/templates.csv"
        const url = portal.s3.presign(key, "templates-upload")
        portal.s3.override(
            (request) => request.method === "PUT" && request.key === key,
            {status: 200},
            1
        )
        portal.graphql.on("GetUploadUrl", () => ({
            data: {get_upload_url: {url, document_id: IMPORT_DOCUMENT_ID}},
        }))
        portal.graphql.on("ImportTemplates", () => ({
            data: {
                import_templates: {
                    error_msg: null,
                    document_id: IMPORT_DOCUMENT_ID,
                    task_execution: {...TASK, type: "IMPORT_TEMPLATES"},
                },
            },
        }))
        portal.graphql.on("GetTaskById", () => ({
            data: {sequent_backend_tasks_execution: []},
        }))
        await openTemplates(page, portal)
        await page.getByRole("button", {name: "Import", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Import Templates"})
        const csv = Buffer.from("alias,name\nreceipt,Ballot receipt\n")
        const uploaded = page.waitForRequest(
            (request) => request.url() === url && request.method() === "PUT"
        )
        await drawer.locator('input[type="file"]').setInputFiles({
            name: "templates.csv",
            mimeType: "text/csv",
            buffer: csv,
        })
        await expect(
            notification(page, "File uploaded to server - but not imported yet")
        ).toBeVisible()
        const request = await uploaded
        expect(request.postDataBuffer()).toEqual(csv)
        expect(request.headers()["content-type"]).toBe("text/csv")
        expect(portal.graphql.callsTo("GetUploadUrl").map(({variables}) => variables)).toEqual([
            {name: "templates.csv", media_type: "text/csv", size: csv.length, is_public: false},
        ])
        await drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}).fill("cd".repeat(32))
        await drawer.getByRole("button", {name: "Import", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("ImportTemplates").length).toBe(1)
        expect(portal.graphql.callsTo("ImportTemplates")[0].variables).toEqual({
            tenantId: TENANT_ID,
            documentId: IMPORT_DOCUMENT_ID,
            sha256: "cd".repeat(32),
        })
        await expect(drawer).not.toBeVisible()
    })

    test("opens a single create drawer", async ({page, portal}) => {
        templates(portal)
        await openTemplates(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        await expect(page.getByRole("dialog").filter({hasText: "Create a Template"})).toBeVisible()
        test.fail(
            true,
            "ListActions and TemplateList each render a create drawer bound to openDrawer (Template/TemplateList.tsx:255,276)"
        )
        await expect(page.getByRole("dialog", {includeHidden: true})).toHaveCount(1, {
            timeout: 1000,
        })
    })
})

test.describe("template permissions", () => {
    test.use({roles: [...BASE_ROLES, "communication-template-read"]})

    test("explains that templates need the templates menu permission", async ({page, portal}) => {
        templates(portal)
        await page.goto(`${portal.origin}/sequent_backend_template?lang=en`)
        await expect(page.getByText("You don't have permission to access templates.")).toBeVisible()
        expect(portal.graphql.callsTo("sequent_backend_template")).toHaveLength(0)
    })
})

test.describe("template reader", () => {
    test.use({roles: readerRoles})

    test("sees the tenant's templates without tenant write access", async ({page, portal}) => {
        templates(portal)
        await page.goto(`${portal.origin}/sequent_backend_template?lang=en`)
        test.fail(
            true,
            "TemplateList shows only the empty state unless the user can write the tenant (Template/TemplateList.tsx:233)"
        )
        await expect(page.getByRole("cell", {name: "Welcome letter"})).toBeVisible({
            timeout: 3000,
        })
    })
})
