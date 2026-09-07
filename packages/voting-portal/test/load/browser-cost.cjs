// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// Read-only browser transport comparison; never submits a cast.
const {chromium} = require("@playwright/test")
const fs = require("node:fs")
const {spawn} = require("node:child_process")
const path = require("node:path")
const directory = process.argv[2]
const input = JSON.parse(fs.readFileSync(path.join(directory, "input.json")))
const phase = (value) => fs.writeFileSync(path.join(directory, "phase"), value)
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
;(async () => {
    let server, browser
    const start = performance.now()
    phase("startup")
    try {
        if (input.engine === "obscura") {
            server = spawn(
                input.obscura,
                ["serve", "--port", String(input.port), "--allow-private-network", "--quiet"],
                {stdio: "ignore"}
            )
            let ready = false
            for (let attempt = 0; attempt < 100; attempt++) {
                try {
                    ready = (await fetch(`http://127.0.0.1:${input.port}/json/version`)).ok
                } catch {}
                if (ready) break
                await pause(50)
            }
            if (!ready) throw new Error("Obscura did not start")
            browser = await chromium.connectOverCDP(`http://127.0.0.1:${input.port}`)
        } else
            browser = await chromium.launch({
                headless: true,
                executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
            })
        const context = await browser.newContext()
        const page = await context.newPage()
        await page.goto(new URL("/favicon.svg", input.login_url).href)
        const startup = performance.now() - start
        await page.evaluate(() => {
            localStorage.setItem("load-probe", "local")
            sessionStorage.setItem("load-probe", "session")
        })
        await page.reload()
        const storage = await page.evaluate(() => ({
            local: localStorage.getItem("load-probe") === "local",
            session: sessionStorage.getItem("load-probe") === "session",
        }))
        const payload = {
            operationName: "GetVoterStatus",
            query: "query GetVoterStatus($electionEventId: String!) { get_ballot_files_urls(election_event_id: $electionEventId) sequent_backend_cast_vote {id tenant_id election_id election_event_id status}}",
            variables: {electionEventId: input.ballot.election_event_id},
        }
        async function requests(count) {
            return page.evaluate(
                async ({count, ballot, payload}) => {
                    const samples = []
                    for (let index = 0; index < count; index++) {
                        const start = performance.now()
                        const response = await fetch(ballot.url, {
                            method: "POST",
                            credentials: "omit",
                            redirect: "error",
                            headers: {
                                "Content-Type": "application/json",
                                "Authorization": ballot.authorization,
                            },
                            body: JSON.stringify(payload),
                            signal: AbortSignal.timeout(30000),
                        })
                        const body = await response.json()
                        if (
                            !response.ok ||
                            body.errors?.length ||
                            body.data?.get_ballot_files_urls?.event_id !== ballot.election_event_id
                        )
                            throw new Error("Status request failed")
                        samples.push(performance.now() - start)
                    }
                    return samples
                },
                {count, ballot: input.ballot, payload}
            )
        }
        phase("warmup")
        await requests(10)
        phase("measured")
        const samples = await requests(input.samples)
        phase("done")
        fs.writeFileSync(
            path.join(directory, "browser.json"),
            JSON.stringify({
                engine: input.engine,
                startup_ms: startup,
                storage,
                samples,
                browser_version: browser.version(),
            }),
            {mode: 0o600}
        )
        await pause(200)
        await context.close()
    } finally {
        await browser?.close()
        server?.kill()
    }
})().catch(() => {
    phase("failed")
    process.exitCode = 1
})
