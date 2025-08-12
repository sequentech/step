// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
