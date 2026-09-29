// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    copyId,
    followedSelectors,
    freshName,
    humanize,
    selectorRef,
    toggleFollowed,
} from "./formValues"
import {EMonitoringScopeSelector} from "./types"

describe("selectorRef", () => {
    it("reads only a lone `selector` key as a reference", () => {
        expect(selectorRef({selector: "breakdown"})).toBe("breakdown")
        expect(selectorRef({selector: "a", other: 1})).toBeUndefined()
        expect(selectorRef(["voted"])).toBeUndefined()
        expect(selectorRef("sex")).toBeUndefined()
    })
})

describe("followed selectors", () => {
    it("reads an absent list as every selector", () => {
        expect(followedSelectors({})).toEqual(["region", "post", "country"])
        expect(followedSelectors({follows: []})).toEqual([])
    })

    it("writes the list only while it is not all of them", () => {
        expect(toggleFollowed({}, EMonitoringScopeSelector.POST, false)).toEqual([
            "region",
            "country",
        ])
        expect(
            toggleFollowed({follows: ["region", "country"]}, EMonitoringScopeSelector.POST, true)
        ).toBeUndefined()
    })
})

describe("names", () => {
    it("finds the first free copy id", () => {
        expect(copyId("w", new Set(["w"]))).toBe("w-copy")
        expect(copyId("w", new Set(["w", "w-copy", "w-copy-2"]))).toBe("w-copy-3")
    })

    it("finds the first free selector name", () => {
        expect(freshName("selector", ["selector_1"])).toBe("selector_2")
    })

    it("humanizes identifiers", () => {
        expect(humanize("pre_enrolled")).toBe("Pre enrolled")
        expect(humanize("")).toBe("")
    })
})
