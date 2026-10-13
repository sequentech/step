// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EInvalidVotePolicy, EOverVotePolicy} from "@sequentech/ui-core"
import {
    buildLimitedSlatesBallot,
    buildSlate,
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
    setBallotSelectionInvalidVote,
    setBallotSelectionVoteChoice,
} from "./ballotSelectionsSlice"

const ballot = buildLimitedSlatesBallot()
const ballotStyle = buildSlatesBallotStyle(ballot)
const forward = buildSlate(ballot, "forward")
const membersFirst = buildSlate(ballot, "members-first")
const voices = buildSlate(ballot, "independent-voices")

const reduce = ballotSelectionsSlice.reducer

const initState = (style = ballotStyle): BallotSelectionsState =>
    reduce({}, resetBallotSelection({ballotStyle: style, force: true}))

const selected = (state: BallotSelectionsState, contestId: string): string[] =>
    (state[ballotStyle.election_id]?.find((entry) => entry.contest_id === contestId)?.choices ?? [])
        .filter((choice) => choice.selected > -1)
        .map((choice) => choice.id)

const set = (
    state: BallotSelectionsState,
    contestId: string,
    id: string,
    isSelected = true,
    style = ballotStyle
) =>
    reduce(
        state,
        setBallotSelectionVoteChoice({
            ballotStyle: style,
            contestId,
            voteChoice: {id, selected: isSelected ? 0 : -1},
        })
    )

const withTrusteesPresentation = (presentation: object) =>
    buildSlatesBallotStyle({
        ...ballot,
        contests: ballot.contests.map((entry) =>
            entry.id === TRUSTEES
                ? {...entry, presentation: {...entry.presentation, ...presentation}}
                : entry
        ),
    })

describe("single-seat office", () => {
    it("replaces the current candidate when another is chosen", () => {
        let state = set(initState(), PRESIDENT, "p-forward")
        state = set(state, PRESIDENT, "p-independent")
        expect(selected(state, PRESIDENT)).toEqual(["p-independent"])
    })

    it("can be left without a candidate again", () => {
        let state = set(initState(), PRESIDENT, "p-forward")
        state = set(state, PRESIDENT, "p-forward", false)
        expect(selected(state, PRESIDENT)).toEqual([])
    })

    it("replacing clears an invalid mark, as the contest is started over", () => {
        let state = reduce(
            initState(),
            setBallotSelectionInvalidVote({
                ballotStyle,
                contestId: PRESIDENT,
                isExplicitInvalid: true,
            })
        )
        state = set(state, PRESIDENT, "p-forward")
        expect(
            state[ballotStyle.election_id]?.find((entry) => entry.contest_id === PRESIDENT)
                ?.is_explicit_invalid
        ).toBe(false)
    })

    it("keeps one candidate when a slate is chosen over an individual choice", () => {
        let state = set(initState(), PRESIDENT, "p-independent")
        state = reduce(state, applySlateSelection({ballotStyle, slate: forward}))
        expect(selected(state, PRESIDENT)).toEqual(["p-forward"])
        state = reduce(state, applySlateSelection({ballotStyle, slate: membersFirst}))
        expect(selected(state, PRESIDENT)).toEqual(["p-members"])
        expect(selected(state, SECRETARY)).toEqual(["s-members"])
    })

    it("does not touch the other contests", () => {
        let state = set(initState(), TRUSTEES, "t-independent")
        state = set(state, PRESIDENT, "p-forward")
        state = set(state, PRESIDENT, "p-members")
        expect(selected(state, TRUSTEES)).toEqual(["t-independent"])
    })
})

describe("trustees, at most three", () => {
    const three = (): BallotSelectionsState => {
        let state = set(initState(), TRUSTEES, "t-forward-1")
        state = set(state, TRUSTEES, "t-members-1")
        return set(state, TRUSTEES, "t-independent")
    }

    it("accepts any mix of three candidates", () => {
        expect(selected(three(), TRUSTEES)).toEqual(["t-forward-1", "t-members-1", "t-independent"])
    })

    it("refuses a fourth and keeps the three earlier choices", () => {
        const before = three()
        const after = set(before, TRUSTEES, "t-voices-1")
        expect(after).toEqual(before)
    })

    it("accepts another candidate once one is deselected", () => {
        let state = set(three(), TRUSTEES, "t-members-1", false)
        state = set(state, TRUSTEES, "t-voices-1")
        expect(selected(state, TRUSTEES)).toEqual(["t-forward-1", "t-voices-1", "t-independent"])
    })

    it("still records a write-in text change at the maximum", () => {
        const state = reduce(
            three(),
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId: TRUSTEES,
                voteChoice: {id: "t-independent", selected: 0, write_in_text: "A name"},
            })
        )
        expect(
            state[ballotStyle.election_id]
                ?.find((entry) => entry.contest_id === TRUSTEES)
                ?.choices.find((choice) => choice.id === "t-independent")?.write_in_text
        ).toBe("A name")
    })

    it("refuses a fourth after a slate filled the contest", () => {
        const before = reduce(initState(), applySlateSelection({ballotStyle, slate: forward}))
        const after = set(before, TRUSTEES, "t-independent")
        expect(after).toEqual(before)
        expect(selected(after, TRUSTEES)).toEqual(["t-forward-1", "t-forward-2", "t-forward-3"])
    })

    it("accepts one more after a slate with two trustees, and no further", () => {
        let state = reduce(initState(), applySlateSelection({ballotStyle, slate: voices}))
        state = set(state, TRUSTEES, "t-independent")
        expect(selected(state, TRUSTEES)).toEqual(["t-voices-1", "t-voices-2", "t-independent"])
        expect(set(state, TRUSTEES, "t-forward-1")).toEqual(state)
    })

    it("replaces three individual choices with a slate's three, never adding to them", () => {
        const state = reduce(three(), applySlateSelection({ballotStyle, slate: membersFirst}))
        expect(selected(state, TRUSTEES)).toEqual(["t-members-1", "t-members-2", "t-members-3"])
    })

    it("refuses an invalid mark that would be a fourth selection", () => {
        const before = three()
        const after = reduce(
            before,
            setBallotSelectionInvalidVote({
                ballotStyle,
                contestId: TRUSTEES,
                isExplicitInvalid: true,
            })
        )
        expect(after).toEqual(before)
    })

    it("accepts an invalid mark that replaces the candidates", () => {
        const style = withTrusteesPresentation({
            invalid_vote_policy: EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT,
        })
        let state = initState(style)
        for (const id of ["t-forward-1", "t-members-1", "t-independent"]) {
            state = set(state, TRUSTEES, id, true, style)
        }
        state = reduce(
            state,
            setBallotSelectionInvalidVote({
                ballotStyle: style,
                contestId: TRUSTEES,
                isExplicitInvalid: true,
            })
        )
        expect(selected(state, TRUSTEES)).toEqual([])
    })
})

describe("contests with another over-vote policy", () => {
    it.each([
        EOverVotePolicy.ALLOWED,
        EOverVotePolicy.ALLOWED_WITH_MSG,
        EOverVotePolicy.ALLOWED_WITH_MSG_AND_ALERT,
        EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_ALERT,
    ])("keep recording a selection above the maximum under %s", (policy) => {
        const style = withTrusteesPresentation({over_vote_policy: policy})
        let state = initState(style)
        for (const id of ["t-forward-1", "t-members-1", "t-independent", "t-voices-1"]) {
            state = set(state, TRUSTEES, id, true, style)
        }
        expect(selected(state, TRUSTEES)).toHaveLength(4)
    })
})

describe("a slate that does not fit", () => {
    it("changes no contest, including those where it would fit", () => {
        const tooMany = structuredClone(forward)
        tooMany.contests[2].contest.max_votes = 2
        const before = set(initState(), SECRETARY, "s-independent")
        const after = reduce(before, applySlateSelection({ballotStyle, slate: tooMany}))
        expect(after).toEqual(before)
    })
})
