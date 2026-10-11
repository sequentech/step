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
    IChannelMessageStats,
    IEventMessagingConfig,
    IMessagingAccount,
    IMessagingConfigError,
} from "@/types/messaging"
import {ERecountOutcome, IVoterCounts} from "@/queries/GetMessagingVoterCounts"
import {EditElectionEventMessaging} from "./EditElectionEventMessaging"

type SaveResult = "saved" | "rejected" | "fails"

interface Scenario {
    /** Replaces the signed-in user's roles. */
    roles: string[]
    /** Whether the event already has a messaging configuration. */
    configured: boolean
    /** What saving answers. */
    save: SaveResult
    /** Whether the deployment has the webhook component's public address. */
    callbackAddress?: boolean
    /** Whether voters with no contact channel have failed messages. */
    noContactFailures?: boolean
    /** Who last wrote the configuration (`updated_by`). */
    updatedBy?: string
    /** The channel whose account's identity changed and wasn't checked since. */
    unchecked?: EMessageChannel
    /** The event's voters as last counted; `"NEVER"`: not counted yet. */
    voterCounts?: IVoterCounts | "NEVER"
    /** The event has no message yet. */
    noMessages?: boolean
    /** Forty bindings, matching a multilingual demo event. */
    manyTemplates?: boolean
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
            out_of_window: EOutOfWindowPolicy.UTILITY_MESSAGES,
        },
    ],
    election_channels: {[STORY_IDS.secondElection]: [EMessageChannel.EMAIL]},
}

/** A configuration saved before the second pass: its fallback order and replies are ignored. */
const STORED_CONFIG = {
    ...CONFIG,
    notice_fallback: [EMessageChannel.EMAIL, EMessageChannel.SMS],
    reply_text: {en: "This account only sends codes and notices. Replies are not read."},
}

const ELECTIONS = [
    electionRecord(),
    electionRecord(undefined, {
        id: STORY_IDS.secondElection,
        presentation: electionPresentation("Pension board", "Pension board"),
    }),
]

const stats = (
    channel: EMessageChannel,
    counts: Partial<IChannelMessageStats>
): IChannelMessageStats => ({
    channel,
    sent_last_24h: 0,
    queued: 0,
    accepted: 0,
    delivered: 0,
    failed: 0,
    unknown: 0,
    unresolved: 0,
    ...counts,
})

// Per message, by the state of its latest attempt.
const STATS: IChannelMessageStats[] = [
    stats(EMessageChannel.EMAIL, {
        sent_last_24h: 12,
        accepted: 2,
        delivered: 310,
        failed: 3,
    }),
    stats(EMessageChannel.SMS, {
        sent_last_24h: 30,
        accepted: 120,
        failed: 4,
        unknown: 1,
        unresolved: 2,
    }),
    stats(EMessageChannel.VIBER, {sent_last_24h: 5, queued: 5, delivered: 41}),
]

// Counted in the background this morning; some voters have no contact channel.
const VOTER_COUNTS: IVoterCounts = {
    total: 2355,
    channels: [
        {channel: EMessageChannel.VIBER, voters: 96, not_offered: 0},
        {channel: EMessageChannel.SMS, voters: 412, not_offered: 0},
        {channel: EMessageChannel.EMAIL, voters: 1840, not_offered: 0},
        {channel: null, voters: 7, not_offered: 0},
    ],
    status: {computed_at: FIXED_TIME, pending: false, outcome: ERecountOutcome.COUNTED},
}

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
    IPermissions.MESSAGING_CONFIG_READ,
    IPermissions.LOGS_READ,
    IPermissions.ELECTION_EVENT_LOGS_TAB,
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({roles}: Scenario) {
    const event = eventRecord()
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
        data = resourceBoundary({
            sequent_backend_election: ELECTIONS,
            sequent_backend_template: [
                {id: storyId(8, 20), tenant_id: TENANT_ID, alias: "voting-reminder"},
            ],
        })
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary({
            GetMessagingAccountSummaries: () => ({
                data: {
                    sequent_backend_messaging_account: ACCOUNTS.map(
                        ({credentials: _credentials, ...entry}) => ({
                            ...entry,
                            // Its last check was of an earlier sender.
                            status:
                                entry.channel === args.unchecked && entry.status
                                    ? {
                                          ...entry.status,
                                          details: {
                                              ...entry.status.details,
                                              identity: {
                                                  provider: entry.provider,
                                                  ids: {sender_id: "Earlier sender"},
                                              },
                                          },
                                      }
                                    : entry.status,
                            __typename: "sequent_backend_messaging_account",
                        })
                    ),
                },
            }),
            GetEventMessagingConfig: () => ({
                data: {
                    sequent_backend_messaging_event_config: args.configured
                        ? [
                              {
                                  __typename: "sequent_backend_messaging_event_config",
                                  election_event_id: EVENT_ID,
                                  config: args.manyTemplates
                                      ? {
                                            ...STORED_CONFIG,
                                            channels: CONFIG.channels.map((entry) => ({
                                                ...entry,
                                                templates: Array.from(
                                                    {
                                                        length:
                                                            entry.channel ===
                                                                EMessageChannel.WHATSAPP ||
                                                            entry.channel === EMessageChannel.VIBER
                                                                ? 16
                                                                : entry.channel ===
                                                                    EMessageChannel.MESSENGER
                                                                  ? 8
                                                                  : 0,
                                                    },
                                                    (_, index) => ({
                                                        purpose: EMessagePurpose.NOTICE,
                                                        language: "en",
                                                        key: `notice-${index}`,
                                                        provider_template: `reminder_${index}`,
                                                    })
                                                ),
                                            })),
                                        }
                                      : STORED_CONFIG,
                                  version: 1,
                                  updated_at: FIXED_TIME,
                              },
                          ]
                        : [],
                },
            }),
            GetMessagingVoterCounts: () => ({
                data: {
                    messaging_voter_counts:
                        args.voterCounts === "NEVER"
                            ? {
                                  total: 0,
                                  channels: [],
                                  status: {computed_at: null, pending: false, outcome: null},
                              }
                            : (args.voterCounts ?? VOTER_COUNTS),
                },
            }),
            RecountMessagingVoters: () => ({
                data: {
                    messaging_recount_voters: {
                        requested: true,
                        status: {...VOTER_COUNTS.status, pending: true},
                    },
                },
            }),
            GetMessagingEventStats: () => ({
                data: {
                    messaging_event_stats: {
                        channels: args.noMessages
                            ? []
                            : args.noContactFailures
                              ? [
                                    ...STATS,
                                    {...stats(EMessageChannel.EMAIL, {failed: 6}), channel: null},
                                ]
                              : STATS,
                    },
                },
            }),
            GetEventMessagingConfigAuthor: () => ({
                data: {
                    sequent_backend_messaging_event_config: args.configured
                        ? [{election_event_id: EVENT_ID, updated_by: args.updatedBy ?? null}]
                        : [],
                },
            }),
            GetMessagingCallbackStatus: () => ({
                data: {
                    messaging_callback_status: {
                        accounts:
                            args.callbackAddress === false
                                ? ACCOUNTS.map(({id}) => ({
                                      account_id: id,
                                      blockers: ["CALLBACK_NOT_CONFIGURED"],
                                      reasons: ["Callback address not configured"],
                                  }))
                                : [],
                    },
                },
            }),
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

export const FluidIntro: Story = {
    play: async ({canvasElement}) => {
        const intro = await within(canvasElement).findByText(
            /^The channels voters of this event can choose/
        )
        const container = intro.parentElement as HTMLElement
        const channels = container.querySelector("#messaging-channels") as HTMLElement
        const originalWidth = container.style.width
        try {
            for (const width of [1200, 600]) {
                container.style.width = `${width}px`
                await waitFor(() =>
                    expect(intro.getBoundingClientRect().width).toBeCloseTo(
                        channels.getBoundingClientRect().width,
                        0
                    )
                )
            }
        } finally {
            container.style.width = originalWidth
        }
    },
}

export const Populated: Story = {
    parameters: {
        widgets: [
            "Section",
            "Helper",
            "MessagingErrors",
            "MessagingChannels",
            "MessagingTemplateBindings",
            "MessagingElectionChannels",
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
        expect(
            within(messenger).getByText(/sends notices with the approved utility template/)
        ).toBeVisible()
        expect(canvas.queryByRole("combobox", {name: /conversation window/})).toBeNull()
        expect(canvas.queryByText(/Fallback order/)).toBeNull()
        expect(canvas.queryByText(/Reply to incoming messages/)).toBeNull()
        // Channel order: WhatsApp, Viber, Messenger, SMS, Email.
        expect(
            Array.from(
                canvasElement.querySelectorAll('table[aria-label="Channels"] tr[data-channel]')
            ).map((row) => row.getAttribute("data-channel"))
        ).toEqual(["WHATSAPP", "VIBER", "MESSENGER", "SMS", "EMAIL"])
        await expect(canvas.getByText(/^For each message the most specific row wins/)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Viber"}))
        const viberTemplate = within(canvas.getByRole("group", {name: "Viber template 1"}))
        await userEvent.click(viberTemplate.getByRole("button", {name: "Viber template 1"}))
        await expect(viberTemplate.getByRole("combobox", {name: "Provider template"})).toHaveValue(
            "otp_en_88213"
        )
        await expect(viberTemplate.getByText("Approved")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "WhatsApp"}))
        await expect(canvas.getByText(/^WhatsApp sends only approved templates/)).toBeVisible()
        const delivery = within(
            canvasElement.querySelector('table[aria-label="Delivery status"]') as HTMLElement
        )
        const sms = within(delivery.getByText("SMS").closest("tr") as HTMLElement)
        await waitFor(() => expect(sms.getByText("Delivery unavailable")).toBeVisible())
        await expect(sms.getByText("412")).toBeVisible()
        await expect(sms.getByText("30")).toBeVisible()
        await expect(
            sms.getByRole("button", {name: "Show the failed SMS messages in Logs"})
        ).toHaveTextContent("4")
        // SES without a notification topic gets no receipts: unavailable, never 0.
        const email = within(delivery.getByText("Email").closest("tr") as HTMLElement)
        await expect(email.getByText("Delivery unavailable")).toBeVisible()
        const viber = within(delivery.getByText("Viber").closest("tr") as HTMLElement)
        expect(viber.getByText("41")).toBeVisible()
        // Voters as last counted, with when.
        expect(canvas.getByText(/^Counted at /)).toBeVisible()
        expect(
            within(delivery.getByText("Email").closest("tr") as HTMLElement).getByText("1,840")
        ).toBeVisible()
        expect(
            within(
                canvasElement.querySelector(
                    'table[aria-label="Delivery status"] tr[data-channel="NONE"]'
                ) as HTMLElement
            ).getByText("7")
        ).toBeVisible()
        expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        for (const operation of [
            "GetEventMessagingConfig",
            "GetMessagingAccountSummaries",
            "GetMessagingEventStats",
            "GetMessagingVoterCounts",
        ]) {
            expect(graphql.calls.find(({name}) => name === operation)?.headers).toEqual({
                "x-hasura-role": IPermissions.MESSAGING_CONFIG_READ,
            })
        }
    },
}

const deliveryRows = (canvasElement: HTMLElement) =>
    Array.from(
        canvasElement.querySelectorAll('table[aria-label="Delivery status"] tr[data-channel]')
    ).map((row) => row.getAttribute("data-channel"))

/**
 * Without a configuration the event's voters and messages still show, on
 * every channel they're on, each said not to be offered by the event.
 */
export const NotConfigured: Story = {
    parameters: {
        widgets: ["Section", "Helper", "MessagingChannels", "MessagingDeliveryStatus"],
    },
    args: {configured: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(deliveryRows(canvasElement)).toEqual(["VIBER", "SMS", "EMAIL", "NONE"])
        )
        expect(canvas.getAllByText("Not offered by this event")).toHaveLength(3)
        expect(canvas.queryByText("No channel is in use.")).toBeNull()
        const email = within(
            canvasElement.querySelector(
                'table[aria-label="Delivery status"] tr[data-channel="EMAIL"]'
            ) as HTMLElement
        )
        expect(email.getByText("1,840")).toBeVisible()
        // Its refused messages are counted as they are, not "unavailable".
        expect(email.queryByText("Delivery unavailable")).toBeNull()
    },
}

/** No configuration, no message and voters never counted: nothing in use, and Recount. */
export const NothingInUse: Story = {
    parameters: {widgets: ["MessagingDeliveryStatus"]},
    args: {configured: false, noMessages: true, voterCounts: "NEVER"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No channel is in use.")).toBeVisible()
        expect(canvas.getByText("Not counted yet.")).toBeVisible()
        expect(canvas.getByRole("button", {name: "Recount"})).toBeEnabled()
    },
}

/**
 * Recount asks for the event's voters to be counted again; the last count
 * that took too long says so and keeps the counts before it.
 */
export const RecountVoters: Story = {
    parameters: {widgets: ["MessagingDeliveryStatus"]},
    args: {
        voterCounts: {
            ...VOTER_COUNTS,
            status: {...VOTER_COUNTS.status, outcome: ERecountOutcome.TIMED_OUT},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(/The last recount took too long and was stopped\./)
        ).toBeVisible()
        expect(canvas.getByText(/The counts shown are from the count before it\./)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Recount"}))
        await notified("Recount asked for. The counts update when it finishes.")
        expect(graphql.calls.find(({name}) => name === "RecountMessagingVoters")).toMatchObject({
            variables: {electionEventId: EVENT_ID},
            headers: {"x-hasura-role": IPermissions.MESSAGING_CONFIG_READ},
        })
    },
}

export const ReadOnly: Story = {
    parameters: {widgets: ["MessagingChannels", "MessagingTemplateBindings"]},
    args: {roles: [IPermissions.MESSAGING_CONFIG_READ]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(/^You can view these settings/)).toBeVisible()
        const email = await channelRow(canvasElement, EMessageChannel.EMAIL)
        expect(purposeSwitch(email, "OTPs")).toBeDisabled()
        expect(canvas.queryByRole("button", {name: /^Add .* template$/})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
        // Without access to Logs, Failed is a plain count.
        expect(canvas.queryByRole("button", {name: /^Show the failed/})).toBeNull()
    },
}

export const NoReadPermission: Story = {
    parameters: {widgets: []},
    args: {roles: [IPermissions.MESSAGING_CONFIG_WRITE]},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                "Reading these settings needs the messaging-config-read permission."
            )
        ).toBeVisible()
        expect(graphql.calls).toEqual([])
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
        // The stored fallback order and replies are never written again.
        expect(Object.keys(config).sort()).toEqual(["channels", "election_channels", "version"])
        expect(
            config.channels.find(({channel}) => channel === EMessageChannel.VIBER)?.purposes
        ).toEqual([EMessagePurpose.OTP, EMessagePurpose.NOTICE])
    },
}

export const RestrictAPost: Story = {
    parameters: {widgets: ["MessagingElectionChannels"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("checkbox", {name: "Council: SMS"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("Messaging settings saved.")
        const config = saveCall()?.variables.config as IEventMessagingConfig
        expect(config.election_channels[STORY_IDS.election]).toEqual([
            EMessageChannel.VIBER,
            EMessageChannel.EMAIL,
        ])
    },
}

export const FailedOpensLogs: Story = {
    parameters: {widgets: ["MessagingDeliveryStatus"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const failed = await canvas.findByRole("button", {
            name: "Show the failed Email messages in Logs",
        })
        await expect(failed).toHaveTextContent("3")
        await userEvent.click(failed)
    },
}

export const MessengerUtilityTemplate: Story = {
    parameters: {widgets: ["MessagingChannels", "MessagingTemplateBindings"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const messenger = await channelRow(canvasElement, EMessageChannel.MESSENGER)
        await expect(
            within(messenger).getByText(/sends notices with the approved utility template/)
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Facebook Messenger"}))
        await expect(
            canvas.getByText(/^No templates are bound for Facebook Messenger/)
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Add Facebook Messenger template"}))
        const row = within(
            await canvas.findByRole("group", {name: "Facebook Messenger template 1"})
        )
        await userEvent.click(row.getByRole("combobox", {name: "For message"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "voting-reminder"})
        )
        await userEvent.type(row.getByRole("combobox", {name: "Provider template"}), "reminder")
        await userEvent.type(row.getByRole("textbox", {name: "Provider language"}), "en_US")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("Messaging settings saved.")
        const config = saveCall()?.variables.config as IEventMessagingConfig
        expect(
            config.channels.find(({channel}) => channel === EMessageChannel.MESSENGER)
        ).toMatchObject({
            out_of_window: EOutOfWindowPolicy.UTILITY_MESSAGES,
            templates: [
                {
                    purpose: EMessagePurpose.NOTICE,
                    key: "voting-reminder",
                    language: "en",
                    provider_template: "reminder",
                    provider_language: "en_US",
                },
            ],
        })
    },
}

export const IncompleteTemplateRow: Story = {
    parameters: {widgets: ["MessagingTemplateBindings"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "WhatsApp"}))
        await userEvent.click(await canvas.findByRole("button", {name: "Add WhatsApp template"}))
        const row = within(await canvas.findByRole("group", {name: "WhatsApp template 1"}))
        await userEvent.type(row.getByRole("combobox", {name: "For message"}), "otp")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await expect(
            await row.findByText(
                "Enter the language and the provider template, or remove this row."
            )
        ).toBeVisible()
        expect(saveCall()).toBeUndefined()
        await userEvent.click(row.getByRole("button", {name: "Remove WhatsApp template 1"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("Messaging settings saved.")
    },
}

export const SaveRejected: Story = {
    parameters: {
        widgets: ["MessagingErrors", "MessagingTemplateBindings", "MessagingElectionChannels"],
    },
    args: {save: "rejected"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("checkbox", {name: "Council: SMS"}))
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
    parameters: {widgets: ["MessagingElectionChannels"]},
    args: {save: "fails"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("checkbox", {name: "Council: SMS"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("Could not save the messaging settings.")
        expect(canvas.getByRole("button", {name: "Save"})).toBeEnabled()
    },
}

/** Without the webhook component's address, receipts are unavailable, with the reason. */
export const WithoutACallbackAddress: Story = {
    args: {callbackAddress: false},
    play: async ({canvasElement}) => {
        const delivery = within(
            await within(canvasElement).findByRole("table", {name: "Delivery status"})
        )
        const viber = await waitFor(() => delivery.getByText("Viber").closest("tr") as HTMLElement)
        const cell = within(viber).getByText("Delivery unavailable").closest("td")
        await expect(cell).toHaveAttribute("title", "Callback address not configured")
    },
}

/** Failures of voters with no contact channel have a row of their own, never under Email. */
export const VotersWithNoContactChannel: Story = {
    args: {noContactFailures: true},
    play: async ({canvasElement}) => {
        const delivery = within(
            await within(canvasElement).findByRole("table", {name: "Delivery status"})
        )
        const row = await waitFor(
            () => delivery.getByText("No contact channel").closest("tr") as HTMLElement
        )
        await expect(within(row).getByText("6")).toBeVisible()
    },
}

/** Copied from the old storage and refused by validation: it asks to be saved again. */
export const MigratedConfigurationNeedsSaving: Story = {
    args: {updatedBy: "migration: needs saving again"},
    parameters: {widgets: ["MigratedConfigNotice"]},
    play: async ({canvasElement}) => {
        const alert = await within(canvasElement).findByText("Configuration needs saving again.")
        await expect(alert).toBeVisible()
        await expect(
            within(canvasElement).getByText(/didn't pass the checks a save makes/)
        ).toBeVisible()
    },
}

/**
 * An account whose number changed is ready for nothing until its check
 * runs: the purposes it doesn't send yet can't be turned on, and each says
 * why. A purpose already on can still be turned off.
 */
export const AccountNotCheckedSinceItsNumberChanged: Story = {
    args: {unchecked: EMessageChannel.SMS},
    play: async ({canvasElement}) => {
        const sms = await channelRow(canvasElement, EMessageChannel.SMS)
        await waitFor(() => expect(purposeSwitch(sms, "OTPs")).toBeDisabled())
        expect(purposeSwitch(sms, "Notices")).toBeEnabled()
        expect(
            within(sms).getAllByText("Missing: Not checked for its current number, Page or sender")
        ).toHaveLength(2)
        // The other channels keep their own readiness.
        const email = await channelRow(canvasElement, EMessageChannel.EMAIL)
        expect(within(email).queryByText(/Not checked for its current/)).toBeNull()
    },
}

export const ManyTemplates: Story = {
    args: {manyTemplates: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("button", {name: "WhatsApp"})).toHaveAttribute(
            "aria-expanded",
            "false"
        )
        expect(canvas.queryByRole("group")).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "WhatsApp"}))
        await expect(canvas.getByRole("group", {name: "WhatsApp template 16"})).toBeVisible()
        expect(canvas.queryByRole("group", {name: "Viber template 16"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "WhatsApp"}))
        await userEvent.click(canvas.getByRole("button", {name: "Viber"}))
        expect(canvas.queryByRole("combobox", {name: "Provider template"})).toBeNull()
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        const row = within(canvas.getByRole("group", {name: "Viber template 16"}))
        await userEvent.click(row.getByRole("button", {name: "Viber template 16"}))
        await expect(row.getByRole("combobox", {name: "Provider template"})).toHaveValue(
            "reminder_15"
        )
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        await userEvent.clear(row.getByRole("combobox", {name: "Provider template"}))
        await userEvent.type(
            row.getByRole("combobox", {name: "Provider template"}),
            "changed_reminder"
        )
        await userEvent.click(row.getByRole("button", {name: "Viber template 16"}))
        await userEvent.click(row.getByRole("button", {name: "Viber template 16"}))
        await expect(row.getByRole("combobox", {name: "Provider template"})).toHaveValue(
            "changed_reminder"
        )
        await userEvent.click(canvas.getByRole("button", {name: "Viber"}))
        expect(canvas.queryByRole("group", {name: "Viber template 16"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Viber"}))
        await expect(
            within(canvas.getByRole("group", {name: "Viber template 16"})).getByRole("combobox", {
                name: "Provider template",
            })
        ).toHaveValue("changed_reminder")
        expect(canvas.getAllByRole("combobox", {name: "Provider template"})).toHaveLength(1)
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await notified("Messaging settings saved.")
        const config = saveCall()?.variables.config as IEventMessagingConfig
        expect(config.channels.flatMap((channel) => channel.templates)).toHaveLength(40)
        expect(
            config.channels.find(({channel}) => channel === EMessageChannel.VIBER)?.templates[15]
                .provider_template
        ).toBe("changed_reminder")
        expect(
            config.channels.find(({channel}) => channel === EMessageChannel.WHATSAPP)?.templates[15]
                .provider_template
        ).toBe("reminder_15")
    },
}

export const ChannelGroupsFitNarrowScreens: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const toggle = await canvas.findByRole("button", {name: "Facebook Messenger"})
        const container = canvasElement.querySelector(
            '[data-template-channel="MESSENGER"]'
        ) as HTMLElement
        const original = container.style.width
        try {
            for (const width of [600, 224]) {
                container.style.width = `${width}px`
                await waitFor(() => {
                    const right = toggle.getBoundingClientRect().right
                    expect(
                        within(toggle).getByText("Facebook Messenger").getBoundingClientRect().right
                    ).toBeLessThanOrEqual(right)
                    expect(
                        within(toggle).getByText("0").getBoundingClientRect().right
                    ).toBeLessThanOrEqual(right)
                    expect(within(toggle).getByTestId("ExpandMoreIcon")).toBeVisible()
                    expect(
                        within(toggle).getByTestId("ExpandMoreIcon").getBoundingClientRect().width
                    ).toBeGreaterThanOrEqual(20)
                    expect(
                        within(toggle).getByTestId("ExpandMoreIcon").getBoundingClientRect().right
                    ).toBeLessThanOrEqual(right)
                })
            }
        } finally {
            container.style.width = original
        }
    },
}
