// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {isUndefined} from "@sequentech/ui-core"

export interface SessionBallotData {
    ballotId: string
    electionId: string
    ballot: string
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
        let ballotStyleElectionIds = Object.keys(state.ballotStyles)
        let elections = ballotStyleElectionIds
            .map((electionId) => state.elections[electionId])
            .filter((election) => !!election)

        return elections.some((election) => {
            let electionCastVotes = (election?.id && state.castVotes[election.id]) || []
            let numAllowedRevotes = election?.num_allowed_revotes ?? 1

            // If num_allowed_revotes is 0, allow voting
            if (numAllowedRevotes === 0) {
                return true
            }

            return electionCastVotes.length < numAllowedRevotes
        })
    }

export default castVotesSlice.reducer
