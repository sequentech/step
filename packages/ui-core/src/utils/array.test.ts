// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {describe, expect, it} from "@jest/globals"
import {keyBy, shuffle, splitList} from "./array"

it("partitions every item once while preserving order within each side", () => {
    const input = [1, 2, 3, 4, 5]
    expect(splitList(input, (value) => value % 2 === 0)).toEqual([
        [1, 3, 5],
        [2, 4],
    ])
    expect(input).toEqual([1, 2, 3, 4, 5])
    expect(splitList([], () => true)).toEqual([[], []])
})

it("shuffles without losing duplicates or changing the input collection", () => {
    const input = ["Ada", "Ada", "Lee"]
    // A random shuffle is allowed to preserve order. Assert its invariant,
    // rather than a probabilistic expectation that eventually becomes flaky.
    expect(shuffle(input).sort()).toEqual(["Ada", "Ada", "Lee"])
    expect(input).toEqual(["Ada", "Ada", "Lee"])
})

describe("keyBy", () => {
    it("uses the last row for a repeated identifier", () => {
        const first = {id: "same", name: "before"}
        const last = {id: "same", name: "after"}
        expect(keyBy([first, last], "id")).toEqual({same: last})
        expect(keyBy([], "id")).toEqual({})
    })

    it.each(["__proto__", "constructor", "toString"])(
        "treats %s as an ordinary identifier, not an inherited property",
        (id) => {
            const row = {id, name: "candidate"}
            const indexed = keyBy([row], "id")
            expect(Object.hasOwn(indexed, id)).toBe(true)
            expect(indexed[id]).toBe(row)
        }
    )
})
