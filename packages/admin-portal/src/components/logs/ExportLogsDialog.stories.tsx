// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n, zoneLabel, zonedToInstant} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {ExportLogsDialog} from "./ExportLogsDialog"

interface Scenario {
    /** The zones offered; the first is the event's primary. */
    zones: string[]
    /** The ELECTION log policy: rows keep their election's zone. */
    byElection?: boolean
    onClose: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Logs/ExportLogsDialog",
    component: ExportLogsDialog,
    args: {zones: ["Asia/Manila", "Asia/Dubai", "America/New_York"], onClose: fn()},
    beforeEach: async () => {
        graphql = graphqlBoundary(
            {
                ExportElectionEventLogs: ({variables}) => ({
                    data: {
                        export_election_event_logs: {
                            document_id: STORY_IDS.tallySession,
                            task_execution: {
                                id: STORY_IDS.keysCeremony,
                                name: "Export logs",
                                execution_status: "IN_PROGRESS",
                                created_at: "2028-04-09T00:00:00.000Z",
                                start_at: "2028-04-09T00:00:00.000Z",
                                end_at: null,
                                logs: [],
                                annotations: {},
                                labels: {},
                                executed_by_user: "admin",
                                tenant_id: STORY_IDS.tenant,
                                election_event_id: variables.electionEventId,
                                type: "EXPORT_ACTIVITY_LOGS_REPORT",
                            },
                        },
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args: Scenario) => (
        <AdminStoryProvider boundary={graphql}>
            <ExportLogsDialog electionEventId={EVENT_ID} open {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const options = {t: i18n.t.bind(i18n), lang: i18n.language}
const dialog = async () => within(await within(document.body).findByRole("dialog"))

/** The note names the chosen zone; CSV is the default format, and there is no SQL. */
export const CsvInThePrimary: Story = {
    play: async ({args}) => {
        const view = await dialog()
        await waitFor(() => expect(view.getByText("Export logs")).toBeVisible())
        expect(view.getByRole("radio", {name: "CSV"})).toBeChecked()
        expect(view.queryByRole("radio", {name: "SQL"})).toBeNull()
        const abbr = zoneLabel(args.zones[0], options)
        expect(view.getByText(i18n.t("logsScreen.exportdialog.zoneNote", {abbr}))).toBeVisible()
    },
}

/** The range is sent as instants of the wall times in the chosen zone, with the zone. */
export const PdfInAnotherZone: Story = {
    play: async ({args}) => {
        const view = await dialog()
        await userEvent.click(view.getByRole("radio", {name: "PDF"}))
        await userEvent.click(view.getByRole("combobox"))
        await userEvent.click(await within(document.body).findByRole("option", {name: /Dubai/}))
        const abbr = zoneLabel("Asia/Dubai", options)
        expect(view.getByText(i18n.t("logsScreen.exportdialog.zoneNotePdf", {abbr}))).toBeVisible()
        fireEvent.change(view.getByLabelText("From"), {target: {value: "2028-04-01T00:00"}})
        fireEvent.change(view.getByLabelText("To"), {target: {value: "2028-04-09T23:59"}})
        await userEvent.click(view.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls).toEqual([
                {
                    name: "ExportElectionEventLogs",
                    variables: {
                        electionEventId: EVENT_ID,
                        format: "PDF",
                        createdFrom: zonedToInstant("2028-04-01T00:00", "Asia/Dubai").instant,
                        createdTo: zonedToInstant("2028-04-09T23:59", "Asia/Dubai").instant,
                        timeZone: "Asia/Dubai",
                    },
                    headers: {"x-hasura-role": "logs-export"},
                },
            ])
        )
        expect(args.onClose).toHaveBeenCalled()
    },
}

/**
 * Under the ELECTION policy the export starts with each row in its election's
 * zone: no zone is sent, and the range is read in the primary.
 */
export const EachRowInItsElectionsZone: Story = {
    args: {byElection: true},
    play: async ({args}) => {
        const view = await dialog()
        await waitFor(() =>
            expect(view.getByRole("combobox")).toHaveTextContent(
                i18n.t("logsScreen.exportdialog.rowZones")
            )
        )
        const primary = args.zones[0]
        fireEvent.change(view.getByLabelText("From"), {target: {value: "2028-04-01T00:00"}})
        const abbr = zoneLabel(
            primary,
            options,
            new Date(zonedToInstant("2028-04-01T00:00", primary).instant)
        )
        const note = view.getByText(i18n.t("logsScreen.exportdialog.zoneNoteRows", {abbr}))
        await waitFor(() => expect(note).toBeVisible())
        await userEvent.click(view.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls.map(({variables}) => variables)).toEqual([
                {
                    electionEventId: EVENT_ID,
                    format: "CSV",
                    createdFrom: zonedToInstant("2028-04-01T00:00", primary).instant,
                    createdTo: null,
                    timeZone: null,
                },
            ])
        )
    },
}

/** A range that ends before it starts can't be exported. */
export const ReversedRange: Story = {
    play: async () => {
        const view = await dialog()
        fireEvent.change(view.getByLabelText("From"), {target: {value: "2028-04-09T00:00"}})
        fireEvent.change(view.getByLabelText("To"), {target: {value: "2028-04-01T00:00"}})
        await waitFor(() => expect(view.getByRole("button", {name: "Export"})).toBeDisabled())
        expect(graphql.calls).toEqual([])
    },
}
