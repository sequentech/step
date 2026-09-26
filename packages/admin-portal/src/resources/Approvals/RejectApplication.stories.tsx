// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {RejectApplicationButton} from "./RejectApplication"

const meta = {
    title: "Admin/Approvals/RejectApplicationButton",
    component: RejectApplicationButton,
    args: {label: "Reject Application", onClick: fn()},
} satisfies Meta<typeof RejectApplicationButton>
export default meta
type Story = StoryObj<typeof meta>

export const OpensTheRejection: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: "Reject Application"})
        )
        expect(args.onClick).toHaveBeenCalledWith(true)
    },
}
