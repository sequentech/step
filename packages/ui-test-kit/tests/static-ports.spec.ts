// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {mkdtemp, realpath, rm, writeFile} from "node:fs/promises"
import {request} from "node:http"
import {tmpdir} from "node:os"
import {join} from "node:path"
import {createPortAllocator, servedRoots, serveDist} from "../server/static"

test("configured static ports keep parallel workers and their second servers separate", () => {
    const first = createPortAllocator(44000, 44003)
    const second = createPortAllocator(44000, 44003)
    const worker0 = {parallelIndex: 0, config: {workers: 2}}
    const worker1 = {parallelIndex: 1, config: {workers: 2}}
    expect([first(worker0), second(worker1), first(worker0), second(worker1)]).toEqual([
        44000, 44001, 44002, 44003,
    ])
    expect(() => first(worker0)).toThrow("range 44000-44003 is exhausted for worker 0")
    expect(() => second(worker1)).toThrow("range 44000-44003 is exhausted for worker 1")
    // Replacement workers use the same parallel slot after their predecessor exits.
    expect(createPortAllocator(44000, 44003)(worker1)).toBe(44001)
})

test("standalone and single-worker ports retain consecutive allocation", () => {
    const standalone = createPortAllocator(44000, 44001)
    const worker = createPortAllocator(44000, 44001)
    const info = {parallelIndex: 0, config: {workers: 1}}
    expect([standalone(), standalone(), worker(info), worker(info)]).toEqual([
        44000, 44001, 44000, 44001,
    ])
    const ephemeral = createPortAllocator(0)
    expect([ephemeral(), ephemeral(info)]).toEqual([0, 0])
    const last = createPortAllocator(65535)
    expect(last()).toBe(65535)
    expect(() => last()).toThrow("range 65535-65535 is exhausted")
})

test("invalid configured ranges and worker metadata fail before binding", () => {
    for (const base of [-1, 0.5, 65536, Number.NaN])
        expect(() => createPortAllocator(base)).toThrow("STEP_UI_TEST_PORT_BASE")
    for (const limit of [43999, 65536, Number.NaN])
        expect(() => createPortAllocator(44000, limit)).toThrow("STEP_UI_TEST_PORT_LIMIT")
    const allocate = createPortAllocator(44000)
    for (const info of [
        {parallelIndex: -1, config: {workers: 2}},
        {parallelIndex: 2, config: {workers: 2}},
        {parallelIndex: 0, config: {workers: 0}},
    ])
        expect(() => allocate(info)).toThrow("valid Playwright parallelIndex and worker count")
})

// Playwright requires destructuring even without fixture dependencies.
// eslint-disable-next-line no-empty-pattern
test("static servers retain distinct simultaneous listeners within one worker", async ({}, testInfo) => {
    const directory = await mkdtemp(join(tmpdir(), "ui-kit-ports-"))
    const servers: Awaited<ReturnType<typeof serveDist>>[] = []
    try {
        await writeFile(join(directory, "index.html"), "<main>Concurrent fixture</main>")
        servers.push(await serveDist(directory, undefined, testInfo))
        servers.push(await serveDist(directory, undefined, testInfo))
        expect(servers[0].origin).not.toBe(servers[1].origin)
        for (const server of servers)
            expect(await (await fetch(`${server.origin}/index.html`)).text()).toBe(
                "<main>Concurrent fixture</main>"
            )
    } finally {
        await Promise.all(servers.map((server) => server.close()))
        await rm(directory, {recursive: true, force: true})
    }
})

test("a scoped Chromium proxy serves only its known fixture after an HTTP redirect", async ({
    browser,
}, testInfo) => {
    const proxyRoot = await mkdtemp(join(tmpdir(), "ui-kit-proxy-"))
    const sentinelRoot = await mkdtemp(join(tmpdir(), "ui-kit-proxy-sentinel-"))
    const servers: Awaited<ReturnType<typeof serveDist>>[] = []
    try {
        await writeFile(join(proxyRoot, "index.html"), "<h1>Expected isolated fixture</h1>")
        await writeFile(join(sentinelRoot, "index.html"), "<h1>Unexpected non-proxied request</h1>")
        const sentinel = await serveDist(sentinelRoot, undefined, testInfo)
        servers.push(sentinel)
        const alias = "http://localhost:" + new URL(sentinel.origin).port
        const proxy = await serveDist(proxyRoot, undefined, testInfo, alias)
        servers.push(proxy)
        expect(servedRoots.get(alias)).toBe(await realpath(proxyRoot))

        const proxyUrl = new URL(proxy.origin)
        const denied = await new Promise((resolve, reject) => {
            const outgoing = request(
                {
                    hostname: proxyUrl.hostname,
                    port: proxyUrl.port,
                    path: "http://outside-fixture.invalid/",
                },
                (response) => {
                    response.resume()
                    resolve(response.statusCode)
                }
            )
            outgoing.on("error", reject)
            outgoing.end()
        })
        expect(denied).toBe(403)

        const proxySettings = {server: proxy.origin, bypass: "<-loopback>"}
        const context = await browser.newContext({
            proxy: proxySettings,
            serviceWorkers: "block",
        })
        try {
            let intercepts = 0
            await context.route("**/*", async (route) => {
                intercepts++
                expect(route.request().url()).toBe(alias + "/auth")
                await route.fulfill({status: 302, headers: {location: alias + "/callback"}})
            })
            const page = await context.newPage()
            await page.goto(alias + "/auth")
            await expect(page).toHaveURL(alias + "/callback")
            await expect(page.getByRole("heading")).toHaveText("Expected isolated fixture")
            // Chromium follows this mocked HTTP redirect without re-running the route.
            expect(intercepts).toBe(1)
        } finally {
            await context.close()
        }
        servers.pop()
        await proxy.close()
        expect(servedRoots.has(alias)).toBe(false)

        // An unavailable proxy must not fall back to the separate localhost listener.
        const unavailable = await browser.newContext({proxy: proxySettings})
        try {
            const page = await unavailable.newPage()
            await expect(page.goto(alias + "/direct-fallback")).rejects.toThrow(
                "ERR_PROXY_CONNECTION_FAILED"
            )
        } finally {
            await unavailable.close()
        }
    } finally {
        await Promise.all(servers.map((server) => server.close()))
        await rm(proxyRoot, {recursive: true, force: true})
        await rm(sentinelRoot, {recursive: true, force: true})
    }
})
