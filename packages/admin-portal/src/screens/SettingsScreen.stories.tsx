// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    ELECTION_TYPE_RESOURCE,
    TENANT_RESOURCE,
    electionTypeRecords,
    settingsTenant,
} from "@/resources/Settings/__stories__/SettingsFixture"
import {IPermissions} from "@/types/keycloak"
import {SettingsScreen} from "./SettingsScreen"

interface Scenario {
    /** The signed-in user's roles. */
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Screens/SettingsScreen",
    component: SettingsScreen,
    args: {roles: [IPermissions.SETTINGS_MENU, IPermissions.TENANT_WRITE]},
    parameters: {
        expectedFailure: {
            reason: "The election types grid labels a MUI 7 span instead of its row checkboxes, and its row actions are unnamed icon buttons under an empty column header.",
            a11y: ["aria-prohibited-attr", "button-name", "empty-table-header", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            [ELECTION_TYPE_RESOURCE]: electionTypeRecords(),
            [TENANT_RESOURCE]: [settingsTenant()],
        })
        // The messaging operations are not in the generated schema yet.
        const messaging = args.roles.includes(IPermissions.MESSAGING_ACCOUNT_READ)
        graphql = graphqlBoundary(
            {
                GetMessagingAccounts: () => ({data: {sequent_backend_messaging_account: []}}),
            },
            {schema: !messaging}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            <SettingsScreen />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tab = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).getByRole("tab", {name: i18n.t(`electionTypeScreen.tabs.${key}`)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("electionTypeScreen.common.settingTitle"))
        ).toBeVisible()
        await expect(tab(canvasElement, "electionTypes")).toHaveAttribute("aria-selected", "true")
        await expect(await canvas.findByRole("row", {name: /Referendum/})).toBeVisible()
        expect(canvas.getAllByRole("tab")).toHaveLength(10)
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getList", ELECTION_TYPE_RESOURCE],
        ])
    },
}

export const OpenTheLocalization: Story = {
    parameters: {
        expectedFailure: {
            reason: "The override rows' edit and delete actions are unnamed icon buttons.",
            a11y: ["button-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("row", {name: /Referendum/})
        await userEvent.click(tab(canvasElement, "localization"))
        await expect(await canvas.findByRole("row", {name: /Welcome, council/})).toBeVisible()
        expect(canvas.queryByRole("row", {name: /Referendum/})).toBeNull()
        await waitFor(() =>
            expect(data.calls.map(({method, args}) => [method, args[0]])).toContainEqual([
                "getOne",
                TENANT_RESOURCE,
            ])
        )
    },
}

export const OpenTheMessagingAccounts: Story = {
    args: {
        roles: [
            IPermissions.SETTINGS_MENU,
            IPermissions.TENANT_WRITE,
            IPermissions.MESSAGING_ACCOUNT_READ,
        ],
    },
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("row", {name: /Referendum/})
        expect(canvas.getAllByRole("tab")).toHaveLength(11)
        await userEvent.click(canvas.getByRole("tab", {name: i18n.t("messagingAccounts.tab")}))
        await expect(await canvas.findByText(i18n.t("messagingAccounts.list.empty"))).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["GetMessagingAccounts"])
    },
}

export const WithoutTenantWritePermission: Story = {
    args: {roles: [IPermissions.SETTINGS_MENU]},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("electionTypeScreen.noPermissions"))
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
        expect(data.calls).toEqual([])
    },
}

export const WithoutTheSettingsMenu: Story = {
    args: {roles: [IPermissions.TENANT_WRITE]},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("electionTypeScreen.noPermissions"))
        ).toBeVisible()
        expect(data.calls).toEqual([])
    },
}
