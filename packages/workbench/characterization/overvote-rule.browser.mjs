// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Over-vote rule, layer 3 (booth): the two things the headless runner
// cannot see — the INPUT CONSTRAINT (does the UI disable the
// second checkbox at max?) and REACHABILITY (can the over-vote state even
// be formed through the UI?) — plus inline visibility and the dialog.
//
// It also reproduces the no-silent-discount violation against the real
// components, in two halves that share the input rather than one ballot
// flowing through the crypto pipeline: (a) the over-vote is formed through
// the REAL BOOTH UI and the absence of any signal is observed there; (b)
// the same selection is decoded (sequent-core) and tallied (velvet-wasm)
// through the same real wasm the workbench tally sandbox uses, confirming
// ImplicitInvalid. Chaining the two halves through the booth's actual
// encrypt→cast→decrypt pipeline is done separately, as one continuous
// run, in `overvote-e2e-pipeline.mjs` (this runner stays the cheaper
// input-shared check of the whole policy grid).
//
// Requires the dev server on :5173.

import {createRequire} from "node:module"
import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {loadWasm, loadVelvetWasm, runChecker, tallyClass} from "./harness.mjs"

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const here = path.dirname(fileURLToPath(import.meta.url))
const base = "http://localhost:5173"
const TENANT = "00000000-0000-0000-0000-000000000001"
const EVENT = "44444444-4444-4444-4444-444444444002"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const booth = `${base}/tenant/${TENANT}/event/${EVENT}/election/${ELECTION}`

const OVER_POLICIES = [
    "allowed",
    "allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-disable",
]

await loadWasm()
await loadVelvetWasm()

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

async function councilContestId() {
    return page.evaluate((electionId) => {
        const bs = window.__store.getState().ballotStyles[electionId]
        const c = bs.ballot_eml.contests.find((x) =>
            x.candidates.some((cd) => cd.presentation?.is_explicit_invalid)
        )
        return c.id
    }, ELECTION)
}

const rows = []
for (const over of OVER_POLICIES) {
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

    await page.evaluate(
        ({electionId, policy}) => {
            const row = window.__store.getState().ballotStyles[electionId]
            const next = JSON.parse(JSON.stringify(row))
            const c = next.ballot_eml.contests.find((x) =>
                x.candidates.some((cd) => cd.presentation?.is_explicit_invalid)
            )
            c.presentation = {
                ...(c.presentation ?? {}),
                over_vote_policy: policy,
                invalid_vote_policy: "allowed",
            }
            window.__store.dispatch({type: "ballotStyles/setBallotStyle", payload: next})
        },
        {electionId: ELECTION, policy: over}
    )

    await page.goto(booth + "/start", {waitUntil: "networkidle", timeout: 60000})
    await page.waitForTimeout(1000)
    for (const rx of [/start voting/i, /vote/i, /continue/i, /next/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(1800)

    // Council seat: select Ada (at max), then try Bruno (would be over).
    const ada = page.getByText(/^Ada$/).first()
    const bruno = page.getByText(/^Bruno$/).first()
    await ada.click().catch(() => {})
    await page.waitForTimeout(500)

    // Is the Bruno control disabled after reaching max? (the constraint probe)
    const brunoDisabled = await page
        .evaluate(() => {
            const label = Array.from(document.querySelectorAll("label")).find((l) =>
                /^\s*Bruno/.test(l.innerText)
            )
            const input =
                label?.querySelector('input') ||
                label?.closest("*")?.querySelector('input')
            return input ? input.disabled : null
        })
        .catch(() => null)

    await bruno.click().catch(() => {})
    await page.waitForTimeout(700)

    // Did the over-vote state actually form?
    const formed = await page.evaluate(
        ({electionId, contestFinder}) => {
            const sel = window.__store.getState().ballotSelections[electionId] ?? []
            const council = sel.find((c) => contestFinder.includes(c.contest_id))
            const n = council ? council.choices.filter((ch) => ch.selected === 0).length : 0
            return n
        },
        {electionId: ELECTION, contestFinder: [await councilContestId()]}
    )

    const inlineAfter = await warnIds()

    // transition
    for (const rx of [/next/i, /review/i, /continue/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(2000)
    const dialog = await page.evaluate(() => {
        const d = document.querySelector('[role="dialog"]')
        if (!d) return {kind: "none"}
        const btns = Array.from(d.querySelectorAll("button")).map((b) => b.innerText.trim())
        return {kind: btns.some((b) => /continue/i.test(b)) ? "dismissible" : "blocking", btns}
    })

    rows.push({
        over_vote_policy: over,
        invalid_vote_policy: "allowed",
        state: "over_max",
        bruno_disabled_at_max: brunoDisabled,
        over_state_formed_selections: formed,
        reachable: formed > 1,
        inline_visible: inlineAfter,
        dialog,
    })
    console.log(
        `${over}: bruno_disabled=${brunoDisabled} formed=${formed} ` +
            `inline=${JSON.stringify(inlineAfter)} dialog=${dialog.kind}`
    )
}

// --- end-to-end verification of the sole no-silent-discount violation ---
// over=allowed / invalid=allowed / over_max: build that exact selection and
// tally it in-workbench; confirm the booth was silent AND the tally discards.
const snap = JSON.parse(
    (await import("node:fs")).readFileSync(
        path.resolve(here, "../app/src/fixtures/snapshots/explicit-blank-invalid.json"),
        "utf8"
    )
)
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const council = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_invalid)
)
const cfg = structuredClone(council)
cfg.presentation = {
    ...(cfg.presentation ?? {}),
    over_vote_policy: "allowed",
    invalid_vote_policy: "allowed",
}
const regulars = council.candidates
    .filter((x) => !x.presentation?.is_explicit_invalid)
    .map((x) => x.id)
const overBallot = {
    contest_id: council.id,
    is_explicit_invalid: false,
    is_decline_to_vote: false,
    invalid_errors: [],
    invalid_alerts: [],
    choices: council.candidates.map((c) => ({
        id: c.id,
        selected: regulars.includes(c.id) ? 0 : -1,
        write_in_text: null,
    })),
}
// Faithfulness matters here: the real tally DECODES ballots before
// classifying, and decode is what populates invalid_errors — feeding the
// raw selection straight into the tally would classify a checker-clean
// ballot (an earlier revision did exactly that and wrongly reported
// "Valid"). So run the same encode→decode round trip the pipeline runs.
const cellEml = structuredClone(eml)
cellEml.contests[cellEml.contests.findIndex((c) => c.id === council.id)] = cfg
const decodedOverBallot = runChecker(overBallot, cellEml)
const cls = tallyClass(cfg, decodedOverBallot)
const boothRow = rows.find((r) => r.over_vote_policy === "allowed")
const violationReproduced =
    boothRow &&
    boothRow.reachable &&
    boothRow.inline_visible.length === 0 &&
    boothRow.dialog.kind === "none" &&
    cls === "ImplicitInvalid"

console.log("\n=== violation check: real booth UI + real decode/tally (over=allowed, invalid=allowed) ===")
console.log(`  booth reachable over-vote: ${boothRow?.reachable}`)
console.log(`  booth signal: inline=${JSON.stringify(boothRow?.inline_visible)} dialog=${boothRow?.dialog.kind}`)
console.log(`  tally class of the same selection (decoded → velvet-wasm): ${cls}`)
console.log(`  VIOLATION REPRODUCED (booth UI + wasm decode/tally, input-shared not crypto-chained): ${violationReproduced}`)

await browser.close()
writeFileSync(
    path.join(here, "overvote-rule.filter.recorded.json"),
    JSON.stringify({invalid_vote_policy: "allowed (default)", rows, violation_reproduced_booth_plus_wasm_tally: violationReproduced, reproduced_tally_class: cls}, null, 2) + "\n"
)
console.log("\nwrote overvote-rule.filter.recorded.json")
