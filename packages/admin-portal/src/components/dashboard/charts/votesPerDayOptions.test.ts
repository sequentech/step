// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {formatVotesBucket, getVotesPerDayChartOptions} from "./votesPerDayOptions"
import {VotesTimeResolution} from "./votesTimeRange"

describe("getVotesPerDayChartOptions", () => {
    it("shows one total per compact stack and channel counts on hover", () => {
        const buckets = ["2026-07-31T10:00:00", "2026-07-31T11:00:00"]
        const options = getVotesPerDayChartOptions({
            buckets,
            resolution: VotesTimeResolution.HOUR,
            locale: "en-US",
        })

        expect(options).toMatchObject({
            chart: {stacked: true},
            dataLabels: {enabled: false},
            plotOptions: {
                bar: {
                    dataLabels: {
                        total: {enabled: true},
                    },
                },
            },
            tooltip: {enabled: true, shared: true, intersect: false},
        })
        expect(options.xaxis?.categories).toEqual(buckets)
        expect(options.tooltip?.x?.formatter?.(0, {dataPointIndex: 0})).toContain("Jul 31")
    })

    it("hides aggregate labels when the selected range would make them unreadable", () => {
        const options = getVotesPerDayChartOptions({
            buckets: Array.from(
                {length: 60},
                (_, index) => `2026-07-31T10:${String(index).padStart(2, "0")}:00`
            ),
            resolution: VotesTimeResolution.MINUTE,
            locale: "en-US",
        })

        expect(options.plotOptions?.bar?.dataLabels?.total?.enabled).toBe(false)
    })

    it("formats local buckets and safely preserves unexpected values", () => {
        expect(
            formatVotesBucket("2026-07-31T10:01:00", VotesTimeResolution.MINUTE, "en-US")
        ).toContain("Jul 31")
        expect(formatVotesBucket("unexpected", VotesTimeResolution.DAY, "en-US")).toBe("unexpected")
    })
})
