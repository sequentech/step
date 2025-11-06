// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"

export interface IBallotStyle {
    id: string
    election_id: string
    election_event_id: string
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

export const ballotStylesSlice = createSlice({
    name: "ballotStyles",
    initialState,
    reducers: {
        setBallotStyle: (
            state: BallotStylesState,
            action: PayloadAction<IBallotStyle>
        ): BallotStylesState => {
            state[action.payload.election_id] = action.payload
            return state
        },
    },
})

export const {setBallotStyle} = ballotStylesSlice.actions

export const selectBallotStyleByElectionId = (electionId: string) => (state: RootState) =>
    state.ballotStyles[electionId]

export const selectBallotStyleElectionIds = (state: RootState) => Object.keys(state.ballotStyles)

export const selectFirstBallotStyle = (state: RootState): IBallotStyle | undefined =>
    Object.values(state.ballotStyles)?.[0]

export default ballotStylesSlice.reducer
