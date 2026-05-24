// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//
// Velvet `ContestResult` JSON  →  ui-essentials `TallyResultsViewModel`.
//
// Velvet's serialised shape lives in packages/workbench/velvet-core/src/result.rs.
// admin-portal expects a view-model derived from its Sequent_Backend_*
// graphql types; ui-essentials' TallyResultsViewModel uses a stripped-down
// plain-shape version of that. This adapter is the workbench-specific glue
// that maps between the two.
//
// See packages/workbench/LIFTING-TALLY.md section B for the field-rename
// table and the canary signals if velvet's output shape evolves.

import type {
    RunoffStatus,
    TallyCandidate,
    TallyParticipationSummary,
    TallyResultsViewModel,
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

/** Best-effort mapping from a velvet `ContestResult` JSON into the
 *  ui-essentials `TallyResultsViewModel`. Returns null when the input
 *  isn't an object (defensive — callers may pass `unknown`). */
export function adaptVelvetContestResult(
    result: unknown,
    contestName?: string
): TallyResultsViewModel | null {
    if (!result || typeof result !== "object") return null
    const r = result as VelvetContestResult

    const candidates: TallyCandidate[] = (r.candidate_result ?? [])
        .map((cr, i) => ({
            rowId: i,
            id: cr.candidate.id,
            status: "active",
            name: cr.candidate.name ?? cr.candidate.id,
            cast_votes: cr.total_count ?? 0,
            // velvet emits percentage_votes in [0,100]; admin-portal's
            // formatPercentOne expects a [0,1] fraction (multiplies by 100).
            cast_votes_percent: (cr.percentage_votes ?? 0) / 100,
            winning_position: null as number | null,
        }))
        .sort((a, b) => (b.cast_votes ?? 0) - (a.cast_votes ?? 0))
        .map((c, i) => ({...c, rowId: i, winning_position: i + 1}))

    const summary: TallyParticipationSummary = {
        id: r.contest?.id ?? "",
        elegible_census: r.census ?? 0,
        total_valid_votes: r.total_valid_votes ?? 0,
        total_invalid_votes: r.total_invalid_votes ?? 0,
        blank_votes: r.total_blank_votes ?? 0,
    }

    return {
        summary,
        candidates,
        winnersCount: r.contest?.winning_candidates_num ?? 1,
        runoff: adaptRunoff(r.process_results),
        countingAlgorithm: r.contest?.counting_algorithm ?? undefined,
        contestName,
    }
}

function adaptRunoff(processResults: unknown): RunoffStatus | null {
    if (!processResults || typeof processResults !== "object") return null
    const pr = processResults as VelvetRunoff
    if (!Array.isArray(pr.rounds) || pr.rounds.length === 0) return null
    return {
        candidates_status: (pr.candidates_status ?? {}) as RunoffStatus["candidates_status"],
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
