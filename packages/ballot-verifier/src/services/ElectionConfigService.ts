// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ICandidate} from "@sequentech/ui-core"

export const findUrlByTitle = (answer: ICandidate, urlTitle: string): string | undefined =>
    answer.presentation?.urls?.find((url) => urlTitle === url.title)?.url

export const getImageUrl = (answer: ICandidate): string | undefined =>
    answer.presentation?.urls?.find((url) => url.is_image)?.url

export const checkIsWriteIn = (answer: ICandidate): boolean =>
    answer.presentation?.is_write_in || false

export const checkIsInvalidVote = (answer: ICandidate): boolean =>
    answer.presentation?.is_explicit_invalid || false
