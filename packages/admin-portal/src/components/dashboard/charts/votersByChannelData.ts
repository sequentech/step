// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {VotingChannel} from "@sequentech/ui-essentials"

export interface PersistedVotersByChannel {
    channel: string
    count: number
}

const castVoteChannels = [
    VotingChannel.ONLINE,
    VotingChannel.KIOSK,
    VotingChannel.EARLY_VOTING,
    VotingChannel.TELEPHONE,
] as const

export type CastVoteChannel = (typeof castVoteChannels)[number]

export interface TotalVotersRow {
    count: number
    channel: CastVoteChannel
}

export const toVotersByChannelRows = (
    data: ReadonlyArray<PersistedVotersByChannel> | null | undefined
): TotalVotersRow[] => {
    const counts = new Map(data?.map(({channel, count}) => [channel, count]) ?? [])
    return castVoteChannels.map((channel) => ({
        channel,
        count: counts.get(channel) ?? 0,
    }))
}
