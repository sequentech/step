// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    parseApprovalStatus,
    parseSetupStatus,
    trustedSigningRedirect,
} from "./operationalApprovalApi"

const scope = {
    tenant: "00000000-0000-4000-8000-000000000001",
    event: "00000000-0000-4000-8000-000000000002",
    request: "00000000-0000-4000-8000-000000000003",
    official: "00000000-0000-4000-8000-000000000004",
}
const status = {scope, phase: "challenge-available", device_profile: "softhsm-test"}

describe("authoritative managed approval status", () => {
    it("never describes a software receipt as hardware and rejects an unconfirmed provider", () => {
        expect(parseApprovalStatus(status, scope)).toEqual({
            phase: "challenge-available",
            softwareDemo: true,
        })
        expect(
            parseApprovalStatus({...status, device_profile: "yubihsm"}, scope).softwareDemo
        ).toBe(false)
        expect(() => parseApprovalStatus({...status, device_profile: "normal"}, scope)).toThrow(
            "Invalid device profile"
        )
    })

    it("offers cancellation only for an exact unused verification attempt", () => {
        const attempt = "00000000-0000-4000-8000-000000000005"
        const waiting = {...status, phase: "waiting-for-identity", cancellable_attempt: attempt}
        expect(parseApprovalStatus(waiting, scope).cancellableAttempt).toBe(attempt)
        expect(
            parseApprovalStatus({...waiting, cancellable_attempt: null}, scope).cancellableAttempt
        ).toBeUndefined()
        expect(() => parseApprovalStatus({...waiting, phase: "pending"}, scope)).toThrow(
            "Invalid unused signing challenge"
        )
        expect(() =>
            parseApprovalStatus({...waiting, cancellable_attempt: "unverified"}, scope)
        ).toThrow("Invalid unused signing challenge")
    })

    it("rejects an otherwise valid answer for another official or request", () => {
        expect(() => parseApprovalStatus(status, {...scope, official: scope.event})).toThrow(
            "Invalid operational signing scope"
        )
        expect(() =>
            parseSetupStatus(
                {...status, phase: "setup-available"},
                {...scope, request: scope.event}
            )
        ).toThrow("Invalid operational signing scope")
    })
})

describe("privileged verification redirect", () => {
    it("accepts only the configured HTTPS tenant authorization endpoint and PAR shape", () => {
        const base = "https://sso.test.local/auth/"
        const url = `https://sso.test.local/auth/realms/tenant-${scope.tenant}/protocol/openid-connect/auth?client_id=sign&request_uri=urn:ietf:params:oauth:request_uri:id`
        expect(trustedSigningRedirect(url, base, scope.tenant)).toBe(url)
        for (const invalid of [
            url.replace("sso.test.local", "untrusted.test.local"),
            url.replace(scope.tenant, scope.event),
            url + "&redirect_uri=https://untrusted.test.local/",
            url.replace("https:", "http:"),
        ]) {
            expect(() => trustedSigningRedirect(invalid, base, scope.tenant)).toThrow(
                "Invalid verification redirect"
            )
        }
    })
})
