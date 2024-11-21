// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {
    isUndefined,
    IDecodedVoteContest,
    IDecodedVoteChoice,
    BallotSelection,
} from "@sequentech/ui-core"
import {IBallotStyle} from "../ballotStyles/ballotStylesSlice"

export interface BallotSelectionsState {
    [electionId: string]: BallotSelection | undefined
}

const initialState: BallotSelectionsState = {}

export const ballotSelectionsSlice = createSlice({
    name: "ballotSelections",
    initialState,
    reducers: {
        clearBallot: (state): BallotSelectionsState => {
            state = initialState
            return initialState
        },
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
                                    invalid_alerts: currentContestValue.invalid_alerts,
                                    choices: currentContestValue.choices,
                                }
                            }

                            return {
                                contest_id: question.id,
                                is_explicit_invalid: false,
                                invalid_errors: [],
                                invalid_alerts: [],
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
                contestId: string
                isExplicitInvalid: boolean
            }>
        ): BallotSelectionsState => {
            const ballotEmlContest = action.payload.ballotStyle.ballot_eml.contests.find(
                (contest) => contest.id === action.payload.contestId
            )
            // check bounds
            if (isUndefined(ballotEmlContest)) {
                return state
            }
            // find question
            let currentElection = state[action.payload.ballotStyle.election_id]
            let currentQuestion = currentElection?.find(
                (contest) => contest.contest_id === action.payload.contestId
            )
            // update state
            if (!isUndefined(currentQuestion)) {
                currentQuestion.is_explicit_invalid = action.payload.isExplicitInvalid
            }
            return state
        },
        setBallotSelectionBlankVote: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
                contestId: string
            }>
        ): BallotSelectionsState => {
            const ballotEmlContest = action.payload.ballotStyle.ballot_eml.contests.find(
                (contest) => contest.id === action.payload.contestId
            )
            // check bounds
            if (isUndefined(ballotEmlContest)) {
                return state
            }
            // find question
            let currentElection = state[action.payload.ballotStyle.election_id]
            let currentQuestion = currentElection?.find(
                (contest) => contest.contest_id === action.payload.contestId
            )
            // update state
            if (!isUndefined(currentQuestion)) {
                currentQuestion.is_explicit_invalid = false
                currentQuestion.choices = currentQuestion.choices.map((choice) => {
                    if (choice.selected > -1) {
                        choice.selected = -1
                    }
                    return choice
                })
            }
            return state
        },
        setBallotSelectionVoteChoice: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
                contestId: string
                voteChoice: IDecodedVoteChoice
            }>
        ): BallotSelectionsState => {
            const ballotEmlContest = action.payload.ballotStyle.ballot_eml.contests.find(
                (contest) => contest.id === action.payload.contestId
            )
            // check bounds
            if (isUndefined(ballotEmlContest)) {
                return state
            }
            let currentElection = state[action.payload.ballotStyle.election_id]
            let currentQuestion = currentElection?.find(
                (contest) => contest.contest_id === action.payload.contestId
            )
            let currentChoiceIndex = currentQuestion?.choices.findIndex(
                (choice) => action.payload.voteChoice.id === choice.id
            )
            const currentChoice =
                !isUndefined(currentElection) &&
                !isUndefined(currentChoiceIndex) &&
                currentChoiceIndex > -1
                    ? currentQuestion?.choices[currentChoiceIndex]
                    : undefined

            // check election state
            if (!currentElection || isUndefined(currentChoice)) {
                return state
            }

            // modify
            if (currentQuestion && !isUndefined(currentChoiceIndex)) {
                currentQuestion.choices[currentChoiceIndex] = action.payload.voteChoice
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
    clearBallot,
    setBallotSelection,
    resetBallotSelection,
    setBallotSelectionInvalidVote,
    setBallotSelectionBlankVote,
    setBallotSelectionVoteChoice,
} = ballotSelectionsSlice.actions

export const selectBallotSelectionVoteChoice =
    (electionId: string, contestId: string, answerIndex: string) => (state: RootState) =>
        state.ballotSelections[electionId]
            ?.find((contest) => contest.contest_id === contestId)
            ?.choices.find((choice) => answerIndex === choice.id)

export const selectBallotSelectionQuestion =
    (electionId: string, contestId: string) => (state: RootState) =>
        state.ballotSelections[electionId]?.find((contest) => contest.contest_id === contestId)

export const selectBallotSelectionByElectionId = (electionId: string) => (state: RootState) =>
    state.ballotSelections[electionId]

export default ballotSelectionsSlice.reducer
