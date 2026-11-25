// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import buildProfile from "./localInitializeProfile.json"
import {approvalControl, signingExperience} from "./operationalApprovalPolicy"
import {SigningAction} from "./types"

jest.mock("./localInitializeProfile.json", () => ({profile: "disabled"}))

afterEach(() => {
    buildProfile.profile = "disabled"
})

describe("certificate-free approval build", () => {
    it("never offers P12, the ordinary managed provider, or a no-approval fallback", () => {
        buildProfile.profile = "comelec-demo-direct-hsm"
        for (const action of Object.values(SigningAction)) {
            expect(signingExperience("hsm-demo-direct-action", action)).toBe("direct")
            for (const mode of [undefined, null, false, "unavailable", "hsm-required", "legacy-p12"]) {
                expect(signingExperience(mode, action)).toBe("unavailable")
            }
        }
        expect(signingExperience("hsm-demo-raw", SigningAction.InitializeVoting)).toBe("direct")
        expect(signingExperience("hsm-demo-raw", SigningAction.CloseVoting)).toBe("unavailable")
        expect(signingExperience("hsm-demo-direct-action", "wrong-action")).toBe("unavailable")
    })
})

describe("operational approval controls", () => {
    it("mounts the upload flow only on explicit server legacy policy", () => {
        expect(signingExperience("legacy-p12", "initialize-voting")).toBe("legacy")
        for (const policy of [undefined, null, "future-mode", false]) {
            expect(signingExperience(policy, "initialize-voting")).toBe("unavailable")
        }
    })

    it("does not fall through to certificate upload for other protected actions", () => {
        expect(signingExperience("hsm-required", "initialize-voting")).toBe("managed")
        for (const action of ["close-voting", "generate-report", "transmit-miru"]) {
            expect(signingExperience("hsm-required", action)).toBe("unavailable")
        }
    })

    it("keeps setup separate from approval and never starts twice while pending", () => {
        expect(approvalControl("setup-required", true)).toBe("set-up")
        expect(approvalControl("setup-retry-available", true)).toBe("set-up")
        expect(approvalControl("setup-recovery-required", true)).toBeNull()
        expect(approvalControl("challenge-available", false)).toBeNull()
        expect(approvalControl("challenge-available", true)).toBe("approve")
        for (const phase of [
            "waiting-for-identity",
            "pending",
            "reconcile",
            "unavailable",
            "approval-recorded",
        ]) {
            expect(approvalControl(phase, true)).toBeNull()
        }
    })
})
