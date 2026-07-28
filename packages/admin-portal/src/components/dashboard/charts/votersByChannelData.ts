// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface PersistedVotersByChannel {
    channel: string
    count: number
}

const castVoteChannels = ["ONLINE", "KIOSK", "EARLY_VOTING", "TELEPHONE"] as const

export type CastVoteChannel = (typeof castVoteChannels)[number]
export const OTHER_VOTING_CHANNEL = "OTHER" as const
export type VotersByChannelRowChannel = CastVoteChannel | typeof OTHER_VOTING_CHANNEL

export interface TotalVotersRow {
    count: number
    channel: VotersByChannelRowChannel
}

export const toVotersByChannelRows = (
    data: ReadonlyArray<PersistedVotersByChannel> | null | undefined
): TotalVotersRow[] => {
    const counts = new Map(data?.map(({channel, count}) => [channel, count]) ?? [])
    const rows: TotalVotersRow[] = castVoteChannels.map((channel) => ({
        channel,
        count: counts.get(channel) ?? 0,
    }))
    const knownChannels = new Set<string>(castVoteChannels)
    const otherCount =
        data
            ?.filter(({channel}) => !knownChannels.has(channel))
            .reduce((total, {count}) => total + count, 0) ?? 0

    return otherCount > 0 ? [...rows, {channel: OTHER_VOTING_CHANNEL, count: otherCount}] : rows
}
