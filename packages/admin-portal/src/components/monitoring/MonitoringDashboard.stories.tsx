// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EMonitoringCapability, type MonitoringRenderWidgetVariables} from "./types"
import {MonitoringProvider} from "./MonitoringProvider"
import {MonitoringDashboard} from "./MonitoringDashboard"
import {
    MONITORING_SNAPSHOT,
    listDashboardsResponse,
    monitoringHandlers,
    refusal,
    turnoutByGroup,
    type MonitoringHandlerOptions,
} from "./__stories__/MonitoringFixture"

interface Scenario {
    configure: EMonitoringCapability
    onEditDashboard: (dashboardId: string) => void
    options?: MonitoringHandlerOptions
    failing?: boolean
    /** Harvest's refusal of get-dashboard, as `extensions.code`. */
    refusal?: string
}

let graphql: ReturnType<typeof graphqlBoundary>

function Fixture({configure, onEditDashboard}: Scenario) {
    return (
        <AdminStoryProvider boundary={graphql}>
            <MonitoringProvider storageKey="monitoring:dashboard-story" actions={{onEditDashboard}}>
                <MonitoringDashboard
                    electionEventId={STORY_IDS.event}
                    dashboards={listDashboardsResponse().dashboards}
                    configure={configure}
                />
            </MonitoringProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Monitoring/MonitoringDashboard",
    component: MonitoringDashboard,
    args: {configure: EMonitoringCapability.DENIED, onEditDashboard: fn()},
    beforeEach: async ({args}) => {
        window.sessionStorage.clear()
        const handlers = monitoringHandlers(args.options)
        graphql = graphqlBoundary(
            args.failing || args.refusal
                ? {
                      ...handlers,
                      MonitoringGetDashboard: () => {
                          if (!args.refusal) throw new Error("Synthetic monitoring outage")
                          return refusal(args.refusal)
                      },
                  }
                : handlers,
            {schema: true}
        )
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const renders = () =>
    graphql.calls
        .filter(({name}) => name === "MonitoringRenderWidget")
        .map(({variables}) => variables as unknown as MonitoringRenderWidgetVariables)

export const Overview: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "5 widgets · SW-F-0247, SW-F-0279, SW-F-0365 · Updated Jan 15, 2026, 8:00 PM · every 30 s"
            )
        ).toBeVisible()
        await canvas.findByTitle("Poll status chart")
        // Widget titles sit under the dashboard's.
        await expect(
            canvas.getByRole("heading", {level: 2, name: "Monitoring overview"})
        ).toBeVisible()
        await expect(canvas.getByRole("heading", {level: 3, name: "Poll status"})).toBeVisible()
        await expect(
            canvas.getByText("Not connected · no attack detection feed is connected")
        ).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Edit dashboard"})).toBeNull()
        // Four charts drawn from one snapshot; the unconnected source is not asked for.
        await waitFor(() => expect(renders()).toHaveLength(4))
        expect(new Set(renders().map(({snapshotRevision}) => snapshotRevision))).toEqual(
            new Set([MONITORING_SNAPSHOT.revision])
        )
    },
}

export const ScopeAppliesToWidgets: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Poll status chart")
        await userEvent.click(canvas.getByRole("combobox", {name: "Region"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "North"}))
        await waitFor(() =>
            expect(renders().filter(({scope}) => scope.region === "north")).toHaveLength(4)
        )
    },
}

export const SwitchDashboard: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Poll status chart")
        await userEvent.click(canvas.getByRole("combobox", {name: "Dashboard"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "Voted vs pre-enrolled"})
        )
        await expect(await canvas.findByText(/^1 widget · SW-F-0260, SW-F-0372/)).toBeVisible()
        expect(
            graphql.calls.filter(({name}) => name === "MonitoringGetDashboard").at(-1)?.variables
        ).toEqual(expect.objectContaining({dashboardId: "req-0260"}))
    },
}

export const RestrictedViewer: Story = {
    args: {options: {restricted: true}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("combobox", {name: "Post"})).toHaveTextContent(
            "All authorized Posts"
        )
    },
}

export const Configurer: Story = {
    args: {configure: EMonitoringCapability.GRANTED},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Edit dashboard"}))
        await expect(args.onEditDashboard).toHaveBeenCalledWith("overview")
    },
}

export const InvalidWidget: Story = {
    args: {
        options: {
            widgets: {
                "turnout-summary": {
                    id: "turnout-summary",
                    title: 5,
                    source: "voter_turnout",
                    chart: {},
                },
                [turnoutByGroup.id]: turnoutByGroup,
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByTitle("Turnout by group chart")).toBeVisible()
        await expect(canvas.getAllByText("This widget cannot be shown").length).toBeGreaterThan(0)
    },
}

export const DashboardFailed: Story = {
    args: {failing: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText("The monitoring dashboard could not be loaded.")
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Retry"})).toBeVisible()
    },
}

export const DashboardNotFound: Story = {
    args: {refusal: "MONITORING_NOT_FOUND"},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                /could not be loaded\. This dashboard or widget is no longer configured\./
            )
        ).toBeVisible()
    },
}

export const ExportDashboardWithEachWidgetsPicks: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Turnout by group chart")
        await userEvent.click(canvas.getByRole("combobox", {name: "Breakdown"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Sex"}))
        await userEvent.click(canvas.getByRole("button", {name: "Export"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls.some(({name}) => name === "MonitoringExport")).toBe(true)
        )
        const exported = graphql.calls.find(({name}) => name === "MonitoringExport")!
        expect(exported.variables).toEqual(
            expect.objectContaining({
                widgetId: null,
                selectorValues: {},
                widgetSelectorValues: {
                    "turnout-summary": {},
                    "turnout-by-group": {breakdown: "sex", measure: "voted_pre"},
                    "voting-activity": {grain: "day"},
                    "poll-status": {},
                    "attack-detections": {},
                },
            })
        )
    },
}
