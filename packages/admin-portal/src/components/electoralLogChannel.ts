// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageChannel, MESSAGE_CHANNELS} from "@/types/messaging"

const SEND_COMMUNICATIONS = "SendCommunications"

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const parse = (value: unknown): unknown => {
    if (typeof value !== "string") {
        return value
    }
    try {
        return JSON.parse(value)
    } catch {
        return undefined
    }
}

const asChannel = (value: unknown): EMessageChannel | undefined =>
    MESSAGE_CHANNELS.find((channel) => channel === value)

/** The messaging channel an electoral log entry records, if it records one. */
export const electoralLogChannel = (
    message: string | null | undefined
): EMessageChannel | undefined => {
    const entry = parse(message)
    if (!isRecord(entry) || !isRecord(entry.statement)) {
        return undefined
    }
    const {head, body} = entry.statement
    const fromHead = isRecord(head) ? asChannel(head.channel) : undefined
    if (fromHead) {
        return fromHead
    }
    const details = isRecord(body) ? parse(body[SEND_COMMUNICATIONS]) : undefined
    return isRecord(details) ? asChannel(details.channel) : undefined
}
