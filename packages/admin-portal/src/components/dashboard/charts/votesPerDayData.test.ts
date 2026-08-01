// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {VotingStatusChannel} from "@sequentech/ui-core"
import {toVotesPerDayChartData} from "./votesPerDayData"

describe("toVotesPerDayChartData", () => {
    it("builds stacked series and omits channels without votes", () => {
        expect(
            toVotesPerDayChartData([
                {day: "2026-07-27", channel: VotingStatusChannel.Online, day_count: 0},
                {day: "2026-07-27", channel: VotingStatusChannel.Kiosk, day_count: 2},
                {day: "2026-07-28", channel: VotingStatusChannel.Online, day_count: 3},
                {day: "2026-07-28", channel: VotingStatusChannel.Kiosk, day_count: 1},
                {day: "2026-07-28", channel: "FUTURE_CHANNEL", day_count: 4},
            ])
        ).toEqual({
            buckets: ["2026-07-27", "2026-07-28"],
            series: [
                {channel: VotingStatusChannel.Online, data: [0, 3]},
                {channel: VotingStatusChannel.Kiosk, data: [2, 1]},
            ],
        })
    })
    it("keeps hour and minute buckets from the same day separate", () => {
        expect(
            toVotesPerDayChartData([
                {
                    day: "2026-07-31",
                    bucket: "2026-07-31T10:00:00",
                    channel: VotingStatusChannel.Online,
                    day_count: 2,
                },
                {
                    day: "2026-07-31",
                    bucket: "2026-07-31T10:01:00",
                    channel: VotingStatusChannel.Online,
                    day_count: 3,
                },
            ])
        ).toEqual({
            buckets: ["2026-07-31T10:00:00", "2026-07-31T10:01:00"],
            series: [{channel: VotingStatusChannel.Online, data: [2, 3]}],
        })
    })

    it("returns no series when there are no votes", () => {
        expect(
            toVotesPerDayChartData([
                {day: "2026-07-27", channel: VotingStatusChannel.Online, day_count: 0},
                {day: "2026-07-28", channel: VotingStatusChannel.Online, day_count: 0},
            ])
        ).toEqual({buckets: ["2026-07-27", "2026-07-28"], series: []})
    })
})
