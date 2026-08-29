// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IContest, IDecodedVoteContest, isAcclaimedContest} from "@sequentech/ui-core"

const createAcclaimedDisplayContest = (contestId: string): IDecodedVoteContest => ({
    contest_id: contestId,
    is_explicit_invalid: false,
    is_decline_to_vote: false,
    is_blank_ballot: false,
    invalid_errors: [],
    invalid_alerts: [],
    choices: [],
})

/**
 * Adds display-only entries for acclaimed contests, which are intentionally
 * absent from the encrypted and decoded vote. Decoded entries are preserved
 * verbatim so this presentation helper cannot alter verification results.
 */
export const getConfirmationContests = (
    sortedContests: IContest[],
    decodedContests: IDecodedVoteContest[]
): IDecodedVoteContest[] => {
    const decodedContestIds = new Set(decodedContests.map(({contest_id}) => contest_id))
    const contestIndexes = new Map(
        sortedContests.map((contest, index) => [contest.id, index] as const)
    )
    const acclaimedDisplayContests = sortedContests
        .filter((contest) => isAcclaimedContest(contest) && !decodedContestIds.has(contest.id))
        .map(({id}) => createAcclaimedDisplayContest(id))

    return [...decodedContests, ...acclaimedDisplayContests].sort((first, second) => {
        const firstIndex = contestIndexes.get(first.contest_id) ?? Number.MAX_SAFE_INTEGER
        const secondIndex = contestIndexes.get(second.contest_id) ?? Number.MAX_SAFE_INTEGER
        return firstIndex - secondIndex
    })
}
