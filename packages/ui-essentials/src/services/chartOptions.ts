// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApexOptions} from "apexcharts"
import {escapeHtml} from "@sequentech/ui-core"

type ChartTooltipY = Exclude<NonNullable<NonNullable<ApexOptions["tooltip"]>["y"]>, unknown[]>

const escapeChartText = (text: string): string => escapeHtml(String(text))

const withEscapedSeriesName = (tooltipY: ChartTooltipY = {}): ChartTooltipY => ({
    ...tooltipY,
    title: {...tooltipY.title, formatter: escapeChartText},
})

// apexcharts inserts legend entries and tooltip series names as HTML, so both
// are escaped here. The labels stay raw: apexcharts also draws them as SVG
// text, where escaped entities would show literally.
export const withEscapedChartText = (options: ApexOptions): ApexOptions => {
    const tooltipY = options.tooltip?.y

    return {
        ...options,
        legend: {...options.legend, formatter: escapeChartText},
        tooltip: {
            ...options.tooltip,
            y: Array.isArray(tooltipY)
                ? tooltipY.map((entry) => withEscapedSeriesName(entry))
                : withEscapedSeriesName(tooltipY),
        },
    }
}
