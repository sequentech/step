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

    it("defaults to Off when neither policy nor activated is set", () => {
        expect(getEffectiveSupportMaterialsPolicy({})).toBe(ESupportMaterialsPolicy.OFF)
    })

    it("falls back to Optional for legacy activated: true", () => {
        expect(getEffectiveSupportMaterialsPolicy({activated: true})).toBe(
            ESupportMaterialsPolicy.OPTIONAL
        )
    })

    it("falls back to Off for legacy activated: false", () => {
        expect(getEffectiveSupportMaterialsPolicy({activated: false})).toBe(
            ESupportMaterialsPolicy.OFF
        )
    })

    it("prefers the explicit policy over the legacy activated flag", () => {
        expect(
            getEffectiveSupportMaterialsPolicy({
                activated: false,
                policy: ESupportMaterialsPolicy.MANDATORY_FOR_VOTING,
            })
        ).toBe(ESupportMaterialsPolicy.MANDATORY_FOR_VOTING)
    })
})
