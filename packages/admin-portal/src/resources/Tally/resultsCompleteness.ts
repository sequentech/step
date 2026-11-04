// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "@/gql/graphql"
import type {TallyDataSnapshot} from "@/atoms/tally-candidates"
import {ITallyCeremonyStatus, ITallyElectionStatus} from "@/types/ceremonies"

export function requiresCompleteResults(tally: {annotations?: unknown} | undefined): boolean {
    return (
        !!tally?.annotations &&
        Object.prototype.hasOwnProperty.call(tally.annotations, "result_readiness_version")
    )
}

export function isNumericallyComplete(tally: {
    is_execution_completed?: boolean
    execution_status?: string | null
    annotations?: unknown
}): boolean {
    return (
        !!tally.is_execution_completed &&
        (tally.execution_status === "SUCCESS" ||
            (requiresCompleteResults(tally) && tally.execution_status === "IN_PROGRESS"))
    )
}

export function hasCompletedEmptyResults(
    tally:
        | Pick<
              Sequent_Backend_Tally_Session,
              "id" | "election_ids" | "annotations" | "execution_status" | "is_execution_completed"
          >
        | undefined,
    execution:
        | Pick<
              Sequent_Backend_Tally_Session_Execution,
              "tally_session_id" | "results_event_id" | "status" | "documents"
          >
        | undefined
): boolean {
    const progress = (execution?.status as ITallyCeremonyStatus | undefined)?.elections_status
    return (
        !!tally &&
        requiresCompleteResults(tally) &&
        tally.execution_status === "SUCCESS" &&
        tally.is_execution_completed &&
        (tally.annotations?.is_post_task_completed === true ||
            tally.annotations?.is_post_task_completed === "true") &&
        !!execution &&
        execution.tally_session_id === tally.id &&
        !execution.results_event_id &&
        !execution.documents?.sqlite &&
        !!tally.election_ids?.length &&
        tally.election_ids.every((id) =>
            progress?.some(
                (row) =>
                    row.election_id === id &&
                    row.status === ITallyElectionStatus.SUCCESS &&
                    row.progress === 100
            )
        )
    )
}

/** Numeric completeness belongs to one execution snapshot and one election.
 * Its SUCCESS progress covers every expected weight batch. The backend
 * certificate supplies the frozen scope, including zero-vote areas.
 * PDF documents and other elections do not determine numeric completeness.
 */
export function getCompleteElectionIds(
    tally:
        | Pick<
              Sequent_Backend_Tally_Session,
              | "id"
              | "election_ids"
              | "area_ids"
              | "annotations"
              | "execution_status"
              | "is_execution_completed"
          >
        | undefined,
    execution:
        | Pick<
              Sequent_Backend_Tally_Session_Execution,
              "tally_session_id" | "results_event_id" | "status" | "documents" | "annotations"
          >
        | undefined,
    data: TallyDataSnapshot | null
): string[] {
    // The server marks new tallies at creation. Existing tallies retain their
    // previous result/report availability; asynchronous identity fixes still apply.
    if (tally && !requiresCompleteResults(tally)) return tally.election_ids ?? []
    if (hasCompletedEmptyResults(tally, execution)) return tally?.election_ids ?? []
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

    const readiness = execution.annotations?.result_readiness
    const expected: unknown = readiness?.expected_area_contests
    if (
        !Array.isArray(readiness?.complete_election_ids) ||
        !Array.isArray(expected) ||
        !expected.every(
            (pair) =>
                Array.isArray(pair) &&
                pair.length === 3 &&
                pair.every((id) => typeof id === "string")
        )
    )
        return []
    // The server certifies the frozen scope on this execution. Live SQLite
    // assignments may have changed since publication and cannot redefine it.
    const expectedPairs = expected as [string, string, string][]
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
    const areaIds = new Set(data.sequent_backend_area.map((area) => area.id))
    return (tally.election_ids ?? []).filter((electionId) => {
        if (!readiness.complete_election_ids.includes(electionId)) return false
        const electionProgress = progress?.find((row) => row.election_id === electionId)
        if (
            electionProgress?.status !== ITallyElectionStatus.SUCCESS ||
            electionProgress.progress !== 100
        )
            return false
        if (!data.sequent_backend_election.some((row) => row.id === electionId)) return false

        const electionPairs = expectedPairs.filter(([id]) => id === electionId)
        // The backend completes elections with no assigned areas without
        // emitting an election result. They must not block the other elections.
        if (electionPairs.length === 0) return true
        if (
            !data.sequent_backend_results_election.some(
                (row) => row.election_id === electionId && row.results_event_id === resultsId
            )
        )
            return false
        return electionPairs.every(
            ([, contestId, areaId]) =>
                areaIds.has(areaId) &&
                contestResults.has(`${electionId}:${contestId}`) &&
                areaResults.has(`${electionId}:${contestId}:${areaId}`)
        )
    })
}
