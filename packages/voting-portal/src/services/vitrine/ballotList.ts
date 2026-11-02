// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the ballot list tells a voter who cannot vote again in an election
// (design board BallotList): the vote button of a ballot already cast says so.

import {canVoteElection, CastVoteStatus} from "../../store/castVotes/castVotesSlice"

export interface IAlreadyCastInput {
    election: {id: string; num_allowed_revotes?: number | null}
    /** The ballots the voter cast in the election. */
    votes: readonly {status?: string | null}[]
    /** Whether the election is open for voting. */
    isOpen: boolean
    /** Whether the voter can start a ballot in it. */
    canVote: boolean
    /** Whether the event keeps a transparency log. */
    keepsTransparencyLog: boolean
}

/** Whether the vote button of an election's card, which is shown but off, says
 * that the voter already cast their ballot: only when it is off because they
 * cast every ballot the election allows them. */
export const showsAlreadyCastHint = ({
    election,
    votes,
    isOpen,
    canVote,
    keepsTransparencyLog,
}: IAlreadyCastInput): boolean =>
    keepsTransparencyLog &&
    isOpen &&
    !canVote &&
    votes.some((vote) => vote.status !== CastVoteStatus.DISCARDED) &&
    !canVoteElection(election, votes)
