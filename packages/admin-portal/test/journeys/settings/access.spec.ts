// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    AUDITOR_ROLES,
    ELECTION_TYPE_ID,
    SETTINGS_CHANGED,
    SETTINGS_ROLES,
    TENANT_LOG_SUBTITLE,
    electionType,
    listOf,
    mockElectionTypes,
    mockTenant,
    mockTenantLog,
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
    ["tenant-write and tenant-logs-read", ["admin-user", "settings-menu", "logs-read"]],
    ["settings-menu", ["admin-user", "tenant-write", "tenant-logs-read", "tenant-logs-export"]],
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
            expect(portal.graphql.callsTo("listTenantLog")).toEqual([])
        })
    })

test.describe("an auditor, with tenant-logs-read and without tenant-write", () => {
    test.use({roles: AUDITOR_ROLES})

    test("sees only the Logs tab, with the tenant log", async ({page, portal}) => {
        mockTenant(portal)
        mockTenantLog(portal)
        await page.goto(`${portal.origin}/settings?lang=en`)
        await expect(page.getByText("General Configuration", {exact: true})).toBeVisible()
        await expect(page.getByRole("tab")).toHaveText(["Logs"])
        await expect(page.getByRole("tab", {name: "Logs"})).toHaveAttribute("aria-selected", "true")
        await expect(page.getByText(TENANT_LOG_SUBTITLE, {exact: true})).toBeVisible()
        await expect(page.getByText(SETTINGS_CHANGED, {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("sequent_backend_election_type")).toEqual([])
    })
})

test.describe("a security officer, with tenant-write and tenant-logs-read", () => {
    test.use({roles: [...SETTINGS_ROLES, "tenant-logs-read", "tenant-logs-export"]})

    test("sees every settings tab, with Logs after Previews", async ({page, portal}) => {
        mockTenant(portal)
        mockTenantLog(portal)
        mockElectionTypes(portal, [electionType()])
        await openSettings(page, portal)
        await expect(page.getByRole("tab")).toHaveText([...TABS, "Logs"])
        await expect(rowWith(page, "General election")).toBeVisible()
        // The tenant log is read only once its tab opens.
        expect(portal.graphql.callsTo("listTenantLog")).toEqual([])
        await page.getByRole("tab", {name: "Logs", exact: true}).click()
        await expect(page.getByText(SETTINGS_CHANGED, {exact: true})).toBeVisible()
    })
})

test.describe("a configuration manager, with tenant-write and without tenant-logs-read", () => {
    test.use({roles: SETTINGS_ROLES})
    test.beforeEach(({portal}) => {
        mockTenant(portal)
    })

    test("shows every settings tab but Logs, starting on election types", async ({
        page,
        portal,
    }) => {
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
