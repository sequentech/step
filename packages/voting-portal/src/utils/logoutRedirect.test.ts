// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {getLogoutRedirectUrl, isKioskClientId} from "./logoutRedirect"

const baseOptions = {
    origin: "https://voting.example.test",
    pathname: "/tenant/tenant-id/event/event-id/election/election-id/vote",
    tenantId: "tenant-id",
    eventId: "event-id",
    defaultClientId: "voting-portal",
    hasKioskQuery: false,
}

describe("voting portal logout redirects", () => {
    it("identifies the kiosk client from the configured voting client id", () => {
        expect(isKioskClientId("voting-portal-kiosk", "voting-portal")).toBe(true)
        expect(isKioskClientId("voting-portal", "voting-portal")).toBe(false)
    })

    it("preserves the existing normal voting client redirect", () => {
        expect(
            getLogoutRedirectUrl({
                ...baseOptions,
                clientId: "voting-portal",
            })
        ).toBe("https://voting.example.test/tenant/tenant-id/event/event-id")
    })

    it("redirects a kiosk client directly to the kiosk login", () => {
        expect(
            getLogoutRedirectUrl({
                ...baseOptions,
                clientId: "voting-portal-kiosk",
            })
        ).toBe("https://voting.example.test/tenant/tenant-id/event/event-id/login?kiosk")
    })

    it("uses the client id instead of a stale kiosk query when Keycloak is available", () => {
        expect(
            getLogoutRedirectUrl({
                ...baseOptions,
                clientId: "voting-portal",
                hasKioskQuery: true,
            })
        ).toBe("https://voting.example.test/tenant/tenant-id/event/event-id")
    })

    it("falls back to the kiosk query when Keycloak is unavailable", () => {
        expect(
            getLogoutRedirectUrl({
                ...baseOptions,
                clientId: undefined,
                hasKioskQuery: true,
            })
        ).toBe("https://voting.example.test/tenant/tenant-id/event/event-id/login?kiosk")
    })

    it("gives an explicit redirect precedence", () => {
        expect(
            getLogoutRedirectUrl({
                ...baseOptions,
                clientId: "voting-portal-kiosk",
                redirectUrl: "https://configured.example.test/finished",
            })
        ).toBe("https://configured.example.test/finished")
    })
})
