// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IElectionDTO, IDecodedVoteQuestion, IDecodedVoteChoice} from "sequent-core"
import {isUndefined} from "@sequentech/ui-essentials"
import { IBallotStyle } from "../elections/electionsSlice"
//import {fetchElectionByIdAsync} from "../elections/electionsSlice"

export type BallotSelection = Array<IDecodedVoteQuestion>

export interface BallotSelectionsState {
    [ballotStyleId: string]: BallotSelection | undefined
}

const initialState: BallotSelectionsState = {}

export const ballotSelectionsSlice = createSlice({
    name: "ballotSelections",
    initialState,
    reducers: {
        resetBallotSelection: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
                force?: boolean
            }>
        ): BallotSelectionsState => {
            let currentElection = state[action.payload.ballotStyle.id]
            if (!currentElection || action.payload.force) {
                state[action.payload.ballotStyle.id] =
                    action.payload.ballotStyle.ballot_eml.configuration.questions.map((question) => ({
                        is_explicit_invalid: false,
                        invalid_errors: [],
                        choices: question.answers.map((answer) => ({
                            id: answer.id,
                            selected: -1,
                        })),
                    }))
            }

            return state
        },
        setBallotSelectionInvalidVote: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
                questionIndex: number
                isExplicitInvalid: boolean
            }>
        ): BallotSelectionsState => {
            // check bounds
            if (
                action.payload.questionIndex >=
                action.payload.ballotStyle.ballot_eml.configuration.questions.length
            ) {
                return state
            }
            // find question
            let currentElection = state[action.payload.ballotStyle.id]
            let currentQuestion = currentElection?.[action.payload.questionIndex]
            // update state
            if (!isUndefined(currentQuestion)) {
                currentQuestion.is_explicit_invalid = action.payload.isExplicitInvalid
            }
            return state
        },
        setBallotSelectionVoteChoice: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
                questionIndex: number
                voteChoice: IDecodedVoteChoice
            }>
        ): BallotSelectionsState => {
            // check bounds
            if (
                action.payload.questionIndex >=
                    action.payload.ballotStyle.ballot_eml.configuration.questions.length ||
                action.payload.voteChoice.id >=
                    action.payload.ballotStyle.ballot_eml.configuration.questions[action.payload.questionIndex]
                        .answers.length
            ) {
                return state
            }
            let currentElection = state[action.payload.ballotStyle.id]
            let currentChoiceIndex = currentElection?.[
                action.payload.questionIndex
            ]?.choices.findIndex((choice) => action.payload.voteChoice.id === choice.id)
            const currentChoice =
                !isUndefined(currentElection) &&
                !isUndefined(currentChoiceIndex) &&
                currentChoiceIndex > -1
                    ? currentElection[action.payload.questionIndex]?.choices[currentChoiceIndex]
                    : undefined

            // check election state
            if (!currentElection || isUndefined(currentChoice)) {
                return state
            }

            // modify
            currentElection = state[action.payload.ballotStyle.id]
            currentChoiceIndex = currentElection?.[action.payload.questionIndex]?.choices.findIndex(
                (choice) => action.payload.voteChoice.id === choice.id
            )
            if (currentElection && !isUndefined(currentChoiceIndex)) {
                currentElection[action.payload.questionIndex].choices[currentChoiceIndex] =
                    action.payload.voteChoice
            }

            return state
        },
    },
    /*extraReducers: (builder) => {
        builder.addCase(fetchElectionByIdAsync.fulfilled, (state, action) => {
            if (!action.payload) {
                return state
            }
            ballotSelectionsSlice.caseReducers.resetBallotSelection(state, {
                payload: {
                    election: action.payload,
                },
                type: "ballotSelections/resetBallotSelection",
            })
            return state
        })
    },*/
})

export const {resetBallotSelection, setBallotSelectionInvalidVote, setBallotSelectionVoteChoice} =
    ballotSelectionsSlice.actions

export const selectBallotSelectionVoteChoice =
    (ballotStyleId: string, questionIndex: number, answerIndex: number) => (state: RootState) =>
        state.ballotSelections[ballotStyleId]?.[questionIndex]?.choices.find(
            (choice) => answerIndex === choice.id
        )

export const selectBallotSelectionQuestion =
    (ballotStyleId: string, questionIndex: number) => (state: RootState) =>
        state.ballotSelections[ballotStyleId]?.[questionIndex]

export const selectBallotSelection = (ballotStyleId: string) => (state: RootState) =>
    state.ballotSelections[ballotStyleId]

export default ballotSelectionsSlice.reducer
