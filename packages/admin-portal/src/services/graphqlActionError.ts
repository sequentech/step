// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IGraphQLActionError} from "@sequentech/ui-core"

// A failed action answers with whatever its handler wrote, which for a
// gateway or proxy failure can be an HTML page or a stack trace. Only the
// first line is offered to a form, and only as much of it as a notification
// can reasonably show.
const MAX_REASON_LENGTH = 200

export const parseActionResponseBody = (body: string | null | undefined): unknown => {
    if (!body) {
        return undefined
    }
    try {
        return JSON.parse(body)
    } catch {
        return undefined
    }
}

const readableReason = (value: unknown): string | undefined => {
    if (typeof value !== "string") {
        return undefined
    }
    const firstLine = value.split("\n")[0].trim()
    if (firstLine.length === 0) {
        return undefined
    }

    return firstLine.length > MAX_REASON_LENGTH
        ? `${firstLine.slice(0, MAX_REASON_LENGTH)}...`
        : firstLine
}

/**
 * Extracts why a Hasura action failed, so a form can tell the admin what went
 * wrong instead of only that something did.
 *
 * Harvest answers a failed action with `{message, extensions: {code}}`. Hasura
 * forwards that message as the GraphQL error message, but when it cannot parse
 * the handler's answer it reports a generic message of its own and keeps the
 * original body in `extensions.internal.response.body`, so the body is read
 * first. Transport failures (a timeout, for instance) carry no body at all and
 * only describe themselves in `extensions.internal.error`.
 */
export const getGraphQLActionErrorReason = (error: unknown): string | undefined => {
    const actionError = error as IGraphQLActionError | undefined

    for (const graphQLError of actionError?.graphQLErrors ?? []) {
        const responseBody = parseActionResponseBody(
            graphQLError.extensions?.internal?.response?.body
        ) as {message?: unknown} | undefined

        const reason =
            readableReason(responseBody?.message) ??
            readableReason(graphQLError.extensions?.internal?.error?.message) ??
            readableReason(graphQLError.message)
        if (reason) {
            return reason
        }
    }

    return readableReason(actionError?.message)
}
