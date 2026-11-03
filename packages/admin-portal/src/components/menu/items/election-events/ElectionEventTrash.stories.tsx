// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TREE_RESOURCE_NAMES} from "../ElectionEvents"
import {
    TRASHED_ARCHIVED_EVENT_ID,
    TreeStory,
    recordsTreeData,
    treeServices,
    trashRows,
    type TreeServices,
} from "../__stories__/ElectionTreeFixture"
import {TreeMenu} from "./TreeMenu"
import {ElectionEventTrash} from "./ElectionEventTrash"
import {useStoryGlobals} from "../../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    trash: "events" | "empty"
    retentionDays: number | null
    writeFailure: boolean
}

let services: TreeServices

/** The side menu's tabs over the synthetic tree, starting on Trash. */
function TrashMenu() {
    const {permissions} = useStoryGlobals()
    const [archived, setArchived] = useState(false)
    const [trash, setTrash] = useState(true)
    const [, setReloads] = useState(0)
    return (
        <TreeStory services={services} role={permissions}>
            <TreeMenu
                data={recordsTreeData(services, archived)}
                treeResourceNames={TREE_RESOURCE_NAMES}
                isArchivedElectionEvents={archived}
                isTrash={trash}
                onArchiveElectionEventsSelect={(value) => {
                    setTrash(false)
                    setArchived(value === 1)
                }}
                onTrashSelect={() => setTrash(true)}
                reloadTree={() => setReloads((count) => count + 1)}
            />
        </TreeStory>
    )
}

const meta = {
    title: "Admin/Menu/Items/Election events/ElectionEventTrash",
    component: ElectionEventTrash,
    args: {trash: "events", retentionDays: 7, writeFailure: false},
    argTypes: {trash: {control: "inline-radio", options: ["events", "empty"]}},
    beforeEach: async ({args}) => {
        services = treeServices({
            trash: args.trash === "events" ? trashRows() : [],
            retentionDays: args.retentionDays,
            writeFailure: args.writeFailure,
        })
        await services.graphql.ready
    },
    render: () => <TrashMenu />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const body = () => within(document.body)

export const Trash: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "Trash"})).toHaveAttribute(
            "aria-pressed",
            "true"
        )
        await expect(await canvas.findByText("Spring by-election")).toBeVisible()
        await expect(canvas.getByText("Showing 3 of 3")).toBeVisible()
        await expect(canvas.getByText("Was archived")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Restore Ward 4 recall"})).toBeDisabled()
        expect(canvas.getAllByText("Permanent deletion in progress").length).toBeGreaterThan(0)
        expect(canvas.queryByLabelText("Search")).toBeNull()
    },
}

export const RestoreToArchived: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Restore Library levy"}))
        const dialog = await body().findByRole("dialog")
        await expect(within(dialog).getByText('Restore "Library levy"?')).toBeVisible()
        await expect(within(dialog).getByText(/It returns to Archived/)).toBeVisible()
        await userEvent.click(within(dialog).getByRole("button", {name: "Restore"}))
        await waitFor(() => expect(body().getByText('"Library levy" restored')).toBeVisible())
        await waitFor(() => expect(canvas.queryByText("Library levy")).toBeNull())
        expect(services.graphql.calls.find(({name}) => name === "RestoreElectionEvent")).toEqual(
            expect.objectContaining({
                variables: {electionEventId: TRASHED_ARCHIVED_EVENT_ID},
                headers: expect.objectContaining({"x-hasura-role": "election-event-delete"}),
            })
        )
        await userEvent.click(canvas.getByRole("button", {name: "Archived"}))
        await expect(await canvas.findByRole("link", {name: "Library levy"})).toBeVisible()
    },
}

export const RestoreFails: Story = {
    args: {writeFailure: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            await canvas.findByRole("button", {name: "Restore Spring by-election"})
        )
        await userEvent.click(
            within(await body().findByRole("dialog")).getByRole("button", {name: "Restore"})
        )
        await waitFor(() =>
            expect(body().getByText(/The election event could not be restored/)).toBeVisible()
        )
        await expect(canvas.getByText("Spring by-election")).toBeVisible()
    },
}

export const EmptyTrash: Story = {
    args: {trash: "empty"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Trash is empty")).toBeVisible()
        await expect(
            canvas.getByText("Deleted election events stay here for 7 days and can be restored.")
        ).toBeVisible()
    },
}

export const KeyboardTabs: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const active = canvas.getByRole("button", {name: "Active"})
        active.focus()
        await userEvent.keyboard("{Enter}")
        await expect(active).toHaveAttribute("aria-pressed", "true")
        await userEvent.tab()
        await userEvent.tab()
        await expect(canvas.getByRole("button", {name: "Trash"})).toHaveFocus()
        await userEvent.keyboard(" ")
        await expect(await canvas.findByText("Spring by-election")).toBeVisible()
    },
}
