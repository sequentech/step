// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ESupportMaterialsPolicy,
    getEffectiveSupportMaterialsPolicy,
} from "./ElectionEventPresentation"

describe("getEffectiveSupportMaterialsPolicy", () => {
    it("defaults to Off when materials is undefined", () => {
        expect(getEffectiveSupportMaterialsPolicy(undefined)).toBe(ESupportMaterialsPolicy.OFF)
    })

    it("defaults to Off when policy is not set", () => {
        expect(getEffectiveSupportMaterialsPolicy({})).toBe(ESupportMaterialsPolicy.OFF)
    })

    it("returns the explicit policy when set", () => {
        expect(
            getEffectiveSupportMaterialsPolicy({
                policy: ESupportMaterialsPolicy.MANDATORY_FOR_VOTING,
            })
        ).toBe(ESupportMaterialsPolicy.MANDATORY_FOR_VOTING)
    })
})
