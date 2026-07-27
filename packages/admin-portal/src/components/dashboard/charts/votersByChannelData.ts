// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum VotingChanel {
    Online = "online",
    Kiosk = "kiosk",
    EarlyVoting = "early_voting",
    Telephone = "telephone",
}

export interface PersistedVotersByChannel {
    channel: string
    count: number
}

export interface TotalVotersRow {
    count: number
    channel: VotingChanel
}

const persistedChannels: Array<[string, VotingChanel]> = [
    ["ONLINE", VotingChanel.Online],
    ["KIOSK", VotingChanel.Kiosk],
    ["EARLY_VOTING", VotingChanel.EarlyVoting],
    ["TELEPHONE", VotingChanel.Telephone],
]

export const toVotersByChannelRows = (
    data: ReadonlyArray<PersistedVotersByChannel> | null | undefined
): TotalVotersRow[] => {
    const counts = new Map(data?.map(({channel, count}) => [channel, count]) ?? [])
    return persistedChannels.map(([persistedChannel, channel]) => ({
        channel,
        count: counts.get(persistedChannel) ?? 0,
    }))
}
