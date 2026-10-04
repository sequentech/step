// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    DEFAULT_NUMBER_FORMAT_POLICY,
    formatNumber,
    formatPercentage,
    resolveNumberFormatPolicy,
} from "./numberFormat"
import {ENumberFormatPolicy} from "../types/ElectionEventPresentation"

describe("number format policy", () => {
    it("writes each policy's sample", () => {
        const samples = Object.values(ENumberFormatPolicy).map((policy) =>
            formatNumber(1234567.89, policy, 2)
        )
        expect(samples).toEqual([
            "1,234,567.89",
            "1.234.567,89",
            "1 234 567,89",
            "1 234 567.89",
            "1’234’567.89",
        ])
    })

    it("uses comma grouping for events saved before the policy existed", () => {
        expect(DEFAULT_NUMBER_FORMAT_POLICY).toBe(ENumberFormatPolicy.COMMA_PERIOD)
        expect(resolveNumberFormatPolicy(undefined)).toBe(ENumberFormatPolicy.COMMA_PERIOD)
        expect(resolveNumberFormatPolicy(null)).toBe(ENumberFormatPolicy.COMMA_PERIOD)
        expect(resolveNumberFormatPolicy("unknown")).toBe(ENumberFormatPolicy.COMMA_PERIOD)
        expect(formatNumber(12000000)).toBe("12,000,000")
        expect(formatNumber(12000000, "unknown")).toBe("12,000,000")
    })

    it("groups integers in thousands", () => {
        const policy = ENumberFormatPolicy.PERIOD_COMMA
        expect(formatNumber(0, policy)).toBe("0")
        expect(formatNumber(999, policy)).toBe("999")
        expect(formatNumber(1000, policy)).toBe("1.000")
        expect(formatNumber(8589934591, policy)).toBe("8.589.934.591")
        expect(formatNumber(-1234, policy)).toBe("-1.234")
        expect(formatNumber("0042", policy)).toBe("42")
    })

    it("keeps counts beyond 2^53 exact when they arrive as text or bigints", () => {
        expect(formatNumber("18446744073709551615")).toBe("18,446,744,073,709,551,615")
        expect(formatNumber(BigInt("9007199254740993"))).toBe("9,007,199,254,740,993")
    })

    it("rounds decimals and uses the policy's decimal separator", () => {
        expect(formatNumber(1234.5, ENumberFormatPolicy.SPACE_COMMA, 2)).toBe("1 234,50")
        expect(formatNumber(99.999, ENumberFormatPolicy.PERIOD_COMMA, 2)).toBe("100,00")
        expect(formatNumber("12.3456", ENumberFormatPolicy.COMMA_PERIOD, 1)).toBe("12.3")
        expect(formatNumber(-0.001, ENumberFormatPolicy.COMMA_PERIOD, 2)).toBe("0.00")
    })

    it("returns values that are not numbers unchanged", () => {
        expect(formatNumber("-")).toBe("-")
        expect(formatNumber("")).toBe("")
        expect(formatNumber(null)).toBe("")
        expect(formatNumber(undefined)).toBe("")
        expect(formatNumber(Number.NaN)).toBe("NaN")
    })

    it("writes percentages with two decimals by default", () => {
        expect(formatPercentage(45.678, ENumberFormatPolicy.PERIOD_COMMA)).toBe("45,68%")
        expect(formatPercentage(100, ENumberFormatPolicy.COMMA_PERIOD, 1)).toBe("100.0%")
    })
})
