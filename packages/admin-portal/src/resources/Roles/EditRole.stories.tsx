// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {ListContextProvider, useList} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EditRole} from "./EditRole"
import {AUDITOR_ROLE_ID, permissionRecords, roleRecords} from "./__stories__/RolesFixture"

interface Scenario {
    /** Whether the role service rejects permission changes. */
    failure: boolean
    /** Whether the list of roles is still loading. */
    loading: boolean
    close: () => void
}

let boundary: ReturnType<typeof graphqlBoundary>

/** The roles list the editor reads its role from, as ListRoles provides it. */
function RoleList({loading, close}: Omit<Scenario, "failure">) {
    const list = useList({
        data: loading ? undefined : roleRecords(),
        isPending: loading,
        isLoading: loading,
    })
    return (
        <ListContextProvider value={list}>
            <EditRole id={AUDITOR_ROLE_ID} close={close} permissions={permissionRecords()} />
        </ListContextProvider>
    )
}

const changed = {id: AUDITOR_ROLE_ID}

const meta = {
    title: "Admin/Roles/EditRole",
    component: EditRole,
    args: {failure: false, loading: false, close: fn()},
    parameters: {
        expectedFailure: {
            reason: "The permission checkboxes of the grid have no accessible name.",
            a11y: ["label"],
        },
    },
    beforeEach: async ({args}) => {
        const reply = (field: string) => () =>
            args.failure
                ? {errors: [new GraphQLError("Synthetic role service failure")]}
                : {data: {[field]: changed}}
        boundary = graphqlBoundary(
            {
                SetRolePermission: reply("set_role_permission"),
                DeleteRolePermission: reply("delete_role_permission"),
            },
            {schema: true}
        )
        await boundary.ready
    },
    render: ({failure: _failure, ...args}) => (
        <AdminStoryProvider boundary={boundary}>
            <RoleList {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const permissionName = (name: string) => i18n.t(`usersAndRolesScreen.permissions.${name}`)

async function permissionCheckbox(canvasElement: HTMLElement, name: string) {
    const row = await within(canvasElement).findByRole("row", {
        name: new RegExp(permissionName(name)),
    })
    return within(row).getByRole("checkbox")
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("textbox", {name: "Name"})).toHaveValue("auditor")
        expect(canvas.getByRole("textbox", {name: "Name"})).toHaveAttribute("readonly")
        await expect(await permissionCheckbox(canvasElement, "role-read")).toBeChecked()
        await expect(await permissionCheckbox(canvasElement, "user-write")).not.toBeChecked()
        // Keycloak's own default roles are not admin permissions.
        expect(canvas.queryByText("offline_access")).not.toBeInTheDocument()
        expect(boundary.calls).toEqual([])
    },
}

export const GrantAPermission: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await permissionCheckbox(canvasElement, "role-write"))
        await waitFor(() =>
            expect(boundary.calls).toEqual([
                {
                    name: "SetRolePermission",
                    variables: {
                        tenantId: TENANT_ID,
                        roleId: AUDITOR_ROLE_ID,
                        permissionName: "role-write",
                    },
                    headers: {},
                },
            ])
        )
        const message = await within(document.body).findByText(
            i18n.t("usersAndRolesScreen.roles.notifications.permissionEditSuccess")
        )
        await waitFor(() => expect(message).toBeVisible())
    },
}

export const RevokeAPermission: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await permissionCheckbox(canvasElement, "role-read"))
        await waitFor(() =>
            expect(boundary.calls).toEqual([
                {
                    name: "DeleteRolePermission",
                    variables: {
                        tenantId: TENANT_ID,
                        roleId: AUDITOR_ROLE_ID,
                        permissionName: "role-read",
                    },
                    headers: {},
                },
            ])
        )
    },
}

export const PermissionChangeFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement}) => {
        await userEvent.click(await permissionCheckbox(canvasElement, "role-write"))
        const message = await within(document.body).findByText(
            i18n.t("usersAndRolesScreen.roles.notifications.permissionEditError")
        )
        await waitFor(() => expect(message).toBeVisible())
        expect(boundary.calls.map(({name}) => name)).toEqual(["SetRolePermission"])
        expect(
            within(document.body).queryByText(
                i18n.t("usersAndRolesScreen.roles.notifications.permissionEditSuccess")
            )
        ).not.toBeInTheDocument()
    },
}

export const WaitingForTheRoles: Story = {
    args: {loading: true},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("textbox")).not.toBeInTheDocument()
        expect(within(canvasElement).queryByRole("grid")).not.toBeInTheDocument()
    },
}
