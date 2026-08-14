// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Reload-free DOM probe — timing spike + first DOM validator for the two
// spec-modeled surfaces (inline visibility via `filterErrorList`; the input
// constraint / reachability). Built on the shared `browser-harness.mjs`.
//
// The point (from the e2e-cost assessment): the older `*.browser.mjs` runners
// cost ~15-20s/cell, dominated by THREE full `page.goto` document loads per
// cell. This loads the snapshot ONCE and drives every cell through CLIENT-SIDE
// navigation, so the ephemeral panel overrides survive and no cell pays a
// reload; fixed sleeps become `waitForSelector` on each screen's landmark. It
// does NOT cast or tally — the DOM lane only needs what the booth SHOWS.
//
// Over-vote grid x {at_max, over_max}, invalid=allowed, on the Council seat.
// Requires the dev server on :5173.

import {createRequire} from "node:module"
import {performance} from "node:perf_hooks"
import {
    warnIds,
    dialogKind,
    loadSnapshot,
    setPanelConfig,
    enterBooth,
    clearSelections,
    dismissDialog,
    backToInspector,
    selectionCount,
} from "./browser-harness.mjs"

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

await loadSnapshot(page, base, SNAPSHOT) // the only full load in the run
const {councilId, voterId} = await page.evaluate((electionId) => {
    const bs = window.__store.getState().ballotStyles[electionId]
    const c = bs.ballot_eml.contests.find((x) =>
        x.candidates.some((cd) => cd.presentation?.is_explicit_invalid)
    )
    const raw = localStorage.getItem("workbench:state:v1")
    const snap = raw ? JSON.parse(raw) : null
    return {councilId: c.id, voterId: snap?.workbench?.voters?.[0]?.id ?? null}
}, ELECTION)

async function probeCell(over, state) {
    await setPanelConfig(page, councilId, {
        selects: {"Over-vote policy": over, "Invalid-vote policy": "allowed"},
    })
    await enterBooth(page, voterId)
    await page.getByText(/^Ada$/).first().waitFor({timeout: 15000})
    await clearSelections(page)
    await page.getByText(/^Ada$/).first().click()
    if (state === "over_max") await page.getByText(/^Bruno$/).first().click()

    // `reachable` (did the state form?) is the robust constraint observable:
    // under the DISABLE over-vote policy the (max+1)th control is inert, so
    // over_max never forms (formed stays at max) — matching spec.reachability.
    const formed = await selectionCount(page, ELECTION, councilId)
    const inlineAtVote = await warnIds(page)

    let dialog = "none"
    const next = page.getByRole("button", {name: /next|review/i}).first()
    if (await next.count().catch(() => 0)) {
        await next.click().catch(() => {})
        dialog = await dialogKind(page)
    }
    if (dialog !== "none") await dismissDialog(page)

    await backToInspector(page)
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
