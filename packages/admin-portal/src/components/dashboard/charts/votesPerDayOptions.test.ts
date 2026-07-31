// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {getVotesPerDayChartOptions} from "./votesPerDayOptions"

describe("getVotesPerDayChartOptions", () => {
    it("shows one total per stacked day and keeps hover tooltips enabled", () => {
        const options = getVotesPerDayChartOptions(["M", "T"])

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
            tooltip: {enabled: true},
        })
        expect(options.xaxis?.categories).toEqual(["M", "T"])
    })
})
