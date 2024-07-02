// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationDict} from "../services/translate"
import {IElectionEventPresentation} from "./ElectionEventPresentation"
import {IContestPresentation} from "./ContestPresentation"
import {ICandidatePresentation} from "./CandidatePresentation"
import {IElectionPresentation} from "./ElectionPresentation"

export enum EVotingStatus {
    NOT_STARTED = "NOT_STARTED",
    OPEN = "OPEN",
    PAUSED = "PAUSED",
    CLOSED = "CLOSED",
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
    num_emails_sent?: number
    num_sms_sent?: number
}

export interface IElectionStatistics {
    num_emails_sent?: number
    num_sms_sent?: number
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

export interface IElection {
    id: string
    election_event_id: string
    name?: string
    name_i18n?: TranslationDict
    description?: string
    description_i18n?: TranslationDict
    alias?: string
    alias_i18n?: TranslationDict
    image_document_id: string
    contests: Array<IContest>
    presentation?: IElectionPresentation
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
    election_presentation?: IElectionPresentation
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

export enum EInvalidPlaintextErrorType {
    Explicit = "Explicit",
    Implicit = "Implicit",
    EncodingError = "EncodingError",
}
