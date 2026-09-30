// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {MonitoringDiscardDialog} from "./MonitoringDiscardDialog"

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringDiscardDialog",
    component: MonitoringDiscardDialog,
    args: {
        open: true,
        body: "Your changes to this theme have not been saved.",
        onKeepEditing: fn(),
        onDiscard: fn(),
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <MonitoringDiscardDialog {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MonitoringDiscardDialog>
export default meta
type Story = StoryObj<typeof meta>

/** The dialog once its fade-in is over, so visibility checks mean something. */
const dialog = async (canvasElement: HTMLElement) => {
    const found = await within(canvasElement.ownerDocument.body).findByRole("dialog")
    await waitFor(() => expect(found).toBeVisible())
    return within(found)
}

export const KeepEditing: Story = {
    play: async ({canvasElement, args}) => {
        const view = await dialog(canvasElement)
        await expect(view.getByText(args.body)).toBeVisible()
        await userEvent.click(view.getByRole("button", {name: "Keep editing"}))
        expect(args.onKeepEditing).toHaveBeenCalledTimes(1)
        expect(args.onDiscard).not.toHaveBeenCalled()
    },
}

export const Discard: Story = {
    play: async ({canvasElement, args}) => {
        const view = await dialog(canvasElement)
        await userEvent.click(view.getByRole("button", {name: "Discard"}))
        expect(args.onDiscard).toHaveBeenCalledTimes(1)
    },
}
