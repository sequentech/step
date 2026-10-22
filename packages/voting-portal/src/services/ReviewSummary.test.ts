// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BallotSelection, IContest} from "@sequentech/ui-core"
import {aCandidate, aContest, marks} from "../testing/ballotHarness"
import {PRESIDENT, SLATES, TRUSTEES} from "../testing/slateFixtures"
import {
    EBallotSelectionLabel,
    getBallotReviewSummary,
    getBallotSelectionCount,
    getBallotSelectionLabel,
} from "./ReviewSummary"
import {resolveSlates} from "./Slates"

const CONTESTS = [PRESIDENT, TRUSTEES]
const RESOLVED = resolveSlates(SLATES, CONTESTS)

const ballot = (president: string[], trustees: string[]): BallotSelection => [
    marks(PRESIDENT, Object.fromEntries(president.map((id) => [id, 0]))),
    marks(TRUSTEES, Object.fromEntries(trustees.map((id) => [id, 0]))),
]

const labelOf = (selection: BallotSelection) => {
    const {label} = getBallotReviewSummary(CONTESTS, RESOLVED, selection)
    return [label.kind, label.slate?.id]
}

describe("getBallotReviewSummary", () => {
    it("counts the selected candidates of every contest against its maximum", () => {
        const summary = getBallotReviewSummary(
            CONTESTS,
            RESOLVED,
            ballot(["f-president"], ["f-t1", "i-t1"])
        )

        expect(summary.contests).toEqual({
            president: {selected: 1, max: 1},
            trustees: {selected: 2, max: 3},
        })
        expect(summary.selected).toBe(3)
        expect(summary.seats).toBe(4)
    })

    it("names the slate the ballot holds exactly", () => {
        expect(labelOf(ballot(["f-president"], ["f-t1", "f-t2"]))).toEqual([
            EBallotSelectionLabel.SLATE,
            "forward",
        ])
    })

    it("reads a slate changed by hand as individual selections", () => {
        expect(labelOf(ballot(["i-president"], ["f-t1", "f-t2"]))).toEqual([
            EBallotSelectionLabel.INDIVIDUAL,
            undefined,
        ])
    })

    it("reads a slate with a member removed as individual selections", () => {
        expect(labelOf(ballot(["f-president"], ["f-t1"]))).toEqual([
            EBallotSelectionLabel.INDIVIDUAL,
            undefined,
        ])
    })

    it("reports an entirely blank ballot", () => {
        expect(getBallotReviewSummary(CONTESTS, RESOLVED, ballot([], []))).toEqual({
            selected: 0,
            seats: 4,
            contests: {president: {selected: 0, max: 1}, trustees: {selected: 0, max: 3}},
            label: {kind: EBallotSelectionLabel.NONE},
        })
    })

    it("reports nothing selected when there is no ballot selection", () => {
        const summary = getBallotReviewSummary(CONTESTS, RESOLVED, undefined)

        expect(summary.selected).toBe(0)
        expect(summary.contests.trustees).toEqual({selected: 0, max: 3})
    })

    it("does not count blank or invalid options as candidates", () => {
        const contest: IContest = aContest({
            id: "auditor",
            candidates: [
                aCandidate("a", "Alice Okonjo", {contest_id: "auditor"}),
                aCandidate("blank", "Blank", {
                    contest_id: "auditor",
                    presentation: {is_explicit_blank: true},
                }),
                aCandidate("invalid", "Invalid", {
                    contest_id: "auditor",
                    presentation: {is_explicit_invalid: true},
                }),
            ],
        })

        const summary = getBallotReviewSummary([contest], RESOLVED, [
            marks(contest, {blank: 0, invalid: 0}),
        ])

        expect(summary.contests.auditor).toEqual({selected: 0, max: 1})
    })

    it("ignores a choice that is not a candidate of the contest", () => {
        const selection = ballot(["f-president"], [])
        selection[0].choices.push({id: "unknown", selected: 0})

        expect(getBallotReviewSummary(CONTESTS, RESOLVED, selection).selected).toBe(1)
    })

    it("leaves acclaimed contests out, as nothing is selected in them", () => {
        const acclaimed: IContest = {...TRUSTEES, is_acclaimed: true}

        const summary = getBallotReviewSummary(
            [PRESIDENT, acclaimed],
            RESOLVED,
            ballot(["f-president"], [])
        )

        expect(summary.seats).toBe(1)
        expect(summary.contests).toEqual({president: {selected: 1, max: 1}})
    })
})

describe("getBallotSelectionCount", () => {
    it("counts the selected candidates against the seats of the ballot", () => {
        expect(
            getBallotSelectionCount(CONTESTS, ballot(["f-president"], ["f-t1", "i-t1"]))
        ).toEqual({selected: 3, seats: 4})
    })

    it("counts nothing for a ballot without selections yet", () => {
        expect(getBallotSelectionCount(CONTESTS, undefined)).toEqual({selected: 0, seats: 4})
    })

    it("leaves acclaimed contests out", () => {
        const acclaimed = {...PRESIDENT, is_acclaimed: true}
        expect(getBallotSelectionCount([acclaimed, TRUSTEES], ballot([], ["f-t1"]))).toEqual({
            selected: 1,
            seats: 3,
        })
    })
})

describe("getBallotSelectionLabel", () => {
    it("names the slate the ballot holds exactly", () => {
        const label = getBallotSelectionLabel(RESOLVED, ballot(["f-president"], ["f-t1", "f-t2"]))
        expect(label.kind).toBe(EBallotSelectionLabel.SLATE)
        expect(label.slate?.id).toBe("forward")
    })

    it("speaks of individual selections otherwise", () => {
        expect(getBallotSelectionLabel(RESOLVED, ballot(["i-president"], ["f-t1"])).kind).toBe(
            EBallotSelectionLabel.INDIVIDUAL
        )
    })

    it("says nothing is selected on an empty ballot", () => {
        expect(getBallotSelectionLabel(RESOLVED, ballot([], [])).kind).toBe(
            EBallotSelectionLabel.NONE
        )
    })
})
