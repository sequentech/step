// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum EVotingChannel {
    PAPER = "PAPER",
    POSTAL = "POSTAL",
    IN_PERSON = "IN_PERSON",
}

export enum EStatus {
    PENDING = "PENDING",
    APPROVED = "APPROVED",
    DISAPPROVED = "DISAPPROVED",
}

export enum ETallySheetImportSourceFormat {
    ESS_ENHANCED_XML = "ESS_ENHANCED_XML",
    CANONICAL_CSV = "CANONICAL_CSV",
}

export enum ETallySheetImportChangeType {
    NEW = "NEW",
    CHANGED = "CHANGED",
    UNCHANGED = "UNCHANGED",
}

export enum ETallySheetImportStatus {
    PENDING_REVIEW = "PENDING_REVIEW",
    APPROVED = "APPROVED",
    DISAPPROVED = "DISAPPROVED",
    FAILED_VALIDATION = "FAILED_VALIDATION",
    CONFLICTED = "CONFLICTED",
}

export enum ETallySheetImportItemStatus {
    PENDING_REVIEW = "PENDING_REVIEW",
    APPROVED = "APPROVED",
    DISAPPROVED = "DISAPPROVED",
    CONFLICTED = "CONFLICTED",
}

export enum ETallySheetImportReviewDecision {
    APPROVE = "APPROVE",
    DISAPPROVE = "DISAPPROVE",
}

export interface IInvalidVotes {
    total_invalid?: number
    implicit_invalid?: number
    explicit_invalid?: number
}

export interface ICandidateResults {
    candidate_id: string
    total_votes?: number
}

export interface IAreaContestResults {
    area_id: string
    contest_id: string
    total_votes?: number
    total_valid_votes?: number
    invalid_votes?: IInvalidVotes
    total_blank_votes?: number
    census?: number
    candidate_results: {[id: string]: ICandidateResults}
}
