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
import {PgAuditTable} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {Logs} from "./Logs"
import {hasuraAuditRecords, keycloakAuditRecords} from "./__stories__/LogsFixture"

interface Scenario {
    /** What reading the audit rows does. */
    reads: ReadState
    /** The signed-in user's roles. */
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Screens/Logs",
    component: Logs,
    args: {reads: "records", roles: [IPermissions.LOGS_READ]},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                [PgAuditTable.PgauditHasura]: hasuraAuditRecords(),
                [PgAuditTable.PgauditKeycloak]: keycloakAuditRecords(),
            },
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
            <Logs />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const readTables = () =>
    data.calls.filter(({method}) => method === "getList").map(({args}) => args[0])

const tab = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).getByRole("tab", {name: i18n.t(`logsScreen.${key}.title`)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("logsScreen.title"))).toBeVisible()
        await expect(tab(canvasElement, "main")).toHaveAttribute("aria-selected", "true")
        await expect(
            await canvas.findByRole("row", {name: /UPDATE sequent_backend.election/})
        ).toBeVisible()
        expect(readTables()).toEqual([PgAuditTable.PgauditHasura])
    },
}

export const IamLogs: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("row", {name: /UPDATE sequent_backend.election/})
        await userEvent.click(tab(canvasElement, "iam"))
        await expect(
            await canvas.findByRole("row", {name: /INSERT INTO user_entity/})
        ).toBeVisible()
        expect(canvas.queryByRole("row", {name: /UPDATE sequent_backend.election/})).toBeNull()
        await waitFor(() =>
            expect(readTables()).toEqual([PgAuditTable.PgauditHasura, PgAuditTable.PgauditKeycloak])
        )
    },
}

export const WithoutLogsPermission: Story = {
    args: {roles: []},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("logsScreen.noPermissions"))
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
        expect(data.calls).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(readTables()).toEqual([PgAuditTable.PgauditHasura]))
        expect(within(canvasElement).queryByRole("row", {name: /UPDATE/})).toBeNull()
    },
}
