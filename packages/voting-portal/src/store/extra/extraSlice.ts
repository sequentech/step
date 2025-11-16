// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"

export interface ElectionVoteStepState {
    [electionId: string]: boolean
}
export interface ExtraState {
    bypassChooser: boolean
    isVoted: ElectionVoteStepState
}

const initialState: ExtraState = {
    bypassChooser: false,
    isVoted: {},
}

export const extraSlice = createSlice({
    name: "extra",
    initialState,
    reducers: {
        setBypassChooser: (state: ExtraState, action: PayloadAction<boolean>): ExtraState => {
            state.bypassChooser = action.payload
            return state
        },
        setIsVoted: (state: ExtraState, action: PayloadAction<any>): ExtraState => {
            state.isVoted[action.payload] = true
            return state
        },
        clearIsVoted: (state: ExtraState): ExtraState => {
            state.isVoted = {}
            return state
        },
    },
})

export const {setBypassChooser, setIsVoted, clearIsVoted} = extraSlice.actions

export const selectBypassChooser = () => (state: RootState) => state.extra.bypassChooser

export const isVotedByElectionId = (electionId: string | undefined) => (state: RootState) => {
    return electionId ? state.extra.isVoted[electionId] : false
}

export default extraSlice.reducer
