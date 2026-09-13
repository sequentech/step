// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict"
import {readFile} from "node:fs/promises"
import {createServer, type Server} from "node:http"
import {createRequire} from "node:module"
import {fileURLToPath} from "node:url"
import {after, before, test} from "node:test"
import {build} from "esbuild"
import {chromium, type Browser, type Page} from "playwright-core"
import type {} from "./fixture.tsx"

const require = createRequire(import.meta.url)
let server: Server
let browser: Browser
let page: Page
let origin: string

before(
    async () => {
        const bundle = await build({
            entryPoints: [fileURLToPath(new URL("./fixture.tsx", import.meta.url))],
            bundle: true,
            write: false,
            format: "esm",
            platform: "browser",
            define: {"process.env.NODE_ENV": '"test"'},
            logLevel: "warning",
        })
        const wasm = await readFile(require.resolve("sequent-core/index_bg.wasm"))
        const resources = new Map([
            [
                "/",
                {
                    type: "text/html",
                    body: Buffer.from(
                        '<!doctype html><html lang="en"><body><div id="root"></div><script type="module" src="/fixture.js"></script></body></html>'
                    ),
                },
            ],
            [
                "/fixture.js",
                {type: "text/javascript", body: Buffer.from(bundle.outputFiles[0].contents)},
            ],
            ["/index_bg.wasm", {type: "application/wasm", body: wasm}],
        ])
        server = createServer((request, response) => {
            const resource = resources.get(request.url || "/")
            response.writeHead(resource ? 200 : 404, {
                "Content-Type": resource?.type || "text/plain",
            })
            response.end(resource?.body || "Not found")
        })
        await new Promise<void>((resolve, reject) => {
            server.once("error", reject)
            server.listen(0, "127.0.0.1", resolve)
        })
        const address = server.address()
        assert(address && typeof address !== "string")
        origin = `http://127.0.0.1:${address.port}`
        browser = await chromium.launch({
            executablePath: process.env.UI_CORE_CHROME_PATH,
            channel: process.env.UI_CORE_CHROME_PATH ? undefined : "chrome",
            headless: true,
        })
        page = await browser.newPage()
        // The fixture needs no remote election, identity provider or paid service.
        await page.route("**/*", (route) =>
            route.request().url().startsWith(`${origin}/`) ? route.continue() : route.abort()
        )
        await page.goto(origin)
        await page.getByRole("heading", {name: "UI Core browser integration"}).waitFor()
    },
    {timeout: 30_000}
)

after(async () => {
    await browser?.close()
    if (server?.listening)
        await new Promise<void>((resolve, reject) =>
            server.close((error) => (error ? reject(error) : resolve()))
        )
})

test("authored HTML stays readable and cannot execute code in Chromium", async () => {
    const section = page.getByRole("region", {name: "Election instructions"})
    assert.equal(await section.locator("p").textContent(), "Conseil")
    assert.equal(await section.locator("p").getAttribute("lang"), "fr")
    await section.locator("p").click()
    await section.getByText("Unsafe link").click()
    assert.equal(await page.evaluate(() => Reflect.get(window, "injected")), undefined)
    assert.equal(await section.locator("script").count(), 0)
})

test("language cookies round-trip through the browser's actual cookie store", async () => {
    const value = await page.evaluate(() => {
        window.uiCore.setCookie("language preference", "français = yes")
        return window.uiCore.getValueFromCookie("language preference")
    })
    assert.equal(value, "français = yes")
})

test("the real WebAssembly module generates, hashes and decodes its sample ballot", async () => {
    const result = await page.evaluate(async () => {
        const core = window.uiCore
        await core.initCore()
        const ballot = core.generateSampleAuditableBallot()
        if (!ballot) throw new Error("the real WASM sample generator failed")
        const decoded = core.decodeAuditableBallot(ballot)
        return {
            contestCount: ballot.config.contests.length,
            decodedCount: decoded?.length,
            configuredIds: ballot.config.contests.map((contest) => contest.id),
            decodedIds: decoded?.map((contest) => contest.contest_id),
            digest: core.hashBallot(ballot),
            aliasDigest: core.hashBallot512(ballot),
            sampleDigest: ballot.ballot_hash,
            changedDateDigest: core.hashBallot({...ballot, issue_date: "2030-01-01T00:00:00Z"}),
        }
    })
    assert(result.contestCount > 0)
    assert.equal(result.decodedCount, result.contestCount)
    assert.deepEqual(result.decodedIds, result.configuredIds)
    assert.equal(result.digest, result.aliasDigest)
    assert.equal(result.digest, result.sampleDigest)
    // The public receipt is the first 256 bits of SHA-512, rendered as hex.
    assert.match(result.digest, /^[0-9a-f]{64}$/i)
    assert.notEqual(result.digest, result.changedDateDigest)
})

test("malformed encoded contests fail instead of becoming an empty decoded ballot", async () => {
    const rejected = await page.evaluate(() => {
        const core = window.uiCore
        const ballot = core.generateSampleAuditableBallot()
        if (!ballot) throw new Error("missing sample ballot")
        try {
            core.decodeAuditableBallot({...ballot, contests: ["%%%invalid-encoding%%%"]})
            return false
        } catch {
            return true
        }
    })
    assert.equal(rejected, true)
})
