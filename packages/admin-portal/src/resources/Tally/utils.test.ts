// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ITallyExecutionStatus, ITallyTrusteeStatus} from "@/types/ceremonies"
import {canTrusteeRestorePrivateKey, orderItemsByIds} from "./utils"

describe("canTrusteeRestorePrivateKey", () => {
    it.each([ITallyExecutionStatus.STARTED, ITallyExecutionStatus.CONNECTED])(
        "allows a waiting trustee while the tally accepts keys (%s)",
        (executionStatus) => {
            expect(canTrusteeRestorePrivateKey(ITallyTrusteeStatus.WAITING, executionStatus)).toBe(
                true
            )
        }
    )

    it.each([
        ITallyExecutionStatus.NOT_STARTED,
        ITallyExecutionStatus.IN_PROGRESS,
        ITallyExecutionStatus.AWAITING_INPUT,
        ITallyExecutionStatus.SUCCESS,
        ITallyExecutionStatus.CANCELLED,
    ])("does not allow key restoration while the tally is %s", (executionStatus) => {
        expect(canTrusteeRestorePrivateKey(ITallyTrusteeStatus.WAITING, executionStatus)).toBe(
            false
        )
    })

    it("does not allow a trustee whose key is already restored", () => {
        expect(
            canTrusteeRestorePrivateKey(
                ITallyTrusteeStatus.KEY_RESTORED,
                ITallyExecutionStatus.STARTED
            )
        ).toBe(false)
    })

    it("does not allow a user who is absent from the tally ceremony", () => {
        expect(canTrusteeRestorePrivateKey(null, ITallyExecutionStatus.STARTED)).toBe(false)
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
