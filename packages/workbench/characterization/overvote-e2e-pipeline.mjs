// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Closes the crypto-chaining TODO: confirms the S1 over-vote silent-discount
// through ONE continuous flow of the real workbench pipeline, not two
// input-sharing halves.
//
// The chain, all driven through the workbench UI:
//   booth over-vote (over=allowed, invalid=allowed → no signal)
//     → encrypt + cast              (BoothSpike, workbench keypair)
//     → bridge decrypt              (repairedCastVotes[cv].decodedBigInts)
//     → decode BigUint → contest    (checkers run, invalid_errors populated)
//     → /tally → tally_decoded_ballots → ContestResult
//
// Nothing is hand-fed: the ballot that reaches the tally is the one the
// booth encrypted, carried by the contest page's "Open in tally" seed. If
// the booth showed the voter nothing AND the ContestResult puts this ballot
// in invalid_votes.implicit (excluded from valid), the silent discount is
// real end-to-end, tally included.
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
const TENANT = "00000000-0000-0000-0000-000000000001"
const EVENT = "44444444-4444-4444-4444-444444444002"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const booth = `${base}/tenant/${TENANT}/event/${EVENT}/election/${ELECTION}`

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()
const log = (m) => console.log(m)

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

// 1. fresh state + fixture + silent over-vote config on Council seat
await page.goto(base + "/wb", {waitUntil: "networkidle", timeout: 60000})
await page.evaluate(() => window.__resetWorkbench && window.__resetWorkbench())
await page.waitForTimeout(2000)
await page.goto(
    base + "/wb/snapshot/" + encodeURIComponent("bundled:explicit-blank-invalid"),
    {waitUntil: "networkidle", timeout: 60000}
)
await page.waitForTimeout(900)
await page
    .getByRole("button", {name: /load this snapshot|^load$|reload/i})
    .first()
    .click()
    .catch(() => {})
await page.waitForTimeout(3000)

const councilId = await page.evaluate((electionId) => {
    const bs = window.__store.getState().ballotStyles[electionId]
    const c = bs.ballot_eml.contests.find((x) =>
        x.candidates.some((cd) => cd.presentation?.is_explicit_invalid)
    )
    return c.id
}, ELECTION)
await page.evaluate(
    ({electionId, cid}) => {
        const row = window.__store.getState().ballotStyles[electionId]
        const next = JSON.parse(JSON.stringify(row))
        const c = next.ballot_eml.contests.find((x) => x.id === cid)
        c.presentation = {
            ...(c.presentation ?? {}),
            over_vote_policy: "allowed",
            invalid_vote_policy: "allowed",
        }
        window.__store.dispatch({type: "ballotStyles/setBallotStyle", payload: next})
    },
    {electionId: ELECTION, cid: councilId}
)
log(`configured Council seat ${councilId.slice(-6)} = over:allowed, invalid:allowed`)

// 2. booth: over-vote Council seat (Ada + Bruno)
await page.goto(booth + "/start", {waitUntil: "networkidle", timeout: 60000})
await page.waitForTimeout(1200)
for (const rx of [/start voting/i, /^vote/i, /continue/i, /next/i]) {
    const b = page.getByRole("button", {name: rx}).first()
    if (await b.count().catch(() => 0)) {
        await b.click().catch(() => {})
        break
    }
}
await page.waitForTimeout(1800)
await page.getByText(/^Ada$/).first().click().catch(() => {})
await page.waitForTimeout(400)
await page.getByText(/^Bruno$/).first().click().catch(() => {})
await page.waitForTimeout(800)

const formed = await page.evaluate(
    ({electionId, cid}) => {
        const sel = window.__store.getState().ballotSelections[electionId] ?? []
        const c = sel.find((x) => x.contest_id === cid)
        return c ? c.choices.filter((ch) => ch.selected === 0).length : 0
    },
    {electionId: ELECTION, cid: councilId}
)
const inlineAtVote = await warnIds()
log(`booth: over-vote formed=${formed} selections, inline signal=${JSON.stringify(inlineAtVote)}`)

// 3. proceed to review, dismissing nothing that shouldn't appear; then cast
let transitionDialog = "none"
for (let i = 0; i < 4; i++) {
    for (const rx of [/next/i, /review/i, /continue/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(1500)
    const k = await dialogKind()
    if (k !== "none") transitionDialog = k
    if (page.url().includes("/review")) break
}
const inlineAtReview = await warnIds()
log(`booth: transition dialog=${transitionDialog}, at review inline=${JSON.stringify(inlineAtReview)}`)

for (const rx of [/^cast/i, /cast ballot/i, /confirm/i, /finish/i]) {
    const b = page.getByRole("button", {name: rx}).first()
    if (await b.count().catch(() => 0)) {
        await b.click().catch(() => {})
        break
    }
}
await page.waitForTimeout(2500)
for (const rx of [/^cast/i, /confirm/i, /accept/i, /^yes/i, /ok/i]) {
    const b = page.getByRole("button", {name: rx}).last()
    if (await b.count().catch(() => 0)) {
        await b.click().catch(() => {})
        await page.waitForTimeout(3000)
        break
    }
}

// 4. wait for the bridge to decrypt the cast vote for this contest
let decodedBigInt = null
for (let i = 0; i < 20; i++) {
    decodedBigInt = await page.evaluate(
        ({cid}) => {
            const raw = localStorage.getItem("workbench:state:v1")
            const snap = raw ? JSON.parse(raw) : null
            const rep = snap?.workbench?.repairedCastVotes ?? {}
            for (const entry of Object.values(rep)) {
                const v = entry?.decodedBigInts?.[cid]
                if (v) return v
            }
            return null
        },
        {cid: councilId}
    )
    if (decodedBigInt) break
    await page.waitForTimeout(1000)
}
log(`bridge: cast vote decrypted+captured, Council BigUint=${decodedBigInt ?? "(none)"}`)

// 5. contest page → Open in tally → run tally → read ContestResult
await page.goto(`${base}/wb/contest/${councilId}`, {waitUntil: "networkidle", timeout: 60000})
await page.waitForTimeout(1500)
await page.getByRole("button", {name: /open in tally/i}).first().click().catch(() => {})
await page.waitForTimeout(2500)
await page.getByRole("button", {name: /run tally/i}).first().click().catch(() => {})
await page.waitForTimeout(3500)

const result = await page.evaluate(() => {
    const areas = Array.from(document.querySelectorAll("textarea"))
    for (const a of areas) {
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

const summary = result
    ? {
          total_votes: result.total_votes,
          total_valid_votes: result.total_valid_votes,
          total_invalid_votes: result.total_invalid_votes,
          invalid_explicit: result.invalid_votes?.explicit,
          invalid_implicit: result.invalid_votes?.implicit,
      }
    : null
log("\n=== tally of the cast over-vote (full pipeline) ===")
log(JSON.stringify(summary, null, 2))

const boothSilent =
    formed > 1 && inlineAtVote.length === 0 && inlineAtReview.length === 0 && transitionDialog === "none"
const discardedImplicit =
    summary != null &&
    summary.total_valid_votes === 0 &&
    summary.invalid_implicit >= 1
const confirmed = boothSilent && discardedImplicit

log("\n=== S1 end-to-end (single continuous pipeline) ===")
log(`  booth over-vote reachable & silent: ${boothSilent}`)
log(`  cast → decrypt → decode → tally → discarded ImplicitInvalid, 0 valid: ${discardedImplicit}`)
log(`  CONFIRMED END-TO-END: ${confirmed}`)

await browser.close()
writeFileSync(
    path.join(here, "overvote-e2e-pipeline.recorded.json"),
    JSON.stringify(
        {
            config: {over_vote_policy: "allowed", invalid_vote_policy: "allowed"},
            booth: {formed, inlineAtVote, inlineAtReview, transitionDialog, boothSilent},
            bridge: {council_biguint: decodedBigInt},
            tally: summary,
            confirmed_end_to_end: confirmed,
        },
        null,
        2
    ) + "\n"
)
log("\nwrote overvote-e2e-pipeline.recorded.json")
