// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What the Logs tab reads from a row's message JSON (the board message as
 * windmill serializes it): the election it belongs to, the statement head and
 * the details of entries that carry them.
 */
import type {IScheduledOutcomeExplanation} from "@sequentech/ui-core"

export interface ILogStatementHead {
    event_type?: string
    log_type?: string
    description?: string
    kind?: string
}

export interface ILogMessage {
    user_id?: string | null
    username?: string | null
    election_id?: string | null
    statement?: {
        head?: ILogStatementHead
        body?: unknown
    }
}

const parsed = new WeakMap<object, ILogMessage | null>()

/** The row's message, parsed once per row; null when it isn't JSON. */
export const logMessage = (record: {message?: string | null} | null | undefined) => {
    if (!record) return null
    if (parsed.has(record)) return parsed.get(record) ?? null
    let message: ILogMessage | null = null
    try {
        const value = JSON.parse(record.message ?? "")
        message = value && typeof value === "object" ? (value as ILogMessage) : null
    } catch {
        message = null
    }
    parsed.set(record, message)
    return message
}

const isObject = (value: unknown): value is Record<string, unknown> =>
    !!value && typeof value === "object" && !Array.isArray(value)

/**
 * The details of an entry whose body carries `details_json` (signing steps
 * and the scheduled outcome entries); null for every other entry.
 */
export const logDetails = (message: ILogMessage | null): Record<string, unknown> | null => {
    const body = message?.statement?.body
    if (!isObject(body)) return null
    for (const variant of Object.values(body)) {
        if (isObject(variant) && typeof variant.details_json === "string") {
            try {
                const details = JSON.parse(variant.details_json)
                return isObject(details) ? details : null
            } catch {
                return null
            }
        }
    }
    return null
}

export const isExplanation = (value: unknown): value is IScheduledOutcomeExplanation =>
    isObject(value) && typeof value.outcome === "string" && Array.isArray(value.checks)

/** The explanation an outcome entry carries in its details, if any. */
export const outcomeExplanation = (
    details: Record<string, unknown> | null
): IScheduledOutcomeExplanation | null => {
    if (!details) return null
    if (isExplanation(details.explanation)) return details.explanation
    return isExplanation(details) ? details : null
}

/** The `{before, after}` of a ScheduledOutcomeChanged entry, if both are explanations. */
export const outcomeChange = (
    details: Record<string, unknown> | null
): {before: IScheduledOutcomeExplanation; after: IScheduledOutcomeExplanation} | null =>
    details && isExplanation(details.before) && isExplanation(details.after)
        ? {before: details.before, after: details.after}
        : null
