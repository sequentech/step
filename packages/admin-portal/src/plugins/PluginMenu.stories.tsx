// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {MenuList} from "@mui/material"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EStoryPermissions} from "../../../ui-essentials/.storybook/globals"
import {PluginProvider} from "./PluginProvider"
import {PluginMenu} from "./PluginRoutes"
import {PluginState} from "./contracts"
import {projectsServices} from "./projects/__stories__/projectsFixture"

interface Scenario {
    roles: string[]
    state: PluginState
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Plugins/PluginMenu",
    component: PluginMenu,
    args: {
        roles: [IPermissions.ADMIN_USER, IPermissions.PROJECTS_READ],
        state: PluginState.Enabled,
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(projectsServices({state: args.state}).handlers, {schema: true})
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
                <MenuList aria-label="Side menu">
                    <PluginMenu />
                </MenuList>
            </PluginProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** A turned-on plugin's item links to its route for users who may read it. */
export const Enabled: Story = {
    play: async ({canvasElement}) => {
        const item = await within(canvasElement).findByRole("menuitem", {name: "Projects"})
        await expect(item).toHaveAttribute("href", "/plugins/projects")
    },
}

/** Without the plugin's read permission there is no item. */
export const WithoutReadPermission: Story = {
    args: {roles: [IPermissions.ADMIN_USER]},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(graphql.calls.map(({name}) => name)).toContain("TenantPlugins"))
        expect(within(canvasElement).queryByRole("menuitem")).toBeNull()
    },
}

/** A plugin turned off for the tenant has no item. */
export const TurnedOff: Story = {
    args: {state: PluginState.Disabled},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(graphql.calls.map(({name}) => name)).toContain("TenantPlugins"))
        expect(within(canvasElement).queryByRole("menuitem")).toBeNull()
    },
}
