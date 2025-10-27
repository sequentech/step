// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Candidate, Sequent_Backend_Contest} from "@/gql/graphql"
import {ICandidate, IContest} from "@sequentech/ui-core"

/**
 * Safely extracts the value from a GraphQL 'Maybe<T>' type.
 * Returns undefined if the value is null or undefined.
 */
const safeExtract = <T>(maybeValue: T | null | undefined): T | undefined => {
    return maybeValue === null || maybeValue === undefined ? undefined : maybeValue
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
        name: safeExtract(contest.name),
        description: safeExtract(contest.description),
        alias: safeExtract(contest.alias),
        voting_type: safeExtract(contest.voting_type),
        counting_algorithm: safeExtract(contest.counting_algorithm),
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
