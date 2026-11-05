// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {RecordContextProvider, type Identifier} from "react-admin"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaRecords} from "@/__stories__/fixtures"
import {ActionsColumn} from "./ActionButons"

interface Scenario {
    /** Whether the column renders inside a row's record context. */
    withRecord: boolean
    /** What the delete action's `showAction` answers. */
    canDelete: boolean
    onEdit: (id: Identifier) => void
    onDelete: (id: Identifier) => void
    /** The row's own click handler, which the actions must not trigger. */
    onRowClick: () => void
}

let boundary: ReturnType<typeof graphqlBoundary>

function Fixture({withRecord, canDelete, onEdit, onDelete, onRowClick}: Scenario) {
    const column = (
        <ActionsColumn
            actions={[
                {icon: <EditIcon />, action: onEdit},
                {icon: <DeleteIcon />, action: onDelete, showAction: () => canDelete},
            ]}
        />
    )
    return (
        <AdminStoryProvider boundary={boundary}>
            <table>
                <tbody>
                    <tr onClick={onRowClick}>
                        <td>North district</td>
                        <td>
                            {withRecord ? (
                                <RecordContextProvider value={areaRecords()[0]}>
                                    {column}
                                </RecordContextProvider>
                            ) : (
                                column
                            )}
                        </td>
                    </tr>
                </tbody>
            </table>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Components/ActionsColumn",
    component: ActionsColumn,
    args: {withRecord: true, canDelete: true, onEdit: fn(), onDelete: fn(), onRowClick: fn()},
    argTypes: {
        onEdit: {table: {disable: true}},
        onDelete: {table: {disable: true}},
        onRowClick: {table: {disable: true}},
    },
    parameters: {
        expectedFailure: {
            reason: "The action icon buttons have no accessible name; the action's label is not rendered.",
            a11y: ["button-name"],
        },
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const actionButton = (canvasElement: HTMLElement, icon: string) =>
    within(canvasElement).queryByTestId(icon)?.closest("button") ?? null

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const row = within(canvasElement).getByRole("row", {name: /North district/})
        expect(within(row).getAllByRole("button")).toHaveLength(2)
        await userEvent.click(actionButton(canvasElement, "EditIcon")!)
        expect(args.onEdit).toHaveBeenCalledTimes(1)
        expect(args.onEdit).toHaveBeenCalledWith(STORY_IDS.area)
        expect(args.onDelete).not.toHaveBeenCalled()
        // The column sits inside a clickable row: an action must not also open the row.
        expect(args.onRowClick).not.toHaveBeenCalled()
    },
}

export const DeleteRunsForTheRowRecord: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(actionButton(canvasElement, "DeleteIcon")!)
        expect(args.onDelete).toHaveBeenCalledTimes(1)
        expect(args.onDelete).toHaveBeenCalledWith(STORY_IDS.area)
        expect(args.onEdit).not.toHaveBeenCalled()
        expect(args.onRowClick).not.toHaveBeenCalled()
    },
}

export const HiddenActionIsNotRendered: Story = {
    args: {canDelete: false},
    play: async ({canvasElement}) => {
        const row = within(canvasElement).getByRole("row", {name: /North district/})
        expect(within(row).getAllByRole("button")).toHaveLength(1)
        await expect(actionButton(canvasElement, "EditIcon")).toBeVisible()
        expect(actionButton(canvasElement, "DeleteIcon")).toBeNull()
    },
}

export const WithoutRecordDoesNothing: Story = {
    args: {withRecord: false},
    play: async ({canvasElement, args}) => {
        // Conditional actions need a record to decide; unconditional ones still render.
        expect(actionButton(canvasElement, "DeleteIcon")).toBeNull()
        await userEvent.click(actionButton(canvasElement, "EditIcon")!)
        expect(args.onEdit).not.toHaveBeenCalled()
        expect(args.onRowClick).not.toHaveBeenCalled()
    },
}
