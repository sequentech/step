// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, TENANT_ID, type AdminPortal} from "../fixtures"
import {
    AUDITOR_ROLES,
    KEYCLOAK_CHANGE,
    SETTINGS_CHANGED,
    SIGN_IN_FAILED,
    TENANT_LOG_SUBTITLE,
    mockTask,
    mockTenant,
    mockTenantLog,
    taskExecution,
    tenantRow,
} from "./data"

const LOG_ZONE = "Europe/Madrid"

/** The tenant whose log is shown in Madrid time. */
function mockLogTenant(portal: AdminPortal) {
    return mockTenant(
        portal,
        tenantRow({
            settings: {
                language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
                logs_timezone: LOG_ZONE,
            },
        })
    )
}

async function openLogs(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}/settings?lang=en`)
    await expect(page.getByText(SETTINGS_CHANGED, {exact: true})).toBeVisible()
}

async function addFilter(page: Page, filter: string, option: string) {
    await page.getByRole("button", {name: "Add filter", exact: true}).click()
    await page.getByRole("menuitemcheckbox", {name: filter, exact: true}).click()
    await page.getByRole("combobox", {name: filter, exact: true}).click()
    await page.getByRole("option", {name: option, exact: true}).click()
}

const listCalls = (portal: AdminPortal) => portal.graphql.callsTo("listTenantLog")

test.describe("the tenant log of an auditor", () => {
    test.use({roles: AUDITOR_ROLES, viewport: {width: 1920, height: 1080}})
    test.beforeEach(({portal}) => {
        mockLogTenant(portal)
        mockTenantLog(portal)
    })

    test("is read without an election event, as tenant-logs-read", async ({page, portal}) => {
        await openLogs(page, portal)
        await expect(page.getByRole("heading", {name: "Logs", exact: true})).toBeVisible()
        await expect(page.getByText(TENANT_LOG_SUBTITLE, {exact: true})).toBeVisible()

        const calls = listCalls(portal)
        expect(calls.length).toBeGreaterThan(0)
        for (const call of calls) {
            expect(call.headers["x-hasura-role"]).toBe("tenant-logs-read")
            expect(call.query).not.toContain("election_event_id")
            expect(JSON.stringify(call.variables)).not.toContain("election_event_id")
        }
        expect(calls.at(-1)?.variables).toMatchObject({
            limit: 10,
            offset: 0,
            order_by: {id: "desc"},
        })
        expect(
            portal.oidc.verifyAccessToken(calls[0].headers.authorization.replace(/^Bearer /, ""))
        ).toMatchObject({"https://hasura.io/jwt/claims": {"x-hasura-tenant-id": TENANT_ID}})
        // An election event's log is not read.
        expect(portal.graphql.callsTo("listElectoralLog")).toEqual([])
    })

    test("shows its entries in the tenant's log timezone, without an event's columns", async ({
        page,
        portal,
    }) => {
        await openLogs(page, portal)
        const headers = page.getByRole("columnheader")
        await expect(headers).toHaveText([
            "Id",
            "Username",
            "Created",
            "Statement Timestamp",
            "Statement kind",
            "Event Type",
            "Log Type",
            "Description",
        ])
        const row = page.getByRole("row").filter({hasText: SETTINGS_CHANGED})
        await expect(row).toContainText("5,259")
        await expect(row).toContainText("rosa.marin")
        await expect(row).toContainText("AdminChange")
        await expect(row).toContainText("USERINFO")
        // 09:21:37 UTC is 10:21:37 in Madrid.
        await expect(row).toContainText("Feb 3, 2028, 10:21:37 AM GMT+1")
        await expect(page.getByRole("button", {name: "Audit", exact: true})).toHaveCount(0)
        await expect(page.getByRole("combobox", {name: "Timezone", exact: true})).toHaveValue(
            /Madrid · primary$/
        )
    })

    test("filters by event type and log type", async ({page, portal}) => {
        await openLogs(page, portal)
        await addFilter(page, "Event Type", "SYSTEM")
        await expect(page.getByText(SETTINGS_CHANGED, {exact: true})).toHaveCount(0)
        await expect(page.getByText(KEYCLOAK_CHANGE, {exact: true})).toBeVisible()
        expect(listCalls(portal).at(-1)?.variables.filter).toEqual({event_type: "SYSTEM"})

        await page.getByRole("button", {name: "Remove this filter", exact: true}).click()
        await expect(page.getByText(SETTINGS_CHANGED, {exact: true})).toBeVisible()
        await addFilter(page, "Log Type", "ERROR")
        await expect(page.getByText(SETTINGS_CHANGED, {exact: true})).toHaveCount(0)
        await expect(page.getByText(SIGN_IN_FAILED, {exact: true})).toBeVisible()
        expect(listCalls(portal).at(-1)?.variables.filter).toEqual({log_type: "ERROR"})
        expect(listCalls(portal).at(-1)?.headers["x-hasura-role"]).toBe("tenant-logs-read")
    })

    test("offers the tenant log's statement kinds", async ({page, portal}) => {
        await openLogs(page, portal)
        await page.getByRole("button", {name: "Add filter", exact: true}).click()
        // Messaging filters belong to an election event's log.
        await expect(page.getByRole("menuitemcheckbox", {name: "Channel"})).toHaveCount(0)
        await page.getByRole("menuitemcheckbox", {name: "Statement kind", exact: true}).click()
        await page.getByRole("combobox", {name: "Statement kind", exact: true}).click()
        await expect(page.getByRole("option").filter({hasText: /\w/})).toHaveText([
            "KeycloakUserEvent",
            "AdminChange",
            "KeycloakAdminEvent",
            "LogsExported",
            "LogWriterStarted",
            "LogWriterStopped",
            "AdminPublicKey",
        ])
        await page.getByRole("option", {name: "KeycloakUserEvent", exact: true}).click()
        await expect(page.getByText(SIGN_IN_FAILED, {exact: true})).toBeVisible()
        await expect(page.getByText(SETTINGS_CHANGED, {exact: true})).toHaveCount(0)
        expect(listCalls(portal).at(-1)?.variables.filter).toEqual({
            statement_kind: "KeycloakUserEvent",
        })
    })

    test("opens an entry's details on a click, and closes them with Escape", async ({
        page,
        portal,
    }) => {
        await openLogs(page, portal)
        await page.getByText(SETTINGS_CHANGED, {exact: true}).click()
        const dialog = page.getByRole("dialog", {name: "Log entry 5,259", exact: true})
        await expect(dialog).toBeVisible()
        await expect(dialog.getByText(SETTINGS_CHANGED, {exact: true})).toBeVisible()
        await expect(dialog.getByText("Rosa Marin (rosa.marin)", {exact: true})).toBeVisible()
        await expect(dialog.getByText("2028-02-03T09:21:37Z UTC", {exact: true})).toBeVisible()
        await expect(dialog.getByText("Tenant settings: Synthetic", {exact: true})).toBeVisible()
        const timezone = dialog.getByRole("row").filter({hasText: "settings.logs_timezone"})
        await expect(timezone.getByRole("cell")).toHaveText(["UTC", LOG_ZONE])
        await expect(
            dialog.getByText("Changed. Secret values are never logged.", {exact: true})
        ).toBeVisible()
        await expect(dialog.getByText("Tenant log", {exact: true})).toBeVisible()
        await expect(dialog.getByText("Rosa Marin's admin key", {exact: true})).toBeVisible()
        await expect(dialog.getByRole("button")).toHaveText(["Close"])

        await page.keyboard.press("Escape")
        await expect(page.getByRole("dialog")).toHaveCount(0)

        // Another entry opens its own details; Close dismisses them too.
        await page.getByText(SIGN_IN_FAILED, {exact: true}).click()
        const signIn = page.getByRole("dialog", {name: "Log entry 5,257", exact: true})
        await expect(signIn.getByText("LOGIN_ERROR", {exact: true})).toBeVisible()
        await expect(signIn.getByText("203.0.113.77", {exact: true})).toBeVisible()
        await signIn.getByRole("button", {name: "Close", exact: true}).click()
        await expect(page.getByRole("dialog")).toHaveCount(0)
        expect(portal.graphql.calls.some((call) => call.query.trim().startsWith("mutation"))).toBe(
            false
        )
    })

    test("exports one account with its election event entries as SQL", async ({page, portal}) => {
        let done = false
        portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
        const task = () =>
            taskExecution("EXPORT_TENANT_LOGS", {
                execution_status: done ? "SUCCESS" : "IN_PROGRESS",
            })
        mockTask(portal, task)
        portal.graphql.on("ExportTenantLogs", () => ({
            data: {
                export_tenant_logs: {document_id: "tenant-logs-document", task_execution: task()},
            },
        }))
        await openLogs(page, portal)
        await page.getByRole("button", {name: "Export", exact: true}).click()
        const dialog = page.getByRole("dialog", {name: "Export logs", exact: true})
        await dialog.getByRole("combobox", {name: "Scope"}).click()
        await expect(page.getByRole("option")).toHaveText(["The whole tenant log", "One account"])
        await page.getByRole("option", {name: "One account", exact: true}).click()
        // One account needs its username.
        await expect(dialog.getByRole("button", {name: "Export", exact: true})).toBeDisabled()
        await dialog.getByRole("textbox", {name: "Username"}).fill("rosa.marin")
        await dialog
            .getByRole("checkbox", {
                name: "Include this account's entries in every election event",
                exact: true,
            })
            .check()
        await dialog.getByRole("radio", {name: "SQL", exact: true}).check()
        await expect(
            dialog.getByText("Rows from election events name their log.", {exact: true})
        ).toBeVisible()
        await expect(
            dialog.getByText("The export itself is recorded in this log.", {exact: true})
        ).toBeVisible()
        expect(portal.graphql.callsTo("ExportTenantLogs")).toHaveLength(0)
        await dialog.getByRole("button", {name: "Export", exact: true}).click()
        await expect(page.getByRole("dialog")).toHaveCount(0)

        await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
        const calls = portal.graphql.callsTo("ExportTenantLogs")
        expect(calls).toHaveLength(1)
        expect(calls[0].variables).toEqual({
            format: "SQL",
            scope: "ACCOUNT_WITH_EVENTS",
            username: "rosa.marin",
            createdFrom: null,
            createdTo: null,
            timeZone: LOG_ZONE,
        })
        expect(calls[0].headers["x-hasura-role"]).toBe("tenant-logs-export")
        expect(portal.graphql.callsTo("ExportElectionEventLogs")).toEqual([])
        done = true
        await page.clock.runFor(250)
        await expect(page.getByText("SUCCESS", {exact: true})).toBeVisible()
    })

    test("exports the whole tenant log as CSV by default", async ({page, portal}) => {
        mockTask(portal, () => taskExecution("EXPORT_TENANT_LOGS"))
        portal.graphql.on("ExportTenantLogs", () => ({
            data: {
                export_tenant_logs: {
                    document_id: "tenant-logs-document",
                    task_execution: taskExecution("EXPORT_TENANT_LOGS"),
                },
            },
        }))
        await openLogs(page, portal)
        await page.getByRole("button", {name: "Export", exact: true}).click()
        const dialog = page.getByRole("dialog", {name: "Export logs", exact: true})
        await expect(dialog.getByRole("radio")).toHaveCount(3)
        await expect(dialog.getByRole("textbox", {name: "Username"})).toHaveCount(0)
        await dialog.getByRole("button", {name: "Export", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("ExportTenantLogs").length).toBe(1)
        expect(portal.graphql.callsTo("ExportTenantLogs")[0].variables).toEqual({
            format: "CSV",
            scope: "TENANT_LOG",
            username: null,
            createdFrom: null,
            createdTo: null,
            timeZone: LOG_ZONE,
        })
    })
})

test.describe("without tenant-logs-export", () => {
    test.use({roles: AUDITOR_ROLES.filter((role) => role !== "tenant-logs-export")})

    test("shows the tenant log but no export", async ({page, portal}) => {
        mockLogTenant(portal)
        mockTenantLog(portal)
        await openLogs(page, portal)
        await expect(page.getByRole("button", {name: "Add filter", exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Export", exact: true})).toHaveCount(0)
        expect(portal.graphql.calls.some((call) => call.query.trim().startsWith("mutation"))).toBe(
            false
        )
    })
})

test.describe("on a narrow screen", () => {
    test.use({roles: AUDITOR_ROLES, viewport: {width: 700, height: 900}})

    test("shows each entry on two lines and opens the same details", async ({page, portal}) => {
        mockLogTenant(portal)
        mockTenantLog(portal)
        await openLogs(page, portal)
        await expect(page.getByRole("table")).toHaveCount(0)
        const entry = page.getByRole("listitem").filter({hasText: SIGN_IN_FAILED})
        await expect(entry).toContainText("lena.ortiz · KeycloakUserEvent · ERROR")
        await entry.getByRole("button").click()
        await expect(page.getByRole("dialog", {name: "Log entry 5,257", exact: true})).toBeVisible()
        await page.keyboard.press("Escape")
        await expect(page.getByRole("dialog")).toHaveCount(0)
    })
})
