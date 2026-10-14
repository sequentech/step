// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {RowOutcome} from "./RowOutcome"
import {runsAuthorized, refusedEdited} from "@/components/timezones/__fixtures__/explanations"

const meta = {
    title: "Admin/Scheduled events/RowOutcome",
    component: RowOutcome,
    args: {
        zone: "Asia/Manila",
        outcomes: [
            {scheduled_event_id: "open", election_id: "post-a", explanation: runsAuthorized()},
            {scheduled_event_id: "open", election_id: "post-b", explanation: refusedEdited()},
        ],
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
export const MixedPostOutcomes: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getAllByRole("button", {name: /why/i})).toHaveLength(2)
        await expect(canvas.getAllByText(/1 of 2/)).toHaveLength(2)
    },
}
