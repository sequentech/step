// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, candidateRecords, storyId} from "@/__stories__/fixtures"
import CustomOrderInput from "./CustomOrderInput"

const [alice, bob] = candidateRecords()
const carol = {
    ...bob,
    id: storyId(6, 3),
    presentation: {i18n: {en: {name: "Carol Example", alias: "Carol"}}, sort_order: 2},
}

interface Scenario {
    /** Whether the contest has candidates to order. */
    empty: boolean
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Custom order/CustomOrderInput",
    component: CustomOrderInput,
    args: {empty: false, onSubmit: fn()},
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({empty, onSubmit}) => (
        <AdminStoryProvider boundary={graphql}>
            <SimpleForm
                record={{candidatesOrder: empty ? [] : [alice, bob, carol]}}
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton />
                    </Toolbar>
                }
            >
                <CustomOrderInput source="candidatesOrder" />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The draggable rows, in their displayed order. */
const rows = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll<HTMLElement>("[draggable='true']"))
const order = (canvasElement: HTMLElement) => rows(canvasElement).map((row) => row.textContent)

/** Drags the row at `from` onto the row at `to`, as the browser's drag events do. */
function drag(canvasElement: HTMLElement, from: number, to: number) {
    const source = rows(canvasElement)[from]
    const target = rows(canvasElement)[to]
    fireEvent.dragStart(source)
    fireEvent.dragOver(target)
    fireEvent.drop(target)
    // The browser ends the drag on the dragged row, wherever the drop has moved it.
    fireEvent.dragEnd(source)
}

const save = (canvasElement: HTMLElement) =>
    userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await waitFor(() => expect(order(canvasElement)).toEqual(["Alice", "Bob", "Carol"]))
        // Nothing has moved, so there is nothing to save.
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
    },
}

export const Empty: Story = {
    args: {empty: true},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeVisible()
        expect(rows(canvasElement)).toEqual([])
    },
}

export const DragToReorder: Story = {
    play: async ({canvasElement, args}) => {
        await waitFor(() => expect(order(canvasElement)).toHaveLength(3))
        drag(canvasElement, 2, 0)
        await waitFor(() => expect(order(canvasElement)).toEqual(["Carol", "Alice", "Bob"]))
        await save(canvasElement)
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        const saved = args.onSubmit.mock.calls[0][0].candidatesOrder as {id: string}[]
        expect(saved.map(({id}) => id)).toEqual([carol.id, STORY_IDS.candidate, bob.id])
    },
}

export const DropOnItselfKeepsTheOrder: Story = {
    play: async ({canvasElement, args}) => {
        await waitFor(() => expect(order(canvasElement)).toHaveLength(3))
        drag(canvasElement, 1, 1)
        expect(order(canvasElement)).toEqual(["Alice", "Bob", "Carol"])
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
        expect(args.onSubmit).not.toHaveBeenCalled()
    },
}
