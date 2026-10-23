// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "@/gql/graphql"
import type {TallyDataSnapshot} from "@/atoms/tally-candidates"
import {ITallyCeremonyStatus, ITallyElectionStatus} from "@/types/ceremonies"

/** Numeric completeness belongs to one execution snapshot and one election.
 * Its SUCCESS progress covers every expected weight batch. The SQLite input
 * tables supply the selected area/contest scope, including zero-vote areas.
 * PDF documents and other elections do not determine numeric completeness.
 */
export function getCompleteElectionIds(
    tally: Pick<Sequent_Backend_Tally_Session, "id" | "election_ids" | "area_ids"> | undefined,
    execution:
        | Pick<
              Sequent_Backend_Tally_Session_Execution,
              "tally_session_id" | "results_event_id" | "status" | "documents"
          >
        | undefined,
    data: TallyDataSnapshot | null
): string[] {
    const resultsId = execution?.results_event_id
    if (
        !tally ||
        execution?.tally_session_id !== tally.id ||
        !resultsId ||
        !execution.documents?.sqlite ||
        data?.sqliteDocumentId !== execution.documents.sqlite ||
        !data?.sequent_backend_results_event.some((row) => row.id === resultsId)
    )
        return []

    const progress = (execution.status as ITallyCeremonyStatus | undefined)?.elections_status
    const areaResults = new Set(
        data.sequent_backend_results_area_contest
            .filter((row) => row.results_event_id === resultsId)
            .map((row) => `${row.election_id}:${row.contest_id}:${row.area_id}`)
    )
    const contestResults = new Set(
        data.sequent_backend_results_contest
            .filter((row) => row.results_event_id === resultsId)
            .map((row) => `${row.election_id}:${row.contest_id}`)
    )
    const selectedAreas = tally.area_ids ? new Set(tally.area_ids) : null
    return (tally.election_ids ?? []).filter((electionId) => {
        const electionProgress = progress?.find((row) => row.election_id === electionId)
        if (
            electionProgress?.status !== ITallyElectionStatus.SUCCESS ||
            electionProgress.progress !== 100
        )
            return false
        if (
            !data.sequent_backend_election.some((row) => row.id === electionId) ||
            !data.sequent_backend_results_election.some(
                (row) => row.election_id === electionId && row.results_event_id === resultsId
            )
        )
            return false

        const contests = data.sequent_backend_contest.filter(
            (row) => row.election_id === electionId
        )
        const contestIds = new Set(contests.map((row) => row.id))
        const expectedPairs = data.sequent_backend_area_contest.filter(
            (row) =>
                contestIds.has(row.contest_id) && (!selectedAreas || selectedAreas.has(row.area_id))
        )
        // A contest assigned only to excluded areas is not part of this run.
        return expectedPairs.every(
            (expected) =>
                contestResults.has(`${electionId}:${expected.contest_id}`) &&
                areaResults.has(`${electionId}:${expected.contest_id}:${expected.area_id}`)
        )
    })
}
