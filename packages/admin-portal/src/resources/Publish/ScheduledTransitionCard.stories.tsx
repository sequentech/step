// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {ETransition, ScheduledTransitionCard} from "./ScheduledTransitionCard"
import {EScheduledOutcomeKind} from "@sequentech/ui-core"
import {refusedEdited} from "@/components/timezones/__fixtures__/explanations"

const meta = {
    title: "Admin/Publish/ScheduledTransitionCard",
    component: ScheduledTransitionCard,
    args: {
        transition: ETransition.CLOSED,
        time: "08 May 2028, 19:00 GMT+8",
        zone: "Asia/Manila",
        post: {
            action: "END_VOTING_PERIOD",
            scheduled_event_id: "close",
            outcome: EScheduledOutcomeKind.RUNS,
            authorized_by: {
                request_id: "approval",
                code: "SCHEDULE-2",
                signers: ["Trustee Alice", "Trustee Bob"],
            },
        },
    },
    decorators: [
        (Story: React.ComponentType) => (
            <AdminStoryProvider boundary={graphqlBoundary({})}>
                <Story />
            </AdminStoryProvider>
        ),
    ],
}
export default meta
type Story = StoryObj<typeof meta>
export const AuthorizedByConfiguration: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {level: 3})).toHaveTextContent(/SCHEDULE-2/)
        await expect(canvas.getByRole("list")).toHaveTextContent("Trustee Alice")
    },
}
export const RefusedEdit: Story = {
    args: {
        post: {
            action: "END_VOTING_PERIOD",
            scheduled_event_id: "close",
            outcome: EScheduledOutcomeKind.REFUSED,
            explanation: refusedEdited(),
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {level: 3})).toHaveTextContent(/refused/i)
        await expect(canvas.getByRole("button", {name: /why/i})).toBeVisible()
    },
}
