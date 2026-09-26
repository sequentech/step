// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, type Identifier, type RaRecord} from "react-admin"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaRecords} from "@/__stories__/fixtures"
import {ListActionsMenu} from "./ListActionsMenu"

interface Scenario {
    /** Whether the menu renders inside a row's record context. */
    withRecord: boolean
    /** What the delete action's `showAction` answers. */
    canDelete: boolean
    onEdit: (id: Identifier) => void
    onDelete: (id: Identifier) => void
    /** Keeps the chosen record, as lists do before opening an edit drawer. */
    onSaveRecord: (record: RaRecord) => void
}

let boundary: ReturnType<typeof graphqlBoundary>

function Fixture({withRecord, canDelete, onEdit, onDelete, onSaveRecord}: Scenario) {
    const menu = (
        <ListActionsMenu
            actions={[
                {
                    icon: <EditIcon />,
                    label: "Edit",
                    action: onEdit,
                    saveRecordAction: onSaveRecord,
                },
                {
                    icon: <DeleteIcon />,
                    label: "Delete",
                    action: onDelete,
                    showAction: () => canDelete,
                },
            ]}
        />
    )
    return (
        <AdminStoryProvider boundary={boundary}>
            {withRecord ? (
                <RecordContextProvider value={areaRecords()[0]}>{menu}</RecordContextProvider>
            ) : (
                menu
            )}
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Components/ListActionsMenu",
    component: ListActionsMenu,
    args: {withRecord: true, canDelete: true, onEdit: fn(), onDelete: fn(), onSaveRecord: fn()},
    argTypes: {
        onEdit: {table: {disable: true}},
        onDelete: {table: {disable: true}},
        onSaveRecord: {table: {disable: true}},
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function openMenu(canvasElement: HTMLElement) {
    const button = within(canvasElement).getByRole("button", {
        name: i18n.t("common.label.actions"),
    })
    await userEvent.click(button)
    const menu = await within(document.body).findByRole("menu")
    await waitFor(() => expect(menu).toBeVisible())
    expect(button).toHaveAttribute("aria-expanded", "true")
    return within(menu)
}

const menuClosed = () =>
    waitFor(() => expect(within(document.body).queryByRole("menu")).not.toBeInTheDocument())

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const menu = await openMenu(canvasElement)
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
            "Edit",
            "Delete",
        ])
        await userEvent.click(menu.getByRole("menuitem", {name: "Edit"}))
        expect(args.onEdit).toHaveBeenCalledTimes(1)
        expect(args.onEdit).toHaveBeenCalledWith(STORY_IDS.area)
        expect(args.onSaveRecord).toHaveBeenCalledTimes(1)
        expect(args.onSaveRecord).toHaveBeenCalledWith(areaRecords()[0])
        expect(args.onDelete).not.toHaveBeenCalled()
        await menuClosed()
    },
}

export const DeleteRunsForTheRowRecord: Story = {
    play: async ({canvasElement, args}) => {
        const menu = await openMenu(canvasElement)
        await userEvent.click(menu.getByRole("menuitem", {name: "Delete"}))
        expect(args.onDelete).toHaveBeenCalledTimes(1)
        expect(args.onDelete).toHaveBeenCalledWith(STORY_IDS.area)
        expect(args.onEdit).not.toHaveBeenCalled()
        expect(args.onSaveRecord).not.toHaveBeenCalled()
        await menuClosed()
    },
}

export const HiddenActionIsNotListed: Story = {
    args: {canDelete: false},
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Edit"])
        await userEvent.keyboard("{Escape}")
        await menuClosed()
    },
}

export const WithoutRecordKeepsTheMenuOpen: Story = {
    args: {withRecord: false},
    play: async ({canvasElement, args}) => {
        const menu = await openMenu(canvasElement)
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Edit"])
        await userEvent.click(menu.getByRole("menuitem", {name: "Edit"}))
        expect(args.onEdit).not.toHaveBeenCalled()
        expect(args.onSaveRecord).not.toHaveBeenCalled()
        await expect(within(document.body).getByRole("menu")).toBeVisible()
        await userEvent.keyboard("{Escape}")
        await menuClosed()
    },
}
