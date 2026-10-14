// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    ELECTION_TYPE_ID,
    SETTINGS_ROLES,
    electionType,
    listOf,
    mockElectionTypes,
    mockTenant,
    openSettings,
    rowWith,
    serveDocument,
} from "./data"

const PREVIEW_DOCUMENT_ID = "50000000-0000-4000-8000-000000000002"
const PREVIEW_URL = "https://voting.synthetic.example/preview/50000000"
const TABS = [
    "ELECTION TYPES",
    "VOTING CHANELS",
    "LANGUAGES",
    "LOCALIZATION",
    "Integrations",
    "Look & Feel",
    "TRUSTEES",
    "Countries",
    "Backup / Restore",
    "Previews",
]

for (const [missing, roles] of [
    ["tenant-write", ["admin-user", "settings-menu"]],
    ["settings-menu", ["admin-user", "tenant-write"]],
] as const)
    test.describe(`without ${missing}`, () => {
        test.use({roles: [...roles]})

        test("shows the no-permission message instead of the settings", async ({page, portal}) => {
            await page.goto(`${portal.origin}/settings?lang=en`)
            await expect(
                page.getByText("You don't have permission to access settings.", {exact: true})
            ).toBeVisible()
            await expect(page.getByRole("tab")).toHaveCount(0)
            expect(portal.graphql.callsTo("sequent_backend_election_type")).toEqual([])
        })
    })

test.describe("with settings access", () => {
    test.use({roles: SETTINGS_ROLES})
    test.beforeEach(({portal}) => {
        mockTenant(portal)
    })

    test("shows every settings tab, starting on election types", async ({page, portal}) => {
        mockElectionTypes(portal, [electionType()])
        await openSettings(page, portal)
        await expect(page.getByText("General Configuration", {exact: true})).toBeVisible()
        await expect(page.getByRole("tab")).toHaveText(TABS)
        await expect(page.getByRole("tab", {name: "ELECTION TYPES"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(rowWith(page, "General election")).toBeVisible()
        expect(portal.graphql.callsTo("sequent_backend_election_type")[0].variables).toEqual({
            where: {_and: []},
            limit: 10,
            offset: 0,
            order_by: {id: "asc"},
        })
    })

    test("opens the settings screen from an election type's route", async ({page, portal}) => {
        mockElectionTypes(portal, [electionType()])
        await page.goto(
            `${portal.origin}/sequent_backend_election_type/${ELECTION_TYPE_ID}?lang=en`
        )
        await expect(page.getByRole("tab")).toHaveText(TABS)
        await expect(rowWith(page, "General election")).toBeVisible()
    })

    test("hides previews without the preview-read permission", async ({page, portal}) => {
        mockElectionTypes(portal)
        await openSettings(page, portal, "Previews")
        await expect(page.getByText("No Previews found", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("sequent_backend_preview")).toEqual([])
    })
})

test.describe("with preview-read", () => {
    test.use({roles: [...SETTINGS_ROLES, "preview-read"]})
    test.beforeEach(({portal}) => {
        mockTenant(portal)
        mockElectionTypes(portal)
    })

    test("lists external preview requests and downloads a preview document", async ({
        page,
        portal,
    }) => {
        portal.graphql.on(
            "sequent_backend_preview",
            listOf("sequent_backend_preview", () => [
                {
                    id: "50000000-0000-4000-8000-000000000001",
                    tenant_id: TENANT_ID,
                    document_id: PREVIEW_DOCUMENT_ID,
                    requested_by: "ballot-api",
                    url: PREVIEW_URL,
                    annotations: {},
                    created_at: FIXED_TIME,
                    updated_at: FIXED_TIME,
                },
            ])
        )
        const {url} = serveDocument(portal, PREVIEW_DOCUMENT_ID, "ballot-preview.pdf")
        await openSettings(page, portal, "Previews")
        await expect(page.getByText("External Previews", {exact: true})).toBeVisible()
        await expect(
            page.getByText("A record of ballot style previews generated via external requests", {
                exact: true,
            })
        ).toBeVisible()
        const row = rowWith(page, "ballot-api")
        await expect(row.getByRole("link", {name: PREVIEW_URL, exact: true})).toHaveAttribute(
            "href",
            PREVIEW_URL
        )
        const download = page.waitForEvent("download")
        // The download control is an icon-only button.
        await row.getByRole("button").click()

        expect((await download).suggestedFilename()).toBe("ballot-preview.pdf")
        expect((await download).url()).toBe(url)
        expect(portal.graphql.callsTo("GetDocument")[0].variables).toEqual({
            id: PREVIEW_DOCUMENT_ID,
            tenantId: TENANT_ID,
        })
    })

    test("shows an empty message when there are no previews", async ({page, portal}) => {
        portal.graphql.on(
            "sequent_backend_preview",
            listOf("sequent_backend_preview", () => [])
        )
        await openSettings(page, portal, "Previews")
        await expect(page.getByText("No Previews found", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("sequent_backend_preview")).toHaveLength(1)
    })
})

test.describe("with messaging account access", () => {
    test.use({roles: [...SETTINGS_ROLES, "messaging-account-read"]})
    test.beforeEach(({portal}) => {
        mockTenant(portal)
    })

    test("shows the messaging tab after the voting channels", async ({page, portal}) => {
        mockElectionTypes(portal)
        portal.graphql.on("GetMessagingAccounts", () => ({
            data: {sequent_backend_messaging_account: []},
        }))
        portal.graphql.on("GetMessagingCallbackStatus", () => ({
            data: {messaging_callback_status: {accounts: []}},
        }))
        portal.graphql.on("GetMessagingMonthlyVolumes", ({variables}) => ({
            data: {messaging_monthly_volumes: {month: variables.month, rows: []}},
        }))
        await openSettings(page, portal, "MESSAGING")
        await expect(page.getByRole("tab")).toHaveText([
            ...TABS.slice(0, 2),
            "MESSAGING",
            ...TABS.slice(2),
        ])
        await expect(page.getByText("No sending accounts yet.", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("GetMessagingAccounts")[0].variables).toEqual({
            tenantId: TENANT_ID,
        })
    })
})

/** The tenant's log as `listTenantLog` answers it: newest first. */
const tenantLogEntry = (
    id: number,
    kind: string,
    username: string | null,
    head: {event_type: string; log_type: string; description: string},
    details?: Record<string, unknown>
) => ({
    id,
    created: 1_841_396_400 + id * 60,
    statement_timestamp: 1_841_396_400 + id * 60,
    statement_kind: kind,
    user_id: username ? `${username}-id` : null,
    username,
    message: JSON.stringify({
        user_id: username ? `${username}-id` : null,
        username,
        statement: {
            head: {event: "Generic Event", kind, ...head},
            body: details
                ? {TenantActivity: {kind, ...head, details_json: JSON.stringify(details)}}
                : {KeycloakUserEvent: ["null", "LOGIN"]},
        },
    }),
})

const TENANT_LOG = [
    tenantLogEntry(
        3,
        "TenantAdminAction",
        "ada.admin",
        {
            event_type: "USER",
            log_type: "INFO",
            description: "Request to create an election event accepted.",
        },
        {
            action: "create-election-event",
            outcome: "succeeded",
            status: 200,
            user_id: "ada.admin-id",
            username: "ada.admin",
            details: {name: "Synthetic election"},
        }
    ),
    tenantLogEntry(2, "KeycloakUserEvent", "ada.admin", {
        event_type: "USER",
        log_type: "INFO",
        description: "LOGIN",
    }),
    tenantLogEntry(
        1,
        "TenantLogOpened",
        null,
        {event_type: "SYSTEM", log_type: "INFO", description: "Opened the tenant log."},
        {board: "synthetictenant", public_key: "synthetic-key"}
    ),
]

/** The log's row whose description starts with `text` (a long one ends in Show More). */
const logEntry = (page: Page, text: string) => page.getByRole("row").filter({hasText: text})

function mockTenantLog(portal: Parameters<typeof mockTenant>[0]) {
    portal.graphql.on("listTenantLog", () => ({
        data: {
            listTenantLog: {
                items: TENANT_LOG,
                total: {aggregate: {count: TENANT_LOG.length}},
            },
        },
    }))
}

test.describe("with the tenant's log and no settings access", () => {
    test.use({roles: ["admin-user", "settings-menu", "tenant-logs-read"]})

    test("shows the Logs tab alone, with the tenant's entries", async ({page, portal}) => {
        mockTenantLog(portal)
        await page.goto(`${portal.origin}/settings?lang=en`)
        await expect(page.getByRole("tab")).toHaveText(["Logs"])
        await expect(logEntry(page, "Request to create an election event accepted.")).toBeVisible()
        await expect(logEntry(page, "Opened the tenant log.")).toBeVisible()
        await expect(page.getByRole("cell", {name: "ada.admin", exact: true})).toHaveCount(2)
        // The list asks for the tenant's log, never for an election event's.
        expect(portal.graphql.callsTo("listTenantLog")[0].variables).toMatchObject({
            limit: 10,
            offset: 0,
            filter: {},
            order_by: {id: "desc"},
        })
        expect(portal.graphql.callsTo("listElectoralLog")).toEqual([])
        expect(portal.graphql.callsTo("sequent_backend_election_type")).toEqual([])

        // A request's entry opens to what was asked, how it ended and by whom.
        const request = logEntry(page, "Request to create an election event accepted.")
        await request.getByRole("button", {name: "Show More"}).click()
        await expect(request.getByText("Action: create-election-event")).toBeVisible()
        await expect(request.getByText("Accepted (HTTP 200)")).toBeVisible()
        await expect(request.getByText("Requested by ada.admin (ada.admin-id)")).toBeVisible()
        await expect(request.getByText("name: Synthetic election")).toBeVisible()
    })
})

test.describe("with settings access and the tenant's log", () => {
    test.use({roles: [...SETTINGS_ROLES, "tenant-logs-read"]})
    test.beforeEach(({portal}) => {
        mockTenant(portal)
    })

    test("shows the Logs tab after the settings tabs", async ({page, portal}) => {
        mockElectionTypes(portal, [electionType()])
        mockTenantLog(portal)
        await openSettings(page, portal, "Logs")
        await expect(page.getByRole("tab")).toHaveText([...TABS, "Logs"])
        await expect(logEntry(page, "Opened the tenant log.")).toBeVisible()
        expect(portal.graphql.callsTo("listTenantLog")).toHaveLength(1)
    })
})
