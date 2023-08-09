// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createAsyncThunk, createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
//import {fetchElection} from "./electionsAPI"
import {IElectionDTO} from "sequent-core"
import { isUndefined } from "@sequentech/ui-essentials"

export interface IBallotStyle {
    id: string
    election_id: string
    election_event_id: string
    status?: string
    tenant_id: string
    ballot_eml: IElectionDTO
    ballot_signature?: string
    created_at?: string
    area_id?: string
    annotations?: string
    labels?: string
    last_updated_at?: string
}

export interface ElectionsState {
    [id: string]: IBallotStyle | undefined
}

const initialState: ElectionsState = {}

/*
export const fetchElectionByIdAsync = createAsyncThunk(
    "elections/fetchElectionByIdAsync",
    async (electionId: number) => {
        console.log("trying fetchElectionByIdAsync")
        const response = await fetchElection(electionId)
        // The value we return becomes the `fulfilled` action payload
        return response
    }
)
*/

export const electionsSlice = createSlice({
    name: "elections",
    initialState,
    reducers: {
        setElection: (
            state: ElectionsState,
            action: PayloadAction<IBallotStyle>
        ): ElectionsState => {
            state[action.payload.id] = action.payload
            return state
        },
    },
})

export const {setElection} = electionsSlice.actions

export const selectElectionById = (electionId: string) => (state: RootState) =>
    state.elections[electionId]

export const selectAllElectionIds = (state: RootState) =>
    Object.keys(state.elections).filter(electionId => !isUndefined(state.elections[electionId]))

export default electionsSlice.reducer
