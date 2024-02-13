// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationDict} from "@root/services/translate"
import {IElectionEventPresentation} from "./ElectionEventPresentation"

export enum EVotingStatus {
    NOT_STARTED = "NOT_STARTED",
    OPEN = "OPEN",
    PAUSED = "PAUSED",
    CLOSED = "CLOSED",
}

export enum CandidatesOrder {
    RANDOM = "random",
    CUSTOM = "custom",
    ALPHABETICAL = "alphabetical",
}

export interface IElectionEventStatus {
    config_created?: boolean
    keys_ceremony_finished?: boolean
    tally_ceremony_finished?: boolean
    is_published?: boolean
    voting_status: EVotingStatus
}

export interface IElectionStatus {
    voting_status: EVotingStatus
}

export interface IElectionEventStatistics {
    num_emails_sent: number
    num_sms_sent: number
}

export interface IElectionStatistics {
    num_emails_sent: number
    num_sms_sent: number
}

export enum EInvalidVotePolicy {
    ALLOWED = "allowed",
    WARN = "warn",
    WARN_INVALID_IMPLICIT_AND_EXPLICIT = "warn-invalid-implicit-and-explicit",
    NOT_ALLOWED = "not-allowed",
}

export enum ECandidatesSelectionPolicy {
    RADIO = "radio", // if you select one, the previously selected one gets unselected
    CUMULATIVE = "cumulative", // default behaviour
}

export interface IContestPresentation {
    allow_writeins: boolean
    base32_writeins: boolean
    invalid_vote_policy: string
    cumulative_number_of_checkboxes?: number
    shuffle_categories: boolean
    shuffle_category_list?: Array<string>
    show_points: boolean
    enable_checkable_lists?: string
    candidates_order?: CandidatesOrder
    candidates_selection_policy?: ECandidatesSelectionPolicy
}

export interface IContest {
    id: string
    tenant_id: string
    election_event_id: string
    election_id: string
    name?: string
    name_i18n?: TranslationDict
    description?: string
    description_i18n?: TranslationDict
    alias?: string
    alias_i18n?: TranslationDict
    max_votes: number
    min_votes: number
    winning_candidates_num: number
    voting_type?: string
    counting_algorithm?: string
    is_encrypted: boolean
    candidates: Array<ICandidate>
    presentation?: IContestPresentation
    created_at?: string
}

export interface ICandidateUrl {
    url: string
    kind?: string
    title?: string
    is_image: boolean
}

export interface ICandidatePresentation {
    is_explicit_invalid: boolean
    is_category_list: boolean
    invalid_vote_position?: string
    is_write_in: boolean
    sort_order?: number
    urls?: Array<ICandidateUrl>
}

export interface ICandidate {
    id: string
    tenant_id: string
    election_event_id: string
    election_id: string
    contest_id: string
    name?: string
    name_i18n?: TranslationDict
    description?: string
    description_i18n?: TranslationDict
    alias?: string
    alias_i18n?: TranslationDict
    candidate_type?: string
    presentation?: ICandidatePresentation
}

export interface IBallotStyle {
    id: string
    tenant_id: string
    election_event_id: string
    election_id: string
    num_allowed_revotes?: number
    description?: string
    public_key?: IPublicKeyConfig
    area_id: string
    contests: Array<IContest>
    election_event_presentation?: IElectionEventPresentation
}

export interface IPublicKeyConfig {
    public_key: string
    is_demo: boolean
}

export interface IAuditableBallot {
    version: number
    issue_date: string
    config: IBallotStyle
    contests: Array<string>
    ballot_hash: string
}

export interface IHashableBallot {
    version: number
    issue_date: string
    contests: Array<string>
    config: IBallotStyle
}
