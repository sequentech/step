// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {GraphQLError} from "graphql"
import {ResourceContextProvider} from "react-admin"
import {i18n, VotingStatusChannel} from "@sequentech/ui-core"
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

interface Scenario {
    /** Whether the drawer edits the saved voting start instead of creating an event. */
    editing: boolean
    /** How the scheduling service answers. */
    answer: "scheduled" | "rejected" | "failure"
    setIsOpenDrawer: Mock<(state: boolean) => void>
    getElectionName: Mock<(scheduledEvent: Sequent_Backend_Scheduled_Event) => string>
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
    },
    argTypes: {
        answer: {control: "inline-radio", options: ["scheduled", "rejected", "failure"]},
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            [SCHEDULED_EVENT_RESOURCE]: scheduledEventRecords(),
            [ELECTION_RESOURCE]: scheduledElections(),
        })
        graphql = graphqlBoundary(
            {
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
            <ResourceContextProvider value="sequent_backend_election_event">
                <CreateEvent
                    electionEventId={EVENT_ID}
                    setIsOpenDrawer={setIsOpenDrawer}
                    isEditEvent={editing}
                    selectedEventId={editing ? VOTING_START_ID : undefined}
                    getElectionName={getElectionName}
                />
            </ResourceContextProvider>
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
        expect(graphql.calls).toEqual([])
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
                    scheduledDate: new Date(LOCAL_DATE).toISOString(),
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
        expect(graphql.calls).toEqual([])
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
                    scheduledDate: new Date(LOCAL_DATE).toISOString(),
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
                    scheduledDate: VOTING_START_DATE,
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
        expect(args.setIsOpenDrawer).toHaveBeenCalledWith(false)
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
