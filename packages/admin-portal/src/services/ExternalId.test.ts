// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {externalIdToSave} from "./ExternalId"

describe("externalIdToSave", () => {
    it("keeps a stored ID exactly, blanks included", () => {
        // The field is read-only once set: an unrelated save must not rewrite it.
        expect(externalIdToSave(" legacy-seat-v1:11 ", " legacy-seat-v1:11 ")).toBe(
            " legacy-seat-v1:11 "
        )
        expect(externalIdToSave("", "")).toBe("")
    })

    it("keeps a stored ID the form did not submit", () => {
        // A disabled field is left out of the submitted values.
        expect(externalIdToSave("legacy-seat-v1:11", undefined)).toBe("legacy-seat-v1:11")
    })

    it("saves an entered ID without the blanks around it", () => {
        expect(externalIdToSave(null, "  council 2026 ")).toBe("council 2026")
        expect(externalIdToSave(undefined, "council-2026")).toBe("council-2026")
    })

    it.each([null, undefined, "", "   "])("saves a blank entry %p as null", (entered) => {
        // Empty stays empty, so the ID can still be set later.
        expect(externalIdToSave(null, entered)).toBeNull()
    })
})
