// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {isUndefined} from "@sequentech/ui-essentials"

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

export const hasVotedAllElections = (currentElectionId: string) => (state: RootState) => {
    let castVoteElectionIds = Object.keys(state.castVotes).filter(
        (electionId) => electionId !== currentElectionId && state.castVotes[electionId]
    )
    castVoteElectionIds.push(currentElectionId)

    let ballotStyleElectionIds = Object.keys(state.ballotStyles)

    let missingElectionId = ballotStyleElectionIds.find(
        (ballotStyleElectionId) => !castVoteElectionIds.includes(ballotStyleElectionId)
    )
    return isUndefined(missingElectionId)
}

export default castVotesSlice.reducer
