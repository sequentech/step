// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {MonitoringHeader} from "./MonitoringHeader"
import {MONITORING_DEFAULT_REFRESH_MS} from "./types"
import {MONITORING_SNAPSHOT, listDashboardsResponse} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringHeader",
    component: MonitoringHeader,
    args: {
        presetTitle: "COMELEC overseas voting",
        dashboards: listDashboardsResponse().dashboards,
        dashboardId: "overview",
        onSelectDashboard: fn(),
        snapshot: MONITORING_SNAPSHOT,
        timeZone: "Asia/Manila",
        refreshMs: MONITORING_DEFAULT_REFRESH_MS,
        onExport: fn(),
        onRefresh: fn(),
    },
} satisfies Meta<typeof MonitoringHeader>
export default meta
type Story = StoryObj<typeof meta>

export const Viewer: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("COMELEC overseas voting")).toBeVisible()
        await expect(canvas.getByRole("combobox", {name: "Section"})).toHaveTextContent(
            "Monitoring overview"
        )
        // The snapshot's time in the event's zone, not the viewer's.
        await expect(canvas.getByText(/^Updated 8:00 PM\b.* · every 30 s$/)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Check for new figures"}))
        await expect(args.onRefresh).toHaveBeenCalled()
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
    args: {snapshot: null, onExport: undefined, presetTitle: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Not counted yet · every 30 s")).toBeVisible()
        expect(canvas.queryByText("Dashboard preset")).toBeNull()
        await expect(canvas.getByRole("button", {name: "Export"})).toBeDisabled()
    },
}

export const ServerInterval: Story = {
    args: {
        snapshot: null,
        onExport: undefined,
        refreshMs: 120_000,
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Not counted yet · every 120 s")).toBeVisible()
    },
}
