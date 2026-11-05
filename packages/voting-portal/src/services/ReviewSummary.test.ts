// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BallotSelection, IContest} from "@sequentech/ui-core"
import {aCandidate, aContest, marks} from "../testing/ballotHarness"
import {PRESIDENT, SLATES, TRUSTEES} from "../testing/slateFixtures"
import {getBallotReviewSummary} from "./ReviewSummary"
import {ESlateSelectionStatus} from "./SlateSelection"
import {resolveSlates} from "./Slates"

const CONTESTS = [PRESIDENT, TRUSTEES]
const RESOLVED = resolveSlates(SLATES, CONTESTS)

const ballot = (president: string[], trustees: string[]): BallotSelection => [
    marks(PRESIDENT, Object.fromEntries(president.map((id) => [id, 0]))),
    marks(TRUSTEES, Object.fromEntries(trustees.map((id) => [id, 0]))),
]

const slateLines = (selection: BallotSelection) =>
    getBallotReviewSummary(CONTESTS, RESOLVED, selection).slates.map(({slate, summary}) => [
        slate.id,
        summary.status,
        summary.selected,
        summary.total,
    ])

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

    it("reads a fully selected slate as all selected", () => {
        expect(slateLines(ballot(["f-president"], ["f-t1", "f-t2"]))).toEqual([
            ["forward", ESlateSelectionStatus.ALL, 3, 3],
        ])
    })

    it("reads a slate changed by hand as mixed, with the replacement counted as independent", () => {
        const selection = ballot(["i-president"], ["f-t1", "f-t2"])

        expect(slateLines(selection)).toEqual([["forward", ESlateSelectionStatus.MIXED, 2, 3]])
        expect(getBallotReviewSummary(CONTESTS, RESOLVED, selection).independent).toBe(1)
    })

    it("reads a slate with a member removed as partly selected", () => {
        expect(slateLines(ballot(["f-president"], ["f-t1"]))).toEqual([
            ["forward", ESlateSelectionStatus.PARTLY, 2, 3],
        ])
    })

    it("lists every slate with a selected member, in the configured order", () => {
        expect(slateLines(ballot([], ["v-t1", "m-t1", "f-t1"]))).toEqual([
            ["forward", ESlateSelectionStatus.MIXED, 1, 3],
            ["members", ESlateSelectionStatus.MIXED, 1, 2],
            ["voices", ESlateSelectionStatus.MIXED, 1, 1],
        ])
    })

    it("reports an entirely blank ballot", () => {
        expect(getBallotReviewSummary(CONTESTS, RESOLVED, ballot([], []))).toEqual({
            selected: 0,
            seats: 4,
            contests: {president: {selected: 0, max: 1}, trustees: {selected: 0, max: 3}},
            slates: [],
            independent: 0,
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
        expect(summary.independent).toBe(0)
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
