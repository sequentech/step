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
        tally.annotations?.is_post_task_completed === true &&
        execution?.tally_session_id === tally.id &&
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
 * Its SUCCESS progress covers every expected weight batch. The SQLite input
 * tables supply the selected area/contest scope, including zero-vote areas.
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
              "tally_session_id" | "results_event_id" | "status" | "documents"
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

    let publishedAreas: Record<string, string[]> | undefined
    if (tally.annotations?.election_areas !== undefined) {
        try {
            const parsed: unknown = JSON.parse(tally.annotations.election_areas)
            if (
                !parsed ||
                typeof parsed !== "object" ||
                Array.isArray(parsed) ||
                !Object.values(parsed).every(
                    (ids) => Array.isArray(ids) && ids.every((id) => typeof id === "string")
                )
            )
                return []
            publishedAreas = parsed as Record<string, string[]>
        } catch {
            return []
        }
    }
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
    // SQLite stores raw assignments, while the tally inherits contests from
    // ancestor areas. Expand the snapshot hierarchy before applying exclusions.
    const areas = data.sequent_backend_area as Array<{id: string; parent_id?: string | null}>
    const areaById = new Map(areas.map((area) => [area.id, area]))
    if (selectedAreas && [...selectedAreas].some((id) => !areaById.has(id))) return []
    const directContests = new Map<string, Set<string>>()
    for (const assignment of data.sequent_backend_area_contest) {
        const ids = directContests.get(assignment.area_id) ?? new Set<string>()
        ids.add(assignment.contest_id)
        directContests.set(assignment.area_id, ids)
    }
    const expectedPairs: Array<{area_id: string; contest_id: string}> = []
    for (const area of areas) {
        if (selectedAreas && !selectedAreas.has(area.id)) continue
        const inheritedContests = new Set<string>()
        const visited = new Set<string>()
        let ancestor: typeof area | undefined = area
        while (ancestor) {
            if (visited.has(ancestor.id)) return []
            visited.add(ancestor.id)
            directContests.get(ancestor.id)?.forEach((id) => inheritedContests.add(id))
            ancestor = ancestor.parent_id ? areaById.get(ancestor.parent_id) : undefined
        }
        inheritedContests.forEach((contest_id) =>
            expectedPairs.push({area_id: area.id, contest_id})
        )
    }
    return (tally.election_ids ?? []).filter((electionId) => {
        const electionProgress = progress?.find((row) => row.election_id === electionId)
        if (
            electionProgress?.status !== ITallyElectionStatus.SUCCESS ||
            electionProgress.progress !== 100
        )
            return false
        if (!data.sequent_backend_election.some((row) => row.id === electionId)) return false

        const contests = data.sequent_backend_contest.filter(
            (row) => row.election_id === electionId
        )
        const contestIds = new Set(contests.map((row) => row.id))
        // A contest assigned only to excluded areas is not part of this run.
        const electionPairs = expectedPairs.filter(
            (row) =>
                contestIds.has(row.contest_id) &&
                (!publishedAreas || publishedAreas[electionId]?.includes(row.area_id))
        )
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
            (expected) =>
                contestResults.has(`${electionId}:${expected.contest_id}`) &&
                areaResults.has(`${electionId}:${expected.contest_id}:${expected.area_id}`)
        )
    })
}
