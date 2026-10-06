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
    EInvalidVotePolicy,
} from "@sequentech/ui-core"
import {IBallotStyle} from "../ballotStyles/ballotStylesSlice"

export interface BallotSelectionsState {
    [electionId: string]: BallotSelection | undefined
}

const initialState: BallotSelectionsState = {}

/**
 * The codec rejects a ballot-level blank flag that disagrees with a
 * contest's actual content (a real selection, an invalid mark, or a
 * decline), so any reducer that makes such a change must clear
 * is_blank_ballot across every contest in the election, not just the
 * one being edited.
 */
const clearBlankBallotFlag = (election: BallotSelection): void => {
    election.forEach((contest) => {
        contest.is_blank_ballot = false
    })
}

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
                                    is_blank_ballot: currentContestValue.is_blank_ballot,
                                    invalid_errors: currentContestValue.invalid_errors,
                                    invalid_alerts: currentContestValue.invalid_alerts,
                                    choices: currentContestValue.choices,
                                }
                            }

                            return {
                                contest_id: question.id,
                                is_explicit_invalid: false,
                                is_decline_to_vote: false,
                                is_blank_ballot: false,
                                invalid_errors: [],
                                invalid_alerts: [],
                                choices: question.candidates.map((answer) => ({
                                    id: answer.id,
                                    selected: -1,
                                })),
                            }
                        }
                    )

                // A single reset contest (force + contestId) is rebuilt as
                // not blank while every other contest keeps its prior
                // is_blank_ballot value, which can leave the election with
                // some contests blank and one not -- a combination the
                // codec rejects.
                const resetElection = state[action.payload.ballotStyle.election_id]
                if (resetElection) {
                    clearBlankBallotFlag(resetElection)
                }
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

                // Under ALLOWED_WITH_EXCLUSIVE_EXPLICIT, marking the ballot
                // explicit-invalid is mutually exclusive with any other
                // selection in the contest, mirroring how blank vote already
                // clears everything else when selected.
                if (
                    action.payload.isExplicitInvalid &&
                    ballotEmlContest.presentation?.invalid_vote_policy ===
                        EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT
                ) {
                    currentQuestion.choices = currentQuestion.choices.map((choice) => ({
                        ...choice,
                        selected: -1,
                    }))
                }

                if (!isUndefined(currentElection)) {
                    clearBlankBallotFlag(currentElection)
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
                if (!isUndefined(currentElection)) {
                    clearBlankBallotFlag(currentElection)
                }
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

                    // Under ALLOWED_WITH_EXCLUSIVE_EXPLICIT, selecting a real
                    // candidate is mutually exclusive with explicit invalid.
                    if (
                        ballotEmlContest.presentation?.invalid_vote_policy ===
                        EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT
                    ) {
                        currentQuestion.is_explicit_invalid = false
                    }
                }

                clearBlankBallotFlag(currentElection)
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
                    // A ballot cannot be both declined and blank.
                    currentQuestion.is_blank_ballot = false
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
        setAllBallotSelectionsBlankBallot: (
            state,
            action: PayloadAction<{
                ballotStyle: IBallotStyle
            }>
        ): BallotSelectionsState => {
            let currentElection = state[action.payload.ballotStyle.election_id]

            if (!isUndefined(currentElection)) {
                currentElection.forEach((currentQuestion) => {
                    currentQuestion.is_blank_ballot = true
                    // A ballot cannot be both blank and declined.
                    currentQuestion.is_decline_to_vote = false
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
    setAllBallotSelectionsBlankBallot,
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
