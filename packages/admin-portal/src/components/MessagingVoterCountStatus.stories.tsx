// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {ERecountOutcome} from "@/queries/GetMessagingVoterCounts"
import {MessagingVoterCountStatus} from "./MessagingVoterCountStatus"

/** The status line: the canvas holds other status regions (notifications). */
const statusLine = (canvasElement: HTMLElement) =>
    within(canvasElement).getByText(/counted|recount/i, {selector: "span"})

const meta = {
    title: "Admin/Components/MessagingVoterCountStatus",
    component: MessagingVoterCountStatus,
    args: {
        status: {
            computed_at: "2026-10-09T08:00:00Z",
            pending: false,
            outcome: ERecountOutcome.COUNTED,
        },
        onRecount: fn(),
    },
} satisfies Meta<typeof MessagingVoterCountStatus>
export default meta
type Story = StoryObj<typeof meta>

/** When the voters were counted, and Recount. */
export const Counted: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(statusLine(canvasElement)).toHaveTextContent(/^Counted at /)
        await userEvent.click(canvas.getByRole("button", {name: "Recount"}))
        expect(args.onRecount).toHaveBeenCalled()
    },
}

/** Never counted yet: says so, and offers Recount. */
export const NotCountedYet: Story = {
    args: {status: {computed_at: null, pending: false, outcome: null}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(statusLine(canvasElement)).toHaveTextContent("Not counted yet.")
        expect(canvas.getByRole("button", {name: "Recount"})).toBeEnabled()
    },
}

/** A recount on its way after one that took too long: the counts before it stay. */
export const TookTooLongAndRecounting: Story = {
    args: {
        status: {
            computed_at: "2026-10-09T08:00:00Z",
            pending: true,
            outcome: ERecountOutcome.TIMED_OUT,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const status = statusLine(canvasElement)
        await expect(status).toHaveTextContent("The last recount took too long and was stopped.")
        expect(status).toHaveTextContent("The counts shown are from the count before it.")
        expect(status).toHaveTextContent("A recount is on its way.")
        expect(canvas.getByRole("button", {name: "Recount"})).toBeDisabled()
    },
}

/** A failed recount, where a recount can't be asked for. */
export const FailedWithoutRecount: Story = {
    args: {
        status: {computed_at: null, pending: false, outcome: ERecountOutcome.FAILED},
        onRecount: undefined,
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(statusLine(canvasElement)).toHaveTextContent(
            "Not counted yet. The last recount failed."
        )
        expect(canvas.queryByRole("button", {name: "Recount"})).toBeNull()
    },
}
