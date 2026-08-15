// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the UNDER-VOTE rule, layers 1+2 + recorded tally class.
//
// Contest: Referendum (Yes / No / explicit-blank marker) with the marker
// ignored here — only the two regular candidates are used, and min_votes is
// forced to 0, max_votes to 2, so the under-vote zone is exactly n = 1.
// under_vote_policy × invalid_vote_policy × {empty(0), under(1), full(2)}.
// Blank policy stays at default.
//
// Expectation worth recording (VALIDATION_LOGIC_DISTILLATION.md §4.4): the
// under-vote checker only ever pushes ALERTS, never errors, so an
// under-voted ballot is structurally valid — it can never be a
// silent-discount cell. This runner exists partly to confirm that boundary.
//
// Run:  node characterization/undervote-rule.mjs   (from packages/workbench)

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {
    loadWasm,
    loadVelvetWasm,
    runChecker,
    runGates,
    tallyClass,
    isSilentDiscount,
    loadMarkerFixture,
    extractErrors,
} from "./harness.mjs"
import {f, inlineViews} from "./spec.mjs"
import {RULE_SPECS} from "./rule-specs.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const UNDER_POLICIES = ["allowed", "warn", "warn-only-in-review", "warn-and-alert"]
const INVALID_POLICIES = [
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
]
// max_votes forced to 2, min_votes to 0:
//   empty — 0 selections (blank; default blank policy)
//   under — 1 selection (the under-vote zone, min ≤ n < max)
//   full  — 2 selections (exactly max)
const STATES = ["empty", "under", "full"]

await loadWasm()
await loadVelvetWasm()
const snap = loadMarkerFixture()
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_blank)
)
const regulars = contest.candidates
    .filter((x) => !x.presentation?.is_explicit_blank)
    .map((x) => x.id)

function makeSelection(state) {
    const picked = state === "under" ? [regulars[0]] : state === "full" ? regulars : []
    return {
        contest_id: contest.id,
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: contest.candidates.map((c) => ({
            id: c.id,
            selected: picked.includes(c.id) ? 0 : -1,
            write_in_text: null,
        })),
    }
}

function makeEml(underPolicy, invalidPolicy) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.min_votes = 0
    c.max_votes = 2
    c.presentation = {
        ...(c.presentation ?? {}),
        under_vote_policy: underPolicy,
        invalid_vote_policy: invalidPolicy,
    }
    return clone
}

// PREDICTION — spec.mjs's complete mapping `f`, fed from this rule's cell
// definitions (rule-specs.mjs: specConfig/voteState). Two facts this rule's
// recording pinned live in the spec now: the checker's under-vote zone is
// `min ≤ n < max`, which with min_votes = 0 INCLUDES n = 0 (the empty ballot
// alerts too, overlapping blank — spec.emissions), and the soft gate's under
// clause requires n > 0, so it skips that same empty ballot — the S4
// threshold discrepancy (spec.softGate).
const CELLS = RULE_SPECS["undervote-rule"]
function predict(under, invalid, state) {
    const cell = {under_vote_policy: under, invalid_vote_policy: invalid, state}
    const r = f(CELLS.specConfig(cell), CELLS.voteState(cell))
    return {errors: r.emissions.errors, alerts: r.emissions.alerts, hard: r.gate.hard, soft: r.gate.soft, tally: r.tally}
}

// Derived (convention 3: labelled, not an observation): the per-point views
// need the policies — the voting view hides the underVote alert under
// WARN_ONLY_IN_REVIEW; the review view shows it.
function derivedInline(observed, under, invalid) {
    return inlineViews({
        errors: observed.errors,
        alerts: observed.alerts,
        policies: {under, invalid},
    })
}

const rows = []
for (const under of UNDER_POLICIES) {
    for (const invalid of INVALID_POLICIES) {
        const cellEml = makeEml(under, invalid)
        const cellContest = cellEml.contests.find((x) => x.id === contest.id)
        for (const state of STATES) {
            const decoded = runChecker(makeSelection(state), cellEml)
            const {errors, alerts} = extractErrors(decoded)
            const gates = runGates([cellContest], {[contest.id]: decoded})
            const tally = tallyClass(cellContest, decoded)
            const observed = {errors, alerts, ...gates, tally}
            const p = predict(under, invalid, state)
            const match =
                JSON.stringify([errors, alerts, gates.hard, gates.soft, tally]) ===
                JSON.stringify([p.errors, p.alerts, p.hard, p.soft, p.tally])
            rows.push({
                under_vote_policy: under,
                invalid_vote_policy: invalid,
                state,
                observed,
                derived_inline: derivedInline(observed, under, invalid),
                predicted: p,
                match,
            })
        }
    }
}

const mismatches = rows.filter((r) => !r.match)
console.log(`cells: ${rows.length}, prediction mismatches: ${mismatches.length}`)
for (const m of mismatches) {
    console.log(`\nMISMATCH under=${m.under_vote_policy} invalid=${m.invalid_vote_policy} state=${m.state}`)
    console.log("  observed :", JSON.stringify(m.observed))
    console.log("  predicted:", JSON.stringify(m.predicted))
}

writeFileSync(
    path.join(here, "undervote-rule.recorded.json"),
    JSON.stringify({contest: contest.name, rows}, null, 2) + "\n"
)

const short = (xs) =>
    xs.length === 0 ? "—" : xs.map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")
const fmt = (r) =>
    `| ${r.under_vote_policy} | ${r.invalid_vote_policy} | ${r.state} | ` +
    `${short(r.observed.errors)} | ${short(r.observed.alerts)} | ` +
    `${r.observed.hard ? "**block**" : "—"} | ${r.observed.soft ? "dialog" : "—"} | ` +
    `${r.observed.tally} | ${r.match ? "✓" : "**✗**"} |`
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Under-vote rule characterization — layers 1+2 + recorded tally class",
    "",
    "Generated by `characterization/undervote-rule.mjs`; do not edit by hand.",
    "",
    "**Experiment:** every row is one vote state on the *Referendum* contest",
    "with `min_votes` forced to 0 and `max_votes` to 2 (marker ignored; only",
    "Yes / No used), under one (under-vote policy × invalid policy) config.",
    "States: *empty* = 0 selections; *under* = 1 (the under-vote zone,",
    "`min ≤ n < max`); *full* = 2 (exactly max). Blank policy at default.",
    "",
    "Columns: *errors* / *alerts* = checker record (keys, prefix stripped);",
    "*hard/soft gate* = review-transition gates; *tally* = **recorded**",
    "per-ballot class (velvet-wasm). `pred?`: ✗ = code and docs disagree.",
    "",
    "This is the **partial (headless) table** (WASM observations only); inline",
    "visibility, the input constraint, and the silent-discount marker (⚠) are",
    "browser-only and live in the complete table (`dom-validate.mjs`). **The",
    "under-vote rule produces no silent discounts by design**: the checker",
    "emits only alerts, never errors, so an under-voted ballot is structurally",
    "`Valid` — confirming §4.4's 'cosmetic policy'.",
    "",
    "Two facts this recording pinned (both initially mis-transcribed, then",
    "corrected against the code): with `min_votes = 0` the under-vote zone",
    "`min ≤ n < max` includes **n = 0**, so the alert fires on an *empty*",
    "ballot too (overlapping blank; here blank policy is `allowed` so no",
    "blankVote dedups it). And the soft gate requires `n > 0`, so it fires",
    "only for `under`, not for the empty ballot the checker alerted on —",
    "the alert and gate thresholds differ.",
    "",
    "| under_policy | invalid_policy | state | errors | alerts | hard gate | soft gate | tally | pred? |",
    "|---|---|---|---|---|---|---|---|---|",
    ...rows.map(fmt),
    "",
].join("\n")
writeFileSync(path.join(here, "undervote-rule.md"), md)
console.log(`\nwrote undervote-rule.recorded.json and undervote-rule.md (${rows.length} rows)`)
