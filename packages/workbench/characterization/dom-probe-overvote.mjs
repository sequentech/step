// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Reload-free DOM probe — timing spike for the per-cell DOM-validation lane.
//
// The question (from the e2e-cost assessment): can browser observation of the
// two spec-modeled surfaces — inline visibility (`filterErrorList`) and the
// input constraint / reachability — be made fast enough to populate a per-cell
// DOM-✓ lane routinely, rather than only for a handful of headline cells?
//
// The existing *.browser.mjs runners cost ~15-20s/cell, dominated by THREE
// full `page.goto` document loads per cell. This probe removes them: it loads
// the snapshot ONCE, then drives every cell through CLIENT-SIDE navigation
// only (Shell's <Link> nav + inspector rail links + booth buttons), so the
// ephemeral policy overrides survive and no cell pays a reload. Fixed sleeps
// are replaced with `waitForSelector` on the actual target element.
//
// It does NOT cast or tally — the DOM lane only needs what the booth SHOWS for
// a (config x state): inline warnings (data-warn-id), the Next-dialog, whether
// the over-max control is disabled (constraint), and whether the state formed
// (reachability). It prints per-cell and total wall-clock.
//
// Over-vote grid x {at_max, over_max}, invalid=allowed. Requires :5173.

import {createRequire} from "node:module"
import {performance} from "node:perf_hooks"
import {fileURLToPath} from "node:url"
import path from "node:path"

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const base = "http://localhost:5173"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const SNAPSHOT = "bundled:explicit-blank-invalid"

const OVER_POLICIES = [
    "allowed",
    "allowed-with-msg",
    "allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-disable",
]
const VOTE_STATES = ["at_max", "over_max"] // 1 and 2 selections (max is 1)

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()

const warnIds = () =>
    page.evaluate(() =>
        Array.from(document.querySelectorAll("[data-warn-id]")).map((el) =>
            el.getAttribute("data-warn-id")
        )
    )
const dialogKind = () =>
    page.evaluate(() => {
        const d = document.querySelector('[role="dialog"]')
        if (!d) return "none"
        const btns = Array.from(d.querySelectorAll("button")).map((b) => b.innerText)
        return btns.some((b) => /continue/i.test(b)) ? "dismissible" : "blocking"
    })

// ---- one-time full load (the only page.goto in the whole run) --------------
await page.goto(base + "/wb", {waitUntil: "networkidle", timeout: 60000})
await page.evaluate(() => window.__resetWorkbench && window.__resetWorkbench())
await page.waitForTimeout(1500)
await page.goto(base + "/wb/snapshot/" + encodeURIComponent(SNAPSHOT), {
    waitUntil: "networkidle",
    timeout: 60000,
})
await page.getByRole("button", {name: /load this snapshot|^load$|reload/i}).first().click().catch(() => {})
await page.waitForTimeout(2500)

const {councilId, voterId} = await page.evaluate((electionId) => {
    const bs = window.__store.getState().ballotStyles[electionId]
    const c = bs.ballot_eml.contests.find((x) =>
        x.candidates.some((cd) => cd.presentation?.is_explicit_invalid)
    )
    const raw = localStorage.getItem("workbench:state:v1")
    const snap = raw ? JSON.parse(raw) : null
    return {councilId: c.id, voterId: snap?.workbench?.voters?.[0]?.id ?? null}
}, ELECTION)

// ---- reload-free per-cell probe --------------------------------------------
const clickLink = async (href) => {
    await page.locator(`a[href="${href}"]`).first().click()
}

async function setPanel(over) {
    await clickLink(`/wb/contest/${councilId}`)
    await page.waitForSelector('select[aria-label="Over-vote policy override"]', {timeout: 15000})
    await page.locator('select[aria-label="Over-vote policy override"]').selectOption(over)
    await page.locator('select[aria-label="Invalid-vote policy override"]').selectOption("allowed")
}

async function probeCell(over, state) {
    await setPanel(over)
    // enter booth (client-side) — wait for each screen's own landmark
    await clickLink(`/wb/voter/${voterId}`)
    const castBtn = page.getByRole("button", {name: /cast a ballot in|recast in/i}).first()
    await castBtn.waitFor({timeout: 15000})
    await castBtn.click()
    const startBtn = page.locator(".start-voting-button").first()
    await startBtn.waitFor({timeout: 15000})
    await startBtn.click()
    await page.getByText(/^Ada$/).first().waitFor({timeout: 15000})
    // clear any residual selection, then form the target state
    const clear = page.getByRole("button", {name: /clear/i}).first()
    if (await clear.count().catch(() => 0)) await clear.click().catch(() => {})
    await page.getByText(/^Ada$/).first().click()
    if (state === "over_max") await page.getByText(/^Bruno$/).first().click()

    const formed = await page.evaluate(
        ({electionId, cid}) => {
            const sel = window.__store.getState().ballotSelections[electionId] ?? []
            const c = sel.find((x) => x.contest_id === cid)
            return c ? c.choices.filter((ch) => ch.selected === 0).length : 0
        },
        {electionId: ELECTION, cid: councilId}
    )
    // `reachable` (did the state form?) is the robust constraint observable:
    // under the DISABLE over-vote policy the (max+1)th control is inert, so
    // over_max never forms (formed stays at max) — matching spec.inputConstraint.
    const inlineAtVote = await warnIds()

    // Next → dialog kind
    let dialog = "none"
    const next = page.getByRole("button", {name: /next|review/i}).first()
    if (await next.count().catch(() => 0)) {
        await next.click().catch(() => {})
        dialog = await dialogKind()
    }
    // dismiss any dialog (its MUI backdrop intercepts pointer events and would
    // block the back-navigation) without continuing to review
    if (dialog !== "none") {
        const dismiss = page
            .getByRole("button", {name: /cancel|back|review selection/i})
            .first()
        if (await dismiss.count().catch(() => 0)) await dismiss.click().catch(() => {})
        else await page.keyboard.press("Escape").catch(() => {})
        await page
            .waitForSelector('[role="dialog"]', {state: "detached", timeout: 5000})
            .catch(() => {})
    }

    // back to inspector (client-side) for the next cell
    await clickLink("/wb")
    await page.waitForSelector('a[href^="/wb/contest/"]', {timeout: 15000}).catch(() => {})
    return {over, state, formed, reachable: formed > (state === "over_max" ? 1 : 0), inlineAtVote, dialog}
}

const results = []
const t0 = performance.now()
for (const over of OVER_POLICIES) {
    for (const state of VOTE_STATES) {
        const cellStart = performance.now()
        const r = await probeCell(over, state)
        const ms = Math.round(performance.now() - cellStart)
        results.push({...r, ms})
        console.log(
            `${over} x ${state}: formed=${r.formed} reachable=${r.reachable} ` +
                `inline=${JSON.stringify(r.inlineAtVote)} dialog=${r.dialog} | ${ms}ms`
        )
    }
}
const totalMs = Math.round(performance.now() - t0)
const cells = results.length
const perCell = Math.round(totalMs / cells)
console.log(
    `\n${cells} cells in ${totalMs}ms (reload-free) → ${perCell}ms/cell. ` +
        `Extrapolated full 248-cell grid: ~${Math.round((perCell * 248) / 1000)}s serial ` +
        `(vs ~${Math.round((17000 * 248) / 1000)}s at the old ~17s/cell).`
)

await browser.close()
