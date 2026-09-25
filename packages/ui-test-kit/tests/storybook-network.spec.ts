// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {createHash} from "node:crypto"
import {createServer} from "node:http"
import type {AddressInfo} from "node:net"
import type {Page} from "playwright"
import {collectNetworkViolations, installNetworkGuard} from "../../test-support/storybook/network"

async function ownedServer() {
    const upgrades: string[] = []
    const requests: string[] = []
    const server = createServer((request, response) => {
        requests.push(`${request.method} ${request.url}`)
        response.end("<main>Story</main>")
    })
    server.on("upgrade", (request, socket) => {
        upgrades.push(request.url ?? "")
        const accept = createHash("sha1")
            .update(`${request.headers["sec-websocket-key"]}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
            .digest("base64")
        const protocol = request.headers["sec-websocket-protocol"]
        socket.end(
            "HTTP/1.1 101 Switching Protocols\r\n" +
                "Upgrade: websocket\r\nConnection: Upgrade\r\n" +
                `Sec-WebSocket-Accept: ${accept}\r\n` +
                (protocol ? `Sec-WebSocket-Protocol: ${protocol}\r\n` : "") +
                "\r\n"
        )
    })
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve))
    return {
        origin: `http://127.0.0.1:${(server.address() as AddressInfo).port}`,
        upgrades,
        requests,
        close: () =>
            new Promise<void>((resolve, reject) => {
                server.close((error) => (error ? reject(error) : resolve()))
                server.closeAllConnections()
            }),
    }
}

const connect = (page: Page, url: string, protocols: string[] = []) =>
    page.evaluate(
        ({url, protocols}) =>
            new Promise<{instance: boolean; open: number; protocol: string} | undefined>(
                (resolve) => {
                    try {
                        const socket = new WebSocket(url, protocols)
                        let opened: {instance: boolean; open: number; protocol: string} | undefined
                        socket.onopen = () => {
                            opened = {
                                instance: socket instanceof WebSocket,
                                open: socket.readyState,
                                protocol: socket.protocol,
                            }
                        }
                        socket.onerror = () => {}
                        socket.onclose = () => resolve(opened)
                    } catch (error) {
                        if (!(error instanceof DOMException) || error.name !== "SecurityError")
                            throw error
                        resolve(undefined)
                    }
                }
            ),
        {url, protocols}
    )

test("Storybook allows its exact runner sockets and rejects application sockets before handshake", async ({
    page,
}) => {
    const local = await ownedServer()
    const external = await ownedServer()
    try {
        await page.goto(local.origin)
        const runner = {
            origin: local.origin,
            hmrToken: "owned-vite-token",
            apiToken: "owned-vitest-token",
            sessionId: "owned-session",
            projectName: "storybook",
        }
        const socketOrigin = local.origin.replace("http:", "ws:")
        const hmr = `${socketOrigin}/?token=owned-vite-token`
        const api = `${socketOrigin}/__vitest_browser_api__?type=tester&rpcId=tester-1&sessionId=owned-session&projectName=storybook&method=run&token=owned-vitest-token`
        await installNetworkGuard(page, runner)
        // Valid controls exercise real upgrades before varying the boundary.
        expect(await connect(page, hmr, ["vite-hmr"])).toEqual({
            instance: true,
            open: 1,
            protocol: "vite-hmr",
        })
        for (const method of ["run", "collect", "none"])
            expect(await connect(page, api.replace("method=run", `method=${method}`))).toEqual({
                instance: true,
                open: 1,
                protocol: "",
            })
        expect(local.upgrades).toHaveLength(4)
        expect(await collectNetworkViolations(page)).toEqual([])

        const rejected = [
            {url: `${socketOrigin}/application`, protocols: []},
            {url: `${socketOrigin}/application.js`, protocols: []},
            {url: external.origin.replace("http:", "ws:") + "/application", protocols: []},
            {url: hmr, protocols: ["application"]},
            {url: hmr.replace("owned-vite-token", "wrong"), protocols: ["vite-hmr"]},
            {url: `${hmr}&application=true`, protocols: ["vite-hmr"]},
            {url: api.replace("owned-session", "other-session"), protocols: []},
            {url: api.replace("owned-vitest-token", "wrong"), protocols: []},
            {url: api.replace("method=run", "method=unexpected"), protocols: []},
            {url: api, protocols: ["application"]},
        ]
        await installNetworkGuard(page, runner)
        for (const {url, protocols} of rejected) await connect(page, url, protocols)
        expect
            .soft(await collectNetworkViolations(page))
            .toEqual(rejected.map(({url}) => `Unexpected WebSocket: ${url}`))
        expect.soft(local.upgrades).toHaveLength(4)
        expect.soft(external.upgrades).toEqual([])

        // A future document is guarded immediately, including a caught attempt
        // followed by navigation before the next story's hook runs.
        await page.goto(local.origin)
        await connect(page, `${socketOrigin}/caught-before-navigation`)
        await page.goto(local.origin)
        await installNetworkGuard(page, runner)
        await connect(page, hmr, ["vite-hmr"])
        expect(await collectNetworkViolations(page)).toEqual([
            `Unexpected WebSocket: ${socketOrigin}/caught-before-navigation`,
        ])
        expect(local.upgrades).toHaveLength(5)
        // Reusing the page consumes earlier violations and does not wrap twice.
        await installNetworkGuard(page, runner)
        await connect(page, `${socketOrigin}/next-story`)
        expect(await collectNetworkViolations(page)).toEqual([
            `Unexpected WebSocket: ${socketOrigin}/next-story`,
        ])
        expect(local.upgrades).toHaveLength(5)
    } finally {
        await page.close()
        await local.close()
        await external.close()
    }
})

test("Storybook permits asset reads but rejects writes to an asset-looking URL", async ({page}) => {
    const local = await ownedServer()
    try {
        await page.goto(local.origin)
        await installNetworkGuard(page, {
            origin: local.origin,
            hmrToken: "owned-vite-token",
            apiToken: "owned-vitest-token",
            sessionId: "owned-session",
            projectName: "storybook",
        })
        for (const method of ["GET", "HEAD"])
            expect(
                await page.evaluate(
                    async (method) => (await fetch("/module.js", {method})).status,
                    method
                )
            ).toBe(200)
        expect(
            await page.evaluate(() =>
                fetch("/module.js", {method: "POST"}).then(
                    () => true,
                    () => false
                )
            )
        ).toBe(false)
        expect(await collectNetworkViolations(page)).toEqual([`POST ${local.origin}/module.js`])
        expect(local.requests.filter((request) => request.endsWith("/module.js"))).toEqual([
            "GET /module.js",
            "HEAD /module.js",
        ])
    } finally {
        await page.close()
        await local.close()
    }
})
