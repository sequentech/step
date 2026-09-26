// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {formatNumber} from "./Numbers"

describe("formatNumber", () => {
    it.each([
        [1234567, "1,234,567"],
        [1234.5, "1,234.5"],
        ["98765.25", "98,765.25"],
        [-1500, "-1,500"],
        [0, "0"],
        // en-US keeps at most three fraction digits.
        [2.34567, "2.346"],
        // A string is read up to its first non-numeric character.
        ["12 votes", "12"],
    ])("formats %p as %p", (value, expected) => {
        expect(formatNumber(value)).toBe(expected)
    })

    it.each(["-", "", "n/a"])("returns the placeholder %p unchanged", (value) => {
        expect(formatNumber(value)).toBe(value)
    })
})
