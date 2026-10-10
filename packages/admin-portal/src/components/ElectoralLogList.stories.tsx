// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {type ReadState, resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, type StoryRecord, eventRecord} from "@/__stories__/fixtures"
import type {ElectoralLogRow} from "@/gql/graphql"
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
const SEALED = "Ballot box of Madrid Post, Spain sealed: 1,340 of 1,342 ballots counted."

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
        {SendCommunications: {channel: "WHATSAPP", purpose: "NOTICE"}}
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
}: Scenario) {
    const {permissions} = useStoryGlobals()
    const list = (
        <ElectoralLogList
            electionEventId={electionEventId}
            filterToShow={filterToShow}
            filterValue={filterValue}
            showActions={showActions}
        />
    )
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
            },
            {schema: true}
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

/** Without a range, the whole log in the event's primary zone (UTC without zones). */
const exported = (electionEventId: string, format: string) => [
    {
        name: "ExportElectionEventLogs",
        variables: {electionEventId, format, createdFrom: null, createdTo: null, timeZone: "UTC"},
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
        expect(graphql.calls).toEqual([])
    },
}

/** A seal entry's details (seal hash, counts) open with its description under Show More. */
export const DetailsUnderShowMore: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("alice")
        const preview = canvas.getByText(SEALED.slice(0, 50).trim())
        await expect(preview).toBeVisible()
        const cell = within(preview.closest("td")!)
        expect(canvas.queryByText(`Seal hash: ${SEAL_HASH}`)).toBeNull()
        // Short descriptions without details get no toggle.
        const keys = within(canvas.getByText("Keys generated").closest("td")!)
        expect(keys.queryByRole("button")).toBeNull()
        await userEvent.click(cell.getByRole("button", {name: "Show More"}))
        await expect(cell.getByText(SEALED)).toBeVisible()
        await expect(cell.getByText(`Seal hash: ${SEAL_HASH}`)).toBeVisible()
        await expect(cell.getByText("1340 of 1342 ballots counted.")).toBeVisible()
        await userEvent.click(cell.getByRole("button", {name: "Show Less"}))
        expect(canvas.queryByText(`Seal hash: ${SEAL_HASH}`)).toBeNull()
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
        expect(graphql.calls).toEqual([])
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
        expect(graphql.calls).toEqual([])
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
