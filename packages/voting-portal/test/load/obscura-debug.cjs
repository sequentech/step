// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// Standalone synthetic repro: no voter credentials, no election mutations.
const {chromium} = require("@playwright/test")
const {spawn} = require("node:child_process")
const {createServer} = require("node:http")
const fs = require("node:fs")
const path = require("node:path")
const output = path.resolve(process.argv[2])
const routed = process.argv.includes("--routed")
fs.mkdirSync(output, {recursive: true, mode: 0o700})
const records = []
const record = (kind, details) => {
    records.push({at: Date.now(), kind, ...details})
    fs.writeFileSync(path.join(output, "events.json"), JSON.stringify(records, null, 2), {
        mode: 0o600,
    })
}
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
async function limit(promise, label, ms = 5000) {
    let timer
    try {
        return await Promise.race([
            promise,
            new Promise((_, reject) => {
                timer = setTimeout(() => reject(new Error(label + " timed out")), ms)
            }),
        ])
    } finally {
        clearTimeout(timer)
    }
}
const html = `<!doctype html><html><head><title>Obscura compatibility probe</title></head><body style="font-family:sans-serif;margin:40px;background:#f2f5f8;color:#14213d"><h1>Obscura compatibility probe</h1><p>This is a synthetic local page. No voter data is used.</p><h2 id="phase">After navigation</h2><div style="padding:24px;background:white;border:2px solid #567"><p id="local"></p><p id="session"></p><p id="request">POST /echo: not started</p></div><script>document.getElementById('local').textContent='localStorage: '+(localStorage.getItem('probe')||'MISSING');document.getElementById('session').textContent='sessionStorage: '+(sessionStorage.getItem('probe')||'MISSING');</script></body></html>`
;(async () => {
    let obscura, browser
    const server = createServer((request, response) => {
        const chunks = []
        request.on("data", (chunk) => chunks.push(chunk))
        request.on("end", () => {
            record("server_request", {
                method: request.method,
                path: request.url,
                body: Buffer.concat(chunks).toString(),
            })
            response.setHeader(
                "Content-Type",
                request.url === "/echo" ? "application/json" : "text/html"
            )
            response.end(
                request.url === "/echo"
                    ? JSON.stringify({
                          method: request.method,
                          received: Buffer.concat(chunks).toString(),
                      })
                    : html
            )
        })
    })
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve))
    const base = `http://127.0.0.1:${server.address().port}`
    try {
        obscura = spawn(
            path.resolve(".cache/obscura/obscura"),
            ["serve", "--port", "19224", "--allow-private-network"],
            {
                stdio: [
                    "ignore",
                    fs.openSync(path.join(output, "server.log"), "w"),
                    fs.openSync(path.join(output, "server-error.log"), "w"),
                ],
            }
        )
        for (let i = 0; i < 100; i++) {
            try {
                if ((await fetch("http://127.0.0.1:19224/json/version")).ok) break
            } catch {}
            await pause(50)
        }
        browser = await chromium.connectOverCDP("http://127.0.0.1:19224")
        const context = await browser.newContext({viewport: {width: 1000, height: 650}})
        const page = await context.newPage()
        page.on("console", (message) => record("console", {text: message.text()}))
        page.on("request", (request) =>
            record("playwright_request", {
                method: request.method(),
                path: new URL(request.url()).pathname,
            })
        )
        if (!process.argv.includes("--no-extra-cdp")) {
            const cdp = await context.newCDPSession(page)
            await cdp.send("Network.enable")
            cdp.on("Network.requestWillBeSent", (event) =>
                record("cdp_network", {method: event.request.method, url: event.request.url})
            )
        }
        await limit(page.goto(base + "/start"), "initial navigation")
        await page.evaluate(() => {
            localStorage.setItem("probe", "PRESENT")
            sessionStorage.setItem("probe", "PRESENT")
            document.getElementById("phase").textContent = "Before navigation: values stored"
            document.getElementById("local").textContent =
                "localStorage: " + localStorage.getItem("probe")
            document.getElementById("session").textContent =
                "sessionStorage: " + sessionStorage.getItem("probe")
        })
        await limit(
            page.screenshot({path: path.join(output, "before-navigation.png")}),
            "before screenshot",
            10000
        )
        await limit(page.goto(base + "/next"), "second navigation")
        record(
            "storage_after_navigation",
            await page.evaluate(() => ({
                local: localStorage.getItem("probe"),
                session: sessionStorage.getItem("probe"),
            }))
        )
        await page.evaluate(() => {
            document.getElementById("local").textContent =
                "localStorage: " + (localStorage.getItem("probe") || "MISSING")
            document.getElementById("session").textContent =
                "sessionStorage: " + (sessionStorage.getItem("probe") || "MISSING")
        })
        record(
            "dom_snapshot",
            await page.evaluate(() => ({
                local: localStorage.getItem("probe"),
                session: sessionStorage.getItem("probe"),
                localText: document.getElementById("local").textContent,
                sessionText: document.getElementById("session").textContent,
            }))
        )
        await page.setViewportSize({width: 1001, height: 650})
        await limit(
            page.screenshot({path: path.join(output, "after-navigation.png")}),
            "after screenshot",
            10000
        )
        if (routed)
            await context.route("**/*", async (route) => {
                record("route_paused", {
                    method: route.request().method(),
                    path: new URL(route.request().url()).pathname,
                })
                await route.continue()
                record("route_continued", {})
            })
        await page.evaluate(
            () => (document.getElementById("request").textContent = "POST /echo: pending")
        )
        await limit(
            page.screenshot({path: path.join(output, "before-fetch.png")}),
            "pending screenshot",
            10000
        )
        await limit(
            page.evaluate((awaitFetch) => {
                document.getElementById("request").textContent = "POST /echo: pending"
                const task = fetch("/echo", {
                    method: "POST",
                    headers: {"Content-Type": "application/json"},
                    body: JSON.stringify({probe: 1}),
                })
                    .then((response) => response.json())
                    .then(
                        (data) =>
                            (document.getElementById("request").textContent =
                                "POST /echo: server received " + data.method)
                    )
                    .catch(
                        (error) =>
                            (document.getElementById("request").textContent =
                                "POST /echo: " + error.name)
                    )
                if (awaitFetch) return task
            }, process.argv.includes("--await-fetch")),
            "start fetch"
        )
        await pause(1000)
        record("fetch_state", {
            text: await limit(
                page.evaluate(() => document.getElementById("request").textContent),
                "read fetch state"
            ),
        })
        await limit(
            page.screenshot({path: path.join(output, "after-fetch.png")}),
            "fetch screenshot",
            10000
        )
        if (
            !records.some(
                (item) =>
                    item.kind === "server_request" &&
                    item.path === "/echo" &&
                    item.method === "POST"
            )
        )
            throw new Error("Synthetic POST did not reach the server")
    } catch (error) {
        record("failure", {message: String(error)})
        process.exitCode = 1
    } finally {
        if (obscura) obscura.kill("SIGKILL")
        await limit(browser?.close() || Promise.resolve(), "browser close", 2000).catch(() => {})
        server.closeAllConnections()
        server.close()
    }
})().catch((error) => {
    record("fatal", {message: String(error)})
    process.exitCode = 1
})
