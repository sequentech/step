// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {EColumnKind} from "../types"
import {
    UNDEFINED_VALUE,
    formatCell,
    formatCompact,
    formatDateTime,
    formatInteger,
    formatRatio,
} from "./format"

describe("format", () => {
    it("groups integers and shows a dash for a missing value", () => {
        expect(formatInteger(874624)).toBe("874,624")
        expect(formatInteger(null)).toBe(UNDEFINED_VALUE)
        expect(formatInteger(undefined)).toBe(UNDEFINED_VALUE)
        expect(formatInteger(Number.NaN)).toBe(UNDEFINED_VALUE)
    })

    it("shows a ratio as a percentage with one decimal", () => {
        expect(formatRatio(0.5323)).toBe("53.2%")
        expect(formatRatio(1 / 16)).toBe("6.3%")
        expect(formatRatio(null)).toBe(UNDEFINED_VALUE)
    })

    it("never claims all or none when it is not, as sequent-core's percent_label", () => {
        expect(formatRatio(9996 / 10000)).toBe("99.9%")
        expect(formatRatio(99999 / 100000)).toBe("99.9%")
        expect(formatRatio(3 / 10000)).toBe("0.1%")
        expect(formatRatio(1)).toBe("100.0%")
        expect(formatRatio(0)).toBe("0.0%")
        expect(formatRatio(5 / 4)).toBe("125.0%")
    })

    it("shortens large numbers for charts", () => {
        expect(formatCompact(647000, "en")).toBe("647K")
    })

    it("formats a table cell by its column kind, keeping exact values", () => {
        expect(formatCell(1234, EColumnKind.INTEGER)).toBe("1,234")
        expect(formatCell(0.12345, EColumnKind.NUMBER)).toBe("12.345%")
        expect(formatCell(null, EColumnKind.NUMBER)).toBe(UNDEFINED_VALUE)
        expect(formatCell("—", EColumnKind.NUMBER)).toBe(UNDEFINED_VALUE)
        expect(formatCell("Male", EColumnKind.TEXT)).toBe("Male")
        expect(formatCell({a: 1}, EColumnKind.TEXT)).toBe('{"a":1}')
    })

    it("shows a table's ratios with more digits, and still never all or none when it is not", () => {
        expect(formatCell(9996 / 10000, EColumnKind.NUMBER)).toBe("99.96%")
        expect(formatCell(99999 / 100000, EColumnKind.NUMBER)).toBe("99.999%")
        expect(formatCell(3 / 10000, EColumnKind.NUMBER)).toBe("0.03%")
        expect(formatCell(9999999 / 10000000, EColumnKind.NUMBER)).toBe("99.9999%")
        expect(formatCell(1 / 10000000, EColumnKind.NUMBER)).toBe("0.0001%")
        expect(formatCell(1, EColumnKind.NUMBER)).toBe("100%")
        expect(formatCell(0, EColumnKind.NUMBER)).toBe("0%")
    })

    it("writes figures in the election event's number format", () => {
        expect(formatInteger(874624, ENumberFormatPolicy.PERIOD_COMMA)).toBe("874.624")
        expect(formatInteger(8589934591, ENumberFormatPolicy.SPACE_COMMA)).toBe(
            "8\u00a0589\u00a0934\u00a0591"
        )
        expect(formatRatio(0.5323, ENumberFormatPolicy.PERIOD_COMMA)).toBe("53,2%")
        expect(formatRatio(9996 / 10000, ENumberFormatPolicy.SPACE_COMMA)).toBe("99,9%")
        expect(formatCell(1234, EColumnKind.INTEGER, ENumberFormatPolicy.APOSTROPHE_PERIOD)).toBe(
            "1\u2019234"
        )
        expect(formatCell(0.12345, EColumnKind.NUMBER, ENumberFormatPolicy.PERIOD_COMMA)).toBe(
            "12,345%"
        )
        expect(formatCompact(1_500_000, "en", ENumberFormatPolicy.PERIOD_COMMA)).toBe("1,5M")
    })

    it("writes figures with commas for events without a number format", () => {
        expect(formatInteger(1234, null)).toBe("1,234")
        expect(formatInteger(1234, "no-such-format")).toBe("1,234")
        expect(formatRatio(0.5323, undefined)).toBe("53.2%")
    })

    it("formats a time in the event's time zone, and falls back for an unknown zone", () => {
        const text = formatDateTime("2026-05-01T10:00:00Z", "Asia/Manila", "en")
        expect(text).toContain("6:00")
        expect(formatDateTime("2026-05-01T10:00:00Z", "Nowhere/Invalid", "en")).not.toBe("")
        expect(formatDateTime("not a date", "UTC", "en")).toBe(UNDEFINED_VALUE)
    })
})
