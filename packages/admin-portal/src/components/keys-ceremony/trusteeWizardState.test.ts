// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {canTrusteeProceedToDownload} from "./trusteeWizardState"

describe("canTrusteeProceedToDownload", () => {
    it("allows an invited trustee to proceed after key generation", () => {
        expect(canTrusteeProceedToDownload(true, true)).toBe(true)
    })

    it("blocks an invited trustee while keys are not generated", () => {
        expect(canTrusteeProceedToDownload(true, false)).toBe(false)
    })

    it("blocks a trustee who is not participating", () => {
        expect(canTrusteeProceedToDownload(false, true)).toBe(false)
        expect(canTrusteeProceedToDownload(false, false)).toBe(false)
    })
})
