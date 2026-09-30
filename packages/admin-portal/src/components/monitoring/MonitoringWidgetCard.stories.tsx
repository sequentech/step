// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {FetchResult, Operation} from "@apollo/client"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {Box, Button} from "@mui/material"
import {pending} from "../../../../ui-essentials/.storybook/screens"
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
    EVENT_DAYS,
    MONITORING_SNAPSHOT,
    WIDGETS,
    getDashboardResponse,
    monitoringHandlers,
    overviewDashboard,
    refusal,
} from "./__stories__/MonitoringFixture"

/** How the renderer answers after the first chart. */
enum ERenderScenario {
    DRAWN = "DRAWN",
    /** Every request fails. */
    FAILING = "FAILING",
    /** The first request fails, and the next ones are drawn. */
    FAILS_ONCE = "FAILS_ONCE",
    /** Nothing ever answers. */
    PENDING = "PENDING",
    /** The chart by sex never answers. */
    SEX_PENDING = "SEX_PENDING",
    /** The chart by sex fails. */
    SEX_FAILS = "SEX_FAILS",
    /** Harvest refuses the scope. */
    FORBIDDEN_SCOPE = "FORBIDDEN_SCOPE",
}

interface Scenario {
    widgetId: string
    render?: Partial<MonitoringRenderWidgetResponse>
    scenario?: ERenderScenario
    configure: EMonitoringCapability
    onConfigureWidget: (widgetId: string, width?: number) => void
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
    const [pollCount, setPollCount] = useState(0)
    return (
        <AdminStoryProvider boundary={graphql}>
            <MonitoringProvider
                storageKey="monitoring:card-story"
                actions={{onConfigureWidget, onDuplicateWidget}}
            >
                <Box sx={{width: 560}}>
                    {/* Stands for the dashboard's 30 s poll. */}
                    <Button onClick={() => setPollCount((count) => count + 1)}>Poll</Button>
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
                            eventDays: EVENT_DAYS,
                            configVersion: "3/1/1",
                            pollCount,
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
        let requests = 0
        const draw = handlers.MonitoringRenderWidget
        const reply = (operation: Operation): FetchResult | Promise<FetchResult> => {
            requests += 1
            const bySex =
                (operation.variables.selectorValues as Record<string, string>).breakdown === "sex"
            switch (args.scenario ?? ERenderScenario.DRAWN) {
                case ERenderScenario.FAILING:
                    throw new Error("Synthetic renderer outage")
                case ERenderScenario.FAILS_ONCE:
                    if (requests === 1) throw new Error("Synthetic renderer outage")
                    return draw(operation)
                case ERenderScenario.PENDING:
                    return pending()
                case ERenderScenario.SEX_PENDING:
                    return bySex ? pending() : draw(operation)
                case ERenderScenario.SEX_FAILS:
                    if (bySex) throw new Error("Synthetic renderer outage")
                    return draw(operation)
                case ERenderScenario.FORBIDDEN_SCOPE:
                    return refusal("MONITORING_FORBIDDEN_SCOPE")
                case ERenderScenario.DRAWN:
                    return draw(operation)
            }
        }
        graphql = graphqlBoundary({...handlers, MonitoringRenderWidget: reply}, {schema: true})
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
        const frame = await canvas.findByTitle("Turnout by group chart")
        await expect(frame).toBeVisible()
        // The engine's font travels with the chart, as a data: URI.
        expect(frame.getAttribute("srcdoc")).toContain(
            "@font-face{font-family:'Inter Variable';src:url(data:font/woff;base64,"
        )
        await expect(
            canvas.getByRole("heading", {level: 3, name: "Turnout by group"})
        ).toBeVisible()
        // Renders are not kept in the Apollo cache, whose snapshots would pile up.
        expect(JSON.stringify(graphql.client.cache.extract())).not.toContain(
            "monitoringRenderWidget"
        )
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

export const Loading: Story = {
    args: {scenario: ERenderScenario.PENDING},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("progressbar", {
                name: "Loading Turnout by group",
            })
        ).toBeVisible()
    },
}

export const RequestFailed: Story = {
    args: {scenario: ERenderScenario.FAILING},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("This widget could not be loaded")
        ).toBeVisible()
    },
}

export const RetriedOnNextPoll: Story = {
    args: {scenario: ERenderScenario.FAILS_ONCE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("This widget could not be loaded")
        // The snapshot has not changed, but the dashboard's poll asks again.
        await userEvent.click(canvas.getByRole("button", {name: "Poll"}))
        await expect(await canvas.findByTitle("Turnout by group chart")).toBeVisible()
    },
}

export const RefusedScope: Story = {
    args: {scenario: ERenderScenario.FORBIDDEN_SCOPE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("This widget could not be loaded")).toBeVisible()
        await expect(
            canvas.getByText("You may not see this region, Post or country. Choose another one.")
        ).toBeVisible()
    },
}

const pickSex = async (canvasElement: HTMLElement) => {
    const canvas = within(canvasElement)
    await canvas.findByTitle("Turnout by group chart")
    await userEvent.click(canvas.getByRole("combobox", {name: "Breakdown"}))
    await userEvent.click(await within(document.body).findByRole("option", {name: "Sex"}))
}

export const UpdatingKeepsPreviousChart: Story = {
    args: {scenario: ERenderScenario.SEX_PENDING},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await pickSex(canvasElement)
        await expect(
            await canvas.findByText("Updating: the chart shown is the previous one.")
        ).toBeVisible()
        await expect(
            canvas.getByRole("progressbar", {name: "Updating Turnout by group"})
        ).toBeVisible()
        await expect(canvas.getByTitle("Turnout by group chart")).toBeVisible()
    },
}

export const FailedChangeShowsFailure: Story = {
    args: {scenario: ERenderScenario.SEX_FAILS},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await pickSex(canvasElement)
        await expect(await canvas.findByText("This widget could not be loaded")).toBeVisible()
        // Not the chart of the previous choice, as if it were the new one.
        expect(canvas.queryByTitle("Turnout by group chart")).toBeNull()
    },
}

export const DayPicker: Story = {
    args: {widgetId: "voting-activity"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Voting activity chart")
        // A toggle shows its name, as a dropdown does.
        await expect(canvas.getByText("Grain")).toBeVisible()
        await userEvent.click(
            within(canvas.getByRole("group", {name: "Grain"})).getByRole("button", {
                name: "Hourly",
            })
        )
        // Hourly asks for a day, offered from the event's days, the latest first chosen.
        await expect(await canvas.findByRole("combobox", {name: "Day"})).toHaveTextContent(
            EVENT_DAYS[1]
        )
        await waitFor(() =>
            expect(renders().at(-1)?.selectorValues).toEqual({grain: "hour", day: EVENT_DAYS[1]})
        )
    },
}

export const ViewData: Story = {
    args: {render: {notices: ["UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY", "SOMETHING_NEW"]}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Turnout by group chart")
        await expect(
            canvas.getByText(
                "Attempts by unregistered usernames belong to no Post, so they are counted for the whole event only."
            )
        ).toBeVisible()
        // A notice this build does not know is shown as words, never as its code.
        await expect(canvas.getByText("Something new")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Actions for Turnout by group"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitem", {name: "View data"})
        )
        const body = await dialog()
        await expect(body.getByText("All regions · All Posts · All countries")).toBeVisible()
        expect(body.queryByRole("columnheader", {name: "position"})).toBeNull()
    },
}

/** View data of a widget that reads several queries shows each one. */
export const ViewDataOfEveryQuery: Story = {
    args: {widgetId: "turnout-summary"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByTitle("Voter turnout chart")
        await userEvent.click(canvas.getByRole("button", {name: "Actions for Voter turnout"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitem", {name: "View data"})
        )
        const body = await dialog()
        expect(
            body.getAllByRole("heading", {level: 3}).map((heading) => heading.textContent)
        ).toEqual(["totals", "voted_reg", "voted_pre"])
        const ratio = within(body.getByRole("table", {name: "Voter turnout · data · voted_reg"}))
        await expect(ratio.getByRole("columnheader", {name: "Registered"})).toBeVisible()
        expect(ratio.getAllByText("37.5%")).toHaveLength(2)
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
        // With the card's width, for the preview to draw what the card does.
        await expect(args.onConfigureWidget).toHaveBeenCalledWith(
            "turnout-by-group",
            expect.any(Number)
        )
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
