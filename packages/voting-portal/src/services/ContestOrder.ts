// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {sortCandidatesInContest} from "@sequentech/ui-core"
import type {ICandidate, IContest} from "@sequentech/ui-core"

/**
 * The contests in the order the ballot shows them: by their configured
 * position, with the contests without one last. The ballot style lists them
 * in another order.
 */
export const sortContestsInBallotOrder = (contests: Array<IContest>): Array<IContest> =>
    [...contests].sort(
        (a, b) =>
            (a.presentation?.sort_order ?? Infinity) - (b.presentation?.sort_order ?? Infinity)
    )

/**
 * Candidates of a contest in the order the ballot shows them: its configured
 * candidate order, without shuffling a random one.
 */
export const sortCandidatesInBallotOrder = (
    contest: IContest,
    candidates: Array<ICandidate>
): Array<ICandidate> => {
    if (candidates.length < 2) {
        return candidates
    }
    const sortedIds = sortCandidatesInContest(
        candidates,
        contest.presentation?.candidates_order,
        false
    ).map((candidate) => candidate.id)
    return sortedIds.flatMap((id) => candidates.find((candidate) => candidate.id === id) ?? [])
}
