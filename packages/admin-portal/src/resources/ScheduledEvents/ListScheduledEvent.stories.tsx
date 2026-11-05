// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {ResourceContextProvider} from "react-admin"
import {
    formatDateTimeZone,
    formatMyTime,
    i18n,
    instantToZoned,
    type IScheduledOutcomeExplanation,
} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import ListScheduledEvents from "./ListScheduledEvent"
import {EventProcessors} from "./CreateScheduledEvent"
import {
    ELECTION_RESOURCE,
    SCHEDULED_EVENT_RESOURCE,
    VOTING_START_ID,
    scheduledElections,
    scheduledEventRecords,
    scheduledEventsProvider,
} from "./__stories__/ScheduledEventsFixture"
import {
    EVENT_RESOURCE,
    REFUSED,
    instantOf,
    lifecycleElections,
    lifecycleEvent,
    lifecycleOutcomes,
    lifecycleSnapshotEntry,
    lifecycleSchedule,
} from "./__stories__/LifecycleScheduleFixture"
import {eventRecord} from "@/__stories__/fixtures"
import {MyTimeZoneProvider, adminDateTimeFormat} from "@/components/timezones/timeZoneService"
import {
    MY_TIME_ZONE,
    madridConfiguration,
    overseasConfiguration,
    type ITimeZoneConfiguration,
} from "@/components/timezones/__fixtures__/configurations"
import {
    refusedEdited,
    runsAuthorized,
    runsNoSignatures,
    runsUnsigned,
} from "@/components/timezones/__fixtures__/explanations"
import type {IScheduledOutcomeRow} from "@/types/lifecycle"

interface Scenario {
    /** What reading the scheduled events does. */
    reads: ReadState
    /** Whether the event has scheduled events. */
    populated: boolean
    /** Whether the scheduling service fails. */
    failure: boolean
    /** The signed-in user's roles. */
    roles: string[]
    /** The event's timezones: none (UTC), the overseas preset or the Madrid association. */
    configuration: "none" | "overseas" | "madrid"
    /** Whether the event was published, and with the schedule as it is now. */
    published: boolean
    /** A row was edited after publication, a tz database update moved one, a row has no offset. */
    changes: boolean
    storedInAnotherZone?: boolean
}

const CONFIGURATIONS: Record<"overseas" | "madrid", () => ITimeZoneConfiguration> = {
    overseas: overseasConfiguration,
    madrid: madridConfiguration,
}

/** Tokyo's opening was edited after signing; the common close runs without signatures. */
const overseasOutcome = (row: {id: string; event_processor: string; election_id: string}) => {
    const tokyo = overseasConfiguration().elections[1].id
    if (row.event_processor === "END_VOTING_PERIOD") return runsUnsigned()
    return row.election_id === tokyo ? refusedEdited() : runsAuthorized()
}

const outcomesOf = (configuration: Scenario["configuration"]): Array<IScheduledOutcomeRow> =>
    configuration === "none"
        ? []
        : lifecycleOutcomes(
              CONFIGURATIONS[configuration](),
              configuration === "overseas"
                  ? overseasOutcome
                  : (): IScheduledOutcomeExplanation => runsNoSignatures()
          )

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const ALL_ROLES = [
    IPermissions.SCHEDULED_EVENT_WRITE,
    IPermissions.SCHEDULED_EVENT_CREATE,
    IPermissions.SCHEDULED_EVENT_DELETE,
    IPermissions.EE_SCHEDULED_EVENT_COLUMNS,
]

const meta = {
    title: "Admin/Scheduled events/ListScheduledEvents",
    component: ListScheduledEvents,
    args: {
        reads: "records",
        populated: true,
        failure: false,
        roles: ALL_ROLES,
        configuration: "none",
        published: false,
        changes: false,
    },
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        configuration: {control: "inline-radio", options: ["none", "overseas", "madrid"]},
    },
    parameters: {
        expectedFailure: {
            reason: "The grid has an unlabelled header cell and the rows' edit and delete actions are unnamed icon buttons.",
            a11y: ["button-name", "empty-table-header"],
        },
    },
    beforeEach: async ({args}) => {
        const configuration =
            args.configuration === "none" ? null : CONFIGURATIONS[args.configuration]()
        const schedule = configuration ? lifecycleSchedule(configuration) : scheduledEventRecords()
        if (args.storedInAnotherZone) {
            for (const row of schedule) {
                const stored = row.cron_config as {
                    scheduled_date: string
                    local?: string
                    timezone?: string
                }
                row.cron_config = {
                    ...stored,
                    timezone: MY_TIME_ZONE,
                    local: instantToZoned(stored.scheduled_date, MY_TIME_ZONE),
                }
                row.stopped_at = stored.scheduled_date
            }
        }
        if (configuration && args.changes) {
            // Edited after publication; moved by a tz database update; stored without an offset.
            schedule[0].cron_config = {...schedule[0].cron_config, local: "2028-04-09T08:00"}
            schedule[1].annotations = {
                schedule_recompute: {
                    scheduled_date: "2028-04-08T14:00:00Z",
                    previous: schedule[1].cron_config.scheduled_date,
                    local: schedule[1].cron_config.local,
                    timezone: schedule[1].cron_config.timezone,
                    checked_at: "2028-03-02T03:00:00Z",
                },
            }
            schedule[2].cron_config = {scheduled_date: "2028-04-09T00:00:00"}
        }
        data = resourceBoundary(
            {
                [SCHEDULED_EVENT_RESOURCE]: args.populated ? schedule : [],
                [ELECTION_RESOURCE]: configuration
                    ? lifecycleElections(configuration)
                    : scheduledElections(),
                [EVENT_RESOURCE]: [configuration ? lifecycleEvent(configuration) : eventRecord()],
            },
            {reads: {[SCHEDULED_EVENT_RESOURCE]: args.reads}}
        )

        graphql = graphqlBoundary(
            {
                ManageElectionDates: () =>
                    args.failure
                        ? {errors: [new GraphQLError("Synthetic scheduler unavailable")]}
                        : {data: {manage_election_dates: {error_msg: null, warnings: []}}},
                GetScheduledOutcomes: () => ({
                    data: {get_scheduled_outcomes: {outcomes: outcomesOf(args.configuration)}},
                }),
                GetLifecycleSnapshots: () => ({
                    data: {
                        get_lifecycle_snapshots: {
                            snapshots:
                                args.published && configuration
                                    ? [lifecycleSnapshotEntry(configuration)]
                                    : [],
                        },
                    },
                }),
                PreviewScheduledOutcomeChange: () => ({
                    data: {
                        preview_scheduled_outcome_change: {
                            applies: null,
                            applies_message_key: null,
                            changes: [],
                        },
                    },
                }),
                ApplyScheduleRecompute: () => ({data: {apply_schedule_recompute: {updated: 1}}}),
                ExportSchedule: () => ({
                    data: {export_schedule: {document_id: "doc-schedule"}},
                }),
                GetDocument: () => ({data: {sequent_backend_document: []}}),
                FetchDocument: () => ({data: {fetchDocument: null}}),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={scheduledEventsProvider(data.provider)}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                <ResourceContextProvider value="sequent_backend_election_event">
                    <ListScheduledEvents electionEventId={EVENT_ID} />
                </ResourceContextProvider>
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const eventRow = (canvasElement: HTMLElement, type: EventProcessors) =>
    within(canvasElement).findByRole("row", {
        name: new RegExp(i18n.t(`eventsScreen.eventType.${type}`)),
    })

const listCalls = () =>
    data.calls.filter(
        ({method, args}) => method === "getList" && args[0] === SCHEDULED_EVENT_RESOURCE
    )

const scheduled = () =>
    graphql.calls.filter(({name}) => name === "ManageElectionDates").map(({variables}) => variables)

async function drawer() {
    const element = await waitFor(() => {
        const found = document.body.querySelector<HTMLElement>(".MuiDrawer-root")
        if (!found) throw new Error("The drawer is not open")
        return found
    })
    await waitFor(() => expect(element).toBeVisible())
    return element
}

async function expectNotification(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

async function confirmTheDeletion(canvasElement: HTMLElement) {
    const buttons = within(
        await eventRow(canvasElement, EventProcessors.START_VOTING_PERIOD)
    ).getAllByRole("button")
    await userEvent.click(buttons[buttons.length - 1])
    const dialog = within(await within(document.body).findByRole("dialog"))
    await expect(dialog.getByText(i18n.t("eventsScreen.edit.delete"))).toBeVisible()
    await userEvent.click(dialog.getByRole("button", {name: i18n.t("common.label.delete")}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const start = within(await eventRow(canvasElement, EventProcessors.START_VOTING_PERIOD))
        await expect(start.getByText("Council")).toBeVisible()
        const tally = within(await eventRow(canvasElement, EventProcessors.ALLOW_TALLY))
        await expect(tally.getByText("Deputy")).toBeVisible()
        await expect(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.add")})
        ).toBeVisible()
        expect(listCalls().at(-1)?.args[1]).toMatchObject({
            filter: {election_event_id: EVENT_ID, tenant_id: TENANT_ID},
        })
        expect(scheduled()).toEqual([])
    },
}

export const Empty: Story = {
    args: {populated: false},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("eventsScreen.empty.header"))).toBeVisible()
        await userEvent.click(
            canvas.getByRole("button", {name: i18n.t("eventsScreen.empty.button")})
        )
        const form = within(await drawer())
        await expect(form.getByText(i18n.t("eventsScreen.create.title"))).toBeVisible()
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(document.body.querySelector(".MuiDrawer-root")).toBeNull())
    },
}

export const ReadOnly: Story = {
    args: {roles: []},
    parameters: {
        expectedFailure: {
            reason: "The grid has an unlabelled header cell.",
            a11y: ["empty-table-header"],
        },
    },
    play: async ({canvasElement}) => {
        const row = within(await eventRow(canvasElement, EventProcessors.START_VOTING_PERIOD))
        expect(row.queryAllByRole("button")).toEqual([])
        expect(
            within(canvasElement).queryByRole("button", {name: i18n.t("common.label.add")})
        ).toBeNull()
    },
}

export const ScheduleAnEvent: Story = {
    play: async ({canvasElement}) => {
        await eventRow(canvasElement, EventProcessors.START_VOTING_PERIOD)
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.add")})
        )
        const element = await drawer()
        const input = element.querySelector<HTMLInputElement>('input[type="datetime-local"]')
        if (!input) throw new Error("The schedule date input is missing")
        fireEvent.change(input, {target: {value: "2026-11-02T09:30"}})
        await userEvent.click(within(element).getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(scheduled()).toEqual([
                expect.objectContaining({
                    electionEventId: EVENT_ID,
                    eventProcessor: EventProcessors.START_VOTING_PERIOD,
                    // No zone configured: the event's zone is UTC.
                    scheduledDate: "2026-11-02T09:30:00Z",
                    localDateTime: "2026-11-02T09:30",
                    timeZone: "UTC",
                }),
            ])
        )
        await expectNotification(i18n.t("eventsScreen.messages.createSuccess"))
        await waitFor(() => expect(document.body.querySelector(".MuiDrawer-root")).toBeNull())
    },
}

export const EditAnEvent: Story = {
    play: async ({canvasElement}) => {
        const [edit] = within(
            await eventRow(canvasElement, EventProcessors.START_VOTING_PERIOD)
        ).getAllByRole("button")
        await userEvent.click(edit)
        const form = within(await drawer())
        await expect(form.getByText(i18n.t("eventsScreen.edit.title"))).toBeVisible()
        await waitFor(() =>
            expect(
                form.getByRole("textbox", {name: i18n.t("eventsScreen.election.label")})
            ).toHaveValue("Council")
        )
        expect(data.calls).toContainEqual({
            method: "getOne",
            args: [SCHEDULED_EVENT_RESOURCE, expect.objectContaining({id: VOTING_START_ID})],
        })
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(document.body.querySelector(".MuiDrawer-root")).toBeNull())
    },
}

export const DeleteAnEvent: Story = {
    play: async ({canvasElement}) => {
        await confirmTheDeletion(canvasElement)
        // A schedule without a date is archived.
        await waitFor(() =>
            expect(scheduled()).toEqual([
                {
                    electionEventId: EVENT_ID,
                    electionId: STORY_IDS.election,
                    eventProcessor: EventProcessors.START_VOTING_PERIOD,
                },
            ])
        )
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const DeleteFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement}) => {
        await confirmTheDeletion(canvasElement)
        await expectNotification(i18n.t("eventsScreen.messages.editError"))
        expect(scheduled()).toHaveLength(1)
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const Loading: Story = {
    parameters: {expectedFailure: null},
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(listCalls().length).toBeGreaterThan(0))
        expect(within(canvasElement).queryByRole("row", {name: /Voting Period/})).toBeNull()
    },
}

export const LoadError: Story = {
    parameters: {expectedFailure: null},
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        await expectNotification("Synthetic service unavailable")
        expect(within(canvasElement).queryByRole("row", {name: /Voting Period/})).toBeNull()
    },
}

/** The two times of a row as the screen words them, derived from the configuration. */
const zonedTexts = (local: string, zone: string) => {
    const options = {t: i18n.t, lang: "en", formatDateTime: adminDateTimeFormat("en")}
    const instant = instantOf(local, zone)
    return {
        inZone: formatDateTimeZone(instant, zone, options),
        mine: formatMyTime(instant, options, MY_TIME_ZONE),
    }
}

const rowOf = async (canvasElement: HTMLElement, name: string, type: string) => {
    const rows = await within(canvasElement).findAllByRole("row", {
        name: new RegExp(i18n.t(`eventsScreen.eventType.${type}`)),
    })
    const row = rows.find((candidate) => within(candidate).queryByText(name))
    if (!row) throw new Error(`No ${type} row for ${name}`)
    return within(row)
}

/** tz-schedule: each Post opens at 00:00 its time; the common close is event-wide. */
export const OverseasSchedule: Story = {
    args: {configuration: "overseas", published: true},
    play: async ({canvasElement}) => {
        const configuration = overseasConfiguration()
        const dubai = configuration.schedule[0]
        const row = await rowOf(canvasElement, "Dubai PCG", "START_VOTING_PERIOD")
        const times = zonedTexts(dubai.local, dubai.timezone)
        await expect(row.getByText(times.inZone)).toBeVisible()
        await expect(row.getByText(times.mine)).toBeVisible()
        await expect(row.getByText(i18n.t("scheduledOutcome.chip.runs"))).toBeVisible()
        const close = configuration.schedule.find(
            ({event_processor}) => event_processor === "END_VOTING_PERIOD"
        )!
        const closeRow = await rowOf(
            canvasElement,
            i18n.t("lifecycle.schedule.allElections"),
            "END_VOTING_PERIOD"
        )
        await expect(
            closeRow.getByText(zonedTexts(close.local, close.timezone).inZone)
        ).toBeVisible()
        await expect(closeRow.getByText(i18n.t("scheduledOutcome.chip.runsUnsigned"))).toBeVisible()
    },
}

/** Stored input zones do not replace the configured display zone of a Post or the event. */
export const ConfiguredRowZones: Story = {
    args: {configuration: "overseas", published: true, storedInAnotherZone: true},
    play: async ({canvasElement}) => {
        const configuration = overseasConfiguration()
        const dubai = configuration.schedule[0]
        const row = await rowOf(canvasElement, "Dubai PCG", "START_VOTING_PERIOD")
        const times = zonedTexts(dubai.local, dubai.timezone)
        await expect(row.getAllByText(times.inZone)).toHaveLength(2)
        await expect(row.getAllByText(times.mine)).toHaveLength(2)
        const close = configuration.schedule.find(
            ({event_processor}) => event_processor === "END_VOTING_PERIOD"
        )!
        const closeRow = await rowOf(
            canvasElement,
            i18n.t("lifecycle.schedule.allElections"),
            "END_VOTING_PERIOD"
        )
        await expect(
            closeRow.getAllByText(zonedTexts(close.local, close.timezone).inZone)
        ).toHaveLength(2)
    },
}

/** The totals banner counts the refused transitions and filters the list to them. */
export const RefusedTotals: Story = {
    args: {configuration: "overseas", published: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const banner = within(await canvas.findByTestId(`schedule-totals-${REFUSED}`))
        await expect(
            banner.getByText(
                i18n.t("lifecycle.schedule.totals.refused", {count: 1, transitions: 1})
            )
        ).toBeVisible()
        await userEvent.click(
            banner.getByRole("button", {name: i18n.t("lifecycle.schedule.totals.review")})
        )
        await waitFor(() =>
            expect(listCalls().at(-1)?.args[1]).toMatchObject({
                filter: {id: {value: {_in: [overseasConfiguration().schedule[1].id]}}},
            })
        )
        const why = await rowOf(canvasElement, "Tokyo PE", "START_VOTING_PERIOD")
        await userEvent.click(
            why.getByRole("button", {name: i18n.t("scheduledOutcome.why.button")})
        )
        const panel = within(await within(document.body).findByRole("dialog"))
        // The popover fades in.
        await waitFor(() =>
            expect(
                panel.getByText(i18n.t("scheduledOutcome.nextStep.publishAndApprove"))
            ).toBeVisible()
        )
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

/** tz-fixture: the Madrid association, one line where the row's zone is mine. */
export const MadridAssociation: Story = {
    args: {configuration: "madrid", published: true},
    play: async ({canvasElement}) => {
        const configuration = madridConfiguration()
        const canary = configuration.schedule[1]
        const row = await rowOf(
            canvasElement,
            "Council: Canary Islands office",
            "START_VOTING_PERIOD"
        )
        await expect(row.getByText(zonedTexts(canary.local, canary.timezone).inZone)).toBeVisible()
        await expect(row.getByText(i18n.t("scheduledOutcome.chip.runs"))).toBeVisible()
    },
}

/** Unpublished edits, a tz database update to apply and a time without an offset are flagged. */
export const ScheduleChanges: Story = {
    args: {configuration: "overseas", published: true, changes: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByTestId("schedule-unpublished")).toBeVisible()
        await expect(canvas.getByTestId("schedule-offsetless")).toBeVisible()
        await expect(canvas.getAllByText(i18n.t("lifecycle.schedule.noOffset"))[0]).toBeVisible()
        const recompute = within(canvas.getByTestId("schedule-recompute"))
        await userEvent.click(
            recompute.getByRole("button", {name: i18n.t("lifecycle.schedule.recompute.apply")})
        )
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name)).toContain("ApplyScheduleRecompute")
        )
        await expectNotification(i18n.t("lifecycle.schedule.recompute.applied", {count: 1}))
    },
}

/** Nothing published: voters see the schedule after the first publication. */
export const NotPublished: Story = {
    args: {configuration: "overseas", published: false},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(i18n.t("lifecycle.schedule.notPublished"))
        ).toBeVisible()
    },
}

/** A first calendar can be imported without creating a placeholder event. */
export const ImportIntoEmptySchedule: Story = {
    parameters: {expectedFailure: null},
    args: {populated: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText(i18n.t("eventsScreen.empty.header"))
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.import")}))
        const form = within(await drawer())
        await expect(form.getByText(i18n.t("lifecycle.import.title"))).toBeVisible()
        await expect(form.getByRole("button", {name: i18n.t("common.label.import")})).toBeDisabled()
        await userEvent.click(form.getByRole("button", {name: i18n.t("common.label.cancel")}))
    },
}
export const EmptyScheduleReadOnly: Story = {
    parameters: {expectedFailure: null},
    args: {populated: false, roles: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText(i18n.t("eventsScreen.empty.header"))
        await expect(canvas.queryByRole("button", {name: i18n.t("common.label.import")})).toBeNull()
        await expect(
            canvas.queryByRole("button", {name: i18n.t("eventsScreen.empty.button")})
        ).toBeNull()
    },
}
