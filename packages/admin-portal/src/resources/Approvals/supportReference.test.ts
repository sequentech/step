// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    findBySupportReference,
    normalizeSupportReference,
    supportReference,
} from "./supportReference"

// The vector Keycloak's SupportReferenceTest pins (voter-enrollment).
const APPLICATION = "7f0c2a9e-1b3d-4c5e-8f60-123456789abc"

describe("support reference (EF-1)", () => {
    it("derives the reference the pending page shows, as Keycloak does", async () => {
        expect(await supportReference(APPLICATION)).toBe("65SR-MSSD")
        expect(await supportReference(APPLICATION.toUpperCase())).toBe("65SR-MSSD")
    })

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

    it("finds the applications whose id has the reference", async () => {
        expect(
            await findBySupportReference(
                ["7f0c2a9e-1b3d-4c5e-8f60-123456789abd", APPLICATION],
                "65SR-MSSD"
            )
        ).toEqual([APPLICATION])
    })
})
