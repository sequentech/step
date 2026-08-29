// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Full-pipeline confirmation of the min-vote family, booth to tally
// through real crypto — the same continuous chain for every cell:
//   booth below-min ballot (invalid=allowed → no signal)
//     → encrypt + cast → bridge decrypt → decode → /tally → ContestResult
//
// Run over the Referendum contest (Yes / No / Blank-marker), overriding
// min_votes per cell. min_votes does not affect the encoding (plurality
// bases are per-candidate, independent of min), so the cast bigint is
// unchanged; the override only changes what the decode-time min-vote rule
// sees — exactly the point.
//
// The four cells (all invalid_vote_policy = allowed) carry PER-CELL
// expectations. The whole fix ledger is injected, so every cell now
// confirms fixed behaviour end-to-end:
//   min=1, none / min=2, none / min=2, one — below-min ballots. The S1
//      display fix (the booth filter renders every emitted error) makes
//      these INFORMED, UNINTERRUPTED discounts: selectedMin renders at
//      review (and on the voting screen once the contest is touched —
//      the two empty cells never touch it, so the untouched-clear keeps
//      their voting screen empty), no dialog interrupts, and the tally
//      still books ImplicitInvalid.
//   min=2, marker_only — the S2 cell. The S2/S3 fix ("explicit blank
//      votes are not subject to min_vote rules") is injected at decode,
//      so this chain confirms it end-to-end: no error, no signal at all
//      (correct — nothing is wrong), and the tally books it
//      blank_votes.explicit, not invalid.
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

const CELLS = [
    {min: 1, state: "none", labels: [], expect: "informed_discarded"},
    {min: 2, state: "none", labels: [], expect: "informed_discarded"},
    {min: 2, state: "one", labels: ["Yes"], expect: "informed_discarded"},
    {
        min: 2,
        state: "marker_only",
        labels: ["Blank vote (explicit blank)"],
        expect: "explicit_blank",
    },
]

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()

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

async function referendumId() {
    return page.evaluate((electionId) => {
        const bs = window.__store.getState().ballotStyles[electionId]
        const c = bs.ballot_eml.contests.find((x) =>
            x.candidates.some((cd) => cd.presentation?.is_explicit_blank)
        )
        return c.id
    }, ELECTION)
}

const results = []
for (const cell of CELLS) {
    // fresh state + fixture
    await page.goto(base + "/wb", {waitUntil: "networkidle", timeout: 60000})
    await page.evaluate(() => window.__resetWorkbench && window.__resetWorkbench())
    await page.waitForTimeout(1800)
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

    const refId = await referendumId()
    // override min_votes + invalid=allowed on Referendum (max_votes left as-is)
    await page.evaluate(
        ({electionId, cid, min}) => {
            const row = window.__store.getState().ballotStyles[electionId]
            const next = JSON.parse(JSON.stringify(row))
            const c = next.ballot_eml.contests.find((x) => x.id === cid)
            c.min_votes = min
            c.presentation = {...(c.presentation ?? {}), invalid_vote_policy: "allowed"}
            window.__store.dispatch({type: "ballotStyles/setBallotStyle", payload: next})
        },
        {electionId: ELECTION, cid: refId, min: cell.min}
    )

    // booth
    await page.goto(booth + "/start", {waitUntil: "networkidle", timeout: 60000})
    await page.waitForTimeout(1200)
    for (const rx of [/start voting/i, /^vote/i, /continue/i, /next/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(1600)
    for (const label of cell.labels) {
        await page.getByText(label, {exact: true}).first().click().catch(() => {})
        await page.waitForTimeout(500)
    }

    const inlineAtVote = await warnIds()

    // to review, dismissing nothing (nothing should appear); then cast
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

    // wait for the bridge to decrypt this contest's cast vote
    let decodedBigInt = null
    for (let i = 0; i < 20; i++) {
        decodedBigInt = await page.evaluate(
            ({cid}) => {
                const raw = localStorage.getItem("workbench:state:v1")
                const snap = raw ? JSON.parse(raw) : null
                const rep = snap?.workbench?.repairedCastVotes ?? {}
                for (const entry of Object.values(rep)) {
                    const v = entry?.decodedBigInts?.[cid]
                    if (v !== undefined && v !== null) return String(v)
                }
                return null
            },
            {cid: refId}
        )
        if (decodedBigInt !== null) break
        await page.waitForTimeout(1000)
    }

    // contest page → Open in tally → run → read ContestResult
    await page.goto(`${base}/wb/contest/${refId}`, {waitUntil: "networkidle", timeout: 60000})
    await page.waitForTimeout(1500)
    await page.getByRole("button", {name: /open in tally/i}).first().click().catch(() => {})
    await page.waitForTimeout(2200)
    await page.getByRole("button", {name: /run tally/i}).first().click().catch(() => {})
    await page.waitForTimeout(3000)
    const result = await page.evaluate(() => {
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

    const summary = result
        ? {
              total_votes: result.total_votes,
              total_valid_votes: result.total_valid_votes,
              invalid_implicit: result.invalid_votes?.implicit,
              blank_explicit: result.blank_votes?.explicit,
          }
        : null
    const boothSilent =
        inlineAtVote.length === 0 && inlineAtReview.length === 0 && transitionDialog === "none"
    // Per-cell expectation (see the header): the three below-min cells are
    // informed, uninterrupted discounts — selectedMin renders at review
    // (and at voting once touched; the empty cells stay untouched), no
    // dialog — while the S2 cell is counted as an explicit blank with
    // nothing to warn about.
    const touched = cell.labels.length > 0
    const informedUninterrupted =
        inlineAtReview.includes("errors.implicit.selectedMin") &&
        (!touched || inlineAtVote.includes("errors.implicit.selectedMin")) &&
        (touched || inlineAtVote.length === 0) &&
        transitionDialog === "none"
    const discarded =
        summary != null && summary.total_valid_votes === 0 && summary.invalid_implicit >= 1
    const countedExplicitBlank =
        summary != null && summary.invalid_implicit === 0 && summary.blank_explicit >= 1
    const confirmed =
        cell.expect === "informed_discarded"
            ? informedUninterrupted && discarded
            : boothSilent && countedExplicitBlank
    results.push({...cell, decodedBigInt, boothSilent, informedUninterrupted, summary, confirmed})
    console.log(
        `min=${cell.min} ${cell.state}: silent=${boothSilent} informed=${informedUninterrupted} ` +
            `bigint=${decodedBigInt} tally(valid=${summary?.total_valid_votes},impl=${summary?.invalid_implicit},blankExpl=${summary?.blank_explicit}) ` +
            `expect=${cell.expect} → CONFIRMED=${confirmed}`
    )
}

await browser.close()
const allConfirmed = results.every((r) => r.confirmed)
console.log(
    `\nall four min-vote cells confirmed end-to-end (three informed discounts + the S2 explicit blank): ${allConfirmed}`
)
writeFileSync(
    path.join(here, "minvote-e2e-pipeline.recorded.json"),
    JSON.stringify({invalid_vote_policy: "allowed", cells: results, all_confirmed: allConfirmed}, null, 2) + "\n"
)
console.log("wrote minvote-e2e-pipeline.recorded.json")
if (!allConfirmed) process.exitCode = 1
