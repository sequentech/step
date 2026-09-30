// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {MonitoringSwitcher} from "./MonitoringSwitcher"
import {listDashboardsResponse} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringSwitcher",
    component: MonitoringSwitcher,
    args: {
        dashboards: listDashboardsResponse().dashboards,
        dashboardId: "overview",
        onChange: fn(),
    },
} satisfies Meta<typeof MonitoringSwitcher>
export default meta
type Story = StoryObj<typeof meta>

export const ChooseDashboard: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const select = canvas.getByRole("combobox", {name: "Dashboard"})
        await expect(select).toHaveTextContent("Monitoring overview")
        await userEvent.click(select)
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "Voted vs pre-enrolled"})
        )
        await expect(args.onChange).toHaveBeenCalledWith("req-0260")
    },
}
