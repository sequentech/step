// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {normalizeSupportReference} from "./supportReference"

describe("support reference (EF-1)", () => {
    it("reads a reference typed loosely, and nothing else as one", () => {
        expect(normalizeSupportReference(" 65sr-mssd ")).toBe("65SR-MSSD")
        expect(normalizeSupportReference("65SR MSSD")).toBe("65SR-MSSD")
        expect(normalizeSupportReference("65SRMSSD")).toBe("65SR-MSSD")
        // Crockford: I and L read as 1, O as 0.
        expect(normalizeSupportReference("Q5KW-8RFO")).toBe("Q5KW-8RF0")
        expect(normalizeSupportReference("dela cruz")).toBeNull()
        expect(normalizeSupportReference("65SR-MSSU")).toBeNull()
        expect(normalizeSupportReference("65SR-MSS")).toBeNull()
    })
})
