// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {isUndefined} from "@sequentech/ui-core"

export enum CastVoteStatus {
    IN_PROGRESS = "in-progress",
    VALID = "valid",
    DISCARDED = "discarded",
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
        // Remove the cast votes with the given ids from every per-
        // election bucket they happen to live in. Used by the
        // workbench's revote/overwrite path (a re-cast from the same
        // voter persona supersedes the prior cast vote rather than
        // stacking on top of it) and by snapshot-wipe operations.
        // Buckets that become empty are pruned so `Object.keys` /
        // `length` checks elsewhere don't see ghost entries.
        removeCastVotes: (
            state: CastVoteState,
            action: PayloadAction<Array<string>>
        ): CastVoteState => {
            const ids = new Set(action.payload)
            if (ids.size === 0) return state
            for (const electionId of Object.keys(state)) {
                const bucket = state[electionId] ?? []
                const kept = bucket.filter((cv) => !ids.has(cv.id))
                if (kept.length === bucket.length) continue
                if (kept.length === 0) {
                    delete state[electionId]
                } else {
                    state[electionId] = kept
                }
            }
            return state
        },
    },
})

export const {addCastVotes, removeCastVotes} = castVotesSlice.actions

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
