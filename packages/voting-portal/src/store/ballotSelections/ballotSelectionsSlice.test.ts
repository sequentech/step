// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EInvalidVoteExclusivityPolicy} from "@sequentech/ui-core"
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

const buildBallotStyle = (
    exclusivityPolicy: EInvalidVoteExclusivityPolicy | undefined
): IBallotStyle => {
    const ballotEml = structuredClone(ELECTION_WITH_INVALID)
    ballotEml.contests[0].presentation = {
        ...ballotEml.contests[0].presentation,
        invalid_vote_exclusivity_policy: exclusivityPolicy,
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

describe("ballotSelectionsSlice mutual exclusion driven by invalid_vote_exclusivity_policy", () => {
    it.each([
        undefined,
        EInvalidVoteExclusivityPolicy.INCLUSIVE,
        EInvalidVoteExclusivityPolicy.EXCLUSIVE,
    ])(
        "selecting explicit invalid alone sets is_explicit_invalid regardless of policy (%s)",
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

    it("selecting a regular candidate clears a previously selected explicit invalid under EXCLUSIVE", () => {
        const ballotStyle = buildBallotStyle(EInvalidVoteExclusivityPolicy.EXCLUSIVE)
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

    it.each([undefined, EInvalidVoteExclusivityPolicy.INCLUSIVE])(
        "selecting a regular candidate does NOT clear a previously selected explicit invalid under %s (bundling preserved)",
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
            expect(contestState?.choices.find((c) => c.id === REGULAR_CANDIDATE_ID)?.selected).toBe(
                0
            )
        }
    )

    it("selecting explicit invalid clears a previously selected regular candidate under EXCLUSIVE", () => {
        const ballotStyle = buildBallotStyle(EInvalidVoteExclusivityPolicy.EXCLUSIVE)
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

    it.each([undefined, EInvalidVoteExclusivityPolicy.INCLUSIVE])(
        "selecting explicit invalid does NOT clear a previously selected regular candidate under %s (bundling preserved)",
        (policy) => {
            const ballotStyle = buildBallotStyle(policy)
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
            expect(contestState?.choices.find((c) => c.id === REGULAR_CANDIDATE_ID)?.selected).toBe(
                0
            )
        }
    )
})
