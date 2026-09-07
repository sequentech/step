// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {chromium, test, expect} from "@playwright/test"
import {readFileSync, appendFileSync} from "node:fs"
import {CastReceipt, PreparedBallot} from "./ballotTypes"

// Browser-transport cast load. Authentication/encryption are prepared beforehand;
// full login-to-confirmation UI coverage stays in capture.spec.ts.
test("cast disjoint prepared ballots through a browser", async () => {
    const config = JSON.parse(readFileSync(process.env.LOAD_CONFIG!, "utf8"))
    test.setTimeout(
        Math.max(180000, config.start_at_ms - Date.now() + config.duration_seconds * 1000 + 60000)
    )
    const ballots: PreparedBallot[] = JSON.parse(readFileSync(process.env.LOAD_BALLOTS!, "utf8"))
    if (config.engine !== "chromium") throw new Error("Browser transport requires Chromium")
    const browser = await chromium.launch({
        headless: true,
        executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    })
    const pending: Promise<void>[] = []
    let active = 0
    const records: {kind: string; accepted?: boolean}[] = []
    const context = await browser.newContext()
    const page = await context.newPage()
    const origin = new URL(config.login_url).origin
    const runnerUrl = origin + "/favicon.svg"
    await context.route("**/*", (route) => {
        if (!config.allowed_origins.includes(new URL(route.request().url()).origin))
            return route.abort("blockedbyclient")
        return route.continue()
    })
    try {
        await page.goto(runnerUrl)
        expect(Date.now()).toBeLessThan(config.start_at_ms)
        for (let index = 0; index < config.rate * config.duration_seconds; index++) {
            const due = config.start_at_ms + (index * 1000) / config.rate
            await new Promise((resolve) => setTimeout(resolve, Math.max(0, due - Date.now())))
            if (active >= config.vus || Date.now() - due > 1000 / config.rate) {
                const record = {kind: "dropped", index}
                records.push(record)
                appendFileSync(process.env.LOAD_SAMPLES!, JSON.stringify(record) + "\n", {
                    mode: 0o600,
                })
                continue
            }
            active++
            pending.push(
                (async () => {
                    const ballot = ballots[index]
                    const record = await page.evaluate(
                        ({ballot, index}) => {
                            const perform = async () => {
                                const started = Date.now()
                                let status = 0,
                                    cast: CastReceipt | null = null,
                                    bytes = 0
                                try {
                                    const response = await fetch(ballot.url, {
                                        method: "POST",
                                        redirect: "error",
                                        credentials: "omit",
                                        headers: {
                                            "Content-Type": "application/json",
                                            "Authorization": ballot.authorization,
                                        },
                                        body: JSON.stringify(ballot.payload),
                                        signal: AbortSignal.timeout(30000),
                                    })
                                    status = response.status
                                    const text = await response.text()
                                    bytes = new TextEncoder().encode(text).length
                                    const body = JSON.parse(text)
                                    if (response.ok && !body.errors?.length)
                                        cast = body.data?.insert_cast_vote
                                } catch (_) {
                                    /* Never log credential-bearing errors or responses. */
                                }
                                const accepted = !!(
                                    cast?.id &&
                                    cast.ballot_id === ballot.payload.variables.ballotId &&
                                    cast.tenant_id === ballot.tenant_id &&
                                    cast.election_event_id === ballot.election_event_id &&
                                    cast.election_id === ballot.payload.variables.electionId
                                )
                                return {
                                    kind: "cast",
                                    index,
                                    started_at_ms: started,
                                    ended_at_ms: Date.now(),
                                    duration_ms: Date.now() - started,
                                    status,
                                    accepted,
                                    method: "POST",
                                    operation: "InsertCastVote",
                                    endpoint: ballot.url,
                                    response_bytes: bytes,
                                    receipt:
                                        accepted && cast
                                            ? {
                                                  id: cast.id,
                                                  ballot_id: cast.ballot_id,
                                                  tenant_id: cast.tenant_id,
                                                  election_id: cast.election_id,
                                                  election_event_id: cast.election_event_id,
                                              }
                                            : null,
                                }
                            }
                            return perform()
                        },
                        {ballot, index}
                    )
                    records.push(record)
                    appendFileSync(process.env.LOAD_SAMPLES!, JSON.stringify(record) + "\n", {
                        mode: 0o600,
                    })
                })()
                    .catch((error) => {
                        appendFileSync(
                            process.env.LOAD_SAMPLES! + ".errors",
                            String(error) + "\n",
                            {mode: 0o600}
                        )
                        const record = {
                            kind: "runner_error",
                            index,
                            error_type: error?.name || "Error",
                        }
                        records.push(record)
                        appendFileSync(process.env.LOAD_SAMPLES!, JSON.stringify(record) + "\n", {
                            mode: 0o600,
                        })
                    })
                    .finally(() => {
                        active--
                    })
            )
        }
        const settled = await Promise.allSettled(pending)
        expect(settled.every((item) => item.status === "fulfilled")).toBe(true)
        expect(records.every((item) => item.kind === "cast" && item.accepted)).toBe(true)
    } finally {
        await context.close()
        await browser.close()
    }
})
