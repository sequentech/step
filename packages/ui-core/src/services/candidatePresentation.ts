// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ICandidate} from "../types/CoreTypes"

export const getImageUrl = (answer: ICandidate): string | undefined =>
    answer.presentation?.urls?.find((url) => url.is_image)?.url

export const checkIsWriteIn = (answer: ICandidate): boolean =>
    answer.presentation?.is_write_in || false

export const checkIsInvalidVote = (answer: ICandidate): boolean =>
    answer.presentation?.is_explicit_invalid || false

export const checkIsCategoryList = (candidate: ICandidate): boolean =>
    candidate.presentation?.is_category_list || false

export const checkIsExplicitBlankVote = (candidate: ICandidate): boolean =>
    candidate.presentation?.is_explicit_blank || false

export const checkIsDisabled = (candidate: ICandidate): boolean =>
    candidate.presentation?.is_disabled || false
