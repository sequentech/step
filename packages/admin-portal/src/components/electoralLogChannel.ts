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

/** Windmill writes the channel in lowercase ("whatsapp"); older entries used the enum value. */
const asChannel = (value: unknown): EMessageChannel | undefined =>
    typeof value === "string"
        ? MESSAGE_CHANNELS.find((channel) => channel === value.toUpperCase())
        : undefined

const asText = (value: unknown): string | undefined =>
    typeof value === "string" && value ? value : undefined

/** What a SendCommunications entry records about one message. */
export interface IElectoralLogDelivery {
    channel?: EMessageChannel
    /** The ledger row (`sequent_backend.message`) of the attempt. */
    messageId?: string
    /** The state when the entry was written: `outcome`, or `state` in older entries. */
    outcome?: string
    attempt?: number
    reason?: string
}

/** The message an electoral log entry records, if it records one. */
export const electoralLogDelivery = (
    message: string | null | undefined
): IElectoralLogDelivery | undefined => {
    const entry = parse(message)
    if (!isRecord(entry) || !isRecord(entry.statement)) {
        return undefined
    }
    const {head, body} = entry.statement
    const fromHead = isRecord(head) ? asChannel(head.channel) : undefined
    const details = isRecord(body) ? parse(body[SEND_COMMUNICATIONS]) : undefined
    if (!isRecord(details)) {
        return fromHead ? {channel: fromHead} : undefined
    }
    return {
        channel: fromHead ?? asChannel(details.channel),
        messageId: asText(details.message_id),
        outcome: asText(details.outcome) ?? asText(details.state),
        attempt: typeof details.attempt === "number" ? details.attempt : undefined,
        reason: asText(details.reason),
    }
}

/** The messaging channel an electoral log entry records, if it records one. */
export const electoralLogChannel = (
    message: string | null | undefined
): EMessageChannel | undefined => electoralLogDelivery(message)?.channel
