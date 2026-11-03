// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"

export interface ElectionVoteStepState {
    [electionId: string]: boolean
}
export interface SlateListsExpandedState {
    [electionId: string]: {[slateId: string]: boolean} | undefined
}
export interface ExtraState {
    bypassChooser: boolean
    isVoted: ElectionVoteStepState
    declinedToVote: ElectionVoteStepState
    completedAcclaimedElections: ElectionVoteStepState
    slateListsExpanded: SlateListsExpandedState
}

const initialState: ExtraState = {
    bypassChooser: false,
    isVoted: {},
    declinedToVote: {},
    completedAcclaimedElections: {},
    slateListsExpanded: {},
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
        setSlateListExpanded: (
            state: ExtraState,
            action: PayloadAction<{electionId: string; slateId: string; expanded: boolean}>
        ): ExtraState => {
            const {electionId, slateId, expanded} = action.payload
            state.slateListsExpanded[electionId] = {
                ...state.slateListsExpanded[electionId],
                [slateId]: expanded,
            }
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
    setSlateListExpanded,
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

export const selectSlateListExpanded =
    (electionId: string, slateId: string) =>
    (state: RootState): boolean | undefined =>
        state.extra.slateListsExpanded?.[electionId]?.[slateId]

export default extraSlice.reducer
