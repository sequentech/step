// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EVotingStatus, VotingStatusChannel} from "@sequentech/ui-core"

/** One voting channel of an election, as Stop Voting and the card see it. */
export interface ISealChannel {
    channel: VotingStatusChannel
    enabled: boolean
    status: EVotingStatus
    /** When the channel first opened, if it ever did. */
    firstStartedAt?: string | null
}

export interface ISealProgress {
    /** Every channel that counts is CLOSED, and one counts: the ballot boxes are due to be sealed. */
    finished: boolean
    /** Enabled channels that hold the seal: not CLOSED. */
    holding: VotingStatusChannel[]
    /**
     * Channels the election doesn't enable that hold the seal: open, paused or
     * ever started, and not CLOSED. They are stopped after enabling them again.
     */
    notEnabled: VotingStatusChannel[]
}

/**
 * Whether a channel counts for the seal (`deadline.rs`'s `counts`): enabled,
 * open or paused, or ever started.
 */
const counts = ({enabled, status, firstStartedAt}: ISealChannel) =>
    enabled || status === EVotingStatus.OPEN || status === EVotingStatus.PAUSED || !!firstStartedAt

/**
 * Where voting at an election stands for its seal (VOTE-FREEZE). Mirrors
 * windmill's `seal_deadline`: every channel that counts must be CLOSED, and
 * one must count. Nothing is guessed: an enabled channel that never started
 * (early voting included) holds the seal until it is closed.
 */
export const sealProgress = (channels: ISealChannel[]): ISealProgress => {
    const counted = channels.filter(counts)
    const held = counted.filter((channel) => channel.status !== EVotingStatus.CLOSED)
    return {
        finished: !held.length && counted.length > 0,
        holding: held.filter(({enabled}) => enabled).map(({channel}) => channel),
        notEnabled: held.filter(({enabled}) => !enabled).map(({channel}) => channel),
    }
}

export interface IStopSealOutcome {
    /** Whether this Stop finishes voting at the election, so its ballot boxes are sealed. */
    seals: boolean
    /** Enabled channels that still hold the seal after this Stop. */
    holding: VotingStatusChannel[]
    /** Channels the election doesn't enable that still hold the seal after this Stop. */
    notEnabled: VotingStatusChannel[]
}

/**
 * What stopping `stopping` does to the seal of an election. The server
 * closes the named channels whether or not the Post enables them (an
 * event-wide Stop closes them on every Post); without names, a Post's Stop
 * closes its enabled channels.
 */
export const stopSealOutcome = (
    channels: ISealChannel[],
    stopping?: VotingStatusChannel[]
): IStopSealOutcome => {
    const after = channels.map((channel) =>
        (stopping ? stopping.includes(channel.channel) : channel.enabled)
            ? {...channel, status: EVotingStatus.CLOSED}
            : channel
    )
    const {finished, holding, notEnabled} = sealProgress(after)
    return {seals: finished, holding, notEnabled}
}

/** The status and period-dates keys of each channel in an election's status. */
export const CHANNEL_STATUS_KEYS: Record<
    `${VotingStatusChannel}`,
    {status: string; dates: string; enabled: string}
> = {
    ONLINE: {
        status: "voting_status",
        dates: "voting_period_dates",
        enabled: "online",
    },
    KIOSK: {
        status: "kiosk_voting_status",
        dates: "kiosk_voting_period_dates",
        enabled: "kiosk",
    },
    EARLY_VOTING: {
        status: "early_voting_status",
        dates: "early_voting_period_dates",
        enabled: "early_voting",
    },
    TELEPHONE: {
        status: "telephone_voting_status",
        dates: "telephone_voting_period_dates",
        enabled: "telephone",
    },
}

const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : {}

/** The channels of an election, from its `status` and `voting_channels` columns. */
export const electionSealChannels = (election: {
    status?: unknown
    voting_channels?: unknown
}): ISealChannel[] => {
    const status = record(election.status)
    const enabled = record(election.voting_channels)
    return (Object.keys(CHANNEL_STATUS_KEYS) as Array<`${VotingStatusChannel}`>).map((name) => {
        const keys = CHANNEL_STATUS_KEYS[name]
        return {
            channel: name as VotingStatusChannel,
            enabled: enabled[keys.enabled] === true,
            status: (status[keys.status] as EVotingStatus | undefined) ?? EVotingStatus.NOT_STARTED,
            firstStartedAt: record(status[keys.dates]).first_started_at as string | undefined,
        }
    })
}

/**
 * Whether online voting ran at the election, which is when its grace period
 * applies (`deadline.rs`: ONLINE with a `first_started_at`, enabled or not,
 * since online ballots were cast under it).
 */
export const onlineRan = (channels: ISealChannel[]): boolean =>
    channels.some(
        (channel) =>
            channel.channel === ("ONLINE" as VotingStatusChannel) && !!channel.firstStartedAt
    )

/** Whether no enabled channel of the election has ever opened. */
export const neverOpened = (channels: ISealChannel[]): boolean =>
    // Opened means it has a start date (the server's `never_opened`): a
    // channel closed without ever starting has none.
    channels.filter((channel) => channel.enabled).every((channel) => !channel.firstStartedAt)

/**
 * The channels an event-wide Start applies, as `election_event_status.rs`
 * does: the ones named (else the event's enabled channels), except those the
 * event already has open.
 */
export const eventStartChannels = (
    eventChannels: ISealChannel[],
    starting?: VotingStatusChannel[]
): VotingStatusChannel[] =>
    eventChannels
        // Named channels; else the event's enabled ones, and every channel
        // when the event enables none (`voting_channels` unset).
        .filter((channel) =>
            starting
                ? starting.includes(channel.channel)
                : channel.enabled || !eventChannels.some(({enabled}) => enabled)
        )
        .filter((channel) => channel.status !== ("OPEN" as EVotingStatus))
        .map(({channel}) => channel)

/**
 * The channels of `applied` an event-wide Start leaves closed on this
 * election under Seal at close: the server skips a Post per channel where
 * that channel is CLOSED on it, enabled or not. (An election with seals
 * stays closed on every channel; the caller knows its seals.)
 */
export const keptClosedChannels = (
    channels: ISealChannel[],
    applied: VotingStatusChannel[]
): VotingStatusChannel[] =>
    applied.filter((name) =>
        channels.some(
            (channel) => channel.channel === name && channel.status === ("CLOSED" as EVotingStatus)
        )
    )
