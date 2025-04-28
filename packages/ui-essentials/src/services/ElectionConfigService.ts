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

export const findUrlByTitle = function (answer: ICandidate, urlTitle: string): string | undefined {
    return answer.presentation?.urls?.find((url) => urlTitle === url.title)?.url
}

export const getImageUrl = function (answer: ICandidate): string | undefined {
    return answer.presentation?.urls?.find((url) => url.is_image)?.url
}

export const getLinkUrl = function (answer: ICandidate): string | undefined {
    return findUrlByTitle(answer, "URL")
}

export const checkIsCategoryList = function (candidate: ICandidate): boolean {
    return candidate.presentation?.is_category_list || false
}

export const checkIsWriteIn = function (answer: ICandidate): boolean {
    return answer.presentation?.is_write_in || false
}

export const checkIsInvalidVote = function (answer: ICandidate): boolean {
    return answer.presentation?.is_explicit_invalid || false
}

export const checkIsExplicitBlankVote = function (answer: ICandidate): boolean {
    return answer.presentation?.is_explicit_blank || false
}

export const checkPositionIsTop = function (answer: ICandidate): boolean {
    return "top" === answer.presentation?.invalid_vote_position
}

export const checkAllowWriteIns = function (question: IContest): boolean {
    return !!question.presentation?.allow_writeins
}

export const checkCustomCandidatesOrder = function (contest: IContest): boolean {
    return contest.presentation?.candidates_order === CandidatesOrder.CUSTOM
}

export const checkShuffleCategories = function (question: IContest): boolean {
    return !!question.presentation?.shuffle_categories
}

export const checkShuffleCategoryList = function (question: IContest): Array<string> {
    return question.presentation?.shuffle_category_list || []
}

export const getCheckableOptions = function (
    question: IContest
): {
    checkableLists: boolean
    checkableCandidates: boolean
} {
    const enableCheckableLists =
        question.presentation?.enable_checkable_lists || EEnableCheckableLists.CANDIDATES_AND_LISTS
    switch (enableCheckableLists) {
        case EEnableCheckableLists.DISABLED:
            return {checkableLists: false, checkableCandidates: false}
        case EEnableCheckableLists.CANDIDATES_ONLY:
            return {checkableLists: false, checkableCandidates: true}
        case EEnableCheckableLists.LISTS_ONLY:
            return {checkableLists: true, checkableCandidates: false}
        default:
            // EEnableCheckableLists.CANDIDATES_AND_LISTS:
            return {checkableLists: true, checkableCandidates: true}
    }
}

export const checkIsRadioSelection = function (contest: IContest): boolean {
    return (
        1 === contest.max_votes &&
        ECandidatesSelectionPolicy.RADIO === contest.presentation?.candidates_selection_policy
    )
}
