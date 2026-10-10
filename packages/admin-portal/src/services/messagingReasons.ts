// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {IGraphQLActionError} from "@sequentech/ui-core"
import {parseActionResponseBody} from "@/services/graphqlActionError"

/**
 * The backend gives every refused send, unavailable channel and failed
 * callback one reason string, shared by the executor, the log and the UI.
 * Each string here is the backend's text, verbatim, mapped to the key of its
 * translation under `messaging.reason`. Any reason not listed is shown as the
 * backend wrote it, so a reason is never hidden.
 *
 * Sources: `sequent_core::types::messaging::MessageReason`, the preflight
 * refusals (`messaging::sender::PreflightError`), the account refusals in
 * `windmill::postgres::messaging` and `callback_readiness`.
 */
export const KNOWN_REASONS: Record<string, string> = {
    // MessageReason
    "No contact channel": "NO_CONTACT_CHANNEL",
    "Channel not offered for notices": "CHANNEL_NOT_OFFERED_FOR_NOTICES",
    "Channel not offered for codes": "CHANNEL_NOT_OFFERED_FOR_CODES",
    "No content for the channel": "NO_CONTENT_FOR_CHANNEL",
    "No sending account for the channel": "NO_ACCOUNT_FOR_CHANNEL",
    "Invalid destination for the channel": "INVALID_DESTINATION",
    "Can't be retried: no stored template": "NOT_RETRYABLE_NO_TEMPLATE",
    "Can't be retried: contains protected fields": "NOT_RETRYABLE_PROTECTED_FIELDS",
    "Reported too late to retry": "REPORTED_TOO_LATE",
    "Can't be retried: the message couldn't be prepared again": "RETRY_NOT_PREPARED",
    "Not the voter's channel": "NOT_THE_VOTERS_CHANNEL",
    "The voter's channel changed: not retried": "CHANNEL_CHANGED",
    "The worker stopped before sending": "WORKER_STOPPED_BEFORE_SENDING",
    "The worker stopped before recording the outcome": "WORKER_STOPPED_AFTER_SENDING",
    "The code expired before it could be sent": "CODE_EXPIRED_BEFORE_SENDING",
    "Delivery couldn't be confirmed": "UNRESOLVED",
    "Select at least one voter": "SELECTED_WITHOUT_VOTERS",
    "Unsupported provider: replace this account": "UNSUPPORTED_PROVIDER",
    // Preflight refusals, stored by their code
    "WRONG_RECIPIENT_KIND": "WRONG_RECIPIENT_KIND",
    "UNSUPPORTED_PURPOSE": "UNSUPPORTED_PURPOSE",
    "MISSING_APPROVED_TEMPLATE": "MISSING_APPROVED_TEMPLATE",
    "OUTSIDE_CONVERSATION_WINDOW": "OUTSIDE_CONVERSATION_WINDOW",
    "DESTINATION_NOT_ALLOWED": "DESTINATION_NOT_ALLOWED",
    "CODE_EXPIRED": "CODE_EXPIRED",
    "MISSING_CODE": "MISSING_CODE",
    // Accounts
    "Callback address not configured": "CALLBACK_NOT_CONFIGURED",
    "Callback address must be https": "CALLBACK_NOT_HTTPS",
    "Generate the report credentials and create the Infobip subscription":
        "REPORT_CREDENTIALS_MISSING",
    "Enter the credentials again for the new address": "CREDENTIALS_FOLLOW_ENDPOINT",
    "The provider address must start with https://": "ENDPOINT_NOT_HTTPS",
}

/** A failure the provider reported by its code only (`provider code <code>`). */
const PROVIDER_CODE = /^provider code (.+)$/

/**
 * Reasons joined by the backend: the failure, then why it isn't retried. The
 * real failure always stays visible next to the "can't be retried" reason.
 */
const REASON_SEPARATOR = "; "

const oneReason = (t: TFunction, text: string): string => {
    const code = KNOWN_REASONS[text]
    if (code) return String(t(`messaging.reason.${code}`))
    const providerCode = PROVIDER_CODE.exec(text)
    if (providerCode) return String(t("messaging.reason.PROVIDER_CODE", {code: providerCode[1]}))
    return text
}

/**
 * A stored reason, translated. A failure that isn't retried is stored as
 * `"<failure>; <why not retried>"` (`MessageReason::split_stored`): split on
 * the last separator, so a provider's own text may hold one, and both parts
 * are shown, each translated.
 */
export const reasonText = (t: TFunction, reason: string | null | undefined): string => {
    const text = reason?.trim()
    if (!text) return ""
    if (KNOWN_REASONS[text]) return oneReason(t, text)
    const at = text.lastIndexOf(REASON_SEPARATOR)
    const notRetried = at >= 0 ? text.slice(at + REASON_SEPARATOR.length).trim() : ""
    if (at < 0 || !KNOWN_REASONS[notRetried]) return oneReason(t, text)
    const failure = text.slice(0, at).trim()
    return [failure && oneReason(t, failure), oneReason(t, notRetried)]
        .filter(Boolean)
        .join(REASON_SEPARATOR)
}

const firstLine = (value: unknown): string | undefined => {
    if (typeof value !== "string") return undefined
    const line = value.split("\n")[0].trim()
    return line ? line.slice(0, 300) : undefined
}

/**
 * Why a messaging action was refused, as harvest wrote it. Harvest answers a
 * refusal with its reason; Hasura forwards it as the error message when it is
 * JSON (`{message}`), and otherwise keeps the plain-text body in
 * `extensions.internal.response.body` under a generic message of its own, so
 * the body is read first. `undefined` when no reason came with the error.
 */
export const actionRefusalReason = (error: unknown): string | undefined => {
    const actionError = error as IGraphQLActionError | undefined
    const graphQLErrors = Array.isArray(actionError?.graphQLErrors) ? actionError.graphQLErrors : []
    for (const graphQLError of graphQLErrors) {
        const body = graphQLError.extensions?.internal?.response?.body
        const parsed = parseActionResponseBody(body) as {message?: unknown} | undefined
        const reason =
            firstLine(parsed?.message) ??
            (parsed === undefined ? firstLine(body) : undefined) ??
            firstLine(graphQLError.message)
        if (reason) return reason
    }
    return firstLine(actionError?.message)
}
