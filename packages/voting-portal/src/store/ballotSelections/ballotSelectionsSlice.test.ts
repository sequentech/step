// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {applySelection, EInvalidVotePolicy} from "@sequentech/ui-core"
import type {BallotSelection, ICandidate, IContest} from "@sequentech/ui-core"
import {ELECTION_WITH_INVALID} from "../../fixtures/election"
import {IBallotStyle} from "../ballotStyles/ballotStylesSlice"
import ballotSelectionsReducer, {
    ballotSelectionsSlice,
    BallotSelectionsState,
    resetBallotSelection,
    setAllBallotSelectionsBlankBallot,
    setAllBallotSelectionsDeclineToVote,
    setBallotSelectionBlankVote,
    setBallotSelectionInvalidVote,
    setBallotSelectionVoteChoice,
} from "./ballotSelectionsSlice"

const contest = ELECTION_WITH_INVALID.contests[0]
const CONTEST_ID = contest.id
const REGULAR_CANDIDATE_ID = contest.candidates[0].id

const buildBallotStyle = (invalidVotePolicy: EInvalidVotePolicy): IBallotStyle => {
    const ballotEml = structuredClone(ELECTION_WITH_INVALID)
    ballotEml.contests[0].presentation = {
        ...ballotEml.contests[0].presentation,
        invalid_vote_policy: invalidVotePolicy,
    }
    return {
        id: ballotEml.id,
        election_id: ballotEml.election_id,
        election_event_id: ballotEml.election_event_id,
        tenant_id: ballotEml.tenant_id,
        ballot_eml: ballotEml,
        created_at: "2026-01-01T00:00:00.000Z",
        last_updated_at: "2026-01-01T00:00:00.000Z",
    }
}

const initState = (ballotStyle: IBallotStyle): BallotSelectionsState =>
    ballotSelectionsSlice.reducer({}, resetBallotSelection({ballotStyle, force: true}))

const getContestState = (state: BallotSelectionsState, ballotStyle: IBallotStyle) =>
    state[ballotStyle.election_id]?.find((c) => c.contest_id === CONTEST_ID)

// The marker rules — when the explicit-invalid marker clears the voter's
// selections and when it stands beside them — are decided by sequent-core
// and tested there, exhaustively and against every policy
// (`the_invalid_marker_keeps_company_unless_the_policy_forbids_it`). What
// remains this suite's business is the wiring: that the reducer asks about
// the right contest, hands over the edit the voter made, and writes the whole
// answer back into state. A mistake there would survive any number of tests
// of the rules themselves.
describe("ballotSelectionsSlice delegates selection edits to the validation rules", () => {
    beforeEach(() => {
        applySelection.mockClear()
    })

    it("asks about this contest, with the voter's choice and the current invalid flag", () => {
        const ballotStyle = buildBallotStyle(EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT)
        let state = initState(ballotStyle)
        state = ballotSelectionsSlice.reducer(
            state,
            setBallotSelectionInvalidVote({
                ballotStyle,
                contestId: CONTEST_ID,
                isExplicitInvalid: true,
            })
        )
        state = ballotSelectionsSlice.reducer(
            state,
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId: CONTEST_ID,
                voteChoice: {id: REGULAR_CANDIDATE_ID, selected: 0},
            })
        )

        expect(applySelection).toHaveBeenCalledTimes(2)

        const [markedContest, , markedChoice, markedFlag] = applySelection.mock.calls[0]
        expect(markedContest.id).toBe(CONTEST_ID)
        expect(markedContest.presentation?.invalid_vote_policy).toBe(
            EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT
        )
        expect(markedChoice).toBeNull()
        expect(markedFlag).toBe(true)

        const [chosenContest, chosenSelection, chosenChoice, chosenFlag] =
            applySelection.mock.calls[1]
        expect(chosenContest.id).toBe(CONTEST_ID)
        expect(chosenChoice).toEqual({id: REGULAR_CANDIDATE_ID, selected: 0})
        // The flag as it stood before this edit, so the rules can decide
        // whether the marker and the selection may stand together.
        expect(chosenFlag).toBe(true)
        expect(chosenSelection.contest_id).toBe(CONTEST_ID)
    })

    it("writes back both the choices and the invalid flag the rules return", () => {
        const ballotStyle = buildBallotStyle(EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT)
        const state = initState(ballotStyle)
        const before = getContestState(state, ballotStyle)
        expect(before).toBeDefined()

        // A verdict the reducer cannot have produced on its own: the flag
        // turned off and every choice cleared.
        applySelection.mockReturnValueOnce({
            ...before!,
            is_explicit_invalid: false,
            choices: before!.choices.map((choice) => ({...choice, selected: 7})),
        })

        const next = ballotSelectionsSlice.reducer(
            state,
            setBallotSelectionInvalidVote({
                ballotStyle,
                contestId: CONTEST_ID,
                isExplicitInvalid: true,
            })
        )

        const contestState = getContestState(next, ballotStyle)
        expect(contestState?.is_explicit_invalid).toBe(false)
        expect(contestState?.choices.every((choice) => choice.selected === 7)).toBe(true)
    })

    it("leaves the other contests on the ballot untouched", () => {
        const ballotStyle = buildBallotStyle(EInvalidVotePolicy.ALLOWED)
        const state = initState(ballotStyle)
        const others = state[ballotStyle.election_id]?.filter((c) => c.contest_id !== CONTEST_ID)

        const next = ballotSelectionsSlice.reducer(
            state,
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId: CONTEST_ID,
                voteChoice: {id: REGULAR_CANDIDATE_ID, selected: 0},
            })
        )

        expect(applySelection).toHaveBeenCalledTimes(1)
        expect(
            next[ballotStyle.election_id]?.filter((c) => c.contest_id !== CONTEST_ID)
        ).toEqual(others)
    })
})

const TENANT_ID = "tenant-1"
const ELECTION_EVENT_ID = "ee-1"
const ELECTION_ID = "election-1"
const CONTEST_ID_1 = "contest-1"
const CONTEST_ID_2 = "contest-2"

const makeCandidate = (id: string, contestId: string): ICandidate => ({
    id,
    tenant_id: TENANT_ID,
    election_event_id: ELECTION_EVENT_ID,
    election_id: ELECTION_ID,
    contest_id: contestId,
})

const makeContest = (id: string): IContest => ({
    id,
    tenant_id: TENANT_ID,
    election_event_id: ELECTION_EVENT_ID,
    election_id: ELECTION_ID,
    max_votes: 1,
    min_votes: 0,
    winning_candidates_num: 1,
    is_encrypted: true,
    candidates: [makeCandidate(`${id}-candidate-1`, id)],
})

const makeBallotStyle = (): IBallotStyle => ({
    id: "ballot-style-1",
    election_id: ELECTION_ID,
    election_event_id: ELECTION_EVENT_ID,
    tenant_id: TENANT_ID,
    created_at: "2026-01-01T00:00:00Z",
    last_updated_at: "2026-01-01T00:00:00Z",
    ballot_eml: {
        id: "ballot-style-1",
        tenant_id: TENANT_ID,
        election_event_id: ELECTION_EVENT_ID,
        election_id: ELECTION_ID,
        area_id: "area-1",
        contests: [makeContest(CONTEST_ID_1), makeContest(CONTEST_ID_2)],
    },
})

const makeBlankElection = (): BallotSelection => [
    {
        contest_id: CONTEST_ID_1,
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: true,
        invalid_errors: [],
        invalid_alerts: [],
        choices: [{id: `${CONTEST_ID_1}-candidate-1`, selected: -1}],
    },
    {
        contest_id: CONTEST_ID_2,
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: true,
        invalid_errors: [],
        invalid_alerts: [],
        choices: [{id: `${CONTEST_ID_2}-candidate-1`, selected: -1}],
    },
]

const makeState = (): BallotSelectionsState => ({
    [ELECTION_ID]: makeBlankElection(),
})

describe("ballotSelectionsSlice is_blank_ballot consistency", () => {
    it("clears is_blank_ballot on every contest when a candidate is selected in one contest", () => {
        const ballotStyle = makeBallotStyle()
        const state = ballotSelectionsReducer(
            makeState(),
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId: CONTEST_ID_1,
                voteChoice: {id: `${CONTEST_ID_1}-candidate-1`, selected: 0},
            })
        )

        const election = state[ELECTION_ID]
        expect(election?.every((contest) => contest.is_blank_ballot === false)).toBe(true)
    })

    it("clears is_blank_ballot on every contest when a contest is marked explicitly invalid", () => {
        const ballotStyle = makeBallotStyle()
        const state = ballotSelectionsReducer(
            makeState(),
            setBallotSelectionInvalidVote({
                ballotStyle,
                contestId: CONTEST_ID_1,
                isExplicitInvalid: true,
            })
        )

        const election = state[ELECTION_ID]
        expect(election?.every((contest) => contest.is_blank_ballot === false)).toBe(true)
    })

    it("clears is_blank_ballot on every contest when the ballot is declined", () => {
        const ballotStyle = makeBallotStyle()
        const state = ballotSelectionsReducer(
            makeState(),
            setAllBallotSelectionsDeclineToVote({ballotStyle})
        )

        const election = state[ELECTION_ID]
        expect(election?.every((contest) => contest.is_blank_ballot === false)).toBe(true)
        expect(election?.every((contest) => contest.is_decline_to_vote === true)).toBe(true)
    })

    it("clears is_blank_ballot on every contest when an explicit blank candidate is selected", () => {
        const ballotStyle = makeBallotStyle()
        const state = ballotSelectionsReducer(
            makeState(),
            setBallotSelectionBlankVote({
                ballotStyle,
                contestId: CONTEST_ID_1,
                candidateId: `${CONTEST_ID_1}-candidate-1`,
            })
        )

        const election = state[ELECTION_ID]
        expect(election?.every((contest) => contest.is_blank_ballot === false)).toBe(true)
    })

    it("clears is_blank_ballot on every contest when a single contest is force-reset", () => {
        const ballotStyle = makeBallotStyle()
        const state = ballotSelectionsReducer(
            makeState(),
            resetBallotSelection({
                ballotStyle,
                force: true,
                contestId: CONTEST_ID_1,
            })
        )

        const election = state[ELECTION_ID]
        expect(election?.every((contest) => contest.is_blank_ballot === false)).toBe(true)
    })

    it("sets is_blank_ballot and clears is_decline_to_vote on every contest, even from a declined state", () => {
        const ballotStyle = makeBallotStyle()
        const declinedState = ballotSelectionsReducer(
            makeState(),
            setAllBallotSelectionsDeclineToVote({ballotStyle})
        )
        const state = ballotSelectionsReducer(
            declinedState,
            setAllBallotSelectionsBlankBallot({ballotStyle})
        )

        const election = state[ELECTION_ID]
        expect(election?.every((contest) => contest.is_blank_ballot === true)).toBe(true)
        expect(election?.every((contest) => contest.is_decline_to_vote === false)).toBe(true)
    })
})
