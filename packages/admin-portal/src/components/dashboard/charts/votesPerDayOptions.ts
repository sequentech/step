// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApexOptions} from "apexcharts"

export const getVotesPerDayChartOptions = (categories: string[]): ApexOptions => ({
    chart: {
        id: "barchart-votes",
        stacked: true,
    },
    dataLabels: {
        enabled: false,
    },
    legend: {
        showForZeroSeries: false,
    },
    plotOptions: {
        bar: {
            dataLabels: {
                total: {
                    enabled: true,
                },
            },
        },
    },
    tooltip: {
        enabled: true,
    },
    xaxis: {
        categories,
    },
})
