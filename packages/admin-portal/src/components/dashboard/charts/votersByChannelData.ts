// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {VOTING_STATUS_CHANNELS, VotingStatusChannel} from "@sequentech/ui-core"

export interface PersistedVotersByChannel {
    channel: string
    count: number
}

export interface TotalVotersRow {
    count: number
    channel: VotingStatusChannel
}

export const toVotersByChannelRows = (
    data: ReadonlyArray<PersistedVotersByChannel> | null | undefined
): TotalVotersRow[] => {
    const counts = new Map(data?.map(({channel, count}) => [channel, count]) ?? [])

    return VOTING_STATUS_CHANNELS.map((channel) => ({
        channel,
        count: counts.get(channel) ?? 0,
    })).filter(({count}) => count > 0)
}
