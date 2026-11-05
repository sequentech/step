// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {EElectionEventLockedDown} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {eventPresentation, eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {LockdownBanner} from "./LockdownBanner"
import {readsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"

interface Scenario {
    timing: "pending" | "scheduled" | "unscheduled"
    roles: string[]
}

const event = {
    ...eventRecord(undefined, {
        presentation: {
            ...eventPresentation,
            locked_down: EElectionEventLockedDown.LOCKED_DOWN,
            timezones: {primary: "Asia/Manila", configured: ["Asia/Manila"]},
        },
    }),
    elections: [],
    elections_aggregate: {nodes: []},
}
let data: ReturnType<typeof recordsOrPending>
let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Election event/LockdownBanner",
    component: LockdownBanner,
    args: {
        timing: "scheduled",
        roles: [IPermissions.LOGS_READ, IPermissions.SCHEDULED_EVENT_WRITE],
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary({})
        await graphql.ready
        data = recordsOrPending(
            args.timing === "pending"
                ? {}
                : {
                      electoral_log: [
                          {id: 2, seconds: 1839981600, previous: true},
                          {id: 1, seconds: 1839974400, previous: false},
                      ].map(({id, seconds, previous}) => ({
                          id,
                          election_event_id: EVENT_ID,
                          statement_kind: "LockdownChanged",
                          statement_timestamp: seconds,
                          message: JSON.stringify({
                              statement: {
                                  body: {
                                      LockdownChanged: {
                                          details_json: JSON.stringify({
                                              locked_down: true,
                                              previous,
                                          }),
                                      },
                                  },
                              },
                          }),
                      })),
                      sequent_backend_scheduled_event:
                          args.timing === "scheduled"
                              ? [
                                    {
                                        id: "end-lockdown",
                                        tenant_id: TENANT_ID,
                                        election_event_id: EVENT_ID,
                                        event_processor: "END_LOCKDOWN_PERIOD",
                                        archived_at: null,
                                        stopped_at: null,
                                        cron_config: {scheduled_date: "2028-05-08T00:00:00Z"},
                                    },
                                ]
                              : [],
                  }
        )
    },
    render: (args) => (
        <AdminStoryProvider
            role={EStoryPermissions.ADMIN}
            boundary={graphql}
            dataProvider={data.provider}
            roles={args.roles}
        >
            <LockdownBanner event={event} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const ScheduledEnd: Story = {
    play: async ({canvasElement}) => {
        const banner = within(canvasElement).getByRole("status", {name: "Lockdown Status"})
        await waitFor(() => expect(banner).toHaveTextContent("End of lockdown scheduled for"))
        expect(banner).toHaveTextContent("Locked down since Apr 22, 2028, 08:00")
        expect(banner).toHaveTextContent("May 08, 2028, 08:00")
        expect(banner).toHaveTextContent("PhST")
        expect(banner).toBeVisible()
    },
}

export const NoScheduledEnd: Story = {
    args: {timing: "unscheduled"},
    play: async ({canvasElement}) => {
        const banner = within(canvasElement).getByRole("status", {name: "Lockdown Status"})
        await waitFor(() => expect(banner).toHaveTextContent("No end of lockdown is scheduled."))
        expect(banner).toHaveTextContent("Locked down since Apr 22, 2028, 08:00")
    },
}

export const LoadingTiming: Story = {
    args: {timing: "pending"},
    play: async ({canvasElement}) => {
        const banner = within(canvasElement).getByRole("status", {name: "Lockdown Status"})
        expect(banner).toHaveTextContent("This election event is in lockdown.")
        expect(banner).not.toHaveTextContent("No end of lockdown is scheduled.")
        expect(readsOf(data)).toEqual([
            "getList electoral_log",
            "getList sequent_backend_scheduled_event",
        ])
    },
}

export const WithoutTimingPermissions: Story = {
    args: {roles: [IPermissions.ADMIN_DASHBOARD_VIEW]},
    play: async ({canvasElement}) => {
        const banner = within(canvasElement).getByRole("status", {name: "Lockdown Status"})
        expect(banner).toHaveTextContent("This election event is in lockdown.")
        expect(banner).not.toHaveTextContent("No end of lockdown is scheduled.")
        expect(readsOf(data)).toEqual([])
    },
}
