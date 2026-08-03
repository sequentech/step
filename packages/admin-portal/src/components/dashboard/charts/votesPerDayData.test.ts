// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {CastVoteChannel} from "./votersByChannelData"
import {toVotesPerDayChartData} from "./votesPerDayData"

describe("toVotesPerDayChartData", () => {
    it("builds stacked series and omits channels without votes", () => {
        expect(
            toVotesPerDayChartData([
                {day: "2026-07-27", channel: "ONLINE", day_count: 0},
                {day: "2026-07-27", channel: "KIOSK", day_count: 2},
                {day: "2026-07-28", channel: "ONLINE", day_count: 3},
                {day: "2026-07-28", channel: "KIOSK", day_count: 1},
                {day: "2026-07-28", channel: "FUTURE_CHANNEL", day_count: 4},
            ])
        ).toEqual({
            days: ["2026-07-27", "2026-07-28"],
            series: [
                {channel: CastVoteChannel.ONLINE, data: [0, 3]},
                {channel: CastVoteChannel.KIOSK, data: [2, 1]},
            ],
        })
    })

    it("returns no series when there are no votes", () => {
        expect(
            toVotesPerDayChartData([
                {day: "2026-07-27", channel: "ONLINE", day_count: 0},
                {day: "2026-07-28", channel: "ONLINE", day_count: 0},
            ])
        ).toEqual({days: ["2026-07-27", "2026-07-28"], series: []})
    })
})
