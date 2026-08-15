// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The no-silent-discount query — now OBSERVATION-based end to end (no model in
// the finding path; the criterion it yields is VALIDATION_LOGIC_DISTILLATION.md
// §4.5).
//
//   no-silent-discount :=
//     ¬∃ (config, vote_state) reachable through the booth such that
//         the voter is shown NO signal on any surface
//       ∧ the tally classifies the ballot ImplicitInvalid
//
// Two phases:
//   1. Pre-filter (headless): a candidate is any recorded cell with
//      `tally == ImplicitInvalid` ∧ no gate (`¬hard ∧ ¬soft`). Both are real
//      WASM observations (velvet tally; the checker/gate WASM), so this is a
//      SOUND superset — every real silent discount is a candidate; it only
//      over-includes cells that show something inline, or are unreachable.
//   2. Confirm (browser): drive each candidate through the real booth and
//      observe the two surfaces the headless side cannot — inline visibility
//      at the REVIEW screen (the decisive last surface before cast; the
//      untouched-clear does not apply there) and reachability (did the state
//      form?). A candidate is a CONFIRMED silent discount iff it is reachable
//      and shows nothing inline at review with no dialog.
//
// The tally half stays headless (velvet is a Node WASM call, not a booth
// observation); the signal half is the browser. No `derived_inline`
// (the model) is consulted — the model is the spec, validated separately by
// `dom-validate`, not the detector here.
//
// Blank / Declined / ExplicitInvalid are excluded by definition (valid,
// voter-intended, deliberate opt-in). Requires the dev server on :5173.

import {createRequire} from "node:module"
import {readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {loadSnapshot} from "./browser-harness.mjs"
import {RULE_SPECS, contestAndVoter, observeBooth, isReached} from "./rule-specs.mjs"

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const here = path.dirname(fileURLToPath(import.meta.url))
const base = "http://localhost:5173"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const SNAPSHOT = "bundled:explicit-blank-invalid"

const SOURCES = [
    "blank-rule",
    "overvote-rule",
    "undervote-rule",
    "minvote-rule",
    "duprank-rule",
    "prefgaps-rule",
    "invalid-rule",
]
const NON_CONFIG = new Set([
    "observed",
    "derived_inline",
    "predicted",
    "match",
    "state",
    "rule",
    "contest",
])
const configEntries = (cell) =>
    Object.entries(cell).filter(([k]) => !NON_CONFIG.has(k))

// ---- phase 1: headless pre-filter (observed tally ∧ observed gates) ---------
const candidates = []
let scanned = 0
for (const src of SOURCES) {
    const doc = JSON.parse(readFileSync(path.join(here, `${src}.recorded.json`), "utf8"))
    for (const cell of doc.rows) {
        scanned++
        const o = cell.observed
        if (o.tally === "ImplicitInvalid" && !o.hard && !o.soft) {
            candidates.push({rule: src, contest: doc.contest, ...cell})
        }
    }
}
console.log(
    `pre-filter: ${scanned} scanned → ${candidates.length} candidates ` +
        `(tally=ImplicitInvalid ∧ no gate)\n`
)

// ---- phase 2: browser-confirm each candidate at the review surface ----------
const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()
await loadSnapshot(page, base, SNAPSHOT)

const ctx = {} // per-rule {contestId, voterId}, resolved once
const confirmed = []
const rejected = []
for (const cand of candidates) {
    const spec = RULE_SPECS[cand.rule]
    if (!spec) {
        // no browser spec yet for this rule — record as unconfirmed
        rejected.push({...cand, reason: "no browser spec", confirmed: false})
        continue
    }
    ctx[cand.rule] ??= await contestAndVoter(page, ELECTION, {
        flag: spec.contestFlag,
        counting: spec.contestCounting,
    })
    const {contestId, voterId} = ctx[cand.rule]
    const obs = await observeBooth(page, {electionId: ELECTION, contestId, voterId, spec, cell: cand})

    const reachable = isReached(spec, obs, cand)
    const silentAtReview = (obs.inlineAtReview ?? []).length === 0 && obs.dialog === "none"
    const isSilent = reachable && silentAtReview
    const cfg = configEntries(cand)
        .map(([k, v]) => `${k.replace("_vote_policy", "")}=${v}`)
        .join(" ")
    const rec = {rule: cand.rule, cfg, state: cand.state, observed: cand.observed,
        formed: obs.formed, reachable, inlineAtReview: obs.inlineAtReview, dialog: obs.dialog, confirmed: isSilent}
    ;(isSilent ? confirmed : rejected).push(rec)
    console.log(
        `  ${isSilent ? "✓ SILENT" : "· rejected"} [${cfg} state=${cand.state}]: ` +
            `formed=${obs.formed} reachable=${reachable} ` +
            `inlineAtReview=${JSON.stringify(obs.inlineAtReview)} dialog=${obs.dialog}`
    )
}
await browser.close()
console.log(
    `\nconfirmed silent discounts: ${confirmed.length} / ${candidates.length} candidates ` +
        `(${rejected.length} rejected: shown inline, or unreachable)`
)

// ---- group confirmed by config (the admin-lint shape) + report --------------
const byConfig = {}
for (const c of confirmed) (byConfig[c.cfg] ??= []).push(c.state)

writeFileSync(
    path.join(here, "no-silent-discount.report.json"),
    JSON.stringify(
        {property: "no-silent-discount", method: "observation-based (headless pre-filter + browser confirm at review)",
            scanned, candidates: candidates.length, confirmed_count: confirmed.length,
            byConfig, confirmed, rejected},
        null,
        2
    ) + "\n"
)

const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# no-silent-discount — observation-based report",
    "",
    "Generated by `characterization/no-silent-discount.mjs`; do not edit by hand.",
    "",
    "**Property.** No reachable (config, vote-state) exists where the voter is",
    "shown no signal on any booth surface — no inline message, no dialog, no",
    "input constraint they'd notice — yet the tally classifies the ballot",
    "`ImplicitInvalid` and discards it. Blank / Declined / ExplicitInvalid are",
    "excluded by definition (valid, voter-intended, deliberate opt-in).",
    "",
    "**Method — observation-based, no model in the finding path.**",
    "*Phase 1 (headless):* a candidate is any recorded cell with `tally ==",
    "ImplicitInvalid` ∧ no gate (`¬hard ∧ ¬soft`) — both real WASM observations",
    "(velvet tally; the checker/gate WASM), so this is a **sound superset**",
    "(every real silent discount is a candidate; it only over-includes cells",
    "that show something inline, or are unreachable). *Phase 2 (browser):* each",
    "candidate is driven through the real booth; it is a **confirmed** silent",
    "discount iff it is reachable and shows nothing inline at the **review**",
    "screen (the decisive last surface before cast) with no dialog. The tally",
    "half is headless (velvet is a Node WASM call, not a booth observation);",
    "the signal half is the browser. `derived_inline` (the model) is",
    "not consulted.",
    "",
    `**Result: ${candidates.length} candidates → ${confirmed.length} confirmed** ` +
        `across ${scanned} scanned (sources: ${SOURCES.map((s) => `${s}.recorded.json`).join(", ")}). ` +
        `${rejected.length} candidate(s) rejected (shown inline, or unreachable).`,
    "",
    "**Status: SUSPECT — escalated for consultation** as S1/S2 in",
    "`../docs/UPSTREAM_FINDINGS.md`. Adjudication belongs to the parties with",
    "design authority, not to this report.",
    "",
    "**Reproduce.** Click-by-click workbench recipes are in",
    "[../docs/REPRODUCE.md](../docs/REPRODUCE.md).",
    "",
    "## Confirmed silently-discounting configurations",
    "",
    "Each row is a policy combination that *permits* a silently-discounted vote",
    "— the candidate content of an admin-portal config lint.",
    "",
    "| configuration | states |",
    "|---|---|",
    ...Object.entries(byConfig).map(([k, states]) => `| ${k} | ${[...new Set(states)].join(", ")} |`),
    "",
].join("\n")
writeFileSync(path.join(here, "no-silent-discount.md"), md)
console.log("\nwrote no-silent-discount.report.json and no-silent-discount.md")
if (confirmed.length === 0) process.exitCode = 1
