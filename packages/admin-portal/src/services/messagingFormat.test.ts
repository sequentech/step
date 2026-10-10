// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {formatCount, intlLocale} from "@/services/messagingFormat"

describe("messaging counts", () => {
    it("name the portal's languages as Intl knows them", () => {
        expect(intlLocale("cat")).toBe("ca")
        expect(intlLocale("tl")).toBe("fil")
        expect(intlLocale("es")).toBe("es")
        expect(intlLocale("en-GB")).toBe("en-GB")
        expect(intlLocale(undefined)).toBe("en")
    })

    it("are written the way the viewer's language writes numbers", () => {
        expect(formatCount(1234567, "en")).toBe("1,234,567")
        expect(formatCount(1234567, "cat")).toBe("1.234.567")
        expect(formatCount(1234567, "tl")).toBe("1,234,567")
        expect(formatCount(1234567, "nl")).toBe("1.234.567")
        expect(formatCount(1234567, "not a language")).toBe("1234567")
    })
})
