// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
export interface ICandidateUrl {
    url: string
    kind?: string
    title?: string
    is_image: boolean
}

export enum IInvalidVotePosition {
    TOP = "top",
    BOTTOM = "bottom",
}

export interface ICandidatePresentation {
    i18n?: Record<string, Record<string, string>>
    is_explicit_invalid?: boolean
    is_explicit_blank?: boolean
    is_disabled?: boolean
    is_category_list?: boolean
    invalid_vote_position?: string
    is_write_in?: boolean
    sort_order?: number
    urls?: Array<ICandidateUrl>
    subtype?: string
}
