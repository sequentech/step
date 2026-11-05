// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {BallotSelection, IBallotStyle as IElectionDTO} from "@sequentech/ui-core"
import {
    buildSlate,
    buildSlatesBallot,
    buildSlatesBallotStyle,
    PRESIDENT,
    SECRETARY,
    TRUSTEES,
} from "../fixtures/slateChoices"
import {
    ballotSelectionsSlice,
    resetBallotSelection,
} from "../store/ballotSelections/ballotSelectionsSlice"
import {computeSlateChoices, SlateChoicesError, slateRemovesChoices} from "./SlateChoices"

const emptySelection = (ballotEml: IElectionDTO): BallotSelection => {
    const ballotStyle = buildSlatesBallotStyle(ballotEml)
    const state = ballotSelectionsSlice.reducer(
        {},
        resetBallotSelection({ballotStyle, force: true})
    )
    return state[ballotStyle.election_id] ?? []
}

const select = (selection: BallotSelection, contestId: string, ids: string[]): BallotSelection =>
    selection.map((contest) =>
        contest.contest_id === contestId
            ? {
                  ...contest,
                  choices: contest.choices.map((choice) =>
                      ids.includes(choice.id) ? {...choice, selected: 0} : choice
                  ),
              }
            : contest
    )

const selectedIds = (selection: BallotSelection, contestId: string): string[] =>
    (selection.find((contest) => contest.contest_id === contestId)?.choices ?? [])
        .filter((choice) => choice.selected > -1)
        .map((choice) => choice.id)

describe("computeSlateChoices", () => {
    const ballot = buildSlatesBallot()
    const forward = buildSlate(ballot, "forward")
    const voices = buildSlate(ballot, "independent-voices")

    it("selects every member of a full slate and nothing else", () => {
        const result = computeSlateChoices(forward, emptySelection(ballot))
        expect(selectedIds(result.selection, PRESIDENT)).toEqual(["p-forward"])
        expect(selectedIds(result.selection, SECRETARY)).toEqual(["s-forward"])
        expect(selectedIds(result.selection, TRUSTEES)).toEqual([
            "t-forward-1",
            "t-forward-2",
            "t-forward-3",
        ])
        expect(result.changes).toEqual([
            {contestId: PRESIDENT, added: ["p-forward"], removed: []},
            {contestId: SECRETARY, added: ["s-forward"], removed: []},
            {
                contestId: TRUSTEES,
                added: ["t-forward-1", "t-forward-2", "t-forward-3"],
                removed: [],
            },
        ])
        expect(slateRemovesChoices(result)).toBe(false)
    })

    it("does not change the selection it was given", () => {
        const current = emptySelection(ballot)
        const before = structuredClone(current)
        computeSlateChoices(forward, current)
        expect(current).toEqual(before)
    })

    it("leaves contests a partial slate does not cover untouched", () => {
        const current = select(emptySelection(ballot), PRESIDENT, ["p-independent"])
        const result = computeSlateChoices(voices, current)
        expect(selectedIds(result.selection, PRESIDENT)).toEqual(["p-independent"])
        expect(selectedIds(result.selection, SECRETARY)).toEqual([])
        expect(selectedIds(result.selection, TRUSTEES)).toEqual(["t-voices-1", "t-voices-2"])
        expect(result.changes.map((change) => change.contestId)).toEqual([TRUSTEES])
    })

    it("replaces the marks of covered contests so no contest exceeds its maximum", () => {
        let current = select(emptySelection(ballot), PRESIDENT, ["p-independent"])
        current = select(current, TRUSTEES, ["t-independent", "t-members-1", "t-forward-1"])
        const result = computeSlateChoices(forward, current)
        for (const contest of ballot.contests) {
            expect(selectedIds(result.selection, contest.id).length).toBeLessThanOrEqual(
                contest.max_votes
            )
        }
        expect(selectedIds(result.selection, PRESIDENT)).toEqual(["p-forward"])
        expect(result.changes).toEqual([
            {contestId: PRESIDENT, added: ["p-forward"], removed: ["p-independent"]},
            {contestId: SECRETARY, added: ["s-forward"], removed: []},
            {
                contestId: TRUSTEES,
                added: ["t-forward-2", "t-forward-3"],
                removed: ["t-members-1", "t-independent"],
            },
        ])
        expect(slateRemovesChoices(result)).toBe(true)
    })

    it("reports no change for contests that already match the slate", () => {
        const applied = computeSlateChoices(forward, emptySelection(ballot)).selection
        expect(computeSlateChoices(forward, applied).changes).toEqual([])
    })

    it("drops the text of a write-in it deselects", () => {
        const current = emptySelection(ballot).map((contest) =>
            contest.contest_id === PRESIDENT
                ? {
                      ...contest,
                      choices: contest.choices.map((choice) =>
                          choice.id === "p-independent"
                              ? {...choice, selected: 0, write_in_text: "Someone"}
                              : choice
                      ),
                  }
                : contest
        )
        const president = computeSlateChoices(forward, current).selection.find(
            (contest) => contest.contest_id === PRESIDENT
        )
        expect(president?.choices).toEqual([
            {id: "p-forward", selected: 0},
            {id: "p-members", selected: -1},
            {id: "p-independent", selected: -1},
        ])
    })

    it("clears invalid, decline and blank markers", () => {
        const current = emptySelection(ballot).map((contest) => ({
            ...contest,
            is_explicit_invalid: contest.contest_id === TRUSTEES,
            is_decline_to_vote: true,
            is_blank_ballot: true,
            invalid_errors: [{message: "errors.implicit.underVote"}],
        })) as BallotSelection
        const result = computeSlateChoices(voices, current)
        const trustees = result.selection.find((contest) => contest.contest_id === TRUSTEES)
        expect(trustees?.is_explicit_invalid).toBe(false)
        expect(trustees?.invalid_errors).toEqual([])
        expect(result.selection.every((contest) => !contest.is_blank_ballot)).toBe(true)
        expect(result.selection.every((contest) => !contest.is_decline_to_vote)).toBe(true)
        const president = result.selection.find((contest) => contest.contest_id === PRESIDENT)
        expect(president?.invalid_errors).toHaveLength(1)
    })

    it("counts a cleared explicit invalid mark as a removed choice", () => {
        const invalidBallot = buildSlatesBallot()
        invalidBallot.contests[0].candidates.push({
            ...invalidBallot.contests[0].candidates[0],
            id: "p-invalid",
            name: "Invalid vote",
            presentation: {is_explicit_invalid: true},
        })
        const current = emptySelection(invalidBallot).map((contest) =>
            contest.contest_id === PRESIDENT ? {...contest, is_explicit_invalid: true} : contest
        )
        const result = computeSlateChoices(buildSlate(invalidBallot, "forward"), current)
        expect(result.changes[0]).toEqual({
            contestId: PRESIDENT,
            added: ["p-forward"],
            removed: ["p-invalid"],
        })
    })

    it("refuses a selection that does not hold the slate's contests or candidates", () => {
        const withoutContest = emptySelection(ballot).filter(
            (contest) => contest.contest_id !== TRUSTEES
        )
        expect(() => computeSlateChoices(forward, withoutContest)).toThrow(SlateChoicesError)
        const withoutChoice = emptySelection(ballot).map((contest) => ({
            ...contest,
            choices: contest.choices.filter((choice) => choice.id !== "p-forward"),
        }))
        expect(() => computeSlateChoices(forward, withoutChoice)).toThrow(SlateChoicesError)
    })

    it("refuses a slate that would exceed a contest's maximum", () => {
        const overCapacity = structuredClone(forward)
        overCapacity.contests[2].contest.max_votes = 2
        expect(() => computeSlateChoices(overCapacity, emptySelection(ballot))).toThrow(
            SlateChoicesError
        )
    })

    it("refuses a slate with nothing to select or the same candidate twice", () => {
        const empty = {...forward, contests: []}
        expect(() => computeSlateChoices(empty, emptySelection(ballot))).toThrow(SlateChoicesError)
        const repeated = structuredClone(buildSlate(ballot, "independent-voices"))
        repeated.contests[0].candidates.push(repeated.contests[0].candidates[0])
        expect(() => computeSlateChoices(repeated, emptySelection(ballot))).toThrow(
            SlateChoicesError
        )
    })
})
