// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
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
        expect(formatInteger(874624, "en")).toBe("874,624")
        expect(formatInteger(null, "en")).toBe(UNDEFINED_VALUE)
        expect(formatInteger(undefined, "en")).toBe(UNDEFINED_VALUE)
        expect(formatInteger(Number.NaN, "en")).toBe(UNDEFINED_VALUE)
    })

    it("shows a ratio as a percentage with one decimal", () => {
        expect(formatRatio(0.5323, "en")).toBe("53.2%")
        expect(formatRatio(null, "en")).toBe(UNDEFINED_VALUE)
    })

    it("shortens large numbers for charts", () => {
        expect(formatCompact(647000, "en")).toBe("647K")
    })

    it("formats a table cell by its column kind, keeping exact values", () => {
        expect(formatCell(1234, EColumnKind.INTEGER, "en")).toBe("1,234")
        expect(formatCell(0.12345, EColumnKind.NUMBER, "en")).toBe("12.35%")
        expect(formatCell(null, EColumnKind.NUMBER, "en")).toBe(UNDEFINED_VALUE)
        expect(formatCell("—", EColumnKind.NUMBER, "en")).toBe(UNDEFINED_VALUE)
        expect(formatCell("Male", EColumnKind.TEXT, "en")).toBe("Male")
        expect(formatCell({a: 1}, EColumnKind.TEXT, "en")).toBe('{"a":1}')
    })

    it("formats a time in the event's time zone, and falls back for an unknown zone", () => {
        const text = formatDateTime("2026-05-01T10:00:00Z", "Asia/Manila", "en")
        expect(text).toContain("6:00")
        expect(formatDateTime("2026-05-01T10:00:00Z", "Nowhere/Invalid", "en")).not.toBe("")
        expect(formatDateTime("not a date", "UTC", "en")).toBe(UNDEFINED_VALUE)
    })
})
