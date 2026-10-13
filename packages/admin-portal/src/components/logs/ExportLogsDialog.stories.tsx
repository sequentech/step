// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n, zoneLabel, zonedToInstant} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaRecords, electionRecord} from "@/__stories__/fixtures"
import {ExportLogsDialog} from "./ExportLogsDialog"
import {ELogScope, type ILogTarget} from "./logScope"

interface Scenario {
    log: ILogTarget
    /** The zones offered; the first is the log's primary. */
    zones: string[]
    /** The ELECTION log policy: rows keep their election's zone. */
    byElection?: boolean
    onClose: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const startedExport = (type: string, electionEventId: string = EVENT_ID) => ({
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
        election_event_id: electionEventId,
        type,
    },
})

const meta = {
    title: "Admin/Logs/ExportLogsDialog",
    component: ExportLogsDialog,
    args: {
        log: {kind: ELogScope.EVENT, electionEventId: EVENT_ID},
        zones: ["Asia/Manila", "Asia/Dubai", "America/New_York"],
        onClose: fn(),
    },
    beforeEach: async () => {
        data = resourceBoundary({
            sequent_backend_election: [electionRecord()],
            sequent_backend_area: areaRecords(),
        })
        graphql = graphqlBoundary(
            {
                ExportElectionEventLogs: ({variables}) => ({
                    data: {
                        export_election_event_logs: startedExport(
                            "EXPORT_ACTIVITY_LOGS_REPORT",
                            variables.electionEventId
                        ),
                    },
                }),
                ExportTenantLogs: () => ({
                    data: {export_tenant_logs: startedExport("EXPORT_TENANT_LOGS")},
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args: Scenario) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ExportLogsDialog open {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const options = {t: i18n.t.bind(i18n), lang: i18n.language}
const dialog = async () => within(await within(document.body).findByRole("dialog"))
const WHOLE_EVENT = {scope: "ALL", electionId: null, areaId: null, username: null}

const choose = async (view: Awaited<ReturnType<typeof dialog>>, label: string, option: string) => {
    await userEvent.click(view.getByRole("combobox", {name: new RegExp(`^${label}`)}))
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
}

/** The note names the chosen zone; CSV is the default of CSV, SQL and PDF. */
export const CsvInThePrimary: Story = {
    play: async ({args}) => {
        const view = await dialog()
        await waitFor(() => expect(view.getByText("Export logs")).toBeVisible())
        expect(view.getByRole("radio", {name: "CSV"})).toBeChecked()
        expect(
            view.getAllByRole("radio").map((radio) => (radio as HTMLInputElement).value)
        ).toEqual(["CSV", "SQL", "PDF"])
        expect(view.getByRole("combobox", {name: "Scope"})).toHaveTextContent(
            "The whole election event"
        )
        const abbr = zoneLabel(args.zones[0], options)
        expect(view.getByText(i18n.t("logsScreen.exportdialog.zoneNote", {abbr}))).toBeVisible()
        expect(view.getByText("The export itself is recorded in this log.")).toBeVisible()
    },
}

/** The range is sent as instants of the wall times in the chosen zone, with the zone. */
export const PdfInAnotherZone: Story = {
    play: async ({args}) => {
        const view = await dialog()
        await userEvent.click(view.getByRole("radio", {name: "PDF"}))
        const picker = view.getByRole("combobox", {name: "Timezone"})
        await userEvent.clear(picker)
        await userEvent.type(picker, "Dubai")
        await userEvent.keyboard("{ArrowDown}{Enter}")
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
                        ...WHOLE_EVENT,
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
            expect(view.getByRole("combobox", {name: "Timezone"})).toHaveValue(
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
                    ...WHOLE_EVENT,
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

/** One election of the event, as SQL: the election is chosen from the event's. */
export const OneElectionAsSql: Story = {
    play: async () => {
        const view = await dialog()
        await choose(view, "Scope", "One election")
        await waitFor(() => expect(view.getByRole("button", {name: "Export"})).toBeDisabled())
        await choose(view, "Election", "Council")
        await userEvent.click(view.getByRole("radio", {name: "SQL"}))
        await userEvent.click(view.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls.map(({variables}) => variables)).toEqual([
                {
                    electionEventId: EVENT_ID,
                    format: "SQL",
                    scope: "ELECTION",
                    electionId: STORY_IDS.election,
                    areaId: null,
                    username: null,
                    createdFrom: null,
                    createdTo: null,
                    timeZone: "Asia/Manila",
                },
            ])
        )
    },
}

/** One area of the event. */
export const OneArea: Story = {
    play: async () => {
        const view = await dialog()
        await choose(view, "Scope", "One area")
        await choose(view, "Area", areaRecords()[0].name as string)
        await userEvent.click(view.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls.map(({variables}) => variables)).toEqual([
                expect.objectContaining({
                    scope: "AREA",
                    areaId: areaRecords()[0].id,
                    electionId: null,
                }),
            ])
        )
    },
}

const TENANT_ARGS: Partial<Scenario> = {
    log: {kind: ELogScope.TENANT},
    zones: ["Europe/Madrid", "America/New_York"],
}

/** The tenant log exports whole by default, with its own role. */
export const TheWholeTenantLog: Story = {
    args: TENANT_ARGS,
    play: async () => {
        const view = await dialog()
        expect(view.getByRole("combobox", {name: "Scope"})).toHaveTextContent(
            "The whole tenant log"
        )
        expect(view.queryByRole("textbox", {name: /Username/})).toBeNull()
        expect(view.queryByRole("checkbox")).toBeNull()
        await userEvent.click(view.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls).toEqual([
                {
                    name: "ExportTenantLogs",
                    variables: {
                        format: "CSV",
                        scope: "TENANT_LOG",
                        username: null,
                        createdFrom: null,
                        createdTo: null,
                        timeZone: "Europe/Madrid",
                    },
                    headers: {"x-hasura-role": "tenant-logs-export"},
                },
            ])
        )
        expect(data.calls).toEqual([])
    },
}

/** One account needs its username; its entries in every election event can be added. */
export const OneAccountWithItsElectionEvents: Story = {
    args: TENANT_ARGS,
    play: async () => {
        const view = await dialog()
        await choose(view, "Scope", "One account")
        const exportButton = view.getByRole("button", {name: "Export"})
        await waitFor(() => expect(exportButton).toBeDisabled())
        await userEvent.type(view.getByRole("textbox", {name: /Username/}), "rosa.marin")
        await waitFor(() => expect(exportButton).toBeEnabled())
        expect(view.queryByText("Rows from election events name their log.")).toBeNull()
        await userEvent.click(
            view.getByRole("checkbox", {
                name: "Include this account's entries in every election event",
            })
        )
        await expect(view.getByText("Rows from election events name their log.")).toBeVisible()
        fireEvent.change(view.getByLabelText("From"), {target: {value: "2028-02-01T00:00"}})
        fireEvent.change(view.getByLabelText("To"), {target: {value: "2028-02-03T23:59"}})
        await userEvent.click(exportButton)
        await waitFor(() =>
            expect(graphql.calls.map(({name, variables}) => [name, variables])).toEqual([
                [
                    "ExportTenantLogs",
                    {
                        format: "CSV",
                        scope: "ACCOUNT_WITH_EVENTS",
                        username: "rosa.marin",
                        createdFrom: zonedToInstant("2028-02-01T00:00", "Europe/Madrid").instant,
                        createdTo: zonedToInstant("2028-02-03T23:59", "Europe/Madrid").instant,
                        timeZone: "Europe/Madrid",
                    },
                ],
            ])
        )
    },
}
