// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BallotSelection} from "@sequentech/ui-core"
import {marks} from "../testing/ballotHarness"
import {PRESIDENT, TRUSTEES} from "../testing/slateFixtures"
import {isSameBallotSelection} from "./BallotSelectionComparison"

const ballot = (president: string[], trustees: string[]): BallotSelection => [
    marks(PRESIDENT, Object.fromEntries(president.map((id) => [id, 0]))),
    marks(TRUSTEES, Object.fromEntries(trustees.map((id) => [id, 0]))),
]

describe("isSameBallotSelection", () => {
    it("holds for the same choices", () => {
        expect(
            isSameBallotSelection(
                ballot(["f-president"], ["f-t1"]),
                ballot(["f-president"], ["f-t1"])
            )
        ).toBe(true)
    })

    it("holds whatever the order of contests and choices", () => {
        const reordered = ballot(["f-president"], ["f-t1"]).reverse()
        reordered.forEach((contest) => contest.choices.reverse())

        expect(isSameBallotSelection(ballot(["f-president"], ["f-t1"]), reordered)).toBe(true)
    })

    it("treats a choice that is not listed as not selected", () => {
        const sparse = ballot(["f-president"], [])
        sparse[1].choices = []

        expect(isSameBallotSelection(ballot(["f-president"], []), sparse)).toBe(true)
    })

    it("treats a missing write-in text as an empty one", () => {
        const withoutText = ballot(["f-president"], [])
        withoutText[0].choices = withoutText[0].choices.map(({id, selected}) => ({id, selected}))

        expect(isSameBallotSelection(ballot(["f-president"], []), withoutText)).toBe(true)
    })

    it("fails when a candidate was replaced", () => {
        expect(
            isSameBallotSelection(ballot(["f-president"], []), ballot(["i-president"], []))
        ).toBe(false)
    })

    it("fails when a candidate was added or removed", () => {
        expect(isSameBallotSelection(ballot([], ["f-t1"]), ballot([], ["f-t1", "f-t2"]))).toBe(
            false
        )
        expect(isSameBallotSelection(ballot([], ["f-t1", "f-t2"]), ballot([], ["f-t1"]))).toBe(
            false
        )
    })

    it("fails when a rank changed", () => {
        const ranked = ballot([], ["f-t1"])
        ranked[1].choices[0].selected = 1

        expect(isSameBallotSelection(ballot([], ["f-t1"]), ranked)).toBe(false)
    })

    it("fails when a write-in text changed", () => {
        const written = ballot(["f-president"], [])
        written[0].choices[0].write_in_text = "Someone Else"

        expect(isSameBallotSelection(ballot(["f-president"], []), written)).toBe(false)
    })

    it.each(["is_explicit_invalid", "is_decline_to_vote", "is_blank_ballot"] as const)(
        "fails when %s changed",
        (flag) => {
            const flagged = ballot([], [])
            flagged[0][flag] = true

            expect(isSameBallotSelection(ballot([], []), flagged)).toBe(false)
        }
    )

    it("fails when a contest is missing", () => {
        expect(isSameBallotSelection(ballot([], []), ballot([], []).slice(0, 1))).toBe(false)
    })
})
