// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import type {ListUserRolesQuery} from "@/gql/graphql"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {storyId} from "@/__stories__/fixtures"
import {
    AUDITOR_ROLE_ID,
    MANAGER_ROLE_ID,
    type RoleRecord,
    roleRecords,
} from "@/resources/Roles/__stories__/RolesFixture"
import {ListUserRoles} from "./EditUserForm"

interface Scenario {
    rolesList: RoleRecord[]
    userRoles?: ListUserRolesQuery
    activeRoleIds?: string[]
    onToggleRole?: (id: string) => void
}

const meta = {
    title: "Admin/User/ListUserRoles",
    component: ListUserRoles,
    args: {
        rolesList: roleRecords(),
        userRoles: {list_user_roles: [{id: AUDITOR_ROLE_ID, name: "auditor"}]},
        onToggleRole: fn(),
    },
    parameters: {
        expectedFailure: {
            reason: "The Active checkbox of each role has no accessible name.",
            a11y: ["label"],
        },
    },
    render: (args) => <ListUserRoles {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const row = (canvasElement: HTMLElement, name: string) => {
    const cell = within(canvasElement).getByRole("gridcell", {name})
    const found = cell.closest<HTMLElement>("[role=row]")
    if (!found) throw new Error(`No row for ${name}`)
    return within(found)
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("columnheader", {name: "Role"})).toBeVisible()
        expect(row(canvasElement, "auditor").getByRole("checkbox")).toBeChecked()
        expect(row(canvasElement, "voter-manager").getByRole("checkbox")).not.toBeChecked()
    },
}

export const PendingSelection: Story = {
    args: {activeRoleIds: [MANAGER_ROLE_ID]},
    play: async ({canvasElement}) => {
        // Unsaved choices win over the roles the user has now.
        expect(row(canvasElement, "voter-manager").getByRole("checkbox")).toBeChecked()
        expect(row(canvasElement, "auditor").getByRole("checkbox")).not.toBeChecked()
    },
}

export const ToggleARole: Story = {
    play: async ({args, canvasElement}) => {
        await userEvent.click(row(canvasElement, "voter-manager").getByRole("checkbox"))
        expect(args.onToggleRole).toHaveBeenCalledTimes(1)
        expect(args.onToggleRole).toHaveBeenCalledWith(MANAGER_ROLE_ID)
        // The parent owns the selection, so the box follows it rather than the click.
        expect(row(canvasElement, "voter-manager").getByRole("checkbox")).not.toBeChecked()
    },
}

export const WithoutRoles: Story = {
    args: {rolesList: [], userRoles: {list_user_roles: []}},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No rows")).toBeVisible()
        expect(within(canvasElement).queryAllByRole("checkbox")).toHaveLength(0)
    },
}

export const ManyRoles: Story = {
    args: {
        rolesList: Array.from({length: 12}, (_, index) => ({
            ...roleRecords()[1],
            id: storyId(3 + Math.floor(index / 10), index),
            name: `role-${String(index + 1).padStart(2, "0")}`,
        })),
        userRoles: {list_user_roles: []},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("1–10 of 12")).toBeVisible()
        await expect(await canvas.findByText("role-01")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Go to next page"}))
        await expect(await canvas.findByText("11–12 of 12")).toBeVisible()
        await expect(await canvas.findByText("role-11")).toBeVisible()
        expect(canvas.queryByText("role-01")).toBeNull()
    },
}
