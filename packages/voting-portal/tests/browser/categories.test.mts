// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict"
import {readFile} from "node:fs/promises"
import {createServer, type Server} from "node:http"
import {createRequire} from "node:module"
import {fileURLToPath} from "node:url"
import {after, before, beforeEach, test} from "node:test"
import {build} from "esbuild"
import {chromium, type Browser, type Page} from "playwright-core"
import type {} from "./fixture.tsx"

const require = createRequire(import.meta.url)
const localFile = (path: string) => fileURLToPath(new URL(path, import.meta.url))
let server: Server
let browser: Browser
let page: Page
let origin: string
const browserErrors: string[] = []

before(
    async () => {
        const bundle = await build({
            entryPoints: [localFile("./fixture.tsx")],
            bundle: true,
            write: false,
            format: "esm",
            platform: "browser",
            logLevel: "warning",
            define: {"process.env.NODE_ENV": '"test"'},
            alias: {
                "@sequentech/ui-core": localFile("../../../ui-core/src/index.tsx"),
                "@sequentech/ui-essentials": localFile("./uiEssentialsEntry.ts"),
            },
        })
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
            [
                "/index_bg.wasm",
                {
                    type: "application/wasm",
                    body: await readFile(require.resolve("sequent-core/index_bg.wasm")),
                },
            ],
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
        const executablePath = process.env.VOTING_PORTAL_TEST_CHROME_PATH
        browser = await chromium.launch({
            executablePath,
            channel: executablePath ? undefined : "chrome",
            headless: true,
        })
        page = await browser.newPage()
        page.setDefaultTimeout(10_000)
        page.on("pageerror", (error) => browserErrors.push(error.message))
        // No authentication, remote election or paid service is part of this fixture.
        await page.route("**/*", (route) =>
            new URL(route.request().url()).origin === origin ? route.continue() : route.abort()
        )
    },
    {timeout: 30_000}
)

beforeEach(async () => {
    browserErrors.length = 0
    await page.goto(origin)
    try {
        await page.getByRole("heading", {name: "Voting Portal browser integration"}).waitFor()
    } catch (cause) {
        // A render exception otherwise looks like an unexplained locator timeout.
        throw new Error(
            `Voting fixture did not render. Browser errors: ${browserErrors.join("; ") || "none"}`,
            {cause}
        )
    }
})

after(async () => {
    await browser?.close()
    if (server?.listening)
        await new Promise<void>((resolve, reject) =>
            server.close((error) => (error ? reject(error) : resolve()))
        )
})

async function expectAllExpanded(expanded: boolean) {
    await page.waitForFunction((expected) => {
        const controls = Array.from(document.querySelectorAll(".candidates-list-toggle"))
        return (
            controls.length === 4 &&
            controls.every((control) => control.getAttribute("aria-expanded") === String(expected))
        )
    }, expanded)
}

test(
    "prototype-like category names honor the initial policy and repeated toggle-all",
    {timeout: 20_000},
    async () => {
        await expectAllExpanded(false)
        await page.getByRole("button", {name: "Expand all", exact: true}).click()
        await expectAllExpanded(true)
        await page.getByRole("button", {name: "Collapse all", exact: true}).click()
        await expectAllExpanded(false)
        assert.deepEqual(browserErrors, [])
        assert.equal(
            await page.evaluate(() =>
                Object.prototype.hasOwnProperty.call(Object.prototype, "name")
            ),
            false
        )
    }
)

test(
    "keyboard expansion and candidate selection use the real components, Redux and WASM",
    {timeout: 20_000},
    async () => {
        const toggle = page.getByRole("button", {name: "Toggle __proto__", exact: true})
        await toggle.focus()
        await page.keyboard.press("Enter")
        const choice = page.getByRole("checkbox", {name: /Candidate 0/})
        await choice.check()
        await page.waitForFunction(() =>
            window.votingTest
                .selection()?.[0]
                .choices.some((choice) => choice.id === "candidate-0" && choice.selected >= 0)
        )
        // Hiding a category is presentation only: it must not silently erase a vote.
        await page.getByRole("button", {name: "Collapse all", exact: true}).click()
        await expectAllExpanded(false)
        assert.equal(
            await page.evaluate(
                () =>
                    window.votingTest
                        .selection()?.[0]
                        .choices.find((choice) => choice.id === "candidate-0")?.selected
            ),
            0
        )
        assert.deepEqual(browserErrors, [])
    }
)
