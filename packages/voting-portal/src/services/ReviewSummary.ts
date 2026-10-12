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
import {ESlateSelectionStatus, getSlateMembers, getSlateSelectionSummary} from "./SlateSelection"
import {IBallotSlates, IResolvedSlate} from "./Slates"

export interface IContestReviewCount {
    selected: number
    max: number
}

export interface IBallotReviewSummary {
    /** The selected candidates of the whole ballot. */
    selected: number
    /** The candidates the voter may select at most, over every contest. */
    seats: number
    /** The count of each contest, by contest id. Acclaimed contests have none. */
    contests: Record<string, IContestReviewCount>
    /** What the ballot holds as a whole. */
    label: IBallotSelectionLabel
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
    const counted = contests.filter((contest) => !isAcclaimedContest(contest))
    const {selected, seats} = getBallotSelectionCount(counted, selection)
    return {
        selected,
        seats,
        contests: Object.fromEntries(
            counted.map((contest) => [
                contest.id,
                {
                    selected: getSelectedCandidateIds(contest, selection).length,
                    max: contest.max_votes,
                },
            ])
        ),
        label: getBallotSelectionLabel(slates, selection),
    }
}

export interface IBallotSelectionCount {
    selected: number
    seats: number
}

/** How many candidates the ballot holds, out of how many it may hold. */
/** How many candidates a contest holds. */
export const getContestSelectionCount = (
    contest: IContest,
    selection: BallotSelection | undefined
): number => getSelectedCandidateIds(contest, selection).length

export const getBallotSelectionCount = (
    contests: Array<IContest>,
    selection: BallotSelection | undefined
): IBallotSelectionCount =>
    contests
        .filter((contest) => !isAcclaimedContest(contest))
        .reduce(
            (count, contest) => ({
                selected: count.selected + getSelectedCandidateIds(contest, selection).length,
                seats: count.seats + contest.max_votes,
            }),
            {selected: 0, seats: 0}
        )

export enum EBallotSelectionLabel {
    SLATE = "slate",
    INDIVIDUAL = "individual",
    NONE = "none",
}

export interface IBallotSelectionLabel {
    kind: EBallotSelectionLabel
    /** The slate the ballot holds exactly, with nothing else. */
    slate?: IResolvedSlate
}

/** What the ballot holds as a whole: exactly one slate, other choices, or nothing. */
export const getBallotSelectionLabel = (
    slates: IBallotSlates,
    selection: BallotSelection | undefined
): IBallotSelectionLabel => {
    const exact = slates.slates.find(
        (slate) =>
            getSlateSelectionSummary(getSlateMembers(slate), selection).status ===
            ESlateSelectionStatus.ALL
    )
    if (exact) {
        return {kind: EBallotSelectionLabel.SLATE, slate: exact}
    }
    const hasChoice = (selection ?? []).some(
        (contest) =>
            contest.is_explicit_invalid || contest.choices.some((choice) => choice.selected > -1)
    )
    return {kind: hasChoice ? EBallotSelectionLabel.INDIVIDUAL : EBallotSelectionLabel.NONE}
}
