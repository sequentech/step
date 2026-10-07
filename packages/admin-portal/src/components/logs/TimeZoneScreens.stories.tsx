// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// VOTE-LIFECYCLE drafts tz-logs, tz-logs-export, tz-tasks and
// tz-localization-admin: Admin Portal times in the event's zones, under the
// two configurations (a Manila primary with the logs in the primary, and a
// Madrid primary with a Canary Islands office and the logs in each
// election's zone). Expectations come from the configuration.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {
    ELogTimeZonePolicy,
    i18n,
    zoneLabel,
    zonedToInstant,
    type IElectionEventPresentation,
} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    STORY_IDS,
    electionPresentation,
    electionRecord,
    eventPresentation,
    eventRecord,
    storyId,
} from "@/__stories__/fixtures"
import {ElectoralLogList} from "@/components/ElectoralLogList"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {formatMine, formatZoned, logTimeZone} from "@/lib/timezones/zonedFormat"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

/** "My time" in the drafts: New York. Explicit, so no story depends on the machine's zone. */
const MY_ZONE = "America/New_York"

type Configuration = "primary" | "election"

interface Scenario {
    configuration: Configuration
}

const MADRID_ELECTION = STORY_IDS.election
const CANARY_ELECTION = STORY_IDS.secondElection
const DUBAI_ELECTION = storyId(3, 5)

const PRESENTATIONS: Record<Configuration, IElectionEventPresentation> = {
    primary: {
        ...eventPresentation,
        timezones: {
            configured: ["Asia/Manila", "Asia/Dubai", "Asia/Tokyo", "Africa/Cairo"],
            primary: "Asia/Manila",
            logs: ELogTimeZonePolicy.PRIMARY,
        },
    },
    election: {
        ...eventPresentation,
        timezones: {
            configured: ["Europe/Madrid", "Atlantic/Canary"],
            primary: "Europe/Madrid",
            logs: ELogTimeZonePolicy.ELECTION,
        },
    },
}

const elections = (configuration: Configuration) =>
    configuration === "primary"
        ? [
              electionRecord(undefined, {
                  id: DUBAI_ELECTION,
                  presentation: {...electionPresentation("Dubai PCG"), timezone: "Asia/Dubai"},
              }),
          ]
        : [
              electionRecord(undefined, {
                  id: MADRID_ELECTION,
                  presentation: electionPresentation("Madrid office"),
              }),
              electionRecord(undefined, {
                  id: CANARY_ELECTION,
                  presentation: {
                      ...electionPresentation("Canary office"),
                      timezone: "Atlantic/Canary",
                  },
              }),
          ]

/** 2028-04-08T22:00:03Z and the rows around it. */
const CREATED = Date.UTC(2028, 3, 8, 22, 0, 3) / 1000

const explanation = (outcome: string, deciding: string, code?: string) => ({
    outcome,
    deciding,
    checks: [
        {
            id: "covered",
            current: {
                message_key: code
                    ? "logsScreen.scheduledOutcome.outcome.runs"
                    : "logsScreen.scheduledOutcome.check.covered",
            },
            published: null,
            allows: outcome !== "refused",
        },
    ],
    next_step: {message_key: "logsScreen.scheduledOutcome.check.defaults"},
    ...(code ? {authorized_by: {request_id: "r", code, signers: []}} : {}),
})

const logRow = (
    id: number,
    created: number,
    kind: string,
    electionId: string | null,
    description: string,
    details?: unknown
) => ({
    id,
    election_event_id: EVENT_ID,
    created,
    statement_timestamp: created - 1,
    statement_kind: kind,
    user_id: "null",
    message: JSON.stringify({
        user_id: null,
        username: "system",
        election_id: electionId,
        statement: {
            head: {event_type: "SYSTEM", log_type: "INFO", description, kind},
            body: details
                ? {Signing: {kind, details_json: JSON.stringify(details)}}
                : {ElectionVotingPeriodOpen: []},
        },
    }),
})

const logRows = (configuration: Configuration) => {
    const [first, second] =
        configuration === "primary"
            ? [DUBAI_ELECTION, DUBAI_ELECTION]
            : [MADRID_ELECTION, CANARY_ELECTION]
    return [
        logRow(48214, CREATED, "ElectionVotingPeriodOpen", first, "Voting opened (scheduled)"),
        logRow(48213, CREATED - 7199, "ElectionVotingPeriodOpen", second, "Voting opened"),
        logRow(48212, CREATED - 3600, "ScheduledOutcomeChanged", second, "Scheduled close", {
            scheduled_event_id: "se-1",
            before: explanation("runs", "covered", "K7Q-2M"),
            after: explanation("refused", "covered"),
        }),
    ]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const event = (configuration: Configuration) =>
    eventRecord(undefined, {
        presentation: PRESENTATIONS[configuration],
    }) as Sequent_Backend_Election_Event

function Fixture({configuration}: Scenario) {
    const {permissions} = useStoryGlobals()
    const record = event(configuration)
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <MyTimeZoneProvider zone={MY_ZONE}>
                <RecordContextProvider value={record}>
                    <EventTimeZoneProvider event={record}>
                        <ElectoralLogList />
                    </EventTimeZoneProvider>
                </RecordContextProvider>
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Screens/Admin/VOTE-LIFECYCLE/Logs and admin times",
    component: ElectoralLogList,
    args: {configuration: "primary"},
    argTypes: {configuration: {control: "inline-radio", options: ["primary", "election"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            electoral_log: logRows(args.configuration),
            sequent_backend_election: elections(args.configuration),
        })
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
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const options = {t: i18n.t.bind(i18n), lang: "en", seconds: true}

/** The zone a log row of `electionId` shows in, from the configuration. */
const rowZone = (configuration: Configuration, electionId: string) =>
    logTimeZone(
        PRESENTATIONS[configuration],
        elections(configuration).find(({id}) => id === electionId)?.presentation
    )

async function expectLogTimes(canvasElement: HTMLElement, configuration: Configuration) {
    const canvas = within(canvasElement)
    for (const row of logRows(configuration)) {
        const electionId = JSON.parse(row.message).election_id
        const zone = rowZone(configuration, electionId)
        const created = row.created * 1000
        const cell = await canvas.findByText(formatZoned(created, zone, options))
        await waitFor(() => expect(cell).toBeVisible())
        expect(cell.textContent).toContain(zoneLabel(zone, options, new Date(created)))
        // My time below, as the draft shows it.
        expect(
            canvas.getAllByText(formatMine(created, zone, options, MY_ZONE)!).length
        ).toBeGreaterThan(0)
    }
}

/** tz-logs: Created and Statement Timestamp in the log zone and in my time; from/to/zone filter. */
export const TzLogs: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expectLogTimes(canvasElement, "primary")
        expect(canvas.getByLabelText("Created from")).toBeVisible()
        expect(canvas.getByLabelText("to")).toBeVisible()
        // The explanation of a scheduled outcome change, as readable lines.
        expect(
            canvas.getByText(
                i18n.t("logsScreen.scheduledOutcome.changed", {
                    after: i18n.t("logsScreen.scheduledOutcome.outcome.refused"),
                    before: i18n.t("logsScreen.scheduledOutcome.outcome.runs"),
                })
            )
        ).toBeVisible()

        // A datetime-local input takes its value whole, as the browser's picker sets it.
        fireEvent.change(canvas.getByLabelText("Created from"), {
            target: {value: "2028-04-08T00:00"},
        })
        // The list sends the wall time and its zone; the data provider makes it an instant.
        await waitFor(
            () =>
                expect(
                    data.calls
                        .filter(
                            ({method, args}) => method === "getList" && args[0] === "electoral_log"
                        )
                        .at(-1)?.args[1]
                ).toMatchObject({
                    filter: expect.objectContaining({
                        created_from: "2028-04-08T00:00",
                        default_time_zone: PRESENTATIONS.primary.timezones!.primary,
                    }),
                }),
            {timeout: 3000}
        )
    },
}

/** tz-logs under the ELECTION policy: each row in its election's zone. */
export const TzLogsByElection: Story = {
    args: {configuration: "election"},
    play: async ({canvasElement}) => {
        await expectLogTimes(canvasElement, "election")
    },
}

/** tz-logs-export: range, zone and CSV or PDF; the range is sent as instants in the zone. */
export const TzLogsExport: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const zone = PRESENTATIONS.primary.timezones!.primary
        await canvas.findAllByText(/my time/)
        await userEvent.click(canvas.getByRole("button", {name: "Export"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await waitFor(() => expect(dialog.getByText("Export logs")).toBeVisible())
        expect(dialog.getByRole("radio", {name: "CSV"})).toBeChecked()
        expect(dialog.getByRole("radio", {name: "PDF"})).not.toBeChecked()
        expect(dialog.queryByRole("radio", {name: "SQL"})).toBeNull()
        const abbr = zoneLabel(zone, options, new Date())
        expect(dialog.getByText(i18n.t("logsScreen.exportdialog.zoneNote", {abbr}))).toBeVisible()

        fireEvent.change(dialog.getByLabelText("From"), {target: {value: "2028-04-01T00:00"}})
        fireEvent.change(dialog.getByLabelText("To"), {target: {value: "2028-04-09T23:59"}})
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(graphql.calls).toEqual([
                {
                    name: "ExportElectionEventLogs",
                    variables: {
                        electionEventId: EVENT_ID,
                        format: "CSV",
                        createdFrom: zonedToInstant("2028-04-01T00:00", zone).instant,
                        createdTo: zonedToInstant("2028-04-09T23:59", zone).instant,
                        timeZone: zone,
                    },
                    headers: {"x-hasura-role": "logs-export"},
                },
            ])
        )
    },
}
