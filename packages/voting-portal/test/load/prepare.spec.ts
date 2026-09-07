// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {chromium, expect, test} from "@playwright/test"
import {readFileSync, writeFileSync} from "node:fs"
import {resolve} from "node:path"
import {castBallotAsVoter, login} from "./flow"

// Preparation is intentionally outside load timing. Every cast route is intercepted
// and rejected locally; a prepared request must never reach Hasura at this stage.
test("prepare unique encrypted ballots without casting", async () => {
    const input = JSON.parse(readFileSync(process.env.PREPARE_INPUT!, "utf8"))
    test.setTimeout(Math.max(180000, input.voters.length * 60000))
    const target = input.target
    const browser = await chromium.launch({
        headless: true,
        executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    })
    const prepared: any[] = []
    try {
        let next = 0
        const workers = await Promise.allSettled(
            Array.from(
                {length: Math.min(input.concurrency || 1, input.voters.length)},
                async () => {
                    while (next < input.voters.length) {
                        const index = next++
                        const voter = input.voters[index]
                        const context = await browser.newContext()
                        const page = await context.newPage()
                        let captured: any
                        let status: any
                        let rejectCapture: (reason: Error) => void = () => {}
                        const ready = new Promise<void>((resolveCapture, reject) => {
                            rejectCapture = reject
                            context
                                .route("**/*", async (route) => {
                                    const request = route.request()
                                    if (
                                        !target.allowed_origins.includes(
                                            new URL(request.url()).origin
                                        )
                                    ) {
                                        await route.abort("blockedbyclient")
                                        reject(new Error("Unexpected preparation origin"))
                                        return
                                    }
                                    let payload: any
                                    try {
                                        payload = request.postDataJSON()
                                    } catch {
                                        /* not JSON */
                                    }
                                    if (payload?.operationName === "InsertCastVote") {
                                        captured = {
                                            url: request.url(),
                                            payload,
                                            authorization:
                                                await request.headerValue("authorization"),
                                        }
                                        await route.fulfill({
                                            status: 200,
                                            contentType: "application/json",
                                            body: JSON.stringify({
                                                errors: [
                                                    {message: "Prepared locally; not submitted"},
                                                ],
                                            }),
                                        })
                                        resolveCapture()
                                    } else if (payload?.query?.trim().startsWith("mutation")) {
                                        await route.abort("blockedbyclient")
                                        reject(new Error("Unexpected mutation during preparation"))
                                    } else await route.continue()
                                })
                                .catch(reject)
                        })
                        const observed = page
                            .waitForResponse((response) => {
                                try {
                                    return (
                                        response.request().postDataJSON()?.operationName ===
                                        "GetVoterStatus"
                                    )
                                } catch {
                                    return false
                                }
                            })
                            .then(async (response) => {
                                expect(response.ok()).toBe(true)
                                const body = await response.json()
                                expect(body.errors).toBeUndefined()
                                status = body.data.get_ballot_files_urls
                                expect(status.event_id).toBe(target.election_event_id)
                                expect(status.files).toHaveLength(1) // One prepared cast per fixture entry.
                                return response
                            })
                        let journey: Promise<unknown> | undefined
                        try {
                            if (input.refresh) {
                                await page.goto(target.login_url)
                                await login(page, voter)
                                const response = await observed
                                const old = input.refresh[index]
                                expect(status.files[0].version).toBe(old.publication_version)
                                expect(status.files[0].id).toBe(old.style_id)
                                captured = {
                                    ...old,
                                    authorization: await response
                                        .request()
                                        .headerValue("authorization"),
                                }
                            } else {
                                journey = castBallotAsVoter(page, {
                                    loginUrl: target.login_url,
                                    credentials: voter,
                                })
                                void journey.catch((error) => {
                                    if (!captured)
                                        rejectCapture(new Error("Preparation journey failed"))
                                })
                                await Promise.all([ready, observed])
                            }
                            expect(captured.authorization).toMatch(/^Bearer /)
                            expect(captured.payload.variables.electionId).toBe(
                                status.files[0].election_id
                            )
                            const claims = JSON.parse(
                                Buffer.from(
                                    captured.authorization.split(".")[1],
                                    "base64url"
                                ).toString()
                            )
                            prepared.push({
                                ...captured,
                                credentials: voter,
                                tenant_id: target.tenant_id,
                                election_event_id: target.election_event_id,
                                style_id: status.files[0].id,
                                publication_version: status.files[0].version,
                                token_expires_at: claims.exp,
                                prepared_at: input.refresh
                                    ? input.refresh[index].prepared_at
                                    : new Date().toISOString(),
                            })
                            writeFileSync(
                                resolve(process.env.PREPARE_OUTPUT!, "ballots.json"),
                                JSON.stringify(prepared),
                                {mode: 0o600}
                            )
                        } finally {
                            await context.close()
                            await journey?.catch(() => {})
                            await observed.catch(() => {})
                        }
                    }
                }
            )
        )
        expect(workers.every((worker) => worker.status === "fulfilled")).toBe(true)
        expect(prepared).toHaveLength(input.voters.length)
        expect(new Set(prepared.map((item) => item.payload.variables.ballotId)).size).toBe(
            prepared.length
        )
    } finally {
        await browser.close()
    }
})
