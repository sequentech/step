// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ContestsOrder, IContest, sortContestList} from "@sequentech/ui-core"
import {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {orderContestsForReview, orderContestsForVoter, paginateContests} from "./ContestsOrder"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    sortContestList: jest.fn(),
}))

const sort = sortContestList as jest.MockedFunction<typeof sortContestList>

const contest = (id: string, sortOrder?: number, page?: string) =>
    ({id, presentation: {sort_order: sortOrder, pagination_policy: page}}) as IContest

const style = (contests: IContest[], order?: ContestsOrder, id = "style") =>
    ({
        id,
        ballot_eml: {contests, election_presentation: {contests_order: order}},
    }) as unknown as IBallotStyle

const ids = (contests: IContest[]) => contests.map((each) => each.id)

beforeEach(() => {
    sessionStorage.clear()
    sort.mockReset()
    sort.mockImplementation((contests) => [...contests].reverse())
})

it("orders by sort_order unless the order is random, unsorted contests last", () => {
    const contests = [contest("c", 2), contest("x"), contest("a", 0), contest("b", 1)]
    for (const order of [undefined, ContestsOrder.CUSTOM, ContestsOrder.ALPHABETICAL]) {
        expect(ids(orderContestsForVoter(style(contests, order)))).toEqual(["a", "b", "c", "x"])
    }
    expect(sort).not.toHaveBeenCalled()
})

it("draws a random order once and repeats it for the ballot", () => {
    const ballot = style([contest("a", 0), contest("b", 1), contest("c", 2)], ContestsOrder.RANDOM)
    expect(ids(orderContestsForVoter(ballot))).toEqual(["c", "b", "a"])
    expect(sort).toHaveBeenCalledWith(ballot.ballot_eml.contests, ContestsOrder.RANDOM, true)
    sort.mockImplementation((contests) => [...contests])
    expect(ids(orderContestsForVoter(ballot))).toEqual(["c", "b", "a"])
    expect(sort).toHaveBeenCalledTimes(1)
})

it("draws again for another ballot, and when the stored order is not this ballot's", () => {
    const contests = [contest("a", 0), contest("b", 1)]
    orderContestsForVoter(style(contests, ContestsOrder.RANDOM))
    orderContestsForVoter(style(contests, ContestsOrder.RANDOM, "other"))
    expect(sort).toHaveBeenCalledTimes(2)

    for (const stale of ['["a","z"]', '["a","a"]', '["a"]', "{", '"a"']) {
        sessionStorage.setItem("contests-order-style", stale)
        sort.mockClear()
        expect(ids(orderContestsForVoter(style(contests, ContestsOrder.RANDOM)))).toEqual([
            "b",
            "a",
        ])
        expect(sort).toHaveBeenCalledTimes(1)
    }
})

it("groups contests into pages in the order given", () => {
    const pages = paginateContests([
        contest("a", 0, "one"),
        contest("b", 1),
        contest("c", 2, "one"),
        contest("d", 3),
    ])
    expect(pages.map(ids)).toEqual([
        ["a", "c"],
        ["b", "d"],
    ])
    expect(paginateContests([])).toEqual([])
})

it("reviews a random ballot page by page in the order the voter saw", () => {
    const ballot = style(
        [contest("a", 0, "one"), contest("b", 1, "two"), contest("c", 2, "one")],
        ContestsOrder.RANDOM
    )
    expect(ids(orderContestsForVoter(ballot))).toEqual(["c", "b", "a"])
    expect(ids(orderContestsForReview(ballot))).toEqual(["c", "a", "b"])
})

it("reviews any other order as the ordering engine returns it", () => {
    const ballot = style([contest("a", 0), contest("b", 1)], ContestsOrder.CUSTOM)
    expect(ids(orderContestsForReview(ballot))).toEqual(["b", "a"])
    expect(sort).toHaveBeenCalledWith(ballot.ballot_eml.contests, ContestsOrder.CUSTOM)
})
