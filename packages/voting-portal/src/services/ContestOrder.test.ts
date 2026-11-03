// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IContest} from "@sequentech/ui-core"
import {aCandidate, aContest} from "../testing/ballotHarness"
import {sortCandidatesInBallotOrder, sortContestsInBallotOrder} from "./ContestOrder"

const contest = (id: string, sortOrder?: number): IContest =>
    aContest({id, presentation: sortOrder === undefined ? {} : {sort_order: sortOrder}})

describe("sortContestsInBallotOrder", () => {
    it("orders the contests by their configured position, not by id", () => {
        const contests = [contest("a-vice", 1), contest("b-president", 0), contest("c-trustees", 2)]

        expect(sortContestsInBallotOrder(contests).map(({id}) => id)).toEqual([
            "b-president",
            "a-vice",
            "c-trustees",
        ])
    })

    it("puts contests without a position last, keeping their order", () => {
        const contests = [contest("x"), contest("y", 0), contest("z")]

        expect(sortContestsInBallotOrder(contests).map(({id}) => id)).toEqual(["y", "x", "z"])
    })

    it("does not change the list it is given", () => {
        const contests = [contest("a", 1), contest("b", 0)]

        sortContestsInBallotOrder(contests)

        expect(contests.map(({id}) => id)).toEqual(["a", "b"])
    })
})

describe("sortCandidatesInBallotOrder", () => {
    const candidates = [
        aCandidate("a-first-id", "Zoe", {presentation: {sort_order: 2}}),
        aCandidate("b-second-id", "Yann", {presentation: {sort_order: 1}}),
        aCandidate("c-third-id", "Xavier", {presentation: {sort_order: 0}}),
    ]
    const ids = (contest: IContest) =>
        sortCandidatesInBallotOrder(contest, contest.candidates).map(({id}) => id)

    it("follows the configured position of the candidates in a custom order", () => {
        const contest = aContest({candidates, presentation: {candidates_order: "custom"}} as never)
        expect(ids(contest)).toEqual(["c-third-id", "b-second-id", "a-first-id"])
    })

    it("follows the names in an alphabetical order", () => {
        const contest = aContest({
            candidates,
            presentation: {candidates_order: "alphabetical"},
        } as never)
        expect(ids(contest)).toEqual(["c-third-id", "b-second-id", "a-first-id"])
    })

    it("keeps the ballot's order otherwise", () => {
        expect(
            ids(aContest({candidates, presentation: {candidates_order: "random"}} as never))
        ).toEqual(["a-first-id", "b-second-id", "c-third-id"])
        expect(ids(aContest({candidates}))).toEqual(["a-first-id", "b-second-id", "c-third-id"])
    })

    it("keeps the candidate objects it was given", () => {
        const contest = aContest({candidates, presentation: {candidates_order: "custom"}} as never)
        expect(sortCandidatesInBallotOrder(contest, contest.candidates)[0]).toBe(candidates[2])
    })
})
