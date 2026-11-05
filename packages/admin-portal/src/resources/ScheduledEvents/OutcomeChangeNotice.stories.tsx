// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {OutcomeChangeNotice} from "./OutcomeChangeNotice"
import {runsAuthorized, refusedEdited} from "@/components/timezones/__fixtures__/explanations"

const meta = {
    title: "Admin/Scheduled events/OutcomeChangeNotice",
    component: OutcomeChangeNotice,
    args: {
        electionEventId: "event",
        zone: "Asia/Manila",
        change: {
            id: "open",
            event_processor: "START_VOTING_PERIOD",
            cron_config: {
                scheduled_date: "2028-05-08T11:00:00Z",
                local: "2028-05-08T19:00",
                timezone: "Asia/Manila",
            },
            event_payload: {election_id: null},
        },
    },
    decorators: [
        (Story: React.ComponentType) => (
            <AdminStoryProvider
                boundary={graphqlBoundary({
                    PreviewScheduledOutcomeChange: () => ({
                        data: {
                            preview_scheduled_outcome_change: {
                                changes: [
                                    {
                                        scheduled_event_id: "open",
                                        election_id: "post-a",
                                        before: runsAuthorized(),
                                        after: refusedEdited(),
                                    },
                                ],
                            },
                        },
                    }),
                })}
            >
                <Story />
            </AdminStoryProvider>
        ),
    ],
}
export default meta
type Story = StoryObj<typeof meta>
export const SignedEditRefused: Story = {
    parameters: {widgets: ["OutcomeChanges"]},
    play: async ({canvasElement}) => {
        const notice = await within(canvasElement).findByTestId("outcome-change-notice")
        await expect(notice).toHaveTextContent(/refused/i)
        await expect(within(notice).getByRole("button", {name: /why/i})).toBeVisible()
    },
}
