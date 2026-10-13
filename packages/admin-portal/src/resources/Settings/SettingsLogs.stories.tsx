// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {Box} from "@mui/material"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {IPermissions} from "@/types/keycloak"
import {
    TENANT_LOG_PAGE,
    TENANT_LOG_ZONE,
    tenantLogRows,
} from "@/components/logs/__stories__/tenantLogFixture"
import {SettingsLogs} from "./SettingsLogs"
import {TENANT_RESOURCE, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** The signed-in user's roles. */
    roles: string[]
    /** The tenant setting; UTC when the tenant has none. */
    logsTimezone?: string
    /** The viewer's zone. */
    myZone: string
    /** The room the tab has, in pixels. */
    width: number
}

const AUDITOR_ROLES = [IPermissions.TENANT_LOGS_READ, IPermissions.TENANT_LOGS_EXPORT]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsLogs",
    component: SettingsLogs,
    args: {
        roles: AUDITOR_ROLES,
        logsTimezone: TENANT_LOG_ZONE,
        myZone: TENANT_LOG_ZONE,
        width: 1600,
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            electoral_log: tenantLogRows(),
            [TENANT_RESOURCE]: [
                settingsTenant(args.logsTimezone ? {logs_timezone: args.logsTimezone} : {}),
            ],
        })
        graphql = graphqlBoundary(
            {
                ExportTenantLogs: () => ({
                    data: {
                        export_tenant_logs: {
                            document_id: STORY_IDS.tallySession,
                            task_execution: {
                                id: STORY_IDS.keysCeremony,
                                name: "Export tenant logs",
                                execution_status: "IN_PROGRESS",
                                created_at: "2028-02-03T10:00:00.000Z",
                                start_at: "2028-02-03T10:00:00.000Z",
                                end_at: null,
                                logs: [],
                                annotations: {},
                                labels: {},
                                executed_by_user: "ana.vega",
                                tenant_id: STORY_IDS.tenant,
                                election_event_id: STORY_IDS.event,
                                type: "EXPORT_TENANT_LOGS",
                            },
                        },
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({roles, myZone, width}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <MyTimeZoneProvider zone={myZone}>
                <Box sx={{width}}>
                    <SettingsLogs />
                </Box>
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const listed = () =>
    data.calls.filter(({method, args}) => method === "getList" && args[0] === "electoral_log")
const lastListed = () =>
    listed().at(-1)?.args[1] as {filter: Record<string, unknown>; meta: unknown}

const headers = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll("thead th")).map(({textContent}) => textContent)

const addFilter = async (canvasElement: HTMLElement, filter: string) => {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Add filter"}))
    await userEvent.click(
        await within(document.body).findByRole("menuitemcheckbox", {name: filter})
    )
}

/**
 * The tenant log: failed sign-ins and a lockout, a settings change, a change
 * made in Keycloak's console, a deleted election event and an export, in the
 * tenant's log zone.
 */
export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Logs"})).toBeVisible()
        await expect(
            canvas.getByText(
                "Sign-ins, staff accounts and roles, settings and election events of this organization. Each election event keeps its own log in its Logs tab."
            )
        ).toBeVisible()
        await expect(
            await canvas.findByText("Settings changed: Languages, Integrations and Voting Channels")
        ).toBeVisible()
        // The tenant has no messages: no Channel or Status column.
        expect(headers(canvasElement)).toEqual([
            "Id",
            "Username",
            "Created",
            "Statement Timestamp",
            "Statement kind",
            "Event Type",
            "Log Type",
            "Description",
        ])
        const rows = canvas.getAllByRole("row").slice(1)
        expect(rows).toHaveLength(TENANT_LOG_PAGE)
        expect(canvas.getByText(`1-${TENANT_LOG_PAGE} of ${tenantLogRows().length}`)).toBeVisible()
        const lockout = within(
            canvas
                .getByText("Account locked for 1 minute after 30 failed sign-ins in a row")
                .closest("tr") as HTMLElement
        )
        expect(lockout.getByText("lena.ortiz")).toBeVisible()
        expect(lockout.getByText("ERROR")).toBeVisible()
        // 05:43:57 UTC on 2 February is 06:43:57 in Madrid.
        expect(lockout.getAllByText(/Feb 2, 2028, 6:43:5\d AM GMT\+1/)).toHaveLength(2)
        expect(canvas.queryByRole("button", {name: "Audit"})).toBeNull()
        await expect(canvas.getByRole("button", {name: "Export"})).toBeVisible()
        // The range is read in the tenant's log zone unless another is chosen.
        expect(canvas.getByRole<HTMLInputElement>("combobox", {name: "Timezone"}).value).toMatch(
            /Madrid · primary$/
        )

        // The tenant log is read without an election event.
        await waitFor(() => expect(listed().length).toBeGreaterThan(0))
        expect(lastListed()).toMatchObject({
            filter: {default_time_zone: TENANT_LOG_ZONE},
            meta: {logScope: "tenant"},
            sort: {field: "id", order: "DESC"},
        })
        expect(Object.keys(lastListed().filter)).not.toContain("election_event_id")
        expect(graphql.calls).toEqual([])
    },
}

/** Without the setting the tenant log shows in UTC, with my time beside it. */
export const InUtcByDefault: Story = {
    args: {logsTimezone: undefined, myZone: "America/New_York"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const lockout = within(
            (
                await canvas.findByText(
                    "Account locked for 1 minute after 30 failed sign-ins in a row"
                )
            ).closest("tr") as HTMLElement
        )
        expect(lockout.getAllByText(/Feb 2, 2028, 5:43:5\d AM UTC/)).toHaveLength(2)
        expect(lockout.getAllByText(/Feb 2, 2028, 12:43:5\d AM EST · my time/)).toHaveLength(2)
        await waitFor(() =>
            expect(lastListed()).toMatchObject({filter: {default_time_zone: "UTC"}})
        )
    },
}

/** The filters: statement kinds of the tenant log, event type and log type. */
export const FailedSignIns: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Signed in: password and authenticator code")
        await addFilter(canvasElement, "Log Type")
        await userEvent.click(await canvas.findByRole("combobox", {name: "Log Type"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "ERROR"}))
        await waitFor(() =>
            expect(canvas.queryByText("Signed in: password and authenticator code")).toBeNull()
        )
        await expect(canvas.getByText("Sign-in failed: unknown account")).toBeVisible()
        expect(canvas.getAllByRole("row").slice(1)).toHaveLength(4)

        await addFilter(canvasElement, "Statement kind")
        await userEvent.click(await canvas.findByRole("combobox", {name: "Statement kind"}))
        const kinds = within(await within(document.body).findByRole("listbox"))
        // The first option clears the filter.
        expect(
            kinds
                .getAllByRole("option")
                .slice(1)
                .map(({textContent}) => textContent)
        ).toEqual([
            "KeycloakUserEvent",
            "AdminChange",
            "KeycloakAdminEvent",
            "LogsExported",
            "LogWriterStarted",
            "LogWriterStopped",
            "AdminPublicKey",
        ])
        await userEvent.click(kinds.getByRole("option", {name: "KeycloakUserEvent"}))
        await waitFor(() =>
            expect(lastListed().filter).toMatchObject({
                log_type: "ERROR",
                statement_kind: "KeycloakUserEvent",
            })
        )
        // Messaging filters belong to an election event's log.
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        const menu = within(await within(document.body).findByRole("menu"))
        await waitFor(() =>
            expect(menu.getByRole("menuitemcheckbox", {name: "Event Type"})).toBeVisible()
        )
        expect(menu.getByRole("menuitemcheckbox", {name: "Username"})).toBeVisible()
        expect(menu.queryByRole("menuitemcheckbox", {name: "Channel"})).toBeNull()
        expect(menu.queryByRole("menuitemcheckbox", {name: "Status"})).toBeNull()
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(within(document.body).queryByRole("menu")).toBeNull())
    },
}

/** Clicking an entry opens its popup: here a settings change with a redacted secret. */
export const OpenAnEntry: Story = {
    args: {myZone: "America/New_York"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            await canvas.findByText("Settings changed: Languages, Integrations and Voting Channels")
        )
        const dialog = within(
            await within(document.body).findByRole("dialog", {name: "Log entry 5,259"})
        )
        await waitFor(() => expect(dialog.getByText("Old and new values")).toBeVisible())
        expect(dialog.getByText("Changed. Secret values are never logged.")).toBeVisible()
        expect(dialog.getByText("Tenant log")).toBeVisible()
        expect(dialog.getByText(/10:21:37 AM GMT\+1/)).toBeVisible()
        expect(dialog.getByText(/4:21:37 AM EST · my time/)).toBeVisible()
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

/** The export offers the tenant's scopes and runs with tenant-logs-export. */
export const ExportOneAccount: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Signed in: password and authenticator code")
        await userEvent.click(canvas.getByRole("button", {name: "Export"}))
        const dialog = within(
            await within(document.body).findByRole("dialog", {name: "Export logs"})
        )
        await userEvent.click(dialog.getByRole("combobox", {name: "Scope"}))
        const scopes = within(await within(document.body).findByRole("listbox"))
        expect(scopes.getAllByRole("option").map(({textContent}) => textContent)).toEqual([
            "The whole tenant log",
            "One account",
        ])
        await userEvent.click(scopes.getByRole("option", {name: "One account"}))
        await userEvent.type(dialog.getByRole("textbox", {name: /Username/}), "rosa.marin")
        await userEvent.click(dialog.getByRole("radio", {name: "SQL"}))
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls).toEqual([
                {
                    name: "ExportTenantLogs",
                    variables: {
                        format: "SQL",
                        scope: "ACCOUNT",
                        username: "rosa.marin",
                        createdFrom: null,
                        createdTo: null,
                        timeZone: TENANT_LOG_ZONE,
                    },
                    headers: {"x-hasura-role": "tenant-logs-export"},
                },
            ])
        )
    },
}

/** Reading the log doesn't give its exports. */
export const WithoutExportPermission: Story = {
    args: {roles: [IPermissions.TENANT_LOGS_READ]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Signed in: password and authenticator code")
        expect(canvas.queryByRole("button", {name: "Export"})).toBeNull()
        await expect(canvas.getByRole("button", {name: "Columns"})).toBeVisible()
    },
}

/** An event's logs-read doesn't open the tenant log. */
export const WithoutPermission: Story = {
    args: {roles: [IPermissions.LOGS_READ, IPermissions.LOGS_EXPORT]},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("You don't have permission to see this log.")
        ).toBeVisible()
        expect(data.calls).toEqual([])
    },
}

/** On a narrow screen each entry is two lines and opens the same popup. */
export const OnANarrowScreen: Story = {
    args: {width: 420},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(canvasElement.querySelectorAll(".log-compact-entry")).toHaveLength(
                TENANT_LOG_PAGE
            )
        )
        expect(canvas.queryByRole("table")).toBeNull()
        const lockout = canvas
            .getByText("Account locked for 1 minute after 30 failed sign-ins in a row")
            .closest("li") as HTMLElement
        await expect(
            within(lockout).getByText(/^5236 · .* · lena\.ortiz · KeycloakUserEvent · ERROR$/)
        ).toBeVisible()
        await userEvent.click(within(lockout).getByRole("button"))
        const dialog = within(
            await within(document.body).findByRole("dialog", {name: "Log entry 5,236"})
        )
        await waitFor(() =>
            expect(dialog.getByText("USER_DISABLED_BY_TEMPORARY_LOCKOUT")).toBeVisible()
        )
    },
}
