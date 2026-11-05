// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    buildSlate,
    buildSlatesBallot,
    buildSlatesBallotStyle,
    PRESIDENT,
    SECRETARY,
    TRUSTEES,
} from "../../fixtures/slateChoices"
import {
    applySlateSelection,
    ballotSelectionsSlice,
    BallotSelectionsState,
    resetBallotSelection,
    setAllBallotSelectionsBlankBallot,
    setBallotSelectionVoteChoice,
} from "./ballotSelectionsSlice"

const ballot = buildSlatesBallot()
const ballotStyle = buildSlatesBallotStyle(ballot)
const forward = buildSlate(ballot, "forward")
const voices = buildSlate(ballot, "independent-voices")

const initState = (): BallotSelectionsState =>
    ballotSelectionsSlice.reducer({}, resetBallotSelection({ballotStyle, force: true}))

const selected = (state: BallotSelectionsState, contestId: string): string[] =>
    (state[ballotStyle.election_id]?.find((entry) => entry.contest_id === contestId)?.choices ?? [])
        .filter((choice) => choice.selected > -1)
        .map((choice) => choice.id)

const mark = (state: BallotSelectionsState, contestId: string, id: string) =>
    ballotSelectionsSlice.reducer(
        state,
        setBallotSelectionVoteChoice({ballotStyle, contestId, voteChoice: {id, selected: 0}})
    )

describe("applySlateSelection", () => {
    it("marks every member of the slate in one action", () => {
        const state = ballotSelectionsSlice.reducer(
            initState(),
            applySlateSelection({ballotStyle, slate: forward})
        )
        expect(selected(state, PRESIDENT)).toEqual(["p-forward"])
        expect(selected(state, SECRETARY)).toEqual(["s-forward"])
        expect(selected(state, TRUSTEES)).toEqual(["t-forward-1", "t-forward-2", "t-forward-3"])
    })

    it("produces the same ballot as marking the same candidates by hand", () => {
        const bySlate = ballotSelectionsSlice.reducer(
            initState(),
            applySlateSelection({ballotStyle, slate: forward})
        )
        let byHand = initState()
        byHand = mark(byHand, PRESIDENT, "p-forward")
        byHand = mark(byHand, SECRETARY, "s-forward")
        byHand = mark(byHand, TRUSTEES, "t-forward-1")
        byHand = mark(byHand, TRUSTEES, "t-forward-2")
        byHand = mark(byHand, TRUSTEES, "t-forward-3")
        expect(bySlate).toEqual(byHand)
    })

    it("keeps the contests a partial slate does not cover", () => {
        let state = mark(initState(), PRESIDENT, "p-independent")
        state = mark(state, TRUSTEES, "t-independent")
        state = ballotSelectionsSlice.reducer(
            state,
            applySlateSelection({ballotStyle, slate: voices})
        )
        expect(selected(state, PRESIDENT)).toEqual(["p-independent"])
        expect(selected(state, TRUSTEES)).toEqual(["t-voices-1", "t-voices-2"])
    })

    it("clears the blank ballot flag across the election", () => {
        let state = ballotSelectionsSlice.reducer(
            initState(),
            setAllBallotSelectionsBlankBallot({ballotStyle})
        )
        state = ballotSelectionsSlice.reducer(
            state,
            applySlateSelection({ballotStyle, slate: voices})
        )
        expect(state[ballotStyle.election_id]?.every((entry) => !entry.is_blank_ballot)).toBe(true)
    })

    it("changes nothing when the slate cannot be applied", () => {
        const overCapacity = structuredClone(forward)
        overCapacity.contests[2].contest.max_votes = 2
        const before = mark(initState(), PRESIDENT, "p-independent")
        const after = ballotSelectionsSlice.reducer(
            before,
            applySlateSelection({ballotStyle, slate: overCapacity})
        )
        expect(after).toEqual(before)
    })

    it("changes nothing for an election without selections", () => {
        expect(
            ballotSelectionsSlice.reducer({}, applySlateSelection({ballotStyle, slate: forward}))
        ).toEqual({})
    })
})
