// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {getKioskAwareUrl, getKioskPortalRedirectUrl} from "./kioskUrls"

describe("getKioskAwareUrl", () => {
    it("uses the configured kiosk URL in kiosk mode", () => {
        expect(
            getKioskAwareUrl(
                "https://login.example.test/auth/",
                "https://login-kiosk.example.test/auth/",
                true
            )
        ).toBe("https://login-kiosk.example.test/auth/")
    })

    it("keeps the default URL outside kiosk mode", () => {
        expect(
            getKioskAwareUrl(
                "https://login.example.test/auth/",
                "https://login-kiosk.example.test/auth/",
                false
            )
        ).toBe("https://login.example.test/auth/")
    })

    it("falls back to the default URL when no kiosk URL is configured", () => {
        expect(getKioskAwareUrl("https://login.example.test/auth/", "", true)).toBe(
            "https://login.example.test/auth/"
        )
    })
})

describe("getKioskPortalRedirectUrl", () => {
    it("moves kiosk traffic to the configured origin and preserves the URL", () => {
        expect(
            getKioskPortalRedirectUrl(
                "https://voting.example.test/tenant/t/event/e/login?kiosk#state",
                "https://voting-kiosk.example.test",
                true
            )
        ).toBe("https://voting-kiosk.example.test/tenant/t/event/e/login?kiosk#state")
    })

    it("does not redirect when already using the kiosk origin", () => {
        expect(
            getKioskPortalRedirectUrl(
                "https://voting-kiosk.example.test/tenant/t/event/e/login?kiosk",
                "https://voting-kiosk.example.test",
                true
            )
        ).toBeNull()
    })

    it("does not redirect outside kiosk mode", () => {
        expect(
            getKioskPortalRedirectUrl(
                "https://voting.example.test/tenant/t/event/e/login",
                "https://voting-kiosk.example.test",
                false
            )
        ).toBeNull()
    })
})
