// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EInvalidVotePolicy} from "@sequentech/ui-core"
import type {BallotSelection, ICandidate, IContest} from "@sequentech/ui-core"
import {ELECTION_WITH_INVALID} from "../../fixtures/election"
import {IBallotStyle} from "../ballotStyles/ballotStylesSlice"
import ballotSelectionsReducer, {
    ballotSelectionsSlice,
    BallotSelectionsState,
    resetBallotSelection,
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

describe("ballotSelectionsSlice mutual exclusion between explicit-invalid and other choices", () => {
    it.each([EInvalidVotePolicy.ALLOWED, EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT])(
        "selecting explicit invalid alone sets is_explicit_invalid under %s",
        (policy) => {
            const ballotStyle = buildBallotStyle(policy)
            let state = initState(ballotStyle)
            state = ballotSelectionsSlice.reducer(
                state,
                setBallotSelectionInvalidVote({
                    ballotStyle,
                    contestId: CONTEST_ID,
                    isExplicitInvalid: true,
                })
            )
            expect(getContestState(state, ballotStyle)?.is_explicit_invalid).toBe(true)
        }
    )

    it("selecting a regular candidate clears a previously selected explicit invalid under ALLOWED_WITH_EXCLUSIVE_EXPLICIT", () => {
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

        const contestState = getContestState(state, ballotStyle)
        expect(contestState?.is_explicit_invalid).toBe(false)
        expect(contestState?.choices.find((c) => c.id === REGULAR_CANDIDATE_ID)?.selected).toBe(0)
    })

    it("selecting a regular candidate does NOT clear a previously selected explicit invalid under ALLOWED (bundling preserved)", () => {
        const ballotStyle = buildBallotStyle(EInvalidVotePolicy.ALLOWED)
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

        const contestState = getContestState(state, ballotStyle)
        expect(contestState?.is_explicit_invalid).toBe(true)
        expect(contestState?.choices.find((c) => c.id === REGULAR_CANDIDATE_ID)?.selected).toBe(0)
    })

    it("selecting explicit invalid clears a previously selected regular candidate under ALLOWED_WITH_EXCLUSIVE_EXPLICIT", () => {
        const ballotStyle = buildBallotStyle(EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT)
        let state = initState(ballotStyle)
        state = ballotSelectionsSlice.reducer(
            state,
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId: CONTEST_ID,
                voteChoice: {id: REGULAR_CANDIDATE_ID, selected: 0},
            })
        )

        state = ballotSelectionsSlice.reducer(
            state,
            setBallotSelectionInvalidVote({
                ballotStyle,
                contestId: CONTEST_ID,
                isExplicitInvalid: true,
            })
        )

        const contestState = getContestState(state, ballotStyle)
        expect(contestState?.is_explicit_invalid).toBe(true)
        expect(contestState?.choices.every((c) => c.selected === -1)).toBe(true)
    })

    it("selecting explicit invalid does NOT clear a previously selected regular candidate under ALLOWED (bundling preserved)", () => {
        const ballotStyle = buildBallotStyle(EInvalidVotePolicy.ALLOWED)
        let state = initState(ballotStyle)
        state = ballotSelectionsSlice.reducer(
            state,
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId: CONTEST_ID,
                voteChoice: {id: REGULAR_CANDIDATE_ID, selected: 0},
            })
        )

        state = ballotSelectionsSlice.reducer(
            state,
            setBallotSelectionInvalidVote({
                ballotStyle,
                contestId: CONTEST_ID,
                isExplicitInvalid: true,
            })
        )

        const contestState = getContestState(state, ballotStyle)
        expect(contestState?.is_explicit_invalid).toBe(true)
        expect(contestState?.choices.find((c) => c.id === REGULAR_CANDIDATE_ID)?.selected).toBe(0)
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
})
