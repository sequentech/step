// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"

export function canVoteElection(
    election: {id: string; num_allowed_revotes?: number | null},
    votes: readonly {status?: string | null}[],
    completedAcclaimed = false
): boolean {
    if (completedAcclaimed) return false
    const limit = election.num_allowed_revotes ?? 1
    return (
        limit === 0 ||
        votes.filter((vote) => vote.status !== CastVoteStatus.DISCARDED).length < limit
    )
}

export enum CastVoteStatus {
    IN_PROGRESS = "in-progress",
    VALID = "valid",
    DISCARDED = "discarded",
}

/** Validate the database string before storing it as a voting status. */
export function parseCastVoteStatus(status: string): CastVoteStatus {
    switch (status) {
        case CastVoteStatus.IN_PROGRESS:
        case CastVoteStatus.VALID:
        case CastVoteStatus.DISCARDED:
            return status
        default:
            throw new Error("Unknown cast vote status")
    }
}

export interface ICastVote {
    id: string
    tenant_id: string
    election_id?: string | null
    area_id?: string | null
    created_at?: string | null
    last_updated_at?: string | null
    annotations?: string | null
    labels?: string | null
    content?: string | null
    cast_ballot_signature?: string | null
    voter_id_string?: string | null
    election_event_id: string
    status?: CastVoteStatus | null
}

export interface CastVoteState {
    [electionId: string]: Array<ICastVote>
}

const initialState: CastVoteState = {}

export const castVotesSlice = createSlice({
    name: "castVotes",
    initialState,
    reducers: {
        addCastVotes: (
            state: CastVoteState,
            action: PayloadAction<Array<ICastVote>>
        ): CastVoteState => {
            for (let castVote of action.payload) {
                if (!castVote.election_id) {
                    continue
                }

                if (castVote.status === CastVoteStatus.DISCARDED) {
                    state[castVote.election_id] = (state[castVote.election_id] || []).filter(
                        (cv) => castVote.id !== cv.id
                    )
                    continue
                }

                state[castVote.election_id] = [
                    ...(state[castVote.election_id] || []).filter((cv) => castVote.id !== cv.id),
                    castVote,
                ]
            }
            return state
        },
    },
})

export const {addCastVotes} = castVotesSlice.actions

export const selectCastVotesByElectionId = (electionId: string) => (state: RootState) =>
    state.castVotes[electionId] || []

export const canVoteSomeElection =
    () =>
    (state: RootState): boolean => {
        return Object.values(state.elections).some(
            (election) =>
                !!election &&
                canVoteElection(
                    election,
                    state.castVotes[election.id] || [],
                    state.extra.completedAcclaimedElections?.[election.id]
                )
        )
    }

export default castVotesSlice.reducer
