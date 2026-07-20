// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * MOCK backend for the "Indeterminate Ballot Resolution" view
 * (DatafixPossibleImplementation.md). Replace with:
 *  - a GraphQL query over sequent_backend_cast_vote (status = "indeterminate")
 *    joined by ballot_id against the electoral log (see the Implementation
 *    Requirements note about exposing ballot_id on that query), and
 *  - a mutation that writes cast_vote.status directly (no VoterView call)
 *    and appends the electoral log entry for the change.
 */

import {ECastVoteResolution, IndeterminateCastVote} from "./types"

const delay = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))

const hoursAgo = (hours: number): string =>
    new Date(Date.now() - hours * 60 * 60 * 1000).toISOString()

// MOCK seed data, standing in for the join query described above.
const seedIndeterminateVotes = (): IndeterminateCastVote[] => [
    {
        id: "mock-cv-1",
        ballotId: "ballot-8f21e4c0",
        voterIdString: "17695",
        createdAt: hoursAgo(20),
        lastUpdatedAt: hoursAgo(3),
        electoralLogEntries: [
            {
                id: "mock-log-1a",
                statementKind: "CastVote",
                description: "Cast vote received and stored.",
                createdAt: hoursAgo(20),
            },
            {
                id: "mock-log-1b",
                statementKind: "CastVoteError",
                description:
                    "SetVoted dispatch to VoterView timed out after the request was accepted - outcome unconfirmed.",
                createdAt: hoursAgo(3),
            },
        ],
    },
    {
        id: "mock-cv-2",
        ballotId: "ballot-3a77b912",
        voterIdString: "79535",
        createdAt: hoursAgo(15),
        lastUpdatedAt: hoursAgo(2),
        electoralLogEntries: [
            {
                id: "mock-log-2a",
                statementKind: "CastVote",
                description: "Cast vote received and stored.",
                createdAt: hoursAgo(15),
            },
            {
                id: "mock-log-2b",
                statementKind: "CastVoteError",
                description:
                    "SetVoted dispatch received a gateway error from VoterView - outcome unconfirmed.",
                createdAt: hoursAgo(2),
            },
        ],
    },
    {
        id: "mock-cv-3",
        ballotId: "ballot-c410f2a5",
        voterIdString: "68684",
        createdAt: hoursAgo(9),
        lastUpdatedAt: hoursAgo(1),
        electoralLogEntries: [
            {
                id: "mock-log-3a",
                statementKind: "CastVote",
                description: "Cast vote received and stored.",
                createdAt: hoursAgo(9),
            },
        ],
    },
]

export const mockFetchIndeterminateCastVotes = async (): Promise<IndeterminateCastVote[]> => {
    await delay(500)
    return seedIndeterminateVotes()
}

// MOCK: writes cast_vote.status directly in Postgres. No SetVoted/SetNotVoted
// dispatch to VoterView, no diff/patch involved - see "Indeterminate Ballot
// Resolution" in DatafixPossibleImplementation.md.
export const mockResolveCastVote = async (
    voteId: string,
    resolution: ECastVoteResolution
): Promise<void> => {
    await delay(500)
}
