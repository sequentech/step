// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {rustTrim, toAsciiUppercase} from "@/services/rustText"
import {
    EMessageAttemptState,
    EMessageChannel,
    ILedgerMessage,
    MESSAGE_CHANNELS,
    VOTER_ATTR_MESSAGE_CHANNEL,
    VOTER_ATTR_MESSAGE_CONSENT,
    VOTER_ATTR_MESSAGE_NUMBER,
    VOTER_ATTR_MESSENGER_ID,
    VOTER_ATTR_MESSENGER_PAGE,
    VOTER_ATTR_MOBILE_NUMBER,
    VOTER_ATTR_VERIFIED_CHANNELS,
    VOTER_ATTR_VIBER_NUMBER,
    VOTER_ATTR_WHATSAPP_NUMBER,
} from "@/types/messaging"

/** Which shape of record the channel was read from, as `VoterChannelSource` in sequent-core. */
export enum EVoterChannelSource {
    /** `message-channel` with its destination. */
    RECORDED = "RECORDED",
    /** A record of the first messaging release (`verified-channels`). */
    VERIFIED_CHANNELS = "VERIFIED_CHANNELS",
    /** A voter enrolled before messaging: the email, else the mobile number. */
    LEGACY = "LEGACY",
}

/** The voter's one channel and where it reaches them. */
export interface IVoterChannel {
    channel: EMessageChannel
    /** E.164 number, email address or Page-scoped ID. Never shown unmasked. */
    destination: string
    /** The Page the Page-scoped ID belongs to; Messenger only. */
    messengerPage?: string
    source: EVoterChannelSource
}

export interface IVoterMessaging {
    /** Undefined when the record reaches the voter on no channel. */
    channel?: IVoterChannel
    /** When the voter consented to messages on their channel. */
    consentAt?: string
}

type Attributes = Record<string, unknown> | null | undefined

// Attributes the drawer shows in its own section, not as editable fields.
const MESSAGING_ATTRIBUTES = [
    VOTER_ATTR_MESSAGE_CHANNEL,
    VOTER_ATTR_MESSAGE_NUMBER,
    VOTER_ATTR_MESSENGER_ID,
    VOTER_ATTR_MESSENGER_PAGE,
    VOTER_ATTR_MESSAGE_CONSENT,
    VOTER_ATTR_WHATSAPP_NUMBER,
    VOTER_ATTR_VIBER_NUMBER,
    VOTER_ATTR_VERIFIED_CHANNELS,
]

export const isVoterMessagingAttribute = (name: string): boolean =>
    MESSAGING_ATTRIBUTES.includes(name)

/** An attribute's values, as Keycloak holds them: a list. */
const values = (attributes: Attributes, name: string): unknown[] | undefined => {
    const value = attributes?.[name]
    if (value === undefined || value === null) return undefined
    return Array.isArray(value) ? value : [value]
}

/** The attribute's first value, unless it is empty: an empty first value is none. */
const first = (attributes: Attributes, name: string): string | undefined => {
    const value = values(attributes, name)?.[0]
    return typeof value === "string" && value !== "" ? value : undefined
}

/**
 * A channel as a record holds it: trimmed and in any (ASCII) case, as
 * `parse_channel_value` reads it. Anything else is no channel.
 */
export const parseChannelValue = (value: unknown): EMessageChannel | undefined => {
    if (typeof value !== "string") return undefined
    const text = toAsciiUppercase(rustTrim(value))
    return MESSAGE_CHANNELS.find((channel) => channel === text)
}

type Found = {destination: string; messengerPage?: string}

const messenger = (attributes: Attributes): Found | undefined => {
    const id = first(attributes, VOTER_ATTR_MESSENGER_ID)
    const page = first(attributes, VOTER_ATTR_MESSENGER_PAGE)
    return id && page ? {destination: id, messengerPage: page} : undefined
}

const number = (value: string | undefined): Found | undefined =>
    value ? {destination: value} : undefined

/** The destination of `channel` in a recorded (ticket-shaped) record. */
const recordedDestination = (
    channel: EMessageChannel,
    email: string | undefined,
    attributes: Attributes
): Found | undefined => {
    switch (channel) {
        case EMessageChannel.WHATSAPP:
        case EMessageChannel.VIBER:
            return number(first(attributes, VOTER_ATTR_MESSAGE_NUMBER))
        case EMessageChannel.SMS:
            // The same channel: the mobile number when there's no message number.
            return number(
                first(attributes, VOTER_ATTR_MESSAGE_NUMBER) ??
                    first(attributes, VOTER_ATTR_MOBILE_NUMBER)
            )
        case EMessageChannel.MESSENGER:
            return messenger(attributes)
        case EMessageChannel.EMAIL:
            return number(email)
    }
}

/** The destination of `channel` in a first-release record. */
const verifiedDestination = (
    channel: EMessageChannel,
    email: string | undefined,
    attributes: Attributes
): Found | undefined => {
    switch (channel) {
        case EMessageChannel.WHATSAPP:
            return number(first(attributes, VOTER_ATTR_WHATSAPP_NUMBER))
        case EMessageChannel.VIBER:
            return number(first(attributes, VOTER_ATTR_VIBER_NUMBER))
        case EMessageChannel.SMS:
            return number(first(attributes, VOTER_ATTR_MOBILE_NUMBER))
        case EMessageChannel.MESSENGER:
            return messenger(attributes)
        case EMessageChannel.EMAIL:
            return number(email)
    }
}

/**
 * The voter's one channel, or undefined when the record has none that can be
 * used. The same rule as `voter_channel` in sequent-core
 * (`src/types/voter_channel.rs`), case for case: its tests are this one's
 * table (R7-05). sequent-core's WASM package doesn't export it, so it is
 * mirrored here. Never another channel than the record's:
 * 1. a first-release record (`verified-channels` with at least one value):
 *    its `message-channel` if verified, else the first verified channel in
 *    channel order that has that release's destination; none, otherwise;
 * 2. `message-channel` with its recorded destination (SMS also reads the
 *    mobile number); none when the destination is missing, never the email;
 * 3. legacy, with neither: the email, else the mobile number. That defines
 *    the voter's one channel; it is not a fallback.
 */
export const voterChannel = (
    email: string | null | undefined,
    attributes: Attributes
): IVoterChannel | undefined => {
    const address = email ? email : undefined
    const recorded = parseChannelValue(first(attributes, VOTER_ATTR_MESSAGE_CHANNEL))

    const declared = values(attributes, VOTER_ATTR_VERIFIED_CHANNELS)
    if (declared && declared.length > 0) {
        const verified = declared
            .map(parseChannelValue)
            .filter((channel): channel is EMessageChannel => !!channel)
        const candidates = [
            ...(recorded && verified.includes(recorded) ? [recorded] : []),
            ...MESSAGE_CHANNELS.filter((channel) => verified.includes(channel)),
        ]
        for (const channel of candidates) {
            const found = verifiedDestination(channel, address, attributes)
            if (found) {
                return {channel, ...found, source: EVoterChannelSource.VERIFIED_CHANNELS}
            }
        }
        return undefined
    }
    if (recorded) {
        const found = recordedDestination(recorded, address, attributes)
        return found
            ? {channel: recorded, ...found, source: EVoterChannelSource.RECORDED}
            : undefined
    }
    if (address) {
        return {
            channel: EMessageChannel.EMAIL,
            destination: address,
            source: EVoterChannelSource.LEGACY,
        }
    }
    const mobile = first(attributes, VOTER_ATTR_MOBILE_NUMBER)
    return mobile
        ? {channel: EMessageChannel.SMS, destination: mobile, source: EVoterChannelSource.LEGACY}
        : undefined
}

/** The consent time recorded with the channel (`{at, version, organization, channel}`). */
const consentAt = (attributes: Attributes): string | undefined => {
    const raw = first(attributes, VOTER_ATTR_MESSAGE_CONSENT)
    if (!raw) return undefined
    try {
        const consent = JSON.parse(raw) as {at?: unknown}
        return typeof consent?.at === "string" && consent.at ? consent.at : undefined
    } catch {
        return undefined
    }
}

/** The messaging the voter's Keycloak record holds: one channel and its consent. */
export const voterMessaging = (
    email: string | null | undefined,
    attributes: Attributes
): IVoterMessaging => ({
    channel: voterChannel(email, attributes),
    consentAt: consentAt(attributes),
})

/** The voter's messages staff should see in the drawer. */
export interface IVoterDeliveryProblems {
    /** The latest message whose latest attempt failed. */
    failure?: ILedgerMessage
    /** The latest message whose latest attempt's delivery isn't confirmed. */
    unconfirmed?: ILedgerMessage
}

const attemptOrder = (a: ILedgerMessage, b: ILedgerMessage): number =>
    (a.attempt ?? 0) - (b.attempt ?? 0) ||
    String(a.created_at ?? "").localeCompare(String(b.created_at ?? ""))

/**
 * A message is in the state of its latest attempt, as the event tab counts it:
 * a failed attempt that a later attempt of the same message settled (retried,
 * accepted or delivered) is no longer a failure. Unknown is never a failure:
 * it is "delivery not confirmed", and Unresolved after 24 hours.
 *
 * `candidates` are the voter's FAILED and UNKNOWN attempts, latest first;
 * `attempts` every attempt of their messages.
 */
export const voterDeliveryProblems = (
    candidates: ILedgerMessage[],
    attempts: ILedgerMessage[]
): IVoterDeliveryProblems => {
    const latest = new Map<string, ILedgerMessage>()
    for (const attempt of [...candidates, ...attempts]) {
        const key = attempt.logical_key ?? attempt.id
        const current = latest.get(key)
        if (!current || attemptOrder(current, attempt) < 0) {
            latest.set(key, attempt)
        }
    }
    const isLatest = (message: ILedgerMessage) =>
        latest.get(message.logical_key ?? message.id)?.id === message.id
    return {
        failure: candidates.find(
            (message) => message.state === EMessageAttemptState.FAILED && isLatest(message)
        ),
        unconfirmed: candidates.find(
            (message) => message.state === EMessageAttemptState.UNKNOWN && isLatest(message)
        ),
    }
}
