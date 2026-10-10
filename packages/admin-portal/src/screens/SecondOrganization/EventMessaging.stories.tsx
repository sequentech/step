// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The election event's Messaging tab of a second organization: its own
// accounts, its own purposes per channel and its own Post restrictions, in
// English and in Spanish.
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
import {FIXED_TIME, storyId} from "@/__stories__/fixtures"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    CANARY_OFFICE_ID,
    CENTRAL_OFFICE_ID,
    OTHER_CLIENT,
    PENSION_BOARD_ID,
    SECOND_ORG_ACCOUNT_NAMES,
    SECOND_ORG_POSTS,
    secondOrgAccounts,
    secondOrgElections,
    secondOrgEvent,
    secondOrgEventConfig,
    secondOrgEventStats,
    secondOrgVoterCounts,
} from "@/__stories__/SecondOrganizationFixture"
import {EditElectionEventMessaging} from "@/resources/ElectionEvent/EditElectionEventMessaging"
import {IPermissions} from "@/types/keycloak"
import {EMessageChannel} from "@/types/messaging"
import {EStoryLocale} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Screens/Admin/Second organization/Event messaging",
    component: EditElectionEventMessaging,
    args: {
        roles: [
            IPermissions.MESSAGING_CONFIG_WRITE,
            IPermissions.MESSAGING_CONFIG_READ,
            IPermissions.LOGS_READ,
            IPermissions.ELECTION_EVENT_LOGS_TAB,
        ],
    },
    beforeEach: async () => {
        data = resourceBoundary({
            sequent_backend_election: secondOrgElections(),
            sequent_backend_template: [
                {id: storyId(10, 9), tenant_id: TENANT_ID, alias: "staff-voting-opens"},
            ],
        })
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetMessagingAccountSummaries: () => ({
                    data: {
                        sequent_backend_messaging_account: secondOrgAccounts().map(
                            ({credentials: _credentials, ...account}) => account
                        ),
                    },
                }),
                GetEventMessagingConfig: () => ({
                    data: {
                        sequent_backend_messaging_event_config: [
                            {
                                election_event_id: EVENT_ID,
                                config: secondOrgEventConfig(),
                                version: 1,
                                updated_at: FIXED_TIME,
                            },
                        ],
                    },
                }),
                GetEventMessagingConfigAuthor: () => ({
                    data: {
                        sequent_backend_messaging_event_config: [
                            {election_event_id: EVENT_ID, updated_by: null},
                        ],
                    },
                }),
                // The voters per channel, counted apart from the messages.
                GetMessagingVoterCounts: ({variables}) => ({
                    data: {
                        messaging_voter_counts:
                            variables.electionEventId === EVENT_ID ? secondOrgVoterCounts() : null,
                    },
                }),
                RecountMessagingVoters: () => ({
                    data: {
                        messaging_recount_voters: {
                            requested: true,
                            status: {...secondOrgVoterCounts().status, pending: true},
                        },
                    },
                }),
                GetMessagingEventStats: () => ({
                    data: {messaging_event_stats: {channels: secondOrgEventStats()}},
                }),
                GetMessagingCallbackStatus: () => ({
                    data: {messaging_callback_status: {accounts: []}},
                }),
            },
            {schema: false}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <RecordContextProvider value={secondOrgEvent()}>
                <EditElectionEventMessaging />
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const channelRow = async (canvasElement: HTMLElement, channel: EMessageChannel) => {
    await within(canvasElement).findAllByText(SECOND_ORG_ACCOUNT_NAMES.SMS)
    return canvasElement.querySelector(
        `table[aria-label="Channels"] tr[data-channel="${channel}"]`
    ) as HTMLElement
}

const purposeSwitch = (row: HTMLElement, purpose: string) =>
    row.querySelector(`input[aria-label$=": ${purpose}"]`) as HTMLInputElement

/** Which of a channel's purposes are on: its own, not the first organization's. */
const purposesOn = async (canvasElement: HTMLElement, channel: EMessageChannel) => {
    const row = await channelRow(canvasElement, channel)
    return ["OTPs", "Notices"].filter((purpose) => purposeSwitch(row, purpose).checked)
}

/** Its accounts send each channel, with its own purposes and counts. */
export const ChannelsAndPurposes: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        for (const name of Object.values(SECOND_ORG_ACCOUNT_NAMES)) {
            await expect((await canvas.findAllByText(name)).length).toBeGreaterThan(0)
        }
        await waitFor(async () =>
            expect(await purposesOn(canvasElement, EMessageChannel.WHATSAPP)).toEqual([
                "OTPs",
                "Notices",
            ])
        )
        expect(await purposesOn(canvasElement, EMessageChannel.VIBER)).toEqual(["Notices"])
        expect(await purposesOn(canvasElement, EMessageChannel.MESSENGER)).toEqual(["OTPs"])
        expect(await purposesOn(canvasElement, EMessageChannel.SMS)).toEqual(["OTPs"])
        expect(await purposesOn(canvasElement, EMessageChannel.EMAIL)).toEqual(["Notices"])
        const delivery = within(await canvas.findByRole("table", {name: "Delivery status"}))
        const whatsapp = within(
            await waitFor(() => delivery.getByText("WhatsApp").closest("tr") as HTMLElement)
        )
        await expect(whatsapp.getByText("640")).toBeVisible()
        // Its own event's voters, from the counts kept apart from the messages.
        expect(graphql.calls.find(({name}) => name === "GetMessagingVoterCounts")).toMatchObject({
            variables: {electionEventId: EVENT_ID},
        })
        expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}

/** Its Posts offer its own channels: Viber only at the Canary Islands office, Email only at the pension board. */
export const PostRestrictions: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const cell = (post: string, channel: string) =>
            canvas.findByRole("checkbox", {name: `${post}: ${channel}`})
        const central = SECOND_ORG_POSTS[CENTRAL_OFFICE_ID].en
        const canary = SECOND_ORG_POSTS[CANARY_OFFICE_ID].en
        const pension = SECOND_ORG_POSTS[PENSION_BOARD_ID].en
        await expect(await cell(central, "SMS")).toBeChecked()
        await expect(await cell(central, "Viber")).not.toBeChecked()
        await expect(await cell(canary, "Viber")).toBeChecked()
        await expect(await cell(canary, "Facebook Messenger")).toBeChecked()
        await expect(await cell(canary, "SMS")).not.toBeChecked()
        await expect(await cell(pension, "Email")).toBeChecked()
        await expect(await cell(pension, "WhatsApp")).not.toBeChecked()
        expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}

/** In Spanish, its Posts carry their Spanish names. */
export const PostRestrictionsInSpanish: Story = {
    globals: {locale: EStoryLocale.SPANISH},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        for (const id of [CENTRAL_OFFICE_ID, CANARY_OFFICE_ID, PENSION_BOARD_ID]) {
            await expect(await canvas.findByText(SECOND_ORG_POSTS[id].es)).toBeVisible()
        }
        await expect(
            await canvas.findByRole("checkbox", {
                name: `${SECOND_ORG_POSTS[CANARY_OFFICE_ID].es}: Viber`,
            })
        ).toBeChecked()
        expect(canvas.queryByText(SECOND_ORG_POSTS[CANARY_OFFICE_ID].en)).toBeNull()
        expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}
