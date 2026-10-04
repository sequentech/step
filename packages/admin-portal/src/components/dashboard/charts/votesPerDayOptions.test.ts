// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {formatVotesBucket, getVotesPerDayChartOptions} from "./votesPerDayOptions"
import {VotesTimeResolution} from "./votesTimeRange"

const single = <T>(value: T | T[] | undefined): T | undefined =>
    Array.isArray(value) ? value[0] : value

describe("getVotesPerDayChartOptions", () => {
    it("shows one total per compact stack and channel counts on hover", () => {
        const buckets = ["2026-07-31T10:00:00", "2026-07-31T11:00:00"]
        const options = getVotesPerDayChartOptions({
            buckets,
            resolution: VotesTimeResolution.HOUR,
            locale: "en-US",
            numberFormatPolicy: ENumberFormatPolicy.COMMA_PERIOD,
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
            numberFormatPolicy: ENumberFormatPolicy.COMMA_PERIOD,
        })

        expect(options.plotOptions?.bar?.dataLabels?.total?.enabled).toBe(false)
    })

    it("writes the axis, hovered counts and stack totals in the event's number format", () => {
        const options = getVotesPerDayChartOptions({
            buckets: ["2026-07-31"],
            resolution: VotesTimeResolution.DAY,
            locale: "en-US",
            numberFormatPolicy: ENumberFormatPolicy.PERIOD_COMMA,
        })

        expect(single(options.yaxis)?.labels?.formatter?.(1234567)).toBe("1.234.567")
        expect(single(options.tooltip?.y)?.formatter?.(1234)).toBe("1.234")
        expect(options.plotOptions?.bar?.dataLabels?.total?.formatter?.("12345")).toBe("12.345")
    })

    it("groups counts with commas for events without a number format policy", () => {
        const options = getVotesPerDayChartOptions({
            buckets: ["2026-07-31"],
            resolution: VotesTimeResolution.DAY,
            locale: "en-US",
            numberFormatPolicy: ENumberFormatPolicy.COMMA_PERIOD,
        })

        expect(single(options.yaxis)?.labels?.formatter?.(12000)).toBe("12,000")
        expect(single(options.tooltip?.y)?.formatter?.(12000)).toBe("12,000")
    })

    it("keeps a decimal on the axis steps of a small range", () => {
        const options = getVotesPerDayChartOptions({
            buckets: ["2026-07-31"],
            resolution: VotesTimeResolution.DAY,
            locale: "en-US",
            numberFormatPolicy: ENumberFormatPolicy.SPACE_COMMA,
        })
        const formatAxis = single(options.yaxis)?.labels?.formatter

        expect([0, 0.5, 1, 1.5, 2].map((value) => formatAxis?.(value))).toEqual([
            "0",
            "0,5",
            "1",
            "1,5",
            "2",
        ])
    })

    it("formats local buckets and safely preserves unexpected values", () => {
        expect(
            formatVotesBucket("2026-07-31T10:01:00", VotesTimeResolution.MINUTE, "en-US")
        ).toContain("Jul 31")
        expect(formatVotesBucket("unexpected", VotesTimeResolution.DAY, "en-US")).toBe("unexpected")
    })
})
