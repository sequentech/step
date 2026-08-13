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
import {loadSnapshot} from "./browser-harness.mjs"
import {inputConstraint} from "./spec.mjs"
import {RULE_SPECS, contestAndVoter, observeBooth} from "./rule-specs.mjs"

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const here = path.dirname(fileURLToPath(import.meta.url))
const base = "http://localhost:5173"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const SNAPSHOT = "bundled:explicit-blank-invalid"

const rec = (f) => JSON.parse(readFileSync(path.join(here, f), "utf8")).rows
const uniq = (xs) => [...new Set(xs)].sort()

// Predicted dialog from the recorded gates: hard → blocking, soft → dismissible.
const expectedDialog = (r) => (r.observed.hard ? "blocking" : r.observed.soft ? "dismissible" : "none")

// The browser-driving specs (contest, config, selection, landmark) come from
// rule-specs.mjs — the single source shared with no-silent-discount. Here we
// add only the validation extras: the recorded rows to validate against, the
// predicted input-constraint, and a display label.
const RULES = [
    {
        name: "over-vote",
        ...RULE_SPECS["overvote-rule"],
        rows: rec("overvote-rule.recorded.json"),
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
        ...RULE_SPECS["minvote-rule"],
        rows: rec("minvote-rule.recorded.json"),
        constraint: () => null, // min-vote imposes no input constraint
        label: (r) => `min=${r.min_votes} × ${r.invalid_vote_policy} × ${r.state}`,
    },
]

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()
await loadSnapshot(page, base, SNAPSHOT)

const short = (xs) =>
    !xs || xs.length === 0
        ? "—"
        : uniq(xs).map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")

const results = []
const t0 = performance.now()
for (const rule of RULES) {
    const {contestId, voterId} = await contestAndVoter(page, ELECTION, rule.contestFlag)
    for (const r of rule.rows) {
        const obs = await observeBooth(page, {electionId: ELECTION, contestId, voterId, spec: rule, cell: r})

        const constrained = rule.constraint(r) === "inputs_disabled"
        const domReachable = obs.formed === rule.want(r)
        const reachableOk = domReachable === !constrained

        // Inline is validated at the REVIEW surface (the model's surface; the
        // untouched-clear does not apply there). Not comparable when the state
        // is unreachable (a phantom state) or a blocking gate preempts review —
        // there the constraint / the blocking dialog is the signal, validated
        // by reachableOk / dialogOk.
        const inlineComparable = !constrained && obs.dialog !== "blocking"
        const inlineOk =
            !inlineComparable ||
            JSON.stringify(uniq(obs.inlineAtReview ?? [])) ===
                JSON.stringify(uniq(r.derived_inline_visible))
        const dialogOk = constrained || obs.dialog === expectedDialog(r)
        const ok = inlineOk && dialogOk && reachableOk

        // Observation-derived silent-discount marker: discarded, reachable, and
        // no signal on any surface (no dialog, nothing inline at review).
        const silent =
            r.observed.tally === "ImplicitInvalid" &&
            obs.dialog === "none" &&
            (obs.inlineAtReview ?? []).length === 0 &&
            domReachable

        results.push({
            rule: rule.name,
            config: rule.label(r).replace(` × ${r.state}`, ""),
            state: r.state,
            inlineReview: obs.dialog === "blocking" ? "(blocked)" : short(obs.inlineAtReview),
            reachable: domReachable,
            dialog: obs.dialog,
            tally: r.observed.tally,
            silent,
            ok,
        })
        if (!ok) {
            console.log(
                `✗ ${rule.name} ${rule.label(r)}: ` +
                    `inline@review=${JSON.stringify(uniq(obs.inlineAtReview ?? []))} ` +
                    `pred=${JSON.stringify(uniq(r.derived_inline_visible))} ` +
                    `dialog=${obs.dialog}/${expectedDialog(r)} reachable=${domReachable}/${!constrained}`
            )
        }
    }
    const rc = results.filter((x) => x.rule === rule.name)
    console.log(
        `${rule.name}: ${rc.filter((x) => x.ok).length}/${rc.length} DOM-✓, ` +
            `${rc.filter((x) => x.silent).length} silent`
    )
}
await browser.close()

const totalMs = Math.round(performance.now() - t0)
const passed = results.filter((x) => x.ok).length
const allOk = passed === results.length
console.log(
    `\n${passed}/${results.length} cells validated against the real DOM in ${totalMs}ms ` +
        `(~${Math.round(totalMs / results.length)}ms/cell). all DOM-✓: ${allOk}`
)

// --- complete tables (one per rule) -----------------------------------------
const fmtRow = (x) =>
    `| ${x.silent ? "**⚠** " : ""}${x.config} | ${x.state} | ${x.inlineReview} | ` +
    `${x.reachable ? "yes" : "**no**"} | ${x.dialog} | ${x.tally} | ${x.ok ? "✓" : "**✗**"} |`
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# DOM-validated complete tables",
    "",
    "Generated by `characterization/dom-validate.mjs`; do not edit by hand.",
    "",
    "The **complete** view — every value is an OBSERVATION. The browser-only",
    "surfaces the partial rule tables cannot show are observed live in the real",
    "booth: *inline (review)* is inline visibility at the decisive review screen",
    "(the untouched-clear does not apply there); *reachable* is the input",
    "constraint (`no` = the state cannot be formed). *tally* is the recorded",
    "velvet class. **⚠** marks an observation-derived silent discount —",
    "discarded, reachable, and no signal on any surface. The single",
    "*matches spec?* column asks whether the spec (`spec.mjs`) agrees with every",
    "observation in the row; ✗ = spec and DOM disagree. `(blocked)` inline means",
    "a blocking dialog preempts review — the dialog is the signal there.",
    "",
]
for (const rule of RULES) {
    const rc = results.filter((x) => x.rule === rule.name)
    md.push(
        `## ${rule.name}`,
        "",
        "| config | state | inline (review) | reachable | dialog | tally | matches spec? |",
        "|---|---|---|---|---|---|---|",
        ...rc.map(fmtRow),
        ""
    )
}
writeFileSync(path.join(here, "dom-validate.md"), md.join("\n") + "\n")
writeFileSync(
    path.join(here, "dom-validate.recorded.json"),
    JSON.stringify({cells: results, passed, total: results.length, all_ok: allOk}, null, 2) + "\n"
)
console.log("wrote dom-validate.md and dom-validate.recorded.json")
if (!allOk) process.exitCode = 1
