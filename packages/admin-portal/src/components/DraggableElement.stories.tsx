// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import DraggableElement from "./DraggableElement"

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/DraggableElement",
    component: DraggableElement,
    args: {
        id: STORY_IDS.candidate,
        name: "Alice Example",
        index: 2,
        isOver: false,
        onDragStart: fn(),
        onDragOver: fn(),
        onDrop: fn(),
    },
    argTypes: {id: {table: {disable: true}}},
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <DraggableElement {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof DraggableElement>
export default meta
type Story = StoryObj<typeof meta>

/** The draggable wrapper and its visible row. */
function parts(canvasElement: HTMLElement) {
    const handle = canvasElement.querySelector<HTMLElement>("[draggable='true']")
    const row = handle?.firstElementChild
    if (!handle || !row) throw new Error("DraggableElement did not render its row")
    return {handle, row}
}

export const Idle: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Alice Example")).toBeVisible()
        const {row} = parts(canvasElement)
        expect(row).not.toHaveClass("over")
        expect(row).not.toHaveClass("dragging")
    },
}

export const DropTarget: Story = {
    args: {isOver: true},
    play: async ({canvasElement}) => {
        expect(parts(canvasElement).row).toHaveClass("over")
    },
}

export const DragReportsItsIndex: Story = {
    play: async ({canvasElement, args}) => {
        const {handle, row} = parts(canvasElement)
        fireEvent.dragStart(handle)
        expect(args.onDragStart).toHaveBeenCalledWith(
            expect.objectContaining({type: "dragstart"}),
            2
        )
        await waitFor(() => expect(row).toHaveClass("dragging"))
        fireEvent.dragOver(handle)
        expect(args.onDragOver).toHaveBeenCalledWith(expect.objectContaining({type: "dragover"}), 2)
        fireEvent.drop(handle)
        expect(args.onDrop).toHaveBeenCalledWith(expect.objectContaining({type: "drop"}), 2)
        fireEvent.dragEnd(handle)
        await waitFor(() => expect(row).not.toHaveClass("dragging"))
    },
}
