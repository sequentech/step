// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it} from "@jest/globals"
import {candidate, contest} from "../../tests/fixtures"
import {areAllContestsAcclaimed, isAcclaimedContest} from "./acclamation"
import {
    checkIsWriteIn,
    checkIsInvalidVote,
    checkIsCategoryList,
    checkIsExplicitBlankVote,
    checkIsDisabled,
    getImageUrl,
} from "./candidatePresentation"
import {formatPercentOne, numberToNDecimalPlaces} from "./percentFormatter"
import {normalizeWriteInText} from "./normalizeWriteInText"
import {toValidClassName} from "./cssClassNameFormatter"
import {sanitizeFilename, takeLastNChars, FOLDER_MAX_CHARS} from "./sanitizeFilename"

it("requires a nonempty list of acclaimed contests before skipping ballot encoding", () => {
    const acclaimed = contest({is_acclaimed: true})
    const selectable = contest({is_acclaimed: false})
    expect(isAcclaimedContest(acclaimed)).toBe(true)
    for (const absent of [undefined, null, selectable, contest()]) {
        expect(isAcclaimedContest(absent)).toBe(false)
    }
    expect(areAllContestsAcclaimed([acclaimed, acclaimed])).toBe(true)
    for (const incomplete of [undefined, null, [], [acclaimed, selectable]]) {
        expect(areAllContestsAcclaimed(incomplete)).toBe(false)
    }
})

it("defaults optional presentation flags to false and keeps them independent", () => {
    const cases = [
        ["is_write_in", checkIsWriteIn],
        ["is_explicit_invalid", checkIsInvalidVote],
        ["is_category_list", checkIsCategoryList],
        ["is_explicit_blank", checkIsExplicitBlankVote],
        ["is_disabled", checkIsDisabled],
    ] as const
    for (const [flag, check] of cases) {
        expect(check(candidate("option"))).toBe(false)
        expect(check(candidate("option", {presentation: {[flag]: false}}))).toBe(false)
        expect(check(candidate("option", {presentation: {[flag]: true}}))).toBe(true)
    }
})

it("selects the first explicitly marked image rather than the first candidate URL", () => {
    expect(getImageUrl(candidate("option"))).toBeUndefined()
    expect(getImageUrl(candidate("option", {presentation: {urls: []}}))).toBeUndefined()
    const urls = [
        {url: "https://example.org/profile", is_image: false},
        {url: "https://example.org/portrait.png", is_image: true},
        {url: "https://example.org/other.png", is_image: true},
    ]
    expect(getImageUrl(candidate("option", {presentation: {urls}}))).toBe(
        "https://example.org/portrait.png"
    )
})

it("formats fractions as percentages and groups decimal quantities consistently", () => {
    expect(formatPercentOne(0)).toBe("0.00%")
    expect(formatPercentOne(0.12345)).toBe("12.35%")
    expect(formatPercentOne(1)).toBe("100.00%")
    expect(numberToNDecimalPlaces(1234.5, 2)).toBe("1,234.50")
    expect(numberToNDecimalPlaces(-1.25, 1)).toBe("-1.3")
})

it("normalizes composed and decomposed write-ins to the same ballot alphabet", () => {
    const expected = "JOSE, ANA (JR.) "
    for (const input of ["José, Ana (Jr.) ~123!", "Jose\u0301, Ana (Jr.) ~123!"]) {
        expect(normalizeWriteInText(input)).toBe(expected)
    }
    expect(normalizeWriteInText("")).toBe("")
})

it("creates bounded CSS identifiers without accepting selector punctuation", () => {
    expect(toValidClassName(" 9 council / [x] # . ")).toBe("e-9councilx")
    expect(toValidClassName("a-b_c")).toBe("e-a-b_c")
    expect(toValidClassName("")).toBe("e-")
    expect(toValidClassName("a".repeat(80))).toBe(`e-${"a".repeat(38)}`)
})

it("removes forbidden filename characters and keeps the suffix within the length budget", () => {
    expect(sanitizeFilename('report<>:"/\\|?*\u0000\u001f.csv . ')).toBe("report.csv")
    expect(sanitizeFilename("abcdef.csv", 7)).toBe("def.csv")
    expect(sanitizeFilename("a".repeat(250))).toHaveLength(FOLDER_MAX_CHARS)
    expect(takeLastNChars("abc", 10)).toBe("abc")
    expect(takeLastNChars("abc", 2)).toBe("bc")
})
