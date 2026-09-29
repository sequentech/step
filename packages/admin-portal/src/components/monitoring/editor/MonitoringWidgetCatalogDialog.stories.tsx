// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {MonitoringWidgetCatalogDialog} from "./MonitoringWidgetCatalogDialog"

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringWidgetCatalogDialog",
    component: MonitoringWidgetCatalogDialog,
    args: {
        open: true,
        entries: [
            {
                id: "turnout-summary",
                title: "Voter turnout",
                source: "voter_turnout",
                requirements: ["SW-F-0259"],
            },
            {
                id: "turnout-by-group",
                title: "Turnout by group",
                source: "voter_turnout",
                requirements: ["SW-F-0259", "SW-F-0260"],
            },
            {
                id: "attack-log",
                title: "Attack detections",
                source: "attack_detections",
                requirements: ["SW-F-0301"],
            },
        ],
        onDashboard: ["turnout-summary"],
        onAdd: fn(),
        onClose: fn(),
    },
} satisfies Meta<typeof MonitoringWidgetCatalogDialog>
export default meta
type Story = StoryObj<typeof meta>

const dialog = async (canvasElement: HTMLElement) => {
    const found = await within(canvasElement.ownerDocument.body).findByRole("dialog", {
        name: "Add widget",
    })
    await waitFor(() => expect(found).toBeVisible())
    return within(found)
}

export const GroupedBySource: Story = {
    play: async ({canvasElement, args}) => {
        const view = await dialog(canvasElement)
        const turnout = view.getByRole("list", {name: "Voter turnout"})
        expect(within(turnout).getAllByRole("button", {name: /^Add /})).toHaveLength(2)
        await expect(within(turnout).getByText("On this dashboard")).toBeVisible()
        await expect(view.getByRole("list", {name: "Attack detections"})).toBeVisible()

        await userEvent.type(
            view.getByRole("searchbox", {name: "Search widgets, data sources or requirements"}),
            "SW-F-0260"
        )
        expect(view.queryByRole("list", {name: "Attack detections"})).toBeNull()
        await userEvent.click(view.getByRole("button", {name: "Add Turnout by group"}))
        expect(args.onAdd).toHaveBeenCalledWith("turnout-by-group")
    },
}

export const NoMatch: Story = {
    play: async ({canvasElement}) => {
        const view = await dialog(canvasElement)
        await userEvent.type(view.getByRole("searchbox"), "SW-F-9999")
        await expect(view.getByText("No widget matches.")).toBeVisible()
    },
}

export const Loading: Story = {
    args: {entries: undefined},
    play: async ({canvasElement}) => {
        const view = await dialog(canvasElement)
        await expect(view.getByRole("progressbar", {name: "Add widget"})).toBeVisible()
    },
}
