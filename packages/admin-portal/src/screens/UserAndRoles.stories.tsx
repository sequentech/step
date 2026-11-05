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
import {permissionRecords, roleRecords} from "@/resources/Roles/__stories__/RolesFixture"
import {IPermissions} from "@/types/keycloak"
import {UserAndRoles} from "./UserAndRoles"
import {userProfileConfiguration, userRecords} from "./__stories__/UserAndRolesFixture"

interface Scenario {
    /** The signed-in user's roles. */
    roles: string[]
}

const ROLE_LIST_DEFECTS = {
    expectedFailure: {
        reason: "The roles grid labels a MUI 7 span instead of its row checkboxes, and its row actions are unnamed icon buttons under an empty column header.",
        a11y: ["aria-prohibited-attr", "button-name", "empty-table-header", "label"],
    },
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Screens/UserAndRoles",
    component: UserAndRoles,
    args: {
        roles: [IPermissions.USERS_MENU, IPermissions.USER_READ, IPermissions.ROLE_READ],
    },
    beforeEach: async () => {
        data = resourceBoundary({
            user: userRecords(),
            role: roleRecords(),
            permission: permissionRecords(),
        })
        graphql = graphqlBoundary(
            {GetUserProfileConfiguration: userProfileConfiguration},
            {schema: true}
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
            <UserAndRoles />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tabNames = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .queryAllByRole("tab")
        .map(({textContent}) => textContent)

const listed = () => data.calls.filter(({method}) => method === "getList").map(({args}) => args[0])

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("alice")).toBeVisible()
        expect(tabNames(canvasElement)).toEqual([
            i18n.t("usersAndRolesScreen.users.title"),
            i18n.t("usersAndRolesScreen.roles.title"),
        ])
        expect(listed()).toContain("user")
    },
}

export const BrowseTheRoles: Story = {
    parameters: ROLE_LIST_DEFECTS,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        await userEvent.click(
            canvas.getByRole("tab", {name: i18n.t("usersAndRolesScreen.roles.title")})
        )
        await expect(await canvas.findByRole("row", {name: /voter-manager/})).toBeVisible()
        expect(canvas.queryByText("alice")).toBeNull()
        await waitFor(() => expect(listed()).toContain("role"))
    },
}

export const OnlyRoles: Story = {
    parameters: ROLE_LIST_DEFECTS,
    args: {roles: [IPermissions.USERS_MENU, IPermissions.ROLE_READ]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("row", {name: /voter-manager/})).toBeVisible()
        expect(tabNames(canvasElement)).toEqual([i18n.t("usersAndRolesScreen.roles.title")])
        expect(listed()).not.toContain("user")
    },
}

export const WithoutTheUsersMenu: Story = {
    args: {roles: [IPermissions.USER_READ, IPermissions.ROLE_READ]},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("usersAndRolesScreen.noPermissions"))
        ).toBeVisible()
        expect(tabNames(canvasElement)).toEqual([])
        expect(data.calls).toEqual([])
    },
}
