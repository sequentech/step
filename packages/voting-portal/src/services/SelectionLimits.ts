// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ECandidatesSelectionPolicy,
    EInvalidVotePolicy,
    EOverVotePolicy,
    IContest,
    IDecodedVoteContest,
} from "@sequentech/ui-core"

const UNSELECTED = -1

export const isSelected = (choice: {selected: number}): boolean => choice.selected > UNSELECTED

/** Choosing a candidate replaces the current one instead of adding to it. */
export const replacesOnSelect = (contest: IContest): boolean =>
    contest.max_votes === 1 &&
    contest.presentation?.candidates_selection_policy === ECandidatesSelectionPolicy.RADIO

/** A selection above the maximum is refused, not recorded and reported. */
export const refusesOverVotes = (contest: IContest): boolean =>
    contest.presentation?.over_vote_policy === EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE

/**
 * How many of the contest's choices are taken, counted as the ballot codec
 * counts them: the explicit blank option is not a choice, and an explicit
 * invalid mark is one unless selecting a candidate clears it.
 */
export const countSelections = (contest: IContest, selection: IDecodedVoteContest): number => {
    const blankIds = new Set(
        contest.candidates
            .filter((candidate) => candidate.presentation?.is_explicit_blank)
            .map((candidate) => candidate.id)
    )
    const candidates = selection.choices.filter(
        (choice) => isSelected(choice) && !blankIds.has(choice.id)
    ).length
    const countsInvalidMark =
        selection.is_explicit_invalid &&
        contest.presentation?.invalid_vote_policy !==
            EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT
    return candidates + (countsInvalidMark ? 1 : 0)
}

export const exceedsMaximum = (contest: IContest, selections: number): boolean =>
    selections > contest.max_votes
