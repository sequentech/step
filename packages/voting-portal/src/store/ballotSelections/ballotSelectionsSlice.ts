// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {
    isUndefined,
    IDecodedVoteContest,
    IDecodedVoteChoice,
    BallotSelection,
    EInvalidVoteExclusivityPolicy,
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
                                    is_decline_to_vote: currentContestValue.is_decline_to_vote,
                                    invalid_errors: currentContestValue.invalid_errors,
                                    invalid_alerts: currentContestValue.invalid_alerts,
                                    choices: currentContestValue.choices,
                                }
                            }

                            return {
                                contest_id: question.id,
                                is_explicit_invalid: false,
                                is_decline_to_vote: false,
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

                // Under EXCLUSIVE, marking the ballot explicit-invalid is
                // mutually exclusive with any other selection in the
                // contest, mirroring how blank vote already clears
                // everything else when selected.
                if (
                    action.payload.isExplicitInvalid &&
                    ballotEmlContest.presentation?.invalid_vote_exclusivity_policy ===
                        EInvalidVoteExclusivityPolicy.EXCLUSIVE
                ) {
                    currentQuestion.choices = currentQuestion.choices.map((choice) => ({
                        ...choice,
                        selected: -1,
                    }))
                }
            }
            return state
        },
        setBallotSelectionBlankVote: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
                contestId: string
                candidateId: string
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
                    return {
                        ...choice,
                        selected: choice.id === action.payload.candidateId ? 0 : -1,
                    }
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

                const explicitBlankCandidateIds = new Set(
                    ballotEmlContest.candidates
                        .filter((candidate) => candidate.presentation?.is_explicit_blank)
                        .map((candidate) => candidate.id)
                )
                const isSelectingExplicitBlank =
                    explicitBlankCandidateIds.has(action.payload.voteChoice.id) &&
                    action.payload.voteChoice.selected > -1

                if (action.payload.voteChoice.selected > -1 && !isSelectingExplicitBlank) {
                    currentQuestion.choices = currentQuestion.choices.map((choice) =>
                        explicitBlankCandidateIds.has(choice.id)
                            ? {...choice, selected: -1}
                            : choice
                    )

                    // Under EXCLUSIVE, selecting a real candidate is
                    // mutually exclusive with explicit invalid.
                    if (
                        ballotEmlContest.presentation?.invalid_vote_exclusivity_policy ===
                        EInvalidVoteExclusivityPolicy.EXCLUSIVE
                    ) {
                        currentQuestion.is_explicit_invalid = false
                    }
                }
            }

            return state
        },
        setAllBallotSelectionsDeclineToVote: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
            }>
        ): BallotSelectionsState => {
            let currentElection = state[action.payload.ballotStyle.election_id]

            if (!isUndefined(currentElection)) {
                currentElection.forEach((currentQuestion) => {
                    currentQuestion.is_decline_to_vote = true
                    // A declined ballot must not carry per-contest explicit
                    // invalid markers, otherwise it would be tallied as
                    // invalid instead of declined.
                    currentQuestion.is_explicit_invalid = false
                    currentQuestion.choices = currentQuestion.choices.map((choice) => {
                        if (choice.selected > -1) {
                            choice.selected = -1
                        }
                        return choice
                    })
                })
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
    setAllBallotSelectionsDeclineToVote,
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
