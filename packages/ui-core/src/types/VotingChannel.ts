// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum VotingStatusChannel {
    EarlyVoting = "EARLY_VOTING",
    Kiosk = "KIOSK",
    Online = "ONLINE",
    Telephone = "TELEPHONE",
}

export enum TallySheetVotingChannel {
    Paper = "PAPER",
    Postal = "POSTAL",
    InPerson = "IN_PERSON",
}

export type KnownParticipationChannel = VotingStatusChannel | TallySheetVotingChannel

declare const unknownParticipationChannelBrand: unique symbol

export type UnknownParticipationChannel = string & {
    readonly [unknownParticipationChannelBrand]: true
}

export type ParticipationChannel = KnownParticipationChannel | UnknownParticipationChannel

export const VOTING_STATUS_CHANNELS = [
    VotingStatusChannel.Online,
    VotingStatusChannel.Kiosk,
    VotingStatusChannel.EarlyVoting,
    VotingStatusChannel.Telephone,
] as const satisfies readonly VotingStatusChannel[]

export const TALLY_SHEET_VOTING_CHANNELS = [
    TallySheetVotingChannel.Paper,
    TallySheetVotingChannel.Postal,
    TallySheetVotingChannel.InPerson,
] as const satisfies readonly TallySheetVotingChannel[]

export const PARTICIPATION_CHANNEL_ORDER = [
    ...VOTING_STATUS_CHANNELS,
    ...TALLY_SHEET_VOTING_CHANNELS,
] as const satisfies readonly KnownParticipationChannel[]

const votingStatusChannels = new Set<string>(VOTING_STATUS_CHANNELS)
const tallySheetVotingChannels = new Set<string>(TALLY_SHEET_VOTING_CHANNELS)

export const isVotingStatusChannel = (channel: string): channel is VotingStatusChannel =>
    votingStatusChannels.has(channel)

export const isTallySheetVotingChannel = (channel: string): channel is TallySheetVotingChannel =>
    tallySheetVotingChannels.has(channel)

export const isKnownParticipationChannel = (
    channel: string
): channel is KnownParticipationChannel =>
    isVotingStatusChannel(channel) || isTallySheetVotingChannel(channel)

/**
 * Converts a serialized participation channel into its domain type.
 *
 * Unknown values stay string-compatible so newer backend channels can still be
 * displayed while every known value remains exhaustively typed.
 */
export const parseParticipationChannel = (channel: string): ParticipationChannel =>
    isKnownParticipationChannel(channel) ? channel : (channel as UnknownParticipationChannel)
