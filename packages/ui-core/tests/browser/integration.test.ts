// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict"
import {createHash} from "node:crypto"
import {readFile} from "node:fs/promises"
import {createServer, type Server} from "node:http"
import {createRequire} from "node:module"
import {fileURLToPath} from "node:url"
import {after, before, test} from "node:test"
import {build} from "esbuild"
import {chromium, type Browser, type Page} from "playwright-core"
import type {} from "./fixture.tsx"
import type {EBlankVotePolicy} from "../../src/types/ContestPresentation.ts"

const require = createRequire(import.meta.url)
let server: Server
let browser: Browser
let page: Page
let origin: string
const unexpectedRequests: string[] = []

async function restrictNetwork(target: Page) {
    await target.route("**/*", (route) => {
        if (route.request().url().startsWith(`${origin}/`)) return route.continue()
        unexpectedRequests.push(route.request().url())
        return route.abort()
    })
    await target.routeWebSocket("**/*", (socket) => {
        unexpectedRequests.push(socket.url())
        socket.close()
    })
}

before(
    async () => {
        const bundle = await build({
            entryPoints: [fileURLToPath(new URL("./fixture.tsx", import.meta.url))],
            bundle: true,
            write: false,
            format: "esm",
            platform: "browser",
            define: {"process.env.NODE_ENV": '"test"'},
            logLevel: "warning",
        })
        const wasm = await readFile(require.resolve("sequent-core/index_bg.wasm"))
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
            ["/index_bg.wasm", {type: "application/wasm", body: wasm}],
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
        browser = await chromium.launch({
            executablePath: process.env.UI_CORE_CHROME_PATH,
            channel: process.env.UI_CORE_CHROME_PATH ? undefined : "chrome",
            headless: true,
        })
        page = await browser.newPage()
        await restrictNetwork(page)
        await page.goto(origin)
        await page.getByRole("heading", {name: "UI Core browser integration"}).waitFor()
    },
    {timeout: 30_000}
)

after(async () => {
    await browser?.close()
    if (server?.listening)
        await new Promise<void>((resolve, reject) =>
            server.close((error) => (error ? reject(error) : resolve()))
        )
    assert.deepEqual(unexpectedRequests, [], "unexpected browser network requests")
})

test("authored HTML stays readable and cannot execute code in Chromium", async () => {
    const section = page.getByRole("region", {name: "Election instructions"})
    assert.equal(await section.locator("p").textContent(), "Conseil")
    assert.equal(await section.locator("p").getAttribute("lang"), "fr")
    await section.locator("p").click()
    await section.getByText("Unsafe link").click()
    assert.equal(await page.evaluate(() => Reflect.get(window, "injected")), undefined)
    assert.equal(await section.locator("script").count(), 0)
})

test("language cookies round-trip through the browser's actual cookie store", async () => {
    const value = await page.evaluate(() => {
        window.uiCore.setCookie("language preference", "français = yes")
        return window.uiCore.getValueFromCookie("language preference")
    })
    assert.equal(value, "français = yes")
})

test("the real WebAssembly module generates, hashes and decodes its sample ballot", async () => {
    const result = await page.evaluate(async () => {
        const core = window.uiCore
        await core.initCore()
        const ballot = core.generateSampleAuditableBallot()
        if (!ballot) throw new Error("the real WASM sample generator failed")
        const decoded = core.decodeAuditableBallot(ballot)
        return {
            contestCount: ballot.config.contests.length,
            decodedCount: decoded?.length,
            configuredIds: ballot.config.contests.map((contest) => contest.id),
            decodedIds: decoded?.map((contest) => contest.contest_id),
            publicBallot: core.toHashableBallot(ballot),
            digest: core.hashBallot(ballot),
            aliasDigest: core.hashBallot512(ballot),
            sampleDigest: ballot.ballot_hash,
            changedDateDigest: core.hashBallot({...ballot, issue_date: "2030-01-01T00:00:00Z"}),
        }
    })
    assert(result.contestCount > 0)
    assert.equal(result.decodedCount, result.contestCount)
    assert.deepEqual(result.decodedIds, result.configuredIds)
    // Independently encode the public Borsh envelope and hash it with Node's
    // crypto implementation. Comparing WASM aliases alone cannot detect the
    // wrong digest algorithm, truncation, or envelope serialization.
    const u32 = (value: number) => {
        const bytes = Buffer.alloc(4)
        bytes.writeUInt32LE(value)
        return bytes
    }
    const text = (value: string) => {
        const bytes = Buffer.from(value, "utf8")
        return Buffer.concat([u32(bytes.length), bytes])
    }
    const ballot = result.publicBallot
    // The receipt hashes RawHashableBallot: version, issue date and decoded
    // contest records. Config metadata and voter signatures are outside it.
    const envelope = Buffer.concat([
        u32(ballot.version),
        text(ballot.issue_date),
        u32(ballot.contests.length),
        ...ballot.contests.map((contest) => Buffer.from(contest, "base64")),
    ])
    assert.equal(result.digest, createHash("sha512").update(envelope).digest("hex").slice(0, 64))
    assert.equal(result.digest, result.aliasDigest)
    assert.equal(result.digest, result.sampleDigest)
    // The public receipt is the first 256 bits of SHA-512, rendered as hex.
    assert.match(result.digest, /^[0-9a-f]{64}$/i)
    assert.notEqual(result.digest, result.changedDateDigest)
})

test("malformed encoded contests fail instead of becoming an empty decoded ballot", async () => {
    const rejected = await page.evaluate(async () => {
        const core = window.uiCore
        await core.initCore()
        const ballot = core.generateSampleAuditableBallot()
        if (!ballot) throw new Error("missing sample ballot")
        try {
            core.decodeAuditableBallot({...ballot, contests: ["%%%invalid-encoding%%%"]})
            return false
        } catch {
            return true
        }
    })
    assert.equal(rejected, true)
})

test("encrypting an explicit selection preserves candidate IDs and zero-based selections", async () => {
    const result = await page.evaluate(async () => {
        const core = window.uiCore
        await core.initCore()
        const sample = core.generateSampleAuditableBallot()
        if (!sample) throw new Error("missing sample ballot")
        const config = {
            ...sample.config,
            contests: sample.config.contests.map((contest) => ({
                ...contest,
                counting_algorithm: core.ICountingAlgorithm.PLURALITY_AT_LARGE,
                min_votes: 1,
                max_votes: 1,
                presentation: {
                    ...contest.presentation,
                    blank_vote_policy: core.EBlankVotePolicy.ALLOWED,
                    over_vote_policy: core.EOverVotePolicy.ALLOWED,
                    invalid_vote_policy: core.EInvalidVotePolicy.ALLOWED,
                },
            })),
        }
        const choices = config.contests.map((contest) => ({
            contest_id: contest.id,
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: contest.candidates.map((candidate, index) => ({
                id: candidate.id,
                selected: index === 0 ? 0 : -1,
            })),
        }))
        const ballot = core.encryptBallotSelection(choices, config)
        return {
            expected: choices.map(({contest_id, choices}) => ({contest_id, choices})),
            actual: core.decodeAuditableBallot(ballot)?.map(({contest_id, choices}) => ({
                contest_id,
                choices: choices.map(({id, selected}) => ({id, selected})),
            })),
            hash: core.hashBallot(ballot),
            receipt: ballot.ballot_hash,
        }
    })
    assert(result.expected.length > 0)
    assert(result.expected.every(({choices}) => choices.length > 0))
    assert(result.actual)
    // Candidate IDs identify selections independently of the codec's canonical order.
    const ordered = (contests: typeof result.expected) =>
        contests
            .map((contest) => ({
                ...contest,
                choices: [...contest.choices].sort((a, b) => a.id.localeCompare(b.id)),
            }))
            .sort((a, b) => a.contest_id.localeCompare(b.contest_id))
    assert.deepEqual(ordered(result.actual), ordered(result.expected))
    assert.equal(result.hash, result.receipt)
})

for (const [policy, blocked, warning] of [
    ["allowed", false, false],
    ["warn", false, true],
    ["not-allowed", true, false],
] as const) {
    test(`real WASM maps blank policy ${policy} to the documented screen decision`, async () => {
        const result = await page.evaluate(async (policy) => {
            const core = window.uiCore
            await core.initCore()
            const sample = core.generateSampleAuditableBallot()
            if (!sample) throw new Error("missing sample ballot")
            const original = sample.config.contests[0]
            const contest = {
                ...original,
                counting_algorithm: core.ICountingAlgorithm.PLURALITY_AT_LARGE,
                is_acclaimed: false,
                min_votes: 1,
                max_votes: 1,
                presentation: {
                    ...original.presentation,
                    blank_vote_policy: policy as EBlankVotePolicy,
                    invalid_vote_policy: core.EInvalidVotePolicy.ALLOWED,
                    over_vote_policy: core.EOverVotePolicy.ALLOWED,
                    under_vote_policy: core.EUnderVotePolicy.ALLOWED,
                },
            }
            const selection = {
                contest_id: contest.id,
                is_explicit_invalid: false,
                is_decline_to_vote: false,
                is_blank_ballot: false,
                invalid_errors: [],
                invalid_alerts: [],
                choices: contest.candidates.map((candidate, index) => ({
                    id: candidate.id,
                    selected: index === 0 ? 0 : -1,
                })),
            }
            const decision = () => {
                const decoded = {[contest.id]: selection}
                return {
                    blocked: core.check_voting_not_allowed_next_bool([contest], decoded),
                    warning: core.check_voting_error_dialog_bool([contest], decoded),
                }
            }
            const valid = decision()
            selection.choices.forEach((choice) => (choice.selected = -1))
            return {valid, blank: decision()}
        }, policy)
        assert.deepEqual(result.valid, {blocked: false, warning: false})
        assert.deepEqual(result.blank, {blocked, warning})
    })
}

test("a missing WASM resource moves the actual provider to error rather than ready", async () => {
    await page.getByRole("status", {name: "WASM status"}).filter({hasText: "ready"}).waitFor()
    const failed = await browser.newPage()
    try {
        await restrictNetwork(failed)
        await failed.route(`${origin}/index_bg.wasm`, (route) =>
            route.fulfill({status: 404, contentType: "text/plain", body: "Not found"})
        )
        await failed.goto(origin)
        await failed.getByRole("status", {name: "WASM status"}).filter({hasText: "error"}).waitFor()
        assert.equal(await failed.getByRole("status", {name: "WASM status"}).textContent(), "error")
    } finally {
        await failed.close()
    }
})
