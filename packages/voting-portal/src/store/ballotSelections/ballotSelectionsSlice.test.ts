// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {BallotSelection, ICandidate, IContest} from "@sequentech/ui-core"
import {IBallotStyle} from "../ballotStyles/ballotStylesSlice"

// @sequentech/ui-core re-exports the sequent-core WASM glue module, which
// ships as raw ESM and isn't transformed by this package's Jest config.
// The slice under test only uses isUndefined at runtime, so stub the
// module rather than pulling in the WASM build.
jest.mock("@sequentech/ui-core", () => ({
    isUndefined: (value: unknown): boolean => value === undefined,
}))

import ballotSelectionsReducer, {
    setAllBallotSelectionsDeclineToVote,
    setBallotSelectionInvalidVote,
    setBallotSelectionVoteChoice,
    BallotSelectionsState,
} from "./ballotSelectionsSlice"

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
})
