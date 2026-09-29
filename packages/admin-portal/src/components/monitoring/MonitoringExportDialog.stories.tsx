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
import {MONITORING_SNAPSHOT, monitoringHandlers} from "./__stories__/MonitoringFixture"

interface Scenario {
    onClose: () => void
    widgetId: string | null
    initialFormat: EMonitoringExportFormat
}

let graphql: ReturnType<typeof graphqlBoundary>

function Fixture({onClose, widgetId, initialFormat}: Scenario) {
    return (
        <AdminStoryProvider boundary={graphql}>
            <MonitoringExportDialog
                open
                onClose={onClose}
                title="Turnout by group"
                scope="North · All Posts · All countries"
                timeZone="Asia/Manila"
                initialFormat={initialFormat}
                target={{
                    electionEventId: STORY_IDS.event,
                    electionId: null,
                    dashboardId: "overview",
                    widgetId,
                    scope: {region: "north"},
                    selectorValues: {breakdown: "age_band", measure: "voted_pre"},
                    snapshotRevision: MONITORING_SNAPSHOT.revision,
                }}
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Monitoring/MonitoringExportDialog",
    component: MonitoringExportDialog,
    args: {onClose: fn(), widgetId: "turnout-by-group", initialFormat: EMonitoringExportFormat.CSV},
    beforeEach: async () => {
        graphql = graphqlBoundary(monitoringHandlers(), {schema: true})
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
        await expect(body.getByText(/Times are in Asia\/Manila/)).toBeVisible()
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
                // Wall-clock times, read in the event's time zone.
                from: "2026-05-11T08:00:00",
                to: "2026-05-12T08:00:00",
            })
        )
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
