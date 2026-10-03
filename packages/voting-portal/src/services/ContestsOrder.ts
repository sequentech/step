// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ContestsOrder, IContest, sortContestList} from "@sequentech/ui-core"
import {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"

const storageKey = (ballotStyleId: string) => `contests-order-${ballotStyleId}`

const bySortOrder = (contests: IContest[]): IContest[] =>
    [...contests].sort(
        (a, b) =>
            (a.presentation?.sort_order ?? Infinity) - (b.presentation?.sort_order ?? Infinity)
    )

const storedOrder = (ballotStyleId: string, contests: IContest[]): IContest[] | undefined => {
    try {
        const ids: unknown = JSON.parse(sessionStorage.getItem(storageKey(ballotStyleId)) ?? "null")
        if (!Array.isArray(ids) || ids.length !== contests.length) {
            return undefined
        }
        const byId = new Map(contests.map((contest) => [contest.id, contest]))
        const ordered = ids.map((id) => byId.get(id))
        return ordered.every((contest) => contest !== undefined) && new Set(ids).size === ids.length
            ? (ordered as IContest[])
            : undefined
    } catch {
        return undefined
    }
}

/**
 * The contests in the order this voter is shown them, before pagination.
 *
 * A random order is drawn once per ballot and browser session, so the ballot does
 * not reorder while the voter marks it and the review screen can repeat it.
 */
export const orderContestsForVoter = (ballotStyle: IBallotStyle): IContest[] => {
    const contests = ballotStyle.ballot_eml.contests ?? []
    const order = ballotStyle.ballot_eml.election_presentation?.contests_order
    if (order !== ContestsOrder.RANDOM) {
        return bySortOrder(contests)
    }
    const stored = storedOrder(ballotStyle.id, contests)
    if (stored) {
        return stored
    }
    const shuffled = sortContestList(contests, order, true)
    sessionStorage.setItem(
        storageKey(ballotStyle.id),
        JSON.stringify(shuffled.map((contest) => contest.id))
    )
    return shuffled
}

/** Contests grouped into the ballot's pages, by `pagination_policy`, keeping their order. */
export const paginateContests = (contests: IContest[]): IContest[][] => {
    const pages = new Map<string, IContest[]>()
    for (const contest of contests) {
        const page = contest.presentation?.pagination_policy || ""
        pages.set(page, [...(pages.get(page) ?? []), contest])
    }
    return Array.from(pages.values())
}

/** The contests as the review screen lists them. */
export const orderContestsForReview = (ballotStyle: IBallotStyle): IContest[] => {
    const order = ballotStyle.ballot_eml.election_presentation?.contests_order
    return order === ContestsOrder.RANDOM
        ? paginateContests(orderContestsForVoter(ballotStyle)).flat()
        : sortContestList(ballotStyle.ballot_eml.contests, order)
}
