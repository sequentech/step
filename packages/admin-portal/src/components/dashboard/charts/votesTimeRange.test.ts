// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    DEFAULT_VOTES_TIME_SELECTION,
    getVotesBucketCount,
    getVotesTimeRangeOptions,
    withVotesTimeResolution,
} from "./votesTimeRange"

describe("votesTimeRange", () => {
    it("defaults to the seven-day daily view", () => {
        expect(DEFAULT_VOTES_TIME_SELECTION).toEqual({resolution: "day", range: "7d"})
        expect(getVotesBucketCount(DEFAULT_VOTES_TIME_SELECTION)).toBe(7)
    })

    it("uses a compact default window when resolution changes", () => {
        expect(withVotesTimeResolution("minute")).toEqual({
            resolution: "minute",
            range: "1h",
        })
        expect(withVotesTimeResolution("hour")).toEqual({
            resolution: "hour",
            range: "24h",
        })
    })

    it("keeps every preset below the server bucket limit", () => {
        for (const resolution of ["minute", "hour", "day"] as const) {
            for (const option of getVotesTimeRangeOptions(resolution)) {
                expect(option.bucketCount).toBeGreaterThan(0)
                expect(option.bucketCount).toBeLessThanOrEqual(1000)
            }
        }
    })

    it("falls back safely if a stale range is paired with a resolution", () => {
        expect(getVotesBucketCount({resolution: "minute", range: "90d"})).toBe(60)
    })
})
