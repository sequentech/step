// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {sortElectionList} from "@sequentech/ui-essentials"

export interface IElection {
    id: string
    annotations?: string | null
    created_at?: string | null
    dates?: string | null
    description?: string | null
    election_event_id: string
    eml?: string | null
    is_consolidated_ballot_encoding?: boolean | null
    labels?: string | null
    last_updated_at?: string | null
    name?: string | null
    alias?: string | null
    num_allowed_revotes?: number | null
    presentation?: string | null
    spoil_ballot_option?: boolean | null
    status?: string | null
    tenant_id: string
}

export interface ElectionState {
    [electionId: string]: IElection | undefined
}

const initialState: ElectionState = {}

export const electionsSlice = createSlice({
    name: "elections",
    initialState,
    reducers: {
        setElection: (state: ElectionState, action: PayloadAction<IElection>): ElectionState => {
            state[action.payload.id] = action.payload
            return state
        },
    },
})

export const {setElection} = electionsSlice.actions

export const selectElectionIds = (state: RootState) => {
    return sortElectionList(
        Object.values(state.elections) as any,
        Object.values(state.electionEvent)[0]?.presentation?.elections_order
    ).map((election) => election.id)
}

export const selectElectionById = (electionId: string) => (state: RootState) =>
    state.elections[electionId]

export default electionsSlice.reducer
