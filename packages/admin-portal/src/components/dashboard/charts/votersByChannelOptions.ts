// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApexOptions} from "apexcharts"
import {ENumberFormatPolicy, formatNumber} from "@sequentech/ui-core"
import {getPieChartNumberFormatOptions} from "./chartNumberFormat"

export interface VotersByChannelChartOptionsInput {
    labels: string[]
    numberFormatPolicy: ENumberFormatPolicy
}

interface DonutTotalContext {
    globals: {seriesTotals: number[]}
}

export const getVotersByChannelChartOptions = ({
    labels,
    numberFormatPolicy,
}: VotersByChannelChartOptionsInput): ApexOptions => ({
    ...getPieChartNumberFormatOptions(numberFormatPolicy),
    labels,
    plotOptions: {
        pie: {
            donut: {
                labels: {
                    show: true,
                    value: {
                        formatter: (count) => formatNumber(count, numberFormatPolicy),
                    },
                    total: {
                        showAlways: true,
                        show: true,
                        formatter: ({globals}: DonutTotalContext) =>
                            formatNumber(
                                globals.seriesTotals.reduce((sum, count) => sum + count, 0),
                                numberFormatPolicy
                            ),
                    },
                },
            },
        },
    },
})
