// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The event's Logs of a second organization: its notices on Viber and
// WhatsApp, each with its channel and its current state from the ledger.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    OTHER_CLIENT,
    SECOND_ORG_LOG_USER,
    SECOND_ORG_MESSAGE_ID,
    SECOND_ORG_VIBER_MESSAGE_ID,
    secondOrgElections,
    secondOrgEvent,
    secondOrgLedgerStates,
    secondOrgLogs,
} from "@/__stories__/SecondOrganizationFixture"
import {ElectoralLogList} from "@/components/ElectoralLogList"

interface Scenario {
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Screens/Admin/Second organization/Logs",
    component: ElectoralLogList,
    args: {roles: ["logs-read", "messaging-config-read"]},
    beforeEach: async () => {
        data = resourceBoundary({
            electoral_log: secondOrgLogs(),
            sequent_backend_election: secondOrgElections(),
        })
        // The ledger query is not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetLedgerMessageStates: () => ({
                    data: {sequent_backend_message: secondOrgLedgerStates()},
                }),
            },
            {schema: false}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <RecordContextProvider value={secondOrgEvent()}>
                <ElectoralLogList />
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** Each notice's channel, and its state now: delivered on Viber, failed on WhatsApp. */
export const MessagingStatus: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect((await canvas.findAllByText(SECOND_ORG_LOG_USER)).length).toBe(2)
        const row = async (channel: string) =>
            within((await canvas.findByText(channel)).closest("tr") as HTMLElement)
        await expect(await (await row("Viber")).findByText("Delivered")).toBeVisible()
        await expect(await (await row("WhatsApp")).findByText("Failed")).toBeVisible()
        const read = graphql.calls.find(({name}) => name === "GetLedgerMessageStates")
        expect(read?.variables).toEqual({
            tenantId: STORY_IDS.tenant,
            ids: expect.arrayContaining([SECOND_ORG_MESSAGE_ID, SECOND_ORG_VIBER_MESSAGE_ID]),
        })
        expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}
