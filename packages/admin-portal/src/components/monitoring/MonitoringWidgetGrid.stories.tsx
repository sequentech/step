// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EMonitoringCapability, type MonitoringDashboard} from "./types"
import {MonitoringProvider} from "./MonitoringProvider"
import {MonitoringWidgetGrid} from "./MonitoringWidgetGrid"
import {layoutCells} from "./lib/layout"
import {parseWidgets} from "./lib/parseDefinitions"
import {
    MONITORING_SNAPSHOT,
    getDashboardResponse,
    monitoringHandlers,
    overviewDashboard,
} from "./__stories__/MonitoringFixture"

interface Scenario {
    dashboard: MonitoringDashboard
}

let graphql: ReturnType<typeof graphqlBoundary>

function Fixture({dashboard}: Scenario) {
    const response = getDashboardResponse({dashboard})
    return (
        <AdminStoryProvider boundary={graphql}>
            <MonitoringProvider storageKey="monitoring:grid-story">
                <MonitoringWidgetGrid
                    cells={layoutCells(dashboard, parseWidgets(response.widgets))}
                    context={{
                        electionEventId: STORY_IDS.event,
                        dashboardId: dashboard.id,
                        scope: {},
                        scopeLabel: "All regions · All Posts · All countries",
                        snapshot: MONITORING_SNAPSHOT,
                        sources: response.sources,
                        timeZone: response.settings.time_zone,
                        configVersion: "3/1/1",
                        configure: EMonitoringCapability.DENIED,
                    }}
                />
            </MonitoringProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Monitoring/MonitoringWidgetGrid",
    component: MonitoringWidgetGrid,
    args: {dashboard: overviewDashboard},
    beforeEach: async () => {
        window.sessionStorage.clear()
        graphql = graphqlBoundary(monitoringHandlers(), {schema: true})
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const cards = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll<HTMLElement>("[data-widget-id]"))

export const Layout: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Voter turnout chart")
        expect(cards(canvasElement).map((card) => card.dataset.widgetId)).toEqual(
            overviewDashboard.layout.map((item) => item.widget)
        )
        // On a wide screen the first widget takes the row and the next two share one.
        const [summary, byGroup, activity] = cards(canvasElement).map((card) =>
            card.getBoundingClientRect()
        )
        if (window.innerWidth >= 900) {
            expect(byGroup.top).toBe(activity.top)
            expect(summary.width).toBeGreaterThan(byGroup.width * 1.8)
        }
    },
}

export const MissingWidget: Story = {
    args: {
        dashboard: {
            ...overviewDashboard,
            layout: [...overviewDashboard.layout.slice(0, 1), {widget: "retired-widget", width: 6}],
        },
    },
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(
                "The dashboard names a widget that does not exist: retired-widget"
            )
        ).toBeVisible()
    },
}
