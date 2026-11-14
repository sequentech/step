// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export type LogMessage = Record<string, unknown>

const isLogMessage = (value: unknown): value is LogMessage =>
    typeof value === "object" && value !== null && !Array.isArray(value)

export const parseLogMessage = (message: string | null | undefined): LogMessage | null => {
    if (!message) {
        return null
    }
    try {
        const parsed: unknown = JSON.parse(message)
        return isLogMessage(parsed) ? parsed : null
    } catch {
        return null
    }
}

export const getLogMessageField = (
    message: LogMessage | null,
    field: string
): string | number | null => {
    const value = message?.[field]
    return typeof value === "string" || typeof value === "number" ? value : null
}

export const getLogMessageHeadField = (
    message: LogMessage | null,
    field: string
): string | number | null => {
    const statement = message?.statement
    if (!isLogMessage(statement) || !isLogMessage(statement.head)) {
        return null
    }
    return getLogMessageField(statement.head, field)
}
