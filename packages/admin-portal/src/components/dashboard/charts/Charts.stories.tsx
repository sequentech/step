// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {Button} from "@mui/material"
import CardChart from "./Charts"

const refresh = fn()

const meta = {
    title: "Admin/Dashboard/Charts/CardChart",
    component: CardChart,
    args: {title: "Votes over time", children: <p>Synthetic chart body</p>},
    argTypes: {children: {table: {disable: true}}, actions: {table: {disable: true}}},
    beforeEach: () => {
        refresh.mockClear()
    },
} satisfies Meta<typeof CardChart>
export default meta
type Story = StoryObj<typeof meta>

export const Static: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Synthetic chart body")).toBeVisible()
        // Without `collapsible`, the header neither toggles nor shows the expand button.
        await userEvent.click(canvas.getByText("Votes over time"))
        await expect(canvas.getByText("Synthetic chart body")).toBeVisible()
        expect(canvas.queryByRole("button")).not.toBeInTheDocument()
    },
}

const unnamedExpandButton = {
    expectedFailure: {
        reason: "The expand button of a collapsible card is an icon button without a name.",
        a11y: ["button-name"],
    },
}

export const Collapsible: Story = {
    args: {collapsible: true},
    parameters: unnamedExpandButton,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Synthetic chart body")).toBeVisible()
        await userEvent.click(canvas.getByText("Votes over time"))
        await waitFor(() =>
            expect(canvas.queryByText("Synthetic chart body")).not.toBeInTheDocument()
        )
        await userEvent.click(canvas.getByRole("button"))
        await expect(await canvas.findByText("Synthetic chart body")).toBeVisible()
    },
}

export const ActionsDoNotCollapse: Story = {
    args: {
        collapsible: true,
        actions: (
            <Button size="small" onClick={refresh}>
                Refresh
            </Button>
        ),
    },
    parameters: unnamedExpandButton,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Refresh"}))
        expect(refresh).toHaveBeenCalledTimes(1)
        await expect(canvas.getByText("Synthetic chart body")).toBeVisible()
    },
}
