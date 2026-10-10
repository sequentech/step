// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ATTR_RESET_VALUE} from "@/types/keycloak"
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

const values = (attributes: Attributes, name: string): string[] => {
    const value = attributes?.[name]
    const list = Array.isArray(value) ? value : value === undefined || value === null ? [] : [value]
    return list
        .filter((item): item is string => typeof item === "string")
        .filter((item) => item !== "" && item !== ATTR_RESET_VALUE)
}

const first = (attributes: Attributes, name: string) => values(attributes, name)[0]

const asChannel = (value: string | undefined): EMessageChannel | undefined =>
    MESSAGE_CHANNELS.find((channel) => channel === value)

const messenger = (attributes: Attributes) => {
    const id = first(attributes, VOTER_ATTR_MESSENGER_ID)
    const page = first(attributes, VOTER_ATTR_MESSENGER_PAGE)
    return id && page ? {destination: id, messengerPage: page} : undefined
}

const recordedDestination = (
    channel: EMessageChannel,
    email: string | undefined,
    attributes: Attributes
) => {
    switch (channel) {
        case EMessageChannel.WHATSAPP:
        case EMessageChannel.VIBER:
        case EMessageChannel.SMS: {
            const number = first(attributes, VOTER_ATTR_MESSAGE_NUMBER)
            return number ? {destination: number} : undefined
        }
        case EMessageChannel.MESSENGER:
            return messenger(attributes)
        case EMessageChannel.EMAIL:
            return email ? {destination: email} : undefined
    }
}

const verifiedDestination = (
    channel: EMessageChannel,
    email: string | undefined,
    attributes: Attributes
) => {
    const number = (name: string) => {
        const value = first(attributes, name)
        return value ? {destination: value} : undefined
    }
    switch (channel) {
        case EMessageChannel.WHATSAPP:
            return number(VOTER_ATTR_WHATSAPP_NUMBER)
        case EMessageChannel.VIBER:
            return number(VOTER_ATTR_VIBER_NUMBER)
        case EMessageChannel.SMS:
            return number(VOTER_ATTR_MOBILE_NUMBER)
        case EMessageChannel.MESSENGER:
            return messenger(attributes)
        case EMessageChannel.EMAIL:
            return email ? {destination: email} : undefined
    }
}

/**
 * The voter's one channel, read like `voter_channel` in sequent-core and
 * `VoterChannel` in Keycloak:
 * 1. a first-release record (`verified-channels`): its `message-channel` if
 *    verified, else the first verified channel in channel order;
 * 2. `message-channel` with its recorded destination;
 * 3. legacy: the email, else the mobile number. That defines the voter's one
 *    channel; it is not a fallback.
 */
export const voterChannel = (
    email: string | null | undefined,
    attributes: Attributes
): IVoterChannel | undefined => {
    const address = email?.trim() || undefined
    const recorded = asChannel(first(attributes, VOTER_ATTR_MESSAGE_CHANNEL))
    const declared = attributes?.[VOTER_ATTR_VERIFIED_CHANNELS]
    if (declared !== undefined && declared !== null) {
        const verified = values(attributes, VOTER_ATTR_VERIFIED_CHANNELS)
            .map(asChannel)
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
    } else if (recorded) {
        const found = recordedDestination(recorded, address, attributes)
        if (found) {
            return {channel: recorded, ...found, source: EVoterChannelSource.RECORDED}
        }
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
