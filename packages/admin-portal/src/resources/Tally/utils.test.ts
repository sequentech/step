// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {
    ETallyKeyRestoreEligibility,
    ITallyExecutionStatus,
    ITallyTrusteeStatus,
} from "@/types/ceremonies"
import {formatTiedVoteShare, getTallyKeyRestoreEligibility, orderItemsByIds} from "./utils"

describe("getTallyKeyRestoreEligibility", () => {
    it.each([ITallyExecutionStatus.STARTED, ITallyExecutionStatus.CONNECTED])(
        "allows a waiting trustee while the tally accepts keys (%s)",
        (executionStatus) => {
            expect(
                getTallyKeyRestoreEligibility(ITallyTrusteeStatus.WAITING, executionStatus)
            ).toBe(ETallyKeyRestoreEligibility.ALLOWED)
        }
    )

    it.each([
        ITallyExecutionStatus.NOT_STARTED,
        ITallyExecutionStatus.IN_PROGRESS,
        ITallyExecutionStatus.AWAITING_INPUT,
        ITallyExecutionStatus.SUCCESS,
        ITallyExecutionStatus.CANCELLED,
    ])("denies key restoration while the tally is %s", (executionStatus) => {
        expect(getTallyKeyRestoreEligibility(ITallyTrusteeStatus.WAITING, executionStatus)).toBe(
            ETallyKeyRestoreEligibility.DENIED
        )
    })

    it.each([undefined, null])(
        "denies key restoration while the tally status is unknown (%s)",
        (executionStatus) => {
            expect(
                getTallyKeyRestoreEligibility(ITallyTrusteeStatus.WAITING, executionStatus)
            ).toBe(ETallyKeyRestoreEligibility.DENIED)
        }
    )

    it("denies a trustee whose key is already restored", () => {
        expect(
            getTallyKeyRestoreEligibility(
                ITallyTrusteeStatus.KEY_RESTORED,
                ITallyExecutionStatus.STARTED
            )
        ).toBe(ETallyKeyRestoreEligibility.DENIED)
    })

    it("denies a user who is absent from the tally ceremony", () => {
        expect(getTallyKeyRestoreEligibility(null, ITallyExecutionStatus.STARTED)).toBe(
            ETallyKeyRestoreEligibility.DENIED
        )
    })
})

describe("orderItemsByIds", () => {
    const items = [
        {id: "election-1", name: "Election 1"},
        {id: "election-2", name: "Election 2"},
    ]

    it("uses the supplied snapshot order without mutating the source", () => {
        expect(orderItemsByIds(items, ["election-2", "election-1"]).map((item) => item.id)).toEqual(
            ["election-2", "election-1"]
        )
        expect(items.map((item) => item.id)).toEqual(["election-1", "election-2"])
    })

    it("returns an empty selection when snapshot IDs are unavailable", () => {
        expect(orderItemsByIds(items, [])).toEqual([])
    })

    it("ignores unknown and duplicate snapshot IDs", () => {
        expect(
            orderItemsByIds(items, [
                "election-2",
                "unknown-election",
                "election-2",
                "election-1",
            ]).map((item) => item.id)
        ).toEqual(["election-2", "election-1"])
    })
})

describe("formatTiedVoteShare", () => {
    it("writes the tied candidates' share of the votes with one decimal", () => {
        expect(formatTiedVoteShare(1, 3, ENumberFormatPolicy.PERIOD_COMMA)).toBe("33,3")
        expect(formatTiedVoteShare(2500, 5000, ENumberFormatPolicy.COMMA_PERIOD)).toBe("50.0")
    })

    it("writes a zero share when the contest counted no votes", () => {
        expect(formatTiedVoteShare(0, 0, ENumberFormatPolicy.SPACE_COMMA)).toBe("0,0")
    })

    it("has no share while the contest's total is unknown", () => {
        expect(formatTiedVoteShare(3, undefined, ENumberFormatPolicy.COMMA_PERIOD)).toBeUndefined()
    })
})
