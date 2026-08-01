// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {VOTING_STATUS_CHANNELS, VotingStatusChannel} from "@sequentech/ui-core"

export interface PersistedVotesPerDay {
    day: string
    bucket?: string | null
    day_count: number
    channel: string
}

export interface VotesPerDaySeries {
    channel: VotingStatusChannel
    data: number[]
}

export interface VotesPerDayChartData {
    buckets: string[]
    series: VotesPerDaySeries[]
}

export const toVotesPerDayChartData = (
    data: ReadonlyArray<PersistedVotesPerDay>
): VotesPerDayChartData => {
    const buckets = Array.from(new Set(data.map(({bucket, day}) => String(bucket ?? day)))).sort()
    const counts = new Map<string, number>()

    for (const {bucket, day, channel, day_count} of data) {
        const key = `${String(bucket ?? day)}:${channel}`
        counts.set(key, (counts.get(key) ?? 0) + day_count)
    }

    const series = VOTING_STATUS_CHANNELS.map((channel) => ({
        channel,
        data: buckets.map((bucket) => counts.get(`${bucket}:${channel}`) ?? 0),
    })).filter(({data: channelData}) => channelData.some((count) => count > 0))

    return {buckets, series}
}
