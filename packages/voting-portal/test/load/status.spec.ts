// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {chromium, expect, test} from "@playwright/test"
import {readFileSync, writeFileSync} from "node:fs"
import {resolve} from "node:path"
import {login} from "./flow"

// Authenticate normally, then replay only the observed read-only status query.
// Tokens and signed URLs stay in memory; samples never contain response bodies.
test("measure authenticated GetVoterStatus", async () => {
    const target = JSON.parse(readFileSync(process.env.CAPTURE_TARGET!, "utf8"))
    const output = process.env.CAPTURE_OUTPUT_DIR!
    const {samples = 100, warmup = 10, concurrency = 1, max_p95_ms} = target.performance || {}
    for (const value of [samples, concurrency]) {
        expect(Number.isInteger(value) && value > 0 && value <= 10000).toBe(true)
    }
    expect(Number.isInteger(warmup) && warmup >= 0 && warmup <= 1000).toBe(true)
    expect(concurrency).toBeLessThanOrEqual(32)
    const browser = await chromium.launch({
        headless: true,
        executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    })
    const context = await browser.newContext({
        recordHar: {path: resolve(output, "journey.har"), content: "omit"},
    })
    const requests: object[] = []
    let completed = false
    let measuredStarted = 0
    let elapsed = 0
    const durations: number[] = []
    try {
        await context.route("**/*", (route) =>
            target.allowed_origins.includes(new URL(route.request().url()).origin)
                ? route.continue()
                : route.abort("blockedbyclient")
        )
        const page = await context.newPage()
        const initial = page.waitForResponse((response) => {
            try {
                return response.request().postDataJSON()?.operationName === "GetVoterStatus"
            } catch {
                return false
            }
        })
        await page.goto(target.login_url)
        await login(page, target.credentials)
        const response = await initial
        expect(response.ok()).toBe(true)
        const request = response.request()
        const payload = request.postDataJSON()
        expect(payload.query.trim().startsWith("query GetVoterStatus")).toBe(true)
        const authorization = await request.headerValue("authorization")
        expect(authorization).toBeTruthy()
        expect(target.allowed_origins).toContain(new URL(request.url()).origin)
        let next = 0
        async function sample(phase: string, index: number) {
            const start = performance.now()
            let status = 0,
                bytes = 0,
                valid = false
            try {
                const result = await context.request.post(request.url(), {
                    headers: {authorization: authorization!},
                    data: payload,
                    timeout: 30000,
                    maxRedirects: 0,
                    maxRetries: 0,
                })
                status = result.status()
                const body = await result.body()
                bytes = body.length
                const data = JSON.parse(body.toString())
                const refs = data.data?.get_ballot_files_urls
                valid =
                    result.ok() &&
                    !data.errors?.length &&
                    refs?.event_id === target.election_event_id &&
                    Array.isArray(refs.files) &&
                    refs.files.length > 0 &&
                    refs.files.every(
                        (file: any) =>
                            Object.keys(file.urls || {}).length === 4 &&
                            ["event_url", "election_url", "summary_url", "style_url"].every(
                                (key) => typeof file.urls[key] === "string"
                            )
                    ) &&
                    Array.isArray(data.data?.sequent_backend_cast_vote) &&
                    !body.toString().includes('"ballot_eml"')
                await result.dispose()
            } finally {
                const duration = performance.now() - start
                if (phase === "measured") durations.push(duration)
                requests.push({
                    id: requests.length + 1,
                    index,
                    phase,
                    method: "POST",
                    url: request.url(),
                    operation: "GetVoterStatus",
                    status,
                    valid,
                    timing: {responseEnd: duration},
                    sizes: {responseBodySize: bytes},
                })
            }
            expect(valid).toBe(true)
        }
        for (let index = 0; index < warmup; index++) await sample("warmup", index)
        measuredStarted = performance.now()
        const workers = await Promise.allSettled(
            Array.from({length: concurrency}, async () => {
                while (next < samples) await sample("measured", next++)
            })
        )
        expect(workers.every((worker) => worker.status === "fulfilled")).toBe(true)
        elapsed = performance.now() - measuredStarted
        durations.sort((a, b) => a - b)
        const percentile = (p: number) => {
            const position = ((durations.length - 1) * p) / 100
            const lower = Math.floor(position),
                upper = Math.ceil(position)
            return durations[lower] + (durations[upper] - durations[lower]) * (position - lower)
        }
        const metrics = {
            samples: durations.length,
            warmup,
            concurrency,
            elapsed_ms: elapsed,
            requests_per_second: (durations.length * 1000) / elapsed,
            p50_ms: percentile(50),
            p95_ms: percentile(95),
            p99_ms: percentile(99),
            min_ms: durations[0],
            max_ms: durations[durations.length - 1],
            errors: 0,
            scope: "one authenticated voter; authentication excluded; closed-loop API requests; no S3 downloads",
        }
        writeFileSync(resolve(output, "performance.json"), JSON.stringify(metrics, null, 2), {
            mode: 0o600,
        })
        if (max_p95_ms !== undefined) expect(metrics.p95_ms).toBeLessThanOrEqual(max_p95_ms)
        completed = true
    } finally {
        await context.close()
        writeFileSync(
            resolve(output, "capture.json"),
            JSON.stringify(
                {
                    schema_version: 1,
                    mode: "status",
                    completed,
                    elapsed_ms:
                        elapsed || (measuredStarted ? performance.now() - measuredStarted : 0),
                    requests,
                    casts: [],
                    browser_version: browser.version(),
                },
                null,
                2
            ),
            {mode: 0o600}
        )
        await browser.close()
    }
})
