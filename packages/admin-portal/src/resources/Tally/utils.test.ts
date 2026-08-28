// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {orderItemsByIds} from "./utils"

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
