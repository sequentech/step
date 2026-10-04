// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EInvalidVotePolicy,
    EOverVotePolicy,
    IContest,
    IDecodedVoteContest,
} from "@sequentech/ui-core"
import {
    buildLimitedSlatesBallot,
    buildSlatesBallot,
    PRESIDENT,
    TRUSTEES,
} from "../fixtures/slateChoices"
import {
    countSelections,
    exceedsMaximum,
    refusesOverVotes,
    replacesOnSelect,
} from "./SelectionLimits"

const contestOf = (id: string, limited = true): IContest => {
    const ballot = limited ? buildLimitedSlatesBallot() : buildSlatesBallot()
    const contest = ballot.contests.find((entry) => entry.id === id)
    if (!contest) {
        throw new Error(`missing contest ${id}`)
    }
    return contest
}

const selectionOf = (contest: IContest, selectedIds: string[]): IDecodedVoteContest => ({
    contest_id: contest.id,
    is_explicit_invalid: false,
    is_decline_to_vote: false,
    is_blank_ballot: false,
    invalid_errors: [],
    invalid_alerts: [],
    choices: contest.candidates.map((candidate) => ({
        id: candidate.id,
        selected: selectedIds.includes(candidate.id) ? 0 : -1,
    })),
})

describe("replacesOnSelect", () => {
    it("is true for a single-seat contest with radio selection", () => {
        expect(replacesOnSelect(contestOf(PRESIDENT))).toBe(true)
    })

    it("is false for a contest with several seats", () => {
        expect(replacesOnSelect(contestOf(TRUSTEES))).toBe(false)
    })

    it("is false for a single-seat contest without radio selection", () => {
        expect(replacesOnSelect(contestOf(PRESIDENT, false))).toBe(false)
    })
})

describe("refusesOverVotes", () => {
    it("is true when the contest disables further choices at its maximum", () => {
        expect(refusesOverVotes(contestOf(TRUSTEES))).toBe(true)
    })

    it.each([
        EOverVotePolicy.ALLOWED,
        EOverVotePolicy.ALLOWED_WITH_MSG,
        EOverVotePolicy.ALLOWED_WITH_MSG_AND_ALERT,
        EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_ALERT,
        undefined,
    ])("is false for the over-vote policy %s", (policy) => {
        const contest = contestOf(TRUSTEES)
        expect(
            refusesOverVotes({...contest, presentation: {over_vote_policy: policy}} as IContest)
        ).toBe(false)
    })
})

describe("countSelections", () => {
    it("counts the selected candidates", () => {
        const contest = contestOf(TRUSTEES)
        expect(countSelections(contest, selectionOf(contest, []))).toBe(0)
        expect(countSelections(contest, selectionOf(contest, ["t-forward-1", "t-voices-2"]))).toBe(
            2
        )
    })

    it("counts an explicit invalid mark as one selection", () => {
        const contest = contestOf(TRUSTEES)
        expect(
            countSelections(contest, {
                ...selectionOf(contest, ["t-forward-1"]),
                is_explicit_invalid: true,
            })
        ).toBe(2)
    })

    it("leaves out an invalid mark that the next candidate would clear", () => {
        const contest = contestOf(TRUSTEES)
        const exclusive = {
            ...contest,
            presentation: {
                ...contest.presentation,
                invalid_vote_policy: EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT,
            },
        } as IContest
        expect(
            countSelections(exclusive, {
                ...selectionOf(exclusive, ["t-forward-1"]),
                is_explicit_invalid: true,
            })
        ).toBe(1)
    })

    it("leaves out the explicit blank option", () => {
        const contest = contestOf(TRUSTEES)
        const withBlank = {
            ...contest,
            candidates: contest.candidates.map((candidate) =>
                candidate.id === "t-independent"
                    ? {...candidate, presentation: {is_explicit_blank: true}}
                    : candidate
            ),
        } as IContest
        expect(
            countSelections(withBlank, selectionOf(withBlank, ["t-independent", "t-forward-1"]))
        ).toBe(1)
    })
})

describe("exceedsMaximum", () => {
    it("allows one candidate for a single-seat office and refuses two", () => {
        const contest = contestOf(PRESIDENT)
        expect(exceedsMaximum(contest, 1)).toBe(false)
        expect(exceedsMaximum(contest, 2)).toBe(true)
    })

    it("allows three trustees and refuses four", () => {
        const contest = contestOf(TRUSTEES)
        expect(exceedsMaximum(contest, 3)).toBe(false)
        expect(exceedsMaximum(contest, 4)).toBe(true)
    })
})
