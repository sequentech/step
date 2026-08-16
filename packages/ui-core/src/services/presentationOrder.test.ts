// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {describe, expect, it} from "@jest/globals"
import {ElectionsOrder} from "../types/ElectionEventPresentation"
import {sortByPresentationOrder} from "./presentationOrder"

interface Item {
    id: string
    name: string
    presentation?: unknown
}

const accessors = {
    getLabel: (item: Item) => item.name,
    getPresentation: (item: Item) => item.presentation,
}

describe("sortByPresentationOrder", () => {
    it("uses serialized custom sort orders without mutating the source", () => {
        const source: Item[] = [
            {id: "a", name: "A", presentation: '{"sort_order":2}'},
            {id: "z", name: "Z", presentation: {sort_order: 1}},
        ]

        const sorted = sortByPresentationOrder(source, ElectionsOrder.CUSTOM, accessors)

        expect(sorted.map((item) => item.id)).toEqual(["z", "a"])
        expect(source.map((item) => item.id)).toEqual(["a", "z"])
    })

    it("uses canonical case-insensitive alphabetical order by default", () => {
        const source: Item[] = [
            {id: "z", name: "Zulu"},
            {id: "a", name: "alpha"},
        ]

        expect(
            sortByPresentationOrder(source, undefined, accessors).map((item) => item.id)
        ).toEqual(["a", "z"])
    })

    it("preserves source order when custom positions are equal", () => {
        const source: Item[] = [
            {id: "b", name: "Same", presentation: {sort_order: 1}},
            {id: "a", name: "Same", presentation: {sort_order: 1}},
        ]

        expect(
            sortByPresentationOrder(source, ElectionsOrder.CUSTOM, accessors).map((item) => item.id)
        ).toEqual(["b", "a"])
    })

    it("preserves a published random snapshot", () => {
        const source: Item[] = [
            {id: "z", name: "Zulu"},
            {id: "a", name: "Alpha"},
        ]

        expect(
            sortByPresentationOrder(source, ElectionsOrder.RANDOM, accessors).map((item) => item.id)
        ).toEqual(["z", "a"])
    })
})
