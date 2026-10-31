// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {MonitoringFooter} from "./MonitoringFooter"
import {MONITORING_SNAPSHOT} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringFooter",
    component: MonitoringFooter,
    args: {
        scopeLabel: "All regions · Dubai PCG · All countries",
        snapshot: MONITORING_SNAPSHOT,
        timeZone: "Asia/Manila",
        requirements: ["SW-F-0259", "SW-F-0371"],
    },
} satisfies Meta<typeof MonitoringFooter>
export default meta
type Story = StoryObj<typeof meta>

/** What the figures are of and when they were counted, in the event's zone, with the records. */
export const Counted: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(
                /^All regions · Dubai PCG · All countries · Data through Jan 15, 2026, 8:00 PM \(PhST\)$/
            )
        ).toBeVisible()
        await expect(canvas.getByText("SW-F-0259, SW-F-0371")).toBeVisible()
    },
}

/** Before the first pass, and for a dashboard that names no requirement. */
export const NotCountedYet: Story = {
    args: {snapshot: null, requirements: [], scopeLabel: "All authorized Posts"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("All authorized Posts · Not counted yet")).toBeVisible()
        expect(canvas.queryByText(/SW-F-/)).toBeNull()
    },
}
