// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// General DOM validator — the browser half of the two prediction-only lanes.
//
// The headless runners predict inline visibility (`spec.inlineVisible`) and
// the input constraint (`spec.inputConstraint`) but cannot observe them —
// `filterErrorList` and the input disable are TypeScript/React. This validates
// those predictions against the REAL DOM, reload-free (~675ms/cell), by:
//   - reading each rule's recorded JSON for the per-cell prediction
//     (`derived_inline_visible`; the gates → expected dialog), and
//   - driving the booth (panel config → cast state → observe) via
//     `browser-harness.mjs`, then comparing.
//
// It drives config through the PANEL (not dispatch) on purpose: a panel
// regression would make the config miss the booth and the DOM diverge from
// the prediction, so this doubles as the reviewer-path check REPRODUCE.md
// relies on — across every cell, not just the headline ones.
//
// Comparison is over UNIQUE message keys: `derived_inline_visible` may carry a
// message twice (the kept error + its alert copy), but the booth renders one
// WarnBox per message, so the set is what "does the voter see it?" turns on.
//
// Requires the dev server on :5173. Currently covers over-vote + min-vote
// (the finding families); adding a rule is a RULES entry.

import {createRequire} from "node:module"
import {readFileSync, writeFileSync} from "node:fs"
import {performance} from "node:perf_hooks"
import {fileURLToPath} from "node:url"
import path from "node:path"
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
import {inputConstraint} from "./spec.mjs"

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const here = path.dirname(fileURLToPath(import.meta.url))
const base = "http://localhost:5173"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const SNAPSHOT = "bundled:explicit-blank-invalid"

const rec = (f) => JSON.parse(readFileSync(path.join(here, f), "utf8")).rows
const uniq = (xs) => [...new Set(xs)].sort()
const clickText = (page, rx) => page.getByText(rx).first().click().catch(() => {})
const clickExact = (page, s) =>
    page.getByText(s, {exact: true}).first().click().catch(() => {})

// Predicted dialog from the recorded gates: hard → blocking, soft → dismissible.
const expectedDialog = (r) => (r.observed.hard ? "blocking" : r.observed.soft ? "dismissible" : "none")

const RULES = [
    {
        name: "over-vote",
        rows: rec("overvote-rule.recorded.json"),
        contestFlag: "is_explicit_invalid", // Council seat
        landmark: /^Ada$/,
        config: (r) => ({
            selects: {
                "Over-vote policy": r.over_vote_policy,
                "Invalid-vote policy": r.invalid_vote_policy,
            },
        }),
        select: async (page, r) => {
            if (r.state === "at_max") await clickText(page, /^Ada$/)
            else if (r.state === "over_max") {
                await clickText(page, /^Ada$/)
                await clickText(page, /^Bruno$/)
            }
        },
        want: (r) => r.state === "over_max" ? 2 : r.state === "at_max" ? 1 : 0,
        constraint: (r) =>
            inputConstraint({
                selections: r.state === "over_max" ? 2 : r.state === "at_max" ? 1 : 0,
                max: 1,
                policies: {over: r.over_vote_policy},
            }),
        label: (r) => `${r.over_vote_policy} × ${r.invalid_vote_policy} × ${r.state}`,
    },
    {
        name: "min-vote",
        rows: rec("minvote-rule.recorded.json"),
        contestFlag: "is_explicit_blank", // Referendum
        landmark: /^Yes$/,
        config: (r) => ({
            selects: {"Invalid-vote policy": r.invalid_vote_policy},
            bounds: {min_votes: r.min_votes},
        }),
        select: async (page, r) => {
            if (r.state === "one") await clickText(page, /^Yes$/)
            else if (r.state === "marker_only")
                await clickExact(page, "Blank vote (explicit blank)")
        },
        want: (r) => (r.state === "none" ? 0 : 1),
        constraint: () => null, // min-vote imposes no input constraint
        label: (r) => `min=${r.min_votes} × ${r.invalid_vote_policy} × ${r.state}`,
    },
]

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()
await loadSnapshot(page, base, SNAPSHOT)

const contestIdFor = (flag) =>
    page.evaluate(
        ({electionId, flag}) => {
            const bs = window.__store.getState().ballotStyles[electionId]
            const c = bs.ballot_eml.contests.find((x) =>
                x.candidates.some((cd) => cd.presentation?.[flag])
            )
            return c.id
        },
        {electionId: ELECTION, flag}
    )
const voterId = await page.evaluate(() => {
    const raw = localStorage.getItem("workbench:state:v1")
    return raw ? JSON.parse(raw)?.workbench?.voters?.[0]?.id ?? null : null
})

const results = []
const t0 = performance.now()
for (const rule of RULES) {
    const contestId = await contestIdFor(rule.contestFlag)
    for (const r of rule.rows) {
        await setPanelConfig(page, contestId, rule.config(r))
        await enterBooth(page, voterId)
        await page.getByText(rule.landmark).first().waitFor({timeout: 15000})
        await clearSelections(page)
        await rule.select(page, r)

        const formed = await selectionCount(page, ELECTION, contestId)
        const inlineDom = await warnIds(page)
        let dialog = "none"
        const next = page.getByRole("button", {name: /next|review/i}).first()
        if (await next.count().catch(() => 0)) {
            await next.click().catch(() => {})
            dialog = await dialogKind(page)
        }
        if (dialog !== "none") await dismissDialog(page)
        await backToInspector(page)

        // compare DOM vs the recorded predictions
        const constrained = rule.constraint(r) === "inputs_disabled"
        const domReachable = formed === rule.want(r)
        const reachableOk = domReachable === !constrained
        // A prevented (unreachable) state can't exist in the DOM, so it shows a
        // DIFFERENT reachable state's signals — the recorded inline/dialog
        // (predicted for the phantom state) don't apply. Reachability is the
        // whole validation for these prevention-guarded cells.
        const inlineOk =
            constrained ||
            JSON.stringify(uniq(inlineDom)) === JSON.stringify(uniq(r.derived_inline_visible))
        const dialogOk = constrained || dialog === expectedDialog(r)
        const ok = inlineOk && dialogOk && reachableOk
        results.push({rule: rule.name, cell: rule.label(r), inlineOk, dialogOk, reachableOk, ok})
        if (!ok) {
            console.log(
                `✗ ${rule.name} ${rule.label(r)}: ` +
                    `inline dom=${JSON.stringify(uniq(inlineDom))} pred=${JSON.stringify(uniq(r.derived_inline_visible))} ` +
                    `| dialog dom=${dialog} pred=${expectedDialog(r)} ` +
                    `| reachable dom=${domReachable} pred=${!constrained}`
            )
        }
    }
    const ruleCells = results.filter((x) => x.rule === rule.name)
    console.log(`${rule.name}: ${ruleCells.filter((x) => x.ok).length}/${ruleCells.length} cells DOM-✓`)
}

const totalMs = Math.round(performance.now() - t0)
const passed = results.filter((x) => x.ok).length
const allOk = passed === results.length
console.log(
    `\n${passed}/${results.length} cells validated against the real DOM in ${totalMs}ms ` +
        `(~${Math.round(totalMs / results.length)}ms/cell). all DOM-✓: ${allOk}`
)
await browser.close()
writeFileSync(
    path.join(here, "dom-validate.recorded.json"),
    JSON.stringify({cells: results, passed, total: results.length, all_ok: allOk}, null, 2) + "\n"
)
if (!allOk) process.exitCode = 1
