// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum VotesTimeResolution {
    MINUTE = "minute",
    HOUR = "hour",
    DAY = "day",
}

export enum VotesTimeRange {
    FIFTEEN_MINUTES = "15m",
    ONE_HOUR = "1h",
    SIX_HOURS = "6h",
    TWENTY_FOUR_HOURS = "24h",
    SEVEN_DAYS = "7d",
    THIRTY_DAYS = "30d",
    NINETY_DAYS = "90d",
}

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
    [VotesTimeResolution.MINUTE]: [
        {
            value: VotesTimeRange.FIFTEEN_MINUTES,
            label: VotesTimeRange.FIFTEEN_MINUTES,
            bucketCount: 15,
        },
        {value: VotesTimeRange.ONE_HOUR, label: VotesTimeRange.ONE_HOUR, bucketCount: 60},
        {value: VotesTimeRange.SIX_HOURS, label: VotesTimeRange.SIX_HOURS, bucketCount: 360},
    ],
    [VotesTimeResolution.HOUR]: [
        {
            value: VotesTimeRange.TWENTY_FOUR_HOURS,
            label: VotesTimeRange.TWENTY_FOUR_HOURS,
            bucketCount: 24,
        },
        {value: VotesTimeRange.SEVEN_DAYS, label: VotesTimeRange.SEVEN_DAYS, bucketCount: 168},
        {value: VotesTimeRange.THIRTY_DAYS, label: VotesTimeRange.THIRTY_DAYS, bucketCount: 720},
    ],
    [VotesTimeResolution.DAY]: [
        {value: VotesTimeRange.SEVEN_DAYS, label: VotesTimeRange.SEVEN_DAYS, bucketCount: 7},
        {value: VotesTimeRange.THIRTY_DAYS, label: VotesTimeRange.THIRTY_DAYS, bucketCount: 30},
        {value: VotesTimeRange.NINETY_DAYS, label: VotesTimeRange.NINETY_DAYS, bucketCount: 90},
    ],
}

const DEFAULT_RANGE: Record<VotesTimeResolution, VotesTimeRange> = {
    [VotesTimeResolution.MINUTE]: VotesTimeRange.ONE_HOUR,
    [VotesTimeResolution.HOUR]: VotesTimeRange.TWENTY_FOUR_HOURS,
    [VotesTimeResolution.DAY]: VotesTimeRange.SEVEN_DAYS,
}

export const DEFAULT_VOTES_TIME_SELECTION: VotesTimeSelection = {
    resolution: VotesTimeResolution.DAY,
    range: DEFAULT_RANGE[VotesTimeResolution.DAY],
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
