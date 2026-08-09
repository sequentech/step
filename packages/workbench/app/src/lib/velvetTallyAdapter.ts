// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//
// Velvet `ContestResult` JSON  →  props for ui-essentials'
// `ResultsAndParticipation`.
//
// Velvet's serialised shape lives in packages/workbench/velvet-core/src/result.rs.
// ui-essentials owns the production tally visualization (shared with
// results-portal); this adapter is the workbench-specific glue that maps
// velvet's snake_case result onto that component's camelCase props.
//
// See packages/workbench/LIFTING-TALLY.md for the field-rename table and
// the canary signals if velvet's output shape evolves.

import type {
    CandidateResultRow,
    PreferentialProcessResults,
    ResultsParticipationSummary,
} from "@sequentech/ui-essentials"

interface VelvetCandidate {
    id: string
    name?: string | null
}

interface VelvetCandidateResult {
    candidate: VelvetCandidate
    total_count: number
    percentage_votes: number
}

interface VelvetContestResult {
    contest: {
        id: string
        winning_candidates_num?: number | null
        counting_algorithm?: string | null
    }
    census: number
    total_valid_votes: number
    total_invalid_votes: number
    total_blank_votes: number
    candidate_result: VelvetCandidateResult[]
    process_results?: unknown
}

interface VelvetRoundCandidateWin {
    name?: string
    wins?: number
    percentage?: number
    transference?: number
}

interface VelvetRound {
    winner?: {id: string; name?: string} | null
    eliminated_candidates?: {id: string; name?: string}[] | null
    candidates_wins?: Record<string, VelvetRoundCandidateWin>
    active_candidates_count?: number
    active_ballots_count?: number
    exhausted_ballots_count?: number
}

interface VelvetRunoff {
    candidates_status?: Record<string, "Active" | "Eliminated">
    name_references?: {id: string; name?: string}[]
    round_count?: number
    rounds?: VelvetRound[]
    max_rounds?: number
}

/** Everything the workbench's tally visualization needs. The first five
 *  fields map 1:1 onto `ResultsAndParticipation`'s props; the last two
 *  have no counterpart on that component and are rendered by the
 *  workbench itself (TallyPage). */
export interface VelvetTallyView {
    chartName: string
    summary: ResultsParticipationSummary
    candidates: CandidateResultRow[]
    processResults: PreferentialProcessResults | null
    winnersCount: number
    countingAlgorithm?: string
}

/** Fraction in [0,1] of `part` over `whole`, or undefined when the base
 *  is zero/absent. ui-essentials' `percentOrDash` runs the value through
 *  `formatPercentOne`, which multiplies by 100 — so these must be
 *  fractions, not percentages. */
function fraction(part: number, whole: number): number | undefined {
    return whole > 0 ? part / whole : undefined
}

/** Best-effort mapping from a velvet `ContestResult` JSON into the props
 *  ui-essentials' tally components expect. Returns null when the input
 *  isn't an object (defensive — callers may pass `unknown`). */
export function adaptVelvetContestResult(
    result: unknown,
    contestName?: string
): VelvetTallyView | null {
    if (!result || typeof result !== "object") return null
    const r = result as VelvetContestResult

    const candidates: CandidateResultRow[] = (r.candidate_result ?? [])
        .map((cr) => ({
            id: cr.candidate.id,
            name: cr.candidate.name ?? cr.candidate.id,
            castVotes: cr.total_count ?? 0,
            // velvet emits percentage_votes in [0,100]; formatPercentOne
            // expects a [0,1] fraction (it multiplies by 100).
            castVotesPercent: (cr.percentage_votes ?? 0) / 100,
        }))
        .sort((a, b) => (b.castVotes ?? 0) - (a.castVotes ?? 0))
        // velvet does not rank candidates; the component consumes
        // winningPosition but never computes it, so assign it here.
        .map((c, i) => ({...c, winningPosition: i + 1}))

    const census = r.census ?? 0
    const valid = r.total_valid_votes ?? 0
    const invalid = r.total_invalid_votes ?? 0
    const blank = r.total_blank_votes ?? 0
    const counted = valid + invalid

    const summary: ResultsParticipationSummary = {
        id: r.contest?.id ?? "",
        eligibleCensus: census,
        totalVotes: counted,
        totalVotesPercent: fraction(counted, census),
        totalValidVotes: valid,
        totalValidVotesPercent: fraction(valid, census),
        totalInvalidVotes: invalid,
        totalInvalidVotesPercent: fraction(invalid, census),
        blankVotes: blank,
        blankVotesPercent: fraction(blank, census),
    }

    return {
        chartName: contestName ?? r.contest?.id ?? "",
        summary,
        candidates,
        processResults: adaptRunoff(r.process_results),
        winnersCount: r.contest?.winning_candidates_num ?? 1,
        countingAlgorithm: r.contest?.counting_algorithm ?? undefined,
    }
}

function adaptRunoff(processResults: unknown): PreferentialProcessResults | null {
    if (!processResults || typeof processResults !== "object") return null
    const pr = processResults as VelvetRunoff
    if (!Array.isArray(pr.rounds) || pr.rounds.length === 0) return null
    return {
        candidates_status: pr.candidates_status ?? {},
        name_references: (pr.name_references ?? []).map((c) => ({
            id: c.id,
            name: c.name ?? c.id,
        })),
        round_count: pr.round_count ?? pr.rounds.length,
        max_rounds: pr.max_rounds ?? pr.rounds.length,
        rounds: pr.rounds.map((rd) => ({
            winner: rd.winner ? {id: rd.winner.id, name: rd.winner.name ?? rd.winner.id} : null,
            eliminated_candidates: (rd.eliminated_candidates ?? []).map((c) => ({
                id: c.id,
                name: c.name ?? c.id,
            })),
            candidates_wins: Object.fromEntries(
                Object.entries(rd.candidates_wins ?? {}).map(([k, v]) => [
                    k,
                    {
                        name: v.name ?? k,
                        wins: v.wins ?? 0,
                        percentage: v.percentage ?? 0,
                        transference: v.transference ?? 0,
                    },
                ])
            ),
            active_candidates_count: rd.active_candidates_count ?? 0,
            active_ballots_count: rd.active_ballots_count ?? 0,
            exhausted_ballots_count: rd.exhausted_ballots_count ?? 0,
        })),
    }
}
