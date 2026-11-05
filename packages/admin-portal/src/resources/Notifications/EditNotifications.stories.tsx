// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {eventRecord} from "@/__stories__/fixtures"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EditNotifications} from "./EditNotifications"
import {NOTIFICATION_RESOURCE, notificationRecords} from "./__stories__/NotificationsFixture"

interface Scenario {
    /** The signed-in user's roles. */
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Notifications/EditNotifications",
    component: EditNotifications,
    args: {roles: [IPermissions.NOTIFICATION_READ]},
    parameters: {
        expectedFailure: {
            reason: "The grid's row checkboxes are unlabelled.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: async () => {
        data = resourceBoundary({[NOTIFICATION_RESOURCE]: notificationRecords()})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            <RecordContextProvider value={eventRecord()}>
                <EditNotifications electionEventId={EVENT_ID} />
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await waitFor(() => expect(within(canvasElement).getAllByRole("row")).toHaveLength(2))
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getList", NOTIFICATION_RESOURCE],
        ])
    },
}

export const WithoutNotificationPermission: Story = {
    args: {roles: []},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        // Only the story's location announcer is left.
        expect(within(canvasElement).queryByRole("table")).toBeNull()
        expect(canvasElement.textContent).toBe("/")
        expect(data.calls).toEqual([])
    },
}
