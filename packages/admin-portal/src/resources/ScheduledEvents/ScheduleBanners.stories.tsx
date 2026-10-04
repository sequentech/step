// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {ScheduleBanners} from "./ScheduleBanners"
import {overseasConfiguration} from "@/components/timezones/__fixtures__/configurations"
import {lifecycleSchedule} from "./__stories__/LifecycleScheduleFixture"

const row = lifecycleSchedule(overseasConfiguration())[0]
const meta = {
    title: "Admin/Scheduled events/ScheduleBanners",
    component: ScheduleBanners,
    args: {
        electionEventId: "event",
        outcomes: new Map(),
        filter: null,
        onFilter: fn(),
        unpublishedCount: 1,
        published: true,
        offsetless: 1,
        canApply: true,
        zoneOf: () => "Asia/Manila",
        scheduledEvents: [
            {
                ...row,
                annotations: {
                    schedule_recompute: {
                        previous: "2028-05-08T10:00:00Z",
                        scheduled_date: "2028-05-08T11:00:00Z",
                        timezone: "Asia/Manila",
                    },
                },
            },
        ],
    },
    decorators: [
        (Story: React.ComponentType) => (
            <AdminStoryProvider
                boundary={graphqlBoundary({
                    ApplyScheduleRecompute: () => ({
                        data: {apply_schedule_recompute: {updated: 1}},
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
export const RecomputeAndUnpublishedWarnings: Story = {
    parameters: {widgets: ["RecomputeBanner"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByTestId("schedule-unpublished")).toBeVisible()
        await expect(canvas.getByTestId("schedule-offsetless")).toBeVisible()
        const recompute = canvas.getByTestId("schedule-recompute")
        await expect(recompute).toHaveTextContent(/18:00/)
        await expect(recompute).toHaveTextContent(/19:00/)
        await userEvent.click(within(recompute).getByRole("button"))
    },
}
