// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {COUNTRIES} from "./countries"

describe("COUNTRIES", () => {
    it("lists every country with a two-letter uppercase code", () => {
        const invalidCodes = COUNTRIES.map(({code}) => code).filter(
            (code) => !/^[A-Z]{2}$/.test(code)
        )

        expect(invalidCodes).toEqual([])
    })

    it("does not repeat a code", () => {
        const codes = COUNTRIES.map(({code}) => code)

        expect(new Set(codes).size).toBe(codes.length)
    })
})
