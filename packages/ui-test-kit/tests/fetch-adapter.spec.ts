// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {routeFetch, type FetchService} from "../adapters/fetch"
import {json, type MockRequest} from "../mocks/http"
import {S3Mock} from "../mocks/s3"
import {ViolationLog} from "../mocks/violations"

const STORYBOOK = "http://localhost:6006"

// The adapter runs in story browsers; Node provides the same fetch classes, and
// each test gives it a page location and a recording fetch to fall back to.
let local: string[]
let restoreGlobals: () => void
test.beforeEach(() => {
    local = []
    const fetch = globalThis.fetch
    const location = Object.getOwnPropertyDescriptor(globalThis, "location")
    Object.defineProperty(globalThis, "location", {
        configurable: true,
        value: new URL(`${STORYBOOK}/iframe.html`),
    })
    globalThis.fetch = async (input) => {
        local.push(new Request(input).url)
        return new Response("local asset")
    }
    restoreGlobals = () => {
        globalThis.fetch = fetch
        if (location) Object.defineProperty(globalThis, "location", location)
        else Reflect.deleteProperty(globalThis, "location")
    }
})
test.afterEach(() => restoreGlobals())

test.describe("story fetch adapter", () => {
    test("answers requests from the service that handles them", async () => {
        const violations = new ViolationLog()
        const s3 = new S3Mock({origin: "https://s3.story.test", violations})
        s3.putBytes("public", "results/full.sqlite", new Uint8Array([1, 2, 3]), "application/x")
        const calls: MockRequest[] = []
        const graphql: FetchService = {
            handles: (url) => url.href === "https://hasura.story.test/v1/graphql",
            handle: (request) => {
                calls.push(request)
                return json(200, {data: {ok: true}})
            },
        }
        const restore = routeFetch([s3, graphql], violations)
        try {
            const bytes = await fetch("https://s3.story.test/s3/public/results/full.sqlite")
            expect(bytes.status).toBe(200)
            expect(bytes.headers.get("content-type")).toBe("application/x")
            expect([...new Uint8Array(await bytes.arrayBuffer())]).toEqual([1, 2, 3])

            const answer = await fetch("https://hasura.story.test/v1/graphql", {
                method: "POST",
                headers: {Authorization: "Bearer token"},
                body: '{"query":"{ ok }"}',
            })
            expect(await answer.json()).toEqual({data: {ok: true}})
            expect(calls).toHaveLength(1)
            expect(calls[0].method).toBe("POST")
            expect(calls[0].headers.authorization).toBe("Bearer token")
            expect(calls[0].body).toBe('{"query":"{ ok }"}')
            expect(violations.list()).toEqual([])
        } finally {
            restore()
        }
    })

    test("lets same-origin assets through and restores the original fetch", async () => {
        const violations = new ViolationLog()
        const original = globalThis.fetch
        const restore = routeFetch([], violations)
        expect(globalThis.fetch).not.toBe(original)
        expect(await (await fetch(`${STORYBOOK}/sql-wasm.wasm`)).text()).toBe("local asset")
        expect(local).toEqual([`${STORYBOOK}/sql-wasm.wasm`])
        restore()
        expect(globalThis.fetch).toBe(original)
        expect(violations.list()).toEqual([])
    })

    test("rejects and records requests no service handles", async () => {
        const violations = new ViolationLog()
        const restore = routeFetch([], violations)
        try {
            await expect(fetch("https://unknown.story.test/data.json")).rejects.toThrow(
                new TypeError("Failed to fetch https://unknown.story.test/data.json")
            )
            expect(violations.list()).toEqual([
                "Unexpected request: GET https://unknown.story.test/data.json",
            ])
            expect(local).toEqual([])
        } finally {
            restore()
        }
    })

    test("fails like the network when a service aborts", async () => {
        const violations = new ViolationLog()
        const aborting: FetchService = {
            handles: () => true,
            handle: () => ({abort: "connectionrefused"}),
        }
        const restore = routeFetch([aborting], violations)
        try {
            await expect(fetch("https://s3.story.test/down")).rejects.toThrow(
                new TypeError("Failed to fetch https://s3.story.test/down: connectionrefused")
            )
            expect(violations.list()).toEqual([])
        } finally {
            restore()
        }
    })
})
