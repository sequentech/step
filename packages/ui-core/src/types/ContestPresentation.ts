// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum CandidatesOrder {
    RANDOM = "random",
    CUSTOM = "custom",
    ALPHABETICAL = "alphabetical",
}

export enum ECandidatesSelectionPolicy {
    RADIO = "radio", // if you select one, the previously selected one gets unselected
    CUMULATIVE = "cumulative", // default behaviour
}

export enum EInvalidVotePolicy {
    ALLOWED = "allowed",
    WARN = "warn",
    WARN_INVALID_IMPLICIT_AND_EXPLICIT = "warn-invalid-implicit-and-explicit",
    NOT_ALLOWED = "not-allowed",
}

export enum EEnableCheckableLists {
    CANDIDATES_AND_LISTS = "allow-selecting-candidates-and-lists",
    CANDIDATES_ONLY = "allow-selecting-candidates",
    LISTS_ONLY = "allow-selecting-lists",
    DISABLED = "disabled",
}

export enum ECandidatesIconCheckboxPolicy {
    SQUARE_CHECKBOX = "square-checkbox",
    ROUND_CHECKBOX = "round-checkbox",
}

export enum EBlankVotePolicy {
    ALLOWED = "allowed",
    WARN = "warn",
    WARN_ONLY_IN_REVIEW = "warn-only-in-review",
    NOT_ALLOWED = "not-allowed",
}

export enum EUnderVotePolicy {
    ALLOWED = "allowed",
    WARN = "warn",
    WARN_ONLY_IN_REVIEW = "warn-only-in-review",
    WARN_AND_ALERT = "warn-and-alert",
}

export enum EOverVotePolicy {
    ALLOWED = "allowed",
    ALLOWED_WITH_MSG = "allowed-with-msg",
    ALLOWED_WITH_MSG_AND_ALERT = "allowed-with-msg-and-alert",
    NOT_ALLOWED_WITH_MSG_AND_ALERT = "not-allowed-with-msg-and-alert",
    NOT_ALLOWED_WITH_MSG_AND_DISABLE = "not-allowed-with-msg-and-disable",
}

export interface ITypePresentation {
    name?: string
    name_i18n?: Record<string, string>
    sort_order?: number
    subtypes_presentation?: Record<string, ITypePresentation>
}

export interface IContestPresentation {
    i18n?: Record<string, Record<string, string>>
    allow_writeins?: boolean
    base32_writeins?: boolean
    invalid_vote_policy?: EInvalidVotePolicy
    blank_vote_policy?: EBlankVotePolicy
    over_vote_policy?: EOverVotePolicy
    pagination_policy?: String
    cumulative_number_of_checkboxes?: number
    shuffle_categories?: boolean
    shuffle_category_list?: Array<string>
    show_points?: boolean
    enable_checkable_lists?: EEnableCheckableLists
    candidates_icon_checkbox_policy?: ECandidatesIconCheckboxPolicy
    candidates_order?: CandidatesOrder
    candidates_selection_policy?: ECandidatesSelectionPolicy
    types_presentation?: Record<string, ITypePresentation>
    sort_order?: number
    under_vote_policy?: EUnderVotePolicy
    columns?: number
}
