// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation from "@/translations/en"
import {IPermissions} from "@/types/keycloak"
import {
    CancelReason,
    CertificateCheckId,
    MAX_EXPIRES_MINUTES,
    MAX_SIGNATURES,
    SIGNING_ACTIONS,
    SIGNING_EXPIRY_OPTIONS,
    SigningAction,
    SigningActionGroup,
    SigningRequestStatus,
} from "./types"

const signing = englishTranslation.translations.signing

describe("signing types", () => {
    // sequent-core's MAX_SIGNATURES and MAX_EXPIRES_MINUTES: its test reads them here.
    it("limits a rule as the server does", () => {
        expect(MAX_SIGNATURES).toBe(100)
        expect(MAX_EXPIRES_MINUTES).toBe(365 * 24 * 60)
    })

    it("signs each action under sign-<action id>", () => {
        for (const action of Object.values(SigningAction)) {
            expect(SIGNING_ACTIONS[action].signPermission).toBe(`sign-${action}`)
        }
        const signPermissions = Object.values(IPermissions).filter((value) =>
            value.startsWith("sign-")
        )
        expect(signPermissions).toHaveLength(Object.values(SigningAction).length)
    })

    it("labels every action, group, status and cancel reason", () => {
        for (const action of Object.values(SigningAction)) {
            const labels = signing.actions[action]
            for (const key of [
                "label",
                "short",
                "permissionName",
                "object",
                "appliesTo",
                "description",
                "certify",
            ] as const) {
                expect(labels[key]).toEqual(expect.any(String))
            }
        }
        expect(Object.keys(signing.groups).sort()).toEqual(Object.values(SigningActionGroup).sort())
        expect(Object.keys(signing.status).sort()).toEqual(
            Object.values(SigningRequestStatus).sort()
        )
        expect(Object.keys(signing.cancelReasons).sort()).toEqual(
            Object.values(CancelReason).sort()
        )
    })

    // A check reads as what happened: a pass as a pass, a failure as a failure.
    it("labels every certificate check both when it passes and when it fails", () => {
        const ids = Object.values(CertificateCheckId).sort()
        expect(Object.keys(signing.dialog.checks.passed).sort()).toEqual(ids)
        expect(Object.keys(signing.dialog.checks.failed).sort()).toEqual(ids)
        expect(signing.dialog.checks.passed["registered-to-other"]).toBe(
            "Not registered to anyone else"
        )
        expect(signing.dialog.checks.failed["registered-to-other"]).toBe("Registered to {{name}}")
        expect(signing.dialog.checks["first-use"]).toEqual(expect.any(String))
    })

    it("counts waiting requests with plural forms", () => {
        expect(signing.pendingRequests_one).toContain("{{count}} request is")
        expect(signing.pendingRequests_other).toContain("{{count}} requests are")
    })

    it("labels every expiry option", () => {
        for (const minutes of SIGNING_EXPIRY_OPTIONS) {
            const key = (minutes === null ? "none" : String(minutes)) as keyof typeof signing.expiry
            expect(signing.expiry[key]).toEqual(expect.any(String))
        }
    })

    // Copy that names the organization takes it from the tenant.
    it("never names an organization literally", () => {
        expect(signing.dialog.problems.issuerNotAccepted).toContain("{{organization}}")
        expect(JSON.stringify(signing)).not.toMatch(/COMELEC|SBEI/i)
        // Preset role names are configuration, not copy.
        expect(JSON.stringify(signing)).not.toMatch(/Security Officer|Configuration Manager|OFOV/)
    })
})
