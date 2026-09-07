// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {chromium, expect, Request, test} from "@playwright/test"
import {mkdirSync, readFileSync, writeFileSync} from "node:fs"
import {resolve} from "node:path"
import {castBallotAsVoter} from "./flow"

// HAR contains authentication material. The runner creates a private output directory;
// only the extracted resource profile and aggregate report are publication candidates.
test("capture one real login-to-cast journey", async () => {
    const output = resolve(process.env.CAPTURE_OUTPUT_DIR!)
    const target = JSON.parse(readFileSync(process.env.CAPTURE_TARGET!, "utf8"))
    mkdirSync(output, {recursive: true, mode: 0o700})
    const browser = await chromium.connectOverCDP(process.env.OBSCURA_CDP_URL!)
    const context = await browser.newContext({
        recordHar: {path: resolve(output, "journey.har"), mode: "full", content: "omit"},
    })
    const page = await context.newPage()
    const started = Date.now()
    const requests: object[] = []
    const casts: object[] = []
    const pending: Promise<void>[] = []
    const failures: string[] = []
    const ids = new Map<Request, number>()
    let completed = false
    context.on("request", (request) => ids.set(request, ids.size + 1))
    context.on("requestfinished", (request) => {
        pending.push(
            (async () => {
                const response = await request.response()
                const timing = request.timing()
                const sizes = await request.sizes()
                let operation: string | undefined
                try {
                    operation = request.postDataJSON()?.operationName
                } catch {
                    // Form submissions and file requests are not GraphQL JSON bodies.
                }
                requests.push({
                    id: ids.get(request),
                    redirectedFrom: ids.get(request.redirectedFrom()!),
                    method: request.method(),
                    url: request.url(),
                    resourceType: request.resourceType(),
                    operation,
                    status: response?.status(),
                    timing,
                    sizes,
                    fromServiceWorker: response?.fromServiceWorker(),
                })
                if (operation === "InsertCastVote") {
                    const result = await response?.json()
                    if (
                        !response?.ok() ||
                        result.errors?.length ||
                        !result.data?.insert_cast_vote?.id
                    ) {
                        throw new Error("Cast API rejected the ballot")
                    }
                    const {id, ballot_id, tenant_id, election_event_id, election_id} =
                        result.data.insert_cast_vote
                    casts.push({
                        id,
                        ballot_id,
                        tenant_id,
                        election_event_id,
                        election_id,
                        acceptedAt: Date.now(),
                    })
                    // Preserve accepted ballots even if a later election or assertion fails.
                    writeFileSync(resolve(output, "casts.json"), JSON.stringify(casts, null, 2))
                }
            })().catch((error) => {
                failures.push(String(error))
            })
        )
    })
    context.on("requestfailed", (request) => {
        requests.push({
            id: ids.get(request),
            method: request.method(),
            url: request.url(),
            resourceType: request.resourceType(),
            error: request.failure()?.errorText,
        })
    })
    try {
        const ballots = await castBallotAsVoter(page, {
            loginUrl: target.login_url,
            credentials: target.credentials,
        })
        // Drain listeners, including work added while an earlier response was decoded.
        for (let index = 0; index < pending.length; index++) await pending[index]
        expect(failures).toEqual([])
        expect(casts.length).toBe(ballots.length)
        expect(casts.length).toBeGreaterThan(0)
        completed = true
    } finally {
        await context.close() // Flush the successful HAR before disconnecting from CDP.
        for (const request of pending) await request
        writeFileSync(
            resolve(output, "capture.json"),
            JSON.stringify(
                {
                    schema_version: 1,
                    engine: "obscura",
                    started_at_ms: started,
                    elapsed_ms: Date.now() - started,
                    completed: completed && failures.length === 0,
                    failures,
                    requests,
                    casts,
                },
                null,
                2
            )
        )
        await browser.close()
    }
})
