// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Machine-checks the reviewer recipes in `docs/REPRODUCE.md` — the same
// three findings the *-e2e-pipeline scripts confirm, but driven the way a
// human reviewer drives them: configuration is set through the **Policy
// overrides panel UI** (not `window.__store.dispatch`), and the booth is
// entered from the **voter page**.
//
// Why a separate script from the *-e2e-pipeline runs, given they share the
// same Playwright mechanism (chromium channel:chrome, headless, :5173)?
// Because they exercise a DIFFERENT config path. The pipeline scripts write
// config into the *persisted* Redux store, so they can navigate with
// `page.goto` freely. The Policy overrides overlay is deliberately
// **ephemeral, module-level, per-tab** (policyOverridesStore.ts) — a full
// document load (any `page.goto` under Vite) wipes it. So from the moment
// the panel is set until the tally/pipeline is read, this script navigates
// **client-side only** (Shell's `<Link>` nav + the inspector rail links),
// exactly as a reviewer would. Confirming the panel path works is the whole
// point: e.g. the over-vote recipe can only select two candidates in a
// max-1 contest if the panel override actually reaches the live voting
// screen.
//
// Requires the dev server on :5173.

import {createRequire} from "node:module"
import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const here = path.dirname(fileURLToPath(import.meta.url))
const base = "http://localhost:5173"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const SNAPSHOT = "bundled:explicit-blank-invalid"

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()

// --- observation helpers (shared with the *-e2e-pipeline scripts) ----------

const warnIds = () =>
    page
        .evaluate(() =>
            Array.from(document.querySelectorAll("[data-warn-id]")).map((el) =>
                el.getAttribute("data-warn-id")
            )
        )
        .catch(() => [])

const dialogKind = () =>
    page.evaluate(() => {
        const d = document.querySelector('[role="dialog"]')
        if (!d) return "none"
        const btns = Array.from(d.querySelectorAll("button")).map((b) => b.innerText)
        return btns.some((b) => /continue/i.test(b)) ? "dismissible" : "blocking"
    })

// --- setup: full-load reset + load fixture through the UI -------------------
// This is the ONLY place a full document load is allowed. Everything after
// panel configuration is client-side so the ephemeral overrides survive.

async function resetAndLoadFixture() {
    await page.goto(base + "/wb", {waitUntil: "networkidle", timeout: 60000})
    await page.evaluate(() => window.__resetWorkbench && window.__resetWorkbench())
    await page.waitForTimeout(1800)
    await page.goto(base + "/wb/snapshot/" + encodeURIComponent(SNAPSHOT), {
        waitUntil: "networkidle",
        timeout: 60000,
    })
    await page.waitForTimeout(900)
    await page
        .getByRole("button", {name: /load this snapshot|^load$|reload/i})
        .first()
        .click()
        .catch(() => {})
    await page.waitForTimeout(3000)
}

async function contestIds() {
    return page.evaluate((electionId) => {
        const bs = window.__store.getState().ballotStyles[electionId]
        const referendum = bs.ballot_eml.contests.find((x) =>
            x.candidates.some((cd) => cd.presentation?.is_explicit_blank)
        )
        const council = bs.ballot_eml.contests.find((x) =>
            x.candidates.some((cd) => cd.presentation?.is_explicit_invalid)
        )
        return {referendum: referendum.id, council: council.id}
    }, ELECTION)
}

async function firstVoterId() {
    // Voters live in the workbench overlay (persisted to the auto-resume
    // slot), not the Redux store, so read them from localStorage.
    return page.evaluate(() => {
        const raw = localStorage.getItem("workbench:state:v1")
        const snap = raw ? JSON.parse(raw) : null
        return snap?.workbench?.voters?.[0]?.id ?? null
    })
}

// --- client-side navigation (no document load) -----------------------------

async function clickLink(href) {
    await page.locator(`a[href="${href}"]`).first().click()
    await page.waitForTimeout(1200)
}

// Set the Policy overrides panel on the current contest page. `selects` maps
// an aria-label to an option value; `bounds` maps a bound key to a string.
async function setPanel(selects = {}, bounds = {}) {
    for (const [label, value] of Object.entries(selects)) {
        await page.locator(`select[aria-label="${label} override"]`).selectOption(value)
        await page.waitForTimeout(200)
    }
    for (const [key, value] of Object.entries(bounds)) {
        const input = page.locator(`input[aria-label="${key} override"]`)
        await input.fill(String(value))
        await page.waitForTimeout(200)
    }
}

// Enter the booth from the voter page, run `selectFn` on the voting screen,
// then walk review → cast → finish. Returns what the voter was (not) shown.
async function castViaBooth(voterId, selectFn) {
    await clickLink(`/wb/voter/${voterId}`)
    await page
        .getByRole("button", {name: /cast a ballot in|recast in/i})
        .first()
        .click()
        .catch(() => {})
    await page.waitForTimeout(1500)
    // StartScreen
    for (const rx of [/start voting/i, /^vote/i, /continue/i, /next/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(1600)
    await selectFn()
    // Capture the live selection count NOW, on the voting screen — after
    // cast the booth clears ballotSelections, so a post-cast read is always 0.
    const selectedAtVote = await page.evaluate((electionId) => {
        const sel = window.__store.getState().ballotSelections[electionId] ?? []
        return sel.reduce(
            (n, c) => n + (c.choices ?? []).filter((ch) => ch.selected === 0).length,
            0
        )
    }, ELECTION)
    const inlineAtVote = await warnIds()

    let transitionDialog = "none"
    for (let i = 0; i < 4; i++) {
        for (const rx of [/next/i, /review/i, /continue/i]) {
            const b = page.getByRole("button", {name: rx}).first()
            if (await b.count().catch(() => 0)) {
                await b.click().catch(() => {})
                break
            }
        }
        await page.waitForTimeout(1400)
        const k = await dialogKind()
        if (k !== "none") transitionDialog = k
        if (page.url().includes("/review")) break
    }
    const inlineAtReview = await warnIds()
    for (const rx of [/^cast/i, /cast ballot/i, /confirm/i, /finish/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(2200)
    for (const rx of [/^cast/i, /confirm/i, /accept/i, /^yes/i, /ok/i]) {
        const b = page.getByRole("button", {name: rx}).last()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            await page.waitForTimeout(2800)
            break
        }
    }
    const silent =
        inlineAtVote.length === 0 &&
        inlineAtReview.length === 0 &&
        transitionDialog === "none"
    return {selectedAtVote, inlineAtVote, inlineAtReview, transitionDialog, silent}
}

async function waitForDecrypt(contestId) {
    for (let i = 0; i < 20; i++) {
        const v = await page.evaluate((cid) => {
            const raw = localStorage.getItem("workbench:state:v1")
            const snap = raw ? JSON.parse(raw) : null
            const rep = snap?.workbench?.repairedCastVotes ?? {}
            for (const entry of Object.values(rep)) {
                const d = entry?.decodedBigInts?.[cid]
                if (d !== undefined && d !== null) return String(d)
            }
            return null
        }, contestId)
        if (v !== null) return v
        await page.waitForTimeout(1000)
    }
    return null
}

// Read the ContestResult that "Run tally" writes into the output textarea.
async function readTallyResult() {
    return page.evaluate(() => {
        for (const a of Array.from(document.querySelectorAll("textarea"))) {
            const v = a.value || ""
            if (v.includes("total_valid_votes") || v.includes("invalid_votes")) {
                try {
                    return JSON.parse(v)
                } catch {
                    /* keep looking */
                }
            }
        }
        return null
    })
}

// From the inspector rail, open a contest → Open in tally → Run tally.
async function tallyFromContest(contestId) {
    await clickLink("/wb") // Shell "Snapshots" — back to the inspector
    await clickLink(`/wb/contest/${contestId}`)
    await page.getByRole("button", {name: /open in tally/i}).first().click().catch(() => {})
    await page.waitForTimeout(2200)
    await page.getByRole("button", {name: /run tally/i}).first().click().catch(() => {})
    await page.waitForTimeout(3000)
    return readTallyResult()
}

const results = {}

// ---------------------------------------------------------------------------
// Recipe 1 — over-vote silently discarded (Part 1, Recipe 1)
// ---------------------------------------------------------------------------
{
    await resetAndLoadFixture()
    const {council} = await contestIds()
    const voter = await firstVoterId()
    await clickLink(`/wb/contest/${council}`)
    await setPanel({
        "Over-vote policy": "allowed",
        "Invalid-vote policy": "allowed",
    })
    const booth = await castViaBooth(voter, async () => {
        await page.getByText(/^Ada$/).first().click().catch(() => {})
        await page.waitForTimeout(400)
        await page.getByText(/^Bruno$/).first().click().catch(() => {})
        await page.waitForTimeout(600)
    })
    // Selecting a 2nd candidate in a max-1 contest is only possible if the
    // panel's over_vote_policy=allowed override reached the live voting
    // screen — this is the panel-path-specific signal.
    const formed = booth.selectedAtVote
    await waitForDecrypt(council)
    const tally = await tallyFromContest(council)
    const notCounted =
        tally != null && tally.total_valid_votes === 0 && tally.invalid_votes?.implicit >= 1
    const pass = formed > 1 && booth.silent && notCounted
    results.overvote = {formed, silent: booth.silent, tally, notCounted, pass}
    console.log(
        `Recipe 1 over-vote: panel set → formed=${formed} selections (>1 proves the ` +
            `override reached the voting screen), silent=${booth.silent}, ` +
            `tally(valid=${tally?.total_valid_votes},impl=${tally?.invalid_votes?.implicit}) → PASS=${pass}`
    )
}

// ---------------------------------------------------------------------------
// Recipe 2, variant d — deliberate blank below min_votes (S2)
// ---------------------------------------------------------------------------
{
    await resetAndLoadFixture()
    const {referendum} = await contestIds()
    const voter = await firstVoterId()
    await clickLink(`/wb/contest/${referendum}`)
    await setPanel({"Invalid-vote policy": "allowed"}, {min_votes: 2})
    const booth = await castViaBooth(voter, async () => {
        await page
            .getByText("Blank vote (explicit blank)", {exact: true})
            .first()
            .click()
            .catch(() => {})
        await page.waitForTimeout(600)
    })
    await waitForDecrypt(referendum)
    const tally = await tallyFromContest(referendum)
    const notCounted =
        tally != null && tally.total_valid_votes === 0 && tally.invalid_votes?.implicit >= 1
    const pass = booth.silent && notCounted
    results.minvote_s2 = {silent: booth.silent, tally, notCounted, pass}
    console.log(
        `Recipe 2d min-vote/S2: panel min_votes=2 + invalid=allowed, blank marker only → ` +
            `silent=${booth.silent}, tally(valid=${tally?.total_valid_votes},impl=${tally?.invalid_votes?.implicit}) → PASS=${pass}`
    )
}

// ---------------------------------------------------------------------------
// Recipe S5 — spoiled ballot leaks the choice (Part 2), via the pipeline
// ---------------------------------------------------------------------------
{
    await resetAndLoadFixture()
    const {council} = await contestIds()
    const voter = await firstVoterId()
    await clickLink(`/wb/contest/${council}`)
    await setPanel({"Invalid-vote policy": "allowed"})
    const booth = await castViaBooth(voter, async () => {
        await page.getByText(/^Ada$/).first().click().catch(() => {})
        await page.waitForTimeout(400)
        await page
            .getByText("Null vote (explicit invalid)", {exact: true})
            .first()
            .click()
            .catch(() => {})
        await page.waitForTimeout(600)
    })
    await waitForDecrypt(council)

    // Open in ballot pipeline (client-side), decrypt+decode the real cast
    // ciphertext, and read the recovered plaintext.
    await clickLink("/wb")
    await clickLink(`/wb/contest/${council}`)
    await page.getByRole("button", {name: /open in ballot pipeline/i}).first().click().catch(() => {})
    await page.waitForTimeout(2000)
    await page.getByRole("button", {name: /decrypt all/i}).first().click().catch(() => {})
    await page.waitForTimeout(2000)
    const decryptedBigInt = await page.evaluate(() => {
        for (const a of Array.from(document.querySelectorAll("textarea"))) {
            const v = (a.value || "").trim()
            if (/^\d+$/.test(v)) return v
        }
        return null
    })
    await page.getByRole("button", {name: /decode all/i}).first().click().catch(() => {})
    await page.waitForTimeout(2000)
    const decoded = await page.evaluate(() => {
        for (const a of Array.from(document.querySelectorAll("textarea"))) {
            const v = a.value || ""
            if (v.includes("is_explicit_invalid") && v.includes("choices")) {
                try {
                    return JSON.parse(v)
                } catch {
                    /* keep looking */
                }
            }
        }
        return null
    })
    const decodedFlag = decoded?.is_explicit_invalid === true
    const decodedSelected = (decoded?.choices ?? []).filter((c) => c.selected === 0).length
    const choicePreserved = decodedFlag && decodedSelected >= 1 && decryptedBigInt === "3"

    await page.getByRole("button", {name: /send to tally/i}).first().click().catch(() => {})
    await page.waitForTimeout(2000)
    await page.getByRole("button", {name: /run tally/i}).first().click().catch(() => {})
    await page.waitForTimeout(3000)
    const tally = await readTallyResult()
    const spoiled =
        tally != null && tally.total_valid_votes === 0 && tally.invalid_votes?.explicit >= 1
    const pass = booth.silent !== undefined && choicePreserved && spoiled
    results.s5 = {
        decryptedBigInt,
        decodedFlag,
        decodedSelected,
        choicePreserved,
        tally,
        spoiled,
        pass,
    }
    console.log(
        `Recipe S5: panel invalid=allowed, Ada+null cast → decrypt bigint=${decryptedBigInt} ` +
            `(3 = null flag + Ada), decoded is_explicit_invalid=${decodedFlag} ` +
            `regular_selections=${decodedSelected}, spoiled=${spoiled} → PASS=${pass}`
    )
}

await browser.close()

const allPass = Object.values(results).every((r) => r.pass)
console.log(`\nall reviewer recipes verified through the panel UI: ${allPass}`)
writeFileSync(
    path.join(here, "reproduce-verify.recorded.json"),
    JSON.stringify({config_path: "policy-overrides-panel-ui", results, all_pass: allPass}, null, 2) + "\n"
)
console.log("wrote reproduce-verify.recorded.json")
if (!allPass) process.exitCode = 1
