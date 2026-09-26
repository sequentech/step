// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, eventRecord} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import Notifications from "./Notifications"
import {NOTIFICATION_RESOURCE, notificationRecords} from "./__stories__/NotificationsFixture"

interface Scenario {
    /** What reading the notifications does. */
    reads: ReadState
    /** Whether the event has notifications. */
    populated: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Notifications/Notifications",
    component: Notifications,
    args: {reads: "records", populated: true},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The grid's row checkboxes are unlabelled.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[NOTIFICATION_RESOURCE]: args.populated ? notificationRecords() : []},
            {reads: args.reads}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    // The event's page renders the list under the event's record.
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <RecordContextProvider value={eventRecord()}>
                <Notifications electionEventId={EVENT_ID} />
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const rows = await within(canvasElement).findAllByRole("row")
        await expect(
            within(rows[1]).getByText(new Date(FIXED_TIME).toLocaleString())
        ).toBeVisible()
        expect(within(canvasElement).queryByText("Invalid Date")).toBeNull()
        expect(data.calls).toEqual([
            {
                method: "getList",
                args: [
                    NOTIFICATION_RESOURCE,
                    expect.objectContaining({
                        filter: {election_event_id: EVENT_ID, tenant_id: TENANT_ID},
                    }),
                ],
            },
        ])
    },
}

export const Empty: Story = {
    args: {populated: false},
    parameters: {
        expectedFailure: {
            reason: "The create button nests an icon button inside it.",
            a11y: ["nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("eventsScreen.empty.header"))).toBeVisible()
        // The create button has nothing to open yet.
        await userEvent.click(
            canvas.getByRole("button", {name: i18n.t("eventsScreen.empty.button")})
        )
        expect(within(document.body).queryByRole("presentation")).toBeNull()
        expect(data.writes).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getList"))
        expect(within(canvasElement).queryByText("Invalid Date")).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByText("Invalid Date")).toBeNull()
    },
}
