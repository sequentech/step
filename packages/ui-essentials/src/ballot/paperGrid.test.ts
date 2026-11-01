// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ECandidatesLayout, IContest} from "@sequentech/ui-core"

import {aCandidate, aContest} from "../testing/ballot"
import {
    EPaperGridDensity,
    PAPER_GRID_DEFAULT_COLUMNS,
    getCandidatesLayout,
    getPaperGridColumns,
    getPaperGridDensity,
    getPaperGridRows,
    getPaperCandidateOrder,
    getResponsivePaperGridColumns,
} from "./paperGrid"

describe("saved paper candidate order", () => {
    const candidates = [aCandidate("a", "Alice"), aCandidate("b", "Bob")]
    beforeEach(() => sessionStorage.clear())
    afterEach(() => jest.restoreAllMocks())

    it("remembers only IDs and separates ballot styles and contests", () => {
        const shuffle = jest.fn(() => [...candidates].reverse())
        expect(getPaperCandidateOrder("style", "contest", candidates, shuffle)).toEqual(
            [...candidates].reverse()
        )
        expect(getPaperCandidateOrder("style", "contest", candidates, () => candidates)).toEqual(
            [...candidates].reverse()
        )
        expect(getPaperCandidateOrder("other", "contest", candidates, () => candidates)).toEqual(
            candidates
        )
        expect(getPaperCandidateOrder("style", "other", candidates, () => candidates)).toEqual(
            candidates
        )
        expect(JSON.parse(sessionStorage.getItem(sessionStorage.key(0)!)!)).toEqual(["b", "a"])
        expect(shuffle).toHaveBeenCalledTimes(1)
    })

    it.each(["broken JSON", '["a","a"]', '["missing","a"]', '["a"]', "[1,2]"])(
        "replaces an unusable saved order: %s",
        (saved) => {
            getPaperCandidateOrder("style", "contest", candidates, () => candidates)
            sessionStorage.setItem(sessionStorage.key(0)!, saved)
            const shuffle = jest.fn(() => [...candidates].reverse())
            expect(getPaperCandidateOrder("style", "contest", candidates, shuffle)).toEqual(
                [...candidates].reverse()
            )
            expect(shuffle).toHaveBeenCalledTimes(1)
        }
    )

    it("keeps the order across remounts when storage is unavailable", () => {
        jest.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
            throw new Error("blocked")
        })
        jest.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
            throw new Error("blocked")
        })
        expect(
            getPaperCandidateOrder("private-style", "contest", candidates, () =>
                [...candidates].reverse()
            )
        ).toEqual([...candidates].reverse())
        expect(
            getPaperCandidateOrder("private-style", "contest", candidates, () => candidates)
        ).toEqual([...candidates].reverse())
    })

    it("keeps the order when storage can be read but its quota is full", () => {
        getPaperCandidateOrder("quota-style", "contest", candidates, () => candidates)
        sessionStorage.setItem(sessionStorage.key(0)!, '["stale"]')
        jest.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
            throw new Error("quota exceeded")
        })
        getPaperCandidateOrder("quota-style", "contest", candidates, () =>
            [...candidates].reverse()
        )
        expect(
            getPaperCandidateOrder("quota-style", "contest", candidates, () => candidates)
        ).toEqual([...candidates].reverse())
    })
})

const withPresentation = (presentation: Record<string, unknown>): IContest =>
    aContest({presentation} as never)

describe("the candidates layout", () => {
    it("is the list when nothing was configured", () => {
        expect(getCandidatesLayout(aContest(), false)).toBe(ECandidatesLayout.LIST)
        expect(getCandidatesLayout(aContest({presentation: undefined} as never), false)).toBe(
            ECandidatesLayout.LIST
        )
    })

    it("is the paper grid when configured", () => {
        const contest = withPresentation({candidates_layout: ECandidatesLayout.PAPER_GRID})
        expect(getCandidatesLayout(contest, false)).toBe(ECandidatesLayout.PAPER_GRID)
    })

    it("stays the list for a ranked contest, which needs a position per candidate", () => {
        const contest = withPresentation({candidates_layout: ECandidatesLayout.PAPER_GRID})
        expect(getCandidatesLayout(contest, true)).toBe(ECandidatesLayout.LIST)
    })
})

describe("the paper grid's columns", () => {
    it("defaults to the printed ballot's four", () => {
        expect(getPaperGridColumns(aContest())).toBe(PAPER_GRID_DEFAULT_COLUMNS)
        expect(PAPER_GRID_DEFAULT_COLUMNS).toBe(4)
    })

    it("uses the contest's columns when set", () => {
        expect(getPaperGridColumns(withPresentation({columns: 3}))).toBe(3)
    })

    it.each([0, -2, Number.NaN])("ignores a nonsensical column count of %p", (columns) => {
        expect(getPaperGridColumns(withPresentation({columns}))).toBe(PAPER_GRID_DEFAULT_COLUMNS)
    })

    it("collapses to at most two on a phone and three on a tablet", () => {
        expect(getResponsivePaperGridColumns(4)).toEqual({small: 2, medium: 3, large: 4})
        expect(getResponsivePaperGridColumns(1)).toEqual({small: 1, medium: 1, large: 1})
    })
})

describe("the paper grid's rows", () => {
    it.each([
        [66, 4, 17],
        [203, 4, 51],
        [203, 2, 102],
        [3, 4, 1],
        [0, 4, 1],
    ])("%i candidates in %i columns take %i rows", (count, columns, rows) => {
        expect(getPaperGridRows(count, columns)).toBe(rows)
    })

    it.each([
        [8, 4, EPaperGridDensity.REGULAR],
        [48, 4, EPaperGridDensity.REGULAR],
        [66, 4, EPaperGridDensity.COMPACT],
        [96, 4, EPaperGridDensity.COMPACT],
        [203, 4, EPaperGridDensity.DENSE],
    ])("%i candidates in %i columns are drawn %s", (count, columns, density) => {
        expect(getPaperGridDensity(count, columns)).toBe(density)
    })
})
