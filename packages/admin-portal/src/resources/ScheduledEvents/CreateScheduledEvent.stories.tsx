// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {GraphQLError} from "graphql"
import {ResourceContextProvider} from "react-admin"
import {
    formatMyTime,
    formatPlaceTime,
    i18n,
    timeZonePrimaryOptionLabel,
    timeZoneOption,
    zonedTimeNote,
    VotingStatusChannel,
} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Scheduled_Event} from "@/gql/graphql"
import CreateEvent, {EventProcessors} from "./CreateScheduledEvent"
import {
    ELECTION_RESOURCE,
    SCHEDULED_EVENT_RESOURCE,
    VOTING_START_DATE,
    VOTING_START_ID,
    scheduledElections,
    scheduledEventRecords,
} from "./__stories__/ScheduledEventsFixture"
import {
    EVENT_RESOURCE,
    instantOf,
    lifecycleElections,
    lifecycleEvent,
    lifecycleSchedule,
} from "./__stories__/LifecycleScheduleFixture"
import {eventRecord} from "@/__stories__/fixtures"
import {MyTimeZoneProvider, adminDateTimeFormat} from "@/components/timezones/timeZoneService"
import {
    MY_TIME_ZONE,
    overseasConfiguration,
} from "@/components/timezones/__fixtures__/configurations"
import {refusedEdited, runsAuthorized} from "@/components/timezones/__fixtures__/explanations"

interface Scenario {
    /** Whether the drawer edits the saved voting start instead of creating an event. */
    editing: boolean
    /** How the scheduling service answers. */
    answer: "scheduled" | "rejected" | "failure"
    setIsOpenDrawer: Mock<(state: boolean) => void>
    getElectionName: Mock<(scheduledEvent: Sequent_Backend_Scheduled_Event) => string>
    /** The event's timezones: none (UTC) or the overseas preset. */
    configuration: "none" | "overseas"
    /** What saving does to the transition's outcome. */
    outcomeChange: "none" | "refused"
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Scheduled events/CreateEvent",
    component: CreateEvent,
    args: {
        editing: false,
        answer: "scheduled",
        setIsOpenDrawer: fn(),
        getElectionName: fn(() => "Council"),
        configuration: "none",
        outcomeChange: "none",
    },
    argTypes: {
        answer: {control: "inline-radio", options: ["scheduled", "rejected", "failure"]},
        configuration: {control: "inline-radio", options: ["none", "overseas"]},
        outcomeChange: {control: "inline-radio", options: ["none", "refused"]},
    },
    beforeEach: async ({args}) => {
        const overseas = args.configuration === "overseas" ? overseasConfiguration() : null
        data = resourceBoundary({
            [SCHEDULED_EVENT_RESOURCE]: overseas
                ? lifecycleSchedule(overseas)
                : scheduledEventRecords(),
            [ELECTION_RESOURCE]: overseas ? lifecycleElections(overseas) : scheduledElections(),
            [EVENT_RESOURCE]: [overseas ? lifecycleEvent(overseas) : eventRecord()],
        })

        graphql = graphqlBoundary(
            {
                PreviewScheduledOutcomeChange: () => ({
                    data: {
                        preview_scheduled_outcome_change: {
                            applies: null,
                            applies_message_key: null,
                            changes:
                                args.outcomeChange === "refused"
                                    ? [
                                          {
                                              scheduled_event_id: "new",
                                              election_id: null,
                                              before: runsAuthorized(),
                                              after: refusedEdited(),
                                          },
                                      ]
                                    : [],
                        },
                    },
                }),
                ManageElectionDates: () =>
                    args.answer === "failure"
                        ? {errors: [new GraphQLError("Synthetic scheduler unavailable")]}
                        : {
                              data: {
                                  manage_election_dates: {
                                      error_msg:
                                          args.answer === "rejected"
                                              ? "Synthetic schedule conflict"
                                              : null,
                                  },
                              },
                          },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({editing, setIsOpenDrawer, getElectionName}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            auth={{tenantId: TENANT_ID}}
        >
            {/* As the election event's scheduled events tab renders the drawer. */}
            <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                <ResourceContextProvider value="sequent_backend_election_event">
                    <CreateEvent
                        electionEventId={EVENT_ID}
                        setIsOpenDrawer={setIsOpenDrawer}
                        isEditEvent={editing}
                        selectedEventId={editing ? VOTING_START_ID : undefined}
                        getElectionName={getElectionName}
                    />
                </ResourceContextProvider>
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const LOCAL_DATE = "2026-11-02T09:30"

const channel = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).getByRole("checkbox", {name: i18n.t(`common.channel.${name}`)})

const typeSelect = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("combobox", {name: i18n.t("eventsScreen.eventType.label")})

async function chooseType(canvasElement: HTMLElement, type: EventProcessors) {
    await userEvent.click(await typeSelect(canvasElement))
    await userEvent.click(
        await within(document.body).findByRole("option", {
            name: i18n.t(`eventsScreen.eventType.${type}`),
        })
    )
}

function setDate(canvasElement: HTMLElement) {
    const input = canvasElement.querySelector<HTMLInputElement>('input[type="datetime-local"]')
    if (!input) throw new Error("The schedule date input is missing")
    fireEvent.change(input, {target: {value: LOCAL_DATE}})
}

const save = (canvasElement: HTMLElement) =>
    userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))

const scheduled = () =>
    graphql.calls.filter(({name}) => name === "ManageElectionDates").map(({variables}) => variables)

async function expectNotification(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

export const Populated: Story = {
    parameters: {widgets: ["VotingChannelsInput"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("eventsScreen.create.title"))).toBeVisible()
        await expect(await typeSelect(canvasElement)).toHaveTextContent(
            i18n.t("eventsScreen.eventType.START_VOTING_PERIOD")
        )
        await expect(channel(canvasElement, "online")).toBeChecked()
        await expect(channel(canvasElement, "kiosk")).toBeChecked()
        await expect(channel(canvasElement, "early_voting")).not.toBeChecked()
        expect(scheduled()).toEqual([])
    },
}

export const ScheduleTheVotingStart: Story = {
    play: async ({canvasElement, args}) => {
        await typeSelect(canvasElement)
        // A single remaining channel cannot be unchecked.
        await userEvent.click(channel(canvasElement, "kiosk"))
        await expect(channel(canvasElement, "online")).toBeDisabled()
        setDate(canvasElement)
        await save(canvasElement)
        await waitFor(() =>
            expect(scheduled()).toEqual([
                {
                    electionEventId: EVENT_ID,
                    electionId: null,
                    // No zone configured: the event's zone is UTC.
                    scheduledDate: `${LOCAL_DATE}:00Z`,
                    localDateTime: LOCAL_DATE,
                    timeZone: "UTC",
                    eventProcessor: EventProcessors.START_VOTING_PERIOD,
                    votingChannels: [VotingStatusChannel.Online],
                },
            ])
        )
        await expectNotification(i18n.t("eventsScreen.messages.createSuccess"))
        expect(args.setIsOpenDrawer).toHaveBeenCalledWith(false)
    },
}

export const RejectOnlineWithEarlyVoting: Story = {
    play: async ({canvasElement}) => {
        await typeSelect(canvasElement)
        await userEvent.click(channel(canvasElement, "early_voting"))
        await expect(
            within(canvasElement).getByText(i18n.t("eventsScreen.messages.onlineWithEarlyVoting"))
        ).toBeVisible()
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
        expect(scheduled()).toEqual([])
    },
}

export const ScheduleTheTallyOfAnElection: Story = {
    play: async ({canvasElement, args}) => {
        await chooseType(canvasElement, EventProcessors.ALLOW_TALLY)
        // Only voting periods open or close channels.
        expect(within(canvasElement).queryByRole("checkbox")).toBeNull()
        const election = within(canvasElement).getByRole("combobox", {
            name: new RegExp(i18n.t("eventsScreen.election.label")),
        })
        await userEvent.type(election, "Deputy")
        const options = () => within(document.body).queryAllByRole("option")
        await waitFor(() =>
            expect(options().map(({textContent}) => textContent)).toEqual(["Deputy"])
        )
        await userEvent.click(options()[0])
        setDate(canvasElement)
        await save(canvasElement)
        await waitFor(() =>
            expect(scheduled()).toEqual([
                {
                    electionEventId: EVENT_ID,
                    electionId: STORY_IDS.secondElection,
                    scheduledDate: `${LOCAL_DATE}:00Z`,
                    localDateTime: LOCAL_DATE,
                    timeZone: "UTC",
                    eventProcessor: EventProcessors.ALLOW_TALLY,
                },
            ])
        )
        expect(args.setIsOpenDrawer).toHaveBeenCalledWith(false)
    },
}

export const EditAnEvent: Story = {
    args: {editing: true},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("eventsScreen.edit.title"))).toBeVisible()
        // The saved event's channels replace the defaults.
        await waitFor(() => expect(channel(canvasElement, "kiosk")).not.toBeChecked())
        await expect(channel(canvasElement, "online")).toBeChecked()
        await expect(
            canvas.getByRole("textbox", {name: i18n.t("eventsScreen.election.label")})
        ).toHaveValue("Council")
        expect(args.getElectionName).toHaveBeenCalledWith(
            expect.objectContaining({id: VOTING_START_ID})
        )
        await expect(await typeSelect(canvasElement)).toHaveAttribute("aria-disabled", "true")
        await userEvent.click(channel(canvasElement, "kiosk"))
        await save(canvasElement)
        await waitFor(() =>
            expect(scheduled()).toEqual([
                {
                    electionEventId: EVENT_ID,
                    electionId: STORY_IDS.election,
                    // The time wasn't edited: saved as stored, so the row keeps its fingerprint.
                    scheduledDate: VOTING_START_DATE,
                    localDateTime: null,
                    timeZone: null,
                    eventProcessor: EventProcessors.START_VOTING_PERIOD,
                    votingChannels: [VotingStatusChannel.Online, VotingStatusChannel.Kiosk],
                },
            ])
        )
        await expectNotification(i18n.t("eventsScreen.messages.editSuccess"))
    },
}

export const SchedulerRejects: Story = {
    args: {answer: "rejected"},
    play: async ({canvasElement, args}) => {
        await typeSelect(canvasElement)
        setDate(canvasElement)
        await save(canvasElement)
        await expectNotification(i18n.t("eventsScreen.messages.createError"))
        expect(scheduled()).toHaveLength(1)
        // The drawer stays open with the chosen date.
        expect(args.setIsOpenDrawer).not.toHaveBeenCalled()
        expect(
            canvasElement.querySelector<HTMLInputElement>('input[type="datetime-local"]')
        ).toHaveValue(LOCAL_DATE)
    },
}

export const SchedulerFailure: Story = {
    args: {answer: "failure"},
    play: async ({canvasElement, args}) => {
        await typeSelect(canvasElement)
        setDate(canvasElement)
        await save(canvasElement)
        await expectNotification("Synthetic scheduler unavailable")
        expect(scheduled()).toHaveLength(1)
        // The drawer stays open to retry.
        expect(args.setIsOpenDrawer).not.toHaveBeenCalled()
    },
}

const TEXT = {t: i18n.t, lang: "en", formatDateTime: adminDateTimeFormat("en")}

async function chooseElection(canvasElement: HTMLElement, name: string) {
    const election = within(canvasElement).getByRole("combobox", {
        name: new RegExp(i18n.t("eventsScreen.election.label")),
    })
    await userEvent.type(election, name)
    const option = await within(document.body).findByRole("option", {name})
    await userEvent.click(option)
}

const timeZoneField = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("combobox", {name: i18n.t("lifecycle.input.timezone")})

/** tz-schedule-create: the time is entered in the Post's zone, with a preview in my time. */
export const CreateInThePostsZone: Story = {
    args: {configuration: "overseas"},
    play: async ({canvasElement, args}) => {
        const dubai = overseasConfiguration().elections[0]
        await typeSelect(canvasElement)
        await chooseElection(canvasElement, dubai.name)
        const zone = dubai.timezone!
        await waitFor(() =>
            expect(timeZoneField(canvasElement)).toHaveValue(
                timeZoneOption(zone, TEXT, new Date()).label
            )
        )
        const input = canvasElement.querySelector<HTMLInputElement>('input[type="datetime-local"]')
        fireEvent.change(input!, {target: {value: "2028-04-09T00:00"}})
        const instant = instantOf("2028-04-09T00:00", zone)
        const preview = within(await within(canvasElement).findByTestId("zoned-preview"))
        await expect(
            preview.getByText(formatPlaceTime(instant, zone, dubai.name, TEXT))
        ).toBeVisible()
        await expect(preview.getByText(formatMyTime(instant, TEXT, MY_TIME_ZONE))).toBeVisible()
        await save(canvasElement)
        await waitFor(() =>
            expect(scheduled()).toEqual([
                expect.objectContaining({
                    electionId: dubai.id,
                    scheduledDate: instant,
                    localDateTime: "2028-04-09T00:00",
                    timeZone: zone,
                }),
            ])
        )
        expect(args.setIsOpenDrawer).toHaveBeenCalledWith(false)
    },
}

/** tz-zone-search: typing finds zones by city; the event's primary is marked. */
export const SearchATimezone: Story = {
    args: {configuration: "overseas"},
    play: async ({canvasElement}) => {
        await typeSelect(canvasElement)
        const field = timeZoneField(canvasElement)
        await userEvent.clear(field)
        await userEvent.type(field, "Manila")
        const primary = overseasConfiguration().presentation.timezones!.primary
        const label = timeZonePrimaryOptionLabel(timeZoneOption(primary, TEXT), TEXT)
        await userEvent.click(
            await within(document.body).findByRole("option", {
                name: new RegExp(label.replace(/[()+]/g, "\\$&")),
            })
        )
        await expect(field).toHaveValue(label)
    },
}

/** A wall time that doesn't exist (DST gap) is explained and saved as the time shown. */
export const TimeInADstGap: Story = {
    args: {configuration: "overseas"},
    play: async ({canvasElement}) => {
        const toronto = overseasConfiguration().elections[7]
        await chooseType(canvasElement, EventProcessors.START_TEST_VOTING)
        await chooseElection(canvasElement, toronto.name)
        const input = canvasElement.querySelector<HTMLInputElement>('input[type="datetime-local"]')
        fireEvent.change(input!, {target: {value: "2028-03-12T02:30"}})
        await expect(
            await within(canvasElement).findByText(
                zonedTimeNote("2028-03-12T02:30", toronto.timezone!, TEXT)!
            )
        ).toBeVisible()
    },
}

/** Before saving: what the edit does to the opening's outcome. */
export const OutcomeNoticeBeforeSaving: Story = {
    args: {configuration: "overseas", outcomeChange: "refused"},
    play: async ({canvasElement}) => {
        await typeSelect(canvasElement)
        const input = canvasElement.querySelector<HTMLInputElement>('input[type="datetime-local"]')
        fireEvent.change(input!, {target: {value: "2028-04-09T00:00"}})
        const notice = within(await within(canvasElement).findByTestId("outcome-change-notice"))
        await expect(
            notice.getByText(
                i18n.t("lifecycle.schedule.outcomeChange", {
                    before: i18n.t("scheduledOutcome.chip.runs"),
                    after: i18n.t("scheduledOutcome.chip.refused"),
                })
            )
        ).toBeVisible()
        await expect(notice.getByText(i18n.t("scheduledOutcome.chip.refused"))).toBeVisible()
    },
}

/** Enrollment opens per election (optional: empty opens it for every election). */
export const EnrollmentForOneElection: Story = {
    args: {configuration: "overseas"},
    play: async ({canvasElement}) => {
        await chooseType(canvasElement, EventProcessors.START_ENROLLMENT_PERIOD)
        const election = within(canvasElement).getByRole("combobox", {
            name: new RegExp(i18n.t("eventsScreen.election.label")),
        })
        await expect(election).not.toBeRequired()
        expect(within(canvasElement).queryByRole("checkbox")).toBeNull()
    },
}
