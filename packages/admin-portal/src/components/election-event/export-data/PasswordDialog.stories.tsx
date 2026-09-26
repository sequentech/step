// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyClipboard} from "./__stories__/ClipboardFixture"
import {PasswordDialog} from "./PasswordDialog"

type Props = React.ComponentProps<typeof PasswordDialog> & {
    /** Whether the browser lets the page write to the clipboard. */
    clipboard: "copied" | "refused"
}

const PASSWORD = "Vq8-synthetic-Kx2"

let boundary: ReturnType<typeof graphqlBoundary>
let clipboard: ReturnType<typeof storyClipboard>

const meta = {
    title: "Admin/Election event/Export data/PasswordDialog",
    component: PasswordDialog,
    args: {password: PASSWORD, onClose: fn(), clipboard: "copied"},
    argTypes: {clipboard: {control: "inline-radio", options: ["copied", "refused"]}},
    parameters: {
        expectedFailure: {
            reason: "The read-only password field has no label.",
            a11y: ["label"],
        },
    },
    beforeEach: ({args}) => {
        boundary = graphqlBoundary({})
        clipboard = storyClipboard(args.clipboard)
        return clipboard.restore
    },
    render: ({clipboard: _clipboard, ...props}) => (
        <AdminStoryProvider boundary={boundary}>
            <PasswordDialog {...props} />
        </AdminStoryProvider>
    ),
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

const notification = async (message: string) => {
    const snackbar = await within(document.body).findByText(message)
    await waitFor(() => expect(snackbar).toBeVisible())
}

const dialog = async () => {
    const element = await within(document.body).findByRole("dialog", {name: "Password"})
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

export const Populated: Story = {
    play: async ({args}) => {
        const password = await dialog()
        await expect(password.getByText("Password to decrypt the file:")).toBeVisible()
        const field = password.getByDisplayValue(PASSWORD)
        await expect(field).toHaveAttribute("readonly")
        expect(args.onClose).not.toHaveBeenCalled()
    },
}

export const CopyPassword: Story = {
    play: async () => {
        const password = await dialog()
        await userEvent.click(password.getByRole("button", {name: "Copy Password"}))
        expect(clipboard.writeText).toHaveBeenCalledWith(PASSWORD)
        await notification("Password copied to clipboard")
    },
}

export const CopyRefused: Story = {
    args: {clipboard: "refused"},
    play: async () => {
        const password = await dialog()
        await userEvent.click(password.getByRole("button", {name: "Copy Password"}))
        expect(clipboard.writeText).toHaveBeenCalledWith(PASSWORD)
        await notification("Error copying password")
        expect(within(document.body).queryByText("Password copied to clipboard")).toBeNull()
    },
}

export const CloseWithOk: Story = {
    play: async ({args}) => {
        const password = await dialog()
        await userEvent.click(password.getByRole("button", {name: "Ok"}))
        await waitFor(() => expect(args.onClose).toHaveBeenCalledTimes(1))
        expect(clipboard.writeText).not.toHaveBeenCalled()
    },
}
