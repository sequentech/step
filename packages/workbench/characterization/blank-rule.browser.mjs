// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the BLANK-VOTE rule, layer 3 (the booth UI: the
// filterErrorList visibility layer plus the real dialog wiring).
//
// Layers 1+2 run headlessly (blank-rule.mjs); this layer cannot —
// `filterErrorList` is component-internal TypeScript — so we drive the
// real booth in a browser against the workbench dev server, using the
// `explicit-blank-invalid` fixture with the blank policy swapped per cell
// by dispatching a modified ballot style into the portal's Redux store.
//
// Observation points per policy: untouched (during voting), touched
// (during voting), the transition dialog, and review. Observables: the
// inline alerts the voter actually sees (role="alert") and the dialog
// class (none / dismissible / blocking).
//
// Requires the workbench dev server on :5173.
// Run:  node characterization/blank-rule.browser.mjs

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

const BLANK_POLICIES = ["allowed", "warn", "warn-only-in-review", "not-allowed"]

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()

// WarnBox renders no ARIA role, but #2832 gave every box a
// data-warn-id/data-warn-type attribute carrying the raw message key —
// which is exactly the observable we want (no i18n ambiguity).
async function alerts() {
    return page
        .evaluate(() =>
            Array.from(document.querySelectorAll("[data-warn-id]")).map(
                (el) =>
                    `${el.getAttribute("data-warn-id")}` +
                    `(${el.getAttribute("data-warn-type") ?? "?"})`
            )
        )
        .catch(() => [])
}

async function dialogState() {
    const dialog = page.getByRole("dialog").first()
    if (!(await dialog.count().catch(() => 0))) return {kind: "none"}
    const text = (await dialog.innerText().catch(() => "")).replace(/\s+/g, " ")
    const buttons = await dialog
        .getByRole("button")
        .allInnerTexts()
        .then((xs) => xs.map((s) => s.trim()))
        .catch(() => [])
    const dismissible = buttons.some((b) => /continue/i.test(b))
    return {kind: dismissible ? "dismissible" : "blocking", buttons, text: text.slice(0, 160)}
}

const rows = []
for (const blank of BLANK_POLICIES) {
    // fresh state per cell
    await page.goto(base + "/wb", {waitUntil: "networkidle", timeout: 60000})
    await page.evaluate(() => window.__resetWorkbench && window.__resetWorkbench())
    await page.waitForTimeout(2000)
    await page.goto(
        base + "/wb/snapshot/" + encodeURIComponent("bundled:explicit-blank-invalid"),
        {waitUntil: "networkidle", timeout: 60000}
    )
    await page.waitForTimeout(1000)
    await page
        .getByRole("button", {name: /load this snapshot|^load$|reload/i})
        .first()
        .click()
        .catch(() => {})
    await page.waitForTimeout(3500)

    // swap the blank policy on the Referendum contest via the portal store
    const applied = await page.evaluate(
        ({electionId, policy}) => {
            const st = window.__store.getState()
            const row = st.ballotStyles[electionId]
            if (!row) return {ok: false, why: "no ballot style row"}
            const next = JSON.parse(JSON.stringify(row))
            const c = next.ballot_eml.contests.find((x) =>
                x.candidates.some((cd) => cd.presentation?.is_explicit_blank)
            )
            c.presentation = {...(c.presentation ?? {}), blank_vote_policy: policy}
            window.__store.dispatch({type: "ballotStyles/setBallotStyle", payload: next})
            return {ok: true, contest: c.name}
        },
        {electionId: ELECTION, policy: blank}
    )
    if (!applied.ok) {
        console.log(`SKIP ${blank}: ${applied.why}`)
        continue
    }

    // into the booth
    await page.goto(booth + "/start", {waitUntil: "networkidle", timeout: 60000})
    await page.waitForTimeout(1200)
    for (const rx of [/start voting/i, /vote/i, /continue/i, /next/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(2000)

    const untouched = await alerts()

    // touch the Referendum contest: select Yes, then deselect -> touched + empty
    const yes = page.getByText(/^Yes$/).first()
    await yes.click().catch(() => {})
    await page.waitForTimeout(700)
    await yes.click().catch(() => {})
    await page.waitForTimeout(900)
    const empty = await page.evaluate((electionId) => {
        const sel = window.__store.getState().ballotSelections[electionId] ?? []
        return sel.every((c) => c.choices.every((ch) => ch.selected < 0))
    }, ELECTION)

    const votingTouched = await alerts()

    // attempt the transition
    for (const rx of [/next/i, /review/i, /continue/i]) {
        const b = page.getByRole("button", {name: rx}).first()
        if (await b.count().catch(() => 0)) {
            await b.click().catch(() => {})
            break
        }
    }
    await page.waitForTimeout(2500)
    const dialog = await dialogState()

    // if dismissible, continue through to review; if none, we may be there already
    let review = null
    if (dialog.kind === "dismissible") {
        await page.getByRole("button", {name: /continue/i}).first().click().catch(() => {})
        await page.waitForTimeout(2500)
    }
    if (dialog.kind !== "blocking") {
        if (page.url().includes("/review")) review = await alerts()
    }

    rows.push({
        blank_vote_policy: blank,
        selection_empty: empty,
        untouched,
        voting_touched: votingTouched,
        dialog,
        review,
        review_reached: review !== null,
    })
    console.log(
        `${blank}: untouched=${untouched.length} touched=${votingTouched.length} ` +
            `dialog=${dialog.kind} review=${review === null ? "unreached" : review.length}`
    )
}

await browser.close()
writeFileSync(
    path.join(here, "blank-rule.filter.recorded.json"),
    JSON.stringify({invalid_vote_policy: "allowed (default)", rows}, null, 2) + "\n"
)
console.log("wrote blank-rule.filter.recorded.json")
