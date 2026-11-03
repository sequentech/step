// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EUnderVotePolicy,
    IContest,
    IDecodedVoteContest,
    isAcclaimedContest,
} from "@sequentech/ui-core"

const UNDER_VOTE_ALERT = "errors.implicit.underVote"

export interface IUnfilledContest {
    contest: IContest
    selected: number
    max: number
}

const toCount = (value: string | undefined, fallback: number): number => {
    const count = Number.parseInt(value ?? "", 10)
    return Number.isNaN(count) ? fallback : count
}

// The contests the voter has to confirm before casting: those whose under-vote
// policy asks for it and that the ballot checker reports as under-voted.
export const getUnfilledContests = (
    contests: Array<IContest>,
    decodedContests: Array<IDecodedVoteContest>
): Array<IUnfilledContest> =>
    contests.flatMap((contest) => {
        if (
            isAcclaimedContest(contest) ||
            contest.presentation?.under_vote_policy !== EUnderVotePolicy.WARN_AND_CONFIRM_IN_REVIEW
        ) {
            return []
        }
        const decoded = decodedContests.find((item) => item.contest_id === contest.id)
        const underVote = decoded?.invalid_alerts.find(
            (alert) => alert.message === UNDER_VOTE_ALERT
        )
        if (!decoded || decoded.is_explicit_invalid || !underVote) {
            return []
        }
        return [
            {
                contest,
                selected: toCount(underVote.message_map?.numSelected, 0),
                max: toCount(underVote.message_map?.max, contest.max_votes),
            },
        ]
    })
