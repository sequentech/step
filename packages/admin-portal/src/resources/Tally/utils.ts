// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ParsedAnnotations, RunoffStatus} from "./types"
import {Sequent_Backend_Candidate, Sequent_Backend_Contest} from "@/gql/graphql"
import {ICandidate, IContest, ICountingAlgorithm} from "@sequentech/ui-core"
import {ITallyExecutionStatus, ITallyTrusteeStatus} from "@/types/ceremonies"

export const canTrusteeRestorePrivateKey = (
    trusteeStatus: ITallyTrusteeStatus | null,
    tallyExecutionStatus: string | null | undefined
): boolean =>
    trusteeStatus === ITallyTrusteeStatus.WAITING &&
    (tallyExecutionStatus === ITallyExecutionStatus.STARTED ||
        tallyExecutionStatus === ITallyExecutionStatus.CONNECTED)

/**
 * Safely extracts the value from a GraphQL 'Maybe<T>' type.
 * Returns undefined if the value is null or undefined.
 */
const safeExtract = <T>(maybeValue: T | null | undefined): T | undefined => {
    return maybeValue === null || maybeValue === undefined ? undefined : maybeValue
}

export const orderItemsByIds = <T extends {id: string}>(
    items: readonly T[],
    orderedIds: readonly string[]
): T[] => {
    const itemById = new Map(items.map((item) => [item.id, item]))
    const seenIds = new Set<string>()

    return orderedIds.reduce<T[]>((orderedItems, id) => {
        if (seenIds.has(id)) return orderedItems
        seenIds.add(id)

        const item = itemById.get(id)
        if (item) orderedItems.push(item)
        return orderedItems
    }, [])
}

const convertSequentCandidateToICandidate = (
    backendCandidate: Sequent_Backend_Candidate
): ICandidate => {
    // Implement mapping logic here if will be neccecery
    return backendCandidate as ICandidate
}

/**
 * Converts a single Sequent_Backend_Contest object to an IContest object.
 * @param contest The contest object from the backend/GraphQL.
 * @returns The converted IContest object.
 */
export function convertSequentContestToIContest(contest: Sequent_Backend_Contest): IContest {
    const convertedCandidates: Array<ICandidate> = contest.candidates.map(
        convertSequentCandidateToICandidate
    )

    const backendIsEncryptedValue = safeExtract(contest.is_encrypted)

    const isEncrypted: boolean = backendIsEncryptedValue ? true : false

    return {
        id: contest.id,
        tenant_id: contest.tenant_id,
        election_event_id: contest.election_event_id,
        election_id: contest.election_id,
        max_votes: safeExtract(contest.max_votes) ?? 0,
        min_votes: safeExtract(contest.min_votes) ?? 0,
        winning_candidates_num: safeExtract(contest.winning_candidates_num) ?? 0,
        is_encrypted: isEncrypted,
        description: safeExtract(contest.description),
        voting_type: safeExtract(contest.voting_type),
        counting_algorithm: safeExtract(contest.counting_algorithm) as ICountingAlgorithm,
        created_at: safeExtract(contest.created_at),
        candidates: convertedCandidates,
        presentation: contest.presentation ? JSON.parse(contest.presentation) : undefined,
    }
}

/**
 * Maps an array of Sequent_Backend_Contest to an array of IContest
 * @param contests Array of backend contest objects.
 * @returns Array of IContest objects.
 */
export function convertContestsArray(contests: Array<Sequent_Backend_Contest>): Array<IContest> {
    return contests.map(convertSequentContestToIContest)
}

export const parseResultAnnotations = (annotations: unknown): ParsedAnnotations | null => {
    if (!annotations) return null

    try {
        const parsed = typeof annotations === "string" ? JSON.parse(annotations) : annotations
        return typeof parsed === "object" && parsed !== null ? (parsed as ParsedAnnotations) : null
    } catch {
        return null
    }
}

/**
 * Parses and processes contest results based on counting algorithm.
 * Handles IRV/Runoff voting and other algorithms.
 *
 * @param annotations - Raw annotations string from general results
 * @param counting_algorithm - The counting algorithm used for this contest
 * @returns Parsed process results or null if parsing fails or no results available
 */
export const parseProcessResults = (
    annotations: string | null | undefined,
    counting_algorithm: ICountingAlgorithm
): RunoffStatus | unknown | null => {
    try {
        const parsedAnnotations = parseResultAnnotations(annotations)

        const results = parsedAnnotations?.process_results ?? null

        if (results && counting_algorithm) {
            switch (counting_algorithm) {
                case ICountingAlgorithm.INSTANT_RUNOFF: {
                    const runoffResults = results as RunoffStatus
                    return runoffResults
                }
                default:
                    console.log("Unknown counting algorithm process_results:", results)
                    return results
            }
        }

        return null
    } catch (error) {
        console.error("Error parsing process_results:", error)
        return null
    }
}
