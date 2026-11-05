// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
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

/** Each section's dashboards under its heading; a heading is not a choice. */
export const GroupedBySection: Story = {
    parameters: {widgets: ["SectionHeading"]},
    args: {
        dashboards: [
            {id: "overview", title: "Overview", requirements: [], widget_count: 1},
            {
                id: "req-0259",
                title: "Voted vs registered",
                section: "Voter turnout",
                requirements: ["SW-F-0259"],
                widget_count: 4,
            },
            {
                id: "req-0260",
                title: "Voted vs pre-enrolled",
                section: "Voter turnout",
                requirements: ["SW-F-0260"],
                widget_count: 4,
            },
        ],
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("combobox", {name: "Dashboard"}))
        const list = within(await within(document.body).findByRole("listbox"))
        expect(list.getAllByRole("option").map((option) => option.textContent)).toEqual([
            "Overview",
            "Voted vs registered",
            "Voted vs pre-enrolled",
        ])
        await waitFor(() => expect(list.getByText("Voter turnout")).toBeVisible())
        await userEvent.click(list.getByRole("option", {name: "Voted vs pre-enrolled"}))
        await expect(args.onChange).toHaveBeenCalledWith("req-0260")
    },
}
