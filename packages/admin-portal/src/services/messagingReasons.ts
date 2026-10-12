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
 * `windmill::postgres::messaging` and `windmill::services::messaging::accounts`,
 * the provider checks (`messaging::providers`) and `callback_readiness`,
 * and harvest's and windmill's refusals of an account or insights request.
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
    "Not sent: the voter is connected to another Messenger Page": "MESSENGER_PAGE_CHANGED",
    // The account's shared sending capacity (windmill::postgres::rate_bucket)
    "Not sent: the account stayed at its sending limit for too long": "CAPACITY_BUSY",
    // windmill::services::messaging::dispatch: the provider's answer came
    // after the caller's deadline (harvest answers before Keycloak's timeout)
    "The provider didn't answer in time: the message may have been sent": "PROVIDER_ANSWER_LATE",
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
    // Account checks and changes (windmill::services::messaging::accounts,
    // windmill::postgres::messaging, the provider checks)
    "The new credentials didn't connect: the previous ones are kept": "CANDIDATE_REFUSED",
    "Used by an election event's messaging settings: remove it there first": "ACCOUNT_IN_USE",
    "A voter changed their WhatsApp number: ask them to verify their contact again":
        "NUMBER_CHANGED",
    "The account's sending identity changed: it is checked again": "IDENTITY_CHANGED",
    "The account changed while it was checked: check it again": "CHECK_DROPPED",
    "The provider's templates couldn't be read": "TEMPLATES_NOT_READ",
    "Infobip refused the account's API key": "PARTNER_LOGIN_REFUSED",
    "the Page token lacks the pages_messaging permission": "NO_PAGES_MESSAGING",
    "the app is not subscribed to the Page": "NOT_SUBSCRIBED",
    "The number's status couldn't be read": "NUMBER_STATUS_NOT_READ",
    // Provider checks (messaging::providers), R5-15
    "The provider couldn't be reached": "PROVIDER_UNREACHABLE",
    "The number is in sandbox mode": "NUMBER_IN_SANDBOX",
    "The number's code is not verified": "NUMBER_NOT_VERIFIED",
    "The account is in the SMS sandbox": "SMS_SANDBOX",
    "The account is in the SES sandbox": "SES_SANDBOX",
    "Sending is disabled for this account": "SES_SENDING_DISABLED",
    "SMTP server unreachable or credentials rejected": "SMTP_REFUSED",
    "The access token isn't valid or has expired": "TOKEN_INVALID",
    "The access token lacks the permission to send": "TOKEN_CANNOT_SEND",
    "The access token couldn't be checked": "TOKEN_NOT_CHECKED",
    "Meta refused the app secret": "APP_SECRET_REFUSED",
    "The app secret couldn't be checked": "APP_SECRET_NOT_CHECKED",
    // harvest::routes::messaging (R7-20): an account deleted in another tab,
    // and a failure inside harvest
    "unknown account": "UNKNOWN_ACCOUNT",
    "internal error": "INTERNAL_ERROR",
    // windmill::services::messaging::accounts::entered_credentials
    "the verify token is generated, not entered": "VERIFY_TOKEN_GENERATED",
    // Insights (harvest::routes::messaging_insights, windmill's insights)
    "Unknown audience": "UNKNOWN_AUDIENCE",
    "The month must be written as YYYY-MM": "INVALID_MONTH",
    "Not a support reference": "INVALID_SUPPORT_REFERENCE",
}

/**
 * Reasons with a value in them, as the backend writes them: the start of the
 * text (a constant in the backend's sources), the translation key, and the
 * name of the value the translation takes. `translate` turns the value into
 * text where it is one of ours (a blocker, a credential); a provider's own
 * value (a status code, its error) is shown as it came.
 */
export interface IReasonShape {
    prefix: string
    suffix?: string
    code: string
    param: string
    translate?: (t: TFunction, value: string) => string
}

/** The value's own label under `base`, or the value as it came when it has none. */
const translatedKey = (base: string) => (t: TFunction, value: string) =>
    /^[A-Z_]+$/.test(value) ? String(t(`${base}.${value}`, {defaultValue: value})) : value

export const REASON_SHAPES: IReasonShape[] = [
    // windmill::services::messaging::accounts::not_ready_reason (R5-01)
    {
        prefix: "Not sent: the account isn't ready (",
        suffix: ")",
        code: "ACCOUNT_NOT_READY",
        param: "blocker",
        translate: translatedKey("messaging.blocker"),
    },
    // messaging::providers (R5-15)
    {prefix: "The provider answered with status ", code: "PROVIDER_STATUS", param: "status"},
    {
        prefix: "Missing credential: ",
        code: "MISSING_CREDENTIAL",
        param: "credential",
        translate: translatedKey("messaging.credential"),
    },
    {prefix: "The provider's check failed: ", code: "PROVIDER_CHECK_FAILED", param: "detail"},
    {prefix: "The number's status is ", code: "NUMBER_STATUS", param: "status"},
    {prefix: "The number's display name is ", code: "NUMBER_NAME_STATUS", param: "status"},
]

/**
 * Reasons with several values in them: the backend's format string, verbatim
 * (`{}` or `{name}` for each value), the translation key, and the name the
 * translation gives each value, in order. The values are the backend's enum
 * names (a credential, a provider), each translated.
 */
export interface IReasonPattern {
    rust: string
    code: string
    params: Array<{name: string; translate: (t: TFunction, value: string) => string}>
}

const CREDENTIAL = {name: "credential", translate: translatedKey("messaging.credential")}
const PROVIDER = {name: "provider", translate: translatedKey("messaging.provider")}

// windmill::services::messaging::accounts: `entered_credentials` and
// `generate_report_credentials` (R7-20).
export const REASON_PATTERNS: IReasonPattern[] = [
    {rust: "{name} is generated, not entered", code: "CREDENTIAL_GENERATED", params: [CREDENTIAL]},
    {
        rust: "{} accounts do not use {name}",
        code: "CREDENTIAL_NOT_USED",
        params: [PROVIDER, CREDENTIAL],
    },
    {rust: "{name} cannot be empty", code: "CREDENTIAL_EMPTY", params: [CREDENTIAL]},
    {
        rust: "{} accounts have no report credentials",
        code: "NO_REPORT_CREDENTIALS",
        params: [PROVIDER],
    },
]

const escapeRegExp = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")

/** The pattern's format as a regular expression: each value an enum name. */
const patternRegExp = (pattern: IReasonPattern) =>
    new RegExp(
        `^${pattern.rust
            .split(/\{\w*\}/)
            .map(escapeRegExp)
            .join("([A-Z_]+)")}$`
    )

const PATTERNS = REASON_PATTERNS.map((pattern) => ({pattern, regExp: patternRegExp(pattern)}))

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
    for (const shape of REASON_SHAPES) {
        const suffix = shape.suffix ?? ""
        if (!text.startsWith(shape.prefix) || !text.endsWith(suffix)) continue
        const value = text.slice(shape.prefix.length, text.length - suffix.length).trim()
        if (!value) continue
        return String(
            t(`messaging.reason.${shape.code}`, {
                [shape.param]: shape.translate ? shape.translate(t, value) : value,
            })
        )
    }
    for (const {pattern, regExp} of PATTERNS) {
        const match = regExp.exec(text)
        if (!match) continue
        return String(
            t(
                `messaging.reason.${pattern.code}`,
                Object.fromEntries(
                    pattern.params.map(({name, translate}, index) => [
                        name,
                        translate(t, match[index + 1]),
                    ])
                )
            )
        )
    }
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
