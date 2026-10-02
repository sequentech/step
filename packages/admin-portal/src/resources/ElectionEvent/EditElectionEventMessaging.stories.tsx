// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {
    FIXED_TIME,
    STORY_IDS,
    electionPresentation,
    electionRecord,
    eventRecord,
    storyId,
} from "@/__stories__/fixtures"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    EProviderApproval,
    IEventMessagingConfig,
    IMessagingAccount,
    IMessagingConfigError,
    MESSAGE_ATTEMPT_STATES,
    MESSAGE_CHANNELS,
    MESSAGING_CONFIG_ANNOTATION,
} from "@/types/messaging"
import {EditElectionEventMessaging} from "./EditElectionEventMessaging"

type SaveResult = "saved" | "rejected" | "fails"

interface Scenario {
    /** Replaces the signed-in user's roles. */
    roles: string[]
    /** Whether the event already has a messaging configuration. */
    configured: boolean
    /** What saving answers. */
    save: SaveResult
}

const account = (
    index: number,
    overrides: Partial<IMessagingAccount> &
        Pick<IMessagingAccount, "channel" | "provider" | "name" | "sender">
): IMessagingAccount => ({
    id: storyId(8, index),
    tenant_id: TENANT_ID,
    provider_approval: EProviderApproval.PENDING,
    status: {
        connected: true,
        production_access: true,
        approved_templates: {},
        checked_at: FIXED_TIME,
    },
    credentials: {},
    limits: {allowed_calling_codes: []},
    webhook_key: null,
    is_default: true,
    created_at: FIXED_TIME,
    updated_at: FIXED_TIME,
    ...overrides,
})

const ACCOUNTS: IMessagingAccount[] = [
    account(1, {
        channel: EMessageChannel.EMAIL,
        provider: EMessagingProvider.AWS_SES,
        name: "Council email",
        sender: {
            provider: EMessagingProvider.AWS_SES,
            from_address: "no-reply@council.example",
            from_name: "Council",
        },
    }),
    account(2, {
        channel: EMessageChannel.SMS,
        provider: EMessagingProvider.AWS_SNS,
        name: "Council SMS",
        sender: {provider: EMessagingProvider.AWS_SNS, sender_id: "COUNCIL"},
    }),
    account(3, {
        channel: EMessageChannel.WHATSAPP,
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        name: "Council WhatsApp",
        sender: {
            provider: EMessagingProvider.WHATSAPP_CLOUD_API,
            business_account_id: "100200300",
            phone_number_id: "400500600",
            display_phone_number: "+34 600 000 001",
            display_name: "Council",
            api_version: "v23.0",
        },
        status: {
            connected: true,
            production_access: true,
            approved_templates: {[EMessagePurpose.NOTICE]: ["en"]},
            checked_at: FIXED_TIME,
        },
    }),
    account(4, {
        channel: EMessageChannel.VIBER,
        provider: EMessagingProvider.VIBER_INFOBIP,
        name: "Council Viber",
        sender: {
            provider: EMessagingProvider.VIBER_INFOBIP,
            base_url: "https://api.infobip.example",
            sender: "Council",
            approved_templates: {
                [EMessagePurpose.OTP]: {en: "otp_en_88213"},
                [EMessagePurpose.NOTICE]: {en: "notice_en_88214"},
            },
        },
        status: {
            connected: true,
            production_access: true,
            approved_templates: {
                [EMessagePurpose.OTP]: ["en"],
                [EMessagePurpose.NOTICE]: ["en"],
            },
            checked_at: FIXED_TIME,
        },
    }),
    account(5, {
        channel: EMessageChannel.MESSENGER,
        provider: EMessagingProvider.MESSENGER_SEND_API,
        name: "Council Page",
        sender: {
            provider: EMessagingProvider.MESSENGER_SEND_API,
            page_id: "118204557331906",
            page_name: "Council",
            page_username: "council",
            api_version: "v23.0",
        },
        status: {
            connected: false,
            production_access: false,
            approved_templates: {},
            checked_at: FIXED_TIME,
            reason: "The Page access token expired",
        },
    }),
]

const accountId = (channel: EMessageChannel) =>
    ACCOUNTS.find((entry) => entry.channel === channel)?.id ?? ""

const CONFIG: IEventMessagingConfig = {
    version: 1,
    channels: [
        {
            channel: EMessageChannel.EMAIL,
            account_id: accountId(EMessageChannel.EMAIL),
            purposes: [EMessagePurpose.OTP, EMessagePurpose.NOTICE],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.SMS,
            account_id: accountId(EMessageChannel.SMS),
            purposes: [EMessagePurpose.NOTICE],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.WHATSAPP,
            account_id: accountId(EMessageChannel.WHATSAPP),
            purposes: [],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.VIBER,
            account_id: accountId(EMessageChannel.VIBER),
            purposes: [EMessagePurpose.OTP],
            templates: [
                {purpose: EMessagePurpose.OTP, language: "en", provider_template: "otp_en_88213"},
            ],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.MESSENGER,
            account_id: accountId(EMessageChannel.MESSENGER),
            purposes: [],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
    ],
    notice_fallback: [EMessageChannel.EMAIL, EMessageChannel.SMS],
    election_channels: {[STORY_IDS.secondElection]: [EMessageChannel.EMAIL]},
    reply_text: {en: "This account only sends codes and notices. Replies are not read."},
}

const ELECTIONS = [
    electionRecord(),
    electionRecord(undefined, {
        id: STORY_IDS.secondElection,
        presentation: electionPresentation("Pension board", "Pension board"),
    }),
]

const COUNTS: Record<string, number> = {
    EMAIL_ACCEPTED: 2,
    EMAIL_DELIVERED: 310,
    EMAIL_FAILED: 3,
    SMS_ACCEPTED: 120,
    SMS_FAILED: 4,
    SMS_UNKNOWN: 1,
    VIBER_QUEUED: 5,
    VIBER_DELIVERED: 41,
}

// Every alias of the query is answered; those without traffic count 0.
const STATS = Object.fromEntries(
    MESSAGE_CHANNELS.flatMap((channel) =>
        MESSAGE_ATTEMPT_STATES.map((state) => `${channel}_${state}`)
    ).map((alias) => [
        alias,
        {
            __typename: "sequent_backend_message_aggregate",
            aggregate: {
                __typename: "sequent_backend_message_aggregate_fields",
                count: COUNTS[alias] ?? 0,
            },
        },
    ])
)

const REJECTED: IMessagingConfigError[] = [
    {
        kind: "TEMPLATE_NOT_APPROVED",
        channel: EMessageChannel.VIBER,
        purpose: EMessagePurpose.OTP,
        language: "es",
    },
    {kind: "UNKNOWN_ELECTION", election_id: storyId(3, 9)},
]

const ALL_ROLES = [
    IPermissions.MESSAGING_CONFIG_WRITE,
    IPermissions.MESSAGING_ACCOUNT_READ,
    IPermissions.NOTIFICATION_READ,
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({roles, configured}: Scenario) {
    const event = eventRecord(undefined, {
        annotations: configured ? {[MESSAGING_CONFIG_ANNOTATION]: JSON.stringify(CONFIG)} : {},
    })
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <RecordContextProvider value={event}>
                <EditElectionEventMessaging />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const saveReply = (result: SaveResult) => () => {
    if (result === "fails") throw new Error("Synthetic messaging service unavailable")
    return {
        data: {
            update_event_messaging_config: {
                __typename: "UpdateEventMessagingConfigOutput",
                errors: result === "rejected" ? REJECTED : [],
            },
        },
    }
}

const meta = {
    title: "Admin/Election event/EditElectionEventMessaging",
    component: EditElectionEventMessaging,
    args: {roles: ALL_ROLES, configured: true, save: "saved"},
    argTypes: {save: {control: "inline-radio", options: ["saved", "rejected", "fails"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({sequent_backend_election: ELECTIONS})
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary({
            GetMessagingAccounts: () => ({
                data: {
                    sequent_backend_messaging_account: ACCOUNTS.map((entry) => ({
                        ...entry,
                        __typename: "sequent_backend_messaging_account",
                    })),
                },
            }),
            GetMessageDeliveryStats: () => ({data: STATS}),
            UpdateEventMessagingConfig: saveReply(args.save),
        })
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const channelRow = async (canvasElement: HTMLElement, channel: EMessageChannel) => {
    await within(canvasElement).findAllByText("Council SMS")
    return canvasElement.querySelector(
        `table[aria-label="Channels"] tr[data-channel="${channel}"]`
    ) as HTMLElement
}

const purposeSwitch = (row: HTMLElement, purpose: string) =>
    row.querySelector(`input[aria-label$=": ${purpose}"]`) as HTMLInputElement

/** Waits for a notification, which slides in. */
const notified = async (text: string) => {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

const saveCall = () => graphql.calls.find(({name}) => name === "UpdateEventMessagingConfig")

export const Populated: Story = {
    parameters: {
        widgets: [
            "Section",
            "Helper",
            "MessagingErrors",
            "MessagingChannels",
            "MessengerOutOfWindow",
            "MessagingTemplateBindings",
            "MessagingFallbackOrder",
            "MessagingElectionChannels",
            "MessagingReplyText",
            "MessagingDeliveryStatus",
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(/^The channels voters of this event can choose/)
        ).toBeVisible()
        const whatsapp = await channelRow(canvasElement, EMessageChannel.WHATSAPP)
        await waitFor(() => expect(purposeSwitch(whatsapp, "OTPs")).toBeDisabled())
        expect(within(whatsapp).getAllByText(/^Missing: Needs provider approval/)).toHaveLength(2)
        const messenger = await channelRow(canvasElement, EMessageChannel.MESSENGER)
        expect(within(messenger).getByText(/stays off until Meta confirms/)).toBeVisible()
        const delivery = within(
            canvasElement.querySelector('table[aria-label="Delivery status"]') as HTMLElement
        )
        const sms = within(delivery.getByText("SMS").closest("tr") as HTMLElement)
        await waitFor(() => expect(sms.getByText("Delivery unavailable")).toBeVisible())
        const viber = within(delivery.getByText("Viber").closest("tr") as HTMLElement)
        expect(viber.getByText("41")).toBeVisible()
        expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(graphql.calls.find(({name}) => name === "GetMessageDeliveryStats")?.headers).toEqual(
            {"x-hasura-role": IPermissions.NOTIFICATION_READ}
        )
    },
}

export const NotConfigured: Story = {
    parameters: {
        widgets: [
            "Section",
            "Helper",
            "MessagingChannels",
            "MessagingFallbackOrder",
            "MessagingDeliveryStatus",
        ],
    },
    args: {configured: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No channel is in use.")).toBeVisible()
        await expect(
            canvas.getByText("Turn on notices for a channel to add it to the fallback order.")
        ).toBeVisible()
    },
}

export const ReadOnly: Story = {
    parameters: {widgets: ["MessagingChannels", "MessengerOutOfWindow"]},
    args: {roles: [IPermissions.MESSAGING_ACCOUNT_READ]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(/^You can view these settings/)).toBeVisible()
        const email = await channelRow(canvasElement, EMessageChannel.EMAIL)
        expect(purposeSwitch(email, "OTPs")).toBeDisabled()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
    },
}

export const EnableNoticesAndSave: Story = {
    parameters: {widgets: ["MessagingChannels"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const viber = await channelRow(canvasElement, EMessageChannel.VIBER)
        await waitFor(() => expect(purposeSwitch(viber, "Notices")).toBeEnabled())
        await userEvent.click(purposeSwitch(viber, "Notices"))
        await expect(await canvas.findByText(/^Saving also updates the channels/)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("Messaging settings saved.")
        const call = saveCall()
        expect(call?.headers).toEqual({"x-hasura-role": IPermissions.MESSAGING_CONFIG_WRITE})
        const config = call?.variables.config as IEventMessagingConfig
        expect(call?.variables.electionEventId).toBe(EVENT_ID)
        expect(config.notice_fallback).toEqual([
            EMessageChannel.EMAIL,
            EMessageChannel.SMS,
            EMessageChannel.VIBER,
        ])
        expect(
            config.channels.find(({channel}) => channel === EMessageChannel.VIBER)?.purposes
        ).toEqual([EMessagePurpose.OTP, EMessagePurpose.NOTICE])
    },
}

export const ReorderFallbackAndRestrictAPost: Story = {
    parameters: {widgets: ["MessagingFallbackOrder", "MessagingElectionChannels"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Move SMS earlier"}))
        await userEvent.click(await canvas.findByRole("checkbox", {name: "Council: SMS"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("Messaging settings saved.")
        const config = saveCall()?.variables.config as IEventMessagingConfig
        expect(config.notice_fallback).toEqual([EMessageChannel.SMS, EMessageChannel.EMAIL])
        expect(config.election_channels[STORY_IDS.election]).toEqual([
            EMessageChannel.EMAIL,
            EMessageChannel.VIBER,
        ])
    },
}

export const SaveRejected: Story = {
    parameters: {
        widgets: [
            "MessagingErrors",
            "MessagingTemplateBindings",
            "MessagingElectionChannels",
            "MessagingReplyText",
        ],
    },
    args: {save: "rejected"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.type(await canvas.findByRole("textbox", {name: "Reply (es)"}), "Hola")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("The messaging settings were not saved. Fix the problems shown.")
        await expect(
            await canvas.findByText(
                "The Viber template for OTPs in es is not approved by the provider."
            )
        ).toBeVisible()
        await expect(
            canvas.getByText(`${storyId(3, 9)} is not an election of this event.`)
        ).toBeVisible()
    },
}

export const SaveFails: Story = {
    parameters: {widgets: ["MessagingReplyText"]},
    args: {save: "fails"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.type(await canvas.findByRole("textbox", {name: "Reply (es)"}), "Hola")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("Could not save the messaging settings.")
        expect(canvas.getByRole("button", {name: "Save"})).toBeEnabled()
    },
}
