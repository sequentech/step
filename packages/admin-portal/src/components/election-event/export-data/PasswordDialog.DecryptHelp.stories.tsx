// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {reportDecryptionCommand} from "@/resources/Reports/ReportPasswordDialog"
import {storyClipboard} from "./__stories__/ClipboardFixture"
import {DecryptHelp} from "./PasswordDialog"

type Props = React.ComponentProps<typeof DecryptHelp> & {
    /** Whether the browser lets the page write to the clipboard. */
    clipboard: "copied" | "refused"
}

let boundary: ReturnType<typeof graphqlBoundary>
let clipboard: ReturnType<typeof storyClipboard>

const meta = {
    title: "Admin/Election event/Export data/DecryptHelp",
    component: DecryptHelp,
    args: {decryptionCommand: reportDecryptionCommand, clipboard: "copied"},
    argTypes: {clipboard: {control: "inline-radio", options: ["copied", "refused"]}},
    parameters: {
        expectedFailure: {
            reason: "The read-only command field has no label.",
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
            <DecryptHelp {...props} />
        </AdminStoryProvider>
    ),
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

const notification = async (message: string) => {
    const snackbar = await within(document.body).findByText(message)
    await waitFor(() => expect(snackbar).toBeVisible())
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/The path to the encrypted file/)).toBeVisible()
        const command = canvas.getByDisplayValue(reportDecryptionCommand)
        await expect(command).toHaveAttribute("readonly")
    },
}

export const CopyCommand: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Copy Password"}))
        expect(clipboard.writeText).toHaveBeenCalledWith(reportDecryptionCommand)
        await notification("Password copied to clipboard")
    },
}

export const CopyRefused: Story = {
    args: {clipboard: "refused"},
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Copy Password"}))
        expect(clipboard.writeText).toHaveBeenCalledWith(reportDecryptionCommand)
        await notification("Error copying password")
    },
}
