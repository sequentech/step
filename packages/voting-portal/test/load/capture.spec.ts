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
    const engine = target.engine || "chromium"
    if (engine !== "chromium") throw new Error("Full portal capture requires Chromium")
    const browser = await chromium.launch({
        headless: true,
        executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    })
    const context = await browser.newContext({
        recordHar: {path: resolve(output, "journey.har"), mode: "full", content: "omit"},
    })
    if (!Array.isArray(target.allowed_origins) || !target.allowed_origins.length) {
        throw new Error("Capture target must explicitly list allowed_origins")
    }
    const unexpectedOrigins: string[] = []
    await context.route("**/*", async (route) => {
        const origin = new URL(route.request().url()).origin
        if (target.allowed_origins.includes(origin)) await route.continue()
        else {
            unexpectedOrigins.push(origin)
            await route.abort("blockedbyclient")
        }
    })
    const page = await context.newPage()
    const started = Date.now()
    const phases: Record<string, number> = {}
    const requests: object[] = []
    const casts: {ballot_id: string; [key: string]: unknown}[] = []
    let publicationFiles: unknown[] = []
    const pending: Promise<void>[] = []
    const failures: string[] = []
    const ids = new Map<Request, number>()
    let completed = false
    let finished = started
    const browserVersion = browser.version()
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
                    query_payload:
                        operation === "GetVoterStatus" ? request.postDataJSON() : undefined,
                    status: response?.status(),
                    timing,
                    sizes,
                    fromServiceWorker: response?.fromServiceWorker(),
                })
                if (operation === "GetVoterStatus") {
                    const body = await response?.json()
                    publicationFiles = body?.data?.get_ballot_files_urls?.files ?? []
                }
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
            onPhase: (name) => {
                phases[name] = Date.now() - started
            },
        })
        // Drain listeners, including work added while an earlier response was decoded.
        for (let index = 0; index < pending.length; index++) await pending[index]
        expect(failures).toEqual([])
        expect(unexpectedOrigins).toEqual([])
        expect(casts.map((cast) => cast.ballot_id).sort()).toEqual([...ballots].sort())
        expect(casts.length).toBeGreaterThan(0)
        completed = true
        finished = Date.now()
    } finally {
        const diagnostic = await page
            .evaluate(() => ({
                path: location.pathname,
                text: Array.from(
                    document.querySelectorAll("h1,h2,.alert-error,.kc-feedback-text,.error-message")
                ).map((element) => element.textContent),
                controls: Array.from(document.querySelectorAll("input,button")).map((element) => ({
                    tag: element.tagName,
                    type: element.getAttribute("type"),
                    name: element.getAttribute("name"),
                    id: element.id,
                    text: element.tagName === "BUTTON" ? element.textContent : undefined,
                })),
            }))
            .catch(() => null)
        writeFileSync(resolve(output, "diagnostic.json"), JSON.stringify(diagnostic, null, 2))
        await context.close() // Flush the successful HAR before disconnecting from CDP.
        for (const request of pending) await request
        writeFileSync(
            resolve(output, "capture.json"),
            JSON.stringify(
                {
                    schema_version: 1,
                    phases,

                    engine,
                    browser_version: browserVersion,
                    started_at_ms: started,
                    elapsed_ms: (completed ? finished : Date.now()) - started,
                    completed: completed && failures.length === 0,
                    failures,
                    unexpected_origins: unexpectedOrigins,
                    http_cache: "disabled by origin-guard routing",
                    requests,
                    casts,
                    publication_files: publicationFiles,
                },
                null,
                2
            )
        )
        await browser.close()
    }
})
