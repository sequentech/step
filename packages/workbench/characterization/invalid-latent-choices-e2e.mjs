// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// End-to-end confirmation of S5 (UPSTREAM_FINDINGS.md): a protest
// (explicit-invalid) ballot preserves the voter's candidate selections all
// the way into the cast ciphertext, even though the tally ignores them.
//
// NOT a silent discount — the voter opts in deliberately. What this
// confirms is (1) REACHABILITY: the booth lets a voter select a regular
// candidate AND mark the ballot explicit-invalid (the invalid reducer does
// not clear choices; the UI does not disable candidates); and (2) the
// PRIVACY-ADJACENT consequence: those candidate selections are encrypted
// into the cast ballot and recovered at decrypt, so a null-voter's latent
// preference lives in the ciphertext for no functional reason (the tally
// classifies ExplicitInvalid and counts no candidate).
//
// Continuous pipeline, driven through the workbench UI:
//   booth: select Ada, then mark invalid (invalid=allowed → castable)
//     → encrypt + cast → bridge decrypt → decode → tally
//
// Predicted Council-seat bigint = 3: choices [invalid=1, Ada=1, Bruno=0]
// over bases [2,2,2] → 1·1 + 1·2 + 0·4 = 3. A recovered 3 proves BOTH the
// invalid flag (bit 0) and Ada's vote (bit 1) are in the decrypted plaintext.
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

const councilId = await page.evaluate((electionId) => {
    const bs = window.__store.getState().ballotStyles[electionId]
    const c = bs.ballot_eml.contests.find((x) =>
        x.candidates.some((cd) => cd.presentation?.is_explicit_invalid)
    )
    return c.id
}, ELECTION)
const invalidMarkerName = await page.evaluate(
    ({electionId, cid}) => {
        const bs = window.__store.getState().ballotStyles[electionId]
        const c = bs.ballot_eml.contests.find((x) => x.id === cid)
        return c.candidates.find((cd) => cd.presentation?.is_explicit_invalid)?.name
    },
    {electionId: ELECTION, cid: councilId}
)
// invalid=allowed so the explicit-invalid ballot is castable without a hard block
await page.evaluate(
    ({electionId, cid}) => {
        const row = window.__store.getState().ballotStyles[electionId]
        const next = JSON.parse(JSON.stringify(row))
        const c = next.ballot_eml.contests.find((x) => x.id === cid)
        c.presentation = {...(c.presentation ?? {}), invalid_vote_policy: "allowed"}
        window.__store.dispatch({type: "ballotStyles/setBallotStyle", payload: next})
    },
    {electionId: ELECTION, cid: councilId}
)

// booth: select Ada, THEN mark invalid
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
await page.getByText(/^Ada$/).first().click().catch(() => {})
await page.waitForTimeout(500)
await page.getByText(invalidMarkerName, {exact: true}).first().click().catch(() => {})
await page.waitForTimeout(800)

// REACHABILITY: does the store hold both Ada selected AND the invalid flag?
const selState = await page.evaluate(
    ({electionId, cid}) => {
        const sel = window.__store.getState().ballotSelections[electionId] ?? []
        const c = sel.find((x) => x.contest_id === cid)
        if (!c) return null
        return {
            is_explicit_invalid: c.is_explicit_invalid,
            selectedRegularCount: c.choices.filter((ch) => ch.selected === 0).length,
        }
    },
    {electionId: ELECTION, cid: councilId}
)
const reachable = !!selState && selState.is_explicit_invalid && selState.selectedRegularCount >= 1
console.log(
    `booth reachability: is_explicit_invalid=${selState?.is_explicit_invalid}, ` +
        `regular selections=${selState?.selectedRegularCount} → mixed state reachable=${reachable}`
)

// to review + cast
for (let i = 0; i < 4; i++) {
    for (const rx of [/next/i, /review/i, /continue/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(1400)
    if (page.url().includes("/review")) break
}
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

// bridge decrypt → the recovered plaintext bigint for Council
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
        {cid: councilId}
    )
    if (decodedBigInt !== null) break
    await page.waitForTimeout(1000)
}

// contest page → Open in tally → read the DECODED ballot + the ContestResult
await page.goto(`${base}/wb/contest/${councilId}`, {waitUntil: "networkidle", timeout: 60000})
await page.waitForTimeout(1500)
await page.getByRole("button", {name: /open in tally/i}).first().click().catch(() => {})
await page.waitForTimeout(2200)
// The seeded decoded ballot appears in an input textarea; capture it before tallying.
const decodedBallot = await page.evaluate(() => {
    for (const a of Array.from(document.querySelectorAll("textarea"))) {
        const v = a.value || ""
        if (v.includes("is_explicit_invalid") && v.includes("choices")) {
            try {
                const parsed = JSON.parse(v)
                return Array.isArray(parsed) ? parsed[0] : parsed
            } catch {
                /* keep looking */
            }
        }
    }
    return null
})
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

const decodedFlag = decodedBallot?.is_explicit_invalid
const decodedSelected = (decodedBallot?.choices ?? []).filter((c) => c.selected === 0).length
const latentPreferencePreserved = decodedFlag === true && decodedSelected >= 1
const summary = result
    ? {
          total_valid_votes: result.total_valid_votes,
          invalid_explicit: result.invalid_votes?.explicit,
      }
    : null

console.log("\n=== decrypted plaintext ===")
console.log(`  Council bigint = ${decodedBigInt}  (expected 3 = invalid bit + Ada bit)`)
console.log(`  decoded is_explicit_invalid = ${decodedFlag}`)
console.log(`  decoded regular selections = ${decodedSelected}`)
console.log("\n=== tally ===")
console.log(`  ${JSON.stringify(summary)}`)
console.log("\n=== S5 end-to-end ===")
console.log(`  mixed state reachable in booth: ${reachable}`)
console.log(`  latent candidate preference preserved through crypto: ${latentPreferencePreserved}`)
console.log(`  tally ignores it (ExplicitInvalid, no candidate counted): ${summary?.invalid_explicit >= 1 && summary?.total_valid_votes === 0}`)
const confirmed =
    reachable && latentPreferencePreserved && summary?.invalid_explicit >= 1 && summary?.total_valid_votes === 0
console.log(`  S5 CONFIRMED END-TO-END: ${confirmed}`)

await browser.close()
writeFileSync(
    path.join(here, "invalid-latent-choices-e2e.recorded.json"),
    JSON.stringify(
        {
            reachable,
            decoded: {bigint: decodedBigInt, is_explicit_invalid: decodedFlag, regular_selections: decodedSelected},
            tally: summary,
            latent_preference_preserved: latentPreferencePreserved,
            confirmed_end_to_end: confirmed,
        },
        null,
        2
    ) + "\n"
)
console.log("\nwrote invalid-latent-choices-e2e.recorded.json")
