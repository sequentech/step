// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Mirrors packages/sequent-core/src/types/messaging.rs.

export const MESSAGING_CONFIG_ANNOTATION = "messaging:config"
export const EVENT_MESSAGING_CONFIG_VERSION = 1

export enum EMessageChannel {
    EMAIL = "EMAIL",
    SMS = "SMS",
    WHATSAPP = "WHATSAPP",
    VIBER = "VIBER",
    MESSENGER = "MESSENGER",
}

export const MESSAGE_CHANNELS: EMessageChannel[] = [
    EMessageChannel.EMAIL,
    EMessageChannel.SMS,
    EMessageChannel.WHATSAPP,
    EMessageChannel.VIBER,
    EMessageChannel.MESSENGER,
]

export enum EMessagePurpose {
    OTP = "OTP",
    NOTICE = "NOTICE",
}

export const MESSAGE_PURPOSES: EMessagePurpose[] = [EMessagePurpose.OTP, EMessagePurpose.NOTICE]

export enum ERecipientKind {
    EMAIL_ADDRESS = "EMAIL_ADDRESS",
    PHONE_NUMBER = "PHONE_NUMBER",
    PAGE_SCOPED_ID = "PAGE_SCOPED_ID",
}

export enum EMessagingProvider {
    AWS_SES = "AWS_SES",
    SMTP = "SMTP",
    AWS_SNS = "AWS_SNS",
    WHATSAPP_CLOUD_API = "WHATSAPP_CLOUD_API",
    MESSENGER_SEND_API = "MESSENGER_SEND_API",
    VIBER_INFOBIP = "VIBER_INFOBIP",
    CONSOLE = "CONSOLE",
}

export enum EDeliveryFeedback {
    PROVIDER_RECEIPTS = "PROVIDER_RECEIPTS",
    UNAVAILABLE = "UNAVAILABLE",
}

export enum EMessageAttemptState {
    QUEUED = "QUEUED",
    ACCEPTED = "ACCEPTED",
    DELIVERED = "DELIVERED",
    FAILED = "FAILED",
    UNKNOWN = "UNKNOWN",
}

export const MESSAGE_ATTEMPT_STATES: EMessageAttemptState[] = [
    EMessageAttemptState.QUEUED,
    EMessageAttemptState.ACCEPTED,
    EMessageAttemptState.DELIVERED,
    EMessageAttemptState.FAILED,
    EMessageAttemptState.UNKNOWN,
]

export enum EProviderApproval {
    PENDING = "PENDING",
    CONFIRMED = "CONFIRMED",
}

export enum EOutOfWindowPolicy {
    DISABLED = "DISABLED",
    UTILITY_MESSAGES = "UTILITY_MESSAGES",
}

export enum EReadinessBlocker {
    NOT_CONNECTED = "NOT_CONNECTED",
    UNSUPPORTED_PURPOSE = "UNSUPPORTED_PURPOSE",
    NEEDS_PROVIDER_APPROVAL = "NEEDS_PROVIDER_APPROVAL",
    NEEDS_PRODUCTION_ACCESS = "NEEDS_PRODUCTION_ACCESS",
    NEEDS_APPROVED_TEMPLATE = "NEEDS_APPROVED_TEMPLATE",
}

export enum ECredentialName {
    ACCESS_TOKEN = "ACCESS_TOKEN",
    APP_SECRET = "APP_SECRET",
    VERIFY_TOKEN = "VERIFY_TOKEN",
    API_KEY = "API_KEY",
    SMTP_PASSWORD = "SMTP_PASSWORD",
    AWS_ACCESS_KEY_ID = "AWS_ACCESS_KEY_ID",
    AWS_SECRET_ACCESS_KEY = "AWS_SECRET_ACCESS_KEY",
}

export enum EChannelSelection {
    SINGLE_CHANNEL = "SINGLE_CHANNEL",
    VOTER_PREFERENCE = "VOTER_PREFERENCE",
}

export interface IProviderCapabilities {
    channel: EMessageChannel
    recipient: ERecipientKind
    purposes: EMessagePurpose[]
    template_required_for: EMessagePurpose[]
    delivery_feedback: EDeliveryFeedback
    reconciliation: boolean
    conversation_window_hours: number | null
    requires_provider_approval: boolean
}

export type IApprovedTemplates = Partial<Record<EMessagePurpose, string[]>>

export interface IAccountCheck {
    connected: boolean
    production_access: boolean
    approved_templates?: IApprovedTemplates
    checked_at?: string | null
    reason?: string | null
}

export interface IPurposeReadiness {
    purpose: EMessagePurpose
    blockers: EReadinessBlocker[]
}

export interface IAccountLimits {
    messages_per_second?: number | null
    otp_reserved_per_second?: number | null
    allowed_calling_codes?: string[]
}

/** Per purpose, language to the partner's template ID. */
export type IViberApprovedTemplates = Partial<Record<EMessagePurpose, Record<string, string>>>

export type IAccountSender =
    | {
          provider: EMessagingProvider.AWS_SES
          from_address: string
          from_name?: string | null
          region?: string | null
          notification_topic_arn?: string | null
      }
    | {
          provider: EMessagingProvider.SMTP
          from_address: string
          from_name?: string | null
          server_url: string
      }
    | {
          provider: EMessagingProvider.AWS_SNS
          sender_id?: string | null
          origination_number?: string | null
          region?: string | null
      }
    | {
          provider: EMessagingProvider.WHATSAPP_CLOUD_API
          business_account_id: string
          phone_number_id: string
          display_phone_number: string
          display_name?: string | null
          api_version: string
      }
    | {
          provider: EMessagingProvider.MESSENGER_SEND_API
          page_id: string
          page_name?: string | null
          page_username?: string | null
          api_version: string
      }
    | {
          provider: EMessagingProvider.VIBER_INFOBIP
          base_url: string
          sender: string
          approved_templates?: IViberApprovedTemplates
      }
    | {provider: EMessagingProvider.CONSOLE}

export interface ICredentialRecord {
    replaced_at: string
}

export type ICredentialRecords = Partial<Record<ECredentialName, ICredentialRecord>>

/** A row of `sequent_backend.messaging_account`. */
export interface IMessagingAccount {
    id: string
    tenant_id: string
    channel: EMessageChannel
    provider: EMessagingProvider
    name: string
    sender: IAccountSender
    credentials?: ICredentialRecords | null
    limits?: IAccountLimits | null
    provider_approval?: EProviderApproval | null
    status?: IAccountCheck | null
    webhook_key?: string | null
    is_default: boolean
    created_at?: string | null
    updated_at?: string | null
}

export interface ITemplateBinding {
    purpose: EMessagePurpose
    language: string
    provider_template: string
}

export interface IEventChannelConfig {
    channel: EMessageChannel
    account_id: string
    purposes: EMessagePurpose[]
    templates: ITemplateBinding[]
    out_of_window: EOutOfWindowPolicy
}

export interface IEventMessagingConfig {
    version: number
    channels: IEventChannelConfig[]
    notice_fallback: EMessageChannel[]
    election_channels: Record<string, EMessageChannel[]>
    reply_text: Record<string, string>
}

export type IMessagingConfigError =
    | {kind: "UNSUPPORTED_VERSION"; version: number}
    | {kind: "DUPLICATE_CHANNEL"; channel: EMessageChannel}
    | {kind: "UNKNOWN_ACCOUNT"; channel: EMessageChannel; account_id: string}
    | {kind: "ACCOUNT_OF_ANOTHER_TENANT"; account_id: string}
    | {kind: "ACCOUNT_CHANNEL_MISMATCH"; channel: EMessageChannel; account_id: string}
    | {
          kind: "PURPOSE_NOT_READY"
          channel: EMessageChannel
          purpose: EMessagePurpose
          blockers: EReadinessBlocker[]
      }
    | {
          kind: "TEMPLATE_NOT_APPROVED"
          channel: EMessageChannel
          purpose: EMessagePurpose
          language: string
      }
    | {kind: "OUT_OF_WINDOW_NOT_SUPPORTED"; channel: EMessageChannel}
    | {kind: "FALLBACK_CHANNEL_NOT_ENABLED"; channel: EMessageChannel}
    | {kind: "DUPLICATE_FALLBACK_CHANNEL"; channel: EMessageChannel}
    | {kind: "ELECTION_CHANNEL_NOT_ENABLED"; election_id: string; channel: EMessageChannel}
    | {kind: "UNKNOWN_ELECTION"; election_id: string}

export type EMessagingConfigErrorKind = IMessagingConfigError["kind"]

/** What validation needs to know about an account. */
export interface IAccountSummary {
    id: string
    tenant_id: string
    channel: EMessageChannel
    provider: EMessagingProvider
    provider_approval: EProviderApproval
    check: IAccountCheck
}

export interface IInstantMessageConfig {
    message: string
    parameters: string[]
}

/** Keycloak user attributes of a voter's messaging channels. */
export const VOTER_ATTR_MESSAGE_CHANNEL = "sequent.read-only.message-channel"
export const VOTER_ATTR_WHATSAPP_NUMBER = "sequent.read-only.whatsapp-number"
export const VOTER_ATTR_VIBER_NUMBER = "sequent.read-only.viber-number"
export const VOTER_ATTR_MESSENGER_ID = "sequent.read-only.messenger-id"
export const VOTER_ATTR_MESSENGER_PAGE = "sequent.read-only.messenger-page"
export const VOTER_ATTR_VERIFIED_CHANNELS = "sequent.read-only.verified-channels"
