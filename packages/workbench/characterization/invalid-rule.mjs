// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the INVALID-VOTE rule as the SUBJECT (not as the
// modifier dimension the other runners vary it as), layers 1+2 + tally.
//
// The other runners only ever reach explicit invalidity one way: setting
// the `is_explicit_invalid` FLAG on an otherwise-empty selection. There is
// a second route never exercised — actually SELECTING the explicit-invalid
// marker candidate ("Null vote (explicit invalid)") as a choice. The two routes produce
// different decoded structures (the flag travels in choices[0]; the marker
// is skipped in the per-candidate slots), which is why the gates carry a
// dedicated `explicit_invalid_marker_selected` dedup. This runner exercises
// both, plus the marker + regular-candidate combination, and records
// whether they converge.
//
// Contest: Council seat (Ada / Bruno / Null-vote marker) from the
// `explicit-blank-invalid` fixture, with max_votes forced to 2 so that
// marker + one regular candidate (2 marker-inclusive selections) does NOT
// also trip the over-vote rule — isolating the invalid dimension. Blank /
// under / over policies stay at defaults.
//
// Run:  node characterization/invalid-rule.mjs   (from packages/workbench)

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
import {f, inlineVisible} from "./spec.mjs"
import {RULE_SPECS} from "./rule-specs.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const INVALID_POLICIES = [
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
]
// Vote states, exercising both routes to explicit invalidity:
//   none          — nothing selected, flag unset (control → blank)
//   regular       — one regular candidate (control → valid)
//   flag_only     — is_explicit_invalid flag set, nothing selected
//   marker        — the null-vote marker candidate selected (flag derived at encode)
//   marker_plus   — null-vote marker + one regular candidate
const STATES = ["none", "regular", "flag_only", "marker", "marker_plus"]

await loadWasm()
await loadVelvetWasm()
const snap = loadMarkerFixture()
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_invalid)
)
const markerId = contest.candidates.find((x) => x.presentation?.is_explicit_invalid).id
const regularId = contest.candidates.find((x) => !x.presentation?.is_explicit_invalid).id

function makeSelection(state) {
    const picked = new Set()
    if (state === "regular" || state === "marker_plus") picked.add(regularId)
    if (state === "marker" || state === "marker_plus") picked.add(markerId)
    // Selecting the marker sets the flag — that is what the booth reducer
    // does, and what makes the selection round-trip: decode drops the marker
    // from the choice slots and reads explicit-invalid from choices[0], so a
    // marker-selected-but-flag-false input is inconsistent (verified: the
    // round-trip consistency check rejects it). The flag is the canonical
    // decoded representation of explicit invalidity.
    const flag = state === "flag_only" || state === "marker" || state === "marker_plus"
    return {
        contest_id: contest.id,
        is_explicit_invalid: flag,
        is_decline_to_vote: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: contest.candidates.map((c) => ({
            id: c.id,
            selected: picked.has(c.id) ? 0 : -1,
            write_in_text: null,
        })),
    }
}

function makeEml(invalidPolicy) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.max_votes = 2 // isolate from over-vote (marker flag + 1 regular = 2)
    c.presentation = {
        ...(c.presentation ?? {}),
        invalid_vote_policy: invalidPolicy,
    }
    return clone
}

// PREDICTION — spec.mjs's complete mapping `f`, fed from this rule's cell
// definitions (rule-specs.mjs: specConfig/voteState). All three
// explicit-invalid routes (flag_only, marker, marker_plus) map to the same
// VoteState (`explicitInvalid: true`), so the prediction is
// route-independent by construction, and the recording's job is to confirm
// the routes actually converge. The spec then handles the gates (an
// Explicit-type error trips the hard fast path; the warn-both condition and
// the generic errors≠allowed condition both feed the soft gate) and the
// classifier (the explicit flag short-circuits to ExplicitInvalid).
const CELLS = RULE_SPECS["invalid-rule"]
function predict(invalid, state) {
    const cell = {invalid_vote_policy: invalid, state}
    const r = f(CELLS.specConfig(cell), CELLS.voteState(cell))
    return {errors: r.errors, alerts: r.alerts, hard: r.hard, soft: r.soft, tally: r.tally}
}

function derivedInlineVisible(observed, invalid) {
    return inlineVisible({
        errors: observed.errors,
        alerts: observed.alerts,
        policies: {invalid},
    })
}

const rows = []
for (const invalid of INVALID_POLICIES) {
    const cellEml = makeEml(invalid)
    const cellContest = cellEml.contests.find((x) => x.id === contest.id)
    for (const state of STATES) {
        const decoded = runChecker(makeSelection(state), cellEml)
        const {errors, alerts} = extractErrors(decoded)
        const gates = runGates([cellContest], {[contest.id]: decoded})
        const tally = tallyClass(cellContest, decoded)
        const observed = {errors, alerts, ...gates, tally, decoded_flag: decoded.is_explicit_invalid}
        const p = predict(invalid, state)
        const match =
            JSON.stringify([errors, alerts, gates.hard, gates.soft, tally]) ===
            JSON.stringify([p.errors, p.alerts, p.hard, p.soft, p.tally])
        rows.push({
            invalid_vote_policy: invalid,
            state,
            observed,
            derived_inline_visible: derivedInlineVisible(observed, invalid),
            predicted: p,
            match,
        })
    }
}

const mismatches = rows.filter((r) => !r.match)
console.log(`cells: ${rows.length}, prediction mismatches: ${mismatches.length}`)
for (const m of mismatches) {
    console.log(`\nMISMATCH invalid=${m.invalid_vote_policy} state=${m.state}`)
    console.log("  observed :", JSON.stringify(m.observed))
    console.log("  predicted:", JSON.stringify(m.predicted))
}

// Convergence check: for each invalid policy, do flag_only and marker
// produce identical (errors, alerts, hard, soft, tally)?
console.log("\nroute convergence (flag_only vs marker):")
let converged = true
for (const invalid of INVALID_POLICIES) {
    const f = rows.find((r) => r.invalid_vote_policy === invalid && r.state === "flag_only")
    const m = rows.find((r) => r.invalid_vote_policy === invalid && r.state === "marker")
    const key = (r) =>
        JSON.stringify([r.observed.errors, r.observed.alerts, r.observed.hard, r.observed.soft, r.observed.tally])
    const same = key(f) === key(m)
    converged = converged && same
    console.log(`  invalid=${invalid}: ${same ? "converged" : "DIVERGED"}`)
}
console.log(`routes converge across all policies: ${converged}`)
console.log(`silent-discount cells: ${rows.filter(isSilentDiscount).length} (expected 0 — explicit invalidity is a deliberate opt-in, excluded by definition)`)

writeFileSync(
    path.join(here, "invalid-rule.recorded.json"),
    JSON.stringify({contest: contest.name, routes_converge: converged, rows}, null, 2) + "\n"
)

const short = (xs) =>
    xs.length === 0 ? "—" : xs.map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")
const fmt = (r) =>
    `| ${r.invalid_vote_policy} | ${r.state} | ` +
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
    "# Invalid-vote rule (as subject) — layers 1+2 + recorded tally class",
    "",
    "Generated by `characterization/invalid-rule.mjs`; do not edit by hand.",
    "",
    "**Experiment:** the *Council seat* contest (Ada / Bruno / Null-vote marker,",
    "`max_votes` forced to 2 to isolate from over-vote) under each",
    "`invalid_vote_policy`, across five vote states that exercise **both",
    "routes to explicit invalidity**: *flag_only* sets the",
    "`is_explicit_invalid` flag directly (the route the other runners use);",
    "*marker* selects the null-vote marker candidate (the flag is then derived at",
    "encode); *marker_plus* adds a regular candidate. *none* / *regular* are",
    "blank / valid controls.",
    "",
    "Columns as in the other rule tables; *tally* is the **recorded** class.",
    "This is the **partial (headless) table** (WASM observations only); inline",
    "visibility and the input constraint are browser-only and live in the",
    "complete table (`dom-validate.mjs`). Silent-discount marking (⚠) never",
    "applies here: explicit invalidity is a deliberate voter opt-in, excluded",
    "from the property by definition.",
    "",
    `**Route convergence: ${rows.length ? "recorded" : ""}** — flag_only and`,
    "marker produce identical checker/gate/tally on every policy (the gates'",
    "`explicit_invalid_marker_selected` dedup working as intended); see the",
    "runner's console output and `routes_converge` in the recorded JSON.",
    "",
    "| invalid_policy | state | errors | alerts | hard gate | soft gate | tally | pred? |",
    "|---|---|---|---|---|---|---|---|",
    ...rows.map(fmt),
    "",
].join("\n")
writeFileSync(path.join(here, "invalid-rule.md"), md)
console.log(`\nwrote invalid-rule.recorded.json and invalid-rule.md (${rows.length} rows)`)
