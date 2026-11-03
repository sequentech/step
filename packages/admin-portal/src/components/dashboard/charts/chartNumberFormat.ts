// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApexOptions} from "apexcharts"
import {ENumberFormatPolicy, formatNumber, formatPercentage} from "@sequentech/ui-core"

/** Writes a pie or donut chart's slice percentages and hovered counts in `numberFormatPolicy`. */
export const getPieChartNumberFormatOptions = (
    numberFormatPolicy: ENumberFormatPolicy
): Pick<ApexOptions, "dataLabels" | "tooltip"> => ({
    dataLabels: {
        formatter: (percentage) => formatPercentage(Number(percentage), numberFormatPolicy, 1),
    },
    tooltip: {
        y: {
            formatter: (count) => formatNumber(count, numberFormatPolicy),
        },
    },
})
