// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ListContextProvider, RecordContextProvider, useList} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EMessageAttemptState} from "@/types/messaging"
import {LedgerStates, LedgerStatusField} from "./LedgerStatus"

interface Scenario {
    roles: string[]
    /** The message's state in the ledger now. */
    current: EMessageAttemptState
}

const MESSAGE_ID = "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a00"

// A Logs entry as windmill writes it: accepted when it was logged.
const entry = {
    id: 3,
    message: JSON.stringify({
        statement: {
            body: {
                SendCommunications: JSON.stringify({
                    channel: "viber",
                    message_id: MESSAGE_ID,
                    attempt: 1,
                    outcome: "ACCEPTED",
                }),
            },
        },
    }),
}

let graphql: ReturnType<typeof graphqlBoundary>

const Row: React.FC = () => {
    const list = useList({data: [entry]})
    return (
        <ListContextProvider value={list}>
            <LedgerStates>
                <RecordContextProvider value={entry}>
                    <LedgerStatusField />
                </RecordContextProvider>
            </LedgerStates>
        </ListContextProvider>
    )
}

const meta = {
    title: "Admin/Logs/LedgerStatusField",
    component: LedgerStates,
    args: {roles: ["logs-read", "messaging-config-read"], current: EMessageAttemptState.FAILED},
    beforeEach: async ({args}) => {
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetLedgerMessageStates: () => ({
                    data: {
                        sequent_backend_message: [
                            {
                                id: MESSAGE_ID,
                                channel: "VIBER",
                                state: args.current,
                                attempt: 1,
                                created_at: FIXED_TIME,
                                updated_at: FIXED_TIME,
                            },
                        ],
                    },
                }),
            },
            {schema: false}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider boundary={graphql} roles={roles}>
            <Row />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** Logged as accepted, failed since: the column shows the current state. */
export const CurrentStateFromTheLedger: Story = {
    parameters: {widgets: ["LedgerStatusField"]},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("Failed")).toBeVisible()
        expect(graphql.calls).toEqual([
            {
                name: "GetLedgerMessageStates",
                variables: {tenantId: STORY_IDS.tenant, ids: [MESSAGE_ID]},
                headers: {"x-hasura-role": "messaging-config-read"},
            },
        ])
    },
}

/** Without messaging-config-read: the state when logged, marked as such. */
export const StateWhenLogged: Story = {
    args: {roles: ["logs-read"]},
    play: async ({canvasElement}) => {
        const state = await within(canvasElement).findByText("Accepted")
        await expect(state).toHaveAttribute(
            "title",
            "The state when the entry was written; the current state needs the messaging-config-read permission."
        )
        expect(graphql.calls).toEqual([])
    },
}
