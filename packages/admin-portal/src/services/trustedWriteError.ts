// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const record = (value: unknown): Record<string, unknown> =>
    value !== null && typeof value === "object" ? (value as Record<string, unknown>) : {}

/** Recognize only our trigger refusals, preserving unrelated permission errors. */
export const getTrustedWriteErrorMessage = (error: unknown): string | undefined => {
    const outer = record(error)
    const body = record(outer.body)
    const source = Array.isArray(outer.graphQLErrors) ? outer : body
    const errors = Array.isArray(source.graphQLErrors) ? source.graphQLErrors : []
    const messages: unknown[] = [outer.message]
    for (const entry of errors) {
        const item = record(entry)
        messages.push(item.message)
        messages.push(record(record(record(item.extensions).internal).error).message)
    }
    for (const message of messages) {
        if (typeof message !== "string") continue
        if (message.includes("can only change through the voting status actions")) {
            return "Voting state changed or cannot be saved directly. Reload, then use the Publish tab or a scheduled event to open, pause or close voting."
        }
        if (message.includes("can only change through a scheduled lockdown event")) {
            return "Lockdown state changed or cannot be saved directly. Reload, then schedule a start or end of the lockdown period."
        }
        if (
            message.includes(
                "Initialization report state can only change through report generation"
            )
        ) {
            return "Initialization state cannot be saved directly. Reload, then generate the initialization report from the Publish tab."
        }
        if (
            message.includes(
                "Scheduled execution and prediction results can only be written by the scheduler"
            )
        ) {
            return "Scheduled results changed since this form was loaded. Reload before saving; execution results and predictions are read-only."
        }
        if (
            message.includes("grace period") &&
            message.includes("can only change before voting starts")
        ) {
            return "The grace period cannot change after voting has started. Reload to restore the saved settings."
        }
    }
    return undefined
}

/** ra-data-graphql retains Apollo errors in HttpError.body. */
export const rethrowTrustedWriteError = (error: unknown): never => {
    const message = getTrustedWriteErrorMessage(error)
    if (message) throw new Error(message)
    throw error
}
