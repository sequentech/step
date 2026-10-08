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
    /** Every enabled channel is finished and one is CLOSED: the ballot boxes are due to be sealed. */
    finished: boolean
    /** Enabled channels that hold the seal: not closed, and able to open. */
    holding: VotingStatusChannel[]
    /** Enabled channels that never opened and no longer can (they don't hold the seal). */
    neverOpened: VotingStatusChannel[]
}

const started = ({status, firstStartedAt}: ISealChannel) =>
    status !== EVotingStatus.NOT_STARTED || !!firstStartedAt

/**
 * Whether a channel that never started can no longer open under the
 * platform's rules. Today only early voting, once online voting has started
 * (`EarlyVotingAfterOnline`). As in `deadline.rs`'s `online_started`, only
 * the online status counts, enabled or not: an event-wide Start sets it on
 * every election.
 */
const cannotOpen = (channel: ISealChannel, channels: ISealChannel[]) =>
    channel.channel === ("EARLY_VOTING" as VotingStatusChannel) &&
    channels.some((other) => other.channel === ("ONLINE" as VotingStatusChannel) && started(other))

/**
 * Where voting at an election stands for its seal (VOTE-FREEZE, D1).
 * Mirrors windmill's `seal_deadline`: an enabled channel is finished when it
 * is CLOSED, or when it never started and can no longer open. Sealing is due
 * when every enabled channel is finished and at least one is CLOSED.
 */
export const sealProgress = (channels: ISealChannel[]): ISealProgress => {
    const enabled = channels.filter((channel) => channel.enabled)
    const neverOpened = enabled.filter(
        (channel) => !started(channel) && cannotOpen(channel, channels)
    )
    const holding = enabled.filter(
        (channel) => channel.status !== EVotingStatus.CLOSED && !neverOpened.includes(channel)
    )
    return {
        finished:
            !holding.length && enabled.some((channel) => channel.status === EVotingStatus.CLOSED),
        holding: holding.map(({channel}) => channel),
        neverOpened: neverOpened.map(({channel}) => channel),
    }
}

export interface IStopSealOutcome {
    /** Whether this Stop finishes voting at the election, so its ballot boxes are sealed. */
    seals: boolean
    /** Enabled channels that still hold the seal after this Stop. */
    holding: VotingStatusChannel[]
    /** Enabled channels that never opened and can't any more: they don't hold the seal. */
    neverOpened: VotingStatusChannel[]
}

/** What stopping `stopping` (every channel when absent) does to the seal of an election. */
export const stopSealOutcome = (
    channels: ISealChannel[],
    stopping?: VotingStatusChannel[]
): IStopSealOutcome => {
    const after = channels.map((channel) =>
        channel.enabled && (!stopping || stopping.includes(channel.channel))
            ? {...channel, status: EVotingStatus.CLOSED}
            : channel
    )
    const progress = sealProgress(after)
    return {
        seals: progress.finished,
        holding: progress.holding,
        neverOpened: progress.finished ? progress.neverOpened : [],
    }
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
 * applies (`deadline.rs`: ONLINE enabled, with a `first_started_at`).
 */
export const onlineRan = (channels: ISealChannel[]): boolean =>
    channels.some(
        (channel) =>
            channel.channel === ("ONLINE" as VotingStatusChannel) &&
            channel.enabled &&
            !!channel.firstStartedAt
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
