// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {chromium, expect, test} from "@playwright/test"
import {createServer} from "node:http"
import {mkdirSync, readFileSync, writeFileSync} from "node:fs"
import {resolve} from "node:path"

// This synthetic capability gate is deliberately separate from a real voter result.
test("Obscura supports the browser primitives required for capture", async () => {
    const server = createServer((request, response) => {
        if (request.url === "/asset.css") {
            response.setHeader("Content-Type", "text/css")
            response.end("body { color: black }")
        } else {
            response.setHeader("Content-Type", "text/html")
            response.end(
                '<html><head><link rel="stylesheet" href="/asset.css"></head><body><button>Continue</button></body></html>'
            )
        }
    })
    await new Promise<void>((done) => server.listen(0, "127.0.0.1", done))
    const address = server.address() as {port: number}
    const output = resolve(process.env.CAPTURE_OUTPUT_DIR!)
    mkdirSync(output, {recursive: true, mode: 0o700})
    const browser = await chromium.connectOverCDP(process.env.OBSCURA_CDP_URL!)
    const checks: Record<string, boolean> = {}
    const coverage: Record<string, boolean> = {}
    const started = Date.now()
    try {
        const context = await browser.newContext({
            recordHar: {path: resolve(output, "probe.har"), content: "omit"},
        })
        const page = await context.newPage()
        const resources: string[] = []
        context.on("request", (request) => resources.push(request.url()))
        await page.goto(`http://127.0.0.1:${address.port}/`)
        await page.getByRole("button", {name: "Continue"}).click()
        checks.selector = true
        Object.assign(
            checks,
            await page.evaluate(async () => {
                let wasm = false
                try {
                    await WebAssembly.instantiate(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]))
                    wasm = true
                } catch {
                    /* The report must expose an unsupported API, not skip it. */
                }
                let webCrypto = false
                try {
                    webCrypto =
                        (await crypto.subtle.digest("SHA-256", new Uint8Array([1]))).byteLength ===
                        32
                } catch {
                    /* Record failed crypto execution as a failed capability. */
                }
                return {wasm, webCrypto}
            })
        )
        checks.stylesheetObserved = resources.some((url) => url.endsWith("/asset.css"))
        await context.addCookies([
            {name: "isolation", value: "first", url: `http://127.0.0.1:${address.port}`},
        ])
        const second = await browser.newContext()
        checks.contextIsolation =
            (await context.cookies()).some((cookie) => cookie.name === "isolation") &&
            !(await second.cookies()).some((cookie) => cookie.name === "isolation")
        await second.close()
        await context.close()
        const har = JSON.parse(readFileSync(resolve(output, "probe.har"), "utf8"))
        checks.harFlushed = har.log.entries.length >= 2
        coverage.harBodySizesValid = har.log.entries.every(
            (entry: {response: {bodySize: number}}) => entry.response.bodySize >= 0
        )
        expect(Object.values(checks).every(Boolean), JSON.stringify(checks)).toBe(true)
    } finally {
        writeFileSync(
            resolve(output, "probe.json"),
            JSON.stringify(
                {
                    kind: "synthetic_browser_capability",
                    engine: "obscura",
                    version: "0.2.2",
                    playwright_version: JSON.parse(
                        readFileSync(require.resolve("@playwright/test/package.json"), "utf8")
                    ).version,
                    architecture: process.arch,
                    measured_at: new Date().toISOString(),
                    elapsed_ms: Date.now() - started,
                    checks,
                    coverage,
                    full_journey_verified: false,
                },
                null,
                2
            )
        )
        await browser.close()
        await new Promise<void>((done) => server.close(() => done()))
    }
})
