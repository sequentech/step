// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EMonitoringExportFormat} from "./types"
import {MonitoringExportDialog} from "./MonitoringExportDialog"
import {
    MONITORING_SNAPSHOT,
    monitoringHandlers,
    refusal,
    turnoutByGroup,
} from "./__stories__/MonitoringFixture"

interface Scenario {
    onClose: () => void
    onSnapshotPruned: () => void
    widgetId: string | null
    initialFormat: EMonitoringExportFormat
    /** Harvest's refusal of the export, as `extensions.code`. */
    refusal?: string
    /** The refusal's `extensions.problems`, for MONITORING_INVALID. */
    problems?: Array<{severity: string; code: string; path: string; message: string}>
}

const DASHBOARD_PICKS = {"turnout-by-group": {breakdown: "sex", measure: "voted_pre"}}

let graphql: ReturnType<typeof graphqlBoundary>

function Fixture({onClose, onSnapshotPruned, widgetId, initialFormat}: Scenario) {
    return (
        <AdminStoryProvider boundary={graphql}>
            <MonitoringExportDialog
                open
                onClose={onClose}
                title="Turnout by group"
                scope="North · All Posts · All countries"
                timeZone="Asia/Manila"
                initialFormat={initialFormat}
                onSnapshotPruned={onSnapshotPruned}
                widgets={[turnoutByGroup]}
                target={{
                    electionEventId: STORY_IDS.event,
                    electionId: null,
                    dashboardId: "overview",
                    widgetId,
                    scope: {region: "north"},
                    selectorValues: widgetId ? {breakdown: "age_band", measure: "voted_pre"} : {},
                    widgetSelectorValues: widgetId ? undefined : DASHBOARD_PICKS,
                    snapshotRevision: MONITORING_SNAPSHOT.revision,
                }}
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Monitoring/MonitoringExportDialog",
    component: MonitoringExportDialog,
    args: {
        onClose: fn(),
        onSnapshotPruned: fn(),
        widgetId: "turnout-by-group",
        initialFormat: EMonitoringExportFormat.CSV,
    },
    beforeEach: async ({args}) => {
        const handlers = monitoringHandlers()
        graphql = graphqlBoundary(
            args.refusal
                ? {
                      ...handlers,
                      MonitoringExport: () =>
                          refusal(
                              args.refusal as string,
                              args.problems ? {problems: args.problems} : {}
                          ),
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
const exports = () => graphql.calls.filter(({name}) => name === "MonitoringExport")

export const WidgetCsv: Story = {
    play: async ({args}) => {
        const body = await dialog()
        await expect(
            body.getByText("Turnout by group · North · All Posts · All countries")
        ).toBeVisible()
        await expect(body.getByText(/Times are in PhST/)).toBeVisible()
        await expect(body.getByRole("radio", {name: "CSV"})).toBeChecked()
        await userEvent.click(body.getByRole("button", {name: "Export"}))
        await expect(args.onClose).toHaveBeenCalled()
        await waitFor(() => expect(exports()).toHaveLength(1))
        expect(exports()[0].variables).toEqual(
            expect.objectContaining({
                electionEventId: STORY_IDS.event,
                dashboardId: "overview",
                widgetId: "turnout-by-group",
                scope: {region: "north"},
                selectorValues: {breakdown: "age_band", measure: "voted_pre"},
                snapshotRevision: MONITORING_SNAPSHOT.revision,
                format: EMonitoringExportFormat.CSV,
            })
        )
        expect(exports()[0].variables.from).toBeUndefined()
        expect(exports()[0].variables.widgetSelectorValues).toBeUndefined()
    },
}

export const DashboardSqlOverRange: Story = {
    args: {widgetId: null},
    play: async () => {
        const body = await dialog()
        await userEvent.click(body.getByRole("radio", {name: "SQL"}))
        fireEvent.change(body.getByLabelText("From"), {target: {value: "2026-05-11T08:00"}})
        fireEvent.change(body.getByLabelText("To"), {target: {value: "2026-05-12T08:00"}})
        await userEvent.click(body.getByRole("button", {name: "Export"}))
        await waitFor(() => expect(exports()).toHaveLength(1))
        expect(exports()[0].variables).toEqual(
            expect.objectContaining({
                widgetId: null,
                format: EMonitoringExportFormat.SQL,
                // Wall-clock times of the event's time zone, sent as instants.
                from: "2026-05-11T08:00:00+08:00",
                to: "2026-05-12T08:00:00+08:00",
                // Each widget exported as the dashboard draws it.
                widgetSelectorValues: DASHBOARD_PICKS,
            })
        )
        await expect(
            body.getByText(
                /Totals, statuses and groups are as of the shown update; only the rows of activity series are limited to the range/
            )
        ).toBeVisible()
    },
}

export const SnapshotPruned: Story = {
    args: {refusal: "MONITORING_SNAPSHOT_PRUNED"},
    play: async ({args}) => {
        const body = await dialog()
        await userEvent.click(body.getByRole("button", {name: "Export"}))
        // The dialog stays open to say why, and the dashboard asks for the current update.
        await expect(
            await body.findByText(
                "The update shown is no longer kept. The dashboard now shows the latest update: export again to use it."
            )
        ).toBeVisible()
        await expect(args.onSnapshotPruned).toHaveBeenCalledTimes(1)
        expect(args.onClose).not.toHaveBeenCalled()
    },
}

export const ForbiddenScope: Story = {
    args: {refusal: "MONITORING_FORBIDDEN_SCOPE"},
    play: async ({args}) => {
        const body = await dialog()
        await userEvent.click(body.getByRole("button", {name: "Export"}))
        await expect(
            await body.findByText(
                "You may not see this region, Post or country. Choose another one."
            )
        ).toBeVisible()
        expect(args.onSnapshotPruned).not.toHaveBeenCalled()
    },
}

export const EndBeforeStart: Story = {
    play: async () => {
        const body = await dialog()
        fireEvent.change(body.getByLabelText("From"), {target: {value: "2026-05-12T08:00"}})
        fireEvent.change(body.getByLabelText("To"), {target: {value: "2026-05-11T08:00"}})
        await expect(body.getByText("The end must be after the start.")).toBeVisible()
        await expect(body.getByRole("button", {name: "Export"})).toBeDisabled()
        expect(exports()).toHaveLength(0)
    },
}

/** Changing the range's zone leaves the shown dashboard and revision intact. */
export const ChosenZoneAndRepeatedTime: Story = {
    play: async () => {
        const body = await dialog()
        const picker = body.getByRole("combobox", {name: "Timezone"})
        await userEvent.clear(picker)
        await userEvent.type(picker, "New York")
        await userEvent.keyboard("{ArrowDown}{Enter}")
        fireEvent.change(body.getByLabelText("From"), {target: {value: "2028-11-05T01:30"}})
        await expect(body.getByText(/happens twice/)).toBeVisible()
        await userEvent.click(body.getByRole("button", {name: "Export"}))
        await waitFor(() => expect(exports()).toHaveLength(1))
        expect(exports()[0].variables).toEqual(
            expect.objectContaining({
                from: "2028-11-05T01:30:00-04:00",
                snapshotRevision: MONITORING_SNAPSHOT.revision,
                scope: {region: "north"},
            })
        )
    },
}

export const NonexistentTime: Story = {
    play: async () => {
        const body = await dialog()
        const picker = body.getByRole("combobox", {name: "Timezone"})
        await userEvent.clear(picker)
        await userEvent.type(picker, "New York")
        await userEvent.keyboard("{ArrowDown}{Enter}")
        fireEvent.change(body.getByLabelText("From"), {target: {value: "2028-03-12T02:30"}})
        await expect(body.getByText(/does not exist/)).toBeVisible()
        await expect(body.getByRole("button", {name: "Export"})).toBeDisabled()
        expect(exports()).toHaveLength(0)
    },
}

/** Harvest refuses a dashboard export's pick: the dialog names the widget and the choice. */
export const RefusedPick: Story = {
    args: {
        widgetId: null,
        refusal: "MONITORING_INVALID",
        problems: [
            {
                severity: "ERROR",
                code: "unknown_option",
                path: "widget_selector_values.turnout-by-group.breakdown",
                message: "'region' is not an option of 'breakdown'.",
            },
        ],
    },
    play: async ({args}) => {
        const body = await dialog()
        await userEvent.click(body.getByRole("button", {name: "Export"}))
        await expect(
            await body.findByText(
                "The value chosen for “Breakdown” in Turnout by group is no longer offered. Choose it again and export."
            )
        ).toBeVisible()
        expect(args.onClose).not.toHaveBeenCalled()
    },
}

/** A request Harvest cannot read, such as a bound without an offset, is told plainly. */
export const BadRequest: Story = {
    args: {refusal: "MONITORING_BAD_REQUEST"},
    play: async () => {
        const body = await dialog()
        await userEvent.click(body.getByRole("button", {name: "Export"}))
        await expect(
            await body.findByText("The request was not accepted. Reload the page and try again.")
        ).toBeVisible()
    },
}
