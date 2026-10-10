// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageChannel, EMessagingProvider, MESSAGE_CHANNELS} from "@/types/messaging"

// The shapes of the Hasura actions `messaging_fake_provider_messages` and
// `messaging_fake_provider_get_started` (plan §2.10), as harvest's
// `routes/fake_provider.rs` serializes them. They are typed here until the
// GraphQL types are generated from the applied metadata.

export interface IFakeTemplateParameter {
    /** The placeholder name of a named parameter (Infobip); null for a positional one (Meta). */
    name?: string | null
    value: string
}

/**
 * What a message said: the text, or the template with its language and
 * parameters, or the email's subject, text and HTML.
 */
export interface IFakeMessageContent {
    text?: string | null
    subject?: string | null
    /** Only in the single-entry read (`messaging_fake_provider_message`). */
    html?: string | null
    /** In the list, which leaves the HTML out: whether the email has HTML. */
    has_html?: boolean | null
    /** A field was cut: the fake keeps the first 64 KiB of each. */
    truncated?: boolean | null
    template_name?: string | null
    template_language?: string | null
    parameters?: IFakeTemplateParameter[] | null
    /** The code a code template carries: WhatsApp's copy-code button, or Infobip's `pin`. */
    code?: string | null
}

/** What became of the delivery report the fake posts for a message. */
export enum EFakeReportState {
    /** SMS (SNS) and email (SES): no report, as in production. */
    NO_RECEIPTS = "NO_RECEIPTS",
    /** Posted about a second after the message was accepted. */
    PENDING = "PENDING",
    /** The webhook component answered; `status` is its HTTP status. */
    ANSWERED = "ANSWERED",
    /** Not posted; `reason` says why. */
    NOT_SENT = "NOT_SENT",
    /** Posted, and the component didn't answer; `reason` says so. */
    UNREACHABLE = "UNREACHABLE",
}

export interface IFakeReport {
    state: EFakeReportState
    status?: number | null
    reason?: string | null
}

export interface IFakeProviderMessage {
    /** The fake's own key for the entry (not the provider message id). */
    id: string
    /** RFC 3339. */
    received_at: string
    channel: EMessageChannel
    provider: EMessagingProvider
    /** Phone number ID, Page ID, Infobip sender, SNS sender ID or From. */
    sender: string
    recipient: string
    content: IFakeMessageContent
    report?: IFakeReport | null
}

export interface IFakeProviderMessagesResult {
    messaging_fake_provider_messages?: {
        messages: IFakeProviderMessage[]
    } | null
}

export interface IFakeProviderMessageResult {
    messaging_fake_provider_message?: IFakeProviderMessage | null
}

/** The webhook component's answer to the Get Started event, or why there was none. */
export interface IFakeGetStartedAnswer {
    status?: number | null
    reason?: string | null
}

export interface IFakeProviderGetStartedResult {
    messaging_fake_provider_get_started?: IFakeGetStartedAnswer | null
}

/** The viewer's modes: every message, or one channel's app. */
export type SentMessagesMode = "ALL" | EMessageChannel

export const SENT_MESSAGES_MODES: SentMessagesMode[] = ["ALL", ...MESSAGE_CHANNELS]

/** The query parameter that keeps the mode; its presence also selects the tab. */
export const SENT_MESSAGES_PARAM = "channel"

/** How often the open tab reads the list again. */
export const SENT_MESSAGES_REFRESH_MS = 5_000

/** `?channel=` value of a mode (lower case, as typed in a link). */
export const modeParam = (mode: SentMessagesMode): string => mode.toLowerCase()

/** The mode a `?channel=` value names; anything else is All. */
export const parseMode = (value: string | null): SentMessagesMode =>
    SENT_MESSAGES_MODES.find((mode) => mode.toLowerCase() === value?.trim().toLowerCase()) ?? "ALL"
