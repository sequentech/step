// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CreateRole} from "./CreateRole"
import {permissionRecords} from "./__stories__/RolesFixture"

interface Scenario {
    /** Whether the realm's permissions are passed to the form. */
    withPermissions: boolean
    /** Whether the role service rejects the new role. */
    failure: boolean
    close: () => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Roles/CreateRole",
    component: CreateRole,
    args: {withPermissions: true, failure: false, close: fn()},
    parameters: {
        expectedFailure: {
            reason: "The permission checkboxes of the grid have no accessible name.",
            a11y: ["label"],
        },
    },
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary(
            {
                CreateRole: ({variables}) =>
                    args.failure
                        ? {errors: [new GraphQLError("Synthetic role service failure")]}
                        : {data: {create_role: {id: storyId(2, 3), ...variables.role}}},
            },
            {schema: true}
        )
        await boundary.ready
    },
    render: ({withPermissions, close}) => (
        <AdminStoryProvider boundary={boundary}>
            <CreateRole close={close} permissions={withPermissions ? permissionRecords() : []} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const permissionName = (name: string) => i18n.t(`usersAndRolesScreen.permissions.${name}`)

async function togglePermission(canvasElement: HTMLElement, name: string) {
    const row = await within(canvasElement).findByRole("row", {
        name: new RegExp(permissionName(name)),
    })
    await userEvent.click(within(row).getByRole("checkbox"))
}

async function nameTheRole(canvasElement: HTMLElement, name: string) {
    await userEvent.type(
        await within(canvasElement).findByRole("textbox", {
            name: i18n.t("usersAndRolesScreen.roles.fields.name"),
        }),
        name
    )
}

const save = (canvasElement: HTMLElement) =>
    userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("usersAndRolesScreen.roles.create.subtitle"))
        ).toBeVisible()
        const rows = await canvas.findAllByRole("row", {name: /Read|Edit/})
        expect(rows.map((row) => within(row).getAllByRole("gridcell")[0].textContent)).toEqual([
            permissionName("role-read"),
            permissionName("role-write"),
            permissionName("user-read"),
            permissionName("user-write"),
        ])
        for (const row of rows) expect(within(row).getByRole("checkbox")).not.toBeChecked()
        expect(canvas.queryByText(/offline_access/)).not.toBeInTheDocument()
    },
}

export const CreateARoleWithPermissions: Story = {
    play: async ({canvasElement, args}) => {
        await nameTheRole(canvasElement, "observer")
        await togglePermission(canvasElement, "role-read")
        await togglePermission(canvasElement, "user-read")
        // A second click removes a permission again.
        await togglePermission(canvasElement, "role-read")
        await save(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(boundary.calls).toEqual([
            {
                name: "CreateRole",
                variables: {
                    tenantId: TENANT_ID,
                    role: {
                        name: "observer",
                        permissions: ["user-read"],
                        access: {
                            manage: true,
                            manageMembers: true,
                            manageMembership: true,
                            view: true,
                            viewMembers: true,
                        },
                    },
                },
                headers: {},
            },
        ])
        const message = await within(document.body).findByText(
            i18n.t("usersAndRolesScreen.roles.errors.createSuccess")
        )
        await waitFor(() => expect(message).toBeVisible())
    },
}

export const CreateFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await nameTheRole(canvasElement, "observer")
        await save(canvasElement)
        const message = await within(document.body).findByText(
            i18n.t("usersAndRolesScreen.roles.errors.createError")
        )
        await waitFor(() => expect(message).toBeVisible())
        expect(boundary.calls.map(({name}) => name)).toEqual(["CreateRole"])
        expect(args.close).toHaveBeenCalledTimes(1)
    },
}

export const WithoutRealmPermissions: Story = {
    args: {withPermissions: false},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No rows")).toBeVisible()
        expect(canvas.queryByRole("checkbox")).not.toBeInTheDocument()
        expect(boundary.calls).toEqual([])
    },
}
