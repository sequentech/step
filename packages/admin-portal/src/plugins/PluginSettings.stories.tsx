// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EStoryPermissions} from "../../../ui-essentials/.storybook/globals"
import {PluginProvider} from "./PluginProvider"
import {PluginSettings} from "./PluginSettings"
import {PluginState} from "./contracts"
import {projectsServices} from "./projects/__stories__/projectsFixture"

interface Scenario {
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Plugins/PluginSettings",
    component: PluginSettings,
    args: {roles: [IPermissions.ADMIN_USER, IPermissions.TENANT_WRITE]},
    beforeEach: async () => {
        graphql = graphqlBoundary(projectsServices({state: PluginState.Disabled}).handlers, {
            schema: true,
        })
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            role={EStoryPermissions.ADMIN}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            <PluginProvider>
                <PluginSettings />
            </PluginProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** Turning Projects on is confirmed first and explains what it grants. */
export const TurnOn: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Projects")).toBeVisible()
        await expect(canvas.getByText("Off")).toBeVisible()
        await expect(canvas.getByText(/^Plugin API 1 · Compatible$/)).toBeVisible()
        await expect(canvas.getByText(/· granted to the admin group when turned on$/)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Turn on"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await waitFor(() => expect(dialog.getByText(/granted to the admin group/)).toBeVisible())
        await userEvent.click(dialog.getByRole("button", {name: "Confirm"}))
        await expect(await canvas.findByText("On")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toContain("SetTenantPluginState")
    },
}

/** Without tenant-write the plugin is listed but cannot be turned on. */
export const WithoutTenantWrite: Story = {
    args: {roles: [IPermissions.ADMIN_USER]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Projects")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Turn on"})).toBeDisabled()
        await expect(
            canvas.getByText("Changing plugins needs the tenant-write permission.")
        ).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).not.toContain("SetTenantPluginState")
    },
}
