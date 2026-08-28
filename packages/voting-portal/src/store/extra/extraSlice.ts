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
    declinedToVote: ElectionVoteStepState
    completedAcclaimedElections: ElectionVoteStepState
}

const initialState: ExtraState = {
    bypassChooser: false,
    isVoted: {},
    declinedToVote: {},
    completedAcclaimedElections: {},
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
        setDeclinedToVote: (state: ExtraState, action: PayloadAction<string>): ExtraState => {
            state.declinedToVote[action.payload] = true
            return state
        },
        clearDeclinedToVoteForElection: (
            state: ExtraState,
            action: PayloadAction<string>
        ): ExtraState => {
            delete state.declinedToVote[action.payload]
            return state
        },
        completeAcclaimedElection: (
            state: ExtraState,
            action: PayloadAction<string>
        ): ExtraState => {
            state.completedAcclaimedElections[action.payload] = true
            return state
        },
        clearIsVoted: (state: ExtraState): ExtraState => {
            state.isVoted = {}
            state.declinedToVote = {}
            return state
        },
    },
})

export const {
    setBypassChooser,
    setIsVoted,
    setDeclinedToVote,
    clearDeclinedToVoteForElection,
    completeAcclaimedElection,
    clearIsVoted,
} = extraSlice.actions

export const selectBypassChooser = () => (state: RootState) => state.extra.bypassChooser

export const isVotedByElectionId = (electionId: string | undefined) => (state: RootState) => {
    return electionId ? state.extra.isVoted[electionId] : false
}

export const isDeclineToVoteByElectionId =
    (electionId: string | undefined) => (state: RootState) => {
        return electionId ? Boolean(state.extra.declinedToVote[electionId]) : false
    }

export const isAcclaimedElectionCompleted =
    (electionId: string | undefined) => (state: RootState) =>
        electionId ? Boolean(state.extra.completedAcclaimedElections?.[electionId]) : false

export default extraSlice.reducer
