// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {signedPackagesRequired} from "./configurationSigningPolicy"

const withSetting = (configuration_signing: unknown) => ({
    settings: {languages: ["en"], configuration_signing},
})

describe("signedPackagesRequired", () => {
    it("requires signed packages where the tenant has no setting", () => {
        expect(signedPackagesRequired({settings: null})).toBe(true)
        expect(signedPackagesRequired({settings: {languages: ["en"]}})).toBe(true)
        expect(signedPackagesRequired(withSetting(null))).toBe(true)
    })

    it("requires them where the setting names no policy or the required one", () => {
        expect(signedPackagesRequired(withSetting({}))).toBe(true)
        expect(signedPackagesRequired(withSetting({required_approvals: 2}))).toBe(true)
        expect(signedPackagesRequired(withSetting({policy: "required"}))).toBe(true)
    })

    it("requires them where the setting names another policy", () => {
        expect(signedPackagesRequired(withSetting({policy: "Optional"}))).toBe(true)
        expect(signedPackagesRequired(withSetting({policy: "none"}))).toBe(true)
        expect(signedPackagesRequired(withSetting("optional"))).toBe(true)
    })

    it("makes them optional only where the setting names the policy optional", () => {
        expect(signedPackagesRequired(withSetting({policy: "optional"}))).toBe(false)
    })

    it("requires them until the tenant has loaded", () => {
        expect(signedPackagesRequired(undefined)).toBe(true)
        expect(signedPackagesRequired(null)).toBe(true)
    })
})
