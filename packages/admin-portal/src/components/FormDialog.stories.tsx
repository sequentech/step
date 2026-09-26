// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import FormDialog from "./FormDialog"

const meta = {
    title: "Admin/Components/FormDialog",
    component: FormDialog,
    args: {
        open: true,
        title: "Reject application",
        children: <p>Choose the reason for the rejection.</p>,
        onClose: fn(),
    },
    argTypes: {children: {table: {disable: true}}},
} satisfies Meta<typeof FormDialog>
export default meta
type Story = StoryObj<typeof meta>

export const Open: Story = {
    play: async ({args}) => {
        const dialog = within(await within(document.body).findByRole("dialog"))
        await waitFor(() => expect(dialog.getByText("Reject application")).toBeVisible())
        await expect(dialog.getByText("Choose the reason for the rejection.")).toBeVisible()
        expect(args.onClose).not.toHaveBeenCalled()
    },
}

export const CloseButtonClosesTheDialog: Story = {
    play: async ({args}) => {
        const dialog = await within(document.body).findByRole("dialog")
        await userEvent.click(within(dialog).getByRole("button"))
        expect(args.onClose).toHaveBeenCalledTimes(1)
    },
}

export const EscapeClosesTheDialog: Story = {
    play: async ({args}) => {
        await within(document.body).findByRole("dialog")
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(args.onClose).toHaveBeenCalledTimes(1))
    },
}

export const Closed: Story = {
    args: {open: false},
    play: async ({args}) => {
        expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        expect(args.onClose).not.toHaveBeenCalled()
    },
}
