// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface PersistedVotersByChannel {
    channel: string
    count: number
}

export enum CastVoteChannel {
    ONLINE = "ONLINE",
    KIOSK = "KIOSK",
    EARLY_VOTING = "EARLY_VOTING",
    TELEPHONE = "TELEPHONE",
}

export interface TotalVotersRow {
    count: number
    channel: CastVoteChannel
}

export const toVotersByChannelRows = (
    data: ReadonlyArray<PersistedVotersByChannel> | null | undefined
): TotalVotersRow[] => {
    const counts = new Map(data?.map(({channel, count}) => [channel, count]) ?? [])

    return Object.values(CastVoteChannel)
        .map((channel) => ({channel, count: counts.get(channel) ?? 0}))
        .filter(({count}) => count > 0)
}
