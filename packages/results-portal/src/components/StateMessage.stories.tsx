// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {StateMessage} from "./StateMessage"

const meta = {
    title: "components/StateMessage",
    component: StateMessage,
    args: {
        title: "Results are unavailable",
        message: "Try loading the published results again.",
    },
} satisfies Meta<typeof StateMessage>
export default meta
type Story = StoryObj<typeof meta>

export const Retry: Story = {
    args: {actionLabel: "Retry", onAction: fn()},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {level: 1})).toHaveTextContent(
            "Results are unavailable"
        )
        await userEvent.click(canvas.getByRole("button", {name: "Retry"}))
        await expect(args.onAction).toHaveBeenCalledTimes(1)
    },
}

export const WithoutAction: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).queryByRole("button")).not.toBeInTheDocument()
    },
}
