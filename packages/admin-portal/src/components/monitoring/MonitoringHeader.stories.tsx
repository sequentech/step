// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {MonitoringHeader} from "./MonitoringHeader"
import {
    MONITORING_SNAPSHOT,
    listDashboardsResponse,
    overviewDashboard,
} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringHeader",
    component: MonitoringHeader,
    args: {
        title: "Monitoring overview",
        dashboards: listDashboardsResponse().dashboards,
        dashboardId: "overview",
        onSelectDashboard: fn(),
        widgetCount: 5,
        requirements: overviewDashboard.requirements ?? [],
        snapshot: MONITORING_SNAPSHOT,
        timeZone: "Asia/Manila",
        onExport: fn(),
    },
} satisfies Meta<typeof MonitoringHeader>
export default meta
type Story = StoryObj<typeof meta>

export const Viewer: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        // The snapshot's time in the event's zone, not the viewer's.
        await expect(
            canvas.getByText(
                "5 widgets · SW-F-0247, SW-F-0279, SW-F-0365 · Updated Jan 15, 2026, 8:00 PM · every 30 s"
            )
        ).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Edit dashboard"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Export"}))
        await expect(args.onExport).toHaveBeenCalled()
    },
}

export const Configurer: Story = {
    args: {onEditDashboard: fn()},
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Edit dashboard"}))
        await expect(args.onEditDashboard).toHaveBeenCalled()
    },
}

export const NotCountedYet: Story = {
    args: {snapshot: null, onExport: undefined, widgetCount: 1, requirements: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("1 widget · Not counted yet · every 30 s")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Export"})).toBeDisabled()
    },
}
