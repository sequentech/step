// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {Box} from "@mui/material"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {
    EMonitoringCapability,
    EWidgetRenderState,
    type MonitoringRenderWidgetResponse,
    type MonitoringRenderWidgetVariables,
} from "./types"
import {MonitoringProvider} from "./MonitoringProvider"
import {MonitoringWidgetCard} from "./MonitoringWidgetCard"
import type {LayoutCell} from "./lib/layout"
import {
    MONITORING_SNAPSHOT,
    WIDGETS,
    getDashboardResponse,
    monitoringHandlers,
    overviewDashboard,
} from "./__stories__/MonitoringFixture"

interface Scenario {
    widgetId: string
    render?: Partial<MonitoringRenderWidgetResponse>
    failing?: boolean
    configure: EMonitoringCapability
    onConfigureWidget: (widgetId: string) => void
    onDuplicateWidget: (widgetId: string) => void
}

let graphql: ReturnType<typeof graphqlBoundary>

function cellOf(widgetId: string): LayoutCell {
    const index = overviewDashboard.layout.findIndex((item) => item.widget === widgetId)
    const item = overviewDashboard.layout[index]
    return {
        key: `${index}:${widgetId}`,
        widgetId,
        width: item.width,
        values: item.values ?? {},
        widget: WIDGETS.find((widget) => widget.id === widgetId) ?? null,
        revision: 2,
    }
}

function Fixture({widgetId, configure, onConfigureWidget, onDuplicateWidget}: Scenario) {
    const response = getDashboardResponse()
    return (
        <AdminStoryProvider boundary={graphql}>
            <MonitoringProvider
                storageKey="monitoring:card-story"
                actions={{onConfigureWidget, onDuplicateWidget}}
            >
                <Box sx={{width: 560}}>
                    <MonitoringWidgetCard
                        cell={cellOf(widgetId)}
                        context={{
                            electionEventId: STORY_IDS.event,
                            dashboardId: overviewDashboard.id,
                            scope: {},
                            scopeLabel: "All regions · All Posts · All countries",
                            snapshot: MONITORING_SNAPSHOT,
                            sources: response.sources,
                            timeZone: response.settings.time_zone,
                            configVersion: "3/1/1",
                            configure,
                        }}
                    />
                </Box>
            </MonitoringProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Monitoring/MonitoringWidgetCard",
    component: MonitoringWidgetCard,
    args: {
        widgetId: "turnout-by-group",
        configure: EMonitoringCapability.DENIED,
        onConfigureWidget: fn(),
        onDuplicateWidget: fn(),
    },
    beforeEach: async ({args}) => {
        window.sessionStorage.clear()
        const handlers = monitoringHandlers({
            renders: args.render ? {[args.widgetId]: args.render} : {},
        })
        graphql = graphqlBoundary(
            args.failing
                ? {
                      ...handlers,
                      MonitoringRenderWidget: () => {
                          throw new Error("Synthetic renderer outage")
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

/** The open dialog, once its fade-in has finished. */
const dialog = async () => {
    const element = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

const renders = () =>
    graphql.calls
        .filter(({name}) => name === "MonitoringRenderWidget")
        .map(({variables}) => variables as unknown as MonitoringRenderWidgetVariables)

export const Rendered: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByTitle("Turnout by group chart")).toBeVisible()
        await expect(canvas.getByText("Voter turnout · SW-F-0260 · SW-F-0372")).toBeVisible()
        // The dashboard's value for the widget, sent with the scope and snapshot.
        await expect(canvas.getByRole("combobox", {name: "Show"})).toHaveTextContent(
            "Voted of pre-enrolled"
        )
        const [request] = renders()
        expect(request).toEqual(
            expect.objectContaining({
                widgetId: "turnout-by-group",
                selectorValues: {breakdown: "age_band", measure: "voted_pre"},
                snapshotRevision: MONITORING_SNAPSHOT.revision,
                colorScheme: "LIGHT",
            })
        )
        // A width in 40 px steps, so renders of nearby sizes are shared.
        expect(request.width % 40).toBe(0)
        expect(request.width).toBeGreaterThan(0)
    },
}

export const SelectorRendersAgain: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Turnout by group chart")
        await userEvent.click(canvas.getByRole("combobox", {name: "Breakdown"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Sex"}))
        await waitFor(() =>
            expect(renders().at(-1)?.selectorValues).toEqual({
                breakdown: "sex",
                measure: "voted_pre",
            })
        )
    },
}

export const NotConnected: Story = {
    args: {widgetId: "attack-detections"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText("Not connected · no attack detection feed is connected")
        ).toBeVisible()
        // Nothing to draw, so nothing is asked of the renderer.
        expect(renders()).toHaveLength(0)
    },
}

export const ScopePending: Story = {
    args: {render: {state: EWidgetRenderState.SCOPE_PENDING, svg: null, table: null}},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("Counting this selection")
        ).toBeVisible()
    },
}

export const RenderFailedShowsTable: Story = {
    args: {render: {state: EWidgetRenderState.RENDER_FAILED, svg: null}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("The chart could not be drawn")).toBeVisible()
        await expect(canvas.getByRole("table", {name: "Turnout by group"})).toBeVisible()
    },
}

export const RequestFailed: Story = {
    args: {failing: true},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("This widget could not be loaded")
        ).toBeVisible()
    },
}

export const ViewData: Story = {
    args: {render: {notices: ["Unregistered attempts are counted at event scope only."]}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Turnout by group chart")
        await expect(
            canvas.getByText("Unregistered attempts are counted at event scope only.")
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Actions for Turnout by group"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitem", {name: "View data"})
        )
        const body = await dialog()
        await expect(body.getByText("All regions · All Posts · All countries")).toBeVisible()
        expect(body.queryByRole("columnheader", {name: "position"})).toBeNull()
    },
}

export const ExportWidget: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Turnout by group chart")
        await userEvent.click(canvas.getByRole("button", {name: "Actions for Turnout by group"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitem", {name: "Export CSV"})
        )
        const body = await dialog()
        await userEvent.click(body.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls.find(({name}) => name === "MonitoringExport")?.variables).toEqual(
                expect.objectContaining({widgetId: "turnout-by-group", format: "CSV"})
            )
        )
    },
}

export const Configurer: Story = {
    args: {configure: EMonitoringCapability.GRANTED},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Actions for Turnout by group"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitem", {name: "Configure widget"})
        )
        await expect(args.onConfigureWidget).toHaveBeenCalledWith("turnout-by-group")
    },
}

export const ViewerCannotConfigure: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: "Actions for Turnout by group"})
        )
        const menu = within(await within(document.body).findByRole("menu"))
        expect(menu.queryByRole("menuitem", {name: "Configure widget"})).toBeNull()
        expect(menu.queryByRole("menuitem", {name: "Duplicate"})).toBeNull()
    },
}
