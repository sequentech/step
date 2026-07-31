// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    DEFAULT_VOTES_TIME_SELECTION,
    getVotesBucketCount,
    getVotesTimeRangeOptions,
    withVotesTimeResolution,
    VotesTimeRange,
    VotesTimeResolution,
} from "./votesTimeRange"

describe("votesTimeRange", () => {
    it("defaults to the seven-day daily view", () => {
        expect(DEFAULT_VOTES_TIME_SELECTION).toEqual({
            resolution: VotesTimeResolution.DAY,
            range: VotesTimeRange.SEVEN_DAYS,
        })
        expect(getVotesBucketCount(DEFAULT_VOTES_TIME_SELECTION)).toBe(7)
    })

    it("uses a compact default window when resolution changes", () => {
        expect(withVotesTimeResolution(VotesTimeResolution.MINUTE)).toEqual({
            resolution: VotesTimeResolution.MINUTE,
            range: VotesTimeRange.ONE_HOUR,
        })
        expect(withVotesTimeResolution(VotesTimeResolution.HOUR)).toEqual({
            resolution: VotesTimeResolution.HOUR,
            range: VotesTimeRange.TWENTY_FOUR_HOURS,
        })
    })

    it("keeps every preset below the server bucket limit", () => {
        for (const resolution of Object.values(VotesTimeResolution)) {
            for (const option of getVotesTimeRangeOptions(resolution)) {
                expect(option.bucketCount).toBeGreaterThan(0)
                expect(option.bucketCount).toBeLessThanOrEqual(1000)
            }
        }
    })

    it("falls back safely if a stale range is paired with a resolution", () => {
        expect(
            getVotesBucketCount({
                resolution: VotesTimeResolution.MINUTE,
                range: VotesTimeRange.NINETY_DAYS,
            })
        ).toBe(60)
    })
})
