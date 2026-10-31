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
    eventPage,
    expectRole,
    notification,
    openEventTab,
    table,
} from "./data"

const readerRoles = [...BASE_ROLES, "election-event-tasks-tab", "tasks-read"]
const operatorRoles = [
    ...readerRoles,
    "task-export",
    "election-event-tasks-columns",
    "election-event-tasks-filters",
    "election-event-tasks-back-button",
    "document-password-read",
    "document-download",
]
const LETTER_TASK_ID = "d0000000-0000-4000-8000-000000000002"
const EXPORT_DOCUMENT_ID = "a0000000-0000-4000-8000-000000000002"

function taskRow(overrides: Row = {}): Row {
    return {
        id: CONTENT_IDS.task,
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        name: "Export council event",
        type: "EXPORT_ELECTION_EVENT",
        execution_status: "SUCCESS",
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: "2026-01-15T12:05:00.000Z",
        executed_by_user: "synthetic-admin",
        annotations: {document_id: CONTENT_IDS.document},
        labels: {},
        logs: [{created_date: FIXED_TIME, log_text: "Export archive written"}],
        ...overrides,
    }
}

function tasks(portal: PortalServices) {
    eventPage(portal)
    const rows = table(portal, "sequent_backend_tasks_execution", [
        taskRow(),
        taskRow({
            id: LETTER_TASK_ID,
            name: "Voter letters",
            type: "VOTER_INFORMATION_LETTER",
            annotations: {document_id: "a0000000-0000-4000-8000-000000000003"},
            logs: [{created_date: FIXED_TIME, log_text: "Letters rendered"}],
        }),
    ])
    portal.graphql.on("GetTaskById", ({variables}) => ({
        data: {
            sequent_backend_tasks_execution: rows.filter((row) => row.id === variables.task_id),
        },
    }))
    return rows
}

/** Serves a private document through FetchDocument and the S3 mock. */
function servedDocument(portal: PortalServices, documentId: string, name: string) {
    const key = `tenant/${TENANT_ID}/documents/${documentId}`
    portal.s3.putBytes("private", key, Buffer.from('{"tasks":[]}'), "application/json")
    const url = portal.s3.presign(key, `download-${documentId}`)
    portal.graphql.on("GetDocument", ({variables}) => ({
        data: {
            sequent_backend_document: variables.id === documentId ? [{name, annotations: {}}] : [],
        },
    }))
    portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url}}}))
    return url
}

/** The view icon is the only (unlabelled) button in a task row. */
const viewTask = (page: Page, name: string) =>
    page.getByRole("row").filter({hasText: name}).getByRole("button").click()

async function openTasks(page: Page, portal: PortalServices) {
    await openEventTab(page, portal, "Tasks")
    await expect(page.getByRole("cell", {name: "Export council event", exact: true})).toBeVisible()
}

test.describe("task operator", () => {
    test.use({roles: operatorRoles})

    test("lists the event's tasks newest first and opens one with its logs", async ({
        page,
        portal,
    }) => {
        tasks(portal)
        await openTasks(page, portal)
        await expect(page.getByText("Tasks Execution", {exact: true})).toBeVisible()
        await expect(page.getByRole("cell", {name: "Voter letters", exact: true})).toBeVisible()
        const list = portal.graphql.callsTo("sequent_backend_tasks_execution")[0].variables
        expect(list).toMatchObject({
            where: {_and: [{election_event_id: {_eq: IDS.event}}]},
            order_by: {start_at: "desc"},
            limit: 10,
            offset: 0,
        })
        await viewTask(page, "Export council event")
        await expect(page.getByText("Task Information", {exact: true})).toBeVisible()
        await expect(page.getByText("status: SUCCESS", {exact: true})).toBeVisible()
        await expect(
            page.getByRole("cell", {name: "Export Election Event", exact: true})
        ).toBeVisible()
        await expect(page.getByRole("cell", {name: "synthetic-admin", exact: true})).toBeVisible()
        await expect(
            page.getByRole("cell", {name: "Export archive written", exact: true})
        ).toBeVisible()
        expect(portal.graphql.callsTo("GetTaskById").at(-1)?.variables).toEqual({
            task_id: CONTENT_IDS.task,
        })
        await page.getByRole("button", {name: "Back", exact: true}).click()
        await expect(page.getByRole("cell", {name: "Voter letters", exact: true})).toBeVisible()
    })

    test("downloads a finished task's document", async ({page, portal}) => {
        tasks(portal)
        const url = servedDocument(portal, CONTENT_IDS.document, "council-export.zip")
        await openTasks(page, portal)
        await viewTask(page, "Export council event")
        const download = page.waitForEvent("download")
        await page.getByRole("button", {name: "Download File"}).click()
        expect((await download).suggestedFilename()).toBe("council-export.zip")
        expect((await download).url()).toBe(url)
        expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({
            electionEventId: IDS.event,
            documentId: CONTENT_IDS.document,
        })
        expect(portal.graphql.callsTo("GetDocument")[0].variables).toEqual({
            id: CONTENT_IDS.document,
            tenantId: TENANT_ID,
        })
    })

    test("reveals and copies a voter letter password before downloading", async ({
        page,
        portal,
        context,
    }) => {
        await context.grantPermissions(["clipboard-read", "clipboard-write"])
        tasks(portal)
        const letterDocument = "a0000000-0000-4000-8000-000000000003"
        servedDocument(portal, letterDocument, "letters.pdf")
        portal.graphql.on("GetDocumentPassword", () => ({
            data: {get_document_password: {password: "synthetic-pdf-secret"}},
        }))
        await openTasks(page, portal)
        await viewTask(page, "Voter letters")
        await expect(page.getByText("Document access", {exact: true})).toBeVisible()
        await page.getByRole("button", {name: "Show password"}).click()
        const password = page.getByRole("textbox", {name: "Password to open the encrypted PDF"})
        await expect(password).toHaveValue("synthetic-pdf-secret")
        expect(portal.graphql.callsTo("GetDocumentPassword")[0].variables).toEqual({
            documentId: letterDocument,
        })
        expectRole(portal, "GetDocumentPassword", "document-password-read")
        await page.getByRole("button", {name: "Copy password"}).click()
        await expect(notification(page, "Password copied")).toBeVisible()
        expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
            "synthetic-pdf-secret"
        )
        const download = page.waitForEvent("download")
        await page.getByRole("button", {name: "Download File"}).click()
        expect((await download).suggestedFilename()).toBe("letters.pdf")
        // The password is already known: downloading does not ask for it again.
        expect(portal.graphql.callsTo("GetDocumentPassword")).toHaveLength(1)
    })

    test("notifies when a voter letter password cannot be retrieved", async ({page, portal}) => {
        tasks(portal)
        portal.graphql.on("GetDocumentPassword", () => ({
            errors: [{message: "Synthetic missing secret", extensions: {code: "unexpected"}}],
        }))
        await openTasks(page, portal)
        await viewTask(page, "Voter letters")
        await page.getByRole("button", {name: "Download File"}).click()
        await expect(notification(page, "The PDF password could not be retrieved")).toBeVisible()
        expect(portal.graphql.callsTo("FetchDocument")).toHaveLength(0)
    })

    test("exports the event's task executions and downloads the archive", async ({
        page,
        portal,
    }) => {
        tasks(portal)
        const url = servedDocument(portal, EXPORT_DOCUMENT_ID, "tasks.json")
        portal.graphql.on("ExportTasksExecution", () => ({
            data: {export_tasks_execution: {error_msg: null, document_id: EXPORT_DOCUMENT_ID}},
        }))
        await openTasks(page, portal)
        await page.getByRole("button", {name: "Export", exact: true}).click()
        const dialog = page.getByRole("dialog")
        const download = page.waitForEvent("download")
        await dialog.getByRole("button", {name: "Export", exact: true}).click()
        expect((await download).suggestedFilename()).toBe("export-tasks-execution.json")
        expect((await download).url()).toBe(url)
        await expect(notification(page, "Export finished successfully")).toBeVisible()
        expect(portal.graphql.callsTo("ExportTasksExecution")[0].variables).toEqual({
            tenantId: TENANT_ID,
            electionEventId: IDS.event,
        })
        expectRole(portal, "ExportTasksExecution", "task-export")
    })

    test("reports a failed task export", async ({page, portal}) => {
        tasks(portal)
        portal.graphql.on("ExportTasksExecution", () => ({
            errors: [{message: "Synthetic export failure", extensions: {code: "unexpected"}}],
        }))
        await openTasks(page, portal)
        await page.getByRole("button", {name: "Export", exact: true}).click()
        await page.getByRole("dialog").getByRole("button", {name: "Export", exact: true}).click()
        await expect(notification(page, "Error exporting Tasks Execution")).toBeVisible()
        await expect(page.getByRole("dialog")).not.toBeVisible()
        expect(portal.graphql.callsTo("FetchDocument")).toHaveLength(0)
    })
})

test.describe("task reader", () => {
    test.use({roles: readerRoles})

    test("views tasks without export, back navigation or letter passwords", async ({
        page,
        portal,
    }) => {
        tasks(portal)
        await openTasks(page, portal)
        await expect(page.getByRole("button", {name: "Export", exact: true})).not.toBeVisible()
        await viewTask(page, "Voter letters")
        await expect(page.getByText("Letters rendered", {exact: true})).toBeVisible()
        await expect(page.getByText("Document access", {exact: true})).not.toBeVisible()
        await expect(page.getByRole("button", {name: "Download File"})).not.toBeVisible()
        await expect(page.getByRole("button", {name: "Back", exact: true})).not.toBeVisible()
        expect(portal.graphql.callsTo("GetDocumentPassword")).toHaveLength(0)
    })
})
