// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A second organization for the messaging screens (ticket: "A second
// organization uses the same sender interfaces and screens with different
// accounts, purposes, labels and election restrictions"). The ui-drafts
// "Second org": tenant STAFF-ASSOCIATION, accounts on all five channels with
// Email through an SMTP server, Spain (CEST, EUR), English and Spanish, and
// the elections "Council: Central office", "Council: Canary Islands office"
// and "Pension board". Every value is invented. The stories that use it
// assert that its own names appear and that no other client is named.
import {ELogTimeZonePolicy, type IElectionPresentation} from "@sequentech/ui-core"
import type {
    ElectoralLogRow,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Template,
    Sequent_Backend_Tenant,
} from "@/gql/graphql"
import {EFakeReportState, type IFakeProviderMessage} from "@/resources/Settings/SentMessages/types"
import {settingsTenant} from "@/resources/Settings/__stories__/SettingsFixture"
import type {IMonthlyVolume} from "@/queries/GetMessagingMonthlyVolumes"
import {
    ESendsFrom,
    type IAudienceChannel,
    type IAudiencePreview,
} from "@/queries/GetMessagingAudiencePreview"
import {ERecountOutcome, type IVoterCounts} from "@/queries/GetMessagingVoterCounts"
import {ITemplateMethod} from "@/types/templates"
import {
    EMessageAttemptState,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    EProviderApproval,
    type IAccountCheck,
    type IChannelMessageStats,
    type IEventMessagingConfig,
    type ILedgerMessage,
    type IMessagingAccount,
} from "@/types/messaging"
import {EVENT_ID, TENANT_ID} from "./AdminStoryProvider"
import {
    LANGUAGE_CONF,
    STORY_IDS,
    electionRecord,
    eventPresentation,
    eventRecord,
    storyId,
    type StoryRecord,
} from "./fixtures"

/** The organization's display name: who voters see the messages from. */
export const SECOND_ORGANIZATION = "Staff Association"
export const SECOND_ORGANIZATION_SLUG = "staff-association"

/** Matches any name of the first organization's client; no second-organization screen shows one. */
export const OTHER_CLIENT = /COMELEC/i

// When the accounts were last checked: 20 May 2028, 10:00 CEST.
const CHECKED_AT = "2028-05-20T08:00:00Z"

/** The tenant, as Settings reads it: Spain, English and Spanish. */
export const secondOrganizationTenant = (): StoryRecord<Sequent_Backend_Tenant> =>
    settingsTenant(
        {
            language_conf: LANGUAGE_CONF,
            voting_countries: ["ES"],
            i18n: {
                en: {"adminPortal:header.welcome": `Welcome, ${SECOND_ORGANIZATION}`},
                es: {"adminPortal:header.welcome": `Bienvenida, ${SECOND_ORGANIZATION}`},
            },
        },
        {slug: SECOND_ORGANIZATION_SLUG}
    )

// ---------------------------------------------------------------------------
// Sending accounts: one per channel, with the organization's own senders.

export const SECOND_ORG_ACCOUNT_IDS = {
    [EMessageChannel.WHATSAPP]: storyId(10, 1),
    [EMessageChannel.VIBER]: storyId(10, 2),
    [EMessageChannel.MESSENGER]: storyId(10, 3),
    [EMessageChannel.SMS]: storyId(10, 4),
    [EMessageChannel.EMAIL]: storyId(10, 5),
} as const

/** Account names, by channel (the Account column). */
export const SECOND_ORG_ACCOUNT_NAMES = {
    [EMessageChannel.WHATSAPP]: "Staff Association",
    [EMessageChannel.VIBER]: "Staff Assoc",
    [EMessageChannel.MESSENGER]: "Staff Association Elections",
    [EMessageChannel.SMS]: "Association sender ID",
    [EMessageChannel.EMAIL]: "Association mail",
} as const

/** What voters see as the sender (the Sends as column), by channel. */
export const SECOND_ORG_SENDERS = {
    [EMessageChannel.WHATSAPP]: "Staff Association",
    [EMessageChannel.VIBER]: "Staff Assoc",
    [EMessageChannel.MESSENGER]: "Staff Association Elections",
    [EMessageChannel.SMS]: "STAFFASSOC",
    [EMessageChannel.EMAIL]: "Staff Association elections",
} as const

export const SECOND_ORG_WHATSAPP_NUMBER = "+34 910 000 123"
export const SECOND_ORG_PAGE_ID = "208877665544332"
export const SECOND_ORG_PAGE_USERNAME = "staffassocelections"
export const SECOND_ORG_EMAIL_FROM = "elections@staffassoc.admin-story.invalid"

const ready = (
    approvedTemplates: IAccountCheck["approved_templates"] = {},
    details: IAccountCheck["details"] = {provider_approval: EProviderApproval.CONFIRMED}
): IAccountCheck => ({
    connected: true,
    production_access: true,
    approved_templates: approvedTemplates,
    checked_at: CHECKED_AT,
    reason: null,
    details,
})

const base = {
    tenant_id: TENANT_ID,
    limits: {allowed_calling_codes: ["34"]},
    provider_approval: EProviderApproval.CONFIRMED,
    is_default: true,
    created_at: CHECKED_AT,
    updated_at: CHECKED_AT,
}

export const secondOrgWhatsAppAccount = (): IMessagingAccount => ({
    ...base,
    id: SECOND_ORG_ACCOUNT_IDS.WHATSAPP,
    channel: EMessageChannel.WHATSAPP,
    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
    name: SECOND_ORG_ACCOUNT_NAMES.WHATSAPP,
    sender: {
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        business_account_id: "300000000000001",
        phone_number_id: "400000000000002",
        display_phone_number: SECOND_ORG_WHATSAPP_NUMBER,
        display_name: SECOND_ORG_SENDERS.WHATSAPP,
        api_version: "v23.0",
    },
    credentials: {
        ACCESS_TOKEN: {replaced_at: CHECKED_AT},
        APP_SECRET: {replaced_at: CHECKED_AT},
        VERIFY_TOKEN: {replaced_at: CHECKED_AT},
    },
    status: ready(
        {OTP: ["en", "es"], NOTICE: ["en", "es"]},
        {
            provider_approval: EProviderApproval.CONFIRMED,
            whatsapp_number: {
                status: "CONNECTED",
                name_status: "APPROVED",
                quality_rating: "GREEN",
                messaging_limit: "TIER_1K",
            },
            templates: [
                {
                    name: "staff_code",
                    language: "en",
                    category: "AUTHENTICATION",
                    status: "APPROVED",
                },
                {
                    name: "staff_code",
                    language: "es",
                    category: "AUTHENTICATION",
                    status: "APPROVED",
                },
            ],
        }
    ),
})

export const secondOrgViberAccount = (): IMessagingAccount => ({
    ...base,
    id: SECOND_ORG_ACCOUNT_IDS.VIBER,
    channel: EMessageChannel.VIBER,
    provider: EMessagingProvider.VIBER_INFOBIP,
    name: SECOND_ORG_ACCOUNT_NAMES.VIBER,
    sender: {
        provider: EMessagingProvider.VIBER_INFOBIP,
        base_url: "https://staffassoc.api.infobip.admin-story.invalid",
        sender: SECOND_ORG_SENDERS.VIBER,
        approved_templates: {NOTICE: {en: "51001", es: "51002"}},
    },
    credentials: {
        API_KEY: {replaced_at: CHECKED_AT},
        REPORT_USERNAME: {replaced_at: CHECKED_AT},
        REPORT_PASSWORD: {replaced_at: CHECKED_AT},
    },
    status: ready({NOTICE: ["en", "es"]}),
})

export const secondOrgMessengerAccount = (): IMessagingAccount => ({
    ...base,
    id: SECOND_ORG_ACCOUNT_IDS.MESSENGER,
    channel: EMessageChannel.MESSENGER,
    provider: EMessagingProvider.MESSENGER_SEND_API,
    name: SECOND_ORG_ACCOUNT_NAMES.MESSENGER,
    sender: {
        provider: EMessagingProvider.MESSENGER_SEND_API,
        page_id: SECOND_ORG_PAGE_ID,
        page_name: SECOND_ORG_SENDERS.MESSENGER,
        page_username: SECOND_ORG_PAGE_USERNAME,
        api_version: "v23.0",
    },
    credentials: {
        ACCESS_TOKEN: {replaced_at: CHECKED_AT},
        APP_SECRET: {replaced_at: CHECKED_AT},
        VERIFY_TOKEN: {replaced_at: CHECKED_AT},
    },
    status: ready(
        {},
        {
            provider_approval: EProviderApproval.CONFIRMED,
            messenger: {pages_messaging: true, subscribed: true, get_started: true},
        }
    ),
})

export const secondOrgSmsAccount = (): IMessagingAccount => ({
    ...base,
    id: SECOND_ORG_ACCOUNT_IDS.SMS,
    channel: EMessageChannel.SMS,
    provider: EMessagingProvider.AWS_SNS,
    name: SECOND_ORG_ACCOUNT_NAMES.SMS,
    sender: {
        provider: EMessagingProvider.AWS_SNS,
        sender_id: SECOND_ORG_SENDERS.SMS,
        origination_number: null,
        region: "eu-south-2",
    },
    credentials: {},
    limits: {messages_per_second: 10, otp_reserved_per_second: 4, allowed_calling_codes: ["34"]},
    status: ready(),
})

/** Email through the organization's own SMTP server. */
export const secondOrgEmailAccount = (): IMessagingAccount => ({
    ...base,
    id: SECOND_ORG_ACCOUNT_IDS.EMAIL,
    channel: EMessageChannel.EMAIL,
    provider: EMessagingProvider.SMTP,
    name: SECOND_ORG_ACCOUNT_NAMES.EMAIL,
    sender: {
        provider: EMessagingProvider.SMTP,
        from_address: SECOND_ORG_EMAIL_FROM,
        from_name: SECOND_ORG_SENDERS.EMAIL,
        server_url: "smtp.staffassoc.admin-story.invalid:587",
    },
    credentials: {SMTP_PASSWORD: {replaced_at: CHECKED_AT}},
    status: ready(),
})

/** As the query returns them; the list shows them in channel order. */
export const secondOrgAccounts = (): IMessagingAccount[] => [
    secondOrgEmailAccount(),
    secondOrgSmsAccount(),
    secondOrgMessengerAccount(),
    secondOrgViberAccount(),
    secondOrgWhatsAppAccount(),
]

/** The callback URL action for the organization's accounts. */
export const secondOrgCallbackUrlHandler = ({variables}: {variables?: Record<string, unknown>}) => {
    const accountId = String(variables?.accountId ?? "")
    const provider =
        accountId === SECOND_ORG_ACCOUNT_IDS.VIBER
            ? "viber"
            : accountId === SECOND_ORG_ACCOUNT_IDS.WHATSAPP ||
                accountId === SECOND_ORG_ACCOUNT_IDS.MESSENGER
              ? "meta"
              : null
    return {
        data: {
            messaging_account_callback_url: provider
                ? {
                      callback_url: `https://callbacks.staffassoc.admin-story.invalid/webhooks/${provider}/${accountId.replace(/-/g, "").padEnd(64, "0")}`,
                      reason: null,
                  }
                : {callback_url: null, reason: null},
        },
    }
}

// ---------------------------------------------------------------------------
// The election event: its elections (Posts) and messaging configuration.

export const CENTRAL_OFFICE_ID = STORY_IDS.election
export const CANARY_OFFICE_ID = STORY_IDS.secondElection
export const PENSION_BOARD_ID = storyId(3, 5)

/** Post names in English and Spanish. */
export const SECOND_ORG_POSTS = {
    [CENTRAL_OFFICE_ID]: {en: "Council: Central office", es: "Consejo: Oficina central"},
    [CANARY_OFFICE_ID]: {
        en: "Council: Canary Islands office",
        es: "Consejo: Oficina de Canarias",
    },
    [PENSION_BOARD_ID]: {en: "Pension board", es: "Junta de pensiones"},
} as const

const postPresentation = (id: keyof typeof SECOND_ORG_POSTS): IElectionPresentation => {
    const {en, es} = SECOND_ORG_POSTS[id]
    return {
        ...electionRecord().presentation,
        i18n: {en: {name: en, alias: en}, es: {name: es, alias: es}},
        language_conf: LANGUAGE_CONF,
    } as IElectionPresentation
}

export const secondOrgElections = (): StoryRecord<Sequent_Backend_Election>[] =>
    [CENTRAL_OFFICE_ID, CANARY_OFFICE_ID, PENSION_BOARD_ID].map((id) =>
        electionRecord(undefined, {
            id,
            description: SECOND_ORG_POSTS[id].en,
            presentation: postPresentation(id),
        })
    )

/** The event: "Staff Association Council 2028", in Madrid's zone with a Canary Islands office. */
export const secondOrgEvent = (): StoryRecord<Sequent_Backend_Election_Event> =>
    eventRecord(undefined, {
        description: "Staff Association Council 2028",
        presentation: {
            ...eventPresentation,
            i18n: {
                en: {
                    name: "Staff Association Council 2028",
                    alias: "Staff Association Council 2028",
                    description: "Council and pension board elections",
                },
                es: {
                    name: "Consejo de la Staff Association 2028",
                    alias: "Consejo 2028",
                    description: "Elecciones al consejo y a la junta de pensiones",
                },
            },
            timezones: {
                configured: ["Europe/Madrid", "Atlantic/Canary"],
                primary: "Europe/Madrid",
                logs: ELogTimeZonePolicy.PRIMARY,
            },
        },
    })

/**
 * Its own purposes: codes on WhatsApp, Messenger and SMS; notices on
 * WhatsApp, Viber and Email. Its own Post restrictions: the Canary Islands
 * office offers Viber and Messenger, the central office SMS, and the pension
 * board only Email.
 */
export const secondOrgEventConfig = (): IEventMessagingConfig => ({
    version: 1,
    channels: [
        {
            channel: EMessageChannel.WHATSAPP,
            account_id: SECOND_ORG_ACCOUNT_IDS.WHATSAPP,
            purposes: [EMessagePurpose.OTP, EMessagePurpose.NOTICE],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.VIBER,
            account_id: SECOND_ORG_ACCOUNT_IDS.VIBER,
            purposes: [EMessagePurpose.NOTICE],
            templates: [
                {purpose: EMessagePurpose.NOTICE, language: "en", provider_template: "51001"},
                {purpose: EMessagePurpose.NOTICE, language: "es", provider_template: "51002"},
            ],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.MESSENGER,
            account_id: SECOND_ORG_ACCOUNT_IDS.MESSENGER,
            purposes: [EMessagePurpose.OTP],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.SMS,
            account_id: SECOND_ORG_ACCOUNT_IDS.SMS,
            purposes: [EMessagePurpose.OTP],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.EMAIL,
            account_id: SECOND_ORG_ACCOUNT_IDS.EMAIL,
            purposes: [EMessagePurpose.NOTICE],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
    ],
    election_channels: {
        [CENTRAL_OFFICE_ID]: [EMessageChannel.WHATSAPP, EMessageChannel.SMS, EMessageChannel.EMAIL],
        [CANARY_OFFICE_ID]: [
            EMessageChannel.WHATSAPP,
            EMessageChannel.VIBER,
            EMessageChannel.MESSENGER,
            EMessageChannel.EMAIL,
        ],
        [PENSION_BOARD_ID]: [EMessageChannel.EMAIL],
    },
})

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

/** The event tab's per-channel message counts (voters are counted apart). */
export const secondOrgEventStats = (): IChannelMessageStats[] => [
    stats(EMessageChannel.WHATSAPP, {sent_last_24h: 58, delivered: 702, failed: 2}),
    stats(EMessageChannel.VIBER, {sent_last_24h: 4, delivered: 61, unknown: 1}),
    stats(EMessageChannel.MESSENGER, {sent_last_24h: 9, delivered: 133}),
    stats(EMessageChannel.SMS, {sent_last_24h: 17, accepted: 230, failed: 1}),
    stats(EMessageChannel.EMAIL, {sent_last_24h: 40, accepted: 1205}),
]

/** When the event's voters were last counted: 20 May 2028, 10:00 CEST. */
const COUNTED = {computed_at: CHECKED_AT, pending: false, outcome: ERecountOutcome.COUNTED}

/** The event's voters per channel, as last counted (`messaging_voter_counts`). */
export const secondOrgVoterCounts = (): IVoterCounts => {
    const channels = [
        // Pension board voters on WhatsApp: their Post offers only Email.
        {channel: EMessageChannel.WHATSAPP, voters: 640, not_offered: 25},
        {channel: EMessageChannel.VIBER, voters: 85, not_offered: 0},
        // Codes only: no voter gets a notice on Messenger or SMS.
        {channel: EMessageChannel.MESSENGER, voters: 120, not_offered: 120},
        {channel: EMessageChannel.SMS, voters: 210, not_offered: 210},
        {channel: EMessageChannel.EMAIL, voters: 1130, not_offered: 0},
    ]
    return {
        total: channels.reduce((sum, row) => sum + row.voters, 0),
        channels,
        status: COUNTED,
    }
}

// ---------------------------------------------------------------------------
// Send Notification.

/** "Each voter's channel" for every voter of the event. */
export const secondOrgAudience = (): IAudienceChannel[] => [
    {
        channel: EMessageChannel.WHATSAPP,
        voters: 640,
        // Pension board voters: their Post offers only Email.
        not_offered: 25,
        sends_from: ESendsFrom.ACCOUNT,
        account_id: SECOND_ORG_ACCOUNT_IDS.WHATSAPP,
        account_name: SECOND_ORG_ACCOUNT_NAMES.WHATSAPP,
    },
    {
        channel: EMessageChannel.VIBER,
        voters: 85,
        not_offered: 0,
        sends_from: ESendsFrom.ACCOUNT,
        account_id: SECOND_ORG_ACCOUNT_IDS.VIBER,
        account_name: SECOND_ORG_ACCOUNT_NAMES.VIBER,
    },
    {
        channel: EMessageChannel.EMAIL,
        voters: 1130,
        not_offered: 0,
        sends_from: ESendsFrom.ACCOUNT,
        account_id: SECOND_ORG_ACCOUNT_IDS.EMAIL,
        account_name: SECOND_ORG_ACCOUNT_NAMES.EMAIL,
    },
]

/** The preview's answer for the event's whole audience: notices go to 1,830 of 1,855. */
export const secondOrgAudiencePreview = (): IAudiencePreview => {
    const channels = secondOrgAudience()
    return {
        total: channels.reduce((sum, row) => sum + row.voters, 0),
        will_send: channels.reduce((sum, row) => sum + row.voters - row.not_offered, 0),
        channels,
        status: COUNTED,
    }
}

/** The organization's notification template, in its own words. */
export const secondOrgTemplates = (): StoryRecord<Sequent_Backend_Template>[] => [
    {
        id: storyId(10, 9),
        tenant_id: TENANT_ID,
        alias: "staff-voting-opens",
        communication_method: ITemplateMethod.EMAIL,
        type: "CREDENTIALS",
        template: {
            alias: "staff-voting-opens",
            selected_methods: {EMAIL: true, WHATSAPP: true, VIBER: true},
            email: {
                subject: "Staff Association Council 2028: voting opens",
                plaintext_body: "Hello {{user.first_name}}, voting opens tomorrow.",
                html_body: "<p>Hello {{user.first_name}}, voting opens tomorrow.</p>",
            },
            whatsapp: {
                message: "Hello {{1}}, the Staff Association vote opens tomorrow.",
                parameters: ["user.first_name"],
            },
            viber: {
                message: "Hello {{1}}, the Staff Association vote opens tomorrow.",
                parameters: ["user.first_name"],
            },
        },
        annotations: {},
        labels: {},
        created_by: "admin",
        created_at: CHECKED_AT,
        updated_at: CHECKED_AT,
    },
]

// ---------------------------------------------------------------------------
// The monthly volumes and the dev fake provider's Sent messages.

export const secondOrgMonthlyVolumes = (): IMonthlyVolume[] => [
    {
        channel: EMessageChannel.WHATSAPP,
        account_id: SECOND_ORG_ACCOUNT_IDS.WHATSAPP,
        account_name: SECOND_ORG_ACCOUNT_NAMES.WHATSAPP,
        destination_country: "ES",
        billing_category: "AUTHENTICATION",
        billable: true,
        messages: 702,
    },
    {
        channel: EMessageChannel.VIBER,
        account_id: SECOND_ORG_ACCOUNT_IDS.VIBER,
        account_name: SECOND_ORG_ACCOUNT_NAMES.VIBER,
        destination_country: "ES",
        billing_category: "TRANSACTIONAL",
        billable: null,
        messages: 62,
    },
    {
        channel: EMessageChannel.SMS,
        account_id: SECOND_ORG_ACCOUNT_IDS.SMS,
        account_name: SECOND_ORG_ACCOUNT_NAMES.SMS,
        destination_country: "ES",
        billing_category: "TRANSACTIONAL",
        billable: null,
        messages: 231,
    },
    {
        channel: EMessageChannel.EMAIL,
        account_id: SECOND_ORG_ACCOUNT_IDS.EMAIL,
        account_name: SECOND_ORG_ACCOUNT_NAMES.EMAIL,
        destination_country: null,
        billing_category: "EMAIL",
        billable: null,
        messages: 1205,
    },
]

export const SECOND_ORG_VOTER_NUMBER = "+34600111214"
export const SECOND_ORG_MASKED_NUMBER = "+34 ••• ••• 214"
export const SECOND_ORG_VOTER_EMAIL = "lucia@staffassoc.admin-story.invalid"

const delivered = {state: EFakeReportState.ANSWERED, status: 200, reason: null}
const noReceipts = {
    state: EFakeReportState.NO_RECEIPTS,
    status: null,
    reason: "This provider sends no delivery reports",
}

/** What the fake provider received from the organization's accounts. */
export const secondOrgSentMessages = (): IFakeProviderMessage[] => [
    {
        id: "staff-1",
        received_at: "2028-05-20T08:10:00Z",
        channel: EMessageChannel.WHATSAPP,
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        sender: "400000000000002",
        recipient: SECOND_ORG_VOTER_NUMBER,
        content: {
            text: null,
            subject: null,
            html: null,
            template_name: "staff_code",
            template_language: "es",
            parameters: [{name: null, value: "604215"}],
            code: "604215",
        },
        report: delivered,
    },
    {
        id: "staff-2",
        received_at: "2028-05-20T08:12:00Z",
        channel: EMessageChannel.MESSENGER,
        provider: EMessagingProvider.MESSENGER_SEND_API,
        sender: SECOND_ORG_PAGE_ID,
        recipient: "7304419925518842",
        content: {
            text: "Your Staff Association verification code is 318402. It expires in 5 minutes. Do not share it with anyone.",
            subject: null,
            html: null,
            template_name: null,
            template_language: null,
            parameters: [],
            code: null,
        },
        report: delivered,
    },
    {
        id: "staff-3",
        received_at: "2028-05-20T08:14:00Z",
        channel: EMessageChannel.EMAIL,
        provider: EMessagingProvider.SMTP,
        sender: SECOND_ORG_EMAIL_FROM,
        recipient: SECOND_ORG_VOTER_EMAIL,
        content: {
            text: "Hello Lucía, voting opens tomorrow.",
            subject: "Staff Association Council 2028: voting opens",
            html: null,
            template_name: null,
            template_language: null,
            parameters: [],
            code: null,
        },
        report: noReceipts,
    },
]

// ---------------------------------------------------------------------------
// A voter, and the Logs.

/** A WhatsApp voter of the Canary Islands office, with consent to the organization. */
export const secondOrgWhatsAppVoter = (): Record<string, string[]> => ({
    "sequent.read-only.message-channel": [EMessageChannel.WHATSAPP],
    "sequent.read-only.message-number": [SECOND_ORG_VOTER_NUMBER],
    "sequent.read-only.message-consent": [
        JSON.stringify({
            at: CHECKED_AT,
            version: "1",
            organization: SECOND_ORGANIZATION,
            channel: EMessageChannel.WHATSAPP,
        }),
    ],
})

/** A voter connected to the organization's Messenger Page. */
export const secondOrgMessengerVoter = (): Record<string, string[]> => ({
    "sequent.read-only.message-channel": [EMessageChannel.MESSENGER],
    "sequent.read-only.messenger-id": ["7304419925518842"],
    "sequent.read-only.messenger-page": [SECOND_ORG_PAGE_ID],
})

export const SECOND_ORG_MESSAGE_ID = "5a7e0c1d-2b3f-4a5c-8d6e-7f8091a2b3c4"

/** The voter's last notice on WhatsApp: refused, because the Post doesn't offer it. */
export const secondOrgFailedNotice = (): ILedgerMessage => ({
    id: SECOND_ORG_MESSAGE_ID,
    channel: EMessageChannel.WHATSAPP,
    state: EMessageAttemptState.FAILED,
    attempt: 1,
    logical_key: "bulk:staff-voting-opens:voter-1",
    error: "Channel not offered for notices",
    created_at: CHECKED_AT,
    updated_at: CHECKED_AT,
    failed_at: CHECKED_AT,
})

export const SECOND_ORG_LOG_USER = "lucia.martin"
export const SECOND_ORG_VIBER_MESSAGE_ID = "5a7e0c1d-2b3f-4a5c-8d6e-7f8091a2b3c5"

const sendLog = (
    id: number,
    channel: string,
    receiver: string,
    messageId: string,
    description: string
): StoryRecord<ElectoralLogRow> & {election_event_id: string} => ({
    id,
    election_event_id: EVENT_ID,
    created: 1842422400 + id * 60,
    statement_timestamp: 1842422400 + id * 60,
    statement_kind: "SendCommunications",
    user_id: STORY_IDS.user,
    message: JSON.stringify({
        user_id: STORY_IDS.user,
        username: SECOND_ORG_LOG_USER,
        election_id: CANARY_OFFICE_ID,
        statement: {
            head: {event_type: "SEND_COMMUNICATIONS", log_type: "INFO", description},
            // As windmill writes it: the channel in lowercase, inside a JSON string.
            body: {
                SendCommunications: JSON.stringify({
                    channel,
                    receiver,
                    message_id: messageId,
                    attempt: 1,
                    outcome: "ACCEPTED",
                }),
            },
        },
    }),
})

/** The Logs of the event: a notice on Viber and one on WhatsApp. */
export const secondOrgLogs = () => [
    sendLog(2, "viber", SECOND_ORG_MASKED_NUMBER, SECOND_ORG_VIBER_MESSAGE_ID, "Notice sent"),
    sendLog(1, "whatsapp", SECOND_ORG_MASKED_NUMBER, SECOND_ORG_MESSAGE_ID, "Notice sent"),
]

/** The ledger's current state of the logged messages. */
export const secondOrgLedgerStates = (): ILedgerMessage[] => [
    {
        id: SECOND_ORG_VIBER_MESSAGE_ID,
        channel: EMessageChannel.VIBER,
        state: EMessageAttemptState.DELIVERED,
        attempt: 1,
        created_at: CHECKED_AT,
        updated_at: CHECKED_AT,
    },
    secondOrgFailedNotice(),
]
