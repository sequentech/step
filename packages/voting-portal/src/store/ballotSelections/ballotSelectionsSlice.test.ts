// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EInvalidVotePolicy} from "@sequentech/ui-core"
import {ELECTION_WITH_INVALID} from "../../fixtures/election"
import {
    ballotSelectionsSlice,
    BallotSelectionsState,
    resetBallotSelection,
    setBallotSelectionInvalidVote,
    setBallotSelectionVoteChoice,
} from "./ballotSelectionsSlice"
import {IBallotStyle} from "../ballotStyles/ballotStylesSlice"

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
