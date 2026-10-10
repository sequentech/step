// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EMessageChannel} from "@/types/messaging"
import {ESendsFrom, IAudienceChannel} from "@/queries/GetMessagingAudiencePreview"
import {ERecountOutcome, IVoterCountStatus} from "@/queries/GetMessagingVoterCounts"
import {SendTemplateAudience} from "./SendTemplateAudience"

interface Scenario {
    audienceSelection: string
    voterIds?: string[]
    roles: string[]
    /** The preview's rows, or `"FAIL"` when it can't be read. */
    preview: IAudienceChannel[] | "FAIL"
    /** When the counts were counted. */
    status?: IVoterCountStatus
}

/** The event's voters were counted in the background this morning. */
const COUNTED: IVoterCountStatus = {
    computed_at: "2026-10-09T08:00:00Z",
    pending: false,
    outcome: ERecountOutcome.COUNTED,
}

/** Voters a notice is handed to a sender for, as the backend counts them. */
const willSend = (rows: IAudienceChannel[]) =>
    rows
        .filter(
            (row) =>
                row.sends_from === ESendsFrom.ACCOUNT || row.sends_from === ESendsFrom.DEPLOYMENT
        )
        .reduce((sum, row) => sum + row.voters - row.not_offered, 0)

/**
 * A second organization's event: voters on WhatsApp (some at a Post that
 * doesn't offer it for notices), on Viber with no account, on SMS through the
 * deployment's sender, on Email, and some with no contact channel.
 */
const EVERY_CASE: IAudienceChannel[] = [
    {
        channel: EMessageChannel.WHATSAPP,
        voters: 1240,
        not_offered: 40,
        sends_from: ESendsFrom.ACCOUNT,
        account_id: storyId(9, 30),
        account_name: "Staff Association WhatsApp",
    },
    {channel: EMessageChannel.VIBER, voters: 12, not_offered: 0, sends_from: ESendsFrom.NONE},
    {channel: EMessageChannel.SMS, voters: 300, not_offered: 0, sends_from: ESendsFrom.DEPLOYMENT},
    {
        channel: EMessageChannel.EMAIL,
        voters: 900,
        not_offered: 0,
        sends_from: ESendsFrom.ACCOUNT,
        account_id: storyId(9, 31),
        account_name: "Staff Association email",
    },
    {channel: null, voters: 7, not_offered: 0},
]

let graphql: ReturnType<typeof graphqlBoundary>

const rowChannels = (canvasElement: HTMLElement) =>
    within(within(canvasElement).getByRole("table", {name: "Each voter's channel"}))
        .getAllByRole("row")
        .slice(1)
        .map((row) => row.getAttribute("data-channel"))

const meta = {
    title: "Admin/User/SendTemplateAudience",
    component: SendTemplateAudience,
    args: {
        audienceSelection: "ALL_USERS",
        roles: [IPermissions.MESSAGING_CONFIG_READ],
        preview: EVERY_CASE,
    },
    beforeEach: async ({args}) => {
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetMessagingAudiencePreview: () => {
                    if (args.preview === "FAIL") throw new Error("Synthetic preview failure")
                    return {
                        data: {
                            messaging_audience_preview: {
                                total: args.preview.reduce((sum, row) => sum + row.voters, 0),
                                will_send: willSend(args.preview),
                                channels: args.preview,
                                status: args.status ?? COUNTED,
                            },
                        },
                    }
                },
                RecountMessagingVoters: () => ({
                    data: {
                        messaging_recount_voters: {
                            requested: true,
                            status: {...(args.status ?? COUNTED), pending: true},
                        },
                    },
                }),
            },
            {schema: false}
        )
        await graphql.ready
    },
    render: ({audienceSelection, voterIds, roles}) => (
        <AdminStoryProvider boundary={graphql} roles={roles}>
            <SendTemplateAudience
                electionEventId={EVENT_ID}
                audienceSelection={audienceSelection}
                voterIds={voterIds}
                channels={[EMessageChannel.SMS, EMessageChannel.EMAIL]}
                sendsFrom={(channel) =>
                    channel === EMessageChannel.SMS ? "Configured SMS" : "Configured email"
                }
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** Each row says who sends, and what happens to voters a channel can't reach. */
export const EveryVoterOnTheirChannel: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Voters in this audience: 2,459")).toBeVisible()
        // Not offered (40), Viber without an account (12) and no channel (7) get nothing.
        expect(canvas.getByText("Will get the notification: 2,400")).toBeVisible()
        expect(canvas.getByText(/^Counted at /)).toBeVisible()
        expect(rowChannels(canvasElement)).toEqual([
            EMessageChannel.WHATSAPP,
            EMessageChannel.VIBER,
            EMessageChannel.SMS,
            EMessageChannel.EMAIL,
            "NONE",
        ])
        expect(
            canvas.getByText("40 of them don't get it: Channel not offered for notices.")
        ).toBeVisible()
        expect(canvas.getByText(/notices fail: No sending account for the channel/)).toBeVisible()
        expect(canvas.getByText("The deployment's default sender")).toBeVisible()
        expect(canvas.getByText("Not sent")).toBeVisible()
        expect(canvas.getByText(/They get nothing/)).toBeVisible()
        expect(graphql.calls[0]).toMatchObject({
            name: "GetMessagingAudiencePreview",
            variables: {electionEventId: EVENT_ID, audienceSelection: "ALL_USERS"},
            headers: {"x-hasura-role": IPermissions.MESSAGING_CONFIG_READ},
        })
        expect(canvasElement.textContent).not.toMatch(/COMELEC/i)
    },
}

/** Selected voters are counted by their ids. */
export const SelectedVoters: Story = {
    args: {
        audienceSelection: "SELECTED",
        voterIds: [STORY_IDS.user, STORY_IDS.secondUser],
        preview: [
            {
                channel: EMessageChannel.SMS,
                voters: 2,
                not_offered: 0,
                sends_from: ESendsFrom.DEPLOYMENT,
            },
        ],
    },
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("Voters in this audience: 2")
        ).toBeVisible()
        expect(graphql.calls[0]?.variables).toEqual({
            electionEventId: EVENT_ID,
            audienceSelection: "SELECTED",
            voterIds: [STORY_IDS.user, STORY_IDS.secondUser],
        })
    },
}

/** Without the counts, the channels show with their sender, and the page says why. */
export const CountsCouldNotBeLoaded: Story = {
    args: {preview: "FAIL"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(/The voter counts could not be loaded/)).toBeVisible()
        expect(rowChannels(canvasElement)).toEqual([EMessageChannel.SMS, EMessageChannel.EMAIL])
        expect(canvas.getAllByRole("cell", {name: "–"})).toHaveLength(2)
        expect(canvas.getByRole("cell", {name: "Configured SMS"})).toBeVisible()
    },
}

/** Without the read permission nothing is asked; the channels show without counts. */
export const WithoutTheReadPermission: Story = {
    args: {roles: []},
    play: async ({canvasElement}) => {
        expect(rowChannels(canvasElement)).toEqual([EMessageChannel.SMS, EMessageChannel.EMAIL])
        expect(graphql.calls).toEqual([])
    },
}

/**
 * A channel none of whose voters may get the notice (their Posts don't offer
 * it) reads "Not sent", with no sender, and isn't counted as sent.
 */
export const ChannelNoVoterCanGet: Story = {
    args: {
        preview: [
            {
                channel: EMessageChannel.WHATSAPP,
                voters: 12000,
                not_offered: 0,
                sends_from: ESendsFrom.ACCOUNT,
                account_id: storyId(9, 30),
                account_name: "Staff Association WhatsApp",
            },
            {channel: EMessageChannel.EMAIL, voters: 40000, not_offered: 40000},
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Voters in this audience: 52,000")).toBeVisible()
        expect(canvas.getByText("Will get the notification: 12,000")).toBeVisible()
        const email = canvasElement.querySelector('tr[data-channel="EMAIL"]') as HTMLElement
        expect(within(email).getByText("Not sent")).toBeVisible()
        expect(
            within(email).getByText(
                "None of these voters' Posts offer this channel for notices, so none of them gets it: Channel not offered for notices."
            )
        ).toBeVisible()
        expect(within(email).getByRole("cell", {name: "40,000"})).toBeVisible()
        expect(within(email).queryByText("The deployment's default sender")).toBeNull()
        expect(within(email).queryByText(/of them don't get it/)).toBeNull()
    },
}

/** Recount asks for the event's voters to be counted again, and says it's on its way. */
export const RecountTheEventsVoters: Story = {
    args: {
        status: {
            computed_at: "2026-10-01T08:00:00Z",
            pending: false,
            outcome: ERecountOutcome.TIMED_OUT,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(/The last recount took too long and was stopped\./)
        ).toBeVisible()
        expect(canvas.getByText(/The counts shown are from the count before it\./)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Recount"}))
        await waitFor(() =>
            expect(graphql.calls.map((call) => call.name)).toContain("RecountMessagingVoters")
        )
        expect(graphql.calls.find((call) => call.name === "RecountMessagingVoters")).toMatchObject({
            variables: {electionEventId: EVENT_ID},
            headers: {"x-hasura-role": IPermissions.MESSAGING_CONFIG_READ},
        })
    },
}

/** Selected voters taking too long to count: no counts, and the send still goes out. */
export const SelectedVotersTookTooLong: Story = {
    args: {
        audienceSelection: "SELECTED",
        voterIds: [STORY_IDS.user],
        preview: [],
        status: {computed_at: null, pending: false, outcome: ERecountOutcome.TIMED_OUT},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(/Counting the selected voters took too long/)
        ).toBeVisible()
        expect(rowChannels(canvasElement)).toEqual([EMessageChannel.SMS, EMessageChannel.EMAIL])
        expect(canvas.queryByRole("button", {name: "Recount"})).toBeNull()
    },
}

/** The SMS account the event names was deleted: shown as deleted, never as the deployment's sender. */
export const DeletedConfiguredAccount: Story = {
    args: {
        preview: [
            {
                channel: EMessageChannel.SMS,
                voters: 300,
                not_offered: 0,
                sends_from: ESendsFrom.ACCOUNT_MISSING,
                account_id: storyId(9, 39),
            },
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const sms = within(
            (await canvas.findByText("Deleted account")).closest("tr") as HTMLElement
        )
        expect(sms.getByText(/notices fail: No sending account for the channel/)).toBeVisible()
        expect(sms.queryByText("The deployment's default sender")).toBeNull()
        expect(canvas.getByText("Will get the notification: 0")).toBeVisible()
    },
}
