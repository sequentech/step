// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export type VotesTimeResolution = "minute" | "hour" | "day"

export type VotesTimeRange = "15m" | "1h" | "6h" | "24h" | "7d" | "30d" | "90d"

export interface VotesTimeSelection {
    resolution: VotesTimeResolution
    range: VotesTimeRange
}

export interface VotesTimeRangeOption {
    value: VotesTimeRange
    label: string
    bucketCount: number
}

export const VOTES_TIME_RANGE_OPTIONS: Record<
    VotesTimeResolution,
    readonly VotesTimeRangeOption[]
> = {
    minute: [
        {value: "15m", label: "15m", bucketCount: 15},
        {value: "1h", label: "1h", bucketCount: 60},
        {value: "6h", label: "6h", bucketCount: 360},
    ],
    hour: [
        {value: "24h", label: "24h", bucketCount: 24},
        {value: "7d", label: "7d", bucketCount: 168},
        {value: "30d", label: "30d", bucketCount: 720},
    ],
    day: [
        {value: "7d", label: "7d", bucketCount: 7},
        {value: "30d", label: "30d", bucketCount: 30},
        {value: "90d", label: "90d", bucketCount: 90},
    ],
}

const DEFAULT_RANGE: Record<VotesTimeResolution, VotesTimeRange> = {
    minute: "1h",
    hour: "24h",
    day: "7d",
}

export const DEFAULT_VOTES_TIME_SELECTION: VotesTimeSelection = {
    resolution: "day",
    range: DEFAULT_RANGE.day,
}

export function getVotesTimeRangeOptions(
    resolution: VotesTimeResolution
): readonly VotesTimeRangeOption[] {
    return VOTES_TIME_RANGE_OPTIONS[resolution]
}

export function withVotesTimeResolution(resolution: VotesTimeResolution): VotesTimeSelection {
    return {
        resolution,
        range: DEFAULT_RANGE[resolution],
    }
}

export function getVotesBucketCount(selection: VotesTimeSelection): number {
    const options = getVotesTimeRangeOptions(selection.resolution)
    return (
        options.find(({value}) => value === selection.range)?.bucketCount ??
        options.find(({value}) => value === DEFAULT_RANGE[selection.resolution])!.bucketCount
    )
}
