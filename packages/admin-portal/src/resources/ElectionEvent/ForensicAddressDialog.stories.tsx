// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EventTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {IPermissions} from "@/types/keycloak"
import {ForensicAddressDialog} from "./ForensicAddressDialog"

interface Scenario {
    state: "records" | "unavailable"
    onClose: () => void
}
let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Election event/ForensicAddressDialog",
    component: ForensicAddressDialog,
    args: {state: "records", onClose: fn()},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary(
            {
                ForensicNetworkDetail: () =>
                    args.state === "unavailable"
                        ? {errors: [new GraphQLError("Synthetic forensic store unavailable")]}
                        : {
                              data: {
                                  forensic_network_detail: {
                                      payload: {
                                          ip: "192.0.2.10",
                                          country: "ES",
                                          vote_count: 3,
                                          first_ballot: "2028-05-08T03:00:00Z",
                                          last_ballot: "2028-05-08T04:00:00Z",
                                          ballots: [
                                              {
                                                  ballot_id: "FTBE-MHRX",
                                                  pseudonym: "7QKM-2HXD",
                                                  cast_at: "2028-05-08T03:00:00Z",
                                              },
                                          ],
                                          next_cursor: null,
                                      },
                                  },
                              },
                          },
            },
            {schema: true}
        )
        await boundary.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <EventTimeZoneProvider event={eventRecord()}>
                <ForensicAddressDialog
                    electionEventId={EVENT_ID}
                    ip="192.0.2.10"
                    shared
                    onClose={args.onClose}
                />
            </EventTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const AddressDetails: Story = {
    play: async ({args}) => {
        const dialog = within(
            await within(document.body).findByRole("dialog", {name: "IP address 192.0.2.10"})
        )
        const ballot = await dialog.findByRole("cell", {name: "FTBE-MHRX"})
        await waitFor(() => expect(ballot).toBeVisible())
        expect(dialog.getByText(/Opening this address was written/)).toBeVisible()
        expect(boundary.calls).toMatchObject([
            {
                variables: {electionEventId: EVENT_ID, ip: "192.0.2.10", cursor: null, limit: 25},
                headers: {"x-hasura-role": IPermissions.FORENSIC_READ},
            },
        ])
        await userEvent.click(dialog.getByRole("button", {name: "Close"}))
        expect(args.onClose).toHaveBeenCalledOnce()
    },
}

export const Unavailable: Story = {
    args: {state: "unavailable"},
    play: async () => {
        const dialog = within(await within(document.body).findByRole("dialog"))
        const error = await dialog.findByText("The forensic records could not be loaded.")
        await waitFor(() => expect(error).toBeVisible())
        expect(dialog.queryByRole("cell", {name: "FTBE-MHRX"})).toBeNull()
    },
}
