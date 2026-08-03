// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {normalizeAreaContestResults} from "./validation"

describe("normalizeAreaContestResults", () => {
    it("converts numeric strings before values cross the Rust/WASM boundary", () => {
        const normalized = normalizeAreaContestResults({
            area_id: "area-1",
            contest_id: "contest-1",
            total_votes: "12",
            total_valid_votes: "10",
            invalid_votes: {
                total_invalid: "2",
                implicit_invalid: "1",
                explicit_invalid: "1",
            },
            total_blank_votes: "3",
            census: "20",
            candidate_results: {
                "candidate-1": {
                    candidate_id: "candidate-1",
                    total_votes: "7",
                },
            },
        } as unknown as Parameters<typeof normalizeAreaContestResults>[0])

        expect(normalized).toEqual({
            area_id: "area-1",
            contest_id: "contest-1",
            total_votes: 12,
            total_valid_votes: 10,
            invalid_votes: {
                total_invalid: 2,
                implicit_invalid: 1,
                explicit_invalid: 1,
            },
            total_blank_votes: 3,
            census: 20,
            candidate_results: {
                "candidate-1": {
                    candidate_id: "candidate-1",
                    total_votes: 7,
                },
            },
        })
    })

    it("turns cleared and invalid counts into absent optional values", () => {
        const normalized = normalizeAreaContestResults({
            area_id: "area-1",
            contest_id: "contest-1",
            total_votes: "0",
            total_valid_votes: "",
            total_blank_votes: "not-a-number",
            candidate_results: {},
        } as unknown as Parameters<typeof normalizeAreaContestResults>[0])

        expect(normalized.total_votes).toBe(0)
        expect(normalized.total_valid_votes).toBeUndefined()
        expect(normalized.total_blank_votes).toBeUndefined()
    })
})
