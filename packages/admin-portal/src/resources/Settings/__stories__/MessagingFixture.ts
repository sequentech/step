// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic sending accounts of the Settings > Messaging tab.
import {FIXED_TIME, storyId} from "@/__stories__/fixtures"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EProviderApproval,
    IMessagingAccount,
} from "@/types/messaging"

export const EMAIL_ACCOUNT_ID = storyId(6, 1)
export const SMS_ACCOUNT_ID = storyId(6, 2)
export const WHATSAPP_ACCOUNT_ID = storyId(6, 3)
export const VIBER_ACCOUNT_ID = storyId(6, 4)
export const MESSENGER_ACCOUNT_ID = storyId(6, 5)

const checked = (approvedTemplates: Partial<Record<EMessagePurpose, string[]>> = {}) => ({
    connected: true,
    production_access: true,
    approved_templates: approvedTemplates,
    checked_at: FIXED_TIME,
    reason: null,
})

const base = {
    tenant_id: TENANT_ID,
    limits: {allowed_calling_codes: []},
    provider_approval: EProviderApproval.PENDING,
    created_at: FIXED_TIME,
    updated_at: FIXED_TIME,
}

export const emailAccount = (): IMessagingAccount => ({
    ...base,
    id: EMAIL_ACCOUNT_ID,
    channel: EMessageChannel.EMAIL,
    provider: EMessagingProvider.AWS_SES,
    name: "Council email",
    sender: {
        provider: EMessagingProvider.AWS_SES,
        from_address: "no-reply@council.admin-story.invalid",
        from_name: "Council elections",
        region: "eu-west-1",
        notification_topic_arn: "arn:aws:sns:eu-west-1:000000000000:ses-events",
    },
    credentials: {},
    status: checked(),
    webhook_key: null,
    is_default: true,
})

export const smsAccount = (): IMessagingAccount => ({
    ...base,
    id: SMS_ACCOUNT_ID,
    channel: EMessageChannel.SMS,
    provider: EMessagingProvider.AWS_SNS,
    name: "Council SMS",
    sender: {
        provider: EMessagingProvider.AWS_SNS,
        sender_id: "COUNCIL",
        origination_number: null,
        region: "eu-west-1",
    },
    credentials: {},
    limits: {
        messages_per_second: 20,
        otp_reserved_per_second: 5,
        allowed_calling_codes: ["34", "63"],
    },
    status: {
        connected: false,
        production_access: false,
        approved_templates: {},
        checked_at: FIXED_TIME,
        reason: "The account is in the SMS sandbox.",
    },
    webhook_key: null,
    is_default: true,
})

export const whatsappAccount = (): IMessagingAccount => ({
    ...base,
    id: WHATSAPP_ACCOUNT_ID,
    channel: EMessageChannel.WHATSAPP,
    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
    name: "Council WhatsApp",
    sender: {
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        business_account_id: "100000000000001",
        phone_number_id: "200000000000002",
        display_phone_number: "+34 600 000 001",
        display_name: "Council elections",
        api_version: "v23.0",
    },
    credentials: {
        ACCESS_TOKEN: {replaced_at: FIXED_TIME},
        APP_SECRET: {replaced_at: FIXED_TIME},
        VERIFY_TOKEN: {replaced_at: FIXED_TIME},
    },
    status: checked({NOTICE: ["en"]}),
    webhook_key: "wa-hook-key",
    is_default: true,
})

export const viberAccount = (): IMessagingAccount => ({
    ...base,
    id: VIBER_ACCOUNT_ID,
    channel: EMessageChannel.VIBER,
    provider: EMessagingProvider.VIBER_INFOBIP,
    name: "Council Viber",
    sender: {
        provider: EMessagingProvider.VIBER_INFOBIP,
        base_url: "https://council.api.infobip.admin-story.invalid",
        sender: "Council",
        approved_templates: {OTP: {en: "88213", es: "88214"}, NOTICE: {en: "88215"}},
    },
    credentials: {API_KEY: {replaced_at: FIXED_TIME}},
    status: checked({OTP: ["en", "es"], NOTICE: ["en"]}),
    webhook_key: "viber-hook-key",
    is_default: true,
})

export const messengerAccount = (): IMessagingAccount => ({
    ...base,
    id: MESSENGER_ACCOUNT_ID,
    channel: EMessageChannel.MESSENGER,
    provider: EMessagingProvider.MESSENGER_SEND_API,
    name: "Council Page",
    sender: {
        provider: EMessagingProvider.MESSENGER_SEND_API,
        page_id: "118204557331906",
        page_name: "Council Elections",
        page_username: "councilelections",
        api_version: "v23.0",
    },
    credentials: {ACCESS_TOKEN: {replaced_at: FIXED_TIME}},
    status: null,
    webhook_key: "messenger-hook-key",
    is_default: false,
})

export const messagingAccounts = (): IMessagingAccount[] => [
    emailAccount(),
    smsAccount(),
    whatsappAccount(),
    viberAccount(),
    messengerAccount(),
]
