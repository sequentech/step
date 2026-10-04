// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {ResourceContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
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

interface Scenario {
    /** What reading the scheduled events does. */
    reads: ReadState
    /** Whether the event has scheduled events. */
    populated: boolean
    /** Whether the scheduling service fails. */
    failure: boolean
    /** The signed-in user's roles. */
    roles: string[]
}

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
    args: {reads: "records", populated: true, failure: false, roles: ALL_ROLES},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The grid has an unlabelled header cell and the rows' edit and delete actions are unnamed icon buttons.",
            a11y: ["button-name", "empty-table-header"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                [SCHEDULED_EVENT_RESOURCE]: args.populated ? scheduledEventRecords() : [],
                [ELECTION_RESOURCE]: scheduledElections(),
            },
            {reads: {[SCHEDULED_EVENT_RESOURCE]: args.reads}}
        )
        graphql = graphqlBoundary(
            {
                ManageElectionDates: () =>
                    args.failure
                        ? {errors: [new GraphQLError("Synthetic scheduler unavailable")]}
                        : {data: {manage_election_dates: {error_msg: null}}},
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
            <ResourceContextProvider value="sequent_backend_election_event">
                <ListScheduledEvents electionEventId={EVENT_ID} />
            </ResourceContextProvider>
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
        expect(graphql.calls).toEqual([])
    },
}

export const Empty: Story = {
    args: {populated: false},
    parameters: {
        expectedFailure: {
            reason: "The empty state's create button nests its icon button.",
            a11y: ["nested-interactive"],
        },
    },
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
                    scheduledDate: new Date("2026-11-02T09:30").toISOString(),
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
