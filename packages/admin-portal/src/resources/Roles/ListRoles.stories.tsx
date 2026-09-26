// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ListRoles} from "./ListRoles"
import {AUDITOR_ROLE_ID, permissionRecords, roleRecords} from "./__stories__/RolesFixture"
import {
    EStoryPermissions,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** What reading the roles does. */
    reads: ReadState
    /** Whether the realm has no roles. */
    empty: boolean
    /** Whether the role service rejects changes. */
    failure: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture() {
    const {permissions} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <ListRoles />
        </AdminStoryProvider>
    )
}

const listDefects = {
    reason: "React-admin row selection labels a MUI 7 span instead of its checkbox; the row actions are unnamed icon buttons under an empty column header.",
    a11y: ["aria-prohibited-attr", "button-name", "empty-table-header", "label"],
}

// The list behind an open drawer is hidden from assistive technology.
const drawerDefects = {
    expectedFailure: {
        reason: "The drawer dialog has no accessible name and the permission checkboxes of its grid have none.",
        a11y: ["aria-dialog-name", "label"],
    },
}

const meta = {
    title: "Admin/Roles/ListRoles",
    component: ListRoles,
    args: {reads: "records", empty: false, failure: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {expectedFailure: listDefects},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {role: args.empty ? [] : roleRecords(), permission: permissionRecords()},
            {reads: {role: args.reads}}
        )
        const reply = (field: string) => () =>
            args.failure
                ? {errors: [new GraphQLError("Synthetic role service failure")]}
                : {data: {[field]: {id: AUDITOR_ROLE_ID}}}
        graphql = graphqlBoundary(
            {
                DeleteRole: reply("delete_role"),
                SetRolePermission: reply("set_role_permission"),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const roleRow = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

/** The row action whose Material icon has this test ID, e.g. "DeleteIcon". */
function rowAction(row: HTMLElement, icon: string) {
    const button = within(row).getByTestId(icon).closest("button")
    if (!button) throw new Error(`The row has no ${icon} action`)
    return button
}

const listReads = (resource: string) =>
    data.calls.filter(({method, args}) => method === "getList" && args[0] === resource)

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        await roleRow(canvasElement, "voter-manager")
        await expect(await roleRow(canvasElement, AUDITOR_ROLE_ID)).toHaveTextContent("auditor")
        expect(listReads("role")[0].args[1]).toMatchObject({filter: {tenant_id: TENANT_ID}})
        expect(listReads("permission")[0].args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID},
        })
        const {permissions} = readStoryGlobals(globals)
        const canCreate = [EStoryPermissions.ADMIN, EStoryPermissions.ADMIN_LIGHT].includes(
            permissions
        )
        expect(!!canvas.queryByRole("button", {name: i18n.t("common.label.add")})).toBe(canCreate)
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(listReads("role")).toHaveLength(1))
        expect(within(canvasElement).queryByRole("row", {name: /auditor/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /auditor/})).toBeNull()
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {
        expectedFailure: {
            reason: "React-admin's empty list message is light grey below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("No Roles yet.")).toBeVisible()
        expect(within(canvasElement).queryByRole("row")).toBeNull()
    },
}

export const WithoutRoleCreation: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LOCKDOWN},
    play: async ({canvasElement}) => {
        await roleRow(canvasElement, "auditor")
        expect(
            within(canvasElement).queryByRole("button", {name: i18n.t("common.label.add")})
        ).toBeNull()
    },
}

export const AddOpensTheRoleForm: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: drawerDefects,
    play: async ({canvasElement}) => {
        await roleRow(canvasElement, "auditor")
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.add")})
        )
        const drawer = within(document.body)
        await expect(
            await drawer.findByText(i18n.t("usersAndRolesScreen.roles.create.subtitle"))
        ).toBeVisible()
        // The form offers the realm permissions the list loaded.
        await expect(
            await drawer.findByRole("row", {
                name: new RegExp(i18n.t("usersAndRolesScreen.permissions.user-write")),
            })
        ).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

export const EditARolesPermissions: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: drawerDefects,
    play: async ({canvasElement}) => {
        await userEvent.click(rowAction(await roleRow(canvasElement, "auditor"), "EditIcon"))
        const drawer = within(document.body)
        await expect(await drawer.findByRole("textbox", {name: "Name"})).toHaveValue("auditor")
        const row = await drawer.findByRole("row", {
            name: new RegExp(i18n.t("usersAndRolesScreen.permissions.role-write")),
        })
        await userEvent.click(within(row).getByRole("checkbox"))
        await waitFor(() =>
            expect(graphql.calls).toEqual([
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
        // The list reads the roles again to show the changed role.
        await waitFor(() => expect(listReads("role")).toHaveLength(2))
    },
}

async function confirmDelete(canvasElement: HTMLElement) {
    await userEvent.click(rowAction(await roleRow(canvasElement, "auditor"), "DeleteIcon"))
    const dialog = within(await within(document.body).findByRole("dialog"))
    await expect(dialog.getByText(i18n.t("usersAndRolesScreen.roles.delete.body"))).toBeVisible()
    expect(graphql.calls).toEqual([])
    await userEvent.click(dialog.getByRole("button", {name: i18n.t("common.label.delete")}))
}

export const DeleteARoleAfterConfirmation: Story = {
    play: async ({canvasElement}) => {
        await confirmDelete(canvasElement)
        await waitFor(() =>
            expect(graphql.calls).toEqual([
                {
                    name: "DeleteRole",
                    variables: {tenantId: TENANT_ID, roleId: AUDITOR_ROLE_ID},
                    headers: {},
                },
            ])
        )
        const message = await within(document.body).findByText(
            i18n.t("usersAndRolesScreen.roles.notifications.deleteSuccess")
        )
        await waitFor(() => expect(message).toBeVisible())
        await waitFor(() => expect(listReads("role")).toHaveLength(2))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const CancelKeepsTheRole: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(rowAction(await roleRow(canvasElement, "auditor"), "DeleteIcon"))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: i18n.t("common.label.cancel")}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        expect(graphql.calls).toEqual([])
        await expect(await roleRow(canvasElement, "auditor")).toBeVisible()
    },
}
