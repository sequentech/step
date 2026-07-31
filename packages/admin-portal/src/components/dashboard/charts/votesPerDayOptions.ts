// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApexOptions} from "apexcharts"
import type {VotesTimeResolution} from "./votesTimeRange"

const MAX_AXIS_LABELS = 8
const MAX_TOTAL_LABELS = 24

export interface VotesPerDayChartOptionsInput {
    buckets: string[]
    resolution: VotesTimeResolution
    locale: string
}

function parseLocalBucket(value: string): Date | null {
    const match = /^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2})(?::(\d{2}))?)?$/.exec(value)
    if (!match) {
        return null
    }

    const [, year, month, day, hour = "0", minute = "0", second = "0"] = match
    const date = new Date(
        Number(year),
        Number(month) - 1,
        Number(day),
        Number(hour),
        Number(minute),
        Number(second)
    )

    if (
        date.getFullYear() !== Number(year) ||
        date.getMonth() !== Number(month) - 1 ||
        date.getDate() !== Number(day) ||
        date.getHours() !== Number(hour) ||
        date.getMinutes() !== Number(minute)
    ) {
        return null
    }

    return date
}

export function formatVotesBucket(
    bucket: string,
    resolution: VotesTimeResolution,
    locale: string,
    detailed = false
): string {
    const date = parseLocalBucket(bucket)
    if (!date) {
        return bucket
    }

    if (resolution === "day") {
        return new Intl.DateTimeFormat(locale, {
            weekday: detailed ? "short" : undefined,
            year: detailed ? "numeric" : undefined,
            month: "short",
            day: "numeric",
        }).format(date)
    }

    return new Intl.DateTimeFormat(locale, {
        weekday: detailed ? "short" : undefined,
        year: detailed ? "numeric" : undefined,
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
    }).format(date)
}

export const getVotesPerDayChartOptions = ({
    buckets,
    resolution,
    locale,
}: VotesPerDayChartOptionsInput): ApexOptions => {
    const labelInterval = Math.max(1, Math.ceil(buckets.length / MAX_AXIS_LABELS))

    return {
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
                        enabled: buckets.length <= MAX_TOTAL_LABELS,
                    },
                },
            },
        },
        tooltip: {
            enabled: true,
            shared: true,
            intersect: false,
            x: {
                formatter: (value, options) => {
                    const bucket = buckets[options?.dataPointIndex] ?? String(value)
                    return formatVotesBucket(bucket, resolution, locale, true)
                },
            },
        },
        xaxis: {
            categories: buckets,
            labels: {
                hideOverlappingLabels: true,
                rotate: 0,
                formatter: (value) => {
                    const index = buckets.indexOf(value)
                    const shouldShow =
                        index === buckets.length - 1 || (index >= 0 && index % labelInterval === 0)

                    return shouldShow ? formatVotesBucket(value, resolution, locale) : ""
                },
            },
        },
    }
}
