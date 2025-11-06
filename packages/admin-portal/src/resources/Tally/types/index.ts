// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Candidate} from "@/gql/graphql"

export interface Sequent_Backend_Candidate_Extended extends Sequent_Backend_Candidate {
    rowId: number
    id: string
    status: string
    winning_position?: number | null
    cast_votes?: number | null
    cast_votes_percent: number | null
}

export interface ExtendedMetricsContest {
    over_votes: number
    under_votes: number
    votes_actually: number
    expected_votes: number
    total_ballots: number
    weight: number
}

export enum ECandidateStatus {
    Active = "Active",
    Eliminated = "Eliminated",
}

export interface CandidatesStatus {
    [candidateId: string]: ECandidateStatus
}

export type CandidatesWins = Record<string, number>

export interface Round {
    winner: string | null
    candidates_wins: CandidatesWins
    eliminated_candidates: string[] | null
    active_candidates_count: number
    active_ballots_count: number
}

export interface RunoffStatus {
    candidates_status: CandidatesStatus
    round_count: number
    rounds: Round[]
    max_rounds: number
}

export interface ParsedAnnotations {
    extended_metrics: ExtendedMetricsContest
    process_results?: RunoffStatus | unknown
}
