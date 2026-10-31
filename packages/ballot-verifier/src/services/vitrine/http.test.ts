// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {fakeResponse} from "../../fixtures/vitrine"
import {
    EPolicyFetchProblem,
    fetchPolicyText,
    FetchLike,
    isFetchableUrl,
    isUnder,
    MAX_POLICY_BYTES,
    sdkFetch,
} from "./http"

const POLICY_URL = "https://logs.acme.example/e/ballots/policy/sha256:00"
const LOG = "https://logs.acme.example/e/ballots/"

const answering = (response: Response) =>
    jest.fn<Promise<Response>, Parameters<FetchLike>>(async () => response)

describe("isFetchableUrl", () => {
    it.each([
        ["https://logs.acme.example/policy", true],
        ["http://localhost:8080/policy", true],
        ["http://127.0.0.1/policy", true],
        ["http://logs.acme.example/policy", false],
        ["https://user:secret@logs.acme.example/", false],
        ["javascript:alert(1)", false],
        ["data:text/plain,vitrine", false],
        ["file:///etc/passwd", false],
        ["not a URL", false],
    ])("%s: %s", (url, fetchable) => {
        expect(isFetchableUrl(url)).toBe(fetchable)
    })
})

describe("isUnder", () => {
    it("holds a URL to the base's origin and path", () => {
        expect(isUnder(`${LOG}checkpoint`, LOG)).toBe(true)
        expect(isUnder(`${LOG}tile/0/000`, LOG)).toBe(true)
        expect(isUnder("https://logs.acme.example/e/ballots-other/checkpoint", LOG)).toBe(false)
        expect(isUnder("https://logs.acme.example.evil/e/ballots/checkpoint", LOG)).toBe(false)
        expect(isUnder(`${LOG}../events/checkpoint`, LOG)).toBe(false)
        expect(isUnder("not a URL", LOG)).toBe(false)
    })
})

describe("fetchPolicyText", () => {
    it("fetches with GET, without credentials, referrer or redirects", async () => {
        const f = answering(fakeResponse({status: 200, body: "vitrine/policy/v1\n"}))
        expect(await fetchPolicyText(POLICY_URL, {fetch: f})).toEqual({
            ok: true,
            text: "vitrine/policy/v1\n",
        })
        expect(f).toHaveBeenCalledWith(
            POLICY_URL,
            expect.objectContaining({
                method: "GET",
                credentials: "omit",
                referrerPolicy: "no-referrer",
                redirect: "error",
            })
        )
    })

    it("fetches nothing from an address it may not reach", async () => {
        const f = answering(fakeResponse({status: 200}))
        expect(await fetchPolicyText("http://logs.acme.example/policy", {fetch: f})).toEqual({
            ok: false,
            problem: EPolicyFetchProblem.NOT_HTTPS,
        })
        expect(f).not.toHaveBeenCalled()
    })

    it("says when the server can't be reached or doesn't answer with the policy", async () => {
        const down: FetchLike = async () => {
            throw new TypeError("network down")
        }
        expect(await fetchPolicyText(POLICY_URL, {fetch: down})).toEqual({
            ok: false,
            problem: EPolicyFetchProblem.UNREACHABLE,
        })
        expect(
            await fetchPolicyText(POLICY_URL, {fetch: answering(fakeResponse({status: 404}))})
        ).toEqual({ok: false, problem: EPolicyFetchProblem.STATUS, status: 404})
    })

    it("refuses more than a policy can be, and what isn't text", async () => {
        const big = fakeResponse({status: 200, body: "x".repeat(MAX_POLICY_BYTES + 1)})
        expect(await fetchPolicyText(POLICY_URL, {fetch: answering(big)})).toEqual({
            ok: false,
            problem: EPolicyFetchProblem.TOO_LARGE,
        })
        const binary = {
            status: 200,
            body: null,
            arrayBuffer: async () => Uint8Array.of(0xff, 0xfe, 0x00),
        } as unknown as Response
        expect(await fetchPolicyText(POLICY_URL, {fetch: answering(binary)})).toEqual({
            ok: false,
            problem: EPolicyFetchProblem.NOT_TEXT,
        })
    })

    it("stops reading a stream once it is larger than a policy can be", async () => {
        const chunks = [new Uint8Array(MAX_POLICY_BYTES), new Uint8Array(1), new Uint8Array(1)]
        const cancel = jest.fn(async () => undefined)
        const read = jest.fn(async () =>
            chunks.length > 0 ? {done: false, value: chunks.shift()} : {done: true}
        )
        const streamed = {status: 200, body: {getReader: () => ({read, cancel})}}
        expect(
            await fetchPolicyText(POLICY_URL, {fetch: answering(streamed as unknown as Response)})
        ).toEqual({ok: false, problem: EPolicyFetchProblem.TOO_LARGE})
        expect(cancel).toHaveBeenCalled()
        expect(read).toHaveBeenCalledTimes(2)
    })
})

describe("sdkFetch", () => {
    it("reaches only the bases it is given", async () => {
        const f = answering(fakeResponse({status: 200}))
        const scoped = sdkFetch(f, [LOG])
        await scoped(`${LOG}checkpoint`)
        await scoped(new URL(`${LOG}tile/0/000`))
        await expect(scoped("https://mirror.example/ballots/checkpoint")).rejects.toThrow(TypeError)
        await expect(scoped("https://monitor.example/gossip", {method: "POST"})).rejects.toThrow(
            TypeError
        )
        expect(f.mock.calls.map(([url]) => url)).toEqual([`${LOG}checkpoint`, `${LOG}tile/0/000`])
    })

    it("never reaches plain http beyond this machine", async () => {
        const f = answering(fakeResponse({status: 200}))
        const open = sdkFetch(f)
        await expect(open("http://logs.acme.example/checkpoint")).rejects.toThrow(TypeError)
        await open("https://mirror.example/ballots/checkpoint")
        expect(f).toHaveBeenCalledTimes(1)
    })
})
