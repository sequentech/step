// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    createOperationalApprovalApi,
    SetupGoldRequired,
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

    it("keeps retry and ambiguous operator recovery separate", () => {
        expect(parseSetupStatus({scope, phase: "setup-retry-available"}, scope)).toBe(
            "setup-retry-available"
        )
        expect(parseSetupStatus({scope, phase: "setup-recovery-required"}, scope)).toBe(
            "setup-recovery-required"
        )
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

describe("managed approval requests", () => {
    const apiFixture = () => {
        const query = jest.fn()
        const mutate = jest.fn()
        return {query, mutate, client: createOperationalApprovalApi({query, mutate})}
    }

    it("uses the ordinary signing role, network-only status and scoped responses", async () => {
        const {query, client} = apiFixture()
        query.mockResolvedValueOnce({data: {operationalInitializeStatus: status}})
        expect((await client.status(scope)).phase).toBe("challenge-available")
        expect(query).toHaveBeenCalledWith(
            expect.objectContaining({
                variables: {request: scope.request},
                context: {headers: {"x-hasura-role": "sign-initialize-voting"}},
                fetchPolicy: "no-cache",
            })
        )
        query.mockResolvedValueOnce({
            data: {operationalInitializeSetupStatus: {scope, phase: "setup-recovery-required"}},
        })
        expect(await client.setupStatus(scope)).toBe("setup-recovery-required")
        query.mockResolvedValue({errors: [{message: "fixture failure"}]})
        await expect(client.status(scope)).rejects.toThrow("Signing status unavailable")
        await expect(client.setupStatus(scope)).rejects.toThrow("Setup status unavailable")
    })

    it("distinguishes Gold setup requirement from unrelated refusal and malformed answers", async () => {
        const {mutate, client} = apiFixture()
        mutate
            .mockResolvedValueOnce({errors: [{extensions: {gold_required: true}}]})
            .mockResolvedValueOnce({
                errors: [
                    {
                        extensions: {
                            internal: {response: {body: '{"extensions":{"gold_required":true}}'}},
                        },
                    },
                ],
            })
            .mockResolvedValueOnce({errors: [{message: "unrelated"}]})
            .mockResolvedValueOnce({
                data: {operationalInitializeSetup: {scope, phase: "setup-pending"}},
            })
        await expect(client.setup(scope)).rejects.toBeInstanceOf(SetupGoldRequired)
        await expect(client.setup(scope)).rejects.toBeInstanceOf(SetupGoldRequired)
        await expect(client.setup(scope)).rejects.toThrow("Setup unavailable")
        await expect(client.setup(scope)).resolves.toBe("setup-pending")
        expect(mutate).toHaveBeenCalledTimes(4)
    })

    it("sends the reviewed digest and exact Post; rejects foreign shapes and malformed attempts", async () => {
        const {mutate, client} = apiFixture()
        const attempt = "00000000-0000-4000-8000-000000000007"
        const page = {path: "election-show", panel: "publish", election_id: scope.event} as const
        const digest = "a".repeat(64)
        mutate
            .mockResolvedValueOnce({
                data: {operationalInitializeStart: {attempt, redirect: "synthetic-redirect"}},
            })
            .mockResolvedValueOnce({data: {operationalInitializeStart: {attempt, redirect: null}}})
            .mockResolvedValueOnce({errors: [{message: "consumed"}]})
            .mockResolvedValueOnce({
                data: {operationalInitializeCancel: {phase: "challenge-available"}},
            })
            .mockResolvedValueOnce({errors: [{message: "fixture reconciliation failed"}]})
            .mockResolvedValueOnce({})
        await expect(client.start(scope, page, digest)).resolves.toEqual({
            attempt,
            redirect: "synthetic-redirect",
        })
        expect(mutate).toHaveBeenCalledWith(
            expect.objectContaining({variables: {request: scope.request, page, digest}})
        )
        await expect(client.start(scope, page, digest)).rejects.toThrow("Signing start unconfirmed")
        await expect(client.cancelUnused(scope, "invalid")).rejects.toThrow(
            "Invalid unused signing challenge"
        )
        await expect(client.cancelUnused(scope, attempt)).rejects.toThrow(
            "Challenge cancellation unconfirmed"
        )
        await expect(client.cancelUnused(scope, attempt)).resolves.toBeUndefined()
        expect(mutate).toHaveBeenCalledWith(
            expect.objectContaining({variables: {request: scope.request, attempt}})
        )
        await expect(client.reconcile(scope)).rejects.toThrow("Signing status unconfirmed")
        await expect(client.reconcile(scope)).resolves.toBeUndefined()
    })
})
