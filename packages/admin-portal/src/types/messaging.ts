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

/** Channel order everywhere: WhatsApp, Viber, Messenger, SMS, Email. */
export const MESSAGE_CHANNELS: EMessageChannel[] = [
    EMessageChannel.WHATSAPP,
    EMessageChannel.VIBER,
    EMessageChannel.MESSENGER,
    EMessageChannel.SMS,
    EMessageChannel.EMAIL,
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
    /** Parse-only: an account saved with it loads as unsupported and can't send. */
    HTTP_API = "HTTP_API",
    /** Parse-only: an account saved with it loads as unsupported and can't send. */
    CONSOLE = "CONSOLE",
}

/** Providers that were removed: their accounts still load, to be replaced. */
export const UNSUPPORTED_PROVIDERS: EMessagingProvider[] = [
    EMessagingProvider.HTTP_API,
    EMessagingProvider.CONSOLE,
]

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

/**
 * How an account's readiness is established. Readiness always comes from the
 * provider's check; `ADMIN_CONFIRMED` is parse-only, read as `PROVIDER_CHECK`
 * and never written. Whether the check counts is the check's identity
 * (`checkIsFor`), never a stored policy.
 */
export enum EReadinessPolicy {
    PROVIDER_CHECK = "PROVIDER_CHECK",
    ADMIN_CONFIRMED = "ADMIN_CONFIRMED",
}

/** Where an account's provider approval comes from (`AccountCheck::approval`). */
export enum EApprovalSource {
    /** The provider reports it, and the check read it. */
    PROVIDER_CHECK = "PROVIDER_CHECK",
    /** The provider has no API for it: an administrator entered it. */
    ADMIN_ENTERED = "ADMIN_ENTERED",
}

export enum EReadinessBlocker {
    UNSUPPORTED_PROVIDER = "UNSUPPORTED_PROVIDER",
    /**
     * The account's last check was of another identity (an earlier number,
     * Page or sender): nothing it said counts until it is checked again.
     */
    UNCHECKED = "UNCHECKED",
    NOT_CONNECTED = "NOT_CONNECTED",
    UNSUPPORTED_PURPOSE = "UNSUPPORTED_PURPOSE",
    NEEDS_PROVIDER_APPROVAL = "NEEDS_PROVIDER_APPROVAL",
    NEEDS_PRODUCTION_ACCESS = "NEEDS_PRODUCTION_ACCESS",
    NEEDS_APPROVED_TEMPLATE = "NEEDS_APPROVED_TEMPLATE",
    /**
     * The webhook component's public address isn't configured: no callback
     * reaches the account. Messenger codes need Get Started, which arrives as
     * a callback; receipts are unavailable (`callback_readiness`).
     */
    CALLBACK_NOT_CONFIGURED = "CALLBACK_NOT_CONFIGURED",
    /**
     * A Viber account without report credentials: Infobip's subscription
     * can't authenticate its reports, so receipts are unavailable.
     */
    REPORT_CREDENTIALS_MISSING = "REPORT_CREDENTIALS_MISSING",
}

export enum ECredentialName {
    ACCESS_TOKEN = "ACCESS_TOKEN",
    APP_SECRET = "APP_SECRET",
    VERIFY_TOKEN = "VERIFY_TOKEN",
    API_KEY = "API_KEY",
    SMTP_PASSWORD = "SMTP_PASSWORD",
    AWS_ACCESS_KEY_ID = "AWS_ACCESS_KEY_ID",
    AWS_SECRET_ACCESS_KEY = "AWS_SECRET_ACCESS_KEY",
    /** Basic auth Infobip's report subscription sends; generated, shown once. */
    REPORT_USERNAME = "REPORT_USERNAME",
    REPORT_PASSWORD = "REPORT_PASSWORD",
}

/** Credentials Step generates and shows once; never typed. */
export const GENERATED_CREDENTIALS: ECredentialName[] = [
    ECredentialName.VERIFY_TOKEN,
    ECredentialName.REPORT_USERNAME,
    ECredentialName.REPORT_PASSWORD,
]

/** Parse-only: a notification goes to each voter's own channel. Never written. */
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

/** Meta's `debug_token` of the account's access token. */
export interface ITokenStatus {
    valid: boolean
    /** Absent when the token doesn't expire. */
    expires_at?: string | null
    missing_scopes?: string[]
}

/** A WhatsApp number as the Cloud API describes it; values are Meta's. */
export interface IWhatsAppNumberStatus {
    status?: string | null
    name_status?: string | null
    quality_rating?: string | null
    messaging_limit?: string | null
}

/** A template at the provider, for one language. */
export interface ITemplateStatus {
    name: string
    language: string
    category?: string | null
    status: string
}

/** A Messenger Page's setup; `null`: not reported. */
export interface IMessengerPageStatus {
    pages_messaging?: boolean | null
    subscribed: boolean
    get_started?: boolean | null
}

export interface IPartnerLogin {
    accepted: boolean
    reason?: string | null
}

export interface ICallbackStatus {
    url?: string | null
    reason?: string | null
}

/** A WhatsApp `user_changed_number` signal: masked numbers and the voters to verify again. */
export interface INumberChange {
    masked_old_number: string
    masked_new_number?: string | null
    voter_ids?: string[]
    reported_at: string
}

/** What the provider reported beyond connection (`AccountCheckDetails`). Every part is optional. */
export interface IAccountCheckDetails {
    token?: ITokenStatus | null
    whatsapp_number?: IWhatsAppNumberStatus | null
    templates?: ITemplateStatus[]
    templates_reason?: string | null
    messenger?: IMessengerPageStatus | null
    partner_login?: IPartnerLogin | null
    callback?: ICallbackStatus | null
    /** Reported by the provider's check; absent where the provider has no API for it. */
    provider_approval?: EProviderApproval | null
    provider_approval_reason?: string | null
    number_changes?: INumberChange[]
    /** The identity the check checked; absent in checks stored before it existed. */
    identity?: IAccountIdentity | null
    /** When the check started. */
    started_at?: string | null
    /** When a check last failed to run; the daily pass backs off from it. */
    attempted_at?: string | null
    /** Meta: whether the app secret matched the token's app. */
    app_secret?: IAppSecretStatus | null
}

/** What a check is a check of (`AccountIdentity`): the provider and the sender's ids. */
export interface IAccountIdentity {
    provider: EMessagingProvider
    ids: Record<string, string>
}

export interface IAppSecretStatus {
    /** `false`: Meta refused it; `null`: it couldn't be checked (`reason`). */
    valid?: boolean | null
    reason?: string | null
}

export interface IAccountCheck {
    connected: boolean
    production_access: boolean
    approved_templates?: IApprovedTemplates
    checked_at?: string | null
    reason?: string | null
    /** Absent in checks stored before it existed. */
    details?: IAccountCheckDetails | null
    /**
     * What stops callbacks (delivery reports, Messenger's Get Started) from
     * reaching the account, from the action `messaging_callback_status`
     * (`callback_readiness`), merged in by the account reader. Absent when
     * the status couldn't be read.
     */
    callback_blockers?: Array<EReadinessBlocker | string>
    /** The backend's reasons for `callback_blockers`, in the same order. */
    callback_reasons?: string[]
}

/** One account's entry of `messaging_callback_status`. */
export interface IAccountCallbackStatus {
    account_id: string
    blockers: string[]
    reasons: string[]
}

export interface IPurposeReadiness {
    purpose: EMessagePurpose
    blockers: EReadinessBlocker[]
}

export interface IAccountLimits {
    messages_per_second?: number | null
    otp_reserved_per_second?: number | null
    /**
     * Codes sent at once through the account, per harvest process; further
     * codes wait their turn. Absent: 8.
     */
    otp_concurrency?: number | null
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
          api_base_url?: string | null
      }
    | {
          provider: EMessagingProvider.MESSENGER_SEND_API
          page_id: string
          page_name?: string | null
          page_username?: string | null
          api_version: string
          api_base_url?: string | null
      }
    | {
          provider: EMessagingProvider.VIBER_INFOBIP
          base_url: string
          sender: string
          approved_templates?: IViberApprovedTemplates
      }
    /** Unsupported; its other fields are ignored. */
    | {provider: EMessagingProvider.HTTP_API; label?: string | null}
    /** Unsupported. */
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
    readiness?: EReadinessPolicy | null
    status?: IAccountCheck | null
    is_default: boolean
    created_at?: string | null
    updated_at?: string | null
}

export interface ITemplateBinding {
    purpose: EMessagePurpose
    /** A template alias or Keycloak's message key; absent for the purpose's default. */
    key?: string | null
    /** The voter's language. */
    language: string
    provider_template: string
    /** The provider's code for the language when it differs, such as `en_US` for `en`. */
    provider_language?: string | null
}

/**
 * Message keys Keycloak sends as `template_key`, offered when binding an
 * approved template to a specific message. Mirrors the `template_key` values
 * of the Keycloak messaging extensions: `otp` for a one-time code, `otl`
 * for a one-time link, and the message key of each notice.
 */
export const KEYCLOAK_OTP_MESSAGE_KEYS: string[] = ["otp", "otl"]

export const KEYCLOAK_NOTICE_MESSAGE_KEYS: string[] = [
    "messageSuccessSms",
    "messageSuccessSmsKiosk",
    "messagePendingSms",
    "messageRejectedSms",
    "forgotPassword.sms.text",
    "newPassword.message.text",
    "contactChanged.notice.text",
]

export interface IEventChannelConfig {
    channel: EMessageChannel
    account_id: string
    purposes: EMessagePurpose[]
    templates: ITemplateBinding[]
    out_of_window: EOutOfWindowPolicy
}

/**
 * A row of `messaging_event_config`. Configurations saved before the second
 * pass may also hold `notice_fallback` and `reply_text`: they are ignored and
 * never written again.
 */
export interface IEventMessagingConfig {
    version: number
    channels: IEventChannelConfig[]
    election_channels: Record<string, EMessageChannel[]>
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
    | {kind: "UTILITY_TEMPLATE_MISSING"; channel: EMessageChannel}
    | {kind: "ELECTION_CHANNEL_NOT_ENABLED"; election_id: string; channel: EMessageChannel}
    | {kind: "UNKNOWN_ELECTION"; election_id: string}

export type EMessagingConfigErrorKind = IMessagingConfigError["kind"]

/** What validation needs to know about an account. */
export interface IAccountSummary {
    id: string
    tenant_id: string
    channel: EMessageChannel
    provider: EMessagingProvider
    /** What the account can do; `null` when its sender cannot serve the channel. */
    capabilities: IProviderCapabilities | null
    provider_approval: EProviderApproval
    /** The account's identity now: `check` counts only if it is of it. */
    identity?: IAccountIdentity | null
    check: IAccountCheck
    public_label?: string | null
    messenger_page?: IMessengerPage | null
}

export interface IMessengerPage {
    page_id: string
    username?: string | null
    name?: string | null
}

export interface IPublicChannel {
    channel: EMessageChannel
    purposes: EMessagePurpose[]
    sender_label: string | null
    messenger_page: IMessengerPage | null
}

/** The `sequent.messaging` realm attribute. */
export interface IPublicMessagingChannels {
    version: number
    channels: IPublicChannel[]
    election_channels: Record<string, EMessageChannel[]>
    election_labels: Record<string, string[]>
}

export interface IInstantMessageConfig {
    message: string
    parameters: string[]
    /** The approved template at the provider; the event's binding is used without it. */
    provider_template?: string | null
    provider_language?: string | null
}

/** Keycloak user attributes of a voter's one channel (written after a verified code). */
export const VOTER_ATTR_MESSAGE_CHANNEL = "sequent.read-only.message-channel"
export const VOTER_ATTR_MESSAGE_NUMBER = "sequent.read-only.message-number"
export const VOTER_ATTR_MESSENGER_ID = "sequent.read-only.messenger-id"
export const VOTER_ATTR_MESSENGER_PAGE = "sequent.read-only.messenger-page"
export const VOTER_ATTR_MESSAGE_CONSENT = "sequent.read-only.message-consent"
export const VOTER_ATTR_MOBILE_NUMBER = "sequent.read-only.mobile-number"
/** Read mapping only: a record of the first messaging release. Never written. */
export const VOTER_ATTR_WHATSAPP_NUMBER = "sequent.read-only.whatsapp-number"
export const VOTER_ATTR_VIBER_NUMBER = "sequent.read-only.viber-number"
export const VOTER_ATTR_VERIFIED_CHANNELS = "sequent.read-only.verified-channels"

/** A ledger row (`sequent_backend.message`) as the event tab, Logs and the drawer read it. */
export interface ILedgerMessage {
    id: string
    /** Null for a voter with no contact channel: the FAILED row has none. */
    channel: EMessageChannel | string | null
    state: EMessageAttemptState | string
    /** The message the attempt belongs to: retries share it. */
    logical_key?: string | null
    attempt?: number | null
    error?: string | null
    created_at?: string | null
    updated_at?: string | null
    failed_at?: string | null
}

/** Messages of a channel in an event, counted per message (its latest attempt). */
export interface IChannelMessageStats {
    /** Null for the voters with no contact channel: their FAILED rows have none. */
    channel: EMessageChannel | null
    /** Voters whose one channel this is; null when it couldn't be counted. */
    voters?: number | null
    sent_last_24h?: number | null
    queued: number
    accepted: number
    delivered: number
    failed: number
    /** Unknown for less than 24 hours: delivery isn't confirmed yet. */
    unknown: number
    /** Unknown for more than 24 hours: delivery couldn't be confirmed. Never counted as failed. */
    unresolved: number
}
