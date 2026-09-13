// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict"
import {createServer, type Server} from "node:http"
import {fileURLToPath} from "node:url"
import {after, before, beforeEach, test} from "node:test"
import {build} from "esbuild"
import {chromium, type Browser, type Page} from "playwright-core"

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
        const executablePath = process.env.UI_ESSENTIALS_TEST_CHROME_PATH
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
        await page.getByRole("heading", {name: "UI Essentials browser integration"}).waitFor()
    } catch (cause) {
        // A render exception otherwise looks like an unexplained locator timeout.
        throw new Error(
            `UI Essentials fixture did not render. Browser errors: ${browserErrors.join("; ") || "none"}`,
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

// The browser supplies real File/FileList, keyboard events and DataTransfer;
// these complement jsdom's faster unit checks at the import callback boundary.
test(
    "keyboard opens the native file picker and a failed import can be retried",
    {timeout: 20_000},
    async () => {
        const picker = page.waitForEvent("filechooser")
        await page.getByRole("button", {name: "Choose election file"}).focus()
        await page.keyboard.press("Enter")
        await (
            await picker
        ).setFiles({
            name: "broken.json",
            mimeType: "application/json",
            buffer: Buffer.from("invalid JSON"),
        })
        await page.getByRole("alert").waitFor()
        assert.equal(await page.getByRole("alert").textContent(), "Cannot import this file")
        await page.locator('input[type="file"]').setInputFiles({
            name: "valid.json",
            mimeType: "application/json",
            buffer: Buffer.from('{"election":"synthetic"}'),
        })
        await page.waitForFunction(
            () =>
                document.querySelector('[aria-label="Imported file"]')?.textContent === "valid.json"
        )
        assert.equal(await page.getByRole("alert").count(), 0)
        assert.deepEqual(browserErrors, [])
    }
)

test("native drag-and-drop forwards the file to the importer", {timeout: 20_000}, async () => {
    const transfer = await page.evaluateHandle(() => {
        const data = new DataTransfer()
        data.items.add(
            new File(['{"election":"synthetic"}'], "dropped.json", {type: "application/json"})
        )
        return data
    })
    await page.locator("form").dispatchEvent("dragenter", {dataTransfer: transfer})
    await page.locator(".drag-file-element").dispatchEvent("drop", {dataTransfer: transfer})
    await page.waitForFunction(
        () => document.querySelector('[aria-label="Imported file"]')?.textContent === "dropped.json"
    )
    assert.equal(await page.locator(".drag-file-element").count(), 0)
    await transfer.dispose()
    assert.deepEqual(browserErrors, [])
})

test(
    "expansion does not select a list and a checkbox invokes its callback once",
    {timeout: 20_000},
    async () => {
        await page.getByRole("button", {name: "Toggle council"}).click()
        assert.equal(await page.getByLabel("Selection calls").textContent(), "0")
        await page.getByRole("checkbox").check()
        assert.equal(await page.getByLabel("Selection calls").textContent(), "1")
        assert.equal(await page.getByRole("checkbox").isChecked(), true)
        assert.deepEqual(browserErrors, [])
    }
)

test("the real browser countdown reaches zero", {timeout: 20_000}, async () => {
    await page.getByRole("button", {name: "Start countdown"}).click()
    await page.waitForFunction(
        () => document.querySelector('[aria-label="Seconds remaining"]')?.textContent === "2"
    )
    await page.waitForFunction(
        () => document.querySelector('[aria-label="Seconds remaining"]')?.textContent === "0"
    )
    assert.deepEqual(browserErrors, [])
})
