// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the BLANK-VOTE rule: checker emissions + gate pair
// plus the recorded per-ballot tally class.
//
// Enumerates blank_vote_policy × invalid_vote_policy × vote-state over the
// Referendum contest of the `explicit-blank-invalid` fixture (Yes / No /
// explicit-blank marker, min_votes=0, max_votes=2; over/under policies left
// at their defaults so the blank rule is isolated). For every cell it
// records what checker.rs emits and what both gates answer, and compares
// each observation against a PREDICTION derived from the documented rules
// in docs/VOTE_VALIDATION.md. A mismatch means one of the two is wrong —
// which is the entire point of recording (distillation §5.3 step 2).
//
// Run:  node characterization/blank-rule.mjs   (from packages/workbench)
// Output: blank-rule.recorded.json + blank-rule.md next to this script.

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

const BLANK_POLICIES = ["allowed", "warn", "warn-only-in-review", "not-allowed"]
const INVALID_POLICIES = [
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
]
// Vote states, chosen to bracket the blank condition:
//   empty            — the blank condition proper (marker-inclusive count 0)
//   explicit_invalid — flag set, nothing selected: blank checker must skip
//   marker_only      — explicit-blank marker selected: count 1, NOT blank
//   one_regular      — control: a normal selection
const STATES = ["empty", "explicit_invalid", "marker_only", "one_regular"]

await loadWasm()
await loadVelvetWasm()
const snap = loadMarkerFixture()
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_blank)
)
const markerId = contest.candidates.find(
    (x) => x.presentation?.is_explicit_blank
).id
const regularId = contest.candidates.find(
    (x) => !x.presentation?.is_explicit_blank
).id

function makeSelection(state) {
    return {
        contest_id: contest.id,
        is_explicit_invalid: state === "explicit_invalid",
        is_decline_to_vote: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: contest.candidates.map((c) => ({
            id: c.id,
            selected:
                (state === "marker_only" && c.id === markerId) ||
                (state === "one_regular" && c.id === regularId)
                    ? 0
                    : -1,
            write_in_text: null,
        })),
    }
}

function makeEml(blankPolicy, invalidPolicy) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.presentation = {
        ...(c.presentation ?? {}),
        blank_vote_policy: blankPolicy,
        invalid_vote_policy: invalidPolicy,
    }
    return clone
}

// ---------------------------------------------------------------------------
// PREDICTION — spec.mjs's complete mapping `f`, fed from this rule's cell
// definitions (rule-specs.mjs: specConfig/voteState). Deliberately
// independent of the implementation; disagreements with the recording are
// findings, not bugs in this file.
// ---------------------------------------------------------------------------
const CELLS = RULE_SPECS["blank-rule"]
function predict(blank, invalid, state) {
    const cell = {blank_vote_policy: blank, invalid_vote_policy: invalid, state}
    const r = f(CELLS.specConfig(cell), CELLS.voteState(cell))
    return {errors: r.emissions.errors, alerts: r.emissions.alerts, hard: r.gate.hard, soft: r.gate.soft, tally: r.tally}
}

// Derived (convention 3: labelled, not an observation): what the booth
// filter leaves rendered at each observation point, computed from the
// RECORDED checker output + the transcribed filterErrorList rules
// (spec.inlineViews). The DOM validator checks the voting and review views
// per cell.
function derivedInline(observed, blank, invalid) {
    return inlineViews({
        errors: observed.errors,
        alerts: observed.alerts,
        policies: {blank, invalid},
    })
}

// ---------------------------------------------------------------------------
const rows = []
for (const blank of BLANK_POLICIES) {
    for (const invalid of INVALID_POLICIES) {
        const cellEml = makeEml(blank, invalid)
        for (const state of STATES) {
            const decoded = runChecker(makeSelection(state), cellEml)
            const {errors, alerts} = extractErrors(decoded)
            const cellContest = cellEml.contests.find(
                (c) => c.id === contest.id
            )
            const gates = runGates([cellContest], {[contest.id]: decoded})
            const tally = tallyClass(cellContest, decoded)
            const p = predict(blank, invalid, state)
            const match =
                JSON.stringify([errors, alerts, gates.hard, gates.soft, tally]) ===
                JSON.stringify([p.errors, p.alerts, p.hard, p.soft, p.tally])
            rows.push({
                blank_vote_policy: blank,
                invalid_vote_policy: invalid,
                state,
                observed: {errors, alerts, ...gates, tally},
                derived_inline: derivedInline(
                    {errors, alerts},
                    blank,
                    invalid
                ),
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
        `\nMISMATCH  blank=${m.blank_vote_policy} invalid=${m.invalid_vote_policy} state=${m.state}`
    )
    console.log("  observed :", JSON.stringify(m.observed))
    console.log("  predicted:", JSON.stringify(m.predicted))
}

writeFileSync(
    path.join(here, "blank-rule.recorded.json"),
    JSON.stringify({generated: "see git history", contest: contest.name, rows}, null, 2) + "\n"
)

// human-readable table
const short = (xs) =>
    xs.length === 0 ? "—" : xs.map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")
const fmt = (r) =>
    `| ${r.blank_vote_policy} | ${r.invalid_vote_policy} | ${r.state} | ` +
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
    "# Blank-rule characterization — checker emissions + gate pair + recorded tally class",
    "",
    "Generated by `characterization/blank-rule.mjs`; do not edit by hand.",
    "",
    "**Experiment:** every row is one vote state on the *Referendum* contest",
    "(Yes / No / explicit-blank marker, `min_votes: 0`, `max_votes: 2`) under",
    "one (blank policy × invalid policy) configuration. States:",
    "*empty* = nothing selected (the blank condition); *explicit_invalid* =",
    "nothing selected, explicit-invalid flag set; *marker_only* = the",
    "explicit-blank marker alone (counts as a selection — NOT blank at the",
    "booth); *one_regular* = one normal candidate (control).",
    "Over/under policies at defaults.",
    "",
    "Columns: *errors* / *alerts* are the checker record (message keys,",
    "`errors.implicit.`/`errors.explicit.` prefix stripped); *hard/soft",
    "gate* are the review-transition gates (blocking vs dismissible dialog);",
    "*tally* is the **recorded** per-ballot class — the counter that",
    "incremented when this decoded ballot ran through velvet-wasm's real",
    "tally. `pred?` compares all five observables against the documented",
    "rules; ✗ = code and docs disagree.",
    "",
    "This is the **partial (headless) table** — those five columns are real",
    "WASM observations. Inline visibility, the input constraint, and the",
    "silent-discount marker (⚠) that depends on them are browser-only; they",
    "live in the DOM-validated *complete* table (`dom-validate.mjs`) and",
    "`no-silent-discount.md`.",
    "",
    "| blank_policy | invalid_policy | state | errors | alerts | hard gate | soft gate | tally | pred? |",
    "|---|---|---|---|---|---|---|---|---|",
    ...rows.map(fmt),
    "",
].join("\n")
writeFileSync(path.join(here, "blank-rule.md"), md)
console.log(`\nwrote blank-rule.recorded.json and blank-rule.md (${rows.length} rows)`)
