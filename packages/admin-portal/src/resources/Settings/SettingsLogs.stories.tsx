// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TENANT_LOG_RESOURCE} from "@/queries/ListTenantLog"
import {IPermissions} from "@/types/keycloak"
import {SettingsLogs} from "./SettingsLogs"
import {
    ADMIN,
    DELETED_EVENT_ID,
    LOG_BOARD,
    LOG_PUBLIC_KEY,
    tenantLogRecords,
} from "./__stories__/TenantLogFixture"

interface Scenario {
    /** What reading the log does. */
    reads: ReadState
    /** Whether the log has entries. */
    populated: boolean
    /** The signed-in user's roles. */
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsLogs",
    component: SettingsLogs,
    args: {reads: "records", populated: true, roles: [IPermissions.TENANT_LOGS_READ]},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[TENANT_LOG_RESOURCE]: args.populated ? tenantLogRecords() : []},
            {reads: args.reads}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            <SettingsLogs />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const listed = () => data.calls.find(({method}) => method === "getList")?.args
const DENIED = "Request to delete an election event denied."

export const Populated: Story = {
    parameters: {widgets: ["TenantLogRowTime"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(DENIED)).toBeVisible()
        await expect(canvas.getByText(i18n.t("tenantLogs.subtitle"))).toBeVisible()
        // Newest first: the refused request, the sign-ins and the log's first entry.
        const kinds = canvas
            .getAllByRole("row")
            .slice(1)
            .map((row) => within(row).getAllByRole("cell")[3].textContent)
        expect(kinds).toEqual([
            "TenantAdminAction",
            "KeycloakUserEvent",
            "KeycloakUserEvent",
            "TenantLogOpened",
        ])
        expect(canvas.getAllByText(ADMIN.name)).toHaveLength(2)
        expect(canvas.getByText("LOGIN_ERROR invalid_user_credentials")).toBeVisible()
        expect(canvas.getByText("Opened the tenant log.")).toBeVisible()
        const headers = canvas.getAllByRole("columnheader").map(({textContent}) => textContent)
        expect(headers).toEqual([
            "Id",
            "Statement Timestamp",
            "Username",
            "Statement kind",
            "Event Type",
            "Log Type",
            "Description",
        ])
        expect(listed()).toEqual([
            TENANT_LOG_RESOURCE,
            expect.objectContaining({
                // The range filter is read in the viewer's zone (UTC in stories).
                filter: {default_time_zone: "UTC"},
                sort: {field: "id", order: "DESC"},
            }),
        ])
        // The tenant's log has no export and no audit yet.
        expect(canvas.queryByRole("button", {name: "Export"})).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

/** A request's entry says what was asked for, how it ended, who asked and on what. */
export const ARequestUnderShowMore: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const cell = within((await canvas.findByText(DENIED)).closest("td")!)
        expect(canvas.queryByText("Action: delete-election-event")).toBeNull()
        await userEvent.click(cell.getByRole("button", {name: "Show More"}))
        await expect(cell.getByText("Action: delete-election-event")).toBeVisible()
        await expect(cell.getByText("Denied (HTTP 401)")).toBeVisible()
        await expect(cell.getByText(`Requested by ${ADMIN.name} (${ADMIN.id})`)).toBeVisible()
        await expect(cell.getByText(`election_event_id: ${DELETED_EVENT_ID}`)).toBeVisible()
        await userEvent.click(cell.getByRole("button", {name: "Show Less"}))
        expect(canvas.queryByText("Action: delete-election-event")).toBeNull()
    },
}

/** The first entry names the log and the key its entries are signed with. */
export const TheFirstEntryUnderShowMore: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const cell = within((await canvas.findByText("Opened the tenant log.")).closest("td")!)
        await userEvent.click(cell.getByRole("button", {name: "Show More"}))
        await expect(cell.getByText(`Log: ${LOG_BOARD}`)).toBeVisible()
        await expect(cell.getByText(`Public key of the log: ${LOG_PUBLIC_KEY}`)).toBeVisible()
        // A sign-in has nothing more to say than its description.
        const signIn = within(canvas.getByText("LOGIN").closest("td")!)
        expect(signIn.queryByRole("button")).toBeNull()
    },
}

export const FilterByStatementKind: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText(DENIED)
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "Statement kind"})
        )
        await userEvent.click(await canvas.findByRole("combobox", {name: "Statement kind"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {
                name: i18n.t("tenantLogs.kind.TenantAdminAction"),
            })
        )
        await waitFor(() => expect(canvas.getAllByRole("row")).toHaveLength(2))
        await expect(canvas.getByText(DENIED)).toBeVisible()
        expect(canvas.queryByText("Opened the tenant log.")).toBeNull()
    },
}

export const NoEntries: Story = {
    args: {populated: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("tenantLogs.title"))).toBeVisible()
        await waitFor(() => expect(listed()?.[0]).toBe(TENANT_LOG_RESOURCE))
        expect(canvas.queryByText(DENIED)).toBeNull()
    },
}

export const LogsUnavailable: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("tenantLogs.title"))).toBeVisible()
        await waitFor(() => expect(listed()?.[0]).toBe(TENANT_LOG_RESOURCE))
        expect(canvas.queryByText(DENIED)).toBeNull()
    },
}

export const WithoutLogPermission: Story = {
    args: {roles: [IPermissions.TENANT_WRITE, IPermissions.LOGS_READ]},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("tenantLogs.noPermissions"))
        ).toBeVisible()
        expect(data.calls).toEqual([])
    },
}
