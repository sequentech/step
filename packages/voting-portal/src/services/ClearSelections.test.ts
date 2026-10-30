// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {BallotSelection, IBallotStyle as IElectionDTO} from "@sequentech/ui-core"
import {
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
import {clearRemovesChoices, computeClearChoices} from "./ClearSelections"

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

const selectedIds = (selection: BallotSelection): string[] =>
    selection.flatMap((contest) =>
        contest.choices.filter((choice) => choice.selected > -1).map((choice) => choice.id)
    )

describe("computeClearChoices", () => {
    const ballot = buildSlatesBallot()

    it("removes every choice on the ballot and lists them by contest", () => {
        let current = select(emptySelection(ballot), PRESIDENT, ["p-independent"])
        current = select(current, TRUSTEES, ["t-forward-1", "t-voices-2"])
        const result = computeClearChoices(ballot.contests, current)
        expect(selectedIds(result.selection)).toEqual([])
        expect(result.changes).toEqual([
            {contestId: PRESIDENT, added: [], removed: ["p-independent"]},
            {contestId: TRUSTEES, added: [], removed: ["t-forward-1", "t-voices-2"]},
        ])
        expect(clearRemovesChoices(result)).toBe(true)
    })

    it("removes nothing from an empty ballot", () => {
        const result = computeClearChoices(ballot.contests, emptySelection(ballot))
        expect(result.changes).toEqual([])
        expect(clearRemovesChoices(result)).toBe(false)
    })

    it("does not change the selection it was given", () => {
        const current = select(emptySelection(ballot), PRESIDENT, ["p-forward"])
        const before = structuredClone(current)
        computeClearChoices(ballot.contests, current)
        expect(current).toEqual(before)
    })

    it("counts an explicit invalid mark as a removed choice and clears it", () => {
        const invalidBallot = buildSlatesBallot()
        invalidBallot.contests[1].candidates.push({
            ...invalidBallot.contests[1].candidates[0],
            id: "s-invalid",
            presentation: {is_explicit_invalid: true},
        })
        const current = emptySelection(invalidBallot).map((contest) =>
            contest.contest_id === SECRETARY
                ? {...contest, is_explicit_invalid: true, invalid_errors: [{message: "x"}]}
                : contest
        ) as BallotSelection
        const result = computeClearChoices(invalidBallot.contests, current)
        const secretary = result.selection.find((contest) => contest.contest_id === SECRETARY)
        expect(secretary?.is_explicit_invalid).toBe(false)
        expect(secretary?.invalid_errors).toEqual([])
        expect(result.changes).toEqual([{contestId: SECRETARY, added: [], removed: ["s-invalid"]}])
    })

    it("drops write-in texts and leaves choices as the voting screen writes them", () => {
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
        const president = computeClearChoices(ballot.contests, current).selection.find(
            (contest) => contest.contest_id === PRESIDENT
        )
        expect(president?.choices).toEqual([
            {id: "p-forward", selected: -1},
            {id: "p-members", selected: -1},
            {id: "p-independent", selected: -1},
        ])
    })

    it("leaves an acclaimed contest as it is", () => {
        const contests = ballot.contests.map((contest) =>
            contest.id === SECRETARY ? {...contest, is_acclaimed: true} : contest
        )
        const current = select(emptySelection(ballot), SECRETARY, ["s-members"])
        const result = computeClearChoices(contests, current)
        expect(selectedIds(result.selection)).toEqual(["s-members"])
        expect(result.changes).toEqual([])
    })

    it("clears the blank ballot and decline flags", () => {
        const current = select(emptySelection(ballot), PRESIDENT, ["p-forward"]).map((contest) => ({
            ...contest,
            is_blank_ballot: true,
            is_decline_to_vote: true,
        }))
        const result = computeClearChoices(ballot.contests, current)
        expect(
            result.selection.every((entry) => !entry.is_blank_ballot && !entry.is_decline_to_vote)
        ).toBe(true)
    })
})
