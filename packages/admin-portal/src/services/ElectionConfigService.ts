// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ICandidate, IContest} from "@sequentech/ui-core"

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

export const checkIsDisabled = (answer: ICandidate): boolean =>
    answer.presentation?.is_disabled || false

export const checkPositionIsTop = (answer: ICandidate): boolean =>
    "top" === answer.presentation?.invalid_vote_position

export const checkAllowWriteIns = (question: IContest): boolean =>
    !!question.presentation?.allow_writeins

export const checkShuffleCategories = (question: IContest): boolean =>
    !!question.presentation?.shuffle_categories

export const checkShuffleCategoryList = (question: IContest): Array<string> =>
    question.presentation?.shuffle_category_list || []

export const getCheckableOptions = (
    question: IContest
): {checkableLists: boolean; checkableCandidates: boolean} => {
    const enableCheckableLists = question.presentation?.enable_checkable_lists || "disabled"
    switch (enableCheckableLists) {
        case "allow-selecting-candidates-and-lists":
            return {checkableLists: true, checkableCandidates: true}
        case "allow-selecting-candidates":
            return {checkableLists: false, checkableCandidates: true}
        case "allow-selecting-lists":
            return {checkableLists: true, checkableCandidates: false}
        default:
            return {checkableLists: false, checkableCandidates: false}
    }
}
