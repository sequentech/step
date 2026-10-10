// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EMessageChannel, EMessageAttemptState, ILedgerMessage} from "@/types/messaging"
import {messengerAccount} from "@/resources/Settings/__stories__/MessagingFixture"
import {VoterMessagingSummary} from "./VoterMessagingSummary"

interface Scenario {
    attributes: Record<string, string[]>
    email?: string
    roles: string[]
    failure: ILedgerMessage | null
    /** Later attempts of the same messages. */
    attempts?: ILedgerMessage[]
}

const CONSENT = JSON.stringify({
    at: FIXED_TIME,
    version: "1",
    organization: "Staff Association",
    channel: "WHATSAPP",
})

const FAILED: ILedgerMessage = {
    id: "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a00",
    channel: EMessageChannel.WHATSAPP,
    state: EMessageAttemptState.FAILED,
    attempt: 3,
    logical_key: "bulk:reminder-1:voter-1",
    error: "Channel not offered for notices",
    created_at: FIXED_TIME,
    updated_at: FIXED_TIME,
    failed_at: FIXED_TIME,
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/User/VoterMessagingSummary",
    component: VoterMessagingSummary,
    args: {
        attributes: {
            "sequent.read-only.message-channel": ["WHATSAPP"],
            "sequent.read-only.message-number": ["+34600111214"],
            "sequent.read-only.message-consent": [CONSENT],
        },
        email: "voter@staff-association.admin-story.invalid",
        roles: [IPermissions.MESSAGING_CONFIG_READ],
        failure: FAILED,
    },
    argTypes: {attributes: {control: "object"}},
    beforeEach: async ({args}) => {
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetVoterUnsettledMessages: () => ({
                    data: {sequent_backend_message: args.failure ? [args.failure] : []},
                }),
                GetVoterMessageAttempts: () => ({
                    data: {
                        sequent_backend_message: [
                            ...(args.failure ? [args.failure] : []),
                            ...(args.attempts ?? []),
                        ],
                    },
                }),
                GetMessagingAccountSummaries: () => ({
                    data: {sequent_backend_messaging_account: [messengerAccount()]},
                }),
            },
            {schema: false}
        )
        await graphql.ready
    },
    render: ({attributes, email, roles}) => (
        <AdminStoryProvider boundary={graphql} roles={roles}>
            <VoterMessagingSummary
                attributes={attributes}
                email={email}
                userId={STORY_IDS.user}
                electionEventId={EVENT_ID}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const WhatsAppWithALastFailure: Story = {
    parameters: {widgets: ["Row"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("WhatsApp")).toBeVisible()
        // Masked: the country code and the last 3 digits only.
        await expect(canvas.getByText("+34 ••• ••• 214")).toBeVisible()
        expect(canvas.queryByText(/600111214/)).toBeNull()
        await expect(
            await canvas.findByText(/^Failed · .* · Channel not offered for notices$/)
        ).toBeVisible()
        expect(canvas.queryByText(/Preferred channel|Verified channels/)).toBeNull()
        expect(canvasElement.textContent).not.toMatch(/COMELEC/i)
        expect(graphql.calls[0]).toMatchObject({
            name: "GetVoterUnsettledMessages",
            variables: {voterId: STORY_IDS.user, electionEventId: EVENT_ID},
            headers: {"x-hasura-role": IPermissions.MESSAGING_CONFIG_READ},
        })
    },
}

export const ConnectedToMessenger: Story = {
    args: {
        attributes: {
            "sequent.read-only.message-channel": ["MESSENGER"],
            "sequent.read-only.messenger-id": ["7304419925518842"],
            "sequent.read-only.messenger-page": ["118204557331906"],
        },
        failure: null,
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Council Elections")).toBeVisible()
        await expect((await canvas.findAllByText("None"))[0]).toBeVisible()
        await expect(canvas.getByText("Not set")).toBeVisible()
    },
}

export const FirstReleaseRecord: Story = {
    args: {
        attributes: {
            "sequent.read-only.message-channel": ["SMS"],
            "sequent.read-only.verified-channels": ["VIBER", "EMAIL"],
            "sequent.read-only.viber-number": ["+971552345678"],
        },
        failure: null,
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Viber")).toBeVisible()
        await expect(canvas.getByText("+971 ••• ••• 678")).toBeVisible()
    },
}

export const LegacyVoterByEmail: Story = {
    args: {attributes: {}, failure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Email")).toBeVisible()
        await expect(
            canvas.getByText("vo••••••••@staff-association.admin-story.invalid")
        ).toBeVisible()
    },
}

// R7-05: the backend's rule. A recorded channel without its number is no
// channel: the email on record isn't used instead.
export const RecordedChannelWithoutItsNumber: Story = {
    args: {
        attributes: {"sequent.read-only.message-channel": [" whatsapp "]},
        failure: null,
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("No contact on record")).toBeVisible()
        expect(canvas.queryByText("Email")).toBeNull()
        expect(canvas.queryByText(/@staff-association/)).toBeNull()
    },
}

export const WithoutTheReadPermission: Story = {
    args: {roles: []},
    play: async ({canvasElement}) => {
        // Both the last failure and the unconfirmed message need it.
        const rows = within(canvasElement).getAllByText(
            "Shown with the messaging-config-read permission."
        )
        expect(rows).toHaveLength(2)
        await expect(rows[0]).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

export const NoContact: Story = {
    args: {attributes: {}, email: "", failure: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No contact on record")).toBeVisible()
    },
}

export const RetriedThenDelivered: Story = {
    args: {
        attempts: [
            {
                ...FAILED,
                id: "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a01",
                attempt: 4,
                state: EMessageAttemptState.DELIVERED,
                error: null,
            },
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // A later attempt of the same message delivered it: no failure is left.
        await expect((await canvas.findAllByText("None")).length).toBe(2)
        expect(canvas.queryByText(/^Failed/)).toBeNull()
    },
}

export const CodeNotConfirmed: Story = {
    args: {
        failure: {
            ...FAILED,
            state: EMessageAttemptState.UNKNOWN,
            attempt: 1,
            logical_key: "otp:session-1",
            error: null,
            // Recent: Unknown for more than 24 hours shows as Unresolved. The
            // time shown is `failed_at`, which stays fixed.
            created_at: new Date().toISOString(),
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // Unknown is never a failure: it shows under Delivery not confirmed.
        await expect(await canvas.findByText(/^Unknown · /)).toBeVisible()
        await expect(canvas.getByText("None")).toBeVisible()
    },
}
