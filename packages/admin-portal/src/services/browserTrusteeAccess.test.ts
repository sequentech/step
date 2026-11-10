// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {offersBrowserTrustee} from "./browserTrusteeAccess"

describe("offersBrowserTrustee", () => {
    it("is false until the tenant has loaded", () => {
        expect(offersBrowserTrustee(undefined)).toBe(false)
        expect(offersBrowserTrustee(null)).toBe(false)
    })

    it("is true for a tenant whose setting makes signed configuration packages optional", () => {
        expect(
            offersBrowserTrustee({
                settings: {languages: ["en"], configuration_signing: {policy: "optional"}},
            })
        ).toBe(true)
    })

    it("is false for a tenant that requires signed configuration packages", () => {
        expect(
            offersBrowserTrustee({
                settings: {languages: ["en"], configuration_signing: {policy: "required"}},
            })
        ).toBe(false)
    })

    it("is false for a tenant without the setting, which requires them too", () => {
        expect(offersBrowserTrustee({settings: null})).toBe(false)
        expect(offersBrowserTrustee({settings: {languages: ["en"]}})).toBe(false)
    })
})
