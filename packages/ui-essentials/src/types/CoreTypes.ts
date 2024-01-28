// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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

export interface IContestPresentation {
    allow_writeins: boolean
    base32_writeins: boolean
    invalid_vote_policy: string
    cumulative_number_of_checkboxes?: number
    shuffle_categories: boolean
    shuffle_all_options: boolean
    shuffle_category_list?: Array<string>
    show_points: boolean
    enable_checkable_lists?: string
    candidates_order?: CandidatesOrder
}

export interface IContest {
    id: string
    tenant_id: string
    election_event_id: string
    election_id: string
    name?: string
    description?: string
    max_votes: number
    min_votes: number
    voting_type?: string
    counting_algorithm?: string
    is_encrypted: boolean
    candidates: Array<ICandidate>
    presentation?: IContestPresentation
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
    description?: string
    candidate_type?: string
    presentation?: ICandidatePresentation
}

export interface IBallotStyle {
    id: string
    tenant_id: string
    election_event_id: string
    election_id: string
    description?: string
    public_key?: IPublicKeyConfig
    area_id: string
    contests: Array<IContest>
}

export interface IPublicKeyConfig {
    public_key: string
    is_demo: boolean
}
