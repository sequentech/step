// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {displayValue} from "./ScanovateConfirmation"
import {tipsFor} from "./ScanovateError"

describe("displayValue", () => {
    it("shows stored dates in the voter's language", () => {
        const date = {key: "dateOfBirth", value: "1990-01-01", type: "date"}
        expect(displayValue(date, "en")).toBe("January 1, 1990")
        expect(displayValue(date, "es")).toBe("1 de enero de 1990")
    })

    it("keeps other values and malformed dates as they are", () => {
        expect(displayValue({key: "firstName", value: "JUAN", type: "text"}, "en")).toBe("JUAN")
        expect(displayValue({key: "dateOfBirth", value: "01.01.1990", type: "date"}, "en")).toBe(
            "01.01.1990"
        )
        expect(displayValue({key: "dateOfBirth", value: "1990-01-01", type: "date"}, "x-")).toBe(
            "1990-01-01"
        )
    })
})

describe("tipsFor", () => {
    it("chooses tips for the reason of the failure", () => {
        expect(tipsFor("scanovateAttributesError")).toContain("scanovateTipSameDocument")
        expect(tipsFor("scanovateDocumentAuthenticationError")).toContain("scanovateTipCorners")
        expect(tipsFor("scanovateFaceMismatchError")).toContain("scanovateTipSameDocument")
        expect(tipsFor("scanovateFaceNotFoundError")).toContain("scanovateTipFaceUncovered")
        expect(tipsFor("scanovateDocumentUnreadableError")).toContain("scanovateTipCorners")
    })

    it("falls back to the capture tips for other errors", () => {
        expect(tipsFor("aRealmSpecificError")).toEqual(tipsFor("scanovateScoringError"))
    })
})
