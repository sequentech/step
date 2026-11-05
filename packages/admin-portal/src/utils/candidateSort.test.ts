// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Sequent_Backend_Candidate_Extended} from "../resources/Tally/types"
import {sortCandidates} from "./candidateSort"

const candidate = (id: string, winning_position?: number, cast_votes?: number) =>
    ({id, winning_position, cast_votes}) as Sequent_Backend_Candidate_Extended

describe("sortCandidates", () => {
    it("lists winners by position, then the rest by votes, most first", () => {
        const sorted = [
            candidate("third-most-votes", undefined, 50),
            candidate("second-winner", 2, 10),
            candidate("first-winner", 1, 5),
            candidate("most-votes", undefined, 70),
            candidate("no-votes"),
        ].sort(sortCandidates)
        expect(sorted.map(({id}) => id)).toEqual([
            "first-winner",
            "second-winner",
            "most-votes",
            "third-most-votes",
            "no-votes",
        ])
    })

    it("treats candidates without winners or votes as equal", () => {
        expect(sortCandidates(candidate("a"), candidate("b"))).toBe(0)
    })
})
