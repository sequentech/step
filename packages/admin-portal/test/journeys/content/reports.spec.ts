// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    type Row,
    allowUnlistedInputFields,
    electionRow,
    eventPage,
    expectRole,
    expireNotification,
    names,
    notification,
    openEventTab,
    table,
} from "./data"

const readerRoles = [...BASE_ROLES, "election-event-reports-tab", "report-read"]
const writerRoles = [
    ...readerRoles,
    "report-write",
    "report-create",
    "report-delete",
    "report-generate",
    "report-preview",
    "election-event-reports-columns",
    "communication-template-read",
    "document-password-read",
    "document-download",
]
const LOGS_REPORT_ID = "c0000000-0000-4000-8000-000000000002"
const NEW_REPORT_ID = "c0000000-0000-4000-8000-000000000009"
const REPORT_DOCUMENT_ID = "a0000000-0000-4000-8000-000000000004"
const TASK_ID = "d0000000-0000-4000-8000-000000000004"

function templateRow(alias: string, type: string, name: string): Row {
    return {
        id: `b0000000-0000-4000-8000-00000000000${type === "INITIALIZATION_REPORT" ? 1 : 2}`,
        tenant_id: TENANT_ID,
        alias,
        type,
        communication_method: "DOCUMENT",
        template: {name, alias},
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        created_by: "synthetic-admin",
        annotations: {},
        labels: {},
    }
}

function reportRow(overrides: Row = {}): Row {
    return {
        id: CONTENT_IDS.report,
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        election_id: IDS.election,
        report_type: "INITIALIZATION_REPORT",
        template_alias: "init-report",
        encryption_policy: "unencrypted",
        cron_config: null,
        created_at: FIXED_TIME,
        ...overrides,
    }
}

function reports(portal: PortalServices, initial?: Row[]) {
    eventPage(portal)
    table(portal, "sequent_backend_election", [
        electionRow({presentation: names("Mayor election")}),
        electionRow({id: CONTENT_IDS.secondElection, presentation: names("Council election")}),
    ])
    table(portal, "sequent_backend_template", [
        templateRow("init-report", "INITIALIZATION_REPORT", "Initialization template"),
        templateRow("logs-report", "ACTIVITY_LOGS", "Activity template"),
    ])
    const rows = table(
        portal,
        "sequent_backend_report",
        initial ?? [
            reportRow(),
            reportRow({
                id: LOGS_REPORT_ID,
                election_id: null,
                report_type: "ACTIVITY_LOGS",
                template_alias: "logs-report",
                encryption_policy: "configured_password",
            }),
        ]
    )
    const inserts = allowUnlistedInputFields(portal, "InsertReport", "object", ["permission_label"])
    const updates = allowUnlistedInputFields(portal, "UpdateReport", "set", ["permission_label"])
    portal.graphql.on("InsertReport", ({variables}) => {
        const row = reportRow({...(variables.object as Row), id: NEW_REPORT_ID})
        rows.push(row)
        return {data: {insert_sequent_backend_report: {returning: [row], affected_rows: 1}}}
    })
    portal.graphql.on("UpdateReport", ({variables}) => {
        const row = rows.find((candidate) => candidate.id === variables.id)
        Object.assign(row ?? {}, variables.set)
        return {data: {update_sequent_backend_report_by_pk: {id: variables.id}}}
    })
    return {rows, inserts, updates}
}

async function openReports(page: Page, portal: PortalServices) {
    await openEventTab(page, portal, "Reports")
    await expect(page.getByRole("cell", {name: "Initialization Report", exact: true})).toBeVisible()
}

async function rowAction(page: Page, reportType: string, action: string) {
    await page
        .getByRole("row")
        .filter({hasText: reportType})
        .getByRole("button", {name: "Actions", exact: true})
        .click()
    await page.getByRole("menuitem", {name: action, exact: true}).click()
}

function generatedDocument(portal: PortalServices, name: string, access?: Row) {
    const key = `tenant/${TENANT_ID}/documents/${REPORT_DOCUMENT_ID}`
    portal.s3.putBytes("private", key, Buffer.from("%PDF-1.7"), "application/pdf")
    const url = portal.s3.presign(key, "report-download")
    portal.graphql.on("GenerateReport", () => ({
        data: {
            generate_report: {
                document_id: REPORT_DOCUMENT_ID,
                encryption_policy: access ? "configured_password" : "unencrypted",
                task_execution: {
                    id: TASK_ID,
                    name: "Generate report",
                    execution_status: "IN_PROGRESS",
                    created_at: FIXED_TIME,
                    start_at: FIXED_TIME,
                    end_at: null,
                    logs: [],
                    annotations: {},
                    labels: {},
                    executed_by_user: "synthetic-admin",
                    tenant_id: TENANT_ID,
                    election_event_id: IDS.event,
                    type: "GENERATE_REPORT",
                },
            },
        },
    }))
    portal.graphql.on("GetTaskById", () => ({
        data: {
            sequent_backend_tasks_execution: [
                {
                    id: TASK_ID,
                    tenant_id: TENANT_ID,
                    election_event_id: IDS.event,
                    execution_status: "IN_PROGRESS",
                    type: "GENERATE_REPORT",
                    start_at: FIXED_TIME,
                    end_at: null,
                    logs: [],
                    annotations: {},
                    executed_by_user: "synthetic-admin",
                },
            ],
        },
    }))
    portal.graphql.on("GetDocument", () => ({
        data: {sequent_backend_document: [{name, annotations: access ? {access} : {}}]},
    }))
    portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url}}}))
    return url
}

test.describe("report administrator", () => {
    test.use({roles: writerRoles})

    test("lists reports with their type, template, election and encryption", async ({
        page,
        portal,
    }) => {
        reports(portal)
        await openReports(page, portal)
        const init = page.getByRole("row").filter({hasText: "Initialization Report"})
        await expect(init.getByRole("cell", {name: "Initialization template"})).toBeVisible()
        await expect(init.getByRole("cell", {name: "Mayor election"})).toBeVisible()
        const logs = page.getByRole("row").filter({hasText: "Activity Logs"})
        await expect(logs.getByRole("cell", {name: "Activity template"})).toBeVisible()
        const list = portal.graphql
            .callsTo("sequent_backend_report")
            .filter((call) => call.variables.limit)
            .at(-1)
        expect(list?.variables.where).toEqual({
            _and: [
                {election_event_id: {_eq: IDS.event}},
                {tenant_id: {_eq: TENANT_ID}},
                {
                    _or: [
                        {election_id: {_in: [IDS.election, CONTENT_IDS.secondElection]}},
                        {election_id: {_is_null: true}},
                    ],
                },
            ],
        })
    })

    test("creates an election report with a template", async ({page, portal}) => {
        const {rows, inserts} = reports(portal, [reportRow({report_type: "ACTIVITY_LOGS"})])
        await openEventTab(page, portal, "Reports")
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Create Report"})
        await drawer.getByRole("combobox", {name: "Type"}).click()
        await page.getByRole("option", {name: "Initialization Report", exact: true}).click()
        await drawer.getByRole("combobox", {name: "Election"}).fill("Mayor")
        await page.clock.runFor(500)
        await page.getByRole("option", {name: "Mayor election", exact: true}).click()
        await drawer.getByRole("combobox", {name: "Template"}).click()
        await page.getByRole("option", {name: "Initialization template", exact: true}).click()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Report created successfully")).toBeVisible()
        expect(inserts[0]).toEqual({
            object: {
                report_type: "INITIALIZATION_REPORT",
                election_id: IDS.election,
                template_alias: "init-report",
                encryption_policy: "unencrypted",
                tenant_id: TENANT_ID,
                election_event_id: IDS.event,
                cron_config: null,
            },
        })
        expect(rows).toHaveLength(2)
        await expect(page.getByRole("cell", {name: "Initialization Report"})).toBeVisible()
    })

    test("creates a password-protected report and sets up its encryption", async ({
        page,
        portal,
    }) => {
        const {inserts} = reports(portal, [reportRow()])
        portal.graphql.on("EncryptReport", () => ({
            data: {encrypt_report: {document_id: null, error_msg: null}},
        }))
        await openReports(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Create Report"})
        await drawer.getByRole("combobox", {name: "Type"}).click()
        await page.getByRole("option", {name: "Activity Logs", exact: true}).click()
        await drawer.getByRole("combobox", {name: "Encryption Policy"}).click()
        await page.getByRole("option", {name: "Configured Password", exact: true}).click()
        const passwords = page.getByRole("dialog").filter({hasText: "Repeat password"})
        const [password, repeat] = await passwords.locator('input[type="password"]').all()
        await password.fill("synthetic-report-secret")
        await repeat.fill("synthetic-report-secret")
        await passwords.getByRole("button", {name: "Save password"}).click()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Report created successfully")).toBeVisible()
        await expireNotification(page)
        await expect(notification(page, "Successfully set up report encryption")).toBeVisible()
        expect(inserts[0].object).toMatchObject({
            report_type: "ACTIVITY_LOGS",
            encryption_policy: "configured_password",
            cron_config: null,
        })
        expect(portal.graphql.callsTo("EncryptReport")[0].variables).toEqual({
            reportId: NEW_REPORT_ID,
            electionEventId: IDS.event,
            password: "synthetic-report-secret",
        })
        expectRole(portal, "EncryptReport", "report-write")
    })

    test("opens a single create drawer", async ({page, portal}) => {
        reports(portal)
        await openReports(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        await expect(page.getByRole("dialog").filter({hasText: "Create Report"})).toBeVisible()

        await expect(page.getByRole("dialog", {includeHidden: true})).toHaveCount(1, {
            timeout: 1000,
        })
    })

    test("keeps a report unencrypted when the passwords differ", async ({page, portal}) => {
        reports(portal, [reportRow()])
        await openReports(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Create Report"})
        await drawer.getByRole("combobox", {name: "Encryption Policy"}).click()
        await page.getByRole("option", {name: "Configured Password", exact: true}).click()
        const passwords = page.getByRole("dialog").filter({hasText: "Repeat password"})
        const [password, repeat] = await passwords.locator('input[type="password"]').all()
        await password.fill("first-secret")
        await repeat.fill("second-secret")
        await expect(passwords.getByRole("button", {name: "Save password"})).toBeDisabled()
        await page.keyboard.press("Escape")
        await expect(
            notification(page, "Password and confirm password do not match.")
        ).toBeVisible()
        await expect(drawer.getByRole("combobox", {name: "Encryption Policy"})).toHaveText(
            "Unencrypted"
        )
    })

    test("moves a report to another election and deletes another after confirmation", async ({
        page,
        portal,
    }) => {
        const {rows, updates} = reports(portal)
        await openReports(page, portal)
        await rowAction(page, "Initialization Report", "Edit")
        const drawer = page.getByRole("dialog").filter({hasText: "Edit Report"})
        await expect(drawer.getByRole("combobox", {name: "Type"})).toHaveValue(
            "Initialization Report"
        )
        await drawer.getByRole("combobox", {name: "Election"}).fill("Council")
        await page.clock.runFor(500)
        await page.getByRole("option", {name: "Council election", exact: true}).click()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Report updated successfully")).toBeVisible()
        expect(updates[0]).toMatchObject({
            id: CONTENT_IDS.report,
            set: {
                id: CONTENT_IDS.report,
                report_type: "INITIALIZATION_REPORT",
                election_id: CONTENT_IDS.secondElection,
                encryption_policy: "unencrypted",
                tenant_id: TENANT_ID,
                election_event_id: IDS.event,
                cron_config: null,
                created_at: FIXED_TIME,
                permission_label: null,
            },
        })

        await rowAction(page, "Activity Logs", "Delete")
        await page
            .getByRole("dialog")
            .filter({hasText: "Are you sure you want delete this Report?"})
            .getByRole("button", {name: "Delete", exact: true})
            .click()
        await expect(page.getByRole("cell", {name: "Activity Logs"})).not.toBeVisible()
        expect(portal.graphql.callsTo("delete_sequent_backend_report")[0].variables).toEqual({
            where: {id: {_eq: LOGS_REPORT_ID}},
        })
        expect(rows.map((row) => row.id)).toEqual([CONTENT_IDS.report])
    })

    test("keeps the saved template when editing a report", async ({page, portal}) => {
        const {updates} = reports(portal)
        await openReports(page, portal)
        await rowAction(page, "Initialization Report", "Edit")
        const drawer = page.getByRole("dialog").filter({hasText: "Edit Report"})
        await drawer.getByRole("combobox", {name: "Election"}).fill("Council")
        await page.clock.runFor(500)
        await page.getByRole("option", {name: "Council election", exact: true}).click()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Report updated successfully")).toBeVisible()

        expect(updates[0]?.set).toMatchObject({template_alias: "init-report"})
    })

    test("previews a report and downloads the generated document", async ({page, portal}) => {
        reports(portal)
        const url = generatedDocument(portal, "initialization.pdf")
        await openReports(page, portal)
        const download = page.waitForEvent("download")
        await rowAction(page, "Initialization Report", "Preview")
        expect((await download).suggestedFilename()).toBe("initialization.pdf")
        expect((await download).url()).toBe(url)
        expect(portal.graphql.callsTo("GenerateReport")[0].variables).toEqual({
            reportId: CONTENT_IDS.report,
            tenantId: TENANT_ID,
            reportMode: "PREVIEW",
            electionEventId: IDS.event,
        })
        expectRole(portal, "GenerateReport", "report-read")
        expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({
            electionEventId: IDS.event,
            documentId: REPORT_DOCUMENT_ID,
        })
    })

    test("generates an encrypted report and shows its password", async ({page, portal}) => {
        reports(portal)
        generatedDocument(portal, "activity.epdf", {password_secret_id: "synthetic-secret-id"})
        portal.graphql.on("GetDocumentPassword", () => ({
            data: {get_document_password: {password: "synthetic-report-password"}},
        }))
        await openReports(page, portal)
        const download = page.waitForEvent("download")
        await rowAction(page, "Activity Logs", "Generate")
        expect((await download).suggestedFilename()).toBe("activity.epdf")
        await expect(
            page.getByRole("dialog", {name: "Password"}).getByRole("textbox").first()
        ).toHaveValue("synthetic-report-password")
        expect(portal.graphql.callsTo("GenerateReport")[0].variables).toMatchObject({
            reportId: LOGS_REPORT_ID,
            reportMode: "REAL",
        })
        expect(portal.graphql.callsTo("GetDocumentPassword")[0].variables).toEqual({
            documentId: REPORT_DOCUMENT_ID,
        })
    })

    test("offers only the actions each report type supports", async ({page, portal}) => {
        reports(portal)
        await openReports(page, portal)
        await page
            .getByRole("row")
            .filter({hasText: "Initialization Report"})
            .getByRole("button", {name: "Actions", exact: true})
            .click()
        await expect(page.getByRole("menuitem")).toHaveText(["Edit", "Delete", "Preview"])
        await page.keyboard.press("Escape")
        await page
            .getByRole("row")
            .filter({hasText: "Activity Logs"})
            .getByRole("button", {name: "Actions", exact: true})
            .click()
        await expect(page.getByRole("menuitem")).toHaveText([
            "Edit",
            "Delete",
            "Generate",
            "Preview",
        ])
    })
})

test.describe("report reader", () => {
    test.use({roles: readerRoles})

    test("sees reports without creating or acting on them", async ({page, portal}) => {
        reports(portal)
        await openReports(page, portal)
        await expect(page.getByRole("button", {name: "Add", exact: true})).not.toBeVisible()
        await expect(page.getByRole("button", {name: "Actions", exact: true})).toHaveCount(0)
    })

    test("shows the empty state without the create button", async ({page, portal}) => {
        reports(portal, [])
        await openEventTab(page, portal, "Reports")
        await expect(page.getByText("No Reports yet.", {exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Create Report"})).not.toBeVisible()
    })
})
