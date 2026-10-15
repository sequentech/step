// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {Box} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {type ReadState, resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, type StoryRecord, eventRecord} from "@/__stories__/fixtures"
import type {ElectoralLogRow} from "@/gql/graphql"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {startedTask, taskHandler} from "@/components/tally/__stories__/DownloadFixture"
import {ElectoralLogFilters, ElectoralLogList} from "./ElectoralLogList"
import {useStoryGlobals} from "../../../ui-essentials/.storybook/globals"

interface Scenario {
    reads: ReadState
    logs: "listed" | "none"
    /** Rendered inside the event's record, as the event's Logs tab does. */
    inEvent: boolean
    electionEventId?: string
    filterToShow?: ElectoralLogFilters
    filterValue?: string
    showActions?: boolean
    roles?: string[]
    zones?: string[]
    /** Mounts the task widgets, which show the audit's task. */
    taskWidgets?: boolean
    /** Whether Harvest refuses to start the audit. */
    auditFails?: boolean
    /** The room the list has, in pixels; the canvas's when absent. */
    width?: number
    /** Rendered inside a popup, as a voter's logs are. */
    inDialog?: boolean
}

const logRow = (
    id: number,
    kind: string,
    user: {id: string; name: string} | null,
    head: {event_type: string; log_type: string; description: string},
    body: Record<string, unknown> = {}
): StoryRecord<ElectoralLogRow> & {election_event_id: string} => ({
    id,
    election_event_id: EVENT_ID,
    created: 1767225600 + id * 60,
    statement_timestamp: 1767225600 + id * 60,
    statement_kind: kind,
    user_id: user?.id ?? "null",
    message: JSON.stringify({
        user_id: user?.id ?? null,
        username: user?.name ?? null,
        statement: {head, body},
    }),
})

const SEAL_HASH = `ef187f0b${"3c".repeat(56)}22a65e5b`
const MESSAGE_ID = "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a00"
const SEALED = "Ballot box of Madrid Post, Spain sealed: 1,340 of 1,342 ballots counted."
const AUDIT_TASK_ID = "99999999-9999-4999-8999-999999999991"

const logs = [
    logRow(
        4,
        "BallotBoxSealed",
        null,
        {event_type: "SYSTEM", log_type: "INFO", description: SEALED},
        {BallotBoxSealed: [null, "spain", SEAL_HASH, 1342, 1340, null]}
    ),
    logRow(
        3,
        "SendCommunications",
        {id: STORY_IDS.secondUser, name: "bob"},
        {event_type: "SEND_COMMUNICATIONS", log_type: "INFO", description: "Notification sent"},
        // As windmill writes it: the channel in lowercase, inside a JSON string.
        {
            SendCommunications: JSON.stringify({
                channel: "whatsapp",
                receiver: "+34 ••• ••• 214",
                message_id: MESSAGE_ID,
                attempt: 1,
                outcome: "ACCEPTED",
            }),
        }
    ),
    logRow(
        2,
        "CastVote",
        {id: STORY_IDS.user, name: "alice"},
        {event_type: "CAST_VOTE", log_type: "INFO", description: "Ballot cast"}
    ),
    logRow(1, "KeyGeneration", null, {
        event_type: "KEYS_CEREMONY",
        log_type: "INFO",
        description: "Keys generated",
    }),
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({
    inEvent,
    electionEventId,
    filterToShow,
    filterValue,
    showActions,
    roles,
    zones,
    taskWidgets,
    width,
    inDialog,
}: Scenario) {
    const {permissions} = useStoryGlobals()
    const logList = (
        <ElectoralLogList
            electionEventId={electionEventId}
            filterToShow={filterToShow}
            filterValue={filterValue}
            showActions={showActions}
        />
    )
    const placed = inDialog ? (
        <Dialog
            fullWidth
            variant="info"
            maxWidth="xl"
            title="Voter logs"
            ok="Close"
            open
            handleClose={() => undefined}
        >
            {logList}
        </Dialog>
    ) : (
        logList
    )
    const sized = width ? <Box sx={{width}}>{placed}</Box> : placed
    const list = taskWidgets ? <WidgetsContextProvider>{sized}</WidgetsContextProvider> : sized
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            roles={roles}
        >
            {inEvent ? (
                <RecordContextProvider
                    value={
                        zones
                            ? {
                                  ...eventRecord(),
                                  presentation: {
                                      timezones: {
                                          configured: zones,
                                          primary: zones[0],
                                          logs: "primary",
                                      },
                                  },
                              }
                            : eventRecord()
                    }
                >
                    {list}
                </RecordContextProvider>
            ) : (
                list
            )}
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Components/ElectoralLogList",
    component: ElectoralLogList,
    args: {reads: "records", logs: "listed", inEvent: true},
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        logs: {control: "inline-radio", options: ["listed", "none"]},
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {electoral_log: args.logs === "listed" ? logs : []},
            {reads: args.reads}
        )
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
                                created_at: "2026-01-01T00:00:00.000Z",
                                start_at: "2026-01-01T00:00:00.000Z",
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
                AuditElectoralLog: () =>
                    args.auditFails
                        ? {errors: [new GraphQLError("Synthetic audit unavailable")]}
                        : {
                              data: {
                                  audit_electoral_log: {
                                      task_execution: startedTask(
                                          AUDIT_TASK_ID,
                                          "AUDIT_ELECTORAL_LOG"
                                      ),
                                  },
                              },
                          },
                ...taskHandler("SUCCESS", "AUDIT_ELECTORAL_LOG"),
                GetLedgerMessageStates: () => ({
                    data: {
                        sequent_backend_message: [
                            {
                                id: MESSAGE_ID,
                                channel: "WHATSAPP",
                                state: "DELIVERED",
                                attempt: 1,
                                error: null,
                                created_at: "2026-01-01T00:00:00.000Z",
                                updated_at: "2026-01-01T00:01:00.000Z",
                            },
                        ],
                    },
                }),
            },
            // The ledger query is not in the generated schema yet.
            {schema: true, unvalidated: ["GetLedgerMessageStates"]}
        )
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const listed = () => data.calls.find(({method}) => method === "getList")?.args

async function exportAs(canvasElement: HTMLElement, format: "CSV" | "PDF") {
    await within(canvasElement).findByText("alice")
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Export"}))
    const dialog = within(await within(document.body).findByRole("dialog"))
    await userEvent.click(dialog.getByRole("radio", {name: format}))
    return dialog
}

const ledgerReads = [
    {
        name: "GetLedgerMessageStates",
        variables: {tenantId: STORY_IDS.tenant, ids: [MESSAGE_ID]},
        headers: {"x-hasura-role": "messaging-config-read"},
    },
]

/** Without a range, the whole log in the event's primary zone (UTC without zones). */
const exported = (electionEventId: string, format: string) => [
    ...ledgerReads,
    {
        name: "ExportElectionEventLogs",
        variables: {
            electionEventId,
            format,
            scope: "ALL",
            electionId: null,
            areaId: null,
            username: null,
            createdFrom: null,
            createdTo: null,
            timeZone: "UTC",
        },
        headers: {"x-hasura-role": "logs-export"},
    },
]

export const Populated: Story = {
    parameters: {widgets: ["LogRowTime"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("alice")).toBeVisible()
        expect(canvas.getByText("CAST_VOTE")).toBeVisible()
        expect(canvas.getByText("Keys generated")).toBeVisible()
        // Each head column has its own label.
        const headers = canvas.getAllByRole("columnheader").map(({textContent}) => textContent)
        expect(headers.filter((text) => text === "Statement kind")).toHaveLength(1)
        expect(headers).toContain("Event Type")
        expect(headers).toContain("Log Type")
        expect(canvas.getByText("WhatsApp")).toBeVisible()
        expect(listed()).toEqual([
            "electoral_log",
            expect.objectContaining({
                // The range filter's zone defaults to the event's primary (UTC without zones).
                filter: {election_event_id: EVENT_ID, default_time_zone: "UTC"},
                sort: {field: "id", order: "DESC"},
            }),
        ])
        await waitFor(() => expect(graphql.calls).toEqual(ledgerReads))
    },
}

/** The Status column reads the message's current state from the ledger. */
export const StatusFromTheLedger: Story = {
    args: {roles: ["logs-read", "messaging-config-read"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("bob")).toBeVisible()
        await expect(canvas.getByText("WhatsApp")).toBeVisible()
        await expect(await canvas.findByText("Delivered")).toBeVisible()
        expect(graphql.calls).toEqual([
            {
                name: "GetLedgerMessageStates",
                variables: {tenantId: STORY_IDS.tenant, ids: [MESSAGE_ID]},
                headers: {"x-hasura-role": "messaging-config-read"},
            },
        ])
    },
}

/** Without the ledger's role, the state the entry was written with. */
export const StatusAsLogged: Story = {
    args: {roles: ["logs-read"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("bob")).toBeVisible()
        await expect(canvas.getByText("Accepted")).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

const entryDialog = () => within(document.body).findByRole("dialog", {name: /^Log entry/})

/**
 * The description stays on one line; clicking the row opens the entry's
 * popup, with the whole description and the seal's details (hash, counts).
 */
export const OpenAnEntry: Story = {
    parameters: {widgets: ["OpenLogEntry"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        const description = canvas.getByText(SEALED)
        await expect(description).toHaveStyle({whiteSpace: "nowrap", textOverflow: "ellipsis"})
        expect(canvas.queryByRole("button", {name: "Show More"})).toBeNull()
        expect(canvas.queryByText(`Seal hash: ${SEAL_HASH}`)).toBeNull()
        await userEvent.click(description)
        const dialog = within(await entryDialog())
        await waitFor(() => expect(dialog.getByText(SEALED)).toBeVisible())
        await expect(dialog.getByText(`Seal hash: ${SEAL_HASH}`)).toBeVisible()
        await expect(dialog.getByText("1340 of 1342 ballots counted.")).toBeVisible()
        await expect(dialog.getByText("Election event log")).toBeVisible()
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        // Another row opens its own entry.
        await userEvent.click(canvas.getByText("Ballot cast"))
        await expect(await entryDialog()).toHaveAccessibleName("Log entry 2")
        await userEvent.click(within(await entryDialog()).getByRole("button", {name: "Close"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

/** The column headers, also while the Columns menu covers the list. */
const headers = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll("thead th")).map(({textContent}) => textContent)

/** From 1,500 px every column shows; the raw message stays under Columns. */
export const WideList: Story = {
    args: {width: 1600},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        expect(headers(canvasElement)).toEqual([
            "Id",
            "Username",
            "Created",
            "Statement Timestamp",
            "Statement kind",
            "Event Type",
            "Log Type",
            "Channel",
            "Status",
            "Description",
        ])
        await userEvent.click(canvas.getByRole("button", {name: "Columns"}))
        const message = await within(document.body).findByRole("switch", {name: "Message"})
        await expect(message).not.toBeChecked()
        await userEvent.click(message)
        await waitFor(() => expect(headers(canvasElement)).toContain("Message"))
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(within(document.body).queryByRole("switch")).toBeNull())
    },
}

/** Below 1,500 px Created starts hidden under Columns. */
export const MediumListHidesCreated: Story = {
    args: {width: 1200},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        await waitFor(() => expect(headers(canvasElement)).not.toContain("Created"))
        expect(headers(canvasElement)).toContain("Statement Timestamp")
        await userEvent.click(canvas.getByRole("button", {name: "Columns"}))
        const created = await within(document.body).findByRole("switch", {name: "Created"})
        await expect(created).not.toBeChecked()
        await userEvent.click(created)
        await waitFor(() => expect(headers(canvasElement)).toContain("Created"))
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(within(document.body).queryByRole("switch")).toBeNull())
    },
}

/** Below 1,000 px each entry is two lines, and opens the same popup. */
export const NarrowList: Story = {
    args: {width: 640},
    parameters: {widgets: ["CompactLogEntries", "CompactLogEntry"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(canvas.getAllByRole("listitem")).toHaveLength(logs.length))
        expect(canvas.queryByRole("table")).toBeNull()
        const entry = canvas.getByText(SEALED).closest("li") as HTMLElement
        await expect(within(entry).getByText(/^4 · .* · BallotBoxSealed$/)).toBeVisible()
        expect(canvas.getByText(/^2 · .* · alice · CastVote$/)).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Columns"})).toBeNull()
        await userEvent.click(within(entry).getByRole("button"))
        const dialog = within(await entryDialog())
        await waitFor(() => expect(dialog.getByText(`Seal hash: ${SEAL_HASH}`)).toBeVisible())
    },
}

/** The event type and log type filters reach the list request. */
export const FilterByEventAndLogType: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        const choose = async (filter: string, option: string) => {
            await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
            await userEvent.click(
                await within(document.body).findByRole("menuitemcheckbox", {name: filter})
            )
            await userEvent.click(await canvas.findByRole("combobox", {name: filter}))
            await userEvent.click(await within(document.body).findByRole("option", {name: option}))
        }
        await choose("Event Type", "SYSTEM")
        await choose("Log Type", "ERROR")
        await waitFor(() =>
            expect(
                data.calls.some(
                    ({method, args}) =>
                        method === "getList" &&
                        (args[1] as {filter?: Record<string, unknown>})?.filter?.event_type ===
                            "SYSTEM" &&
                        (args[1] as {filter?: Record<string, unknown>})?.filter?.log_type ===
                            "ERROR"
                )
            ).toBe(true)
        )
    },
}

export const SearchableRangeZone: Story = {
    parameters: {widgets: ["LogTimeZoneFilter"]},
    args: {zones: ["Asia/Manila", "Asia/Dubai"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        const picker = canvas.getByRole("combobox", {name: "Timezone"})
        await userEvent.clear(picker)
        await userEvent.type(picker, "Dubai")
        await userEvent.keyboard("{ArrowDown}{Enter}")
        await waitFor(() =>
            expect(
                data.calls.some(
                    ({method, args}) =>
                        method === "getList" &&
                        (args[1] as {filter?: {time_zone?: string}})?.filter?.time_zone ===
                            "Asia/Dubai"
                )
            ).toBe(true)
        )
        await expect(picker).toHaveValue("(GMT+04:00) Dubai")
    },
}

export const NoLogs: Story = {
    args: {logs: "none"},
    parameters: {
        expectedFailure: {
            reason: "React-admin's empty list message is grey below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await waitFor(() => expect(listed()).toBeDefined())
        await expect(await within(canvasElement).findByText("No Electoral logs yet.")).toBeVisible()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(listed()).toBeDefined())
        expect(within(canvasElement).queryByText("alice")).toBeNull()
    },
}

export const LogsUnavailable: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(listed()).toBeDefined())
        const notice = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(notice).toBeVisible())
        expect(within(canvasElement).queryByText("alice")).toBeNull()
    },
}

export const OneVotersLog: Story = {
    args: {
        inEvent: false,
        electionEventId: EVENT_ID,
        filterToShow: ElectoralLogFilters.USER_ID,
        filterValue: STORY_IDS.user,
    },
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("alice")).toBeVisible()
        expect(listed()?.[1]).toMatchObject({
            filter: {election_event_id: EVENT_ID, user_id: STORY_IDS.user},
        })
    },
}

/** In a voter's logs popup an entry opens as a second popup over it; Escape closes only that one. */
export const EntryOverTheVotersLogs: Story = {
    args: {
        inEvent: false,
        inDialog: true,
        electionEventId: EVENT_ID,
        showActions: false,
        filterToShow: ElectoralLogFilters.USER_ID,
        filterValue: STORY_IDS.user,
    },
    play: async () => {
        const logsPopup = await within(document.body).findByRole("dialog", {name: "Voter logs"})
        await userEvent.click(await within(logsPopup).findByText("Ballot cast"))
        const entry = await within(document.body).findByRole("dialog", {name: "Log entry 2"})
        await waitFor(() => expect(within(entry).getByText("Ballot cast")).toBeVisible())
        await expect(within(entry).getByText("alice")).toBeVisible()
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog", {name: "Log entry 2"})).toBeNull()
        )
        // The voter's logs are reachable again once the entry's popup has gone.
        await waitFor(() =>
            expect(within(document.body).getByRole("dialog", {name: "Voter logs"})).toBeVisible()
        )
        await userEvent.click(await within(logsPopup).findByText("Ballot cast"))
        await within(document.body).findByRole("dialog", {name: "Log entry 2"})
    },
}

export const WithoutActions: Story = {
    args: {showActions: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        expect(canvas.queryByRole("button", {name: "Export"})).toBeNull()
    },
}

export const WithoutExportPermission: Story = {
    args: {roles: ["logs-read"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        expect(canvas.queryByRole("button", {name: "Export"})).toBeNull()
    },
}

export const ExportAsCsv: Story = {
    play: async ({canvasElement}) => {
        const dialog = await exportAs(canvasElement, "CSV")
        const title = dialog.getByText("Export logs")
        await waitFor(() => expect(title).toBeVisible())
        await waitFor(() => expect(graphql.calls).toEqual(ledgerReads))
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() => expect(graphql.calls).toEqual(exported(EVENT_ID, "CSV")))
    },
}

export const CancelThePdfExport: Story = {
    play: async ({canvasElement}) => {
        const dialog = await exportAs(canvasElement, "PDF")
        const title = dialog.getByText("Export logs")
        await waitFor(() => expect(title).toBeVisible())
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        await waitFor(() => expect(graphql.calls).toEqual(ledgerReads))
    },
}

export const ExportOutsideTheEventRecord: Story = {
    args: {inEvent: false, electionEventId: EVENT_ID},
    play: async ({canvasElement}) => {
        const dialog = await exportAs(canvasElement, "CSV")
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() => expect(graphql.calls).toEqual(exported(EVENT_ID, "CSV")))
        expect(listed()?.[1]).toMatchObject({filter: {election_event_id: EVENT_ID}})
    },
}

const audits = () => graphql.calls.filter(({name}) => name === "AuditElectoralLog")

async function openAudit(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await canvas.findByText("alice")
    await userEvent.click(canvas.getByRole("button", {name: "Audit"}))
    const dialog = within(
        await within(document.body).findByRole("dialog", {name: "Audit electoral log"})
    )
    await expect(dialog.getByText(/checks every log entry against its Merkle log/)).toBeVisible()
    return dialog
}

export const AuditTheLog: Story = {
    args: {taskWidgets: true},
    parameters: {widgets: ["AuditDialog"]},
    play: async ({canvasElement}) => {
        const dialog = await openAudit(canvasElement)
        await userEvent.click(dialog.getByRole("button", {name: "Audit"}))
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
        expect(audits()).toEqual([
            {
                name: "AuditElectoralLog",
                variables: {electionEventId: EVENT_ID},
                headers: {"x-hasura-role": "electoral-log-audit"},
            },
        ])
        expect(graphql.calls.map(({name}) => name)).toContain("GetTaskById")
    },
}

export const CancelTheAudit: Story = {
    args: {taskWidgets: true},
    parameters: {widgets: ["AuditDialog"]},
    play: async ({canvasElement}) => {
        const dialog = await openAudit(canvasElement)
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        expect(audits()).toEqual([])
    },
}

export const AuditRefused: Story = {
    args: {taskWidgets: true, auditFails: true},
    parameters: {widgets: ["AuditDialog"]},
    play: async ({canvasElement}) => {
        const dialog = await openAudit(canvasElement)
        await userEvent.click(dialog.getByRole("button", {name: "Audit"}))
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(audits()).toHaveLength(1)
        expect(graphql.calls.map(({name}) => name)).not.toContain("GetTaskById")
    },
}

export const WithoutAuditPermission: Story = {
    args: {roles: ["logs-read", "logs-export"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        await expect(canvas.getByRole("button", {name: "Export"})).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Audit"})).toBeNull()
    },
}
