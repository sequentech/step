// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {CastVoteChannel} from "./votersByChannelData"

export interface PersistedVotesPerDay {
    day: string
    day_count: number
    channel: string
}

export interface VotesPerDaySeries {
    channel: CastVoteChannel
    data: number[]
}

export interface VotesPerDayChartData {
    days: string[]
    series: VotesPerDaySeries[]
}

export const toVotesPerDayChartData = (
    data: ReadonlyArray<PersistedVotesPerDay>
): VotesPerDayChartData => {
    const days = Array.from(new Set(data.map(({day}) => String(day)))).sort()
    const counts = new Map<string, number>()

    for (const {day, channel, day_count} of data) {
        const key = `${String(day)}:${channel}`
        counts.set(key, (counts.get(key) ?? 0) + day_count)
    }

    const series = Object.values(CastVoteChannel)
        .map((channel) => ({
            channel,
            data: days.map((day) => counts.get(`${day}:${channel}`) ?? 0),
        }))
        .filter(({data: channelData}) => channelData.some((count) => count > 0))

    return {days, series}
}
