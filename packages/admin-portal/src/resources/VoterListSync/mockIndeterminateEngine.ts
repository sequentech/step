// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * MOCK backend for the "Indeterminate Ballot Resolution" view
 * (DatafixPossibleImplementation.md). Replace with:
 *  - a GraphQL query over sequent_backend_cast_vote (status = "indeterminate")
 *    joined against the user record for userId/voterIdString/enabled/voted
 *    (the electoral log entries shown in the Review drawer are already real -
 *    see ElectoralLogList usage in IndeterminateVotesTab.tsx), and
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
        userId: "a3f1c2e4-1b2a-4c3d-9e8f-1234567890ab",
        voterIdString: "17695",
        enabled: true,
        // SetVoted never confirmed, so VoterView may still show this voter as
        // not having voted despite the ballot sitting in cast_vote.
        voted: false,
        ballotId: "51c6364f50d9ae77d684f4fe557d571ff08c85ba33fa6c2e2dc4606a14c15b97",
        createdAt: hoursAgo(20),
        lastUpdatedAt: hoursAgo(3),
    },
    {
        id: "mock-cv-2",
        userId: "b7d2e5f6-2c3b-4d4e-8f9a-2345678901bc",
        voterIdString: "79535",
        enabled: true,
        voted: true,
        ballotId: "46046a7da3451520f72acd48dd41624d696686cc64217f074c1bccc5d20a07b6",
        createdAt: hoursAgo(15),
        lastUpdatedAt: hoursAgo(2),
    },
    {
        id: "mock-cv-3",
        userId: "c8e3f6a7-3d4c-4e5f-9a0b-3456789012cd",
        voterIdString: "68684",
        enabled: false,
        voted: false,
        ballotId: "b4df381c3172728880376d3f4bc5bdded91fe1bf47cc54e232ca4aba9d82a95c",
        createdAt: hoursAgo(9),
        lastUpdatedAt: hoursAgo(1),
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
