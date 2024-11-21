// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    CandidatesOrder,
    ICandidate,
    IContest,
    ECandidatesSelectionPolicy,
    EEnableCheckableLists,
} from "@sequentech/ui-core"

export const findUrlByTitle = (answer: ICandidate, urlTitle: string): string | undefined =>
    answer.presentation?.urls?.find((url) => urlTitle === url.title)?.url

export const getImageUrl = (answer: ICandidate): string | undefined =>
    answer.presentation?.urls?.find((url) => url.is_image)?.url

export const getLinkUrl = (answer: ICandidate): string | undefined => findUrlByTitle(answer, "URL")

export const checkIsCategoryList = (candidate: ICandidate): boolean =>
    candidate.presentation?.is_category_list || false

export const checkIsWriteIn = (answer: ICandidate): boolean =>
    answer.presentation?.is_write_in || false

export const checkIsInvalidVote = (answer: ICandidate): boolean =>
    answer.presentation?.is_explicit_invalid || false

export const checkIsExplicitBlankVote = (answer: ICandidate): boolean =>
    answer.presentation?.is_explicit_blank || false

export const checkPositionIsTop = (answer: ICandidate): boolean =>
    "top" === answer.presentation?.invalid_vote_position

export const checkAllowWriteIns = (question: IContest): boolean =>
    !!question.presentation?.allow_writeins

export const checkCustomCandidatesOrder = (contest: IContest): boolean =>
    contest.presentation?.candidates_order === CandidatesOrder.CUSTOM

export const checkShuffleCategories = (question: IContest): boolean =>
    !!question.presentation?.shuffle_categories

export const checkShuffleCategoryList = (question: IContest): Array<string> =>
    question.presentation?.shuffle_category_list || []

export const getCheckableOptions = (
    question: IContest
): {checkableLists: boolean; checkableCandidates: boolean} => {
    const enableCheckableLists =
        question.presentation?.enable_checkable_lists || EEnableCheckableLists.CANDIDATES_AND_LISTS
    switch (enableCheckableLists) {
        case EEnableCheckableLists.DISABLED:
            return {checkableLists: false, checkableCandidates: false}
        case EEnableCheckableLists.CANDIDATES_ONLY:
            return {checkableLists: false, checkableCandidates: true}
        case EEnableCheckableLists.LISTS_ONLY:
            return {checkableLists: true, checkableCandidates: false}
        default: // EEnableCheckableLists.CANDIDATES_AND_LISTS:
            return {checkableLists: true, checkableCandidates: true}
    }
}

export const checkIsRadioSelection = (contest: IContest): boolean => {
    return (
        1 === contest.max_votes &&
        ECandidatesSelectionPolicy.RADIO === contest.presentation?.candidates_selection_policy
    )
}
