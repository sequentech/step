// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the OVER-VOTE rule: checker emissions + gate pair
// PLUS the recorded tally class per cell — the third column the
// no-silent-discount query needs.
//
// Contest: "Council seat" from the `explicit-blank-invalid` fixture —
// Ada / Bruno / explicit-invalid marker, min_votes=0, max_votes=1. Chosen
// because over-voting it needs only regular candidates (Ada+Bruno), so the
// over-vote condition is not conflated with marker semantics. Blank and
// under policies stay at defaults so the over-vote rule is isolated.
//
// Run:  node characterization/overvote-rule.mjs   (from packages/workbench)
// Output: overvote-rule.recorded.json + overvote-rule.md next to this file.

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

const OVER_POLICIES = [
    "allowed",
    "allowed-with-msg",
    "allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-disable",
]
const INVALID_POLICIES = [
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
]
// Vote states bracketing the over-vote condition (max_votes = 1):
//   empty    — 0 selections (blank condition; default blank policy)
//   at_max   — Ada: exactly max — the DISABLE variant's alert case
//   over_max — Ada + Bruno: max + 1 — the over-vote condition proper
const STATES = ["empty", "at_max", "over_max"]

await loadWasm()
await loadVelvetWasm()
const snap = loadMarkerFixture()
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_invalid)
)
const regulars = contest.candidates
    .filter((x) => !x.presentation?.is_explicit_invalid)
    .map((x) => x.id)

function makeSelection(state) {
    const picked =
        state === "at_max" ? [regulars[0]] : state === "over_max" ? regulars : []
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

function makeEml(overPolicy, invalidPolicy) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.presentation = {
        ...(c.presentation ?? {}),
        over_vote_policy: overPolicy,
        invalid_vote_policy: invalidPolicy,
    }
    return clone
}

// ---------------------------------------------------------------------------
// PREDICTION — spec.mjs's complete mapping `f`, fed from this rule's cell
// definitions (rule-specs.mjs: specConfig/voteState). Independent of the
// implementation by construction; mismatches are findings.
// ---------------------------------------------------------------------------
const CELLS = RULE_SPECS["overvote-rule"]
function predict(over, invalid, state) {
    const cell = {over_vote_policy: over, invalid_vote_policy: invalid, state}
    const r = f(CELLS.specConfig(cell), CELLS.voteState(cell))
    return {errors: r.emissions.errors, alerts: r.emissions.alerts, hard: r.gate.hard, soft: r.gate.soft, tally: r.tally}
}

function derivedInline(observed, over, invalid) {
    return inlineViews({
        errors: observed.errors,
        alerts: observed.alerts,
        policies: {over, invalid},
    })
}

// ---------------------------------------------------------------------------
const rows = []
for (const over of OVER_POLICIES) {
    for (const invalid of INVALID_POLICIES) {
        const cellEml = makeEml(over, invalid)
        const cellContest = cellEml.contests.find((x) => x.id === contest.id)
        for (const state of STATES) {
            const decoded = runChecker(makeSelection(state), cellEml)
            const {errors, alerts} = extractErrors(decoded)
            const gates = runGates([cellContest], {[contest.id]: decoded})
            const tally = tallyClass(cellContest, decoded)
            const observed = {errors, alerts, ...gates, tally}
            const p = predict(over, invalid, state)
            const match =
                JSON.stringify([errors, alerts, gates.hard, gates.soft, tally]) ===
                JSON.stringify([p.errors, p.alerts, p.hard, p.soft, p.tally])
            rows.push({
                over_vote_policy: over,
                invalid_vote_policy: invalid,
                state,
                observed,
                derived_inline: derivedInline(observed, over, invalid),
                predicted: p,
                match,
            })
        }
    }
}

const mismatches = rows.filter((r) => !r.match)
console.log(`cells: ${rows.length}, prediction mismatches: ${mismatches.length}`)
for (const m of mismatches) {
    console.log(
        `\nMISMATCH  over=${m.over_vote_policy} invalid=${m.invalid_vote_policy} state=${m.state}`
    )
    console.log("  observed :", JSON.stringify(m.observed))
    console.log("  predicted:", JSON.stringify(m.predicted))
}

writeFileSync(
    path.join(here, "overvote-rule.recorded.json"),
    JSON.stringify({contest: contest.name, rows}, null, 2) + "\n"
)

const short = (xs) =>
    xs.length === 0 ? "—" : xs.map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")
const fmt = (r) =>
    `| ${r.over_vote_policy} | ${r.invalid_vote_policy} | ${r.state} | ` +
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
    "# Over-vote rule characterization — checker emissions + gate pair + recorded tally class",
    "",
    "Generated by `characterization/overvote-rule.mjs`; do not edit by hand.",
    "",
    "**Experiment:** every row is one vote state on the *Council seat* contest",
    "(Ada / Bruno / explicit-invalid marker, `min_votes: 0`, `max_votes: 1`)",
    "under one (over-vote policy × invalid policy) configuration. States:",
    "*empty* = nothing selected; *at_max* = Ada (exactly `max_votes`);",
    "*over_max* = Ada + Bruno (one over). Blank/under policies at defaults.",
    "",
    "Columns: *errors* / *alerts* are the checker record (message keys,",
    "`errors.implicit.` prefix stripped); *hard/soft gate* are the",
    "review-transition gates — evaluated when the voter clicks Next on the",
    "*last* voting page (blocking dialog = review unreachable; dismissible =",
    "may continue); *tally* is the **recorded** class — the counter that incremented",
    "when this exact decoded ballot was run through velvet-wasm's real tally.",
    "`pred?` compares all five observables against the documented rules;",
    "✗ = code and docs disagree.",
    "",
    "This is the **partial (headless) table** — those five columns are real",
    "WASM observations. Inline visibility, the input constraint, and the",
    "silent-discount marker (⚠) that depends on them are browser-only; they",
    "live in the DOM-validated *complete* table (`dom-validate.mjs`) and",
    "`no-silent-discount.md`.",
    "",
    "| over_policy | invalid_policy | state | errors | alerts | hard gate | soft gate | tally | pred? |",
    "|---|---|---|---|---|---|---|---|---|",
    ...rows.map(fmt),
    "",
].join("\n")
writeFileSync(path.join(here, "overvote-rule.md"), md)
console.log(`\nwrote overvote-rule.recorded.json and overvote-rule.md (${rows.length} rows)`)
