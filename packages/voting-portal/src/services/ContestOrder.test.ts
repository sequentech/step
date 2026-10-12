// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IContest} from "@sequentech/ui-core"
import {aContest} from "../testing/ballotHarness"
import {sortContestsInBallotOrder} from "./ContestOrder"

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
