// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {resendCountdownStart} from "./resendCountdown"

describe("resendCountdownStart", () => {
    it("starts the full wait when a code was just sent", () => {
        expect(resendCountdownStart(60, true, undefined)).toBe(60)
        expect(resendCountdownStart(60, true, 12)).toBe(60)
    })

    it("continues the server's wait when the page is shown again after Back", () => {
        expect(resendCountdownStart(60, false, 37)).toBe(37)
    })

    it("lifts the countdown once the server allows a resend", () => {
        expect(resendCountdownStart(60, false, 0)).toBe(0)
    })

    it("never runs past the full wait, nor below zero", () => {
        expect(resendCountdownStart(60, false, 600)).toBe(60)
        expect(resendCountdownStart(60, false, -5)).toBe(0)
    })

    it("keeps the running countdown when the server says nothing", () => {
        expect(resendCountdownStart(60, false, undefined)).toBeUndefined()
        expect(resendCountdownStart(60, false, Number.NaN)).toBeUndefined()
    })
})
