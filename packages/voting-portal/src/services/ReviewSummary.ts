// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BallotSelection,
    checkIsCategoryList,
    checkIsExplicitBlankVote,
    checkIsInvalidVote,
    IContest,
    isAcclaimedContest,
} from "@sequentech/ui-core"
import {
    ESlateSelectionStatus,
    getSlateMembers,
    getSlateSelectionSummary,
    ISlateSelectionSummary,
} from "./SlateSelection"
import {IBallotSlates, IResolvedSlate} from "./Slates"

export interface IContestReviewCount {
    selected: number
    max: number
}

export interface ISlateReviewLine {
    slate: IResolvedSlate
    summary: ISlateSelectionSummary
}

export interface IBallotReviewSummary {
    /** The selected candidates of the whole ballot. */
    selected: number
    /** The candidates the voter may select at most, over every contest. */
    seats: number
    /** The count of each contest, by contest id. Acclaimed contests have none. */
    contests: Record<string, IContestReviewCount>
    /** The slates with a selected member, in their configured order. */
    slates: Array<ISlateReviewLine>
    /** The selected candidates that belong to no slate. */
    independent: number
}

const getSelectedCandidateIds = (
    contest: IContest,
    selection: BallotSelection | undefined
): Array<string> => {
    const decoded = selection?.find((entry) => entry.contest_id === contest.id)
    if (!decoded) {
        return []
    }
    return contest.candidates
        .filter(
            (candidate) =>
                !checkIsExplicitBlankVote(candidate) &&
                !checkIsInvalidVote(candidate) &&
                !checkIsCategoryList(candidate)
        )
        .filter((candidate) =>
            decoded.choices.some((choice) => choice.id === candidate.id && choice.selected > -1)
        )
        .map((candidate) => candidate.id)
}

/**
 * What the review screen says about a ballot with slates, read from the
 * ordinary candidate selections. No slate choice is stored anywhere, so a
 * slate that was chosen and then changed by hand is reported as it is now.
 */
export const getBallotReviewSummary = (
    contests: Array<IContest>,
    slates: IBallotSlates,
    selection: BallotSelection | undefined
): IBallotReviewSummary => {
    const slateMemberIds = new Set(
        slates.slates.flatMap((slate) =>
            slate.contests.flatMap((entry) => entry.candidates.map((candidate) => candidate.id))
        )
    )
    const summary: IBallotReviewSummary = {
        selected: 0,
        seats: 0,
        contests: {},
        slates: [],
        independent: 0,
    }

    for (const contest of contests) {
        if (isAcclaimedContest(contest)) {
            continue
        }
        const selectedIds = getSelectedCandidateIds(contest, selection)
        summary.contests[contest.id] = {selected: selectedIds.length, max: contest.max_votes}
        summary.selected += selectedIds.length
        summary.seats += contest.max_votes
        summary.independent += selectedIds.filter((id) => !slateMemberIds.has(id)).length
    }

    summary.slates = slates.slates
        .map((slate) => ({
            slate,
            summary: getSlateSelectionSummary(getSlateMembers(slate), selection),
        }))
        .filter((line) => line.summary.status !== ESlateSelectionStatus.NONE)

    return summary
}
