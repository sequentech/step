// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {Tabs} from "./Tabs"

const Dashboard: React.FC = () => <p>Turnout: 42%</p>
const Voters: React.FC<{area?: string}> = ({area}) => <p>Voters of {area ?? "every area"}</p>
const Publish: React.FC = () => <p>Ballot publication</p>

const publishAction = fn()

/** A screen that keeps the selected tab in its state, as the tab parent does. */
function SelectionHost({
    selectedTab,
    onSelectedTabChange,
    ...props
}: React.ComponentProps<typeof Tabs>) {
    const [selected, setSelected] = useState(selectedTab)
    return (
        <Tabs
            {...props}
            selectedTab={selected}
            onSelectedTabChange={(index) => {
                onSelectedTabChange?.(index)
                setSelected(index)
            }}
        />
    )
}

const meta = {
    title: "Admin/Components/Tabs",
    component: Tabs,
    args: {
        elements: [
            {label: "Dashboard", component: Dashboard},
            {label: "Voters", component: Voters, props: {area: "North district"}},
            {label: "Publish", component: Publish, action: publishAction},
        ],
        onSelectedTabChange: fn(),
    },
    argTypes: {elements: {table: {disable: true}}},
    beforeEach: () => {
        publishAction.mockClear()
    },
} satisfies Meta<typeof Tabs>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("tab", {name: "Dashboard"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(canvas.getByText("Turnout: 42%")).toBeVisible()
        expect(canvas.queryByText(/Voters of/)).not.toBeInTheDocument()
    },
}

export const SwitchingTabs: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("tab", {name: "Voters"}))
        // A tab's own props reach its component.
        await expect(await canvas.findByText("Voters of North district")).toBeVisible()
        expect(args.onSelectedTabChange).toHaveBeenLastCalledWith(1)
        await userEvent.click(canvas.getByRole("tab", {name: "Publish"}))
        await expect(await canvas.findByText("Ballot publication")).toBeVisible()
        expect(publishAction).toHaveBeenCalledWith(2)
        expect(args.onSelectedTabChange).toHaveBeenLastCalledWith(2)
    },
}

export const ControlledSelection: Story = {
    args: {selectedTab: 2},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("tab", {name: "Publish"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(canvas.getByText("Ballot publication")).toBeVisible()
    },
}

export const RemovedTabFallsBackToTheFirst: Story = {
    args: {selectedTab: 5},
    render: (args) => <SelectionHost {...args} />,
    play: async ({canvasElement, args}) => {
        // A permission change can remove the selected tab; the parent is asked to select the first.
        await waitFor(() => expect(args.onSelectedTabChange).toHaveBeenCalledWith(0))
        await expect(await within(canvasElement).findByText("Turnout: 42%")).toBeVisible()
    },
}

export const Empty: Story = {
    args: {elements: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(canvas.queryAllByRole("tab")).toEqual([])
        expect(canvas.getByRole("tablist")).toBeEmptyDOMElement()
    },
}
