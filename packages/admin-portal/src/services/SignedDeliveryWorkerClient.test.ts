/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    CREDENTIALS_DELIVERY_VERSION,
    SERVER_UNWRAP_PATH,
    passwordTransportAllowed,
    signedDeliveryVersion,
    unwrapOnServer,
} from "./SignedDeliveryWorkerClient"

/** fetch's options, derived from fetch itself (no DOM global for lint). */
type FetchInit = NonNullable<Parameters<typeof globalThis.fetch>[1]>

const PASSWORD = "synthetic transfer password"
const SHA = "ab".repeat(32)
const DOCUMENT = "00000000-0000-4000-8000-000000000007"

const envelope = (version: number) =>
    new File([new Uint8Array([...Array.from("SEQUENT-EZIP", (c) => c.charCodeAt(0)), version, 1, 2])], "x.ezip")

const answer = (status: number, body: unknown) =>
    ({ok: status >= 200 && status < 300, status, json: async () => body}) as unknown as Response

describe("server unwrap of packages with messaging credentials", () => {
    it("reads the envelope version without reading the payload", async () => {
        expect(await signedDeliveryVersion(envelope(CREDENTIALS_DELIVERY_VERSION))).toBe(2)
        expect(await signedDeliveryVersion(envelope(1))).toBe(1)
        expect(await signedDeliveryVersion(new File([new Uint8Array([80, 75, 3, 4])], "a.zip"))).toBe(
            undefined
        )
    })

    it("allows HTTPS, or plain HTTP to this machine only", () => {
        for (const ok of [
            "https://admin.example.invalid",
            "https://admin.example.invalid:8443",
            "http://localhost:3002",
            "http://127.0.0.1",
            "http://[::1]:8080",
        ]) {
            expect(passwordTransportAllowed(new URL(ok))).toBe(true)
        }
        for (const refused of [
            "http://admin.example.invalid",
            "http://localhost.example.invalid",
            "http://10.0.0.1",
            "ftp://localhost",
        ]) {
            expect(passwordTransportAllowed(new URL(refused))).toBe(false)
        }
    })

    it("never sends the password over plain HTTP to another host, or without a token", async () => {
        const fetchImpl = jest.fn()
        const signal = new AbortController().signal
        for (const [secure, origin] of [
            [true, "http://admin.example.invalid"],
            [false, "https://admin.example.invalid"],
            [true, ""],
        ] as const) {
            expect(
                await unwrapOnServer(DOCUMENT, PASSWORD, "token", signal, fetchImpl, secure, origin)
            ).toEqual({ok: false, reason: "insecure"})
        }
        expect(
            await unwrapOnServer(
                DOCUMENT,
                PASSWORD,
                undefined,
                signal,
                fetchImpl,
                true,
                "https://admin.example.invalid"
            )
        ).toEqual({ok: false, reason: "refused"})
        expect(fetchImpl).not.toHaveBeenCalled()
    })

    it("posts same-origin with a bearer token, not cookies or GraphQL", async () => {
        const fetchImpl = jest.fn(async () => answer(200, {sha256: SHA}))
        const reply = await unwrapOnServer(
            DOCUMENT,
            PASSWORD,
            "token",
            new AbortController().signal,
            fetchImpl,
            true,
            "https://admin.example.invalid:8443"
        )
        expect(reply).toEqual({ok: true, sha256: SHA})
        const [url, init] = fetchImpl.mock.calls[0] as unknown as [string, FetchInit]
        expect(url).toBe(`https://admin.example.invalid:8443${SERVER_UNWRAP_PATH}`)
        expect(init).toMatchObject({
            method: "POST",
            credentials: "omit",
            mode: "same-origin",
            cache: "no-store",
            // The password is never sent on to wherever a redirect points.
            redirect: "error",
        })
        expect((init.headers as Record<string, string>).Authorization).toBe("Bearer token")
        // No expected digest given: the field is absent, not null.
        expect(JSON.parse(init.body as string)).toEqual({document_id: DOCUMENT, password: PASSWORD})
    })

    it("sends the optional expected inner SHA-256 only in this direct request", async () => {
        const fetchImpl = jest.fn(async () => answer(200, {sha256: SHA}))
        await unwrapOnServer(
            DOCUMENT,
            PASSWORD,
            "token",
            new AbortController().signal,
            fetchImpl,
            true,
            "https://admin.example.invalid",
            SHA
        )
        const [, init] = fetchImpl.mock.calls[0] as unknown as [string, FetchInit]
        expect(init.redirect).toBe("error")
        expect(JSON.parse(init.body as string)).toEqual({
            document_id: DOCUMENT,
            password: PASSWORD,
            expected_sha256: SHA,
        })
    })

    it("treats a redirect as a failure rather than following it", async () => {
        // fetch rejects when redirect is "error" and the server redirects.
        const fetchImpl = jest.fn(async (_url: string, init?: FetchInit) => {
            if (init?.redirect === "error") throw new TypeError("redirect")
            return answer(302, {})
        })
        expect(
            await unwrapOnServer(
                DOCUMENT,
                PASSWORD,
                "token",
                new AbortController().signal,
                fetchImpl as unknown as typeof fetch,
                true,
                "https://admin.example.invalid"
            )
        ).toEqual({ok: false, reason: "unavailable"})
        expect(fetchImpl).toHaveBeenCalledTimes(1)
    })

    it("maps refusals to fixed reasons and never echoes the server body", async () => {
        const signal = new AbortController().signal
        const origin = "https://admin.example.invalid"
        const cases: [Response | Error, string][] = [
            [answer(400, {detail: PASSWORD}), "decrypt"],
            [answer(409, {}), "mismatch"],
            [answer(403, {}), "refused"],
            [answer(500, {}), "unavailable"],
            [answer(200, {sha256: "not-a-digest"}), "unavailable"],
            [new Error("network"), "unavailable"],
        ]
        for (const [result, reason] of cases) {
            const fetchImpl = jest.fn(async () => {
                if (result instanceof Error) throw result
                return result
            })
            const reply = await unwrapOnServer(DOCUMENT, PASSWORD, "t", signal, fetchImpl, true, origin)
            expect(reply).toEqual({ok: false, reason})
            expect(JSON.stringify(reply)).not.toContain(PASSWORD)
        }
    })
})
