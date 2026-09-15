// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {chromium, test} from "@playwright/test"
import {appendFileSync, readFileSync, writeFileSync} from "node:fs"
import {dirname, join} from "node:path"
import {castBallotAsVoter} from "./flow"

interface Workload {
    journey_timeout_ms?: number
    action_timeout_ms?: number
    start: number
    count: number
    shard_size: number
    vus: number
    username_prefix: string
    login_fields?: Record<string, string>
    login_url: string
    election_id: string
    election_event_id: string
    allowed_origins: string[]
    candidates_pattern?: string
}

/** Run full UI journeys with one fresh context per voter and no cast retries. */
test("finite Chromium voting shard", async () => {
    const config: Workload = JSON.parse(readFileSync(process.env.LOAD_CONFIG!, "utf8"))
    const shard = Number(process.env.LOAD_SHARD)
    const first = config.start + shard * config.shard_size
    const count = Math.min(config.shard_size, config.count - shard * config.shard_size)
    const journeyTimeout = config.journey_timeout_ms ?? 180_000
    test.setTimeout(Math.max(journeyTimeout, Math.ceil(count / config.vus) * journeyTimeout))
    const traffic: Record<string, number> = {}
    const browser = await chromium.launch({
        headless: true,
        executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    })
    let next = 0
    let failures = 0
    try {
        await Promise.all(
            Array.from({length: Math.min(config.vus, count)}, async () => {
                while (next < count) {
                    const index = first + next++
                    const start = Date.now()
                    const context = await browser.newContext()
                    context.setDefaultTimeout(config.action_timeout_ms ?? 15_000)
                    let passed = false
                    let receipt: string | null = null
                    let castMs: number | null = null
                    let statusMs: number | null = null
                    const pending: Promise<void>[] = []
                    const timings: Record<string, number> = {}
                    const publicationNames = new Map<string, string>()
                    const requests: {url: string; start: number; duration: number}[] = []
                    context.on("requestfinished", (request) => {
                        const timing = request.timing()
                        if (timing.responseEnd >= 0)
                            requests.push({
                                url: request.url(),
                                start: timing.startTime,
                                duration: timing.responseEnd,
                            })
                    })
                    await context.route("**/*", async (route) => {
                        const allowed = config.allowed_origins.includes(
                            new URL(route.request().url()).origin
                        )
                        if (allowed) await route.continue()
                        else await route.abort("blockedbyclient")
                    })
                    context.on("response", (response) => {
                        const request = response.request()
                        let operation: string | undefined
                        try {
                            const body: unknown = request.postDataJSON()
                            if (
                                body &&
                                typeof body === "object" &&
                                "operationName" in body &&
                                typeof body.operationName === "string"
                            )
                                operation = body.operationName
                        } catch {
                            /* Resource GETs do not have GraphQL bodies. */
                        }
                        const url = new URL(response.url())
                        const name = operation || `${request.method()} ${url.origin}${url.pathname}`
                        traffic[name] = (traffic[name] || 0) + 1
                        if (operation === "GetVoterStatus")
                            pending.push(
                                (async () => {
                                    const body = await response.json()
                                    await response.finished()
                                    statusMs =
                                        request.timing().responseEnd >= 0
                                            ? request.timing().responseEnd
                                            : null
                                    for (const file of body.data?.get_ballot_files_urls?.files ??
                                        [])
                                        for (const name of [
                                            "event_url",
                                            "election_url",
                                            "summary_url",
                                            "style_url",
                                        ])
                                            if (file.urls?.[name])
                                                publicationNames.set(file.urls[name], name)
                                })()
                            )
                        if (operation === "InsertCastVote")
                            pending.push(
                                (async () => {
                                    const body: {
                                        errors?: unknown[]
                                        data?: {
                                            insert_cast_vote?: {
                                                id: string
                                                election_id: string
                                                election_event_id: string
                                            }
                                        }
                                    } = await response.json()
                                    await response.finished()
                                    castMs =
                                        request.timing().responseEnd >= 0
                                            ? request.timing().responseEnd
                                            : null
                                    const cast = body.data?.insert_cast_vote
                                    if (
                                        response.ok() &&
                                        !body.errors &&
                                        cast?.election_id === config.election_id &&
                                        cast.election_event_id === config.election_event_id
                                    ) {
                                        receipt = cast.id
                                    }
                                })()
                            )
                    })
                    try {
                        const page = await context.newPage()
                        const ids = await castBallotAsVoter(page, {
                            loginUrl: config.login_url,
                            castTimeoutMs: journeyTimeout,
                            credentials: {
                                ...config.login_fields,
                                username: config.username_prefix + index,
                                password: process.env.LOAD_PASSWORD!,
                            },
                            candidatesPattern: config.candidates_pattern,
                        })
                        await Promise.all(pending)
                        passed = ids.length === 1 && receipt !== null
                    } catch {
                        // Failure bodies can contain tokens and votes; the public report records counts.
                    } finally {
                        await Promise.allSettled(pending)
                        await context.close()
                        let authStart: number | undefined
                        let tokenEnd: number | undefined
                        for (const request of requests) {
                            const path = new URL(request.url).pathname
                            const phase =
                                publicationNames.get(request.url) ??
                                (path.endsWith("/protocol/openid-connect/auth")
                                    ? "auth"
                                    : path.includes("/login-actions/authenticate")
                                      ? "login"
                                      : path.endsWith("/protocol/openid-connect/token")
                                        ? "token"
                                        : undefined)
                            if (phase) timings[phase] = (timings[phase] ?? 0) + request.duration
                            if (phase === "auth")
                                authStart = Math.min(authStart ?? request.start, request.start)
                            if (phase === "token")
                                tokenEnd = Math.max(tokenEnd ?? 0, request.start + request.duration)
                        }
                        if (
                            authStart !== undefined &&
                            tokenEnd !== undefined &&
                            tokenEnd >= authStart
                        )
                            timings.keycloak_ms = tokenEnd - authStart
                        if (!passed) failures++
                        appendFileSync(
                            process.env.LOAD_RESULTS!,
                            JSON.stringify({
                                index,
                                passed,
                                start,
                                end: Date.now(),
                                cast_ms: castMs,
                                status_ms: statusMs,
                                timings,
                                receipt,
                            }) + "\n"
                        )
                    }
                }
            })
        )
    } finally {
        await browser.close()
        writeFileSync(
            join(dirname(process.env.LOAD_RESULTS!), "traffic.json"),
            JSON.stringify(traffic)
        )
    }
    if (failures) throw new Error(`${failures} voter journeys failed`)
})
