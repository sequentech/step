// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it, jest, afterEach} from "@jest/globals"
import {candidate, contest, decodedContest} from "../../tests/fixtures"
import * as arrays from "../utils/array"
import {
    categorizeCandidates,
    getShuffledCategories,
    sortCategoryEntries,
    isChoiceSelected,
    showCategoryOnReview,
    isCategoryListSelected,
    shouldShowCategoryCandidateOnReview,
    type CategoriesMap,
} from "./categoryService"

afterEach(() => {
    jest.restoreAllMocks()
})

it("separates special choices, category headers and independent candidates", () => {
    const independent = candidate("independent")
    const member = candidate("member", {candidate_type: "council"})
    const header = candidate("header", {
        candidate_type: "council",
        presentation: {is_category_list: true},
    })
    const blank = candidate("blank", {presentation: {is_explicit_blank: true}})
    const invalid = candidate("invalid", {presentation: {is_explicit_invalid: true}})
    const disabled = candidate("disabled", {presentation: {is_disabled: true}})
    const result = categorizeCandidates(
        contest({
            candidates: [independent, member, blank, header, invalid, disabled],
        })
    )

    expect(result.noCategoryCandidates).toEqual([independent])
    expect(result.invalidOrBlankCandidates).toEqual([blank, invalid])
    expect(result.categoriesMap).toEqual({council: {header, candidates: [member]}})
})

it.each(["__proto__", "constructor", "toString"])(
    "keeps the authored category %s separate from JavaScript's object prototype",
    (categoryName) => {
        const member = candidate("member", {candidate_type: categoryName})
        const header = candidate("header", {
            candidate_type: categoryName,
            presentation: {is_category_list: true},
        })
        const inherited = Reflect.get({}, categoryName) as object
        const previousHeader = Object.getOwnPropertyDescriptor(inherited, "header")
        try {
            const grouped = categorizeCandidates(contest({candidates: [header, member]}))
            expect(Object.hasOwn(grouped.categoriesMap, categoryName)).toBe(true)
            expect(grouped.categoriesMap[categoryName]).toEqual({header, candidates: [member]})
            const reordered = getShuffledCategories(grouped.categoriesMap, false, false, [])
            expect(Object.hasOwn(reordered, categoryName)).toBe(true)
            expect(reordered[categoryName]).toEqual({header, candidates: [member]})
            expect(Object.getOwnPropertyDescriptor(inherited, "header")).toEqual(previousHeader)
        } finally {
            // Keep the red regression isolated even if the old implementation
            // writes a header onto Object.prototype before throwing.
            if (previousHeader) Object.defineProperty(inherited, "header", previousHeader)
            else Reflect.deleteProperty(inherited, "header")
        }
    }
)

it("honors configured category order and leaves unspecified categories at order zero", () => {
    const categories: CategoriesMap = {
        later: {candidates: []},
        default: {candidates: []},
        earlier: {candidates: []},
    }
    const presentation = {later: {sort_order: 2}, earlier: {sort_order: -1}}
    expect(Object.keys(getShuffledCategories(categories, false, false, [], presentation))).toEqual([
        "earlier",
        "default",
        "later",
    ])
    expect(sortCategoryEntries(categories, presentation).map(([name]) => name)).toEqual([
        "earlier",
        "default",
        "later",
    ])
    expect(sortCategoryEntries(categories).map(([name]) => name)).toEqual([
        "later",
        "default",
        "earlier",
    ])
})

it("shuffles only the configured levels, preserving every candidate and header", () => {
    // A deterministic shuffle lets this test detect an omitted or extra shuffle
    // without depending on a particular random outcome.
    jest.spyOn(arrays, "shuffle").mockImplementation((items) => [...items].reverse())
    const first = candidate("first")
    const second = candidate("second")
    const header = candidate("header")
    const makeCategories = (): CategoriesMap => ({
        alpha: {header, candidates: [first, second]},
        beta: {candidates: [second, first]},
    })
    const selected = getShuffledCategories(makeCategories(), false, true, ["alpha"])
    expect(Object.keys(selected)).toEqual(["beta", "alpha"])
    expect(selected.alpha).toEqual({header, candidates: [second, first]})
    expect(selected.beta.candidates).toEqual([second, first])
    const all = getShuffledCategories(makeCategories(), true, false, [])
    expect(all.alpha.candidates).toEqual([second, first])
    expect(all.beta.candidates).toEqual([first, second])
})

it("recognizes a zero-ranked selection in both wire arrays and indexed choices", () => {
    const choices = [
        {id: "selected", selected: 0},
        {id: "absent", selected: -1},
    ]
    for (const representation of [
        choices,
        Object.fromEntries(choices.map((choice) => [choice.id, choice])),
    ]) {
        expect(isChoiceSelected(representation, "selected")).toBe(true)
        expect(isChoiceSelected(representation, "absent")).toBe(false)
        expect(isChoiceSelected(representation, "unknown")).toBe(false)
    }
})

it("shows a review category when its list or a member was selected", () => {
    const category = {header: candidate("list"), candidates: [candidate("member")]}
    const selectedList = [{id: "list", selected: 0}]
    const selectedMember = [{id: "member", selected: 0}]
    const unselected = [{id: "member", selected: -1}]
    expect(showCategoryOnReview(category)).toBe(false)
    expect(showCategoryOnReview(category, decodedContest({choices: selectedList}))).toBe(true)
    expect(
        showCategoryOnReview(
            {candidates: category.candidates},
            decodedContest({choices: selectedMember})
        )
    ).toBe(true)
    expect(showCategoryOnReview(category, decodedContest({choices: unselected}))).toBe(false)
    expect(isCategoryListSelected(category, selectedList)).toBe(true)
    expect(isCategoryListSelected({candidates: []}, selectedList)).toBe(false)
    expect(shouldShowCategoryCandidateOnReview(category, "member", selectedList)).toBe(true)
    expect(shouldShowCategoryCandidateOnReview(category, "member", selectedMember)).toBe(true)
    expect(shouldShowCategoryCandidateOnReview(category, "member", unselected)).toBe(false)
})
