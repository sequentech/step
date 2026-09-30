// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {resolveSelectors} from "./lib/selectors"
import {MonitoringWidgetSelectors} from "./MonitoringWidgetSelectors"
import {turnoutByGroup, votingActivity} from "./__stories__/MonitoringFixture"
import {EDynamicOptions, type MonitoringWidget} from "./types"

const hourlyWithDay: MonitoringWidget = {
    ...votingActivity,
    selectors: {
        ...votingActivity.selectors,
        day: {
            label: "Day",
            options_from: EDynamicOptions.EVENT_DAYS,
            when: {selector: "grain", in: ["hour"]},
        },
    },
}

const meta = {
    title: "Admin/Monitoring/MonitoringWidgetSelectors",
    component: MonitoringWidgetSelectors,
    args: {
        widgetId: "turnout-by-group",
        selectors: resolveSelectors(turnoutByGroup, {measure: "voted_pre"}, {}, {}),
        onChange: fn(),
    },
} satisfies Meta<typeof MonitoringWidgetSelectors>
export default meta
type Story = StoryObj<typeof meta>

export const Dropdowns: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        // The dashboard's value for the widget, not the selector's own default.
        await expect(canvas.getByRole("combobox", {name: "Show"})).toHaveTextContent(
            "Voted of pre-enrolled"
        )
        await expect(canvas.getByRole("combobox", {name: "Breakdown"})).toHaveTextContent("Age")
        await userEvent.click(canvas.getByRole("combobox", {name: "Breakdown"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Sex"}))
        await expect(args.onChange).toHaveBeenCalledWith("breakdown", "sex")
    },
}

export const Toggle: Story = {
    args: {widgetId: "voting-activity", selectors: resolveSelectors(hourlyWithDay, {}, {}, {})},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const group = canvas.getByRole("group", {name: "Grain"})
        await expect(within(group).getByRole("button", {name: "Daily"})).toHaveAttribute(
            "aria-pressed",
            "true"
        )
        // The Day picker only appears for an hourly series.
        expect(canvas.queryByRole("combobox", {name: "Day"})).toBeNull()
        await userEvent.click(within(group).getByRole("button", {name: "Hourly"}))
        await expect(args.onChange).toHaveBeenCalledWith("grain", "hour")
    },
}

export const DayPickerWhenHourly: Story = {
    args: {
        widgetId: "voting-activity",
        selectors: resolveSelectors(
            hourlyWithDay,
            {},
            {grain: "hour"},
            {event_days: ["2026-05-11", "2026-05-12"]}
        ),
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("combobox", {name: "Day"})).toHaveTextContent(
            "2026-05-12"
        )
    },
}
