// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationDict} from "../services/translate"
import {IElectionEventPresentation} from "./ElectionEventPresentation"
import {IContestPresentation} from "./ContestPresentation"
import {IAreaPresentation} from "./AreaPresentation"
import {ICandidatePresentation} from "./CandidatePresentation"
import {IElectionDates, IElectionPresentation} from "./ElectionPresentation"

export enum EAllowTally {
    ALLOWED = "allowed",
    DISALLOWED = "disallowed",
    REQUIRES_VOTING_PERIOD_END = "requires-voting-period-end",
}

export enum EInitReport {
    ALLOWED = "allowed",
    DISALLOWED = "disallowed",
}

export enum EVotingStatus {
    NOT_STARTED = "NOT_STARTED",
    OPEN = "OPEN",
    PAUSED = "PAUSED",
    CLOSED = "CLOSED",
}

export interface IVotingChannelsConfig {
    kiosk: boolean
    online: boolean
    early_voting: boolean
}

export interface IChannelButtonInfo {
    status: EVotingStatus
    is_channel_enabled: boolean
}

export interface IPeriodDates {
    first_started_at?: string
    last_started_at?: string
    first_paused_at?: string
    last_paused_at?: string
    first_stopped_at?: string
    last_stopped_at?: string
}

export interface IElectionEventStatus {
    is_published?: boolean
    voting_status: EVotingStatus
    kiosk_voting_status: EVotingStatus
    early_voting_status: EVotingStatus
}

export interface IElectionStatus {
    is_published?: boolean
    voting_status: EVotingStatus
    kiosk_voting_status: EVotingStatus
    early_voting_status: EVotingStatus
    voting_period_dates: IPeriodDates
    kiosk_voting_period_dates: IPeriodDates
    early_voting_period_dates: IPeriodDates
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
    tenant_id: string
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
    area_presentation?: IAreaPresentation
    contests: Array<IContest>
    election_event_presentation?: IElectionEventPresentation
    election_presentation?: IElectionPresentation
    election_dates?: IElectionDates
}

export interface IPublicKeyConfig {
    public_key: string
    is_demo: boolean
}

export interface IAuditableBallot {
    version: number
    issue_date: string
    config: IBallotStyle
    ballot_hash: string
    voter_signing_pk?: string
    voter_ballot_signature?: string
}
export interface IAuditableSingleBallot extends IAuditableBallot {
    contests: Array<string>
}
export interface IAuditableMultiBallot extends IAuditableBallot {
    contests: string
}

export interface IHashableBallot {
    version: number
    issue_date: string
    config: string
    voter_signing_pk?: string
    voter_ballot_signature?: string
}
export interface IHashableSingleBallot extends IHashableBallot {
    contests: Array<string>
}

export interface IHashableMultiBallot extends IHashableBallot {
    contests: string
}

export interface ISignedContent {
    public_key: string
    signature: string
}

export enum EInvalidPlaintextErrorType {
    Explicit = "Explicit",
    Implicit = "Implicit",
    EncodingError = "EncodingError",
}

export interface IVotingPortalCountdownPolicy {
    policy: EVotingPortalCountdownPolicy
    countdown_anticipation_secs: number
    countdown_alert_anticipation_secs: number
}

export enum EVotingPortalCountdownPolicy {
    NO_COUNTDOWN = "NO_COUNTDOWN",
    COUNTDOWN = "COUNTDOWN",
    COUNTDOWN_WITH_ALERT = "COUNTDOWN_WITH_ALERT",
}

export enum ETaskExecutionStatus {
    STARTED = "STARTED",
    IN_PROGRESS = "IN_PROGRESS",
    SUCCESS = "SUCCESS",
    FAILED = "FAILED",
    CANCELLED = "CANCELLED",
}

export interface ITaskExecuted {
    id: string
    created_at: string
    name: string
    execution_status: string
    start_at: string
    end_at: string | null
    tenant_id: string
    election_event_id: string
    executed_by_user: string
    annotations: object | null
    labels: object | null
    logs: object | null
    type: string
}

export enum EGraphQLInternalErrorMessage {
    TIMEOUT_ERROR = "Response timeout",
}

export enum EGraphQLErrorCode {
    UNEXPECTED = "unexpected",
}

export interface IExtensionErrorInternalError {
    message?: string | null
}

export interface IExtensionErrorInternal {
    error?: IExtensionErrorInternalError | null
}

export interface IExtensionError {
    code?: string | null
    internal?: IExtensionErrorInternal | null
}

export interface IGraphQLError {
    extensions?: IExtensionError | null
    message?: string | null
}

export interface IGraphQLActionError {
    message?: string | null
    name?: string | null
    graphQLErrors?: Array<IGraphQLError>
}
