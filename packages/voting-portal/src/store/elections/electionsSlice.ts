// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IElection, sortElectionList} from "@sequentech/ui-core"

export interface IElectionExtended extends IElection {
    annotations?: string | null
    created_at?: string | null
    dates?: string | null
    eml?: string | null
    is_consolidated_ballot_encoding?: boolean | null
    labels?: string | null
    last_updated_at?: string | null
    num_allowed_revotes?: number | null
    spoil_ballot_option?: boolean | null
    status?: string | null
}

export interface ElectionState {
    [electionId: string]: IElectionExtended | undefined
}

const initialState: ElectionState = {}

export const electionsSlice = createSlice({
    name: "elections",
    initialState,
    reducers: {
        setElection: (
            state: ElectionState,
            action: PayloadAction<IElectionExtended>
        ): ElectionState => {
            state[action.payload.id] = action.payload
            return state
        },
    },
})

export const {setElection} = electionsSlice.actions

export const selectElectionIds = (state: RootState) => {
    return sortElectionList(
        Object.values(state.elections) as unknown as IElection[],
        Object.values(state.electionEvent)[0]?.presentation?.elections_order
    ).map((election) => election.id)
}

export const selectElectionById = (electionId: string) => (state: RootState) =>
    state.elections[electionId]

export default electionsSlice.reducer
