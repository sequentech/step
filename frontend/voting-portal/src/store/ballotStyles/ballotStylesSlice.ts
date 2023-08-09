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
    status?: string | null
    tenant_id: string
    ballot_eml: IElectionDTO
    ballot_signature?: string | null
    created_at: string
    area_id?: string | null
    annotations?: string | null
    labels?: string | null
    last_updated_at: string
}

export interface BallotStylesState {
    [electionId: string]: IBallotStyle | undefined
}

const initialState: BallotStylesState = {}

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

export const ballotStylesSlice = createSlice({
    name: "ballotStyles",
    initialState,
    reducers: {
        setBallotStyle: (
            state: BallotStylesState,
            action: PayloadAction<IBallotStyle>
        ): BallotStylesState => {
            state[action.payload.id] = action.payload
            return state
        },
    },
})

export const {setBallotStyle} = ballotStylesSlice.actions

export const selectBallotStyleByElectionId = (electionId: string) => (state: RootState) =>
    state.ballotStyles[electionId]

export const selectAllElectionIds = (state: RootState) =>
    Object.keys(state.ballotStyles).filter(electionId => !isUndefined(state.ballotStyles[electionId]))

export default ballotStylesSlice.reducer
