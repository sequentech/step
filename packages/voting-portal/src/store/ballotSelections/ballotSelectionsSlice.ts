// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IDecodedVoteContest, IDecodedVoteChoice} from "sequent-core"
import {isUndefined} from "@sequentech/ui-essentials"
import {IBallotStyle} from "../ballotStyles/ballotStylesSlice"

export type BallotSelection = Array<IDecodedVoteContest>

export interface BallotSelectionsState {
    [electionId: string]: BallotSelection | undefined
}

const initialState: BallotSelectionsState = {}

export const ballotSelectionsSlice = createSlice({
    name: "ballotSelections",
    initialState,
    reducers: {
        setBallotSelection: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
                ballotSelection: BallotSelection
            }>
        ): BallotSelectionsState => {
            let currentElection = state[action.payload.ballotStyle.election_id]
            if (currentElection) {
                state[action.payload.ballotStyle.election_id] = action.payload.ballotSelection
            }

            return state
        },
        resetBallotSelection: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
                force?: boolean
                contestId?: string
            }>
        ): BallotSelectionsState => {
            let currentElection = state[action.payload.ballotStyle.election_id]
            if (!currentElection || action.payload.force) {
                state[action.payload.ballotStyle.election_id] =
                    action.payload.ballotStyle.ballot_eml.contests.map(
                        (question): IDecodedVoteContest => {
                            let currentContestValue = state[
                                action.payload.ballotStyle.election_id
                            ]?.find((contest) => contest.contest_id === question.id)

                            if (
                                currentContestValue &&
                                action.payload.contestId &&
                                action.payload.contestId !== question.id
                            ) {
                                return {
                                    contest_id: currentContestValue.contest_id,
                                    is_explicit_invalid: currentContestValue.is_explicit_invalid,
                                    invalid_errors: currentContestValue.invalid_errors,
                                    choices: currentContestValue.choices,
                                }
                            }

                            return {
                                contest_id: question.id,
                                is_explicit_invalid: false,
                                invalid_errors: [],
                                choices: question.candidates.map((answer) => ({
                                    id: answer.id,
                                    selected: -1,
                                })),
                            }
                        }
                    )
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
                action.payload.ballotStyle.ballot_eml.contests.length
            ) {
                return state
            }
            // find question
            let currentElection = state[action.payload.ballotStyle.election_id]
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
                action.payload.ballotStyle.ballot_eml.contests.length
            ) {
                return state
            }
            let currentElection = state[action.payload.ballotStyle.election_id]
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
            currentElection = state[action.payload.ballotStyle.election_id]
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

export const {
    setBallotSelection,
    resetBallotSelection,
    setBallotSelectionInvalidVote,
    setBallotSelectionVoteChoice,
} = ballotSelectionsSlice.actions

export const selectBallotSelectionVoteChoice =
    (electionId: string, questionIndex: number, answerIndex: string) => (state: RootState) =>
        state.ballotSelections[electionId]?.[questionIndex]?.choices.find(
            (choice) => answerIndex === choice.id
        )

export const selectBallotSelectionQuestion =
    (electionId: string, questionIndex: number) => (state: RootState) =>
        state.ballotSelections[electionId]?.[questionIndex]

export const selectBallotSelectionByElectionId = (electionId: string) => (state: RootState) =>
    state.ballotSelections[electionId]

export default ballotSelectionsSlice.reducer
