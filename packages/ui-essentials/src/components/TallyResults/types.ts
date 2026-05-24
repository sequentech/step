// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//
// Lifted from admin-portal/src/resources/Tally/types/index.ts.
// Adaptations:
//   T1: drop the Sequent_Backend_Candidate_Extended subset (admin-portal-only
//       graphql type); replace with plain TallyCandidate row interface.
//   T2: drop ExtendedMetricsContest / ParsedAnnotations (admin-only).
// Everything else (RunoffStatus, Round, CandidateReference, CandidateOutcome,
// CandidatesStatus, ECandidateStatus) is copied verbatim so callers can rely
// on the same shape admin-portal does.

export interface TallyCandidate {
    rowId: number
    id: string
    status: string
    winning_position?: number | null
    cast_votes?: number | null
    cast_votes_percent: number | null
    name: string
}

export interface CandidateReference {
    id: string
    name: string
}

export enum ECandidateStatus {
    Active = "Active",
    Eliminated = "Eliminated",
}

export interface CandidatesStatus {
    [candidateId: string]: ECandidateStatus
}

export interface CandidateOutcome {
    name: string
    wins: number
    transference: number
    percentage: number
}

export type CandidatesOutcomes = Record<string, CandidateOutcome>

export interface Round {
    winner: CandidateReference | null
    candidates_wins: CandidatesOutcomes
    eliminated_candidates: CandidateReference[] | null
    active_candidates_count: number
    active_ballots_count: number
    exhausted_ballots_count: number
}

export interface RunoffStatus {
    candidates_status: CandidatesStatus
    name_references: CandidateReference[]
    round_count: number
    rounds: Round[]
    max_rounds: number
}

/** Plain shape consumed by ParticipationSummaryChart and TallyResultsView. */
export interface TallyParticipationSummary {
    id: string
    elegible_census: number
    total_valid_votes: number
    total_invalid_votes: number
    blank_votes: number
}

/** Top-level view-model used by TallyResultsView. */
export interface TallyResultsViewModel {
    summary: TallyParticipationSummary
    candidates: TallyCandidate[]
    winnersCount: number
    runoff: RunoffStatus | null
    countingAlgorithm?: string
    contestName?: string
}
